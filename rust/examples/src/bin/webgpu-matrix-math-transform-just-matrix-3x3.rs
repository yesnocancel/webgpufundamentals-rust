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

mod mat3 {
    #[rustfmt::skip]
    pub fn projection(width: f32, height: f32) -> [f32; 9] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        [
            2.0 / width, 0.0, 0.0,
            0.0, -2.0 / height, 0.0,
            -1.0, 1.0, 1.0,
        ]
    }

    #[allow(dead_code)]
    #[rustfmt::skip]
    pub fn identity() -> [f32; 9] {
        [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ]
    }

    pub fn multiply(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
        let a00 = a[0 * 3 + 0];
        let a01 = a[0 * 3 + 1];
        let a02 = a[0 * 3 + 2];
        let a10 = a[1 * 3 + 0];
        let a11 = a[1 * 3 + 1];
        let a12 = a[1 * 3 + 2];
        let a20 = a[2 * 3 + 0];
        let a21 = a[2 * 3 + 1];
        let a22 = a[2 * 3 + 2];
        let b00 = b[0 * 3 + 0];
        let b01 = b[0 * 3 + 1];
        let b02 = b[0 * 3 + 2];
        let b10 = b[1 * 3 + 0];
        let b11 = b[1 * 3 + 1];
        let b12 = b[1 * 3 + 2];
        let b20 = b[2 * 3 + 0];
        let b21 = b[2 * 3 + 1];
        let b22 = b[2 * 3 + 2];

        [
            b00 * a00 + b01 * a10 + b02 * a20,
            b00 * a01 + b01 * a11 + b02 * a21,
            b00 * a02 + b01 * a12 + b02 * a22,
            b10 * a00 + b11 * a10 + b12 * a20,
            b10 * a01 + b11 * a11 + b12 * a21,
            b10 * a02 + b11 * a12 + b12 * a22,
            b20 * a00 + b21 * a10 + b22 * a20,
            b20 * a01 + b21 * a11 + b22 * a21,
            b20 * a02 + b21 * a12 + b22 * a22,
        ]
    }

    #[rustfmt::skip]
    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 9] {
        [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            tx, ty, 1.0,
        ]
    }

    #[rustfmt::skip]
    pub fn rotation(angle_in_radians: f32) -> [f32; 9] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        [
            c, s, 0.0,
            -s, c, 0.0,
            0.0, 0.0, 1.0,
        ]
    }

    #[rustfmt::skip]
    pub fn scaling([sx, sy]: [f32; 2]) -> [f32; 9] {
        [
            sx, 0.0, 0.0,
            0.0, sy, 0.0,
            0.0, 0.0, 1.0,
        ]
    }
}

async fn run() {
    let mut app = App::new("WebGPU Matrix Transform Move Origin").await;
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
        matrix: mat3x3f,
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

        let clipSpace = (uni.matrix * vec3f(vert.position, 1)).xy;

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

    // color, matrix
    const UNIFORM_BUFFER_SIZE: u64 = (4 + 12) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_COLOR_OFFSET: usize = 0;
    const K_MATRIX_OFFSET: usize = 4;

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

            let translation = [
                wgpu_fun::setting_f64("translationX", 150.0) as f32,
                wgpu_fun::setting_f64("translationY", 100.0) as f32,
            ];
            let rotation = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
            let scale = [
                wgpu_fun::setting_f64("scaleX", 1.0) as f32,
                wgpu_fun::setting_f64("scaleY", 1.0) as f32,
            ];

            let projection_matrix = mat3::projection(frame.width as f32, frame.height as f32);
            let translation_matrix = mat3::translation(translation);
            let rotation_matrix = mat3::rotation(rotation);
            let scale_matrix = mat3::scaling(scale);

            let mut matrix = mat3::multiply(&projection_matrix, &translation_matrix);
            matrix = mat3::multiply(&matrix, &rotation_matrix);
            matrix = mat3::multiply(&matrix, &scale_matrix);

            // Set the uniform values in our Rust side array
            // copy each column of 3 values, followed by 1 float of padding
            uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 3].copy_from_slice(&matrix[0..3]);
            uniform_values[K_MATRIX_OFFSET + 4..K_MATRIX_OFFSET + 7].copy_from_slice(&matrix[3..6]);
            uniform_values[K_MATRIX_OFFSET + 8..K_MATRIX_OFFSET + 11]
                .copy_from_slice(&matrix[6..9]);

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
