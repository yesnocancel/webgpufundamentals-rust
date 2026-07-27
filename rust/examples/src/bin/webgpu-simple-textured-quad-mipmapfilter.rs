use glam::{vec3, Mat4};
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

fn create_blended_mipmap() -> Vec<Mip> {
    let w: [u8; 4] = [255, 255, 255, 255];
    let r: [u8; 4] = [255, 0, 0, 255];
    let b: [u8; 4] = [0, 28, 116, 255];
    let y: [u8; 4] = [255, 231, 0, 255];
    let g: [u8; 4] = [58, 181, 75, 255];
    let a: [u8; 4] = [38, 123, 167, 255];
    #[rustfmt::skip]
    let data = [
        w, r, r, r, r, r, r, a, a, r, r, r, r, r, r, w,
        w, w, r, r, r, r, r, a, a, r, r, r, r, r, w, w,
        w, w, w, r, r, r, r, a, a, r, r, r, r, w, w, w,
        w, w, w, w, r, r, r, a, a, r, r, r, w, w, w, w,
        w, w, w, w, w, r, r, a, a, r, r, w, w, w, w, w,
        w, w, w, w, w, w, r, a, a, r, w, w, w, w, w, w,
        w, w, w, w, w, w, w, a, a, w, w, w, w, w, w, w,
        b, b, b, b, b, b, b, b, a, y, y, y, y, y, y, y,
        b, b, b, b, b, b, b, g, y, y, y, y, y, y, y, y,
        w, w, w, w, w, w, w, g, g, w, w, w, w, w, w, w,
        w, w, w, w, w, w, r, g, g, r, w, w, w, w, w, w,
        w, w, w, w, w, r, r, g, g, r, r, w, w, w, w, w,
        w, w, w, w, r, r, r, g, g, r, r, r, w, w, w, w,
        w, w, w, r, r, r, r, g, g, r, r, r, r, w, w, w,
        w, w, r, r, r, r, r, g, g, r, r, r, r, r, w, w,
        w, r, r, r, r, r, r, g, g, r, r, r, r, r, r, w,
    ]
    .concat();
    generate_mips(data, 16)
}

// The JS version draws these mip levels with the canvas 2d api. We fill
// the same rectangles ourselves.
fn create_checked_mipmap() -> Vec<Mip> {
    let fill_rect = |data: &mut [u8], size: u32, x: u32, y: u32, w: u32, h: u32, color: [u8; 4]| {
        for py in y..y + h {
            for px in x..x + w {
                let offset = ((py * size + px) * 4) as usize;
                data[offset..offset + 4].copy_from_slice(&color);
            }
        }
    };
    let levels: [(u32, [u8; 4]); 7] = [
        (64, [128, 0, 255, 255]),
        (32, [0, 255, 0, 255]),
        (16, [255, 0, 0, 255]),
        (8, [255, 255, 0, 255]),
        (4, [0, 0, 255, 255]),
        (2, [0, 255, 255, 255]),
        (1, [255, 0, 255, 255]),
    ];
    levels
        .iter()
        .enumerate()
        .map(|(i, &(size, color))| {
            let mut data = vec![0u8; (size * size * 4) as usize];
            let background: [u8; 4] = if i & 1 != 0 {
                [0, 0, 0, 255] // '#000'
            } else {
                [255, 255, 255, 255] // '#fff'
            };
            fill_rect(&mut data, size, 0, 0, size, size, background);
            fill_rect(&mut data, size, 0, 0, size / 2, size / 2, color);
            fill_rect(&mut data, size, size / 2, size / 2, size / 2, size / 2, color);
            Mip {
                data,
                width: size,
                height: size,
            }
        })
        .collect()
}

fn create_texture_with_mips(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mips: Vec<Mip>,
    label: &str,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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
        queue.write_texture(
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
    texture
}

async fn run() {
    let mut app = App::new("WebGPU Simple Textured Quad MipFilter").await;
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

      struct Uniforms {
        matrix: mat4x4f,
      };

      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> OurVertexShaderOutput {
        let pos = array(

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
        vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(1, 50);
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

    let textures = [
        create_texture_with_mips(&app.device, &app.queue, create_blended_mipmap(), "blended"),
        create_texture_with_mips(&app.device, &app.queue, create_checked_mipmap(), "checker"),
    ];
    let texture_views: Vec<wgpu::TextureView> = textures
        .iter()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        .collect();

    struct ObjectInfo {
        bind_groups: Vec<wgpu::BindGroup>,
        uniform_buffer: wgpu::Buffer,
    }

    let mut object_infos: Vec<ObjectInfo> = Vec::new();
    for i in 0..8 {
        let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: if i & 1 != 0 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            min_filter: if i & 2 != 0 {
                wgpu::FilterMode::Linear
            } else {
                wgpu::FilterMode::Nearest
            },
            mipmap_filter: if i & 4 != 0 {
                wgpu::MipmapFilterMode::Linear
            } else {
                wgpu::MipmapFilterMode::Nearest
            },
            ..Default::default()
        });

        // create a buffer for the uniform values
        const UNIFORM_BUFFER_SIZE: u64 = 16 * 4; // matrix is 16 32bit floats (4bytes each)
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms for quad"),
            size: UNIFORM_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_groups = texture_views
            .iter()
            .map(|texture_view| {
                app.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: uniform_buffer.as_entire_binding(),
                        },
                    ],
                })
            })
            .collect();

        // Save the data we need to render this object.
        object_infos.push(ObjectInfo {
            bind_groups,
            uniform_buffer,
        });
    }

    app.run(RenderMode::Once, move |frame: &Frame| {
        // clicking the canvas cycles this through the textures (see the
        // page's click handler)
        let tex_ndx = wgpu_fun::setting_f64("texNdx", 0.0) as usize % textures.len();

        let fov = 60.0f32.to_radians(); // 60 degrees in radians
        let aspect = frame.width as f32 / frame.height as f32;
        let z_near = 1.0;
        let z_far = 2000.0;
        let projection_matrix = Mat4::perspective_rh(fov, aspect, z_near, z_far);

        let camera_position = vec3(0.0, 0.0, 2.0);
        let up = vec3(0.0, 1.0, 0.0);
        let target = vec3(0.0, 0.0, 0.0);
        let view_matrix = Mat4::look_at_rh(camera_position, target, up);
        let view_projection_matrix = projection_matrix * view_matrix;

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

            for (i, ObjectInfo { bind_groups, uniform_buffer }) in object_infos.iter().enumerate() {
                let bind_group = &bind_groups[tex_ndx];

                let x_spacing = 1.2;
                let y_spacing = 0.7;
                let z_depth = 50.0;

                let x = (i % 4) as f32 - 1.5;
                let y = if i < 4 { 1.0 } else { -1.0 };

                let matrix = view_projection_matrix
                    * Mat4::from_translation(vec3(x * x_spacing, y * y_spacing, -z_depth * 0.5))
                    * Mat4::from_rotation_x(0.5 * std::f32::consts::PI)
                    * Mat4::from_scale(vec3(1.0, z_depth * 2.0, 1.0))
                    * Mat4::from_translation(vec3(-0.5, -0.5, 0.0));

                // copy the values from Rust to the GPU
                frame.queue.write_buffer(
                    uniform_buffer,
                    0,
                    bytemuck::cast_slice(&matrix.to_cols_array()),
                );

                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..6, 0..1); // call our vertex shader 6 times
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
