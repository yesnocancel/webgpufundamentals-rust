use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    // Like the JS version checks for the 'bgra8unorm-storage' feature, we
    // request it if the adapter supports it so we can use the surface's
    // bgra8unorm texture as a storage texture.
    let mut app = App::new_with_features(
        "WebGPU Storage Texture",
        wgpu::Features::BGRA8UNORM_STORAGE,
    )
    .await;
    app.auto_resize = true;
    // TEXTURE_BINDING so the canvas itself can be displayed,
    // STORAGE_BINDING so we can write to it from the compute shader.
    app.usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING;

    if app.format == wgpu::TextureFormat::Bgra8Unorm
        && !app.device.features().contains(wgpu::Features::BGRA8UNORM_STORAGE)
    {
        panic!("bgra8unorm-storage is not supported");
    }

    let format_name = match app.format {
        wgpu::TextureFormat::Rgba8Unorm => "rgba8unorm",
        wgpu::TextureFormat::Bgra8Unorm => "bgra8unorm",
        f => panic!("unsupported canvas format {f:?}"),
    };

    // The storage texture's format must be in the shader itself, so we
    // splice it in, like the JS version's template literal.
    let code = /* wgsl */ format!(
        "
      @group(0) @binding(0)
      var tex: texture_storage_2d<{format_name}, write>;
"
    ) + &r#"
      @compute @workgroup_size(1) fn cs(
        @builtin(global_invocation_id) id : vec3u
      )  {
        let size = textureDimensions(tex);
        let center = vec2f(size) / 2.0;

        // the pixel we're going to write to
        let pos = id.xy;

        // The distance from the center of the texture
        let dist = distance(vec2f(pos), center);

        // Compute stripes based on the distance
        let stripe = dist / 32.0 % 2.0;
        let red = vec4f(1, 0, 0, 1);
        let cyan = vec4f(0, 1, 1, 1);
        let color = select(red, cyan, stripe < 1.0);

        // Write the color to the texture
        textureStore(tex, pos, color);
      }
    "#;

    let module = app
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("circles in storage texture"),
            source: wgpu::ShaderSource::Wgsl(code.into()),
        });

    let pipeline = app
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("circles in storage texture"),
            layout: None,
            module: &module,
            entry_point: None,
            compilation_options: Default::default(),
            cache: None,
        });

    app.run(RenderMode::Once, move |frame: &Frame| {
        let bind_group = frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(frame.view),
            }],
        });

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("our encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(frame.width, frame.height, 1);
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
