Title: WebGPU Immediates
Description: Immediates
TOC: Immediates

This article is one in a series of the various ways to provide data
to a shader. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="passing-data.hanson"}}}

<div class="warn">
Immediates are a new (2026) feature of WebGPU. They are supposed to be a **core** feature,
meaning, they are suppose to be available everywhere, regardless of device.
They will hopefully be shipping in all browsers by the end of 2026.
wgpu already supports them natively (Vulkan, Metal, DX12), and exposes them as the
optional <code>wgpu::Features::IMMEDIATES</code> feature; in the browser, getting the
feature depends on the browser having shipped immediates. You request the feature
(and its size limit) when creating the device and check what you got:
<pre class="prettyprint lang-rust"><code>
let can_use_immediates = app.device.features().contains(wgpu::Features::IMMEDIATES);
</code></pre>
Ideally, by 2027, you should no longer need this check.
</div>

Immediates are a convenient way to easily pass a small amount of data to a shader.
In [the article uniforms](webgpu-uniforms.html) and [the article on storage buffers](webgpu-storage-buffers.html),
we covered how to pass data to a shader, via a buffer. We defined a `var<uniform>` or `var<storage, ...>` bindings
in our shaders and the bound buffers to those bindings. With immediates we use `var<immediate>` and no binding. 

The differences between `var<immediate>` vs `var<uniform>` and `var<storage>`:

* You can only have one `var<immediate>` per shader

  With `var<uniform>` and `var<storage, ...>` you can declare multiple bindings.
  With `var<immediate>` there can be only one

* Your immediates can only use 64bytes total [^maxImmediateSize]

[^maxImmediateSize]: The limit `max_immediate_size` might let you [request](webgpu-limits-and-features.html) more than 64.

* You must initializes all immediates

  With buffers, the buffer's contents are initialized to 0. With immediates, they are uninitialized
  and you must explicitly initialize them. If you don't you'll get a validation error.

* Immediates are reset to undefined when

  * you begin a new compute or render pass
  * you execute a render bundle
  * after executing a render bundle.

You can kind of think of immediates as a mini uniform buffer.
There is only one. It's small. You set it with `pass.set_immediates`

Let's take the simple triangle example from
[the bottom of the article on fundamentals](webgpu-fundamentals.html#a-resizing)
and updated it to draw 3 triangles in different colors
using immediates.

First, since immediates are an optional feature (for now), we ask for the
feature and its size limit when we create the device, and fail like the
JS examples do if we didn't get it

```rust
-  let mut app = App::new("WebGPU Immediates").await;
+  // ask for the immediates feature (and its size limit) if the adapter
+  // supports it
+  let mut app = App::new_with_features_and_limits(
+    "WebGPU Immediates",
+    wgpu::Features::IMMEDIATES,
+    |features, limits| wgpu::Limits {
+      max_immediate_size: if features.contains(wgpu::Features::IMMEDIATES) {
+        limits.max_immediate_size.min(64)
+      } else {
+        0
+      },
+      ..wgpu::Limits::default()
+    },
+  )
+  .await;
+  // You can probably remove this check by 2027 🙏
+  if !app.device.features().contains(wgpu::Features::IMMEDIATES) {
+    wgpu_fun::fail("need a browser that supports WebGPU immediates");
+    return;
+  }
```

Then let's add an offset and color to the shaders

```wgsl
+struct MyImmediates {
+  color: vec4f,
+  offset: vec2f,
+};
+
+var<immediate> myImmediates: MyImmediates;

@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32
) -> @builtin(position) vec4f {
  let pos = array(
    vec2f( 0.0,  0.5),  // top center
    vec2f(-0.5, -0.5),  // bottom left
    vec2f( 0.5, -0.5)   // bottom right
  );

-  return vec4f(pos[vertexIndex], 0.0, 1.0);
+  return vec4f(pos[vertexIndex] + myImmediates.offset, 0.0, 1.0);
}

@fragment fn fs() -> @location(0) vec4f {
-  return vec4f(1, 0, 0, 1);
+  return myImmediates.color;
}
```

Then we can update the render code to draw 3 times, setting the immediates
using `set_immediates` each time to draw in a different color in a different
location.

```rust
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
+      pass.set_immediates(
+        0,
+        bytemuck::cast_slice(&[
+          1.0f32, 0.0, 0.0, 1.0, // color
+          -0.4, -0.2, // offset
+        ]),
+      );
      pass.draw(0..3, 0..1);

+      pass.set_immediates(
+        0,
+        bytemuck::cast_slice(&[
+          0.0f32, 1.0, 0.0, 1.0, // color
+          0.4, -0.2, // offset
+        ]),
+      );
+      pass.draw(0..3, 0..1);
+
+      pass.set_immediates(
+        0,
+        bytemuck::cast_slice(&[
+          0.0f32, 0.0, 1.0, 1.0, // color
+          0.0, 0.2, // offset
+        ]),
+      );
+      pass.draw(0..3, 0..1);
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
```

Just like `var<uniform>` and `var<storage, ...>` the data
in immediates follows the same [memory layout rules](webgpu-memory-layout.html).
The arguments to `set_immediates` are 

```rust
pass.set_immediates(
  byte_offset,  // offset in the immediates
  data,         // a &[u8] byte slice
);
```

The JS version takes optional source offset and size arguments; in Rust you
just slice the data yourself, and `bytemuck::cast_slice` turns our `f32`
arrays into the byte slice the API wants.

{{{example url="../webgpu-immediates-triangles.html"}}}

You might be wondering, with a limit of 64 bytes, what's the use case
for immediates.

The most common usage is probably just to pass indices into other data.
Imagine making a per model storage buffer array and a per material
storage buffer array

```wgsl
struct PerModel {
  matrix: mat4x4f,
};

struct Material {
  color: vec4f,
  shininess: f32,
};

@group(0) @binding(0) var<storage, read> models: array<PerModel>;
@group(0) @binding(1) var<storage, read> materials: array<Material>;
...
```

Then you could use immediates to select the `PerModel` and `Material`
values

```wgsl
struct RenderIndices {
  modelNdx: u32,
  materialNdx: u32,
};
var<immediate> renderIndices: RenderIndices;

... in vertex shader ...

   let modelMatrix = models[renderIndices.modelNdx];

... in fragment shader ...

   let material = materials[renderIndices.materialNdx];

```

Now at render time you can select a per model data
and material data just by passing in the indices

```rust
   pass.set_immediates(0, bytemuck::cast_slice(&[model_ndx, material_ndx]));
```

This could be [an optimization](webgpu-optimization.html) as you won't
have to manage a uniform buffer per model and per material.

Here's a full shader as an example

```wgsl
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
```

The shader above uses immediates to select a material and per model
data. It uses [matrix math](webgpu-matrix-math.html) to position the
vertices.

It also has a global uniform buffer for things that are shared
by all models. In this case it uses a shared [viewProjection matrix](webgpu-orthographic-projection.html).

We make a pipeline that uses this shader and specifies [vertex buffers](webgpu-vertex-buffers.html) that use 2 floats per vertex.

```rust
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
```

We create vertex buffers for 3 different shapes, a triangle, a square,
and a circle.

```rust
  let square_vertices: Vec<f32> = vec![
    -0.5, -0.5,
     0.5, -0.5,
    -0.5,  0.5,
    -0.5,  0.5,
     0.5, -0.5,
     0.5,  0.5,
  ];
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

  let vertices = [
    create_vertex_buffer(&app.device, &app.queue, &triangle_vertices),
    create_vertex_buffer(&app.device, &app.queue, &circle_vertices),
    create_vertex_buffer(&app.device, &app.queue, &square_vertices),
  ];
```

Then we'll make a storage buffer with 6 materials.

```rust
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
```

And we'll defined 200 "models" where a model is the combination of
a vertex buffer, a material, a per model data.

```rust
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
```

Above we used [our math](webgpu-matrix-math.html) to choose a random position, scale
and orientation. This is stored in the model data.

We then need to upload that data into a storage buffer

```rust
  let per_model_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("our per model buffer"),
    size: (model_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue
    .write_buffer(&per_model_buffer, 0, bytemuck::cast_slice(&model_data));
```

We also have a shared buffer that all models will use. This will store
our [projection matrix](webgpu-orthographic-projection.html).

```rust
  let mut shared_data = [0.0f32; 16];
  let shared_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("our shared data buffer"),
    size: (shared_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
```

We then make a bind group that references our 3 buffers

```rust
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
```

Finally we can render. First we compute [an orthographic matrix](webgpu-orthographic-projection.html)
that will make our rendering fit the aspect of our canvas and upload
it to the shared buffer.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
+    let aspect = frame.width as f32 / frame.height as f32;
+    let ortho =
+      glam::camera::rh::proj::directx::orthographic(-aspect, aspect, -1.0, 1.0, -1.0, 1.0);
+    shared_data.copy_from_slice(&ortho.to_cols_array());
+    frame
+      .queue
+      .write_buffer(&shared_buffer, 0, bytemuck::cast_slice(&shared_data));
```

Then we can render all of our models

```rust
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
*      pass.set_bind_group(0, &bind_group, &[]);
*      for model in &models {
*        pass.set_immediates(0, bytemuck::cast_slice(&model.immediates));
*        pass.set_vertex_buffer(0, vertices[model.vertex_buffer_ndx].buffer.slice(..));
*        pass.draw(0..model.num_vertices, 0..1);
*      }
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

And with that we're drawing multiple models and selecting materials
and per model data using immediates.

{{{example url="../webgpu-immediates-models.html"}}}

Hopefully this gives you some idea of how to use immediates. The fact that
they have a small 64 byte limit generally means you need to be creative in
how to take advantage of them.
