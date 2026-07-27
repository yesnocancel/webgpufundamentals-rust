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
    let mut app = App::new("WebGPU Multiple Triangles w/split Uniforms").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct OurStruct {
        color: vec4f,
        offset: vec2f,
      };

      struct OtherStruct {
        scale: vec2f,
      };

      @group(0) @binding(0) var<uniform> ourStruct: OurStruct;
      @group(0) @binding(1) var<uniform> otherStruct: OtherStruct;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(
          pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return ourStruct.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("multiple uniform buffer"),
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

    // create 2 buffers for the uniform values
    const STATIC_UNIFORM_BUFFER_SIZE: u64 = 4 * 4 + // color is 4 32bit floats (4bytes each)
        2 * 4 + // offset is 2 32bit floats (4bytes each)
        2 * 4; // padding
    const UNIFORM_BUFFER_SIZE: u64 = 2 * 4; // scale is 2 32bit floats (4bytes each)

    // offsets to the various uniform values in float32 indices
    const K_COLOR_OFFSET: usize = 0;
    const K_OFFSET_OFFSET: usize = 4;

    const K_SCALE_OFFSET: usize = 0;

    struct ObjectInfo {
        scale: f32,
        uniform_buffer: wgpu::Buffer,
        uniform_values: [f32; UNIFORM_BUFFER_SIZE as usize / 4],
        bind_group: wgpu::BindGroup,
    }

    const K_NUM_OBJECTS: usize = 100;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    for i in 0..K_NUM_OBJECTS {
        let static_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("static uniforms for obj: {i}")),
            size: STATIC_UNIFORM_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // These are only set once so set them now
        {
            let mut uniform_values = [0.0f32; STATIC_UNIFORM_BUFFER_SIZE as usize / 4];
            uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                rand(0.0, 1.0),
                1.0,
            ]); // set the color
            uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2]
                .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

            // copy these values to the GPU
            app.queue.write_buffer(
                &static_uniform_buffer,
                0,
                bytemuck::cast_slice(&uniform_values),
            );
        }

        // create an array of f32s to hold the values for the uniforms in Rust
        let uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("changing uniforms for obj: {i}")),
            size: UNIFORM_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("bind group for obj: {i}")),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: static_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        object_infos.push(ObjectInfo {
            scale: rand(0.2, 0.5),
            uniform_buffer,
            uniform_values,
            bind_group,
        });
    }

    app.run(RenderMode::Once, move |frame: &Frame| {
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

            // Set the uniform values in our Rust-side array of f32s
            let aspect = frame.width as f32 / frame.height as f32;

            for object_info in object_infos.iter_mut() {
                let scale = object_info.scale;
                object_info.uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
                    .copy_from_slice(&[scale / aspect, scale]); // set the scale
                frame.queue.write_buffer(
                    &object_info.uniform_buffer,
                    0,
                    bytemuck::cast_slice(&object_info.uniform_values),
                );

                pass.set_bind_group(0, &object_info.bind_group, &[]);
                pass.draw(0..3, 0..1); // call our vertex shader 3 times
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
