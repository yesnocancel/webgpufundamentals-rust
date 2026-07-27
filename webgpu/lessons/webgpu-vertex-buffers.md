Title: WebGPU Vertex Buffers
Description: Passing Vertex Data to Shaders
TOC: Vertex Buffers

This article is one in a series of the various ways to provide data
to a shader. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="passing-data.hanson"}}}

In [the previous article](webgpu-storage-buffers.html) we put vertex
data in a storage buffer and indexed it using the builtin `vertex_index`.
While that technique is growing in popularity, the traditional way to
provide vertex data to a vertex shader is via vertex buffers and
attributes.

Vertex buffers are just like any other WebGPU buffer; they hold data.
The difference is we don't access them directly from the vertex shader.
Instead, we tell WebGPU what kind of data is in the buffer and how it's
organized. It then pulls the data out of the buffer and provides it for us.

Let's take the last example from
[the previous article](webgpu-storage-buffers.html)
and change it from using a storage buffer to using a vertex buffer.

The first thing to do is change the shader to get its vertex data
from a vertex buffer. 

```wgsl
struct OurStruct {
  color: vec4f,
  offset: vec2f,
};

struct OtherStruct {
  scale: vec2f,
};

struct Vertex {
-  position: vec2f,
+  @location(0) position: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

@group(0) @binding(0) var<storage, read> ourStructs: array<OurStruct>;
@group(0) @binding(1) var<storage, read> otherStructs: array<OtherStruct>;
-@group(0) @binding(2) var<storage, read> pos: array<Vertex>;

@vertex fn vs(
-  @builtin(vertex_index) vertexIndex : u32,
+  vert: Vertex,
  @builtin(instance_index) instanceIndex: u32
) -> VSOutput {
  let otherStruct = otherStructs[instanceIndex];
  let ourStruct = ourStructs[instanceIndex];

  var vsOut: VSOutput;
  vsOut.position = vec4f(
-      pos[vertexIndex].position * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
+      vert.position * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
  vsOut.color = ourStruct.color;
  return vsOut;
}

...
```

As you can see, it's a small change. The important part is declaring the
position field with `@location(0)`. 

Next, we have to tell WebGPU how to supply data for `@location(0)` - 
for that, we use the render pipeline:

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("vertex buffer pipeline"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
-      buffers: &[],
+      buffers: &[
+        Some(wgpu::VertexBufferLayout {
+          array_stride: 2 * 4, // 2 floats, 4 bytes each
+          step_mode: wgpu::VertexStepMode::Vertex,
+          attributes: &[
+            // position
+            wgpu::VertexAttribute {
+              shader_location: 0,
+              offset: 0,
+              format: wgpu::VertexFormat::Float32x2,
+            },
+          ],
+        }),
+      ],
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

To the `vertex` entry (`wgpu::VertexState`) of the pipeline descriptor
we added a `buffers` array which is used to describe how to pull data out of 1 or more vertex buffers.
Each entry is an `Option` — passing `None` leaves that buffer slot unused
(the equivalent of a `null` entry in the JavaScript API).
For our first and only buffer, we set an `array_stride` in number of bytes. A *stride* in this case is
how many bytes to get from the data for one vertex in the buffer, to the next vertex in the buffer.

<div class="webgpu_center"><img src="resources/vertex-buffer-one.svg" style="width: 1024px;"></div>

Since our data is `vec2f`, which is two float32 numbers, we set the
`array_stride` to 8.

Next we define an array of attributes. We only have one: `shader_location: 0`
corresponds to `location(0)` in our `Vertex` struct. `offset: 0` says the data
for this attribute starts at byte 0 in the vertex buffer. Finally `format:
wgpu::VertexFormat::Float32x2` says we want WebGPU to pull the data out of the buffer as two 32bit
floating point numbers. (Note: the `attributes` property is shown in the 
[simplified draw diagram](webgpu-fundamentals.html#a-draw-diagram) 
from the first article).

We need to change the usages of the buffer holding vertex data from `STORAGE`
to `VERTEX` and remove it from the bind group.

```rust
-  let vertex_storage_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-    label: Some("storage buffer vertices"),
-    size: (vertex_data.len() * 4) as u64,
-    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
-    mapped_at_creation: false,
-  });
+  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+    label: Some("vertex buffer vertices"),
+    size: (vertex_data.len() * 4) as u64,
+    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
+    mapped_at_creation: false,
+  });
+  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bind group for objects"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: static_storage_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: changing_storage_buffer.as_entire_binding() },
-      wgpu::BindGroupEntry { binding: 2, resource: vertex_storage_buffer.as_entire_binding() },
    ],
  });
```

Then, at draw time we need to tell WebGPU which vertex buffer to use:

```rust
    pass.set_pipeline(&pipeline);
+    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
```

The `0` here corresponds to first element of the render pipeline `buffers`
array we specified above. The `slice(..)` picks which part of the buffer to
use — all of it in our case.

With that, we've switched from using a storage buffer for vertices to a
vertex buffer.

{{{example url="../webgpu-vertex-buffers.html"}}}

The state when the draw command is executed would look something like this:

<div class="webgpu_center"><img src="resources/webgpu-draw-diagram-vertex-buffer.svg" style="width: 960px;"></div>

The attribute `format` field can be one of these types:

<div class="webgpu_center data-table">
  <style>
    .vertex-type {
      text-align: center;
    }
  </style>
  <div>
  <table class="vertex-type">
    <thead>
     <tr>
      <th>Vertex format</th>
      <th>Data type</th>
      <th>Components</th>
      <th>Byte size</th>
      <th>Example WGSL type</th>
     </tr>
    </thead>
    <tbody>
      <tr><td><code>Uint8x2</code></td><td>unsigned int </td><td>2 </td><td>2 </td><td><code>vec2&lt;u32&gt;</code>, <code>vec2u</code></td></tr>
      <tr><td><code>Uint8x4</code></td><td>unsigned int </td><td>4 </td><td>4 </td><td><code>vec4&lt;u32&gt;</code>, <code>vec4u</code></td></tr>
      <tr><td><code>Sint8x2</code></td><td>signed int </td><td>2 </td><td>2 </td><td><code>vec2&lt;i32&gt;</code>, <code>vec2i</code></td></tr>
      <tr><td><code>Sint8x4</code></td><td>signed int </td><td>4 </td><td>4 </td><td><code>vec4&lt;i32&gt;</code>, <code>vec4i</code></td></tr>
      <tr><td><code>Unorm8x2</code></td><td>unsigned normalized </td><td>2 </td><td>2 </td><td><code>vec2&lt;f32&gt;</code>, <code>vec2f</code></td></tr>
      <tr><td><code>Unorm8x4</code></td><td>unsigned normalized </td><td>4 </td><td>4 </td><td><code>vec4&lt;f32&gt;</code>, <code>vec4f</code></td></tr>
      <tr><td><code>Snorm8x2</code></td><td>signed normalized </td><td>2 </td><td>2 </td><td><code>vec2&lt;f32&gt;</code>, <code>vec2f</code></td></tr>
      <tr><td><code>Snorm8x4</code></td><td>signed normalized </td><td>4 </td><td>4 </td><td><code>vec4&lt;f32&gt;</code>, <code>vec4f</code></td></tr>
      <tr><td><code>Uint16x2</code></td><td>unsigned int </td><td>2 </td><td>4 </td><td><code>vec2&lt;u32&gt;</code>, <code>vec2u</code></td></tr>
      <tr><td><code>Uint16x4</code></td><td>unsigned int </td><td>4 </td><td>8 </td><td><code>vec4&lt;u32&gt;</code>, <code>vec4u</code></td></tr>
      <tr><td><code>Sint16x2</code></td><td>signed int </td><td>2 </td><td>4 </td><td><code>vec2&lt;i32&gt;</code>, <code>vec2i</code></td></tr>
      <tr><td><code>Sint16x4</code></td><td>signed int </td><td>4 </td><td>8 </td><td><code>vec4&lt;i32&gt;</code>, <code>vec4i</code></td></tr>
      <tr><td><code>Unorm16x2</code></td><td>unsigned normalized </td><td>2 </td><td>4 </td><td><code>vec2&lt;f32&gt;</code>, <code>vec2f</code></td></tr>
      <tr><td><code>Unorm16x4</code></td><td>unsigned normalized </td><td>4 </td><td>8 </td><td><code>vec4&lt;f32&gt;</code>, <code>vec4f</code></td></tr>
      <tr><td><code>Snorm16x2</code></td><td>signed normalized </td><td>2 </td><td>4 </td><td><code>vec2&lt;f32&gt;</code>, <code>vec2f</code></td></tr>
      <tr><td><code>Snorm16x4</code></td><td>signed normalized </td><td>4 </td><td>8 </td><td><code>vec4&lt;f32&gt;</code>, <code>vec4f</code></td></tr>
      <tr><td><code>Float16x2</code></td><td>float </td><td>2 </td><td>4 </td><td><code>vec2&lt;f16&gt;</code>, <code>vec2h</code></td></tr>
      <tr><td><code>Float16x4</code></td><td>float </td><td>4 </td><td>8 </td><td><code>vec4&lt;f16&gt;</code>, <code>vec4h</code></td></tr>
      <tr><td><code>Float32</code></td><td>float </td><td>1 </td><td>4 </td><td><code>f32</code></td></tr>
      <tr><td><code>Float32x2</code></td><td>float </td><td>2 </td><td>8 </td><td><code>vec2&lt;f32&gt;</code>, <code>vec2f</code></td></tr>
      <tr><td><code>Float32x3</code></td><td>float </td><td>3 </td><td>12 </td><td><code>vec3&lt;f32&gt;</code>, <code>vec3f</code></td></tr>
      <tr><td><code>Float32x4</code></td><td>float </td><td>4 </td><td>16 </td><td><code>vec4&lt;f32&gt;</code>, <code>vec4f</code></td></tr>
      <tr><td><code>Uint32</code></td><td>unsigned int </td><td>1 </td><td>4 </td><td><code>u32</code></td></tr>
      <tr><td><code>Uint32x2</code></td><td>unsigned int </td><td>2 </td><td>8 </td><td><code>vec2&lt;u32&gt;</code>, <code>vec2u</code></td></tr>
      <tr><td><code>Uint32x3</code></td><td>unsigned int </td><td>3 </td><td>12 </td><td><code>vec3&lt;u32&gt;</code>, <code>vec3u</code></td></tr>
      <tr><td><code>Uint32x4</code></td><td>unsigned int </td><td>4 </td><td>16 </td><td><code>vec4&lt;u32&gt;</code>, <code>vec4u</code></td></tr>
      <tr><td><code>Sint32</code></td><td>signed int </td><td>1 </td><td>4 </td><td><code>i32</code></td></tr>
      <tr><td><code>Sint32x2</code></td><td>signed int </td><td>2 </td><td>8 </td><td><code>vec2&lt;i32&gt;</code>, <code>vec2i</code></td></tr>
      <tr><td><code>Sint32x3</code></td><td>signed int </td><td>3 </td><td>12 </td><td><code>vec3&lt;i32&gt;</code>, <code>vec3i</code></td></tr>
      <tr><td><code>Sint32x4</code></td><td>signed int </td><td>4 </td><td>16 </td><td><code>vec4&lt;i32&gt;</code>, <code>vec4i</code></td></tr>
    </tbody>
  </table>
  </div>
</div>

## <a id="a-instancing"></a>Instancing with Vertex Buffers

Attributes can advance per vertex or per instance. Advancing them per instance is effectively
the same thing we're doing when we index `otherStructs[instanceIndex]` and `ourStructs[instanceIndex]`
where `instanceIndex` got its value from `@builtin(instance_index)`.

Let's get rid of the storage buffers and use vertex buffers to accomplish the same thing.
First lets change the shader to use vertex attributes instead of storage buffers.

```wgsl
-struct OurStruct {
-  color: vec4f,
-  offset: vec2f,
-};
-
-struct OtherStruct {
-  scale: vec2f,
-};

struct Vertex {
  @location(0) position: vec2f,
+  @location(1) color: vec4f,
+  @location(2) offset: vec2f,
+  @location(3) scale: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

-@group(0) @binding(0) var<storage, read> ourStructs: array<OurStruct>;
-@group(0) @binding(1) var<storage, read> otherStructs: array<OtherStruct>;

@vertex fn vs(
  vert: Vertex,
-  @builtin(instance_index) instanceIndex: u32
) -> VSOutput {
-  let otherStruct = otherStructs[instanceIndex];
-  let ourStruct = ourStructs[instanceIndex];

  var vsOut: VSOutput;
-  vsOut.position = vec4f(
-      vert.position * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
-  vsOut.color = ourStruct.color;
+  vsOut.position = vec4f(
+      vert.position * vert.scale + vert.offset, 0.0, 1.0);
+  vsOut.color = vert.color;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vsOut.color;
}
```

Now we need to update our render pipeline to tell it how we want
to supply data to those attributes. To keep the changes to a minimum
we'll use the data we created for the storage buffers almost as is.
We'll use two buffers, one buffer will hold the `color` and `offset`
per instance, the other will hold the `scale`.

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("flat colors"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[
        Some(wgpu::VertexBufferLayout {
          array_stride: 2 * 4, // 2 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[
            // position
            wgpu::VertexAttribute {
              shader_location: 0,
              offset: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
          ],
        }),
+        Some(wgpu::VertexBufferLayout {
+          array_stride: 6 * 4, // 6 floats, 4 bytes each
+          step_mode: wgpu::VertexStepMode::Instance,
+          attributes: &[
+            // color
+            wgpu::VertexAttribute {
+              shader_location: 1,
+              offset: 0,
+              format: wgpu::VertexFormat::Float32x4,
+            },
+            // offset
+            wgpu::VertexAttribute {
+              shader_location: 2,
+              offset: 16,
+              format: wgpu::VertexFormat::Float32x2,
+            },
+          ],
+        }),
+        Some(wgpu::VertexBufferLayout {
+          array_stride: 2 * 4, // 2 floats, 4 bytes each
+          step_mode: wgpu::VertexStepMode::Instance,
+          attributes: &[
+            // scale
+            wgpu::VertexAttribute {
+              shader_location: 3,
+              offset: 0,
+              format: wgpu::VertexFormat::Float32x2,
+            },
+          ],
+        }),
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

Above we added 2 entries to the `buffers` array on our pipeline description so now there are 3 buffer entries, meaning
we're telling WebGPU we'll supply the data in 3 buffers.

For our 2 new entries we set the `step_mode` to `wgpu::VertexStepMode::Instance`. This means this attribute
will only advance to next value once per instance. The default is `wgpu::VertexStepMode::Vertex`
which advances once per vertex (and starts over for each instance).

We have 2 buffers. The one that holds just `scale` is simple. Just like our
first buffer that holds `position` it's 2 32 floats per vertex.

Our other buffer holds `color` and `offset` and they're going to be interleaved in the data like this

<div class="webgpu_center"><img src="resources/vertex-buffer-f32x4-f32x2.svg" style="width: 1024px;"></div>

So above we say the `array_stride` to get from one set of data to the next is `6 * 4`, 6 32bit floats
each 4 bytes (24 bytes total). The `color` starts at offset 0 but the `offset` starts 16 bytes in.

Next we can change the code that sets up the buffers.

```rust
-  // create 2 storage buffers
+  // create 2 vertex buffers
  let static_unit_size = 4 * 4 + // color is 4 32bit floats (4bytes each)
-      2 * 4 + // offset is 2 32bit floats (4bytes each)
-      2 * 4; // padding
+      2 * 4; // offset is 2 32bit floats (4bytes each)

  let changing_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
*  let static_vertex_buffer_size = static_unit_size * k_num_objects;
*  let changing_vertex_buffer_size = changing_unit_size * k_num_objects;

*  let static_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
*    label: Some("static vertex for objects"),
*    size: static_vertex_buffer_size as u64,
-    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
+    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

*  let changing_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
*    label: Some("changing vertex for objects"),
*    size: changing_vertex_buffer_size as u64,
-    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
+    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

```

Vertex attributes do not have the same padding restrictions as structures
in storage buffers so we no longer need the padding. Otherwise all we
did was change the usage from `STORAGE` to `VERTEX` (and we renamed all the
variables from "storage" to "vertex").

Since we're no longer using the storage buffers we no longer need
the bind group:

```rust
-  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
-    label: Some("bind group for objects"),
-    layout: &pipeline.get_bind_group_layout(0),
-    entries: &[
-      wgpu::BindGroupEntry { binding: 0, resource: static_storage_buffer.as_entire_binding() },
-      wgpu::BindGroupEntry { binding: 1, resource: changing_storage_buffer.as_entire_binding() },
-    ],
-  });
```

Finally, we don't need to set the bind group but, we do need
to set the vertex buffers:

```rust
    pass.set_pipeline(&pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
+    pass.set_vertex_buffer(1, static_vertex_buffer.slice(..));
+    pass.set_vertex_buffer(2, changing_vertex_buffer.slice(..));

    ...
-    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..num_vertices, 0..k_num_objects as u32);
```

Here the first parameter to `set_vertex_buffer` corresponds to the elements of
the `buffers` array in the pipeline we created above.

With that we have the same thing we had before, but we're using all vertex buffers
and no storage buffers.

{{{example url="../webgpu-vertex-buffers-instanced-colors.html"}}}

Just for fun, let's add another attribute for a per vertex color. First let's change the shader:

```wgsl
struct Vertex {
  @location(0) position: vec2f,
  @location(1) color: vec4f,
  @location(2) offset: vec2f,
  @location(3) scale: vec2f,
+  @location(4) perVertexColor: vec3f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

@vertex fn vs(
  vert: Vertex,
) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = vec4f(
      vert.position * vert.scale + vert.offset, 0.0, 1.0);
-  vsOut.color = vert.color;
+  vsOut.color = vert.color * vec4f(vert.perVertexColor, 1);
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vsOut.color;
}
```

Then we need to update the pipeline to describe how we'll supply the data.
We're going to interleave the `perVertexColor` data with the `position` like this:

<div class="webgpu_center"><img src="resources/vertex-buffer-mixed.svg" style="width: 1024px;"></div>

So, the `array_stride` needs to be changed to cover our new data and we need
to add the new attribute. It starts after two 32bit floating point numbers
so its `offset` into the buffer is 8 bytes.

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("per vertex color"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[
        Some(wgpu::VertexBufferLayout {
-          array_stride: 2 * 4, // 2 floats, 4 bytes each
+          array_stride: 5 * 4, // 5 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[
            // position
            wgpu::VertexAttribute {
              shader_location: 0,
              offset: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
+            // perVertexColor
+            wgpu::VertexAttribute {
+              shader_location: 4,
+              offset: 8,
+              format: wgpu::VertexFormat::Float32x3,
+            },
          ],
        }),
        Some(wgpu::VertexBufferLayout {
          array_stride: 6 * 4, // 6 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &[
            // color
            wgpu::VertexAttribute {
              shader_location: 1,
              offset: 0,
              format: wgpu::VertexFormat::Float32x4,
            },
            // offset
            wgpu::VertexAttribute {
              shader_location: 2,
              offset: 16,
              format: wgpu::VertexFormat::Float32x2,
            },
          ],
        }),
        Some(wgpu::VertexBufferLayout {
          array_stride: 2 * 4, // 2 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &[
            // scale
            wgpu::VertexAttribute {
              shader_location: 3,
              offset: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
          ],
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

First, a small refactor: in the previous article `create_circle_vertices`
took positional arguments. The parameter list is about to grow, so — like
the JavaScript version's options object with default values — we'll move
the parameters into an options struct with a `Default` impl.

```rust
struct CircleVerticesOptions {
  radius: f32,
  num_subdivisions: u32,
  inner_radius: f32,
  start_angle: f32,
  end_angle: f32,
}

impl Default for CircleVerticesOptions {
  fn default() -> Self {
    Self {
      radius: 1.0,
      num_subdivisions: 24,
      inner_radius: 0.0,
      start_angle: 0.0,
      end_angle: std::f32::consts::PI * 2.0,
    }
  }
}
```

Callers set the fields they care about and take the rest from the defaults:
`create_circle_vertices(CircleVerticesOptions { radius: 0.5, inner_radius: 0.25, ..Default::default() })`.

We'll update the circle vertex generation code to provide a dark color
for vertices on the outer edge of the circle and a light color for
the inner vertices.

```rust
fn create_circle_vertices(options: CircleVerticesOptions) -> (Vec<f32>, u32) {
  let CircleVerticesOptions {
    radius,
    num_subdivisions,
    inner_radius,
    start_angle,
    end_angle,
  } = options;
  // 2 triangles per subdivision, 3 verts per tri, 5 values (xyrgb) each.
  let num_vertices = num_subdivisions * 3 * 2;
-  let mut vertex_data = vec![0.0f32; (num_vertices * 2) as usize];
+  let mut vertex_data = vec![0.0f32; (num_vertices * (2 + 3)) as usize];

  let mut offset = 0;
-  let mut add_vertex = |x: f32, y: f32| {
+  let mut add_vertex = |x: f32, y: f32, [r, g, b]: [f32; 3]| {
    vertex_data[offset] = x;
    offset += 1;
    vertex_data[offset] = y;
    offset += 1;
+    vertex_data[offset] = r;
+    offset += 1;
+    vertex_data[offset] = g;
+    offset += 1;
+    vertex_data[offset] = b;
+    offset += 1;
  };

+  let inner_color = [1.0, 1.0, 1.0];
+  let outer_color = [0.1, 0.1, 0.1];

  // 2 triangles per subdivision
  //
  // 0--1 4
  // | / /|
  // |/ / |
  // 2 3--5
  for i in 0..num_subdivisions {
    let angle1 =
        start_angle + (i + 0) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
    let angle2 =
        start_angle + (i + 1) as f32 * (end_angle - start_angle) / num_subdivisions as f32;

    let c1 = angle1.cos();
    let s1 = angle1.sin();
    let c2 = angle2.cos();
    let s2 = angle2.sin();

    // first triangle
-    add_vertex(c1 * radius, s1 * radius);
-    add_vertex(c2 * radius, s2 * radius);
-    add_vertex(c1 * inner_radius, s1 * inner_radius);
+    add_vertex(c1 * radius, s1 * radius, outer_color);
+    add_vertex(c2 * radius, s2 * radius, outer_color);
+    add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);

    // second triangle
-    add_vertex(c1 * inner_radius, s1 * inner_radius);
-    add_vertex(c2 * radius, s2 * radius);
-    add_vertex(c2 * inner_radius, s2 * inner_radius);
+    add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);
+    add_vertex(c2 * radius, s2 * radius, outer_color);
+    add_vertex(c2 * inner_radius, s2 * inner_radius, inner_color);
  }

  (vertex_data, num_vertices)
}
```

And with that we get shaded circles:

{{{example url="../webgpu-vertex-buffers-per-vertex-colors.html"}}}

## <a id="a-default-values"></a>Attributes in WGSL do not have to match attributes in Rust

Above in WGSL we declared the `perVertexColor` attribute as a `vec3f` like this:

```wgsl
struct Vertex {
  @location(0) position: vec2f,
  @location(1) color: vec4f,
  @location(2) offset: vec2f,
  @location(3) scale: vec2f,
*  @location(4) perVertexColor: vec3f,
};
```

And used it like this:

```wgsl
@vertex fn vs(
  vert: Vertex,
) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = vec4f(
      vert.position * vert.scale + vert.offset, 0.0, 1.0);
*  vsOut.color = vert.color * vec4f(vert.perVertexColor, 1);
  return vsOut;
}
```

We could also declare it as a `vec4f` and use it like this:

```wgsl
struct Vertex {
  @location(0) position: vec2f,
  @location(1) color: vec4f,
  @location(2) offset: vec2f,
  @location(3) scale: vec2f,
-  @location(4) perVertexColor: vec3f,
+  @location(4) perVertexColor: vec4f,
};

...

@vertex fn vs(
  vert: Vertex,
) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = vec4f(
      vert.position * vert.scale + vert.offset, 0.0, 1.0);
-  vsOut.color = vert.color * vec4f(vert.perVertexColor, 1);
+  vsOut.color = vert.color * vert.perVertexColor;
  return vsOut;
}
```

And change nothing else. In Rust we're still only supplying the data as
3 floats per vertex.

```rust
    Some(wgpu::VertexBufferLayout {
      array_stride: 5 * 4, // 5 floats, 4 bytes each
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &[
        // position
        wgpu::VertexAttribute {
          shader_location: 0,
          offset: 0,
          format: wgpu::VertexFormat::Float32x2,
        },
*        // perVertexColor
*        wgpu::VertexAttribute {
*          shader_location: 4,
*          offset: 8,
*          format: wgpu::VertexFormat::Float32x3,
*        },
      ],
    }),
```

This works because attributes always have 4 values available in the shader. They default
to `0, 0, 0, 1` so any values we don't supply get these defaults.

{{{example url="../webgpu-vertex-buffers-per-vertex-colors-3-in-4-out.html"}}}

## <a id="a-normalized-attributes"></a>Using normalized values to save space

We're using 32bit floating point values for colors. Each `perVertexColor` has 3 values for a total of 
12 bytes per color per vertex. Each `color` has 4 values for a total of 16 bytes per color per instance.

We could optimize that by using 8bit values and telling WebGPU they should be normalized from 0 ↔ 255 to 0.0 ↔ 1.0.

Looking at the list of valid attribute formats there is no 3 value 8bit format but there is `Unorm8x4` so let's
use that.

First let's change the code that generates the vertices to store colors as 8bit values that
will be normalized:

```rust
fn create_circle_vertices(options: CircleVerticesOptions) -> (Vec<f32>, u32) {
  let CircleVerticesOptions {
    radius,
    num_subdivisions,
    inner_radius,
    start_angle,
    end_angle,
  } = options;
-  // 2 triangles per subdivision, 3 verts per tri, 5 values (xyrgb) each.
+  // 2 triangles per subdivision, 3 verts per tri
  let num_vertices = num_subdivisions * 3 * 2;
-  let mut vertex_data = vec![0.0f32; (num_vertices * (2 + 3)) as usize];
+  // 2 32-bit values for position (xy) and 1 32-bit value for color (rgb_)
+  // The 32-bit color value will be written/read as 4 8-bit values
+  let mut vertex_data = vec![0.0f32; (num_vertices * (2 + 1)) as usize];

  let mut offset = 0;
+  let mut color_offset = 8;
  let mut add_vertex = |x: f32, y: f32, [r, g, b]: [f32; 3]| {
    vertex_data[offset] = x;
    offset += 1;
    vertex_data[offset] = y;
    offset += 1;
-    vertex_data[offset] = r;
-    offset += 1;
-    vertex_data[offset] = g;
-    offset += 1;
-    vertex_data[offset] = b;
-    offset += 1;
+    offset += 1;  // skip the color
+
+    // a u8 view of the same data as vertex_data
+    let color_data: &mut [u8] = bytemuck::cast_slice_mut(&mut vertex_data);
+    color_data[color_offset] = (r * 255.0) as u8;
+    color_offset += 1;
+    color_data[color_offset] = (g * 255.0) as u8;
+    color_offset += 1;
+    color_data[color_offset] = (b * 255.0) as u8;
+    color_offset += 1;
+    color_offset += 9;  // skip extra byte and the position
  };
```

Above we make `color_data`, which is a `&mut [u8]` view of the same
data as `vertex_data`. `bytemuck::cast_slice_mut` reinterprets the
`f32`s as bytes in place — no copying. Since Rust won't let us keep an
`f32` view and a `u8` view borrowed at the same time, we create the byte
view right where we need it and let the borrow end. Review the
[data memory layout article](webgpu-memory-layout.html#multiple-views-of-the-same-arraybuffer) if this is unclear.

We then use `color_data` to insert the colors, expanding them from 0 ↔ 1
to 0 ↔ 255.

The memory layout of this (per vertex) data is like this:

<div class="webgpu_center"><img src="resources/vertex-buffer-f32x2-u8x4.svg" style="width: 1024px;"></div>

We also need to update the per instance data.

```rust
  let k_num_objects = 100;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  // create 2 vertex buffers
  let static_unit_size =
-      4 * 4 + // color is 4 32bit floats (4bytes each)
+      4 +     // color is 4 bytes
      2 * 4; // offset is 2 32bit floats (4bytes each)
  let changing_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
  let static_vertex_buffer_size = static_unit_size * k_num_objects;
  let changing_vertex_buffer_size = changing_unit_size * k_num_objects;

  let static_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("static vertex for objects"),
    size: static_vertex_buffer_size as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let changing_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("changing storage for objects"),
    size: changing_vertex_buffer_size as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  // offsets to the various uniform values in float32 indices
  let k_color_offset = 0;
-  let k_offset_offset = 4;
+  let k_offset_offset = 1;

  let k_scale_offset = 0;

  {
-    let mut static_vertex_values = vec![0.0f32; static_vertex_buffer_size / 4];
+    let mut static_vertex_values_f32 = vec![0.0f32; static_vertex_buffer_size / 4];
    for i in 0..k_num_objects {
-      let static_offset = i * (static_unit_size / 4);
+      let static_offset_u8 = i * static_unit_size;
+      let static_offset_f32 = static_offset_u8 / 4;

      // These are only set once so set them now
-      static_vertex_values[static_offset + k_color_offset..][..4].copy_from_slice(&[
-        rand(0.0, 1.0),
-        rand(0.0, 1.0),
-        rand(0.0, 1.0),
-        1.0,
-      ]); // set the color
+      // a u8 view of the same data as static_vertex_values_f32
+      let static_vertex_values_u8: &mut [u8] =
+          bytemuck::cast_slice_mut(&mut static_vertex_values_f32);
+      static_vertex_values_u8[static_offset_u8 + k_color_offset..][..4].copy_from_slice(&[
+        (rand(0.0, 1.0) * 255.0) as u8,
+        (rand(0.0, 1.0) * 255.0) as u8,
+        (rand(0.0, 1.0) * 255.0) as u8,
+        255,
+      ]); // set the color

-      static_vertex_values[static_offset + k_offset_offset..][..2]
+      static_vertex_values_f32[static_offset_f32 + k_offset_offset..][..2]
        .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

      object_infos.push(ObjectInfo {
        scale: rand(0.2, 0.5),
      });
    }
-    app.queue.write_buffer(&static_vertex_buffer, 0, bytemuck::cast_slice(&static_vertex_values));
+    app.queue.write_buffer(&static_vertex_buffer, 0, bytemuck::cast_slice(&static_vertex_values_f32));
  }
```

The layout for the per instance data is like this:

<div class="webgpu_center"><img src="resources/vertex-buffer-u8x4-f32x2.svg" style="width: 1024px;"></div>

We then need to change the pipeline to pull out the data as 8bit unsigned
values and to normalize them back to 0 ↔ 1, update the offsets, and update the stride to its
new size.

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("per vertex color"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[
        Some(wgpu::VertexBufferLayout {
-          array_stride: 5 * 4, // 5 floats, 4 bytes each
+          array_stride: 2 * 4 + 4, // 2 floats, 4 bytes each + 4 bytes
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[
            // position
            wgpu::VertexAttribute {
              shader_location: 0,
              offset: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
            // perVertexColor
            wgpu::VertexAttribute {
              shader_location: 4,
              offset: 8,
-              format: wgpu::VertexFormat::Float32x3,
+              format: wgpu::VertexFormat::Unorm8x4,
            },
          ],
        }),
        Some(wgpu::VertexBufferLayout {
-          array_stride: 6 * 4, // 6 floats, 4 bytes each
+          array_stride: 4 + 2 * 4, // 4 bytes + 2 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &[
            // color
            wgpu::VertexAttribute {
              shader_location: 1,
              offset: 0,
-              format: wgpu::VertexFormat::Float32x4,
+              format: wgpu::VertexFormat::Unorm8x4,
            },
            // offset
            wgpu::VertexAttribute {
              shader_location: 2,
-              offset: 16,
+              offset: 4,
              format: wgpu::VertexFormat::Float32x2,
            },
          ],
        }),
        Some(wgpu::VertexBufferLayout {
          array_stride: 2 * 4, // 2 floats, 4 bytes each
          step_mode: wgpu::VertexStepMode::Instance,
          attributes: &[
            // scale
            wgpu::VertexAttribute {
              shader_location: 3,
              offset: 0,
              format: wgpu::VertexFormat::Float32x2,
            },
          ],
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

And with that we've save a little space. We were using 20 bytes per vertex,
now we're using 12 bytes per vertex, a 40% savings. And we were using 24 bytes
per instance, now we're using 12, a 50% savings.

{{{example url="../webgpu-vertex-buffers-8bit-colors.html"}}}

Note that we don't have to use a struct. This would work just as well:

```WGSL
@vertex fn vs(
-  vert: Vertex,
+  @location(0) position: vec2f,
+  @location(1) color: vec4f,
+  @location(2) offset: vec2f,
+  @location(3) scale: vec2f,
+  @location(4) perVertexColor: vec3f,
) -> VSOutput {
  var vsOut: VSOutput;
-  vsOut.position = vec4f(
-      vert.position * vert.scale + vert.offset, 0.0, 1.0);
-  vsOut.color = vert.color * vec4f(vert.perVertexColor, 1);
+  vsOut.position = vec4f(
+      position * scale + offset, 0.0, 1.0);
+  vsOut.color = color * vec4f(perVertexColor, 1);
  return vsOut;
}
```

As again, all WebGPU cares about that we define `locations` in the shader
and supply data to those locations via the API.

## <a id="a-index-buffers"></a>Index Buffers

One last thing to cover here are index buffers. Index buffers describe
the order to process and use the vertices.

You can think of `draw` as going through the vertices in order:

```
0, 1, 2, 3, 4, 5, .....
```

With an index buffer we can change that order.

We were creating 6 vertices per subdivision of the circle even though 2
of them were identical.

<div class="webgpu_center"><img src="resources/vertices-non-indexed.svg" style="width: 400px"></div>  

Now instead, we'll only create 4 but then use indices to
use those 4 vertices 6 times by telling WebGPU to draw indices in this order:

```
0, 1, 2, 2, 1, 3, ...
```

<div class="webgpu_center"><img src="resources/vertices-indexed.svg" style="width: 400px"></div>

```rust
fn create_circle_vertices(options: CircleVerticesOptions) -> (Vec<f32>, Vec<u32>, u32) {
  let CircleVerticesOptions {
    radius,
    num_subdivisions,
    inner_radius,
    start_angle,
    end_angle,
  } = options;
-  // 2 triangles per subdivision, 3 verts per tri
-  let num_vertices = num_subdivisions * 3 * 2;
+  // 2 vertices at each subdivision, + 1 to wrap around the circle.
+  let num_vertices = (num_subdivisions + 1) * 2;
  // 2 32-bit values for position (xy) and 1 32-bit value for color (rgb)
  // The 32-bit color value will be written/read as 4 8-bit values
  let mut vertex_data = vec![0.0f32; (num_vertices * (2 + 1)) as usize];

  let mut offset = 0;
  let mut color_offset = 8;
  let mut add_vertex = |x: f32, y: f32, [r, g, b]: [f32; 3]| {
    vertex_data[offset] = x;
    offset += 1;
    vertex_data[offset] = y;
    offset += 1;
    offset += 1;  // skip the color

    // a u8 view of the same data as vertex_data
    let color_data: &mut [u8] = bytemuck::cast_slice_mut(&mut vertex_data);
    color_data[color_offset] = (r * 255.0) as u8;
    color_offset += 1;
    color_data[color_offset] = (g * 255.0) as u8;
    color_offset += 1;
    color_data[color_offset] = (b * 255.0) as u8;
    color_offset += 1;
    color_offset += 9;  // skip extra byte and the position
  };
  let inner_color = [1.0, 1.0, 1.0];
  let outer_color = [0.1, 0.1, 0.1];

-  // 2 triangles per subdivision
-  //
-  // 0--1 4
-  // | / /|
-  // |/ / |
-  // 2 3--5
-  for i in 0..num_subdivisions {
-    let angle1 =
-        start_angle + (i + 0) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
-    let angle2 =
-        start_angle + (i + 1) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
-
-    let c1 = angle1.cos();
-    let s1 = angle1.sin();
-    let c2 = angle2.cos();
-    let s2 = angle2.sin();
-
-    // first triangle
-    add_vertex(c1 * radius, s1 * radius, outer_color);
-    add_vertex(c2 * radius, s2 * radius, outer_color);
-    add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);
-
-    // second triangle
-    add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);
-    add_vertex(c2 * radius, s2 * radius, outer_color);
-    add_vertex(c2 * inner_radius, s2 * inner_radius, inner_color);
-  }
+  // 2 triangles per subdivision
+  //
+  // 0  2  4  6  8 ...
+  //
+  // 1  3  5  7  9 ...
+  for i in 0..=num_subdivisions {
+    let angle =
+        start_angle + (i + 0) as f32 * (end_angle - start_angle) / num_subdivisions as f32;
+
+    let c1 = angle.cos();
+    let s1 = angle.sin();
+
+    add_vertex(c1 * radius, s1 * radius, outer_color);
+    add_vertex(c1 * inner_radius, s1 * inner_radius, inner_color);
+  }

+  let mut index_data = vec![0u32; (num_subdivisions * 6) as usize];
+  let mut ndx = 0;
+
+  // 1st tri  2nd tri  3rd tri  4th tri
+  // 0 1 2    2 1 3    2 3 4    4 3 5
+  //
+  // 0--2        2     2--4        4  .....
+  // | /        /|     | /        /|
+  // |/        / |     |/        / |
+  // 1        1--3     3        3--5  .....
+  for i in 0..num_subdivisions {
+    let ndx_offset = i * 2;
+
+    // first triangle
+    index_data[ndx] = ndx_offset;
+    ndx += 1;
+    index_data[ndx] = ndx_offset + 1;
+    ndx += 1;
+    index_data[ndx] = ndx_offset + 2;
+    ndx += 1;
+
+    // second triangle
+    index_data[ndx] = ndx_offset + 2;
+    ndx += 1;
+    index_data[ndx] = ndx_offset + 1;
+    ndx += 1;
+    index_data[ndx] = ndx_offset + 3;
+    ndx += 1;
+  }

+  let num_vertices = index_data.len() as u32;
-  (vertex_data, num_vertices)
+  (vertex_data, index_data, num_vertices)
}
```

Then we need to create an index buffer:

```rust
-  let (vertex_data, num_vertices) = create_circle_vertices(CircleVerticesOptions {
+  let (vertex_data, index_data, num_vertices) = create_circle_vertices(CircleVerticesOptions {
    radius: 0.5,
    inner_radius: 0.25,
    ..Default::default()
  });
  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("vertex buffer"),
    size: (vertex_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
+  let index_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+    label: Some("index buffer"),
+    size: (index_data.len() * 4) as u64,
+    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
+    mapped_at_creation: false,
+  });
+  app.queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));
```

Notice we set the usage to `INDEX`.

Then finally at draw time we need to specify the index buffer:

```rust
    pass.set_pipeline(&pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.set_vertex_buffer(1, static_vertex_buffer.slice(..));
    pass.set_vertex_buffer(2, changing_vertex_buffer.slice(..));
+    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
```

Because our buffer contains 32bit unsigned integer indices
we need to pass `wgpu::IndexFormat::Uint32` here. We could also use 16 bit
unsigned indices in which case we'd pass in `wgpu::IndexFormat::Uint16`.

And we need to call `draw_indexed` instead of `draw`:

```rust
-    pass.draw(0..num_vertices, 0..k_num_objects as u32);
+    pass.draw_indexed(0..num_vertices, 0, 0..k_num_objects as u32);
```

The extra middle parameter, `0`, is a *base vertex* — a value added to every
index pulled from the index buffer. We don't need one here.

With that we saved some space (33%) and, potentially
a similar amount of processing when computing vertices
in the vertex shader as it's possible the GPU can reuse
vertices it has already calculated.

{{{example url="../webgpu-vertex-buffers-index-buffer.html"}}}

Note that we could have also used an index buffer with the
storage buffer example from [the previous article](webgpu-storage-buffers.html).
In that case the value from `@builtin(vertex_index)` that's passed in matches the index
from the index buffer.

Next up we'll cover [textures](webgpu-textures.html).

