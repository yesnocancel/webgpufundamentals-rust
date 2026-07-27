use glam::Mat4;
use wgpu_fun::{Canvas, MultiApp, MultiFrame, RenderMode};

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

fn random_color() -> [f32; 4] {
    [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0]
}

// Everything we need per canvas: the JS `infos` array entries.
struct Info {
    canvas: Canvas,
    clear_value: [f32; 4],
    uniform_values: [f32; 16 + 4],
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    rotation: f32,
}

async fn run() {
    let mut app = MultiApp::new("WebGPU Multiple Canvases - 200").await;
    // Each canvas's drawing buffer follows its displayed size, like the
    // ResizeObserver in the original.
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our triangle shaders"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct Uniforms {
        matrix: mat4x4f,
        color: vec4f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return uni.matrix * vec4f(pos[vertexIndex], 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return uni.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("our hardcoded red triangle pipeline"),
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

    // One canvas per product card. On the page the cards (and their CSS
    // sizes) are made by the page's JS; natively these are the four
    // .size0-.size3 CSS sizes from the original.
    const NUM_PRODUCTS: usize = 200;
    let sizes: Vec<(u32, u32)> = (0..NUM_PRODUCTS)
        .map(|i| [(200, 200), (250, 200), (300, 200), (100, 200)][i % 4])
        .collect();

    let mut infos = Vec::new();
    for canvas in app.canvases(&sizes) {
        // Make a uniform buffer and values for our uniforms.
        let mut uniform_values = [0.0f32; 16 + 4];
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: (uniform_values.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        const K_COLOR_OFFSET: usize = 16;
        uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&random_color());

        // Make a bind group for this uniform
        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        infos.push(Info {
            canvas,
            clear_value: random_color(),
            uniform_values,
            uniform_buffer,
            bind_group,
            rotation: rand(0.0, std::f32::consts::PI * 2.0),
        });
    }

    // Our own time that only advances while we're animating, so nothing
    // jumps when the Stop/Start button pauses us.
    let mut time = 0.0;
    let mut then = 0.0;
    app.run(RenderMode::Continuous, move |frame: &MultiFrame| {
        let now = frame.time;
        let delta_time = now - then;
        then = now;
        // The page's Stop/Start button toggles this setting (instead of
        // cancelling requestAnimationFrame like the JS version).
        if !wgpu_fun::setting_bool("running", true) {
            return;
        }
        time += delta_time;

        // make a command encoder to start encoding commands
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("our encoder"),
            });

        for Info {
            canvas,
            clear_value,
            uniform_values,
            uniform_buffer,
            bind_group,
            rotation,
        } in infos.iter_mut()
        {
            // Get the current texture from the canvas context and
            // set it as the texture to render to.
            let view = canvas.current_view();

            let aspect = canvas.width() as f32 / canvas.height() as f32;
            let matrix = glam::camera::rh::proj::directx::orthographic(-aspect, aspect, -1.0, 1.0, -1.0, 1.0)
                * Mat4::from_rotation_z(time as f32 * 0.1 + *rotation);
            uniform_values[0..16].copy_from_slice(&matrix.to_cols_array());

            // Upload our uniform values.
            frame
                .queue
                .write_buffer(uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

            // make a render pass encoder to encode render specific commands
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("our basic canvas renderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: clear_value[0] as f64,
                                g: clear_value[1] as f64,
                                b: clear_value[2] as f64,
                                a: clear_value[3] as f64,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &*bind_group, &[]);
                pass.draw(0..3, 0..1); // call our vertex shader 3 times.
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
