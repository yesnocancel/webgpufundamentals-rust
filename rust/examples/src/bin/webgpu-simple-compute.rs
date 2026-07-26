use wgpu_fun::print;

async fn main_async() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
    else {
        fail("need a browser that supports WebGPU");
        return;
    };
    let Ok((device, queue)) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
    else {
        fail("need a browser that supports WebGPU");
        return;
    };

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("doubling compute module"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
      @group(0) @binding(0) var<storage, read_write> data: array<f32>;

      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        let i = id.x;
        data[i] = data[i] * 2.0;
      }
    "#
            .into(),
        ),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("doubling compute pipeline"),
        layout: None,
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        cache: None,
    });

    let input: [f32; 3] = [1.0, 3.0, 5.0];

    // create a buffer on the GPU to hold our computation
    // input and output
    let work_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("work buffer"),
        size: std::mem::size_of_val(&input) as u64,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Copy our input data to that buffer
    queue.write_buffer(&work_buffer, 0, bytemuck::cast_slice(&input));

    // create a buffer on the GPU to get a copy of the results
    let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("result buffer"),
        size: std::mem::size_of_val(&input) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Setup a bindGroup to tell the shader which
    // buffer to use for the computation
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bindGroup for work buffer"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: work_buffer.as_entire_binding(),
        }],
    });

    // Encode commands to do the computation
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("doubling encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("doubling compute pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(input.len() as u32, 1, 1);
    }

    // Encode a command to copy the results to a mappable buffer.
    encoder.copy_buffer_to_buffer(&work_buffer, 0, &result_buffer, 0, result_buffer.size());

    // Finish encoding and submit the commands
    let command_buffer = encoder.finish();
    queue.submit([command_buffer]);

    // Read the results
    wgpu_fun::map_async(&device, &result_buffer, wgpu::MapMode::Read).await;
    let result: Vec<f32> = {
        let data = result_buffer.slice(..).get_mapped_range().unwrap();
        bytemuck::cast_slice(&data).to_vec()
    };

    print(&format!("input {input:?}"));
    print(&format!("result {result:?}"));

    result_buffer.unmap();
}

fn fail(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    wgpu_fun::fail(msg);
    #[cfg(not(target_arch = "wasm32"))]
    panic!("{msg}");
}

fn main() {
    wgpu_fun::start(main_async());
}
