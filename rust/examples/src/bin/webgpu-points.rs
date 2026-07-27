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

async fn run() {
    let mut app = App::new("WebGPU Points").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Vertex {
        @location(0) position: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
      };

      @vertex fn vs(vert: Vertex,) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = vec4f(vert.position, 0, 1);
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return vec4f(1, 1, 0, 1); // yellow
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("1 pixel points"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
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
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(app.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    const K_NUM_POINTS: usize = 100;
    let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 2];
    for i in 0..K_NUM_POINTS {
        let offset = i * 2;
        vertex_data[offset] = rand(-1.0, 1.0);
        vertex_data[offset + 1] = rand(-1.0, 1.0);
    }

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
            pass.draw(0..K_NUM_POINTS as u32, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
