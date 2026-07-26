use glam::{Mat4, Vec3};
use wgpu_fun::{App, Frame, RenderMode};

const SAMPLE_COUNT: u32 = 4; // can be 1 or 4

#[rustfmt::skip]
fn cube_data() -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<u16>) {
    let positions: Vec<f32> = vec![1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, -1.0, -1.0, -1.0];
    let normals: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0, 0.0, 0.0, -1.0];
    let texcoords: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let indices: Vec<u16> = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23];
    (positions, normals, texcoords, indices)
}

fn create_buffer(device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: data.len() as u64,
        usage: usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
}

async fn run() {
    let mut app = App::new("WebGPU Cube Multiple").await;
    app.auto_resize = true;

    let shader_src = r#"
  struct VSUniforms {
    worldViewProjection: mat4x4f,
    worldInverseTranspose: mat4x4f,
  };
  @group(0) @binding(0) var<uniform> vsUniforms: VSUniforms;

  struct MyVSInput {
      @location(0) position: vec4f,
      @location(1) normal: vec3f,
      @location(2) texcoord: vec2f,
  };

  struct MyVSOutput {
    @builtin(position) position: vec4f,
    @location(0) normal: vec3f,
    @location(1) texcoord: vec2f,
  };

  @vertex
  fn myVSMain(v: MyVSInput) -> MyVSOutput {
    var vsOut: MyVSOutput;
    vsOut.position = vsUniforms.worldViewProjection * v.position;
    vsOut.normal = (vsUniforms.worldInverseTranspose * vec4f(v.normal, 0.0)).xyz;
    vsOut.texcoord = v.texcoord;
    return vsOut;
  }

  struct FSUniforms {
    lightDirection: vec3f,
  };

  @group(0) @binding(1) var<uniform> fsUniforms: FSUniforms;
  @group(0) @binding(2) var diffuseSampler: sampler;
  @group(0) @binding(3) var diffuseTexture: texture_2d<f32>;

  @fragment
  fn myFSMain(v: MyVSOutput) -> @location(0) vec4f {
    var diffuseColor = textureSample(diffuseTexture, diffuseSampler, v.texcoord);
    var a_normal = normalize(v.normal);
    var l = dot(a_normal, fsUniforms.lightDirection) * 0.5 + 0.5;
    return vec4f(diffuseColor.rgb * l, diffuseColor.a);
  }
  "#;

    let (positions, normals, texcoords, indices) = cube_data();
    let position_buffer = create_buffer(&app.device, &app.queue, bytemuck::cast_slice(&positions), wgpu::BufferUsages::VERTEX);
    let normal_buffer = create_buffer(&app.device, &app.queue, bytemuck::cast_slice(&normals), wgpu::BufferUsages::VERTEX);
    let texcoord_buffer = create_buffer(&app.device, &app.queue, bytemuck::cast_slice(&texcoords), wgpu::BufferUsages::VERTEX);
    let indices_buffer = create_buffer(&app.device, &app.queue, bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX);
    let num_indices = indices.len() as u32;

    let tex = app.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
    });
    app.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &[
            255, 255, 128, 255, //
            128, 255, 255, 255, //
            255, 128, 255, 255, //
            255, 128, 128, 255,
        ],
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
    );

    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let shader_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("fake lighting"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: None,
            compilation_options: Default::default(),
            buffers: &[
                // position
                Some(wgpu::VertexBufferLayout {
                    array_stride: 3 * 4, // 3 floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        shader_location: 0,
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }),
                // normals
                Some(wgpu::VertexBufferLayout {
                    array_stride: 3 * 4, // 3 floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        shader_location: 1,
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x3,
                    }],
                }),
                // texcoords
                Some(wgpu::VertexBufferLayout {
                    array_stride: 2 * 4, // 2 floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        shader_location: 2,
                        offset: 0,
                        format: wgpu::VertexFormat::Float32x2,
                    }],
                }),
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: None,
            compilation_options: Default::default(),
            targets: &[Some(app.format.into())],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
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
        multisample: wgpu::MultisampleState {
            count: SAMPLE_COUNT,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    });

    let vs_uniform_buffer_size = 2 * 16 * 4; // 2 mat4s * 16 floats per mat * 4 bytes per float
    let fs_uniform_buffer_size = 3 * 4; // 1 vec3 * 3 floats per vec3 * 4 bytes per float
    let fs_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::max(16, fs_uniform_buffer_size),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    struct ObjectInfo {
        vs_uniform_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
        translation: Vec3,
    }

    let num_objects = 100;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();
    let tex_view = tex.create_view(&Default::default());

    for i in 0..num_objects {
        let vs_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::max(16, vs_uniform_buffer_size),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vs_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fs_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&tex_view),
                },
            ],
        });

        let across = (num_objects as f32).sqrt() as i32;
        let x = ((i % across) as f32 - (across - 1) as f32 / 2.0) * 3.0;
        let y = ((i / across) as f32 - (across - 1) as f32 / 2.0) * 3.0;

        object_infos.push(ObjectInfo {
            vs_uniform_buffer, // needed to update the buffer
            bind_group,        // needed to render this object
            translation: Vec3::new(x, y, 0.0),
        });
    }

    let mut render_target: Option<wgpu::Texture> = None;
    let mut depth_texture: Option<wgpu::Texture> = None;

    app.run(RenderMode::Continuous, move |frame: &Frame| {
        let time = frame.time as f32;

        // recreate the render target and depth texture on size change
        if depth_texture
            .as_ref()
            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
        {
            if let Some(t) = render_target.take() {
                t.destroy();
            }
            if let Some(t) = depth_texture.take() {
                t.destroy();
            }
            if SAMPLE_COUNT > 1 {
                render_target = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: frame.width,
                        height: frame.height,
                        depth_or_array_layers: 1,
                    },
                    format: frame.format,
                    sample_count: SAMPLE_COUNT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    mip_level_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    view_formats: &[],
                }));
            }
            depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: frame.width,
                    height: frame.height,
                    depth_or_array_layers: 1,
                },
                format: wgpu::TextureFormat::Depth24Plus,
                sample_count: SAMPLE_COUNT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                dimension: wgpu::TextureDimension::D2,
                view_formats: &[],
            }));
        }

        let projection = Mat4::perspective_rh(
            30.0f32.to_radians(),
            frame.width as f32 / frame.height as f32,
            0.5,
            100.0,
        );
        let eye = Vec3::new(1.0, 4.0, -46.0);
        let target = Vec3::new(0.0, 0.0, 0.0);
        let up = Vec3::new(0.0, 1.0, 0.0);
        let view = Mat4::look_at_rh(eye, target, up);
        let view_projection = projection * view;

        // the lighting info is shared so set these uniforms once
        let light_direction = Vec3::new(1.0, 8.0, -10.0).normalize();
        let fs_uniform_values: [f32; 3] = light_direction.to_array(); // 1 vec3
        frame.queue.write_buffer(&fs_uniform_buffer, 0, bytemuck::cast_slice(&fs_uniform_values));

        let render_target_view = render_target.as_ref().map(|t| t.create_view(&Default::default()));
        let depth_view = depth_texture.as_ref().unwrap().create_view(&Default::default());

        let mut encoder = frame.device.create_command_encoder(&Default::default());
        {
            let (view, resolve_target) = if SAMPLE_COUNT == 1 {
                (frame.view, None)
            } else {
                (render_target_view.as_ref().unwrap(), Some(frame.view))
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.5,
                            g: 0.5,
                            b: 0.5,
                            a: 1.0,
                        }),
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

            // Of course these could be per object but since we're drawing the same object
            // multiple times, just set them once.
            pass.set_vertex_buffer(0, position_buffer.slice(..));
            pass.set_vertex_buffer(1, normal_buffer.slice(..));
            pass.set_vertex_buffer(2, texcoord_buffer.slice(..));
            pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint16);

            for (ndx, object) in object_infos.iter().enumerate() {
                pass.set_bind_group(0, &object.bind_group, &[]);

                let world = Mat4::from_translation(object.translation)
                    * Mat4::from_rotation_x(time * 0.9 + ndx as f32)
                    * Mat4::from_rotation_y(time + ndx as f32);
                let world_inverse_transpose = world.inverse().transpose();
                let world_view_projection = view_projection * world;

                let mut vs_uniform_values = [0.0f32; 2 * 16]; // 2 mat4s
                vs_uniform_values[0..16].copy_from_slice(&world_view_projection.to_cols_array());
                vs_uniform_values[16..32].copy_from_slice(&world_inverse_transpose.to_cols_array());
                frame.queue.write_buffer(&object.vs_uniform_buffer, 0, bytemuck::cast_slice(&vs_uniform_values));
                pass.draw_indexed(0..num_indices, 0, 0..1);
            }
        }

        frame.queue.submit([encoder.finish()]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
