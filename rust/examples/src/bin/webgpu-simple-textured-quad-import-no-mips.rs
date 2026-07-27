use wgpu_fun::{App, Frame, ImageData, RenderMode};

async fn run() {
    let mut app = App::new("WebGPU Simple Textured Quad Import Texture (no mips)").await;
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

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> OurVertexShaderOutput {
        let pos = array(
          // 1st triangle
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
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy;
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

    // wgpu's write_texture has no flipY option like the browser's
    // copyExternalImageToTexture, so we flip the rows of pixels ourselves.
    fn flip_image_y(source: &ImageData) -> Vec<u8> {
        let bytes_per_row = (source.width * 4) as usize;
        source
            .data
            .chunks(bytes_per_row)
            .rev()
            .flatten()
            .copied()
            .collect()
    }

    let url = "resources/images/f-texture.png";
    let source = wgpu_fun::load_image(url).await;
    let texture = app.device.create_texture(&wgpu::TextureDescriptor {
        label: Some(url),
        format: wgpu::TextureFormat::Rgba8Unorm,
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    app.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &flip_image_y(&source), // flipY: true
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

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut bind_groups = Vec::new();
    for i in 0..8 {
        let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: if i & 1 != 0 {
                wgpu::AddressMode::Repeat
            } else {
                wgpu::AddressMode::ClampToEdge
            },
            address_mode_v: if i & 2 != 0 {
                wgpu::AddressMode::Repeat
            } else {
                wgpu::AddressMode::ClampToEdge
            },
            mag_filter: if i & 4 != 0 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            ..Default::default()
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
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });
        bind_groups.push(bind_group);
    }

    app.run(RenderMode::Once, move |frame: &Frame| {
        // read the settings the GUI on the page sets
        let address_mode_u = wgpu_fun::setting_str("addressModeU", "repeat");
        let address_mode_v = wgpu_fun::setting_str("addressModeV", "repeat");
        let mag_filter = wgpu_fun::setting_str("magFilter", "linear");

        let ndx = if address_mode_u == "repeat" { 1 } else { 0 }
            + if address_mode_v == "repeat" { 2 } else { 0 }
            + if mag_filter == "linear" { 4 } else { 0 };
        let bind_group = &bind_groups[ndx];

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
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..6, 0..1); // call our vertex shader 6 times
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
