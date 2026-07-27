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
    // 2 triangles per subdivision, 3 verts per tri, 2 values (xy) each.
    let num_vertices = num_subdivisions * 3 * 2;
    let mut vertex_data = vec![0.0f32; (num_subdivisions * 2 * 3 * 2) as usize];

    let mut offset = 0;
    let mut add_vertex = |x: f32, y: f32| {
        vertex_data[offset] = x;
        offset += 1;
        vertex_data[offset] = y;
        offset += 1;
    };

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
        add_vertex(c1 * radius, s1 * radius);
        add_vertex(c2 * radius, s2 * radius);
        add_vertex(c1 * inner_radius, s1 * inner_radius);

        // second triangle
        add_vertex(c1 * inner_radius, s1 * inner_radius);
        add_vertex(c2 * radius, s2 * radius);
        add_vertex(c2 * inner_radius, s2 * inner_radius);
    }

    (vertex_data, num_vertices)
}

struct ObjectInfo {
    scale: f32,
}

async fn run() {
    let mut app = App::new("WebGPU Vertex Buffers with instanced colors").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Vertex {
        @location(0) position: vec2f,
        @location(1) color: vec4f,
        @location(2) offset: vec2f,
        @location(3) scale: vec2f,
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
        vsOut.color = vert.color;
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
            label: Some("flat colors"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 2 * 4, // 2 floats, 4 bytes each
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            // position
                            wgpu::VertexAttribute {
                                shader_location: 0,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 6 * 4, // 6 floats, 4 bytes each
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // color
                            wgpu::VertexAttribute {
                                shader_location: 1,
                                offset: 0,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                            // offset
                            wgpu::VertexAttribute {
                                shader_location: 2,
                                offset: 16,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                        ],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 2 * 4, // 2 floats, 4 bytes each
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // scale
                            wgpu::VertexAttribute {
                                shader_location: 3,
                                offset: 0,
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

    let k_num_objects = 100;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    // create 2 vertex buffers
    let static_unit_size = 4 * 4 + // color is 4 32bit floats (4bytes each)
        2 * 4; // offset is 2 32bit floats (4bytes each)
    let changing_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
    let static_vertex_buffer_size = static_unit_size * k_num_objects;
    let changing_vertex_buffer_size = changing_unit_size * k_num_objects;

    let static_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("static vertex for objects"),
        size: static_vertex_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let changing_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("changing vertex for objects"),
        size: changing_vertex_buffer_size as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // offsets to the various uniform values in float32 indices
    let k_color_offset = 0;
    let k_offset_offset = 4;

    let k_scale_offset = 0;

    {
        let mut static_vertex_values = vec![0.0f32; static_vertex_buffer_size / 4];
        for i in 0..k_num_objects {
            let static_offset = i * (static_unit_size / 4);

            // These are only set once so set them now
            static_vertex_values[static_offset + k_color_offset..][..4].copy_from_slice(&[
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                1.0,
            ]); // set the color
            static_vertex_values[static_offset + k_offset_offset..][..2]
                .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

            object_infos.push(ObjectInfo {
                scale: rand(0.2, 0.5),
            });
        }
        app.queue.write_buffer(
            &static_vertex_buffer,
            0,
            bytemuck::cast_slice(&static_vertex_values),
        );
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

    app.run(RenderMode::Once, move |frame: &Frame| {
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
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
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, static_vertex_buffer.slice(..));
            pass.set_vertex_buffer(2, changing_vertex_buffer.slice(..));

            // Set the uniform values in our Rust side Vec
            let aspect = frame.width as f32 / frame.height as f32;

            // set the scales for each object
            for (ndx, ObjectInfo { scale }) in object_infos.iter().enumerate() {
                let offset = ndx * (changing_unit_size / 4);
                vertex_values[offset + k_scale_offset..][..2]
                    .copy_from_slice(&[scale / aspect, *scale]); // set the scale
            }
            // upload all scales at once
            frame.queue.write_buffer(
                &changing_vertex_buffer,
                0,
                bytemuck::cast_slice(&vertex_values),
            );

            pass.draw(0..num_vertices, 0..k_num_objects as u32);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
