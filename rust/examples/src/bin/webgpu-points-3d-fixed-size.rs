use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, RenderMode};

struct FibonacciSphereOptions {
    num_samples: usize,
    radius: f32,
}

fn create_fibonacci_sphere_vertices(
    FibonacciSphereOptions {
        num_samples,
        radius,
    }: FibonacciSphereOptions,
) -> Vec<f32> {
    let mut vertices = Vec::new();
    let increment = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    for i in 0..num_samples {
        let offset = 2.0 / num_samples as f32;
        let y = ((i as f32 * offset) - 1.0) + (offset / 2.0);
        let r = (1.0 - y * y).sqrt();
        let phi = (i % num_samples) as f32 * increment;
        let x = phi.cos() * r;
        let z = phi.sin() * r;
        vertices.extend_from_slice(&[x * radius, y * radius, z * radius]);
    }
    vertices
}

async fn run() {
    let mut app = App::new("WebGPU Points 3D fixed size").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Vertex {
        @location(0) position: vec4f,
      };

      struct Uniforms {
        matrix: mat4x4f,
        resolution: vec2f,
        size: f32,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;

      @vertex fn vs(
          vert: Vertex,
          @builtin(vertex_index) vNdx: u32,
      ) -> VSOutput {
        let points = array(
          vec2f(-1, -1),
          vec2f( 1, -1),
          vec2f(-1,  1),
          vec2f(-1,  1),
          vec2f( 1, -1),
          vec2f( 1,  1),
        );
        var vsOut: VSOutput;
        let pos = points[vNdx];
        let clipPos = uni.matrix * vert.position;
        let pointPos = vec4f(pos * uni.size / uni.resolution * clipPos.w, 0, 0);
        vsOut.position = clipPos + pointPos;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return vec4f(1, 0.5, 0.2, 1);
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3d points with fixed size"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 3 * 4, // 3 floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                })],
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

    let vertex_data = create_fibonacci_sphere_vertices(FibonacciSphereOptions {
        radius: 1.0,
        num_samples: 1000,
    });
    let k_num_points = (vertex_data.len() / 3) as u32;

    let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer vertices"),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

    // matrix, resolution, size, padding
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (16 + 2 + 1 + 1) * 4,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time = frame.time as f32;

        // Set the matrix in the uniform buffer
        let fov = 90.0f32.to_radians();
        let aspect = frame.width as f32 / frame.height as f32;
        let projection = Mat4::perspective_rh(fov, aspect, 0.1, 50.0);
        let view = Mat4::look_at_rh(
            Vec3::new(0.0, 0.0, 1.5), // position
            Vec3::new(0.0, 0.0, 0.0), // target
            Vec3::new(0.0, 1.0, 0.0), // up
        );
        let view_projection = projection * view;
        let matrix =
            view_projection * Mat4::from_rotation_y(time) * Mat4::from_rotation_x(time * 0.5);

        // Copy the uniform values to the GPU
        let mut uniform_values = [0.0f32; 16 + 2 + 1 + 1];
        uniform_values[0..16].copy_from_slice(&matrix.to_cols_array());
        // Update the resolution in the uniform buffer
        uniform_values[16..18].copy_from_slice(&[frame.width as f32, frame.height as f32]);
        // Set the size in the uniform buffer
        uniform_values[18] = 10.0;
        frame
            .queue
            .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));

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
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..k_num_points);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
