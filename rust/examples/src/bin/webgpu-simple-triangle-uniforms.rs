use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let mut app = App::new("WebGPU Simple Triangle w/Uniforms").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct OurStruct {
        color: vec4f,
        scale: vec2f,
        offset: vec2f,
      };

      @group(0) @binding(0) var<uniform> ourStruct: OurStruct;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(
          pos[vertexIndex] * ourStruct.scale + ourStruct.offset, 0.0, 1.0);
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
            label: Some("triangle with uniforms"),
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

    // create a buffer for the uniform values
    const UNIFORM_BUFFER_SIZE: u64 = 4 * 4 + // color is 4 32bit floats (4bytes each)
        2 * 4 + // scale is 2 32bit floats (4bytes each)
        2 * 4; // offset is 2 32bit floats (4bytes each)
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms for triangle"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // create an array of f32s to hold the values for the uniforms in Rust
    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_COLOR_OFFSET: usize = 0;
    const K_SCALE_OFFSET: usize = 4;
    const K_OFFSET_OFFSET: usize = 6;

    uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]); // set the color
    uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2].copy_from_slice(&[-0.5, -0.25]); // set the offset

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("triangle bind group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    app.run(RenderMode::Once, move |frame: &Frame| {
        // Set the uniform values in our Rust-side array of f32s
        let aspect = frame.width as f32 / frame.height as f32;
        uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2].copy_from_slice(&[0.5 / aspect, 0.5]); // set the scale

        // copy the values from Rust to the GPU
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
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1); // call our vertex shader 3 times
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
