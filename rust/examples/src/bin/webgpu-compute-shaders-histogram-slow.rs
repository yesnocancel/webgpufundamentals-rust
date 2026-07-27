use wgpu_fun::{App, Frame, ImageData, RenderMode};

// The JS version appends two canvases to the page: one showing the image and
// one showing the histogram (drawn with the 2D canvas API). We have one
// WebGPU canvas, so we draw both into it: the image on top and the
// histogram below it, each as a textured quad in pixel space.

fn histogram_to_image(histogram: &[u32], num_entries: u32, height: usize) -> ImageData {
    let num_bins = histogram.len();
    let max = *histogram.iter().max().unwrap();
    let scale = (1.0 / max as f32).max(0.2 * num_bins as f32 / num_entries as f32);
    let mut data = vec![0u8; num_bins * height * 4];
    for x in 0..num_bins {
        let v = (histogram[x] as f32 * scale * height as f32) as usize;
        for y in (height - v.min(height))..height {
            let o = (y * num_bins + x) * 4;
            data[o..o + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    ImageData {
        data,
        width: num_bins as u32,
        height: height as u32,
    }
}

fn create_texture_from_source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &ImageData,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        format: wgpu::TextureFormat::Rgba8Unorm,
        size: wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &source.data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(source.width * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: source.width,
            height: source.height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

const DRAW_SHADER: &str = r#"
      struct Uniforms {
        rect: vec4f,        // x, y, width, height in pixels
        resolution: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @group(0) @binding(0) var<uniform> uni: Uniforms;
      @group(0) @binding(1) var s: sampler;
      @group(0) @binding(2) var t: texture_2d<f32>;

      @vertex fn vs(@builtin(vertex_index) vNdx: u32) -> VSOutput {
        let corners = array(
          vec2f(0, 0), vec2f(1, 0), vec2f(0, 1),
          vec2f(0, 1), vec2f(1, 0), vec2f(1, 1),
        );
        let c = corners[vNdx];
        let px = uni.rect.xy + c * uni.rect.zw;
        let clip = (px / uni.resolution * 2.0 - 1.0) * vec2f(1, -1);
        var vsOut: VSOutput;
        vsOut.position = vec4f(clip, 0, 1);
        vsOut.texcoord = c;
        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        return textureSample(t, s, vsOut.texcoord);
      }
"#;

struct DrawnImage {
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    width: u32,
    height: u32,
}

fn show_images(app: App, images: Vec<wgpu::Texture>) {
    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(DRAW_SHADER.into()),
        });
    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("draw image"),
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
    let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let drawn: Vec<DrawnImage> = images
        .iter()
        .map(|texture| {
            let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (4 + 2 + 2) * 4,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
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
            DrawnImage {
                bind_group,
                uniform_buffer,
                width: texture.width(),
                height: texture.height(),
            }
        })
        .collect();

    app.run(RenderMode::Once, move |frame: &Frame| {
        let mut encoder = frame
            .device
            .create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
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
            // stack the images top to bottom, like the JS version's canvases
            let mut y = 0.0f32;
            for image in &drawn {
                // scale the image down if it's wider than the canvas
                let scale = (frame.width as f32 / image.width as f32).min(1.0);
                let (w, h) = (image.width as f32 * scale, image.height as f32 * scale);
                let uniforms: [f32; 8] = [
                    0.0,
                    y,
                    w,
                    h,
                    frame.width as f32,
                    frame.height as f32,
                    0.0,
                    0.0,
                ];
                frame
                    .queue
                    .write_buffer(&image.uniform_buffer, 0, bytemuck::cast_slice(&uniforms));
                pass.set_bind_group(0, &image.bind_group, &[]);
                pass.draw(0..6, 0..1);
                y += h;
            }
        }
        frame.queue.submit([encoder.finish()]);
    });
}

async fn run() {
    let mut app = App::new("WebGPU Compute Shaders Histogram (slow)").await;
    app.auto_resize = true;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("histogram shader"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      @group(0) @binding(0) var<storage, read_write> bins: array<u32>;
      @group(0) @binding(1) var ourTexture: texture_2d<f32>;

      // from: https://www.w3.org/WAI/GL/wiki/Relative_luminance
      const kSRGBLuminanceFactors = vec3f(0.2126, 0.7152, 0.0722);
      fn srgbLuminance(color: vec3f) -> f32 {
        return saturate(dot(color, kSRGBLuminanceFactors));
      }

      @compute @workgroup_size(1) fn cs() {
        let size = textureDimensions(ourTexture, 0);
        let numBins = f32(arrayLength(&bins));
        let lastBinIndex = u32(numBins - 1);
        for (var y = 0u; y < size.y; y++) {
          for (var x = 0u; x < size.x; x++) {
            let position = vec2u(x, y);
            let color = textureLoad(ourTexture, position, 0);
            let v = srgbLuminance(color.rgb);
            let bin = min(u32(v * numBins), lastBinIndex);
            bins[bin] += 1;
          }
        }
      }
    "#
                .into(),
            ),
        });

    let pipeline = app
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("histogram"),
            layout: None,
            module: &module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

    let img =
        wgpu_fun::load_image("resources/images/pexels-francesco-ungaro-96938-mid.jpg").await;
    let texture = create_texture_from_source(&app.device, &app.queue, &img);

    let num_bins = 256u32;
    let histogram_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (num_bins * 4) as u64, // 256 entries * 4 bytes per (u32)
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: histogram_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("histogram bindGroup"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: histogram_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
        ],
    });

    let mut encoder = app
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("histogram encoder"),
        });
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&histogram_buffer, 0, &result_buffer, 0, result_buffer.size());
    let command_buffer = encoder.finish();
    app.queue.submit([command_buffer]);

    wgpu_fun::map_async(&app.device, &result_buffer, wgpu::MapMode::Read).await;
    let histogram: Vec<u32> = {
        let data = result_buffer.slice(..).get_mapped_range().unwrap();
        bytemuck::cast_slice(&data).to_vec()
    };
    result_buffer.unmap();

    let num_entries = texture.width() * texture.height();
    let histogram_image = histogram_to_image(&histogram, num_entries, 100);
    let histogram_texture = create_texture_from_source(&app.device, &app.queue, &histogram_image);

    show_images(app, vec![texture, histogram_texture]);
}

fn main() {
    wgpu_fun::start(run());
}
