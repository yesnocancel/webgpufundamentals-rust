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

struct ObjectInfo {
    scale: f32,
}

async fn run() {
    let mut app = App::new("WebGPU Multiple Triangles w/split Storage Buffers").await;
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

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) color: vec4f,
      }

      @group(0) @binding(0) var<storage, read> ourStructs: array<OurStruct>;
      @group(0) @binding(1) var<storage, read> otherStructs: array<OtherStruct>;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
        @builtin(instance_index) instanceIndex: u32
      ) -> VSOutput {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        let otherStruct = otherStructs[instanceIndex];
        let ourStruct = ourStructs[instanceIndex];

        var vsOut: VSOutput;
        vsOut.position = vec4f(
            pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
        vsOut.color = ourStruct.color;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return vsOut.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("split storage buffer pipeline"),
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

    const K_NUM_OBJECTS: usize = 100;

    // create 2 storage buffers
    let static_storage_unit_size = 4 * 4 + // color is 4 32bit floats (4bytes each)
        2 * 4 + // offset is 2 32bit floats (4bytes each)
        2 * 4; // padding
    let storage_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
    let static_storage_buffer_size = static_storage_unit_size * K_NUM_OBJECTS;
    let storage_buffer_size = storage_unit_size * K_NUM_OBJECTS;

    let static_storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("static storage for objects"),
        size: static_storage_buffer_size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("changing storage for objects"),
        size: storage_buffer_size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut static_storage_values = vec![0.0f32; static_storage_buffer_size / 4];
    let mut storage_values = vec![0.0f32; storage_buffer_size / 4];

    // offsets to the various uniform values in float32 indices
    const K_COLOR_OFFSET: usize = 0;
    const K_OFFSET_OFFSET: usize = 4;

    const K_SCALE_OFFSET: usize = 0;

    let mut object_infos: Vec<ObjectInfo> = Vec::new();

    for i in 0..K_NUM_OBJECTS {
        let static_offset = i * (static_storage_unit_size / 4);

        // These are only set once so set them now
        static_storage_values[static_offset + K_COLOR_OFFSET..static_offset + K_COLOR_OFFSET + 4]
            .copy_from_slice(&[rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0]); // set the color
        static_storage_values[static_offset + K_OFFSET_OFFSET..static_offset + K_OFFSET_OFFSET + 2]
            .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

        object_infos.push(ObjectInfo {
            scale: rand(0.2, 0.5),
        });
    }
    app.queue.write_buffer(
        &static_storage_buffer,
        0,
        bytemuck::cast_slice(&static_storage_values),
    );

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind group for objects"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: static_storage_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buffer.as_entire_binding(),
            },
        ],
    });

    app.run(RenderMode::Once, move |frame: &Frame| {
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
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

            // Set the uniform values in our Rust side Vec
            let aspect = frame.width as f32 / frame.height as f32;

            for (ndx, ObjectInfo { scale }) in object_infos.iter().enumerate() {
                let offset = ndx * (storage_unit_size / 4);
                storage_values[offset + K_SCALE_OFFSET..offset + K_SCALE_OFFSET + 2]
                    .copy_from_slice(&[scale / aspect, *scale]); // set the scale
            }
            frame
                .queue
                .write_buffer(&storage_buffer, 0, bytemuck::cast_slice(&storage_values));

            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..K_NUM_OBJECTS as u32); // call our vertex shader 3 times for several instances
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
