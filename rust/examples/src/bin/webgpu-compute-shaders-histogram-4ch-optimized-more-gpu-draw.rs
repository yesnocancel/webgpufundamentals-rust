use wgpu_fun::{App, Frame, ImageData, RenderMode};

// The JS version appends two canvases to the page: one showing the image and
// one showing the histogram (drawn with the 2D canvas API). We have one
// WebGPU canvas, so we draw both into it: the image on top and the
// histogram below it, each as a textured quad in pixel space.

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
    let mut app = App::new("Histogram 4ch (gpu draw)").await;
    app.auto_resize = true;

    let k_chunk_width = 256u32;
    let k_chunk_height = 1u32;
    let shared_constants = format!(
        "
      const chunkWidth = {k_chunk_width};
      const chunkHeight = {k_chunk_height};
"
    );

    let histogram_chunk_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("histogram chunk shader"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ (shared_constants.clone()
                    + r#"
      const chunkSize = chunkWidth * chunkHeight;
      var<workgroup> bins: array<array<atomic<u32>, 4>, chunkSize>;
      @group(0) @binding(0) var<storage, read_write> chunks: array<array<vec4u, chunkSize>>;
      @group(0) @binding(1) var ourTexture: texture_2d<f32>;

      const kSRGBLuminanceFactors = vec3f(0.2126, 0.7152, 0.0722);
      fn srgbLuminance(color: vec3f) -> f32 {
        return saturate(dot(color, kSRGBLuminanceFactors));
      }

      @compute @workgroup_size(chunkWidth, chunkHeight, 1)
      fn cs(
        @builtin(workgroup_id) workgroup_id: vec3u,
        @builtin(local_invocation_id) local_invocation_id: vec3u,
      ) {
        let size = textureDimensions(ourTexture, 0);
        let position = workgroup_id.xy * vec2u(chunkWidth, chunkHeight) + 
                       local_invocation_id.xy;
        if (all(position < size)) {
          let numBins = f32(chunkSize);
          let lastBinIndex = u32(numBins - 1);
          var channels = textureLoad(ourTexture, position, 0);
          channels.w = srgbLuminance(channels.rgb);
          for (var ch = 0; ch < 4; ch++) {
            let v = channels[ch];
            let bin = min(u32(v * numBins), lastBinIndex);
            atomicAdd(&bins[bin][ch], 1u);
          }
        }

        workgroupBarrier();

        let chunksAcross = (size.x + chunkWidth - 1) / chunkWidth;
        let chunk = workgroup_id.y * chunksAcross + workgroup_id.x;
        let bin = local_invocation_id.y * chunkWidth + local_invocation_id.x;

        chunks[chunk][bin] = vec4u(
          atomicLoad(&bins[bin][0]),
          atomicLoad(&bins[bin][1]),
          atomicLoad(&bins[bin][2]),
          atomicLoad(&bins[bin][3]),
        );
      }
    "#)
                .into(),
            ),
        });

    let chunk_sum_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("chunk sum shader"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ (shared_constants
                    + r#"
      const chunkSize = chunkWidth * chunkHeight;

      struct Uniforms {
        stride: u32,
      };

      @group(0) @binding(0) var<storage, read_write> chunks: array<array<vec4u, chunkSize>>;
      @group(0) @binding(1) var<uniform> uni: Uniforms;

      @compute @workgroup_size(chunkSize, 1, 1) fn cs(
        @builtin(local_invocation_id) local_invocation_id: vec3u,
        @builtin(workgroup_id) workgroup_id: vec3u,
      ) {
        let chunk0 = workgroup_id.x * uni.stride * 2;
        let chunk1 = chunk0 + uni.stride;

        let sum = chunks[chunk0][local_invocation_id.x] +
                  chunks[chunk1][local_invocation_id.x];
        chunks[chunk0][local_invocation_id.x] = sum;
      }
    "#)
                .into(),
            ),
        });

    let histogram_chunk_pipeline = app
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("histogram"),
            layout: None,
            module: &histogram_chunk_module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

    let chunk_sum_pipeline = app
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("chunk sum"),
            layout: None,
            module: &chunk_sum_module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

    let scale_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("histogram scale shader"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      @group(0) @binding(0) var<storage, read> bins: array<vec4u>;
      @group(0) @binding(1) var<storage, read_write> scale: vec4f;
      @group(0) @binding(2) var ourTexture: texture_2d<f32>;

      @compute @workgroup_size(1, 1, 1) fn cs() {
        let size = textureDimensions(ourTexture, 0);
        let numEntries = f32(size.x * size.y);

        var m = vec4u(0);
        let numBins = arrayLength(&bins);
        for (var i = 0u ; i < numBins; i++) {
          m = max(m, bins[i]);
        }
        scale = max(1.0 / vec4f(m), vec4f(0.2 * f32(numBins) / numEntries));
      }
    "#
                .into(),
            ),
        });

    let draw_histogram_module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("draw histogram shader"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      struct OurVertexShaderOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      struct Uniforms {
        matrix: mat4x4f,
        colors: array<vec4f, 16>,
        channelMult: vec4u,
      };

      @group(0) @binding(0) var<storage, read> bins: array<vec4u>;
      @group(0) @binding(1) var<uniform> uni: Uniforms;
      @group(0) @binding(2) var<storage, read_write> scale: vec4f;

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

      @fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
        let numBins = arrayLength(&bins);
        let lastBinIndex = u32(numBins - 1);
        let bin = clamp(
            u32(fsInput.texcoord.x * f32(numBins)),
            0,
            lastBinIndex);
        let heights = vec4f(bins[bin]) * scale;
        let bits = heights > vec4f(fsInput.texcoord.y);
        let ndx = dot(select(vec4u(0), uni.channelMult, bits), vec4u(1));
        return uni.colors[ndx];
      }
    "#
                .into(),
            ),
        });

    let scale_pipeline = app
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("scale"),
            layout: None,
            module: &scale_module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

    let draw_histogram_pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("draw histogram"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &draw_histogram_module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_histogram_module,
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


    let img =
        wgpu_fun::load_image("resources/images/pexels-francesco-ungaro-96938-mid.jpg").await;
    let texture = create_texture_from_source(&app.device, &app.queue, &img);

    let chunk_size = k_chunk_width * k_chunk_height;
    let chunks_across = texture.width().div_ceil(k_chunk_width);
    let chunks_down = texture.height().div_ceil(k_chunk_height);
    let num_chunks = chunks_across * chunks_down;

    let chunks_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (num_chunks * chunk_size * 4 * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let scale_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scale buffer"),
        size: 4 * 4,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let mut sum_bind_groups = Vec::new();
    let num_steps = (num_chunks as f32).log2().ceil() as u32;
    for i in 0..num_steps {
        let stride = 2u32.pow(i);
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 4,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        app.queue
            .write_buffer(&uniform_buffer, 0, bytemuck::bytes_of(&stride));

        let chunk_sum_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("sum bindGroup {i}")),
            layout: &chunk_sum_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunks_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        sum_bind_groups.push(chunk_sum_bind_group);
    }

    let chunks_binding = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
        buffer: &chunks_buffer,
        offset: 0,
        size: wgpu::BufferSize::new((chunk_size * 4 * 4) as u64),
    });

    let histogram_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("histogram bindGroup"),
        layout: &histogram_chunk_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: chunks_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(
                    &texture.create_view(&Default::default()),
                ),
            },
        ],
    });

    let scale_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scale bindGroup"),
        layout: &scale_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: chunks_binding.clone(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: scale_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
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

        // create a histogram for each chunk
        pass.set_pipeline(&histogram_chunk_pipeline);
        pass.set_bind_group(0, &histogram_bind_group, &[]);
        pass.dispatch_workgroups(chunks_across, chunks_down, 1);

        // reduce the chunks
        pass.set_pipeline(&chunk_sum_pipeline);
        let mut chunks_left = num_chunks;
        for bind_group in &sum_bind_groups {
            pass.set_bind_group(0, bind_group, &[]);
            let dispatch_count = chunks_left / 2;
            chunks_left -= dispatch_count;
            pass.dispatch_workgroups(dispatch_count, 1, 1);
        }

        // compute scales for the channels
        pass.set_pipeline(&scale_pipeline);
        pass.set_bind_group(0, &scale_bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    app.queue.submit([encoder.finish()]);

    // Draw a histogram entirely on the GPU into its own texture.
    let draw_histogram = |channels: &[usize], height: u32| -> wgpu::Texture {
        let num_bins = chunk_size;

        //  matrix: mat4x4f;
        //  colors: array<vec4f, 16>;
        //  channelMult; vec4u,
        let mut uniform_values_f32 = [0.0f32; 16 + 64 + 4 + 4];
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("draw histogram uniform buffer"),
            size: (uniform_values_f32.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        #[rustfmt::skip]
        uniform_values_f32[16..16 + 64].copy_from_slice(&[
            0.0, 0.0, 0.0, 1.0,
            1.0, 0.0, 0.0, 1.0,
            0.0, 1.0, 0.0, 1.0,
            1.0, 1.0, 0.0, 1.0,
            0.0, 0.0, 1.0, 1.0,
            1.0, 0.0, 1.0, 1.0,
            0.0, 1.0, 1.0, 1.0,
            0.5, 0.5, 0.5, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
            1.0, 1.0, 1.0, 1.0,
        ]);
        let channel_mult: [u32; 4] =
            std::array::from_fn(|i| if channels.contains(&i) { 2u32.pow(i as u32) } else { 0 });
        uniform_values_f32[16 + 64..16 + 64 + 4]
            .copy_from_slice(bytemuck::cast_slice(&channel_mult));
        // matrix: cover clip space
        let matrix = glam::Mat4::from_translation(glam::Vec3::new(-1.0, -1.0, 0.0))
            * glam::Mat4::from_scale(glam::Vec3::new(2.0, 2.0, 1.0));
        uniform_values_f32[0..16].copy_from_slice(&matrix.to_cols_array());
        app.queue
            .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values_f32));

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &draw_histogram_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunks_binding.clone(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: scale_buffer.as_entire_binding(),
                },
            ],
        });

        // In the JS version each histogram gets its own canvas; we render
        // into a texture and composite them into our one canvas below.
        let target = app.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: num_bins,
                height,
                depth_or_array_layers: 1,
            },
            format: app.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
        });

        let mut encoder = app
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render histogram"),
            });
        {
            let view = target.create_view(&Default::default());
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            pass.set_pipeline(&draw_histogram_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1); // call our vertex shader 6 times
        }
        app.queue.submit([encoder.finish()]);
        target
    };

    // draw the red, green, and blue channels
    let color_histogram = draw_histogram(&[0, 1, 2], 100);
    // draw the luminosity channel
    let luminosity_histogram = draw_histogram(&[3], 100);

    show_images(app, vec![texture, color_histogram, luminosity_histogram]);

}

fn main() {
    wgpu_fun::start(run());
}
