use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let mut app = App::new("webgpu-multisample-centroid").await;
    app.auto_resize = true;
    app.resize_divisor = 16;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded red triangle shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct VOut {
        @builtin(position) position: vec4f,
        @location(0) @interpolate(perspective, centroid) baryCoord: vec3f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> VOut {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );
        let bary = array(
          vec3f(1, 0, 0),
          vec3f(0, 1, 0),
          vec3f(0, 0, 1),
        );
        var vout: VOut;
        vout.position = vec4f(pos[vertexIndex], 0.0, 1.0);
        vout.baryCoord = bary[vertexIndex];
        return vout;
      }

      @fragment fn fs(vin: VOut) -> @location(0) vec4f {
        let allAbove0 = all(vin.baryCoord >= vec3f(0));
        let allBelow1 = all(vin.baryCoord <= vec3f(1));
        let inside = allAbove0 && allBelow1;
        let red = vec4f(1, 0, 0, 1);
        let yellow = vec4f(1, 1, 0, 1);
        return select(yellow, red, inside);
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
            multisample: wgpu::MultisampleState {
                count: 4,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });

    let mut multisample_texture: Option<wgpu::Texture> = None;

    app.run(RenderMode::Once, move |frame: &Frame| {
        // If the multisample texture doesn't exist or
        // is the wrong size then make a new one.
        if multisample_texture
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            // If we have an existing multisample texture destroy it.
            if let Some(texture) = multisample_texture.take() {
                texture.destroy();
            }

            // Create a new multisample texture that matches our
            // canvas's size
            multisample_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                format: frame.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                sample_count: 4,
                mip_level_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }

        let multisample_view = multisample_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("our encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // Render to the multisample texture...
                    view: &multisample_view,
                    // ...and "resolve" it to the canvas texture.
                    resolve_target: Some(frame.view),
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
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
