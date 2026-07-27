use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let mut app = App::new("WebGPU Misc - instance_index").await;
    // The drawing buffer follows the canvas/window size, like the
    // ResizeObserver in the original.
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded red triangle shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
        struct VertexOut {
          @builtin(position) pos: vec4f,
          @location(0) @interpolate(flat, either) colorNdx: u32,
        };

        @vertex fn vs(
          @builtin(vertex_index) vertexIndex : u32,
          @builtin(instance_index) instanceIndex: u32,
        ) -> VertexOut {
          let pos = array(
            vec2f( 0.0,  0.5),  // top center
            vec2f(-0.5, -0.5),  // bottom left
            vec2f( 0.5, -0.5)   // bottom right
          );
          let offsets = array(
            vec2f( 0.0,  0.5),  // top middle
            vec2f(-0.5, -0.5),  // left bottom
            vec2f( 0.5, -0.5),  // right bottom
          );

          return VertexOut(
            vec4f(pos[vertexIndex] + offsets[instanceIndex], 0.0, 1.0),
            instanceIndex,
          );
        }

        @fragment fn fs(in: VertexOut) -> @location(0) vec4f {
          let colors = array(
            vec4f(1, 1, 0, 1),  // yellow
            vec4f(0, 1, 1, 1),  // cyan
            vec4f(1, 0, 1, 1),  // magenta
          );
          return colors[in.colorNdx];
        }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("our hardcoded red triangle pipeline"),
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

    app.run(RenderMode::Once, move |frame: &Frame| {
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("our encoder"),
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
            pass.draw(0..3, 0..1); // call our vertex shader 3 times
            pass.draw(0..3, 1..2); // pass 1 for instance_index
            pass.draw(0..3, 2..3); // pass 2 for instance_index
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
