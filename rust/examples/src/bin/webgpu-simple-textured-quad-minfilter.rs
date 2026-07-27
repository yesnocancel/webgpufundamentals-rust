use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let mut app = App::new("WebGPU Simple Textured Quad MinFilter").await;
    app.auto_resize = true;
    app.resize_divisor = 64;

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
        scale: vec2f,
        offset: vec2f,
      };

      @group(0) @binding(2) var<uniform> uni: Uniforms;

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
        vsOutput.position = vec4f(xy * uni.scale + uni.offset, 0.0, 1.0);
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

    const K_TEXTURE_WIDTH: u32 = 5;
    const K_TEXTURE_HEIGHT: u32 = 7;
    let r: [u8; 4] = [255, 0, 0, 255]; // red
    let y: [u8; 4] = [255, 255, 0, 255]; // yellow
    let b: [u8; 4] = [0, 0, 255, 255]; // blue
    #[rustfmt::skip]
    let texture_data = [
        r, r, r, r, r,
        r, y, r, r, r,
        r, y, r, r, r,
        r, y, y, r, r,
        r, y, r, r, r,
        r, y, y, y, r,
        b, r, r, r, r,
    ]
    .concat();

    let texture = app.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("yellow F on red"),
        size: wgpu::Extent3d {
            width: K_TEXTURE_WIDTH,
            height: K_TEXTURE_HEIGHT,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
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
        &texture_data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(K_TEXTURE_WIDTH * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: K_TEXTURE_WIDTH,
            height: K_TEXTURE_HEIGHT,
            depth_or_array_layers: 1,
        },
    );

    // create a buffer for the uniform values
    const UNIFORM_BUFFER_SIZE: u64 = 2 * 4 + // scale is 2 32bit floats (4bytes each)
        2 * 4; // offset is 2 32bit floats (4bytes each)
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms for quad"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // create an array of f32s to hold the values for the uniforms in Rust
    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_SCALE_OFFSET: usize = 0;
    const K_OFFSET_OFFSET: usize = 2;

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut bind_groups = Vec::new();
    for i in 0..16 {
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
            min_filter: if i & 8 != 0 {
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        bind_groups.push(bind_group);
    }

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time = frame.time as f32;

        // read the settings the GUI on the page sets
        let address_mode_u = wgpu_fun::setting_str("addressModeU", "repeat");
        let address_mode_v = wgpu_fun::setting_str("addressModeV", "repeat");
        let mag_filter = wgpu_fun::setting_str("magFilter", "linear");
        let min_filter = wgpu_fun::setting_str("minFilter", "linear");

        let ndx = if address_mode_u == "repeat" { 1 } else { 0 }
            + if address_mode_v == "repeat" { 2 } else { 0 }
            + if mag_filter == "linear" { 4 } else { 0 }
            + if min_filter == "linear" { 8 } else { 0 };
        let bind_group = &bind_groups[ndx];

        // compute a scale that will draw our 0 to 1 clip space quad
        // 2x2 pixels in the canvas.
        let scale_x = 4.0 / frame.width as f32;
        let scale_y = 4.0 / frame.height as f32;

        uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2].copy_from_slice(&[scale_x, scale_y]); // set the scale
        uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2]
            .copy_from_slice(&[(time * 0.5).sin() * 0.8, -0.8]); // set the offset

        // copy the values from Rust to the GPU
        frame
            .queue
            .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));

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
