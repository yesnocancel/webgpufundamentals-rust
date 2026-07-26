Title: WebGPU Scale
Description: Scaling an Object
TOC: Scale

This article is the 3nd in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

1. [Translation](webgpu-translation.html)
2. [Rotation](webgpu-rotation.html)
3. [Scaling](webgpu-scale.html) ⬅ you are here
4. [Matrix Math](webgpu-matrix-math.html)
5. [Orthographic Projection](webgpu-orthographic-projection.html)
6. [Perspective Projection](webgpu-perspective-projection.html)
7. [Cameras](webgpu-cameras.html)
8. [Matrix Stacks](webgpu-matrix-stacks.html)
9. [Scene Graphs](webgpu-scene-graphs.html)

Scaling is just as [easy as translation](webgpu-translation.html).

We multiply the vertex positions by our desired scale. Here are the changes
to the shader from our [previous example](webgpu-rotation.html).

```wgsl
struct Uniforms {
  color: vec4f,
  resolution: vec2f,
  translation: vec2f,
  rotation: vec2f,
  scale: vec2f,
};

struct Vertex {
  @location(0) position: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;

+  // Scale the position
+  let scaledPosition = vert.position * uni.scale;

  // Rotate the position
  let rotatedPosition = vec2f(
-    vert.position.x * uni.rotation.y - vert.position.y * uni.rotation.x,
-    vert.position.x * uni.rotation.x + vert.position.y * uni.rotation.y
+    scaledPosition.x * uni.rotation.y - scaledPosition.y * uni.rotation.x,
+    scaledPosition.x * uni.rotation.x + scaledPosition.y * uni.rotation.y
  );

  // Add in the translation
  let position = rotatedPosition + uni.translation;

  // convert the position from pixels to a 0.0 to 1.0 value
  let zeroToOne = position / uni.resolution;

  // convert from 0 <-> 1 to 0 <-> 2
  let zeroToTwo = zeroToOne * 2.0;

  // covert from 0 <-> 2 to -1 <-> +1 (clip space)
  let flippedClipSpace = zeroToTwo - 1.0;

  // flip Y
  let clipSpace = flippedClipSpace * vec2f(1, -1);

  vsOut.position = vec4f(clipSpace, 0.0, 1.0);
  return vsOut;
}
```

And, like before, we need to update our uniform buffer to have room for
the scale value.

```rust
-  // color, resolution, translation, rotation, padding
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 2) * 4 + 8;
+  // color, resolution, translation, rotation, scale
+  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 2 + 2) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_RESOLUTION_OFFSET: usize = 4;
  const K_TRANSLATION_OFFSET: usize = 6;
  const K_ROTATION_OFFSET: usize = 8;
+  const K_SCALE_OFFSET: usize = 10;
```

and at render time we need to update the scale. In the page we add scale
sliders,

```js
  const settings = {
    translation: [150, 100],
    rotation: degToRad(30),
+    scale: [1, 1],
  };

  const gui = new GUI();
  ...
+  gui.add(settings.scale, '0', -5, 5).name('scale.x')
+     .onChange(v => wasm.set_setting_num('scaleX', v));
+  gui.add(settings.scale, '1', -5, 5).name('scale.y')
+     .onChange(v => wasm.set_setting_num('scaleY', v));
```

and in the Rust we read and set them.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...
    uniform_values[K_ROTATION_OFFSET..K_ROTATION_OFFSET + 2]
        .copy_from_slice(&rotation);
+    let scale = [
+        wgpu_fun::setting_f64("scaleX", 1.0) as f32,
+        wgpu_fun::setting_f64("scaleY", 1.0) as f32,
+    ];
+    uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
+        .copy_from_slice(&scale);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

And now we have scale. Drag the sliders.

{{{example url="../webgpu-scale.html" }}}

One thing to notice is that scaling by a negative value flips our geometry.

Another thing to notice is it scales from 0, 0 which for our F is the
top left corner. That makes sense since we're multiplying the positions
by the scale they will move away from 0, 0. You can probably
imagine ways to fix that. For example you could add another translation
before you scale, a *pre scale* translation. Another solution would be
to change the actual F position data. We'll go over another way soon.

I hope these last 3 posts were helpful in understanding
[translation](webgpu-translation.html), [rotation](webgpu-rotation.html)
and scale. Next we'll go over [the magic that is matrices](webgpu-matrix-math.html)
that combines all 3 of these into a **much simpler** and often more useful form.
