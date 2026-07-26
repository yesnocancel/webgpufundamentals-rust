use wgpu_fun::{App, Frame, RenderMode};

// A random number between [min and max)
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

struct CircleVerticesOptions {
    radius: f32,
    num_subdivisions: u32,
    inner_radius: f32,
    start_angle: f32,
    end_angle: f32,
}

impl Default for CircleVerticesOptions {
    fn default() -> Self {
        Self {
            radius: 1.0,
            num_subdivisions: 24,
            inner_radius: 0.0,
            start_angle: 0.0,
            end_angle: std::f32::consts::PI * 2.0,
        }
    }
}

fn create_circle_vertices(options: CircleVerticesOptions) -> (Vec<f32>, u32) {
    let CircleVerticesOptions {
        radius,
        num_subdivisions,
        inner_radius,
        start_angle,
        end_angle,
    } = options;
    // 2 triangles per subdivision, 3 verts per tri
    let num_vertices = num_subdivisions * 3 * 2;
    // 2 32-bit values for position (xy) and 1 32-bit value for color (rgb_)
    // The 32-bit color value will be written/read as 4 8-bit values
    let mut vertex_data = vec![0.0f32; (num_vertices * (2 + 1)) as usize];

    let mut offset = 0;
    let mut color_offset = 8;
    let mut add_vertex = |x: f32, y: f32, [r, g, b]: [f32; 3]| {
        vertex_data[offset] = x;
        offset += 1;
        vertex_data[offset] = y;
        offset += 1;
        offset += 1; // skip the color

        // a u8 view of the same data as vertex_data
        let color_data: &mut [u8] = bytemuck::cast_slice_mut(&mut vertex_data);
        color_data[color_offset] = (r * 255.0) as u8;
        color_offset += 1;
        color_data[color_offset] = (g * 255.0) as u8;
        color_offset += 1;
        color_data[color_offset] = (b * 255.0) as u8;
        color_offset += 1;
        color_offset += 9; // skip extra byte and the position
    };

    let inner_color = [1.0, 1.0, 1.0];
    let outer_color = [0.1, 0.1, 0.1];

    // 2 vertices per subdivision
    //
    // 0--1 4
    // | / /|
    // |/ / |
    // 2 3--5
    for i in 0..num_subdivisions {
        let angle1 =
            start_angle + (i + 0) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
        let angle2 =
            start_angle + (i + 1) as f32 * (end_angle - start_angle) / num_subdivisions as f32;

        let c1 = angle1.cos();
        let s1 = angle1.sin();
        let c2 = angle2.cos();
        let s2 = angle2.sin();

        // first triangle
        add_vertex(c1 * radius, s1 * radius, outer_color);
        add_vertex(c2 * radius, s2 * radius, outer_color);
        add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);

        // second triangle
        add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);
        add_vertex(c2 * radius, s2 * radius, outer_color);
        add_vertex(c2 * inner_radius, s2 * inner_radius, inner_color);
    }

    (vertex_data, num_vertices)
}

struct ObjectInfo {
    scale: f32,
    offset: [f32; 2],
    velocity: [f32; 2],
}

async fn run() {
    let mut app = App::new("WebGPU Post Processing - Step 02 - scanlines").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct Vertex {
        @location(0) position: vec2f,
        @location(1) color: vec4f,
        @location(2) offset: vec2f,
        @location(3) scale: vec2f,
        @location(4) perVertexColor: vec3f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) color: vec4f,
      };

      @vertex fn vs(
        vert: Vertex,
      ) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = vec4f(
            vert.position * vert.scale + vert.offset, 0.0, 1.0);
        vsOut.color = vert.color * vec4f(vert.perVertexColor, 1);
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return vsOut.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("per vertex color"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 2 * 4 + 4, // 2 floats, 4 bytes each + 4 bytes
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            // position
                            wgpu::VertexAttribute {
                                shader_location: 0,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            // perVertexColor
                            wgpu::VertexAttribute {
                                shader_location: 4,
                                offset: 8,
                                format: wgpu::VertexFormat::Unorm8x4,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 4, // 4 bytes
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // color
                            wgpu::VertexAttribute {
                                shader_location: 1,
                                offset: 0,
                                format: wgpu::VertexFormat::Unorm8x4,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 4 * 4, // 4 floats, 4 bytes each
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // offset
                            wgpu::VertexAttribute {
                                shader_location: 2,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            // scale
                            wgpu::VertexAttribute {
                                shader_location: 3,
                                offset: 8,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    }),
                ],
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

    let post_process_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
        return vsOutput;
      }

      struct Uniforms {
        effectAmount: f32,
        bandMult: f32,
      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
        let banding = abs(sin(fsInput.position.y * uni.bandMult));
        let effect = mix(1.0, banding, uni.effectAmount);

        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
        return vec4f(color.rgb * effect, color.a);
      }
    "#
                .into(),
            ),
        });

    let post_process_pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &post_process_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_process_module,
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

    let post_process_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let post_process_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 8,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut render_target: Option<wgpu::Texture> = None;
    let mut post_process_bind_group: Option<wgpu::BindGroup> = None;

    let k_num_objects = 10000;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    // create 2 vertex buffers
    let static_unit_size = 4; // color is 4 bytes
    let changing_unit_size = 2 * 4 + // offset is 2 32bit floats (4bytes each)
        2 * 4; // scale is 2 32bit floats (4bytes each)
    let static_vertex_buffer_size = static_unit_size * k_num_objects;
    let changing_vertex_buffer_size = changing_unit_size * k_num_objects;

    let static_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("static vertex for objects"),
        size: static_vertex_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let changing_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("changing storage for objects"),
        size: changing_vertex_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // offsets to the various uniform values in float32 indices
    let k_color_offset = 0;

    let k_offset_offset = 0;
    let k_scale_offset = 2;

    {
        let mut static_vertex_values_u8 = vec![0u8; static_vertex_buffer_size];
        for i in 0..k_num_objects {
            let static_offset_u8 = i * static_unit_size;

            // These are only set once so set them now
            static_vertex_values_u8[static_offset_u8 + k_color_offset..][..4].copy_from_slice(&[
                (rand(0.0, 1.0) * 255.0) as u8,
                (rand(0.0, 1.0) * 255.0) as u8,
                (rand(0.0, 1.0) * 255.0) as u8,
                255,
            ]); // set the color

            object_infos.push(ObjectInfo {
                scale: rand(0.2, 0.5),
                offset: [rand(-0.9, 0.9), rand(-0.9, 0.9)],
                velocity: [rand(-0.1, 0.1), rand(-0.1, 0.1)],
            });
        }
        app.queue
            .write_buffer(&static_vertex_buffer, 0, &static_vertex_values_u8);
    }

    // a Vec we can use to update the changingVertexBuffer
    let mut vertex_values = vec![0.0f32; changing_vertex_buffer_size / 4];

    let (vertex_data, num_vertices) = create_circle_vertices(CircleVerticesOptions {
        radius: 0.5,
        inner_radius: 0.25,
        ..Default::default()
    });
    let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer vertices"),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

    let euclidean_modulo = |x: f32, a: f32| x - a * (x / a).floor();

    let mut then = 0.0;
    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let now = frame.time;
        let delta_time = (now - then) as f32;
        then = now;

        // read the settings the GUI on the page sets
        let num_objects = wgpu_fun::setting_f64("numObjects", 200.0) as usize;

        // If we don't have a render target or it doesn't match the canvas
        // size, make a new one (setupPostProcess in the JS version).
        if render_target
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            if let Some(t) = render_target.take() {
                t.destroy();
            }
            let texture = frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            });
            post_process_bind_group =
                Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &post_process_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &texture.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&post_process_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: post_process_uniform_buffer.as_entire_binding(),
                        },
                    ],
                }));
            render_target = Some(texture);
        }
        let render_target_view = render_target
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_target_view,
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
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, static_vertex_buffer.slice(..));
            pass.set_vertex_buffer(2, changing_vertex_buffer.slice(..));

            // Set the uniform values in our Rust side Vec
            let aspect = frame.width as f32 / frame.height as f32;

            // set the scales for each object
            for ndx in 0..num_objects {
                let ObjectInfo {
                    scale,
                    offset,
                    velocity,
                } = &mut object_infos[ndx];
                // -1.5 to 1.5
                offset[0] = euclidean_modulo(offset[0] + velocity[0] * delta_time + 1.5, 3.0) - 1.5;
                offset[1] = euclidean_modulo(offset[1] + velocity[1] * delta_time + 1.5, 3.0) - 1.5;

                let off = ndx * (changing_unit_size / 4);
                vertex_values[off + k_offset_offset..][..2].copy_from_slice(offset);
                vertex_values[off + k_scale_offset..][..2]
                    .copy_from_slice(&[*scale / aspect, *scale]);
            }

            // upload all offsets and scales at once
            frame.queue.write_buffer(
                &changing_vertex_buffer,
                0,
                bytemuck::cast_slice(&vertex_values[..num_objects * (changing_unit_size / 4)]),
            );

            pass.draw(0..num_vertices, 0..num_objects as u32);
        }

        // read the settings the GUI on the page sets
        let affect_amount = wgpu_fun::setting_f64("affectAmount", 1.0) as f32;
        let band_mult = wgpu_fun::setting_f64("bandMult", 1.0) as f32;
        frame.queue.write_buffer(
            &post_process_uniform_buffer,
            0,
            bytemuck::cast_slice(&[affect_amount, band_mult]),
        );

        // post process the render target to the canvas
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post process render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&post_process_pipeline);
            pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
            pass.draw(0..3, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
