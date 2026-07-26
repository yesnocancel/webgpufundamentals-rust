Title: WebGPU Points
Description: Drawing Points in WebGPU
TOC: Points

WebGPU supports drawing to points. We do this by setting the
primitive topology to `'point-list'` in a render pipeline.

Let's create a simple example with random points
starting with ideas presented in [the article on vertex buffers](webgpu-vertex-buffers.html).

First, a simple vertex shader and fragment shader. To keep it simple we'll
just use clip space coordinates for positions and hard code the color
yellow in our fragment shader.

```wgsl
struct Vertex {
  @location(0) position: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@vertex fn vs(vert: Vertex,) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = vert.position;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vec4f(1, 1, 0, 1); // yellow
}
```

Then, when we create a pipeline, we set the topology to `PointList`

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("1 pixel points"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
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
      })],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
+    primitive: wgpu::PrimitiveState {
+      topology: wgpu::PrimitiveTopology::PointList,
+      ..Default::default()
+    },
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

Let's fill a vertex buffer with some random clips space points, using the
same deterministic `rand(min, max)` helper as the earlier lessons

```rust
  const K_NUM_POINTS: usize = 100;
  let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 2];
  for i in 0..K_NUM_POINTS {
    let offset = i * 2;
    vertex_data[offset] = rand(-1.0, 1.0);
    vertex_data[offset + 1] = rand(-1.0, 1.0);
  }

  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("vertex buffer vertices"),
    size: (vertex_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
```

And then draw 

```rust
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
      pass.draw(0..K_NUM_POINTS as u32, 0..1);
    }
```

And with that we get 100 random yellow points

{{{example url="../webgpu-points.html"}}}

Unfortunately they are all only 1 pixel in size. 1 pixel size points is all WebGPU
supports. If we want something larger we need to do it ourselves. Fortunately it's
easy to do. We'll just make a quad and use [instancing](webgpu-vertex-buffers.html#a-instancing);

Let's add a quad to our vertex shader and a size attribute. Let's also add a uniform
to pass in the size of the texture we are drawing to.

```wgsl
struct Vertex {
  @location(0) position: vec2f,
+  @location(1) size: f32,
};

+struct Uniforms {
+  resolution: vec2f,
+};

struct VSOutput {
  @builtin(position) position: vec4f,
};

+@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(
    vert: Vertex,
+    @builtin(vertex_index) vNdx: u32,
) -> VSOutput {
+  let points = array(
+    vec2f(-1, -1),
+    vec2f( 1, -1),
+    vec2f(-1,  1),
+    vec2f(-1,  1),
+    vec2f( 1, -1),
+    vec2f( 1,  1),
+  );
  var vsOut: VSOutput;
+  let pos = points[vNdx];
-  vsOut.position = vec4f(vert.position, 0, 1);
+  vsOut.position = vec4f(vert.position + pos * vert.size / uni.resolution, 0, 1);
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vec4f(1, 1, 0, 1); // yellow
}
```

In Rust we need to add an attribute for a size per point, we need to set
the attributes to advance per instance by setting the step mode to `Instance`, and we
can remove the topology setting since we want the default `TriangleList`

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("sizeable points"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: 2 * 4, // 2 floats, 4 bytes each
+        array_stride: (2 + 1) * 4, // 3 floats, 4 bytes each
-        step_mode: wgpu::VertexStepMode::Vertex,
+        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x2,
          },
+          // size
+          wgpu::VertexAttribute {
+            shader_location: 1,
+            offset: 8,
+            format: wgpu::VertexFormat::Float32,
+          },
        ],
      })],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
-    primitive: wgpu::PrimitiveState {
-      topology: wgpu::PrimitiveTopology::PointList,
-      ..Default::default()
-    },
+    primitive: Default::default(),
    ...
  });
```

Let's add a random size per point to our vertex data

```rust
  const K_NUM_POINTS: usize = 100;
-  let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 2];
+  let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 3];
  for i in 0..K_NUM_POINTS {
-    let offset = i * 2;
+    let offset = i * 3;
    vertex_data[offset] = rand(-1.0, 1.0);
    vertex_data[offset + 1] = rand(-1.0, 1.0);
+    vertex_data[offset + 2] = rand(1.0, 32.0);
  }
```

We need a uniform buffer so we can pass in the resolution

```rust
  let mut uniform_values = [0.0f32; 2];
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: (uniform_values.len() * 4) as u64,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  const K_RESOLUTION_OFFSET: usize = 0;
```

And we need a bind group to bind the uniform buffer

```rust
  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buffer.as_entire_binding(),
    }],
  });
```

Then at render time we can update the uniform buffer with the current
resolution.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
+    // Update the resolution in the uniform buffer
+    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
+        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
+    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

then set our bind group and render an instance per point

```rust
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
+      pass.set_bind_group(0, &bind_group, &[]);
-      pass.draw(0..K_NUM_POINTS as u32, 0..1);
+      pass.draw(0..6, 0..K_NUM_POINTS as u32);
    }
```

And now we have sizable points

{{{example url="../webgpu-points-w-size.html"}}}

What if we wanted to texture our points? We just need to pass in
texture coordinates from the vertex shader to the fragment shader.

```wgsl
struct Vertex {
  @location(0) position: vec2f,
  @location(1) size: f32,
};

struct Uniforms {
  resolution: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
+  @location(0) texcoord: vec2f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(
    vert: Vertex,
    @builtin(vertex_index) vNdx: u32,
) -> VSOutput {
  let points = array(
    vec2f(-1, -1),
    vec2f( 1, -1),
    vec2f(-1,  1),
    vec2f(-1,  1),
    vec2f( 1, -1),
    vec2f( 1,  1),
  );
  var vsOut: VSOutput;
  let pos = points[vNdx];
  vsOut.position = vec4f(vert.position + pos * vert.size / uni.resolution, 0, 1);
+  vsOut.texcoord = pos * 0.5 + 0.5;
  return vsOut;
}
```

And of course use a texture in the fragment shader

```wgsl
+@group(0) @binding(1) var s: sampler;
+@group(0) @binding(2) var t: texture_2d<f32>;

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
-  return vec4f(1, 1, 0, 1); // yellow
+  return textureSample(t, s, vsOut.texcoord);
}
```

The JS version draws a 🥑 emoji into a small canvas; we load a pre-made
32x32 image of the same emoji like we covered in
[the article on importing textures](webgpu-importing-textures.html), and
premultiply its alpha ourselves (the JS version asks the browser for that
with `premultipliedAlpha: true`).

```rust
  let mut source = wgpu_fun::load_image("resources/images/emoji/avocado.png").await;
  for pixel in source.data.chunks_mut(4) {
    let a = pixel[3] as u32;
    pixel[0] = (pixel[0] as u32 * a / 255) as u8;
    pixel[1] = (pixel[1] as u32 * a / 255) as u8;
    pixel[2] = (pixel[2] as u32 * a / 255) as u8;
  }

  let texture = app.device.create_texture(&wgpu::TextureDescriptor {
    size: wgpu::Extent3d { width: 32, height: 32, depth_or_array_layers: 1 },
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::RENDER_ATTACHMENT,
    ...
  });
  // upload, flipping Y like the JS version
  app.queue.write_texture(/* ... flipped rows of source.data ... */);
```

And we need a sampler and we need to add them to our bind group

```rust
  let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
    min_filter: wgpu::FilterMode::Linear,
    mag_filter: wgpu::FilterMode::Linear,
    ..Default::default()
  });

  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
+      wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
+      wgpu::BindGroupEntry {
+        binding: 2,
+        resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
+      },
    ],
    ...
  });
```

Let's also turn on blending so we get [transparency](webgpu-transparency.html)

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("sizeable points with texture"),
    ...
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
-      targets: &[Some(app.format.into())],
+      targets: &[Some(wgpu::ColorTargetState {
+        format: app.format,
+        blend: Some(wgpu::BlendState {
+          color: wgpu::BlendComponent {
+            src_factor: wgpu::BlendFactor::One,
+            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
+            operation: wgpu::BlendOperation::Add,
+          },
+          alpha: wgpu::BlendComponent {
+            src_factor: wgpu::BlendFactor::One,
+            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
+            operation: wgpu::BlendOperation::Add,
+          },
+        }),
+        write_mask: Default::default(),
+      })],
    }),
  });
```

And now we have textured points

{{{example url="../webgpu-points-w-texture.html"}}}

And we could keep going, how about a rotation per point? Using the math we covered
in [the article on matrix math](webgpu-matrix-math.html).

```wgsl
struct Vertex {
  @location(0) position: vec2f,
  @location(1) size: f32,
+  @location(2) rotation: f32,
};

struct Uniforms {
  resolution: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) texcoord: vec2f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(
    vert: Vertex,
    @builtin(vertex_index) vNdx: u32,
) -> VSOutput {
  let points = array(
    vec2f(-1, -1),
    vec2f( 1, -1),
    vec2f(-1,  1),
    vec2f(-1,  1),
    vec2f( 1, -1),
    vec2f( 1,  1),
  );
  var vsOut: VSOutput;
  let pos = points[vNdx];
+  let c = cos(vert.rotation);
+  let s = sin(vert.rotation);
+  let rot = mat2x2f(
+     c, s,
+    -s, c,
+  );
-  vsOut.position = vec4f(vert.position + pos * vert.size / uni.resolution, 0, 1);
+  vsOut.position = vec4f(vert.position + rot * pos * vert.size / uni.resolution, 0, 1);
  vsOut.texcoord = pos * 0.5 + 0.5;
  return vsOut;
      }
```

We need to add the rotation attribute to our pipeline

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("sizeable rotatable points with texture"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      ...
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (2 + 1) * 4, // 3 floats, 4 bytes each
+        array_stride: (2 + 1 + 1) * 4, // 4 floats, 4 bytes each
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
          // position
          wgpu::VertexAttribute { shader_location: 0, offset: 0, format: wgpu::VertexFormat::Float32x2 },
          // size
          wgpu::VertexAttribute { shader_location: 1, offset: 8, format: wgpu::VertexFormat::Float32 },
+          // rotation
+          wgpu::VertexAttribute { shader_location: 2, offset: 12, format: wgpu::VertexFormat::Float32 },
        ],
      })],
    },
    ...
```

We need to add rotation to our vertex data

```rust
  const K_NUM_POINTS: usize = 100;
-  let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 3];
+  let mut vertex_data = vec![0.0f32; K_NUM_POINTS * 4];
  for i in 0..K_NUM_POINTS {
-    let offset = i * 3;
+    let offset = i * 4;
    vertex_data[offset] = rand(-1.0, 1.0);
    vertex_data[offset + 1] = rand(-1.0, 1.0);
*    vertex_data[offset + 2] = rand(10.0, 64.0);
+    vertex_data[offset + 3] = rand(0.0, std::f32::consts::PI * 2.0);
  }
```

Let's also change the texture from 🥑 to 👉

```rust
-  let mut source = wgpu_fun::load_image("resources/images/emoji/avocado.png").await;
+  let mut source = wgpu_fun::load_image("resources/images/emoji/pointing-right.png").await;
```

{{{example url="../webgpu-points-w-rotation.html" }}}

# What about points in 3D?

The simple answer is just add in the quad values after doing
[the 3d math for the vertices](webgpu-perspective-projection.html).

For example, here's some code to make 3d positions for a
[fibonacci sphere](https://www.google.com/search?q=fibonacci+sphere).

```rust
fn create_fibonacci_sphere_vertices(
    FibonacciSphereOptions { num_samples, radius }: FibonacciSphereOptions,
) -> Vec<f32> {
    let mut vertices = Vec::new();
    let increment = std::f32::consts::PI * (3.0 - 5.0f32.sqrt());
    for i in 0..num_samples {
        let offset = 2.0 / num_samples as f32;
        let y = ((i as f32 * offset) - 1.0) + (offset / 2.0);
        let r = (1.0 - y * y).sqrt();
        let phi = (i % num_samples) as f32 * increment;
        let x = phi.cos() * r;
        let z = phi.sin() * r;
        vertices.extend_from_slice(&[x * radius, y * radius, z * radius]);
    }
    vertices
}
```

We can draw the vertices with points by applying 3D math to the vertices
like [we covered in the series on 3d math](webgpu-cameras.js).

```wgsl
struct Vertex {
  @location(0) position: vec4f,
};

struct Uniforms {
*  matrix: mat4x4f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(
    vert: Vertex,
) -> VSOutput {
  var vsOut: VSOutput;
*  let clipPos = uni.matrix * vert.position;
  vsOut.position = clipPos;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vec4f(1, 0.5, 0.2, 1);  // orange
}
```

Here's our pipeline and vertex buffer

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("3d points with fixed size"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
        array_stride: (3) * 4, // 3 floats, 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
        ],
      })],
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
    primitive: wgpu::PrimitiveState {
      topology: wgpu::PrimitiveTopology::PointList,
      ..Default::default()
    },
    ...
  });

  let vertex_data = create_fibonacci_sphere_vertices(FibonacciSphereOptions {
    radius: 1.0,
    num_samples: 1000,
  });
  let k_num_points = (vertex_data.len() / 3) as u32;

  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("vertex buffer vertices"),
    size: (vertex_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
```

And, a uniform buffer for our matrix as well
as a bind group to pass the uniform buffer our shader.

```rust
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: 16 * 4,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buffer.as_entire_binding(),
    }],
  });
```

And the code to draw using a projection matrix, camera, and other
3d math (via [`glam`](https://docs.rs/glam)).

```rust
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let time = frame.time as f32;

    // Set the matrix in the uniform buffer
    let fov = 90.0f32.to_radians();
    let aspect = frame.width as f32 / frame.height as f32;
    let projection = Mat4::perspective_rh(fov, aspect, 0.1, 50.0);
    let view = Mat4::look_at_rh(
      Vec3::new(0.0, 0.0, 1.5), // position
      Vec3::new(0.0, 0.0, 0.0), // target
      Vec3::new(0.0, 1.0, 0.0), // up
    );
    let view_projection = projection * view;
    let matrix = view_projection
        * Mat4::from_rotation_y(time)
        * Mat4::from_rotation_x(time * 0.5);

    // Copy the uniform values to the GPU
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&matrix.to_cols_array()));

    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
      pass.set_bind_group(0, &bind_group, &[]);
      pass.draw(0..k_num_points, 0..1);
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

We also switched to `RenderMode::Continuous` (a `requestAnimationFrame` loop).

{{{example url="../webgpu-points-3d-1px.html"}}}

That's hard to see, so, to apply the techniques above, we just
add the in quad position just like we did previously.

```wgsl
struct Vertex {
  @location(0) position: vec4f,
};

struct Uniforms {
  matrix: mat4x4f,
+  resolution: vec2f,
+  size: f32,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(
    vert: Vertex,
+    @builtin(vertex_index) vNdx: u32,
) -> VSOutput {
+  let points = array(
+    vec2f(-1, -1),
+    vec2f( 1, -1),
+    vec2f(-1,  1),
+    vec2f(-1,  1),
+    vec2f( 1, -1),
+    vec2f( 1,  1),
+  );
  var vsOut: VSOutput;
+  let pos = points[vNdx];
  let clipPos = uni.matrix * vert.position;
+  let pointPos = vec4f(pos * uni.size / uni.resolution, 0, 0);
-  vsOut.position = clipPos;
+  vsOut.position = clipPos + pointPos;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return vec4f(1, 0.5, 0.2, 1);
}
```

Unlike the previous example we won't use a different size for each vertex.
Instead we'll pass a single size for all vertices.

```rust
-  let uniform_buffer_len = 16;
+  // matrix, resolution, size, padding
+  let uniform_buffer_len = 16 + 2 + 1 + 1;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: (uniform_buffer_len * 4) as u64,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  const K_MATRIX_OFFSET: usize = 0;
+  const K_RESOLUTION_OFFSET: usize = 16;
+  const K_SIZE_OFFSET: usize = 18;
```

We need to set the resolution as we did above, and we need to set a size

```rust
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    ...
    let mut uniform_values = [0.0f32; 16 + 2 + 1 + 1];
    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
        .copy_from_slice(&matrix.to_cols_array());
+    // Update the resolution in the uniform buffer
+    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
+        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
+    // Set the size in the uniform buffer
+    uniform_values[K_SIZE_OFFSET] = 10.0;

    // Copy the uniform values to the GPU
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

And, like we did before, we need to switch from drawing points to drawing
instanced quads

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("3d points"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      ...
      buffers: &[Some(wgpu::VertexBufferLayout {
        array_stride: (3) * 4, // 3 floats, 4 bytes each
-        step_mode: wgpu::VertexStepMode::Vertex,
+        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
        ],
      })],
    },
    ...
-    primitive: wgpu::PrimitiveState {
-      topology: wgpu::PrimitiveTopology::PointList,
-      ..Default::default()
-    },
+    primitive: Default::default(),
  });

  ...

  app.run(RenderMode::Continuous, move |frame: &Frame| {

    ...

-    pass.draw(0..k_num_points, 0..1);
+    pass.draw(0..6, 0..k_num_points);

    ...
```

This gives us points in 3D. They even scale based on their distance from the camera.

{{{example url="../webgpu-points-3d.html"}}}

## <a id="a-fixed-size-3d-points"></a> Fixed size 3d points

What if we want the points to stay a fixed size?

Recall from [the article on perspective projection](webgpu-perspective-projection.html) that the GPU divides the position
we return from the vertex shader by W. This divide gives us perspective by making
things further way appear smaller. So, for points we don't want to change size we
just need to multiply them by that W so after they're divided they'll be the
value we really wanted.

```wgsl
    var vsOut: VSOutput;
    let pos = points[vNdx];
    let clipPos = uni.matrix * vert.position;
-    let pointPos = vec4f(pos * uni.size / uni.resolution, 0, 0);
+    let pointPos = vec4f(pos * uni.size / uni.resolution * clipPos.w, 0, 0);
    vsOut.position = clipPos + pointPos;
    return vsOut;
```

And now they stay the same size

{{{example url="../webgpu-points-3d-fixed-size.html"}}}

<div class="webgpu_bottombar">
<h3>Why doesn't WebGPU support points larger than 1x1 pixel?</h3>
<p>WebGPU is based on native GPU APIs like Vulkan, Metal, DirectX, and even OpenGL.
Unfortunately, those APIs do not agree with each other on what it means to support
rendering points. Some APIs have device dependent limits on the size of points.
Some APIs don't render a point if its center is outside of clip space while others
do. In some APIs, this second issue is up to the driver. All of that means WebGPU decided to do the portable thing and only support 1x1
sized pixels.</p>
<p>The good thing is it's easy to support larger points yourself as shown above. The solutions
above are portable across devices, they have no limit on the size of a point and
they consistently clip points across devices. They draw the portion of any point
that is inside clip space regardless of if the point's center is outside of clip space.</p>
<p>Even better, these solutions are more flexible. For example rotating points
is not a thing supported by native APIs. By implementing our own solutions
we can easily add more features making things even more flexible.</p>
</div>
