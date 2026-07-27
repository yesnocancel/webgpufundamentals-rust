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
        label: Some("add elements compute module"),
        source: wgpu::ShaderSource::Wgsl(
            /* wgsl */ r#"
      @group(0) @binding(0) var<storage, read_write> a: array<f32>;
      @group(0) @binding(1) var<storage, read_write> b: array<f32>;
      @group(0) @binding(2) var<storage, read_write> dst: array<f32>;

      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        let i = id.x;
        dst[i] = a[i] + b[i];
      }
    "#
            .into(),
        ),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("add elements compute pipeline"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        cache: None,
    });

    let mut input = [0.0f32; 64 * 3];
    input[0..3].copy_from_slice(&[1.0, 3.0, 5.0]);
    input[64..64 + 3].copy_from_slice(&[11.0, 12.0, 13.0]);

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
    // buffers to use for the computation
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bindGroup for work buffer"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &work_buffer,
                    offset: 0,
                    size: Some(wgpu::BufferSize::new(256).unwrap()),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &work_buffer,
                    offset: 0,
                    size: Some(wgpu::BufferSize::new(256).unwrap()),
                }),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &work_buffer,
                    offset: 0,
                    size: Some(wgpu::BufferSize::new(256).unwrap()),
                }),
            },
        ],
    });

    // Encode commands to do the computation
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("adding encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("adding compute pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[0, 256, 512]);
        pass.dispatch_workgroups(3, 1, 1);
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
    result_buffer.unmap();

    print(&format!("a {:?}", &input[0..3]));
    print(&format!("b {:?}", &input[64..64 + 3]));
    print(&format!("dst {:?}", &result[128..128 + 3]));
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
