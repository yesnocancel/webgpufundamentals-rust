use std::collections::HashMap;

use glam::{vec3, Mat4};
use wgpu_fun::{App, Frame, RenderMode};

/// A CPU-made image: tightly packed un-premultiplied rgba8 pixels,
/// standing in for the 2d canvases the JS version draws its images with.
struct SourceImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

// The JS version makes its colors with CSS `hsl(...)`/`hsla(...)` strings.
// This is the same conversion CSS does (s and l fixed at 1 and 0.5 like the
// examples use them; h is in turns, `h * 360` is degrees).
fn hsl(h: f32) -> [f32; 3] {
    let h = ((h * 360.0) as i32 as f32).rem_euclid(360.0) / 60.0; // `h * 360 | 0`
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    match h as u32 {
        0 => [1.0, x, 0.0],
        1 => [x, 1.0, 0.0],
        2 => [0.0, 1.0, x],
        3 => [0.0, x, 1.0],
        4 => [x, 0.0, 1.0],
        _ => [1.0, 0.0, x],
    }
}

fn to_rgba8(pixels: Vec<[f32; 4]>, width: u32, height: u32) -> SourceImage {
    let data = pixels
        .iter()
        .flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
        .collect();
    SourceImage { data, width, height }
}

// Reproduces the JS createSourceImage: three circles with radial
// hsla gradients (opaque in the middle, transparent at the edge),
// drawn with the 'screen' composite mode.
fn create_source_image(size: u32) -> SourceImage {
    let sizef = size as f32;
    let mut pixels = vec![[0.0f32; 4]; (size * size) as usize];

    const NUM_CIRCLES: u32 = 3;
    for i in 0..NUM_CIRCLES {
        // the canvas version rotates PI * 2 / numCircles each time and
        // translates by size / 6; these are the resulting circle centers
        let angle = std::f32::consts::PI * 2.0 * (i + 1) as f32 / NUM_CIRCLES as f32;
        let center_x = sizef / 2.0 + angle.cos() * sizef / 6.0;
        let center_y = sizef / 2.0 + angle.sin() * sizef / 6.0;

        let radius = sizef / 3.0;
        let color = hsl(i as f32 / NUM_CIRCLES as f32);

        for y in 0..size {
            for x in 0..size {
                let dx = x as f32 + 0.5 - center_x;
                let dy = y as f32 + 0.5 - center_y;
                let dist = (dx * dx + dy * dy).sqrt();

                // the radial gradient: alpha 1 from the center to half way
                // between radius / 2 and radius, fading to 0 at radius
                let t = ((dist - radius / 2.0) / (radius / 2.0)).clamp(0.0, 1.0);
                let src_alpha = ((1.0 - t) * 2.0).clamp(0.0, 1.0);
                if src_alpha <= 0.0 {
                    continue;
                }

                // composite onto what's already there with the canvas
                // 'screen' blend mode
                let [dst_r, dst_g, dst_b, dst_a] = pixels[(y * size + x) as usize];
                let out_a = src_alpha + dst_a * (1.0 - src_alpha);
                let screen = |cb: f32, cs: f32| cb + cs - cb * cs;
                let blend = |cb: f32, cs: f32| {
                    (src_alpha * (1.0 - dst_a) * cs
                        + src_alpha * dst_a * screen(cb, cs)
                        + (1.0 - src_alpha) * dst_a * cb)
                        / out_a
                };
                pixels[(y * size + x) as usize] = [
                    blend(dst_r, color[0]),
                    blend(dst_g, color[1]),
                    blend(dst_b, color[2]),
                    out_a,
                ];
            }
        }
    }
    to_rgba8(pixels, size, size)
}

// Reproduces the JS createDestinationImage: a diagonal rainbow linear
// gradient with diagonal stripes erased with the 'destination-out'
// composite mode.
fn create_destination_image(size: u32) -> SourceImage {
    let sizef = size as f32;

    // the 7 gradient color stops, hsl(0 / -6) ... hsl(6 / -6)
    let stops: Vec<[f32; 3]> = (0..=6).map(|i| hsl(i as f32 / -6.0)).collect();

    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            // the linear gradient runs from the top-left corner (0, 0) to
            // the bottom-right corner (size, size)
            let t = ((x as f32 + 0.5) + (y as f32 + 0.5)) / (sizef * 2.0);
            let seg = (t * 6.0).clamp(0.0, 6.0);
            let ndx = (seg as usize).min(5);
            let f = seg - ndx as f32;
            let color: [f32; 3] =
                std::array::from_fn(|c| stops[ndx][c] + (stops[ndx + 1][c] - stops[ndx][c]) * f);

            // erase 16 pixel tall stripes every 32 pixels, rotated by
            // PI / -4, like the rotate + fillRect loop (4x4 supersampled
            // to keep the anti-aliased edges the canvas gives us)
            let mut coverage = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                    let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                    let stripe = (px + py) / std::f32::consts::SQRT_2;
                    if stripe.rem_euclid(32.0) < 16.0 {
                        coverage += 1.0 / 16.0;
                    }
                }
            }
            pixels.push([color[0], color[1], color[2], 1.0 - coverage]);
        }
    }
    to_rgba8(pixels, size, size)
}

// The premultipliedAlpha: true option of copyExternalImageToTexture,
// done on the CPU: multiply the colors by the alpha value as we copy.
fn premultiply_alpha(source: &SourceImage) -> SourceImage {
    let data = source
        .data
        .chunks_exact(4)
        .flat_map(|p| {
            let a = p[3] as f32 / 255.0;
            [
                (p[0] as f32 * a).round() as u8,
                (p[1] as f32 * a).round() as u8,
                (p[2] as f32 * a).round() as u8,
                p[3],
            ]
        })
        .collect();
    SourceImage {
        data,
        width: source.width,
        height: source.height,
    }
}

fn num_mip_levels(sizes: &[u32]) -> u32 {
    let max_size = *sizes.iter().max().unwrap();
    1 + (max_size as f64).log2() as u32
}

// The generateMips function from the article on importing textures: render
// each mip level from the level above it. The JS version caches its module,
// sampler and per-format pipelines in a closure; we use a struct.
struct MipGenerator {
    sampler: wgpu::Sampler,
    module: wgpu::ShaderModule,
    pipeline_by_format: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
}

impl MipGenerator {
    fn new(device: &wgpu::Device) -> MipGenerator {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("textured quad shaders for mip level generation"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
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
            ..Default::default()
        });

        MipGenerator {
            sampler,
            module,
            pipeline_by_format: HashMap::new(),
        }
    }

    fn generate_mips(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
        let module = &self.module;
        let pipeline = self
            .pipeline_by_format
            .entry(texture.format())
            .or_insert_with(|| {
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
            let src_view = texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: base_mip_level - 1,
                mip_level_count: Some(1),
                ..Default::default()
            });
            let dst_view = texture.create_view(&wgpu::TextureViewDescriptor {
                base_mip_level,
                mip_level_count: Some(1),
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                ],
            });

            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("our basic canvas renderPass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &dst_view,
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
        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
    }
}

fn copy_source_to_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mip_gen: &mut MipGenerator,
    texture: &wgpu::Texture,
    source: &SourceImage,
    premultiplied_alpha: bool,
) {
    let image = if premultiplied_alpha {
        premultiply_alpha(source)
    } else {
        SourceImage {
            data: source.data.clone(),
            width: source.width,
            height: source.height,
        }
    };
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(image.width * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
    );

    if texture.mip_level_count() > 1 {
        mip_gen.generate_mips(device, queue, texture);
    }
}

fn create_texture_from_source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    mip_gen: &mut MipGenerator,
    source: &SourceImage,
    mips: bool,
    premultiplied_alpha: bool,
) -> wgpu::Texture {
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
            depth_or_array_layers: 1,
        },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    copy_source_to_texture(device, queue, mip_gen, &texture, source, premultiplied_alpha);
    texture
}

fn blend_operation(name: &str) -> wgpu::BlendOperation {
    match name {
        "add" => wgpu::BlendOperation::Add,
        "subtract" => wgpu::BlendOperation::Subtract,
        "reverse-subtract" => wgpu::BlendOperation::ReverseSubtract,
        "min" => wgpu::BlendOperation::Min,
        "max" => wgpu::BlendOperation::Max,
        _ => wgpu::BlendOperation::Add,
    }
}

fn blend_factor(name: &str) -> wgpu::BlendFactor {
    match name {
        "zero" => wgpu::BlendFactor::Zero,
        "one" => wgpu::BlendFactor::One,
        "src" => wgpu::BlendFactor::Src,
        "one-minus-src" => wgpu::BlendFactor::OneMinusSrc,
        "src-alpha" => wgpu::BlendFactor::SrcAlpha,
        "one-minus-src-alpha" => wgpu::BlendFactor::OneMinusSrcAlpha,
        "dst" => wgpu::BlendFactor::Dst,
        "one-minus-dst" => wgpu::BlendFactor::OneMinusDst,
        "dst-alpha" => wgpu::BlendFactor::DstAlpha,
        "one-minus-dst-alpha" => wgpu::BlendFactor::OneMinusDstAlpha,
        "src-alpha-saturated" => wgpu::BlendFactor::SrcAlphaSaturated,
        "constant" => wgpu::BlendFactor::Constant,
        "one-minus-constant" => wgpu::BlendFactor::OneMinusConstant,
        _ => wgpu::BlendFactor::One,
    }
}

// if the operation is min or max, srcFactor and dstFactor must be one or
// we'll get an error
fn make_blend_component_valid(blend: &mut wgpu::BlendComponent) {
    if blend.operation == wgpu::BlendOperation::Min || blend.operation == wgpu::BlendOperation::Max
    {
        blend.src_factor = wgpu::BlendFactor::One;
        blend.dst_factor = wgpu::BlendFactor::One;
    }
}

async fn run() {
    let mut app = App::new("WebGPU Blend").await;
    app.auto_resize = true;
    // the GUI on the page picks the canvas alphaMode at runtime; the JS
    // version calls context.configure with the choice every render, our
    // equivalent is having wgpu_fun ask for the current choice
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
    app.alpha_mode_fn = Some(Box::new(|| {
        match wgpu_fun::setting_str("alphaMode", "premultiplied").as_str() {
            "opaque" => wgpu::CompositeAlphaMode::Auto,
            _ => wgpu::CompositeAlphaMode::PreMultiplied,
        }
    }));

    let size = 300;
    let src_image = create_source_image(size);
    let dst_image = create_destination_image(size);

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("our hardcoded textured quad shaders"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
      struct OurVertexShaderOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      struct Uniforms {
        matrix: mat4x4f,
      };

      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> OurVertexShaderOutput {
        let pos = array(

          vec2f( 0.0,  0.0),  // center
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 0.0,  1.0),  // center, top

          // 2st triangle
          vec2f( 0.0,  1.0),  // center, top
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 1.0,  1.0),  // right, top
        );

        var vsOutput: OurVertexShaderOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy;
        return vsOutput;
      }

      @group(0) @binding(0) var ourSampler: sampler;
      @group(0) @binding(1) var ourTexture: texture_2d<f32>;

      @fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
        return textureSample(ourTexture, ourSampler, fsInput.texcoord);
      }
    "#
                .into(),
            ),
        });

    let mut mip_gen = MipGenerator::new(&app.device);

    let bind_group_layout = app
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let pipeline_layout = app
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

    let src_texture_unpremultiplied_alpha = create_texture_from_source(
        &app.device, &app.queue, &mut mip_gen, &src_image,
        true, false);
    let dst_texture_unpremultiplied_alpha = create_texture_from_source(
        &app.device, &app.queue, &mut mip_gen, &dst_image,
        true, false);

    let src_texture_premultiplied_alpha = create_texture_from_source(
        &app.device, &app.queue, &mut mip_gen, &src_image,
        true, true);
    let dst_texture_premultiplied_alpha = create_texture_from_source(
        &app.device, &app.queue, &mut mip_gen, &dst_image,
        true, true);

    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });

    fn make_uniform_buffer_and_values(device: &wgpu::Device) -> (wgpu::Buffer, [f32; 16]) {
        // create a buffer for the uniform values
        const UNIFORM_BUFFER_SIZE: u64 = 16 * 4; // matrix is 16 32bit floats (4bytes each)
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms for quad"),
            size: UNIFORM_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // create an array of f32s to hold the matrix for the uniforms in Rust
        let values = [0.0f32; 16];
        (buffer, values)
    }
    let (src_uniform_buffer, mut src_uniform_values) = make_uniform_buffer_and_values(&app.device);
    let (dst_uniform_buffer, mut dst_uniform_values) = make_uniform_buffer_and_values(&app.device);

    let make_bind_group = |texture: &wgpu::Texture, uniform_buffer: &wgpu::Buffer| {
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    };

    let src_bind_group_unpremultiplied_alpha =
        make_bind_group(&src_texture_unpremultiplied_alpha, &src_uniform_buffer);
    let dst_bind_group_unpremultiplied_alpha =
        make_bind_group(&dst_texture_unpremultiplied_alpha, &dst_uniform_buffer);
    let src_bind_group_premultiplied_alpha =
        make_bind_group(&src_texture_premultiplied_alpha, &src_uniform_buffer);
    let dst_bind_group_premultiplied_alpha =
        make_bind_group(&dst_texture_premultiplied_alpha, &dst_uniform_buffer);

    struct TextureSet {
        src_texture: wgpu::Texture,
        dst_texture: wgpu::Texture,
        src_bind_group: wgpu::BindGroup,
        dst_bind_group: wgpu::BindGroup,
    }

    let texture_sets = [
        TextureSet {
            src_texture: src_texture_premultiplied_alpha,
            dst_texture: dst_texture_premultiplied_alpha,
            src_bind_group: src_bind_group_premultiplied_alpha,
            dst_bind_group: dst_bind_group_premultiplied_alpha,
        },
        TextureSet {
            src_texture: src_texture_unpremultiplied_alpha,
            dst_texture: dst_texture_unpremultiplied_alpha,
            src_bind_group: src_bind_group_unpremultiplied_alpha,
            dst_bind_group: dst_bind_group_unpremultiplied_alpha,
        },
    ];

    let dst_pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hardcoded textured quad pipeline"),
            layout: Some(&pipeline_layout),
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

    app.run(RenderMode::Once, move |frame: &Frame| {
        // read the settings the GUI on the page sets
        let texture_set_ndx =
            (wgpu_fun::setting_f64("textureSet", 0.0) as usize).min(texture_sets.len() - 1);
        let mut color = wgpu::BlendComponent {
            operation: blend_operation(&wgpu_fun::setting_str("colorOperation", "add")),
            src_factor: blend_factor(&wgpu_fun::setting_str("colorSrcFactor", "one")),
            dst_factor: blend_factor(&wgpu_fun::setting_str("colorDstFactor", "one-minus-src")),
        };
        let mut alpha = wgpu::BlendComponent {
            operation: blend_operation(&wgpu_fun::setting_str("alphaOperation", "add")),
            src_factor: blend_factor(&wgpu_fun::setting_str("alphaSrcFactor", "one")),
            dst_factor: blend_factor(&wgpu_fun::setting_str("alphaDstFactor", "one-minus-src")),
        };
        let constant_color = [
            wgpu_fun::setting_f64("constantColor0", 1.0),
            wgpu_fun::setting_f64("constantColor1", 0.5),
            wgpu_fun::setting_f64("constantColor2", 0.25),
        ];
        let constant_alpha = wgpu_fun::setting_f64("constantAlpha", 1.0);
        let clear_color = [
            wgpu_fun::setting_f64("clearColor0", 0.0),
            wgpu_fun::setting_f64("clearColor1", 0.0),
            wgpu_fun::setting_f64("clearColor2", 0.0),
        ];
        let clear_alpha = wgpu_fun::setting_f64("clearAlpha", 0.0);
        let clear_premultiply = wgpu_fun::setting_bool("clearPremultiply", true);

        make_blend_component_valid(&mut color);
        make_blend_component_valid(&mut alpha);

        // blend state is baked into a pipeline, so the srcPipeline is
        // created here, at render time, with whatever blend settings are
        // currently selected
        let src_pipeline = frame
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("hardcoded textured quad pipeline"),
                layout: Some(&pipeline_layout),
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
                    targets: &[Some(wgpu::ColorTargetState {
                        format: frame.format,
                        blend: Some(wgpu::BlendState { color, alpha }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });

        let TextureSet {
            src_texture,
            dst_texture,
            src_bind_group,
            dst_bind_group,
        } = &texture_sets[texture_set_ndx];

        let mult = if clear_premultiply { clear_alpha } else { 1.0 };
        let clear_value = wgpu::Color {
            r: clear_color[0] * mult,
            g: clear_color[1] * mult,
            b: clear_color[2] * mult,
            a: clear_alpha,
        };

        let update_uniforms =
            |uniform_buffer: &wgpu::Buffer, values: &mut [f32; 16], texture: &wgpu::Texture| {
                let projection_matrix = glam::camera::rh::proj::directx::orthographic(
                    0.0,
                    frame.width as f32,
                    frame.height as f32,
                    0.0,
                    -1.0,
                    1.0,
                );
                let matrix = projection_matrix
                    * Mat4::from_scale(vec3(texture.width() as f32, texture.height() as f32, 1.0));
                values.copy_from_slice(&matrix.to_cols_array());

                // copy the values from Rust to the GPU
                frame
                    .queue
                    .write_buffer(uniform_buffer, 0, bytemuck::cast_slice(values));
            };
        update_uniforms(&src_uniform_buffer, &mut src_uniform_values, src_texture);
        update_uniforms(&dst_uniform_buffer, &mut dst_uniform_values, dst_texture);

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render quad encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_value),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            // draw dst
            pass.set_pipeline(&dst_pipeline);
            pass.set_bind_group(0, dst_bind_group, &[]);
            pass.draw(0..6, 0..1); // call our vertex shader 6 times

            // draw src
            pass.set_pipeline(&src_pipeline);
            pass.set_bind_group(0, src_bind_group, &[]);
            pass.set_blend_constant(wgpu::Color {
                r: constant_color[0],
                g: constant_color[1],
                b: constant_color[2],
                a: constant_alpha,
            });
            pass.draw(0..6, 0..1); // call our vertex shader 6 times
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
