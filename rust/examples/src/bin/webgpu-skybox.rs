use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, ImageData, RenderMode};

fn num_mip_levels(sizes: &[u32]) -> u32 {
    let max_size = *sizes.iter().max().unwrap();
    1 + (max_size as f32).log2() as u32
}

fn copy_sources_to_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    sources: &[ImageData],
) {
    for (layer, source) in sources.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer as u32,
                },
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
    }
    if texture.mip_level_count() > 1 {
        generate_mips(device, queue, texture);
    }
}

fn create_texture_from_sources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sources: &[ImageData],
    mips: bool,
) -> wgpu::Texture {
    // Assume all sources are the same size so just use the first one for width and height
    let source = &sources[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        format: wgpu::TextureFormat::Rgba8Unorm,
        mip_level_count: if mips {
            num_mip_levels(&[source.width, source.height])
        } else {
            1
        },
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: sources.len() as u32,
        },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    copy_sources_to_texture(device, queue, &texture, sources);
    texture
}

fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static CACHE: RefCell<Option<(wgpu::ShaderModule, wgpu::Sampler)>> = const { RefCell::new(None) };
        static PIPELINE_BY_FORMAT: RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>> =
            RefCell::new(HashMap::new());
    }

    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let (module, sampler) = cache.get_or_insert_with(|| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("textured quad shaders for mip level generation"),
                source: wgpu::ShaderSource::Wgsl(
                    /* wgsl */ r#"
            struct VSOutput {
              @builtin(position) position: vec4f,
              @location(0) texcoord: vec2f,
            };

            @vertex fn vs(
              @builtin(vertex_index) vertexIndex : u32
            ) -> VSOutput {
              let pos = array(

                vec2f( 0.0,  0.0),  // center
                vec2f( 1.0,  0.0),  // right, center
                vec2f( 0.0,  1.0),  // center, top

                // 2st triangle
                vec2f( 0.0,  1.0),  // center, top
                vec2f( 1.0,  0.0),  // right, center
                vec2f( 1.0,  1.0),  // right, top
              );

              var vsOutput: VSOutput;
              let xy = pos[vertexIndex];
              vsOutput.position = vec4f(xy * 2.0 - 1.0, 0.0, 1.0);
              vsOutput.texcoord = vec2f(xy.x, 1.0 - xy.y);
              return vsOutput;
            }

            @group(0) @binding(0) var ourSampler: sampler;
            @group(0) @binding(1) var ourTexture: texture_2d<f32>;

            @fragment fn fs(fsInput: VSOutput) -> @location(0) vec4f {
              return textureSample(ourTexture, ourSampler, fsInput.texcoord);
            }
          "#
                    .into(),
                ),
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            (module, sampler)
        });

        PIPELINE_BY_FORMAT.with(|pipelines| {
            let mut pipelines = pipelines.borrow_mut();
            let pipeline = pipelines.entry(texture.format()).or_insert_with(|| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("mip level generator pipeline"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module,
                        entry_point: None,
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module,
                        entry_point: None,
                        compilation_options: Default::default(),
                        targets: &[Some(texture.format().into())],
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
            });

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mip gen encoder"),
            });

            for base_mip_level in 1..texture.mip_level_count() {
                for layer in 0..texture.depth_or_array_layers() {
                    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: &pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Sampler(sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(
                                    &texture.create_view(&wgpu::TextureViewDescriptor {
                                        dimension: Some(wgpu::TextureViewDimension::D2),
                                        base_mip_level: base_mip_level - 1,
                                        mip_level_count: Some(1),
                                        base_array_layer: layer,
                                        array_layer_count: Some(1),
                                        ..Default::default()
                                    }),
                                ),
                            },
                        ],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    });
                    {
                        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("our basic canvas renderPass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                    store: wgpu::StoreOp::Store,
                                },
                                depth_slice: None,
                            })],
                            ..Default::default()
                        });
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.draw(0..6, 0..1); // call our vertex shader 6 times
                    }
                }
            }

            let command_buffer = encoder.finish();
            queue.submit([command_buffer]);
        });
    });
}

async fn create_texture_from_images(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    urls: &[&str],
    mips: bool,
) -> wgpu::Texture {
    let mut images = Vec::new();
    for url in urls {
        images.push(wgpu_fun::load_image(url).await);
    }
    create_texture_from_sources(device, queue, &images, mips)
}

async fn run() {
    let mut app = App::new("WebGPU SkyBox").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Uniforms {
        viewDirectionProjectionInverse: mat4x4f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) pos: vec4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;
      @group(0) @binding(1) var ourSampler: sampler;
      @group(0) @binding(2) var ourTexture: texture_cube<f32>;

      @vertex fn vs(@builtin(vertex_index) vNdx: u32) -> VSOutput {
        let pos = array(
          vec2f(-1, 3),
          vec2f(-1,-1),
          vec2f( 3,-1),
        );
        var vsOut: VSOutput;
        vsOut.position = vec4f(pos[vNdx], 1, 1);
        vsOut.pos = vsOut.position;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        let t = uni.viewDirectionProjectionInverse * vsOut.pos;
        return textureSample(ourTexture, ourSampler, normalize(t.xyz / t.w) * vec3f(1, 1, -1));
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("no attributes"),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                format: wgpu::TextureFormat::Depth24Plus,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    let texture = create_texture_from_images(
        &app.device,
        &app.queue,
        &[
            "resources/images/leadenhall_market/pos-x.jpg",
            "resources/images/leadenhall_market/neg-x.jpg",
            "resources/images/leadenhall_market/pos-y.jpg",
            "resources/images/leadenhall_market/neg-y.jpg",
            "resources/images/leadenhall_market/pos-z.jpg",
            "resources/images/leadenhall_market/neg-z.jpg",
        ],
        true, // mips
    )
    .await;

    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });

    // viewDirectionProjectionInverse
    const UNIFORM_BUFFER_SIZE: u64 = (16) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET: usize = 0;

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind group for object"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&texture.create_view(
                    &wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::Cube),
                        ..Default::default()
                    },
                )),
            },
        ],
    });

    let mut depth_texture: Option<wgpu::Texture> = None;

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time = frame.time as f32;

        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
        if depth_texture
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            if let Some(texture) = depth_texture.take() {
                texture.destroy();
            }
            depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }
        let depth_view = depth_texture
            .as_ref()
            .unwrap()
            .create_view(&Default::default());

        let aspect = frame.width as f32 / frame.height as f32;
        let projection = Mat4::perspective_rh(
            60.0f32.to_radians(),
            aspect,
            0.1,  // zNear
            10.0, // zFar
        );
        // Camera going in circle from origin looking at origin
        let camera_position = Vec3::new((time * 0.1).cos(), 0.0, (time * 0.1).sin());
        let mut view = Mat4::look_at_rh(
            camera_position,
            Vec3::new(0.0, 0.0, 0.0), // target
            Vec3::new(0.0, 1.0, 0.0), // up
        );
        // We only care about direction so remove the translation
        view.w_axis.x = 0.0;
        view.w_axis.y = 0.0;
        view.w_axis.z = 0.0;

        let view_projection = projection * view;
        let view_direction_projection_inverse = view_projection.inverse();

        uniform_values[K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET
            ..K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET + 16]
            .copy_from_slice(&view_direction_projection_inverse.to_cols_array());

        // upload the uniform values to the uniform buffer
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
