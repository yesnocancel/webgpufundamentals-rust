Title: WebGPU Uniforms
Description: Passing Constant Data to a Shader
TOC: Uniforms

The previous article was about [inter-stage variables](webgpu-inter-stage-variables.html).
This article will be about uniforms.

Uniforms are kind of like global variables for your shader. You can set their
values before you execute the shader and they'll have those values for every
iteration of the shader. You can set them to something else the next time
you ask the GPU to execute the shader.

We'll start again with the triangle example from [the first article](webgpu-fundamentals.html) and modify it to use some uniforms.

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("triangle shaders with uniforms"),
    source: wgpu::ShaderSource::Wgsl(r#"
+      struct OurStruct {
+        color: vec4f,
+        scale: vec2f,
+        offset: vec2f,
+      };
+
+      @group(0) @binding(0) var<uniform> ourStruct: OurStruct;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

-        return vec4f(pos[vertexIndex], 0.0, 1.0);
+        return vec4f(
+          pos[vertexIndex] * ourStruct.scale + ourStruct.offset, 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
-        return vec4f(1, 0, 0, 1);
+        return ourStruct.color;
      }
    "#.into()),
  });
```

First, we declared a struct with 3 members.

```wsgl
      struct OurStruct {
        color: vec4f,
        scale: vec2f,
        offset: vec2f,
      };
```

Then we declared a uniform variable with a type of that struct.
The variable is `ourStruct` and its type is `OurStruct`.

```wsgl
      @group(0) @binding(0) var<uniform> ourStruct: OurStruct;
```

Next, we changed what is returned from the vertex shader to use
the uniforms.

```wgsl
      @vertex fn vs(
         ...
      ) ... {
        ...
        return vec4f(
          pos[vertexIndex] * ourStruct.scale + ourStruct.offset, 0.0, 1.0);
      }
```

You can see we multiply the vertex position by scale and then add an offset.
This will let us set the size of a triangle and position it.

We also changed the fragment shader to return the color from our uniforms.

```wgsl
      @fragment fn fs() -> @location(0) vec4f {
        return ourStruct.color;
      }
```

Now that we've set up the shader to use uniforms, we need to create
a buffer on the GPU to hold values for them.

This is an area where if you've never dealt with native data and sizes,
there's a bunch to learn. It's a big topic so [here is a separate
article about the topic](webgpu-memory-layout.html). If you don't
know how to layout structs in memory, please [go read the article](webgpu-memory-layout.html). Then come back here. This article
will assume you [already read it](webgpu-memory-layout.html).

Having read [the article](webgpu-memory-layout.html), we can
now go ahead and fill out a buffer with data that matches the
struct in our shader.

First, we make a buffer and assign it usage flags so it can
be used with uniforms, and so that we can update by copying
data to it.

```rust
  // create a buffer for the uniform values
  const UNIFORM_BUFFER_SIZE: u64 = 4 * 4 + // color is 4 32bit floats (4bytes each)
      2 * 4 + // scale is 2 32bit floats (4bytes each)
      2 * 4; // offset is 2 32bit floats (4bytes each)
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms for triangle"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
```

Then we make an array of `f32`s so we can stage values on the CPU side, the
same job a `Float32Array` does in JavaScript.

```rust
  // create an array of f32s to hold the values for the uniforms in Rust
  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
```

and we'll fill out 2 of the values of our struct that won't be changing later.
The offsets were computed using what we covered in
[the article on memory-layout](webgpu-memory-layout.html).

```rust
  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_SCALE_OFFSET: usize = 4;
  const K_OFFSET_OFFSET: usize = 6;

  uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]); // set the color
  uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2].copy_from_slice(&[-0.5, -0.25]); // set the offset
```

Above we're setting the color to green. The offset will move the triangle
to the left 1/4th of the canvas and down 1/8th. (remember, clip space goes
from -1 to 1 which is 2 units wide so 0.25 is 1/8 of 2). 

Next, [as the diagram showed in the first article](webgpu-fundamentals.html#a-draw-diagram),
to tell a shader about our buffer we need to create a bind group and bind the buffer.
We need to pass the same `@group(?)` and `@binding(?)` we set in our shader.

```rust
  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("triangle bind group"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: uniform_buffer.as_entire_binding(),
    }],
  });
```

Now sometimes before we submit our command buffer, we need to set
the remaining values of `uniform_values` and then copy those values to the buffer on the GPU.
We'll do it at the top of our frame closure. 

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    // Set the uniform values in our Rust-side array of f32s
    let aspect = frame.width as f32 / frame.height as f32;
    uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2].copy_from_slice(&[0.5 / aspect, 0.5]); // set the scale

    // copy the values from Rust to the GPU
    frame
        .queue
        .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

> Note: `write_buffer` is one way to copy data to a buffer. 
> There are several other ways covered in [this article](webgpu-copying-data.html).

`write_buffer` wants plain bytes, so just like in
[the first article](webgpu-fundamentals.html), `bytemuck::cast_slice`
reinterprets our `f32`s as bytes without copying.

We're setting the scale to half size AND taking into account the aspect of the canvas
so the triangle will keep the same width-to-height ratio regardless
of the size of the canvas. 

Finally, we need to set the bind group before drawing.

```rust
    pass.set_pipeline(&pipeline);
+    pass.set_bind_group(0, &bind_group, &[]);
    pass.draw(0..3, 0..1);  // call our vertex shader 3 times
```

The extra `&[]` argument to `set_bind_group` is a list of *dynamic offsets*.
We're not using any so it's empty.

And with that, we get a green triangle as described.

{{{example url="../webgpu-simple-triangle-uniforms.html"}}}

For this single triangle, our state when the draw command is
executed is something like this.

<div class="webgpu_center"><img src="resources/webgpu-draw-diagram-triangle-uniform.svg" style="width: 863px;"></div>

Up until now, all of the data we've used in our shaders was either
hardcoded (the triangle vertex positions in the vertex shader, 
and the color in the fragment shader).
Now that we're able to pass values into our shader, we can call `draw`
multiple times with different data.

We could draw in different places with different offsets, scales,
and colors by updating our single buffer. It's important to remember
though that our commands get put in a command buffer, they are not
actually executed until we submit them. So, we **can NOT** do this

```rust
    // BAD!
    let mut x = -1.0;
    while x < 1.0 {
      uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2].copy_from_slice(&[x, x]);
      frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
      pass.draw(0..3, 0..1);
      x += 0.1;
    }
    drop(pass);

    // Finish encoding and submit the commands
    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
```

Because, as you can see above, the `queue.xxx` functions happen on
a "queue" but the `pass.xxx` functions just encode a command in the command buffer.\
When we actually call `submit` with our command buffer,
the only thing in our buffer would be the last values we wrote.

We could change it to this. 

```rust
    // BAD! Slow!
    let mut x = -1.0;
    while x < 1.0 {
      uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2].copy_from_slice(&[x, 0.0]);
      frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));

      let mut encoder = frame.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
      {
        let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
      }

      // Finish encoding and submit the commands
      let command_buffer = encoder.finish();
      frame.queue.submit([command_buffer]);
      x += 0.1;
    }
```

The code above updates one buffer, creates one command buffer,
adds commands to draw one thing, then finishes the command buffer
and submits it. This works but is slow for multiple reasons. The biggest is that it's
best practice to do more work in a single command buffer.

So instead, we could create one uniform buffer per thing we want
to draw. And, since buffers are used indirectly through bind groups,
we'll also need one bind group per thing we want to draw. Then we
can put all the things we want to draw into a single command buffer.

Let's do it.

First, let's make a random function. JavaScript has `Math.random` built in;
Rust's standard library doesn't ship a random number generator, so we'll write
a tiny [xorshift](https://en.wikipedia.org/wiki/Xorshift) one. (We could pull
in a crate for this, but a fixed-seed helper is all we need, and it keeps the
output reproducible.)

```rust
// A random number between [min and max)
fn rand(min: f32, max: f32) -> f32 {
    use std::cell::Cell;
    thread_local!(static STATE: Cell<u32> = const { Cell::new(0x13579bdf) });
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        s.set(x);
        min + (max - min) * (x as f32 / u32::MAX as f32)
    })
}
```

And now, let's set up buffers with a bunch of colors and offsets
we can draw a bunch of individual things. In JavaScript we'd push anonymous
objects into an array; in Rust we declare a struct for the per-object data
and push instances of it into a `Vec`.

```rust
  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_SCALE_OFFSET: usize = 4;
  const K_OFFSET_OFFSET: usize = 6;

+  struct ObjectInfo {
+    scale: f32,
+    uniform_buffer: wgpu::Buffer,
+    uniform_values: [f32; UNIFORM_BUFFER_SIZE as usize / 4],
+    bind_group: wgpu::BindGroup,
+  }
+
+  const K_NUM_OBJECTS: usize = 100;
+  let mut object_infos: Vec<ObjectInfo> = Vec::new();
+
+  for i in 0..K_NUM_OBJECTS {
+    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+      label: Some(&format!("uniforms for obj: {i}")),
+      size: UNIFORM_BUFFER_SIZE,
+      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      mapped_at_creation: false,
+    });
+
+    // create an array of f32s to hold the values for the uniforms in Rust
+    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
-  uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]); // set the color
-  uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2].copy_from_slice(&[-0.5, -0.25]); // set the offset
+    uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
+      rand(0.0, 1.0),
+      rand(0.0, 1.0),
+      rand(0.0, 1.0),
+      1.0,
+    ]); // set the color
+    uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2]
+      .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset
+
+    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
+      label: Some(&format!("bind group for obj: {i}")),
+      layout: &pipeline.get_bind_group_layout(0),
+      entries: &[wgpu::BindGroupEntry {
+        binding: 0,
+        resource: uniform_buffer.as_entire_binding(),
+      }],
+    });
+
+    object_infos.push(ObjectInfo {
+      scale: rand(0.2, 0.5),
+      uniform_buffer,
+      uniform_values,
+      bind_group,
+    });
+  }
```

We're not setting the values in our buffer yet because we want it to take into account
the aspect of the canvas and we won't know the aspect of the canvas until
render time.

At render time, we'll update all of the buffers with the correct aspect-adjusted
scale.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
-    // Set the uniform values in our Rust-side array of f32s
-    let aspect = frame.width as f32 / frame.height as f32;
-    uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2].copy_from_slice(&[0.5 / aspect, 0.5]); // set the scale
-
-    // copy the values from Rust to the GPU
-    frame
-        .queue
-        .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));

    let mut encoder = frame
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        ...
      });
      pass.set_pipeline(&pipeline);

+      // Set the uniform values in our Rust-side array of f32s
+      let aspect = frame.width as f32 / frame.height as f32;

+      for object_info in object_infos.iter_mut() {
+        let scale = object_info.scale;
+        object_info.uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
+          .copy_from_slice(&[scale / aspect, scale]); // set the scale
+        frame.queue.write_buffer(
+          &object_info.uniform_buffer,
+          0,
+          bytemuck::cast_slice(&object_info.uniform_values),
+        );

        pass.set_bind_group(0, &object_info.bind_group, &[]);
        pass.draw(0..3, 0..1);  // call our vertex shader 3 times
+      }
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

Again, remember that the `encoder` and `pass` objects are just encoding commands
into a command buffer. So when the frame closure exits we've effectively
issued these *commands* in this order.

```rust
queue.write_buffer(...) // update uniform buffer 0 with data for object 0
queue.write_buffer(...) // update uniform buffer 1 with data for object 1
queue.write_buffer(...) // update uniform buffer 2 with data for object 2
queue.write_buffer(...) // update uniform buffer 3 with data for object 3
...
// execute commands that draw 100 things, each with its own uniform buffer.
queue.submit([command_buffer]);
```

Here's that

{{{example url="../webgpu-simple-triangle-uniforms-multiple.html"}}}

While we're here, one more thing to cover. You're free to reference multiple
uniform buffers in your shaders. In our example above, every time we draw
we update the scale, then we `write_buffer` to upload `uniform_values` for that
object to the corresponding uniform buffer. But, only the scale is being updated,
color and offset are not, so we're wasting time uploading color and offset.

We could split the uniforms into uniforms that need to be set once and uniforms
that are updated each time we draw.

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    source: wgpu::ShaderSource::Wgsl(r#"
      struct OurStruct {
        color: vec4f,
-        scale: vec2f,
        offset: vec2f,
      };

+      struct OtherStruct {
+        scale: vec2f,
+      };

      @group(0) @binding(0) var<uniform> ourStruct: OurStruct;
+      @group(0) @binding(1) var<uniform> otherStruct: OtherStruct;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(
-          pos[vertexIndex] * ourStruct.scale + ourStruct.offset, 0.0, 1.0);
+          pos[vertexIndex] * otherStruct.scale + ourStruct.offset, 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return ourStruct.color;
      }
    "#.into()),
  });
```

When we need 2 uniform buffers per thing we want to draw

```rust
-  // create a buffer for the uniform values
-  const UNIFORM_BUFFER_SIZE: u64 = 4 * 4 + // color is 4 32bit floats (4bytes each)
-      2 * 4 + // scale is 2 32bit floats (4bytes each)
-      2 * 4; // offset is 2 32bit floats (4bytes each)
-  // offsets to the various uniform values in float32 indices
-  const K_COLOR_OFFSET: usize = 0;
-  const K_SCALE_OFFSET: usize = 4;
-  const K_OFFSET_OFFSET: usize = 6;
+  // create 2 buffers for the uniform values
+  const STATIC_UNIFORM_BUFFER_SIZE: u64 = 4 * 4 + // color is 4 32bit floats (4bytes each)
+      2 * 4 + // offset is 2 32bit floats (4bytes each)
+      2 * 4; // padding
+  const UNIFORM_BUFFER_SIZE: u64 = 2 * 4; // scale is 2 32bit floats (4bytes each)
+
+  // offsets to the various uniform values in float32 indices
+  const K_COLOR_OFFSET: usize = 0;
+  const K_OFFSET_OFFSET: usize = 4;
+
+  const K_SCALE_OFFSET: usize = 0;

  const K_NUM_OBJECTS: usize = 100;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  for i in 0..K_NUM_OBJECTS {
+    let static_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+      label: Some(&format!("static uniforms for obj: {i}")),
+      size: STATIC_UNIFORM_BUFFER_SIZE,
+      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      mapped_at_creation: false,
+    });
+
+    // These are only set once so set them now
+    {
-      let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
+      let mut uniform_values = [0.0f32; STATIC_UNIFORM_BUFFER_SIZE as usize / 4];
      uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
        rand(0.0, 1.0),
        rand(0.0, 1.0),
        rand(0.0, 1.0),
        1.0,
      ]); // set the color
      uniform_values[K_OFFSET_OFFSET..K_OFFSET_OFFSET + 2]
        .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

+      // copy these values to the GPU
+      app.queue.write_buffer(
+        &static_uniform_buffer,
+        0,
+        bytemuck::cast_slice(&uniform_values),
+      );
    }

+    // create an array of f32s to hold the values for the uniforms in Rust
+    let uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
+    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+      label: Some(&format!("changing uniforms for obj: {i}")),
+      size: UNIFORM_BUFFER_SIZE,
+      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      mapped_at_creation: false,
+    });

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some(&format!("bind group for obj: {i}")),
      layout: &pipeline.get_bind_group_layout(0),
-      entries: &[wgpu::BindGroupEntry {
-        binding: 0,
-        resource: uniform_buffer.as_entire_binding(),
-      }],
+      entries: &[
+        wgpu::BindGroupEntry {
+          binding: 0,
+          resource: static_uniform_buffer.as_entire_binding(),
+        },
+        wgpu::BindGroupEntry {
+          binding: 1,
+          resource: uniform_buffer.as_entire_binding(),
+        },
+      ],
    });

    object_infos.push(ObjectInfo {
      scale: rand(0.2, 0.5),
      uniform_buffer,
      uniform_values,
      bind_group,
    });
  }
```

Nothing changes in our render code. The bind group for each object contains
a reference to both uniform buffers for each object. Just as before we are
updating the scale. But now we're only uploading the scale when we call
`write_buffer` to update the uniform buffer that holds the scale value
whereas before we were uploading the color + offset + scale for each object.

{{{example url="../webgpu-simple-triangle-uniforms-split.html"}}}

While in this simple example, splitting into multiple uniform buffers was probably
overkill, it's common to split based on what changes and when. Examples might include
one uniform buffer for matrices that are shared. For example [a projection matrix, a view
matrix, and a camera matrix](webgpu-cameras.html). Since often these are the same for all things we want to draw
we can just make one buffer and have all objects use the same uniform buffer.

Separately our shader might reference another uniform buffer that contains just the
things that are specific to this object like its [world / model matrix](webgpu-cameras.html) and its [normal matrix](webgpu-lighting-directional.html).

Another uniform buffer might contain material settings. Those settings might be shared
by multiple objects.

We'll do much of this when we cover drawing 3D.

Next up, [storage buffers](webgpu-storage-buffers.html)
