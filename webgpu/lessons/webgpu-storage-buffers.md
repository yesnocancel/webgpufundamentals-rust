Title: WebGPU Storage Buffers
Description: Passing Large Data to Shaders
TOC: Storage Buffers

This article is about storage buffers and continues where the
[previous article](webgpu-uniforms.html) left off.

Storage buffers are similar to uniform buffers in many ways.
If all we did was change `UNIFORM` to `STORAGE` in our Rust
and `var<uniform>` to `var<storage, read>` in our WGSL, the examples
on the previous page would just work.

In fact, here are the differences, without renaming variables to have more
appropriate names.

```rust
    let static_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("static uniforms for obj: {i}")),
      size: static_uniform_buffer_size as u64,
-      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });


...

    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some(&format!("changing uniforms for obj: {i}")),
      size: uniform_buffer_size as u64,
-      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
```

and in our WSGL

```wsgl
-@group(0) @binding(0) var<uniform> ourStruct: OurStruct;
-@group(0) @binding(1) var<uniform> otherStruct: OtherStruct;
+@group(0) @binding(0) var<storage, read> ourStruct: OurStruct;
+@group(0) @binding(1) var<storage, read> otherStruct: OtherStruct;
```

And with no other changes it works, just like before.

{{{example url="../webgpu-simple-triangle-storage-split-minimal-changes.html"}}}

## Differences between uniform buffers and storage buffers

The major differences between uniform buffers and storage buffers are:

1. Uniform buffers can be faster for their typical use-case

   It really depends on the use case. A typical app will need to draw
   lots of different things. Say it's a 3D game. The app might draw
   cars, buildings, rocks, bushes, people, etc... Each of those will
   require passing in orientations and material properties similar
   to what our example above passes in. In this case, using a uniform buffer
   is the recommended solution.

2. Storage buffers can be much larger than uniform buffers.

   * By default, the maximum size of a uniform buffer is 64 kiB (65536 bytes).
   * By default, the maximum size of a storage buffer is 128 MiB (134217728 bytes).

   All implementations are required to support at least these sizes. We'll cover how to check for and request larger limits in
   detail in [another article](webgpu-limits-and-features.html).

3. Storage buffers can be read/write, Uniform buffers are read-only.

   We saw an example of writing to a storage buffer in the compute shader
   example in [the first article](webgpu-fundamentals.html).

## <a id="a-instancing"></a>Instancing with Storage Buffers

Given the first 2 points above, let's take our last example and change it
to draw all 100 triangles in a single draw call. This is a use-case that
*might* fit storage buffers. I say might because again, WebGPU is similar
to other programming languages. There are many ways to achieve the same thing.
`iter().for_each` vs `for elem in &array` vs `for i in 0..array.len()`. Each has its uses. The same is true of WebGPU. Each thing we try to do
has multiple ways we can achieve it. When it comes to drawing triangles,
all that WebGPU cares about is we return a value for `builtin(position)` from
the vertex shader and return a color/value for `location(0)` from the fragment shader.[^colorAttachments] 

[^colorAttachments]: We can have multiple color attachments and then we'll need to return more colors/values for `location(1)`, `location(2)`, etc...

The first thing we'll do is change our storage declarations to runtime-sized
arrays.

```wgsl
-@group(0) @binding(0) var<storage, read> ourStruct: OurStruct;
-@group(0) @binding(1) var<storage, read> otherStruct: OtherStruct;
+@group(0) @binding(0) var<storage, read> ourStructs: array<OurStruct>;
+@group(0) @binding(1) var<storage, read> otherStructs: array<OtherStruct>;
```

Then we'll change the shader to use these values.

```wgsl
@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32,
+  @builtin(instance_index) instanceIndex: u32
) -> @builtin(position) {
  let pos = array(
    vec2f( 0.0,  0.5),  // top center
    vec2f(-0.5, -0.5),  // bottom left
    vec2f( 0.5, -0.5)   // bottom right
  );

+  let otherStruct = otherStructs[instanceIndex];
+  let ourStruct = ourStructs[instanceIndex];

   return vec4f(
     pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
}
```

We added a new parameter to our vertex shader called
`instanceIndex` and gave it the `@builtin(instance_index)` attribute
which means it gets its value from WebGPU for each "instance" drawn.
When we call `draw`, we can pass a second range for *the instances to draw*
and for each instance drawn, the number of the instance being processed
will be passed to our function.

Using `instanceIndex`, we can get specific struct elements from our arrays
of structs.

We also need to get the color from the correct array element and use
it in our fragment shader. The fragment shader doesn't have access to
`@builtin(instance_index)` because that would make no sense. We could pass
it as an [inter-stage variable](webgpu-inter-stage-variables.html) but it
would be more common to look up the color in the vertex shader and just pass
the color.

To do this we'll use another struct like we did in
[the article on inter-stage variables](webgpu-inter-stage-variables.html).

```wgsl
+struct VSOutput {
+  @builtin(position) position: vec4f,
+  @location(0) color: vec4f,
+}

@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32,
  @builtin(instance_index) instanceIndex: u32
-) -> @builtin(position) vec4f {
+) -> VSOutput {
  let pos = array(
    vec2f( 0.0,  0.5),  // top center
    vec2f(-0.5, -0.5),  // bottom left
    vec2f( 0.5, -0.5)   // bottom right
  );

  let otherStruct = otherStructs[instanceIndex];
  let ourStruct = ourStructs[instanceIndex];

-  return vec4f(
-    pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
+  var vsOut: VSOutput;
+  vsOut.position = vec4f(
+      pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
+  vsOut.color = ourStruct.color;
+  return vsOut;
}

-@fragment fn fs() -> @location(0) vec4f {
-  return ourStruct.color;
+@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
+  return vsOut.color;
}

```

Now that we've modified our WGSL shaders, let's update the Rust.

Here's the setup.

```rust
  struct ObjectInfo {
    scale: f32,
  }

  const K_NUM_OBJECTS: usize = 100;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  // create 2 storage buffers
  let static_unit_size = 4 * 4 + // color is 4 32bit floats (4bytes each)
      2 * 4 + // offset is 2 32bit floats (4bytes each)
      2 * 4; // padding
  let changing_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
  let static_storage_buffer_size = static_unit_size * K_NUM_OBJECTS;
  let changing_storage_buffer_size = changing_unit_size * K_NUM_OBJECTS;

  let static_storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("static storage for objects"),
    size: static_storage_buffer_size as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let changing_storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("changing storage for objects"),
    size: changing_storage_buffer_size as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_OFFSET_OFFSET: usize = 4;

  const K_SCALE_OFFSET: usize = 0;

  {
    let mut static_storage_values = vec![0.0f32; static_storage_buffer_size / 4];
    for i in 0..K_NUM_OBJECTS {
      let static_offset = i * (static_unit_size / 4);

      // These are only set once so set them now
      static_storage_values[static_offset + K_COLOR_OFFSET..static_offset + K_COLOR_OFFSET + 4]
          .copy_from_slice(&[rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0]); // set the color
      static_storage_values[static_offset + K_OFFSET_OFFSET..static_offset + K_OFFSET_OFFSET + 2]
          .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

      object_infos.push(ObjectInfo {
        scale: rand(0.2, 0.5),
      });
    }
    app.queue.write_buffer(&static_storage_buffer, 0, bytemuck::cast_slice(&static_storage_values));
  }

  // a Vec<f32> we can use to update the changing_storage_buffer
  let mut storage_values = vec![0.0f32; changing_storage_buffer_size / 4];

  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bind group for objects"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: static_storage_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: changing_storage_buffer.as_entire_binding() },
    ],
  });
```

Above we create 2 storage buffers. One for an array of `OurStruct`
and the other for an array of `OtherStruct`.

We then fill out the values for the array of `OurStruct` with offsets
and colors and then upload that data to the `static_storage_buffer`.

We make just one bind group that references both buffers.

The new rendering code is

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    let mut encoder = frame
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        ...
      });
      pass.set_pipeline(&pipeline);

      // Set the uniform values in our Rust side Vec
      let aspect = frame.width as f32 / frame.height as f32;

-      for object_info in object_infos.iter_mut() {
-        let scale = object_info.scale;
-        object_info.uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
-            .copy_from_slice(&[scale / aspect, scale]); // set the scale
-        frame.queue.write_buffer(
-            &object_info.uniform_buffer,
-            0,
-            bytemuck::cast_slice(&object_info.uniform_values),
-        );
-
-        pass.set_bind_group(0, &object_info.bind_group, &[]);
-        pass.draw(0..3, 0..1); // call our vertex shader 3 times
-      }

+      // set the scales for each object
+      for (ndx, ObjectInfo { scale }) in object_infos.iter().enumerate() {
+        let offset = ndx * (changing_unit_size / 4);
+        storage_values[offset + K_SCALE_OFFSET..offset + K_SCALE_OFFSET + 2]
+            .copy_from_slice(&[scale / aspect, *scale]); // set the scale
+      }
+      // upload all scales at once
+      frame.queue.write_buffer(&changing_storage_buffer, 0, bytemuck::cast_slice(&storage_values));
+
+      pass.set_bind_group(0, &bind_group, &[]);
+      pass.draw(0..3, 0..K_NUM_OBJECTS as u32); // call our vertex shader 3 times for each instance
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

The code above is going to draw `K_NUM_OBJECTS` instances. For each instance
WebGPU will call the vertex shader 3 times with `vertex_index` set to 0, 1, 2
and `instance_index` set to 0 ~ K_NUM_OBJECTS - 1

{{{example url="../webgpu-simple-triangle-storage-buffer-split.html"}}}

We managed to draw all 100 triangles, each with a different scale, color, and
offset, with a single draw call. For situations where you want to draw lots
of instances of the same object, this is one way to do it.

## Using storage buffers for vertex data

Until this point, we've used a hard-coded triangle directly in our shader.
One use case of storage buffers is to store vertex data. Just like we indexed
the current storage buffers by `instance_index` in our example above, we could
index another storage buffer with `vertex_index` to get vertex data.

Let's do it!

```wgsl
struct OurStruct {
  color: vec4f,
  offset: vec2f,
};

struct OtherStruct {
  scale: vec2f,
};

+struct Vertex {
+  position: vec2f,
+};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

@group(0) @binding(0) var<storage, read> ourStructs: array<OurStruct>;
@group(0) @binding(1) var<storage, read> otherStructs: array<OtherStruct>;
+@group(0) @binding(2) var<storage, read> pos: array<Vertex>;

@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32,
  @builtin(instance_index) instanceIndex: u32
) -> VSOutput {
-  let pos = array(
-    vec2f( 0.0,  0.5),  // top center
-    vec2f(-0.5, -0.5),  // bottom left
-    vec2f( 0.5, -0.5)   // bottom right
-  );

  let otherStruct = otherStructs[instanceIndex];
  let ourStruct = ourStructs[instanceIndex];

  var vsOut: VSOutput;
  vsOut.position = vec4f(
-      pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
+      pos[vertexIndex].position * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
  vsOut.color = ourStruct.color;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vsOut.color;
}
```

Now we need to set up one more storage buffer with some vertex data.
First, let's make a function to generate some vertex data. Let's make a circle.
<a id="a-create-circle"></a>

```rust
fn create_circle_vertices(
    radius: f32,
    num_subdivisions: usize,
    inner_radius: f32,
    start_angle: f32,
    end_angle: f32,
) -> (Vec<f32>, usize) {
    // 2 triangles per subdivision, 3 verts per tri, 2 values (xy) each.
    let num_vertices = num_subdivisions * 3 * 2;
    let mut vertex_data = vec![0.0f32; num_subdivisions * 2 * 3 * 2];

    let mut offset = 0;
    let mut add_vertex = |x: f32, y: f32| {
        vertex_data[offset] = x;
        offset += 1;
        vertex_data[offset] = y;
        offset += 1;
    };

    // 2 triangles per subdivision
    //
    // 0--1 4
    // | / /|
    // |/ / |
    // 2 3--5
    for i in 0..num_subdivisions {
        let angle1 = start_angle + (i + 0) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
        let angle2 = start_angle + (i + 1) as f32 * (end_angle - start_angle) / num_subdivisions as f32;

        let c1 = angle1.cos();
        let s1 = angle1.sin();
        let c2 = angle2.cos();
        let s2 = angle2.sin();

        // first triangle
        add_vertex(c1 * radius, s1 * radius);
        add_vertex(c2 * radius, s2 * radius);
        add_vertex(c1 * inner_radius, s1 * inner_radius);

        // second triangle
        add_vertex(c1 * inner_radius, s1 * inner_radius);
        add_vertex(c2 * radius, s2 * radius);
        add_vertex(c2 * inner_radius, s2 * inner_radius);
    }

    (vertex_data, num_vertices)
}
```

The code above makes a circle from triangles like this.

<div class="webgpu_center"><div class="center"><div data-diagram="circle" style="width: 300px;"></div></div></div>

So we can use that to fill a storage buffer with the vertices for a circle.

```rust
  // setup a storage buffer with vertex data
  let (vertex_data, num_vertices) = create_circle_vertices(
    0.5,                        // radius
    24,                         // numSubdivisions
    0.25,                       // innerRadius
    0.0,                        // startAngle
    std::f32::consts::PI * 2.0, // endAngle
  );
  let vertex_storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("storage buffer vertices"),
    size: (vertex_data.len() * std::mem::size_of::<f32>()) as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_storage_buffer, 0, bytemuck::cast_slice(&vertex_data));
```

And then we need to add it to our bind group.

```rust
  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bind group for objects"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: static_storage_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: changing_storage_buffer.as_entire_binding() },
+      wgpu::BindGroupEntry { binding: 2, resource: vertex_storage_buffer.as_entire_binding() },
    ],
  });
```

and finally, at render time, we need to ask to render all the vertices in the circle.

```rust
-    pass.draw(0..3, 0..K_NUM_OBJECTS as u32); // call our vertex shader 3 times for several instances
+    pass.draw(0..num_vertices as u32, 0..K_NUM_OBJECTS as u32);
```

{{{example url="../webgpu-storage-buffer-vertices.html"}}}

Above we used 

```wsgl
struct Vertex {
  pos: vec2f;
};

@group(0) @binding(2) var<storage, read> pos: array<Vertex>;
```

we could have just as easily used no struct and just directly used a `vec2f`.

```wgsl
-@group(0) @binding(2) var<storage, read> pos: array<Vertex>;
+@group(0) @binding(2) var<storage, read> pos: array<vec2f>;
...
-pos[vertexIndex].position * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
+pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
```

But, by making it a struct, it would arguably be easier to add per-vertex
data later?

Passing in vertices via storage buffers is gaining popularity.
I'm told though that for some older devices, it's slower than the *classic* way
which we'll cover next in an article on [vertex buffers](webgpu-vertex-buffers.html).

<!-- keep this at the bottom of the article -->
<script type="module" src="./webgpu-storage-buffers.js"></script>
