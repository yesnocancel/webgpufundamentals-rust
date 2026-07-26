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

#[rustfmt::skip]
fn create_f_vertices() -> (Vec<f32>, Vec<u32>, u32) {
    let vertex_data: Vec<f32> = vec![
        // left column
        0.0, 0.0,
        30.0, 0.0,
        0.0, 150.0,
        30.0, 150.0,

        // top rung
        30.0, 0.0,
        100.0, 0.0,
        30.0, 30.0,
        100.0, 30.0,

        // middle rung
        30.0, 60.0,
        70.0, 60.0,
        30.0, 90.0,
        70.0, 90.0,
    ];

    let index_data: Vec<u32> = vec![
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top run
        8,  9, 10,   10,  9, 11,  // middle run
    ];

    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}

async fn run() {
    let mut app = App::new("WebGPU Scale").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct Uniforms {
        color: vec4f,
        resolution: vec2f,
        translation: vec2f,
        rotation: vec2f,
        scale: vec2f,
      };

      struct Vertex {
        @location(0) position: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;

        // Scale the position
        let scaledPosition = vert.position * uni.scale;

        // Rotate the position
        let rotatedPosition = vec2f(
          scaledPosition.x * uni.rotation.x - scaledPosition.y * uni.rotation.y,
          scaledPosition.x * uni.rotation.y + scaledPosition.y * uni.rotation.x
        );

        // Add in the translation
        let position = rotatedPosition + uni.translation;

        // convert the position from pixels to a 0.0 to 1.0 value
        let zeroToOne = position / uni.resolution;

        // convert from 0 <-> 1 to 0 <-> 2
        let zeroToTwo = zeroToOne * 2.0;

        // covert from 0 <-> 2 to -1 <-> +1 (clip space)
        let flippedClipSpace = zeroToTwo - 1.0;

        // flip Y
        let clipSpace = flippedClipSpace * vec2f(1, -1);

        vsOut.position = vec4f(clipSpace, 0.0, 1.0);
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return uni.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("just 2d position"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (2) * 4, // (2) floats, 4 bytes each
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
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
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

    // color, resolution, translation, rotation, scale
    const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 2 + 2) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_COLOR_OFFSET: usize = 0;
    const K_RESOLUTION_OFFSET: usize = 4;
    const K_TRANSLATION_OFFSET: usize = 6;
    const K_ROTATION_OFFSET: usize = 8;
    const K_SCALE_OFFSET: usize = 10;

    // The color will not change so let's set it once at init time
    uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
        rand(0.0, 1.0),
        rand(0.0, 1.0),
        rand(0.0, 1.0),
        1.0,
    ]);

    let (vertex_data, index_data, num_vertices) = create_f_vertices();
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
        size: (index_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind group for object"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

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
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            // Set the uniform values in our Rust side array
            uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
                .copy_from_slice(&[frame.width as f32, frame.height as f32]);
            let translation = [
                wgpu_fun::setting_f64("translationX", 150.0) as f32,
                wgpu_fun::setting_f64("translationY", 100.0) as f32,
            ];
            uniform_values[K_TRANSLATION_OFFSET..K_TRANSLATION_OFFSET + 2]
                .copy_from_slice(&translation);
            let angle = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
            let rotation = [angle.cos(), angle.sin()];
            uniform_values[K_ROTATION_OFFSET..K_ROTATION_OFFSET + 2]
                .copy_from_slice(&rotation);
            let scale = [
                wgpu_fun::setting_f64("scaleX", 1.0) as f32,
                wgpu_fun::setting_f64("scaleY", 1.0) as f32,
            ];
            uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
                .copy_from_slice(&scale);

            // upload the uniform values to the uniform buffer
            frame
                .queue
                .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));

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
