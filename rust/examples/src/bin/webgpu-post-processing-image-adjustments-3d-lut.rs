use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, ImageData, RenderMode};

fn create_texture_from_source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &ImageData,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        format: wgpu::TextureFormat::Rgba8Unorm,
        mip_level_count: 1,
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &source.data,
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
    texture
}

async fn create_texture_from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    url: &str,
) -> wgpu::Texture {
    let source = wgpu_fun::load_image(url).await;
    create_texture_from_source(device, queue, &source)
}

fn make_identity_lut_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 2,
        },
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1,
        sample_count: 1,
        view_formats: &[],
    });

    #[rustfmt::skip]
    let identity_lut: [u8; 32] = [
        0,   0,   0, 255,  // black
      255,   0,   0, 255,  // red
        0, 255,   0, 255,  // green
      255, 255,   0, 255,  // yellow
        0,   0, 255, 255,  // blue
      255,   0, 255, 255,  // magenta
        0, 255, 255, 255,  // cyan
      255, 255, 255, 255,  // white
    ];

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &identity_lut,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 2,
        },
    );

    texture
}

async fn run() {
    let mut app = App::new("WebGPU Post Processing - Step 8 - 3D-LUT").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      struct Uniforms {
        matrix: mat4x4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;
      @group(0) @binding(1) var tex: texture_2d<f32>;
      @group(0) @binding(2) var smp: sampler;

      @vertex fn vs(@builtin(vertex_index) vNdx: u32) -> VSOutput {
        let positions = array(
          vec2f( 0,  0),
          vec2f( 1,  0),
          vec2f( 0,  1),
          vec2f( 0,  1),
          vec2f( 1,  0),
          vec2f( 1,  1),
        );
        let pos = positions[vNdx];
        return VSOutput(
          uni.matrix * vec4f(pos, 0, 1),
          pos,
        );
      }

      @fragment fn fs(fsInput: VSOutput) -> @location(0) vec4f {
        return textureSample(tex, smp, fsInput.texcoord);
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("textured unit quad"),
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
                targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    let image_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4 * 16, // mat4x4
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let image_texture = create_texture_from_image(
        &app.device,
        &app.queue,
        "resources/images/david-clode-clown-fish.jpg",
    )
    .await;

    let image_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let image_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: image_uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &image_texture.create_view(&Default::default()),
                ),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&image_sampler),
            },
        ],
    });

    let post_process_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      fn apply3DLUT(
          color: vec3f,
          lut: texture_3d<f32>,
          smp: sampler) -> vec3f {
        let size = vec3f(textureDimensions(lut, 0));
        let range = (size - 1) / size;
        let uvw = 0.5 / size + color * range;
        return textureSample(lut, smp, uvw).rgb;
      }

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5) + vec2f(0.5);
        return vsOutput;
      }

      struct Uniforms {
        lutAmount: f32,
      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;
      @group(1) @binding(0) var lut: texture_3d<f32>;
      @group(1) @binding(1) var lutSampler: sampler;

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
        var rgb = color.rgb;
        rgb = mix(rgb, apply3DLUT(rgb, lut, lutSampler), uni.lutAmount);
        return vec4f(rgb, color.a);
      }
    "#
                .into(),
            ),
        });

    let post_process_pipeline =
        app.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None,
                vertex: wgpu::VertexState {
                    module: &post_process_module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_process_module,
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

    let post_process_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let post_process_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let lut_nearest_sampler = app.device.create_sampler(&Default::default());
    let lut_linear_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let make_lut_bind_group = |texture: &wgpu::Texture, sampler: &wgpu::Sampler| {
        app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &post_process_pipeline.get_bind_group_layout(1),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    };

    let identity_lut_texture = make_identity_lut_texture(&app.device, &app.queue);
    // (name, bindGroup) pairs; the names fill the GUI dropdown on the page
    let lut_bind_groups: Vec<(&str, wgpu::BindGroup)> = vec![
        (
            "identity",
            make_lut_bind_group(&identity_lut_texture, &lut_linear_sampler),
        ),
        (
            "identity (nearest)",
            make_lut_bind_group(&identity_lut_texture, &lut_nearest_sampler),
        ),
    ];

    let mut render_target: Option<wgpu::Texture> = None;
    let mut post_process_bind_group: Option<wgpu::BindGroup> = None;

    app.run(RenderMode::Once, move |frame: &Frame| {
        // If we don't have a render target or it doesn't match the canvas
        // size, make a new one (setupPostProcess in the JS version).
        if render_target
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            if let Some(t) = render_target.take() {
                t.destroy();
            }
            let texture = frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            });
            post_process_bind_group =
                Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &post_process_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                &texture.create_view(&Default::default()),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&post_process_sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: post_process_uniform_buffer.as_entire_binding(),
                        },
                    ],
                }));
            render_target = Some(texture);
        }
        let render_target_view = render_target
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        // css 'cover'
        let canvas_aspect = frame.width as f32 / frame.height as f32;
        let image_aspect = image_texture.width() as f32 / image_texture.height() as f32;
        let aspect = canvas_aspect / image_aspect;
        let aspect_scale = if aspect > 1.0 {
            Vec3::new(1.0, aspect, 1.0)
        } else {
            Vec3::new(1.0 / aspect, 1.0, 1.0)
        };

        let matrix = Mat4::from_scale(aspect_scale)
            * Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0))
            * Mat4::from_translation(Vec3::new(-0.5, -0.5, 1.0));

        // Copy our the uniform values to the GPU
        frame.queue.write_buffer(
            &image_uniform_buffer,
            0,
            bytemuck::cast_slice(&matrix.to_cols_array()),
        );

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_target_view,
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
            pass.set_bind_group(0, &image_bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        // read the settings the GUI on the page sets
        let lut_amount = wgpu_fun::setting_f64("lutAmount", 1.0) as f32;
        let lut = wgpu_fun::setting_f64("lut", 0.0) as usize % lut_bind_groups.len();
        frame.queue.write_buffer(
            &post_process_uniform_buffer,
            0,
            bytemuck::cast_slice(&[lut_amount]),
        );

        // post process the render target to the canvas
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post process render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&post_process_pipeline);
            pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
            pass.set_bind_group(1, &lut_bind_groups[lut].1, &[]);
            pass.draw(0..3, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
