use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, ImageData, RenderMode};

#[rustfmt::skip]
fn create_cube_vertices() -> (Vec<f32>, Vec<u16>, u32) {
    let vertex_data: Vec<f32> = vec![
         //  position   |  normals
         //-------------+----------------------
         // front face      positive z
        -1.0,  1.0,  1.0,    0.0,  0.0,  1.0,
        -1.0, -1.0,  1.0,    0.0,  0.0,  1.0,
         1.0,  1.0,  1.0,    0.0,  0.0,  1.0,
         1.0, -1.0,  1.0,    0.0,  0.0,  1.0,
         // right face      positive x
         1.0,  1.0, -1.0,    1.0,  0.0,  0.0,
         1.0,  1.0,  1.0,    1.0,  0.0,  0.0,
         1.0, -1.0, -1.0,    1.0,  0.0,  0.0,
         1.0, -1.0,  1.0,    1.0,  0.0,  0.0,
         // back face       negative z
         1.0,  1.0, -1.0,    0.0,  0.0, -1.0,
         1.0, -1.0, -1.0,    0.0,  0.0, -1.0,
        -1.0,  1.0, -1.0,    0.0,  0.0, -1.0,
        -1.0, -1.0, -1.0,    0.0,  0.0, -1.0,
        // left face        negative x
        -1.0,  1.0,  1.0,   -1.0,  0.0,  0.0,
        -1.0,  1.0, -1.0,   -1.0,  0.0,  0.0,
        -1.0, -1.0,  1.0,   -1.0,  0.0,  0.0,
        -1.0, -1.0, -1.0,   -1.0,  0.0,  0.0,
        // bottom face      negative y
         1.0, -1.0,  1.0,    0.0, -1.0,  0.0,
        -1.0, -1.0,  1.0,    0.0, -1.0,  0.0,
         1.0, -1.0, -1.0,    0.0, -1.0,  0.0,
        -1.0, -1.0, -1.0,    0.0, -1.0,  0.0,
        // top face         positive y
        -1.0,  1.0,  1.0,    0.0,  1.0,  0.0,
         1.0,  1.0,  1.0,    0.0,  1.0,  0.0,
        -1.0,  1.0, -1.0,    0.0,  1.0,  0.0,
         1.0,  1.0, -1.0,    0.0,  1.0,  0.0,
    ];

    let index_data: Vec<u16> = vec![
         0,  1,  2,  2,  1,  3,  // front
         4,  5,  6,  6,  5,  7,  // right
         8,  9, 10, 10,  9, 11,  // back
        12, 13, 14, 14, 13, 15,  // left
        16, 17, 18, 18, 17, 19,  // bottom
        20, 21, 22, 22, 21, 23,  // top
    ];

    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}

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
    let mut app = App::new("WebGPU Environment Map").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Uniforms {
        projection: mat4x4f,
        view: mat4x4f,
        world: mat4x4f,
        cameraPosition: vec3f,
      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) normal: vec3f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) worldPosition: vec3f,
        @location(1) worldNormal: vec3f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;
      @group(0) @binding(1) var ourSampler: sampler;
      @group(0) @binding(2) var ourTexture: texture_cube<f32>;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = uni.projection * uni.view * uni.world * vert.position;
        vsOut.worldPosition = (uni.world * vert.position).xyz;
        vsOut.worldNormal = (uni.world * vec4f(vert.normal, 0)).xyz;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        let worldNormal = normalize(vsOut.worldNormal);
        let eyeToSurfaceDir = normalize(vsOut.worldPosition - uni.cameraPosition);
        let direction = reflect(eyeToSurfaceDir, worldNormal);

        return textureSample(ourTexture, ourSampler, direction * vec3f(1, 1, -1));
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("2 attributes"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (3 + 3) * 4, // (6) floats 4 bytes each
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // normal
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 12,
                            format: wgpu::VertexFormat::Float32x3,
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
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
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

    // projection, view, world, cameraPosition, pad
    const UNIFORM_BUFFER_SIZE: u64 = (16 + 16 + 16 + 3 + 1) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_PROJECTION_OFFSET: usize = 0;
    const K_VIEW_OFFSET: usize = 16;
    const K_WORLD_OFFSET: usize = 32;
    const K_CAMERA_POSITION_OFFSET: usize = 48;

    let (vertex_data, index_data, num_vertices) = create_cube_vertices();
    let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer vertices"),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

    let index_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("index buffer"),
        size: (index_data.len() * 2) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));

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
        let camera_position = Vec3::new(0.0, 0.0, 4.0); // camera position
        let view = Mat4::look_at_rh(
            camera_position,
            Vec3::new(0.0, 0.0, 0.0), // target
            Vec3::new(0.0, 1.0, 0.0), // up
        );
        let world = Mat4::from_rotation_x(time * -0.1) * Mat4::from_rotation_y(time * -0.2);

        uniform_values[K_PROJECTION_OFFSET..K_PROJECTION_OFFSET + 16]
            .copy_from_slice(&projection.to_cols_array());
        uniform_values[K_VIEW_OFFSET..K_VIEW_OFFSET + 16].copy_from_slice(&view.to_cols_array());
        uniform_values[K_WORLD_OFFSET..K_WORLD_OFFSET + 16].copy_from_slice(&world.to_cols_array());
        uniform_values[K_CAMERA_POSITION_OFFSET..K_CAMERA_POSITION_OFFSET + 3]
            .copy_from_slice(&camera_position.to_array());

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
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw_indexed(0..num_vertices, 0, 0..1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
