use wgpu_fun::log;

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

    let dispatch_count: [u32; 3] = [4, 3, 2];
    let workgroup_size: [u32; 3] = [2, 3, 4];

    // multiply all elements of an array
    let array_prod = |arr: [u32; 3]| arr.iter().product::<u32>();

    let num_threads_per_workgroup = array_prod(workgroup_size);

    // Build the WGSL with the workgroup size hard coded into the shader,
    // like the JS version's template literal.
    let [wx, wy, wz] = workgroup_size;
    let code = format!(
        "
  // NOTE!: vec3u is padded to by 4 bytes
  @group(0) @binding(0) var<storage, read_write> workgroupResult: array<vec3u>;
  @group(0) @binding(1) var<storage, read_write> localResult: array<vec3u>;
  @group(0) @binding(2) var<storage, read_write> globalResult: array<vec3u>;

  @compute @workgroup_size({wx}, {wy}, {wz}) fn computeSomething("
    ) + &r#"
      @builtin(workgroup_id) workgroup_id : vec3<u32>,
      @builtin(local_invocation_id) local_invocation_id : vec3<u32>,
      @builtin(global_invocation_id) global_invocation_id : vec3<u32>,
      @builtin(local_invocation_index) local_invocation_index: u32,
      @builtin(num_workgroups) num_workgroups: vec3<u32>
  ) {
    // workgroup_index is similar to local_invocation_index except for
    // workgroups, not threads inside a workgroup.
    // It is not a builtin so we compute it ourselves.

    let workgroup_index =
       workgroup_id.x +
       workgroup_id.y * num_workgroups.x +
       workgroup_id.z * num_workgroups.x * num_workgroups.y;

    // global_invocation_index is like local_invocation_index
    // except linear across all invocations across all dispatched
    // workgroups. It is not a builtin so we compute it ourselves.

    let global_invocation_index =
       workgroup_index * NUM_THREADS_PER_WORKGROUP +
       local_invocation_index;

    // now we can write each of these builtins to our buffers.
    workgroupResult[global_invocation_index] = workgroup_id;
    localResult[global_invocation_index] = local_invocation_id;
    globalResult[global_invocation_index] = global_invocation_id;
  }
  "#
    .replace(
        "NUM_THREADS_PER_WORKGROUP",
        &num_threads_per_workgroup.to_string(),
    );

    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(code.into()),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute pipeline"),
        layout: None,
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        cache: None,
    });

    let num_workgroups = array_prod(dispatch_count);
    let num_results = num_workgroups * num_threads_per_workgroup;
    let size = (num_results * 4 * 4) as u64; // vec3u * u32

    let mut usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;
    let make_buffer = |usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let workgroup_buffer = make_buffer(usage);
    let local_buffer = make_buffer(usage);
    let global_buffer = make_buffer(usage);

    usage = wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST;
    let workgroup_read_buffer = make_buffer(usage);
    let local_read_buffer = make_buffer(usage);
    let global_read_buffer = make_buffer(usage);

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: workgroup_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: local_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: global_buffer.as_entire_binding(),
            },
        ],
    });

    // Encode commands to do the computation
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("compute builtin encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute builtin pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let [dx, dy, dz] = dispatch_count;
        pass.dispatch_workgroups(dx, dy, dz);
    }

    encoder.copy_buffer_to_buffer(&workgroup_buffer, 0, &workgroup_read_buffer, 0, size);
    encoder.copy_buffer_to_buffer(&local_buffer, 0, &local_read_buffer, 0, size);
    encoder.copy_buffer_to_buffer(&global_buffer, 0, &global_read_buffer, 0, size);

    // Finish encoding and submit the commands
    let command_buffer = encoder.finish();
    queue.submit([command_buffer]);

    // Read the results. We must wait for all 3 buffers to be mapped, which
    // is what awaiting each in turn does.
    wgpu_fun::map_async(&device, &workgroup_read_buffer, wgpu::MapMode::Read).await;
    wgpu_fun::map_async(&device, &local_read_buffer, wgpu::MapMode::Read).await;
    wgpu_fun::map_async(&device, &global_read_buffer, wgpu::MapMode::Read).await;

    let workgroup_range = workgroup_read_buffer.slice(..).get_mapped_range().unwrap();
    let local_range = local_read_buffer.slice(..).get_mapped_range().unwrap();
    let global_range = global_read_buffer.slice(..).get_mapped_range().unwrap();
    let workgroup: &[u32] = bytemuck::cast_slice(&workgroup_range);
    let local: &[u32] = bytemuck::cast_slice(&local_range);
    let global: &[u32] = bytemuck::cast_slice(&global_range);

    let get3 = |arr: &[u32], i: u32| {
        let off = (i * 4) as usize;
        format!("{}, {}, {}", arr[off], arr[off + 1], arr[off + 2])
    };

    for i in 0..num_results {
        if i % num_threads_per_workgroup == 0 {
            log(&format!(
                "\
 ---------------------------------------
 global                 local     global   dispatch: {}
 invoc.    workgroup    invoc.    invoc.
 index     id           id        id
 ---------------------------------------",
                i / num_threads_per_workgroup
            ));
        }
        log(&format!(
            " {:3}:      {}      {}   {}",
            i,
            get3(workgroup, i),
            get3(local, i),
            get3(global, i)
        ));
    }
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
