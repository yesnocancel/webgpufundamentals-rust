use wgpu_fun::{App, Frame, RenderMode};

/// One mip level: tightly packed rgba8unorm pixels.
struct Mip {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    std::array::from_fn(|i| lerp(a[i], b[i], t))
}

fn bilinear_filter(
    tl: [f32; 4],
    tr: [f32; 4],
    bl: [f32; 4],
    br: [f32; 4],
    t1: f32,
    t2: f32,
) -> [f32; 4] {
    let t = mix(tl, tr, t1);
    let b = mix(bl, br, t1);
    mix(t, b, t2)
}

fn create_next_mip_level_rgba8_unorm(
    Mip {
        data: src,
        width: src_width,
        height: src_height,
    }: &Mip,
) -> Mip {
    // compute the size of the next mip
    let dst_width = 1.max(src_width / 2);
    let dst_height = 1.max(src_height / 2);
    let mut dst = vec![0u8; (dst_width * dst_height * 4) as usize];

    let get_src_pixel = |x: u32, y: u32| -> [f32; 4] {
        let offset = ((y * src_width + x) * 4) as usize;
        std::array::from_fn(|i| src[offset + i] as f32)
    };

    for y in 0..dst_height {
        for x in 0..dst_width {
            // compute texcoord of the center of the destination texel
            let u = (x as f32 + 0.5) / dst_width as f32;
            let v = (y as f32 + 0.5) / dst_height as f32;

            // compute the same texcoord in the source - 0.5 a pixel
            let au = u * *src_width as f32 - 0.5;
            let av = v * *src_height as f32 - 0.5;

            // compute the src top left texel coord (not texcoord)
            let tx = au as u32;
            let ty = av as u32;

            // compute the mix amounts between pixels
            let t1 = au % 1.0;
            let t2 = av % 1.0;

            // get the 4 pixels
            let tl = get_src_pixel(tx, ty);
            let tr = get_src_pixel(tx + 1, ty);
            let bl = get_src_pixel(tx, ty + 1);
            let br = get_src_pixel(tx + 1, ty + 1);

            // copy the "sampled" result into the dest.
            let dst_offset = ((y * dst_width + x) * 4) as usize;
            let sampled = bilinear_filter(tl, tr, bl, br, t1, t2);
            for (d, s) in dst[dst_offset..dst_offset + 4].iter_mut().zip(sampled) {
                *d = s as u8;
            }
        }
    }
    Mip {
        data: dst,
        width: dst_width,
        height: dst_height,
    }
}

fn generate_mips(src: Vec<u8>, src_width: u32) -> Vec<Mip> {
    let src_height = src.len() as u32 / 4 / src_width;

    // populate with first mip level (base level)
    let mut mips = vec![Mip {
        data: src,
        width: src_width,
        height: src_height,
    }];

    while mips.last().unwrap().width > 1 || mips.last().unwrap().height > 1 {
        let mip = create_next_mip_level_rgba8_unorm(mips.last().unwrap());
        mips.push(mip);
    }
    mips
}

async fn run() {
    let mut app = App::new("WebGPU Simple Textured Quad Mipmap").await;
    app.auto_resize = true;
    app.resize_divisor = 64;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded textured quad shaders"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
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

    let mips = generate_mips(texture_data, K_TEXTURE_WIDTH);

    let texture = app.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("yellow F on red"),
        size: wgpu::Extent3d {
            width: mips[0].width,
            height: mips[0].height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (mip_level, Mip { data, width, height }) in mips.iter().enumerate() {
        app.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip_level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: *width,
                height: *height,
                depth_or_array_layers: 1,
            },
        );
    }

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
        let scale = wgpu_fun::setting_f64("scale", 1.0) as f32;

        let ndx = if address_mode_u == "repeat" { 1 } else { 0 }
            + if address_mode_v == "repeat" { 2 } else { 0 }
            + if mag_filter == "linear" { 4 } else { 0 }
            + if min_filter == "linear" { 8 } else { 0 };
        let bind_group = &bind_groups[ndx];

        let scale_x = 4.0 / frame.width as f32 * scale;
        let scale_y = 4.0 / frame.height as f32 * scale;

        uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2].copy_from_slice(&[scale_x, scale_y]); // set the scale
        uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2]
            .copy_from_slice(&[(time * 0.25).sin() * 0.9, -0.8]); // set the offset

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
