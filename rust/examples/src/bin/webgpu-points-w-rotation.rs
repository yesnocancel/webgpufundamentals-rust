use wgpu_fun::{App, Frame, RenderMode};

// A random number between [min and max)
fn rand(min: f32, max: f32) -> f32 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u32> = const { Cell::new(0x13579bdf) });
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        min + (max - min) * (x as f32 / u32::MAX as f32)
    })
}

async fn run() {
    let mut app = App::new("WebGPU Points with Rotation").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Vertex {
        @location(0) position: vec2f,
        @location(1) size: f32,
        @location(2) rotation: f32,
      };

      struct Uniforms {
        resolution: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;

      @vertex fn vs(
          vert: Vertex,
          @builtin(vertex_index) vNdx: u32,
      ) -> VSOutput {
        let points = array(
          vec2f(-1, -1),
          vec2f( 1, -1),
          vec2f(-1,  1),
          vec2f(-1,  1),
          vec2f( 1, -1),
          vec2f( 1,  1),
        );
        var vsOut: VSOutput;
        let pos = points[vNdx];
        let c = cos(vert.rotation);
        let s = sin(vert.rotation);
        let rot = mat2x2f(
           c, s,
          -s, c,
        );
        vsOut.position = vec4f(vert.position + rot * pos * vert.size / uni.resolution, 0, 1);
        vsOut.texcoord = pos * 0.5 + 0.5;
        return vsOut;
      }

      @group(0) @binding(1) var s: sampler;
      @group(0) @binding(2) var t: texture_2d<f32>;

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return textureSample(t, s, vsOut.texcoord);
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sizeable rotatable points with texture"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (2 + 1 + 1) * 4, // 4 floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        // size
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 8,
                            format: wgpu::VertexFormat::Float32,
                        },
                        // rotation
                        wgpu::VertexAttribute {
                            shader_location: 2,
                            offset: 12,
                            format: wgpu::VertexFormat::Float32,
                        },
                    ],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: app.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: Default::default(),
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    const K_NUM_POINTS: usize = 100;
    let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 4];
    for i in 0..K_NUM_POINTS {
        let offset = i * 4;
        vertex_data[offset] = rand(-1.0, 1.0);
        vertex_data[offset + 1] = rand(-1.0, 1.0);
        vertex_data[offset + 2] = rand(10.0, 64.0);
        vertex_data[offset + 3] = rand(0.0, std::f32::consts::PI * 2.0);
    }

    let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vertex buffer vertices"),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

    // The JS version draws an emoji into an OffscreenCanvas; we load a
    // pre-made 32x32 image of the same emoji and premultiply its alpha
    // (the JS version asks for that with premultipliedAlpha: true).
    let mut source = wgpu_fun::load_image("resources/images/emoji/pointing-right.png").await;
    for pixel in source.data.chunks_mut(4) {
        let a = pixel[3] as u32;
        pixel[0] = (pixel[0] as u32 * a / 255) as u8;
        pixel[1] = (pixel[1] as u32 * a / 255) as u8;
        pixel[2] = (pixel[2] as u32 * a / 255) as u8;
    }
    let texture = app.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
    });
    // flipY like the JS version
    let flipped: Vec<u8> = source
        .data
        .chunks(32 * 4)
        .rev()
        .flatten()
        .copied()
        .collect();
    app.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &flipped,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: 32,
            height: 32,
            depth_or_array_layers: 1,
        },
    );

    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        min_filter: wgpu::FilterMode::Linear,
        mag_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let uniform_values = [0.0f32; 2];
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (uniform_values.len() * 4) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    const K_RESOLUTION_OFFSET: usize = 0;

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
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
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
        ],
    });

    let mut uniform_values = uniform_values;
    app.run(RenderMode::Once, move |frame: &Frame| {
        // Update the resolution in the uniform buffer
        uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
            .copy_from_slice(&[frame.width as f32, frame.height as f32]);
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
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..K_NUM_POINTS as u32);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
