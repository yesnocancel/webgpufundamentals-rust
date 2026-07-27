use glam::{Mat4, Vec3};
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

fn rand_int(max: usize) -> usize {
    rand(0.0, max as f32) as usize
}

struct VertexInfo {
    buffer: wgpu::Buffer,
    num_vertices: u32,
}

fn create_vertex_buffer(device: &wgpu::Device, queue: &wgpu::Queue, data: &[f32]) -> VertexInfo {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(data));
    VertexInfo {
        buffer,
        num_vertices: (data.len() / 2) as u32,
    }
}

// Everything a "model" needs: a shape, plus the indices we'll pass in as
// immediates to select its per model data and material.
struct Model {
    num_vertices: u32,
    vertex_buffer_ndx: usize,
    immediates: [u32; 2],
}

async fn run() {
    // ask for the immediates feature (and its size limit) if the adapter
    // supports it
    let mut app = App::new_with_features_and_limits(
        "WebGPU Immediates - select model and material",
        wgpu::Features::IMMEDIATES,
        |features, limits| wgpu::Limits {
            max_immediate_size: if features.contains(wgpu::Features::IMMEDIATES) {
                limits.max_immediate_size.min(64)
            } else {
                0
            },
            ..wgpu::Limits::default()
        },
    )
    .await;
    // You can probably remove this check by 2027 🙏
    if !app.device.features().contains(wgpu::Features::IMMEDIATES) {
        wgpu_fun::fail("need a browser that supports WebGPU immediates");
        return;
    }
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model and material selection via immediates shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct Material {
        color: vec4f,
      };

      struct PerModel {
        matrix: mat4x4f,
      };

      struct Globals {
        viewProjection: mat4x4f,
      };

      struct Vertex {
        @location(0) position: vec4f,
      };

      struct MyImmediates {
        modelNdx: u32,
        materialNdx: u32,
      };

      @group(0) @binding(0) var<storage, read> materials: array<Material>;
      @group(0) @binding(1) var<storage, read> perModel: array<PerModel>;
      @group(0) @binding(2) var<uniform> glb: Globals;

      var<immediate> imm: MyImmediates;

      @vertex fn vs(v: Vertex) -> @builtin(position) vec4f {
        let model = perModel[imm.modelNdx];
        return glb.viewProjection * model.matrix * v.position;
      }

      @fragment fn fs() -> @location(0) vec4f {
        let material = materials[imm.materialNdx];
        return material.color;
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("our select model and material via immediates pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[
                    // position
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 2 * 4, // 2 floats, 4 bytes each
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        }],
                    }),
                ],
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

    #[rustfmt::skip]
    let square_vertices: Vec<f32> = vec![
        -0.5, -0.5,
         0.5, -0.5,
        -0.5,  0.5,
        -0.5,  0.5,
         0.5, -0.5,
         0.5,  0.5,
    ];
    #[rustfmt::skip]
    let triangle_vertices: Vec<f32> = vec![
         0.0,  0.5,
        -0.5, -0.5,
         0.5, -0.5,
    ];
    let mut circle_vertices: Vec<f32> = Vec::new();
    let num_circle_triangles = 100;
    for i in 0..num_circle_triangles {
        let angle0 = (i + 0) as f32 / num_circle_triangles as f32 * 2.0 * std::f32::consts::PI;
        let angle1 = (i + 1) as f32 / num_circle_triangles as f32 * 2.0 * std::f32::consts::PI;
        circle_vertices.extend([angle0.cos() * 0.5, angle0.sin() * 0.5]);
        circle_vertices.extend([angle1.cos() * 0.5, angle1.sin() * 0.5]);
        circle_vertices.extend([0.0, 0.0]);
    }

    let vertices = [
        create_vertex_buffer(&app.device, &app.queue, &triangle_vertices),
        create_vertex_buffer(&app.device, &app.queue, &circle_vertices),
        create_vertex_buffer(&app.device, &app.queue, &square_vertices),
    ];

    #[rustfmt::skip]
    let material_data: Vec<f32> = vec![
        1.0, 0.5, 0.5, 1.0,  // red
        0.5, 1.0, 0.5, 1.0,  // green
        0.5, 0.5, 1.0, 1.0,  // blue
        1.0, 1.0, 0.5, 1.0,  // yellow
        1.0, 0.5, 1.0, 1.0,  // magenta
        0.5, 1.0, 1.0, 1.0,  // cyan
    ];
    let num_materials = material_data.len() / 4;
    let material_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("our material buffer"),
        size: (material_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&material_buffer, 0, bytemuck::cast_slice(&material_data));

    let mut models = Vec::new();
    const NUM_MODELS: usize = 200;
    let mut model_data = vec![0.0f32; NUM_MODELS * 16];
    for i in 0..NUM_MODELS {
        let model_ndx = i as u32;
        let material_ndx = rand_int(num_materials) as u32;
        let geometry_ndx = rand_int(vertices.len());
        let num_vertices = vertices[geometry_ndx].num_vertices;

        let mat = Mat4::from_translation(Vec3::new(
            (rand(0.0, 1.0) - 0.5) * 2.0,
            (rand(0.0, 1.0) - 0.5) * 2.0,
            0.0,
        )) * Mat4::from_rotation_z(rand(0.0, 1.0) * std::f32::consts::PI * 2.0)
            * Mat4::from_scale(Vec3::new(
                rand(0.0, 1.0) * 0.1 + 0.1,
                rand(0.0, 1.0) * 0.1 + 0.1,
                1.0,
            ));

        model_data[i * 16..i * 16 + 16].copy_from_slice(&mat.to_cols_array());

        models.push(Model {
            num_vertices,
            vertex_buffer_ndx: geometry_ndx,
            immediates: [model_ndx, material_ndx],
        });
    }

    let per_model_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("our per model buffer"),
        size: (model_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    app.queue
        .write_buffer(&per_model_buffer, 0, bytemuck::cast_slice(&model_data));

    let mut shared_data = [0.0f32; 16];
    let shared_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("our shared data buffer"),
        size: (shared_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("our bind group"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: material_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: per_model_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: shared_buffer.as_entire_binding(),
            },
        ],
    });

    app.run(RenderMode::Once, move |frame: &Frame| {
        let aspect = frame.width as f32 / frame.height as f32;
        let ortho =
            glam::camera::rh::proj::directx::orthographic(-aspect, aspect, -1.0, 1.0, -1.0, 1.0);
        shared_data.copy_from_slice(&ortho.to_cols_array());
        frame
            .queue
            .write_buffer(&shared_buffer, 0, bytemuck::cast_slice(&shared_data));

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("our encoder"),
            });
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
            for model in &models {
                pass.set_immediates(0, bytemuck::cast_slice(&model.immediates));
                pass.set_vertex_buffer(0, vertices[model.vertex_buffer_ndx].buffer.slice(..));
                pass.draw(0..model.num_vertices, 0..1);
            }
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
