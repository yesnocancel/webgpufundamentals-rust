use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    // ask for the immediates feature (and its size limit) if the adapter
    // supports it
    let mut app = App::new_with_features_and_limits(
        "WebGPU Immediates",
        wgpu::Features::IMMEDIATES,
        |features, limits| wgpu::Limits {
            max_immediate_size: if features.contains(wgpu::Features::IMMEDIATES) {
                limits.max_immediate_size.min(64)
            } else {
                0
            },
            ..wgpu::Limits::default()
        },
    )
    .await;
    // You can probably remove this check by 2027 🙏
    if !app.device.features().contains(wgpu::Features::IMMEDIATES) {
        wgpu_fun::fail("need a browser that supports WebGPU immediates");
        return;
    }
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded triangle with immediates shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct MyImmediates {
        color: vec4f,
        offset: vec2f,
      };

      var<immediate> myImmediates: MyImmediates;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(pos[vertexIndex] + myImmediates.offset, 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return myImmediates.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("our hardcoded immediates triangle pipeline"),
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
            pass.set_immediates(
                0,
                bytemuck::cast_slice(&[
                    1.0f32, 0.0, 0.0, 1.0, // color
                    -0.4, -0.2, // offset
                ]),
            );
            pass.draw(0..3, 0..1);

            pass.set_immediates(
                0,
                bytemuck::cast_slice(&[
                    0.0f32, 1.0, 0.0, 1.0, // color
                    0.4, -0.2, // offset
                ]),
            );
            pass.draw(0..3, 0..1);

            pass.set_immediates(
                0,
                bytemuck::cast_slice(&[
                    0.0f32, 0.0, 1.0, 1.0, // color
                    0.0, 0.2, // offset
                ]),
            );
            pass.draw(0..3, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
