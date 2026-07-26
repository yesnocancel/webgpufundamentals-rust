Title: WebGPU Compute Shader Basics
Description: How to use compute shaders in WebGPU
TOC: Compute Shader Basics

This article continues from [the article on fundamentals](webgpu-fundamentals.html).
We're going to start with some basic of compute shaders and then hopefully move on
to examples of solving real world problems.

In the [previous article](webgpu-fundamentals.html) we made an extremely simple
compute shader that doubled numbers in place.

Here's the shader

```wgsl
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(1) fn computeSomething(
  @builtin(global_invocation_id) id: vec3<u32>
) {
  let i = id.x;
  data[i] = data[i] * 2.0;
}
```

We then effectively ran the compute shader like this

```rust
  ...
  pass.dispatch_workgroups(count, 1, 1);
```

We need to go over the definition of workgroup.

You can think of a workgroup as small collection of threads. Each thread
runs in parallel. You define the size of workgroup statically in WGSL.
Workgroup sizes are defined in 3 dimensions but default to 1 so
our `@workgroup_size(1)` is equivalent to `@workgroup_size(1, 1, 1)`.

<a id="a-local-invocation-id"></a>If we define a workgroup as say `@workgroup_size(3, 4, 2)` then we're
defining 3 * 4 * 2 threads or another way put it, we're defining a 24 thread workgroup.

<div class="webgpu_center">
  <img src="resources/gpu-workgroup.svg" style="width: 500px;">
  <div><code>local_invocation_id</code> of threads in a workgroup</div>
</div>

<a id="a-workgroup-id"></a>If we then call `pass.dispatchWorkgroups(4, 3, 2)` we're saying, execute a workgroup of 24 threads,
4 * 3 * 2 times (24) for a total of 576 threads.

<div class="webgpu_center">
  <img src="resources/gpu-workgroup-dispatch.svg" style="width: 500px;">
  <div><code>workgroup_id</code> of dispatched workgroups</div>
</div>

Inside each "invocation" of our compute shader the following builtin variables
are available.

* `local_invocation_id`: The id of this thread within a workgroup

  [See the diagram above](#a-local-invocation-id).

* `workgroup_id`: The id of the workgroup.

  Every thread within a workgroup will have the same workgroup id.
  [See the diagram above](#a-workgroup-id).

* `global_invocation_id`: A unique id for each thread

  You can think of this as

  ```
  global_invocation_id = workgroup_id * workgroup_size + local_invocation_id
  ```

* `num_workgroups`: What you passed to `pass.dispatch_workgroups`

* `local_invocation_index`: The id of this thread linearized

  You can think of this as

  ```
  rowSize = workgroup_size.x
  sliceSize = rowSize * workgroup_size.y
  local_invocation_index =
        local_invocation_id.x +
        local_invocation_id.y * rowSize +
        local_invocation_id.z * sliceSize
  ```

Let's make a sample to use these values. We'll just write the values
from each invocation to buffers and then print out the values

Here's the shader

```rust
let dispatch_count: [u32; 3] = [4, 3, 2];
let workgroup_size: [u32; 3] = [2, 3, 4];

// multiply all elements of an array
let array_prod = |arr: [u32; 3]| arr.iter().product::<u32>();

let num_threads_per_workgroup = array_prod(workgroup_size);

let [wx, wy, wz] = workgroup_size;
let code = format!("
  // NOTE!: vec3u is padded to by 4 bytes
  @group(0) @binding(0) var<storage, read_write> workgroupResult: array<vec3u>;
  @group(0) @binding(1) var<storage, read_write> localResult: array<vec3u>;
  @group(0) @binding(2) var<storage, read_write> globalResult: array<vec3u>;

  @compute @workgroup_size({wx}, {wy}, {wz}) fn computeSomething(") + &r#"
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
  "#.replace("NUM_THREADS_PER_WORKGROUP", &num_threads_per_workgroup.to_string());
```

We used Rust's `format!` for the first part of the WGSL so we can set the
workgroup size from the Rust variable `workgroup_size`, and a plain
`replace` to substitute the thread count into the rest (a `format!` across
the whole shader would make us escape every `{` in the WGSL). Either way,
the values end up hard coded into the shader.

Now that we have the shader we can make 3 buffers to store these results.

```rust
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
```

As we pointed out before, we can not map storage buffers to read them
directly so we need some buffers we can map. We'll copy
the results from the storage buffers to these mappable result
buffers and then read the results.

```rust
  usage = wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST;
  let workgroup_read_buffer = make_buffer(usage);
  let local_read_buffer = make_buffer(usage);
  let global_read_buffer = make_buffer(usage);
```

We make a bindgroup to bind all our storage buffers

```rust
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: workgroup_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: local_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 2, resource: global_buffer.as_entire_binding() },
    ],
  });
```

We start an encoder and a compute pass encoder, the same as our previous
example, then add the commands to run the compute shader.

```rust
  // Encode commands to do the computation
  let mut encoder = device.create_command_encoder(
    &wgpu::CommandEncoderDescriptor { label: Some("compute builtin encoder") });
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
```

We need to copy the results from the storage buffers to the mappable 
result buffers.

```rust
  encoder.copy_buffer_to_buffer(&workgroup_buffer, 0, &workgroup_read_buffer, 0, size);
  encoder.copy_buffer_to_buffer(&local_buffer, 0, &local_read_buffer, 0, size);
  encoder.copy_buffer_to_buffer(&global_buffer, 0, &global_read_buffer, 0, size);
```

And then end the encoder and submit the command buffer.

```rust
  // Finish encoding and submit the commands
  let command_buffer = encoder.finish();
  queue.submit([command_buffer]);
```

Like before, to read the results we map the buffers and once they are
ready we get typed array views of their contents.

```rust
  // Read the results
  wgpu_fun::map_async(&device, &workgroup_read_buffer, wgpu::MapMode::Read).await;
  wgpu_fun::map_async(&device, &local_read_buffer, wgpu::MapMode::Read).await;
  wgpu_fun::map_async(&device, &global_read_buffer, wgpu::MapMode::Read).await;

  let workgroup_range = workgroup_read_buffer.slice(..).get_mapped_range().unwrap();
  let local_range = local_read_buffer.slice(..).get_mapped_range().unwrap();
  let global_range = global_read_buffer.slice(..).get_mapped_range().unwrap();
  let workgroup: &[u32] = bytemuck::cast_slice(&workgroup_range);
  let local: &[u32] = bytemuck::cast_slice(&local_range);
  let global: &[u32] = bytemuck::cast_slice(&global_range);
```

> Important: We mapped 3 buffers here and awaited each of them. You can
> **NOT** just wait on the last buffer. You must wait on all 3 buffers.

Finally we can print them out

```rust
  let get3 = |arr: &[u32], i: u32| {
    let off = (i * 4) as usize;
    format!("{}, {}, {}", arr[off], arr[off + 1], arr[off + 2])
  };

  for i in 0..num_results {
    if i % num_threads_per_workgroup == 0 {
      log(&format!("\
 ---------------------------------------
 global                 local     global   dispatch: {}
 invoc.    workgroup    invoc.    invoc.
 index     id           id        id
 ---------------------------------------", i / num_threads_per_workgroup));
    }
    log(&format!(" {:3}:      {}      {}   {}",
        i, get3(workgroup, i), get3(local, i), get3(global, i)));
  }
```

`log` here is `wgpu_fun::log`, which appends a `<pre>` element to the page
in the browser — the same as the JS version's `log` helper — and prints to
the terminal natively.

Here's the result

{{{example url="../webgpu-compute-shaders-builtins.html"}}}

These builtins are generally the only inputs that change
per thread of a compute shader for one call to `pass.dispatchWorkgroups`
so to be effective you need to figure out how to use them to design
a compute shader function to do what you want, given these `..._id`
builtins as input.

## Workgroup Size

What size should you make a workgroup? The question often comes up,
why not just always use `@workgroup_size(1, 1, 1)` and then it would
be more trivial to decide how many iterations to run by only the
parameters to `pass.dispatch_workgroups`.

The reason is multiple threads within a workgroup are faster than
individual dispatches.

For one, threads in a workgroup often run in lockstep so running
16 of them is just as fast as running 1.

The default limits for WebGPU are as follows

* `maxComputeInvocationsPerWorkgroup`: 256
* `maxComputeWorkgroupSizeX`: 256
* `maxComputeWorkgroupSizeY`:	256
* `maxComputeWorkgroupSizeZ`:	64

As you can see, the first limit `maxComputeInvocationsPerWorkgroup` means the 3 parameters
to `@workgroup_size` can not multiply to a number larger than 256. In other words

```
   @workgroup_size(256, 1, 1)   // ok
   @workgroup_size(128, 2, 1)   // ok
   @workgroup_size(16, 16, 1)   // ok
   @workgroup_size(16, 16, 2)   // bad 16 * 16 * 2 = 512
```

Unfortunately, the perfect size is GPU dependent and WebGPU can not provide that info.
**The general advice for WebGPU is to choose a workgroup size of 64** unless you have
some specific reason to choose another size. Apparently most GPUs can efficiently
run 64 things in lockstep. If you choose a higher number and the GPU can't do it
as a fast path it will chose a slower path. If on the other hand you chose a number
below what the GPU can do then you may not get the maximum performance.

## <a href="a-race-conditions"></a>Races in Compute Shaders

A common mistake in WebGPU is not handling race conditions. A race
condition is where multiple threads are running at the same time and
effectively they are in a race for who comes in first or last.

Let's say you had this compute shader

```wgsl
@group(0) @binding(0) var<storage, read_write> result: array<f32>;

@compute @workgroup_size(32) fn computeSomething(
    @builtin(local_invocation_id) local_invocation_id : vec3<u32>,
) {
  result[0] = local_invocation_id.x;
`;
```

If that's hard to read, here's kind of the same Rust

```rust
let mut result = vec![0];
for i in 0..32 {
  result[0] = i;
}
```

In the Rust case, after the code runs, `result[0]` is clearly 31. In the compute shader case though,
all 32 iterations of the shader are running in parallel. Which ever one finishes
last is the one who's value will be in `result[0]`. Which one runs last is undefined.

From the spec:

> WebGPU provides no guarantees about:
>
> * Whether invocations from different workgroups execute concurrently. That is,
>   you cannot assume more than one workgroup executes at a time.
>
> * Whether, once invocations from a workgroup begin executing, that other
>   workgroups are blocked from execution. That is, you cannot assume that only
>   one workgroup executes at a time. While a workgroup is executing, the
>   implementation may choose to concurrently execute other workgroups as well,
>   or other queued but unblocked work.
>
> * Whether invocations from one particular workgroup begin executing before the
>   invocations of another workgroup. That is, you cannot assume that workgroups
>   are launched in a particular order.

We'll go over some of the ways to deal with this issue in future examples. For now, our
two examples have no race conditions as each iteration of the compute shader does something
unaffected by the other iterations.

Next up: [Example Compute Shaders - Image Histogram](webgpu-compute-shaders-histogram.html)
