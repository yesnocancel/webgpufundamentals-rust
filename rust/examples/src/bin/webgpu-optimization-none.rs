use std::sync::{Arc, Mutex};

use glam::{Mat3, Mat4, Vec3};
use wgpu_fun::{App, Frame, ImageData, RenderMode};

// see https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    Free,
    NeedResolve,
    WaitForResult,
}

struct TimingHelper {
    can_timestamp: bool,
    device: wgpu::Device,
    // timestamps are in GPU ticks; this many nanoseconds each (1.0 on the web)
    timestamp_period: f64,
    query_set: Option<wgpu::QuerySet>,
    resolve_buffer: Option<wgpu::Buffer>,
    result_buffer: Option<wgpu::Buffer>,
    result_buffers: Arc<Mutex<Vec<wgpu::Buffer>>>,
    // state can be Free, NeedResolve, WaitForResult
    state: State,
}

impl TimingHelper {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let can_timestamp = device.features().contains(wgpu::Features::TIMESTAMP_QUERY);
        let (query_set, resolve_buffer) = if can_timestamp {
            let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                label: None,
                ty: wgpu::QueryType::Timestamp,
                count: 2,
            });
            let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: query_set.count() as u64 * 8,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            (Some(query_set), Some(resolve_buffer))
        } else {
            (None, None)
        };
        Self {
            can_timestamp,
            device: device.clone(),
            timestamp_period: queue.get_timestamp_period() as f64,
            query_set,
            resolve_buffer,
            result_buffer: None,
            result_buffers: Arc::new(Mutex::new(Vec::new())),
            state: State::Free,
        }
    }

    fn begin_render_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut wgpu::CommandEncoder,
        descriptor: &wgpu::RenderPassDescriptor<'_>,
    ) -> wgpu::RenderPass<'encoder> {
        if self.can_timestamp {
            assert!(self.state == State::Free, "state not free");
            self.state = State::NeedResolve;

            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                    query_set: self.query_set.as_ref().unwrap(),
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
                ..descriptor.clone()
            })
        } else {
            encoder.begin_render_pass(descriptor)
        }
    }

    // In JS this runs automatically when pass.end() is called. In Rust a
    // pass ends when it's dropped, so call this right after.
    fn resolve_timing(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.can_timestamp {
            return;
        }
        assert!(
            self.state == State::NeedResolve,
            "you must use timing_helper.begin_render_pass or timing_helper.begin_compute_pass",
        );
        self.state = State::WaitForResult;

        let query_set = self.query_set.as_ref().unwrap();
        let resolve_buffer = self.resolve_buffer.as_ref().unwrap();
        let result_buffer = self
            .result_buffers
            .lock()
            .unwrap()
            .pop()
            .unwrap_or_else(|| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: resolve_buffer.size(),
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            });

        encoder.resolve_query_set(query_set, 0..query_set.count(), resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(resolve_buffer, 0, &result_buffer, 0, result_buffer.size());
        self.result_buffer = Some(result_buffer);
    }

    // In JS this is async and returns the duration; in Rust the mapping
    // completes through a callback, so we pass the duration (in
    // nanoseconds) to a callback. Call after submitting the command buffer.
    fn get_result(&mut self, callback: impl FnOnce(f64) + Send + 'static) {
        if !self.can_timestamp {
            callback(0.0);
            return;
        }
        assert!(
            self.state == State::WaitForResult,
            "you must call resolve_timing and submit the command buffer before you can read the result",
        );
        self.state = State::Free;

        let result_buffer = self.result_buffer.take().unwrap();
        let result_buffers = self.result_buffers.clone();
        let timestamp_period = self.timestamp_period;
        result_buffer
            .clone()
            .map_async(wgpu::MapMode::Read, .., move |result| {
                result.expect("failed to map result buffer");
                let duration = {
                    let view = result_buffer.slice(..).get_mapped_range().unwrap();
                    let times: &[i64] = bytemuck::cast_slice(&view);
                    (times[1] - times[0]) as f64 * timestamp_period
                };
                result_buffer.unmap();
                result_buffers.lock().unwrap().push(result_buffer);
                callback(duration);
            });
    }
}

// Note: We disallow negative values as this is used for timestamp queries
// where it's possible for a query to return a beginning time greater than the
// end time. See: https://gpuweb.github.io/gpuweb/#timestamp
struct NonNegativeRollingAverage {
    total: f64,
    samples: Vec<f64>,
    cursor: usize,
    num_samples: usize,
}

impl NonNegativeRollingAverage {
    fn new() -> Self {
        Self {
            total: 0.0,
            samples: Vec::new(),
            cursor: 0,
            num_samples: 30,
        }
    }

    fn add_sample(&mut self, v: f64) {
        if !v.is_nan() && v.is_finite() && v >= 0.0 {
            if self.samples.len() <= self.cursor {
                self.samples.push(0.0);
            }
            self.total += v - self.samples[self.cursor];
            self.samples[self.cursor] = v;
            self.cursor = (self.cursor + 1) % self.num_samples;
        }
    }

    fn get(&self) -> f64 {
        self.total / self.samples.len() as f64
    }
}

/// Given hue, saturation, and luminance values in the range of 0 to 1
/// returns an array of 4 values from 0 to 1.
/// (The JS version makes a CSS `hsl()` string and reads the color back
/// through a 2D canvas; we do the same hsl → rgb conversion directly.)
fn hsl_to_rgba(h: f32, s: f32, l: f32) -> [f32; 4] {
    let h = (h * 360.0).floor(); // the JS `h * 360 | 0`
    let l = (l * 100.0).floor() / 100.0; // the JS `l * 100 | 0`
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h as u32) / 60 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    // the 2D canvas readback quantizes to 8 bits
    [
        ((r + m) * 255.0).round() / 255.0,
        ((g + m) * 255.0).round() / 255.0,
        ((b + m) * 255.0).round() / 255.0,
        1.0,
    ]
}

/// Returns a random number between min and max.
/// (The JS version also has 0 and 1 argument forms; we always pass a range.)
fn rand(min: f32, max: f32) -> f32 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u32> = const { Cell::new(0x13579bdf) });
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        min + (max - min) * (x as f32 / u32::MAX as f32)
    })
}

/// Selects a random array element
fn random_array_element<T>(arr: &[T]) -> &T {
    &arr[(rand(0.0, 1.0) * arr.len() as f32) as usize % arr.len()]
}

fn num_mip_levels(sizes: &[u32]) -> u32 {
    let max_size = *sizes.iter().max().unwrap();
    1 + (max_size as f32).log2() as u32
}

// see https://webgpufundamentals.org/webgpu/lessons/webgpu-importing-textures.html
fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    // The JS version lazily caches the module/sampler/pipeline in a closure;
    // we cache them in thread locals keyed by format.
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static CACHE: RefCell<Option<(wgpu::ShaderModule, wgpu::Sampler)>> = const { RefCell::new(None) };
        static PIPELINE_BY_FORMAT: RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>> =
            RefCell::new(HashMap::new());
    }

    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let (module, sampler) = cache.get_or_insert_with(|| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("textured quad shaders for mip level generation"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
            struct VSOutput {
              @builtin(position) position: vec4f,
              @location(0) texcoord: vec2f,
            };

            @vertex fn vs(
              @builtin(vertex_index) vertexIndex : u32
            ) -> VSOutput {
              let pos = array(

                vec2f( 0.0,  0.0),  // center
                vec2f( 1.0,  0.0),  // right, center
                vec2f( 0.0,  1.0),  // center, top

                // 2st triangle
                vec2f( 0.0,  1.0),  // center, top
                vec2f( 1.0,  0.0),  // right, center
                vec2f( 1.0,  1.0),  // right, top
              );

              var vsOutput: VSOutput;
              let xy = pos[vertexIndex];
              vsOutput.position = vec4f(xy * 2.0 - 1.0, 0.0, 1.0);
              vsOutput.texcoord = vec2f(xy.x, 1.0 - xy.y);
              return vsOutput;
            }

            @group(0) @binding(0) var ourSampler: sampler;
            @group(0) @binding(1) var ourTexture: texture_2d<f32>;

            @fragment fn fs(fsInput: VSOutput) -> @location(0) vec4f {
              return textureSample(ourTexture, ourSampler, fsInput.texcoord);
            }
          "#
                    .into(),
                ),
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            (module, sampler)
        });

        PIPELINE_BY_FORMAT.with(|pipelines| {
            let mut pipelines = pipelines.borrow_mut();
            let pipeline = pipelines.entry(texture.format()).or_insert_with(|| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("mip level generator pipeline"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: None,
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: None,
                        compilation_options: Default::default(),
                        targets: &[Some(texture.format().into())],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mip gen encoder"),
            });

            for base_mip_level in 1..texture.mip_level_count() {
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.create_view(
                                &wgpu::TextureViewDescriptor {
                                    base_mip_level: base_mip_level - 1,
                                    mip_level_count: Some(1),
                                    ..Default::default()
                                },
                            )),
                        },
                    ],
                });

                let view = texture.create_view(&wgpu::TextureViewDescriptor {
                    base_mip_level,
                    mip_level_count: Some(1),
                    ..Default::default()
                });
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("our basic canvas renderPass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                            depth_slice: None,
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.draw(0..6, 0..1); // call our vertex shader 6 times
                }
            }

            let command_buffer = encoder.finish();
            queue.submit([command_buffer]);
        });
    });
}

// see https://webgpufundamentals.org/webgpu/lessons/webgpu-importing-textures.html
fn create_texture_from_source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &ImageData,
    mips: bool,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        format: wgpu::TextureFormat::Rgba8Unorm,
        mip_level_count: if mips {
            num_mip_levels(&[source.width, source.height])
        } else {
            1
        },
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &source.data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(source.width * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
    );
    if texture.mip_level_count() > 1 {
        generate_mips(device, queue, &texture);
    }
    texture
}

#[derive(Clone)]
struct Material {
    color: [f32; 4],
    shininess: f32,
    texture: wgpu::Texture,
    sampler: wgpu::Sampler,
}

struct ObjectInfo {
    bind_group: wgpu::BindGroup,

    uniform_buffer: wgpu::Buffer,
    uniform_values: Vec<f32>,

    axis: Vec3,
    material: Material,
    radius: f32,
    speed: f32,
    rotation_speed: f32,
    scale: f32,
}

async fn run() {
    // ask for the timestamp-query feature if the adapter supports it
    let mut app = App::new_with_features(
        "WebGPU Optimization - None",
        wgpu::Features::TIMESTAMP_QUERY,
    )
    .await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
    let can_timestamp = app
        .device
        .features()
        .contains(wgpu::Features::TIMESTAMP_QUERY);

    let mut timing_helper = TimingHelper::new(&app.device, &app.queue);

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct Uniforms {
        normalMatrix: mat3x3f,
        viewProjection: mat4x4f,
        world: mat4x4f,
        color: vec4f,
        lightWorldPosition: vec3f,
        viewWorldPosition: vec3f,
        shininess: f32,
      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) normal: vec3f,
        @location(2) texcoord: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) normal: vec3f,
        @location(1) surfaceToLight: vec3f,
        @location(2) surfaceToView: vec3f,
        @location(3) texcoord: vec2f,
      };

      @group(0) @binding(0) var diffuseTexture: texture_2d<f32>;
      @group(0) @binding(1) var diffuseSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = uni.viewProjection * uni.world * vert.position;

        // Orient the normals and pass to the fragment shader
        vsOut.normal = uni.normalMatrix * vert.normal;

        // Compute the world position of the surface
        let surfaceWorldPosition = (uni.world * vert.position).xyz;

        // Compute the vector of the surface to the light
        // and pass it to the fragment shader
        vsOut.surfaceToLight = uni.lightWorldPosition - surfaceWorldPosition;

        // Compute the vector of the surface to the light
        // and pass it to the fragment shader
        vsOut.surfaceToView = uni.viewWorldPosition - surfaceWorldPosition;

        // Pass the texture coord on to the fragment shader
        vsOut.texcoord = vert.texcoord;

        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        // Because vsOut.normal is an inter-stage variable
        // it's interpolated so it will not be a unit vector.
        // Normalizing it will make it a unit vector again
        let normal = normalize(vsOut.normal);

        let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
        let surfaceToViewDirection = normalize(vsOut.surfaceToView);
        let halfVector = normalize(
          surfaceToLightDirection + surfaceToViewDirection);

        // Compute the light by taking the dot product
        // of the normal with the direction to the light
        let light = dot(normal, surfaceToLightDirection);

        var specular = dot(normal, halfVector);
        specular = select(
            0.0,                           // value if condition is false
            pow(specular, uni.shininess),  // value if condition is true
            specular > 0.0);               // condition

        let diffuse = uni.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
        // Lets multiply just the color portion (not the alpha)
        // by the light
        let color = diffuse.rgb * light + specular;
        return vec4f(color, diffuse.a);
      }
    "#
                .into(),
            ),
        });

    fn create_buffer_with_data(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: data.len() as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, data);
        buffer
    }

    #[rustfmt::skip]
    let positions: Vec<f32> = vec![1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
    #[rustfmt::skip]
    let normals: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0];
    #[rustfmt::skip]
    let texcoords: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    #[rustfmt::skip]
    let indices: Vec<u16> = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23];

    let position_buffer = create_buffer_with_data(
        &app.device,
        &app.queue,
        bytemuck::cast_slice(&positions),
        wgpu::BufferUsages::VERTEX,
    );
    let normal_buffer = create_buffer_with_data(
        &app.device,
        &app.queue,
        bytemuck::cast_slice(&normals),
        wgpu::BufferUsages::VERTEX,
    );
    let texcoord_buffer = create_buffer_with_data(
        &app.device,
        &app.queue,
        bytemuck::cast_slice(&texcoords),
        wgpu::BufferUsages::VERTEX,
    );
    let indices_buffer = create_buffer_with_data(
        &app.device,
        &app.queue,
        bytemuck::cast_slice(&indices),
        wgpu::BufferUsages::INDEX,
    );
    let num_vertices = indices.len() as u32;

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("textured model with point light w/specular highlight"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    // position
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 3 * 4, // 3 floats
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    }),
                    // normal
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 3 * 4, // 3 floats
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        }],
                    }),
                    // uvs
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 2 * 4, // 2 floats
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            shader_location: 2,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(app.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    // The JS version draws each emoji into a 2D canvas and creates a texture
    // from it; there is no 2D canvas outside the browser so we load pre-made
    // 128x128 images of the same emoji.
    let mut textures = Vec::new();
    for url in [
        "resources/images/emoji/face-with-tears-of-joy.png", // 😂
        "resources/images/emoji/alien-monster.png",          // 👾
        "resources/images/emoji/thumbs-up.png",              // 👍
        "resources/images/emoji/eyes.png",                   // 👀
        "resources/images/emoji/sun-with-face.png",          // 🌞
        "resources/images/emoji/ring-buoy.png",              // 🛟
    ] {
        let source = wgpu_fun::load_image(url).await;
        textures.push(create_texture_from_source(
            &app.device,
            &app.queue,
            &source,
            true,
        ));
    }

    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });

    let num_materials = 20;
    let mut materials: Vec<Material> = Vec::new();
    for _ in 0..num_materials {
        let color = hsl_to_rgba(rand(0.0, 1.0), rand(0.5, 0.8), rand(0.5, 0.7));
        let shininess = rand(10.0, 120.0);
        materials.push(Material {
            color,
            shininess,
            texture: random_array_element(&textures).clone(),
            sampler: sampler.clone(),
        });
    }

    let max_objects = 30000;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    // offsets to the various uniform values in float32 indices
    const K_NORMAL_MATRIX_OFFSET: usize = 0;
    const K_VIEW_PROJECTION_OFFSET: usize = 12;
    const K_WORLD_OFFSET: usize = 28;
    const K_COLOR_OFFSET: usize = 44;
    const K_LIGHT_WORLD_POSITION_OFFSET: usize = 48;
    const K_VIEW_WORLD_POSITION_OFFSET: usize = 52;
    const K_SHININESS_OFFSET: usize = 55;

    for _ in 0..max_objects {
        let uniform_buffer_size = (12 + 16 + 16 + 4 + 4 + 4) * 4;
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: uniform_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // (in JS this is a Float32Array with per-value subarray views; in
        // Rust we keep one Vec and index it with the offsets above)
        let uniform_values = vec![0.0f32; uniform_buffer_size as usize / 4];

        let material = random_array_element(&materials).clone();

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind group for object"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &material.texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&material.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let axis = Vec3::new(rand(-1.0, 1.0), rand(-1.0, 1.0), rand(-1.0, 1.0)).normalize();
        let radius = rand(10.0, 100.0);
        let speed = rand(0.1, 0.4);
        let rotation_speed = rand(-1.0, 1.0);
        let scale = rand(2.0, 10.0);

        object_infos.push(ObjectInfo {
            bind_group,

            uniform_buffer,
            uniform_values,

            axis,
            material,
            radius,
            speed,
            rotation_speed,
            scale,
        });
    }

    let mut fps_average = NonNegativeRollingAverage::new();
    let mut js_average = NonNegativeRollingAverage::new();
    let gpu_average = Arc::new(Mutex::new(NonNegativeRollingAverage::new()));
    let mut math_average = NonNegativeRollingAverage::new();

    let mut depth_texture: Option<wgpu::Texture> = None;
    // a 1x1 texture to render to when 'render' is unchecked (see below)
    let mut small_target: Option<wgpu::Texture> = None;
    let mut then = 0.0;

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time = frame.time; // seconds
        let delta_time = time - then;
        then = time;
        let time = time as f32;

        let start_time_ms = wgpu_fun::now_ms();

        // read the settings the GUI on the page sets
        let num_objects = (wgpu_fun::setting_f64("numObjects", 1000.0) as usize).min(max_objects);
        let render = wgpu_fun::setting_bool("render", true);

        // The JS version resizes the canvas to 1x1 when 'render' is
        // unchecked, which removes nearly all of the rasterization work. We
        // can't resize the window/canvas from inside the frame callback so we
        // render to a cached 1x1 offscreen texture instead, which removes the
        // same work.
        let (target_width, target_height) = if render {
            (frame.width, frame.height)
        } else {
            (1, 1)
        };
        if !render && small_target.is_none() {
            small_target = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format: frame.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }
        let small_target_view = small_target
            .as_ref()
            .map(|t| t.create_view(&Default::default()));
        let color_view = if render {
            frame.view
        } else {
            small_target_view.as_ref().unwrap()
        };

        // If we don't have a depth texture OR if its size is different
        // from the target texture make a new depth texture
        if depth_texture
            .as_ref()
            .is_none_or(|t| t.width() != target_width || t.height() != target_height)
        {
            if let Some(t) = depth_texture.take() {
                t.destroy();
            }
            depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: target_width,
                    height: target_height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }
        let depth_view = depth_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        let mut math_elapsed_time_ms = 0.0;

        let mut encoder = frame.device.create_command_encoder(&Default::default());
        {
            let mut pass = timing_helper.begin_render_pass(
                &mut encoder,
                &wgpu::RenderPassDescriptor {
                    label: Some("our basic canvas renderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: color_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.3,
                                g: 0.3,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    ..Default::default()
                },
            );
            pass.set_pipeline(&pipeline);
            pass.set_vertex_buffer(0, position_buffer.slice(..));
            pass.set_vertex_buffer(1, normal_buffer.slice(..));
            pass.set_vertex_buffer(2, texcoord_buffer.slice(..));
            pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint16);

            let aspect = frame.width as f32 / frame.height as f32;
            let projection = Mat4::perspective_rh(
                60.0f32.to_radians(),
                aspect,
                1.0,    // zNear
                2000.0, // zFar
            );

            let eye = Vec3::new(100.0, 150.0, 200.0);
            let target = Vec3::new(0.0, 0.0, 0.0);
            let up = Vec3::new(0.0, 1.0, 0.0);

            // Compute a view matrix
            let view_matrix = Mat4::look_at_rh(eye, target, up);

            // Combine the view and projection matrixes
            let view_projection_matrix = projection * view_matrix;

            for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
                let math_time_start_ms = wgpu_fun::now_ms();

                let uniform_values = &mut object.uniform_values;

                // Copy the viewProjectionMatrix into the uniform values for this object
                uniform_values[K_VIEW_PROJECTION_OFFSET..][..16]
                    .copy_from_slice(&view_projection_matrix.to_cols_array());

                // Compute a world matrix
                let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
                    * Mat4::from_translation(Vec3::new(
                        0.0,
                        0.0,
                        (i as f32 * 3.721 + time * object.speed).sin() * object.radius,
                    ))
                    * Mat4::from_translation(Vec3::new(
                        0.0,
                        0.0,
                        (i as f32 * 9.721 + time * 0.1).sin() * object.radius,
                    ))
                    * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
                    * Mat4::from_scale(Vec3::splat(object.scale));
                uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

                // Inverse and transpose it into the normalMatrix value
                // (a WGSL mat3x3f is 3 columns of 3 floats + 1 float of padding each)
                let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
                uniform_values[K_NORMAL_MATRIX_OFFSET..][..3]
                    .copy_from_slice(&normal_matrix.x_axis.to_array());
                uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3]
                    .copy_from_slice(&normal_matrix.y_axis.to_array());
                uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3]
                    .copy_from_slice(&normal_matrix.z_axis.to_array());

                let Material {
                    color, shininess, ..
                } = object.material;

                // copy the materials values.
                uniform_values[K_COLOR_OFFSET..][..4].copy_from_slice(&color);
                uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..][..3]
                    .copy_from_slice(&[-10.0, 30.0, 300.0]);
                uniform_values[K_VIEW_WORLD_POSITION_OFFSET..][..3]
                    .copy_from_slice(&eye.to_array());
                uniform_values[K_SHININESS_OFFSET] = shininess;

                math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

                // upload the uniform values to the uniform buffer
                frame.queue.write_buffer(
                    &object.uniform_buffer,
                    0,
                    bytemuck::cast_slice(uniform_values),
                );

                pass.set_bind_group(0, &object.bind_group, &[]);
                pass.draw_indexed(0..num_vertices, 0, 0..1);
            }
        }

        timing_helper.resolve_timing(&mut encoder);

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);

        {
            let gpu_average = gpu_average.clone();
            timing_helper.get_result(move |gpu_time| {
                gpu_average.lock().unwrap().add_sample(gpu_time / 1000.0);
            });
        }
        // mapAsync results are delivered when the device is polled; the
        // browser does that for us, natively we poll once per frame.
        let _ = frame.device.poll(wgpu::PollType::Poll);

        let elapsed_time_ms = wgpu_fun::now_ms() - start_time_ms;
        fps_average.add_sample(1.0 / delta_time);
        js_average.add_sample(elapsed_time_ms);
        math_average.add_sample(math_elapsed_time_ms);

        wgpu_fun::set_info_text(&format!(
            "\
js  : {:.1}ms
math: {:.1}ms
fps : {:.0}
gpu : {}
",
            js_average.get(),
            math_average.get(),
            fps_average.get(),
            if can_timestamp {
                format!("{:.1}ms", gpu_average.lock().unwrap().get() / 1000.0)
            } else {
                "N/A".to_string()
            },
        ));
    });
}

fn main() {
    wgpu_fun::start(run());
}
