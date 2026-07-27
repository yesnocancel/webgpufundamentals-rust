use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, RenderMode};

fn num_mip_levels(sizes: &[u32]) -> u32 {
    let max_size = *sizes.iter().max().unwrap();
    1 + (max_size as f32).log2() as u32
}

fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
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
                    /* wgsl */ r#"
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

const SIZE: usize = 256;
const HALF: f32 = SIZE as f32 / 2.0;

fn hsl(h: f32, s: f32, l: f32) -> [u8; 4] {
    // matches CSS hsl(); h in turns here like the JS helper's input
    let h = (h.fract() + 1.0).fract() * 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
        255,
    ]
}

// A CPU stand-in for the JS example's 2D-canvas animation: the same 20
// nested squares, each rotated, scaled 0.85x and offset from the previous,
// with the same cycling hues. (There's no 2D canvas outside the browser, so
// we rasterize the squares ourselves; each square is drawn by
// inverse-transforming pixels through its accumulated transform.)
fn update_2d_canvas(pixels: &mut [u8], time: f64) {
    let time = time * 0.0001;
    pixels.fill(0); // ctx.clearRect
    let num = 20;
    // canvas-style affine transform: [a b; c d] + translation [e f]
    let mut t = [1.0f32, 0.0, 0.0, 1.0, HALF, HALF]; // ctx.translate(half, half)
    for i in 0..num {
        let color = hsl(
            i as f32 / num as f32 * 0.2 + time as f32 * 0.1,
            1.0,
            (i % 2) as f32 * 0.5,
        );

        // ctx.fillRect(-half, -half, size, size) under the current transform:
        // inverse-map each pixel and test the square.
        let [a, b, c, d, e, f] = t;
        let det = a * d - b * c;
        let (ia, ib, ic, id) = (d / det, -b / det, -c / det, a / det);
        for y in 0..SIZE {
            for x in 0..SIZE {
                let px = x as f32 + 0.5 - e;
                let py = y as f32 + 0.5 - f;
                let sx = ia * px + ic * py;
                let sy = ib * px + id * py;
                if sx >= -HALF && sx < HALF && sy >= -HALF && sy < HALF {
                    let o = (y * SIZE + x) * 4;
                    pixels[o..o + 4].copy_from_slice(&color);
                }
            }
        }

        // ctx.rotate(time * 0.5); ctx.scale(0.85, 0.85); ctx.translate(size / 16, 0);
        let (sin, cos) = (time as f32 * 0.5).sin_cos();
        t = mul(t, [cos, sin, -sin, cos, 0.0, 0.0]);
        t = mul(t, [0.85, 0.0, 0.0, 0.85, 0.0, 0.0]);
        t = mul(t, [1.0, 0.0, 0.0, 1.0, SIZE as f32 / 16.0, 0.0]);
    }
}

// canvas-style transform multiply: t * m
fn mul(t: [f32; 6], m: [f32; 6]) -> [f32; 6] {
    let [a1, b1, c1, d1, e1, f1] = t;
    let [a2, b2, c2, d2, e2, f2] = m;
    [
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    ]
}

fn copy_source_to_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    pixels: &[u8],
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(SIZE as u32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
    );

    if texture.mip_level_count() > 1 {
        generate_mips(device, queue, texture);
    }
}

struct ObjectInfo {
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

async fn run() {
    let mut app = App::new("WebGPU Simple Textured Quad Import Canvas").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded textured quad shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct OurVertexShaderOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      struct Uniforms {
        matrix: mat4x4f,
      };

      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> OurVertexShaderOutput {
        let pos = array(

          vec2f( 0.0,  0.0),  // center
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 0.0,  1.0),  // center, top

          // 2st triangle
          vec2f( 0.0,  1.0),  // center, top
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 1.0,  1.0),  // right, top
        );

        var vsOutput: OurVertexShaderOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(1, 50);
        return vsOutput;
      }

      @group(0) @binding(0) var ourSampler: sampler;
      @group(0) @binding(1) var ourTexture: texture_2d<f32>;

      @fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
        return textureSample(ourTexture, ourSampler, fsInput.texcoord);
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hardcoded textured quad pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(app.format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    let mut pixels = vec![0u8; SIZE * SIZE * 4];

    let texture = app.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        format: wgpu::TextureFormat::Rgba8Unorm,
        mip_level_count: num_mip_levels(&[SIZE as u32, SIZE as u32]),
        size: wgpu::Extent3d {
            width: SIZE as u32,
            height: SIZE as u32,
            depth_or_array_layers: 1,
        },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let mut object_infos: Vec<ObjectInfo> = Vec::new();
    for i in 0..8 {
        let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: if i & 1 != 0 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            min_filter: if i & 2 != 0 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            mipmap_filter: if i & 4 != 0 {
                wgpu::MipmapFilterMode::Linear
            } else {
                wgpu::MipmapFilterMode::Nearest
            },
            ..Default::default()
        });

        // create a buffer for the uniform values
        let uniform_buffer_size = 16 * 4; // matrix is 16 32bit floats (4bytes each)
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms for quad"),
            size: uniform_buffer_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Save the data we need to render this object.
        object_infos.push(ObjectInfo {
            bind_group,
            uniform_buffer,
        });
    }

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time_ms = frame.time * 1000.0;
        update_2d_canvas(&mut pixels, time_ms);
        copy_source_to_texture(frame.device, frame.queue, &texture, &pixels);

        let fov = 60.0f32.to_radians(); // 60 degrees in radians
        let aspect = frame.width as f32 / frame.height as f32;
        let z_near = 1.0;
        let z_far = 2000.0;
        let projection_matrix = Mat4::perspective_rh(fov, aspect, z_near, z_far);

        let camera_position = Vec3::new(0.0, 0.0, 2.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let view_matrix = Mat4::look_at_rh(camera_position, target, up);
        let view_projection_matrix = projection_matrix * view_matrix;

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render quad encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
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
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);

            for (i, ObjectInfo { bind_group, uniform_buffer }) in object_infos.iter().enumerate() {
                let x_spacing = 1.2;
                let y_spacing = 0.7;
                let z_depth = 50.0;

                let x = (i % 4) as f32 - 1.5;
                let y = if i < 4 { 1.0 } else { -1.0 };

                let matrix = view_projection_matrix
                    * Mat4::from_translation(Vec3::new(
                        x * x_spacing,
                        y * y_spacing,
                        -z_depth * 0.5,
                    ))
                    * Mat4::from_rotation_x(0.5 * std::f32::consts::PI)
                    * Mat4::from_scale(Vec3::new(1.0, z_depth * 2.0, 1.0))
                    * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0));

                // copy the values from Rust to the GPU
                frame.queue.write_buffer(
                    uniform_buffer,
                    0,
                    bytemuck::cast_slice(&matrix.to_cols_array()),
                );

                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1); // call our vertex shader 6 times
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
