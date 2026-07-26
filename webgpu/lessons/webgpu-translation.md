Title: WebGPU Translation
Description: Moving an object
TOC: Translation

This article assumes you've read [the article on fundamentals](webgpu-fundamentals.html),
[the article uniforms](webgpu-uniforms.html) and 
[the article on vertex-buffers](webgpu-vertex-buffers.html).
If you have not read them I suggest you read them first, then come back.

This article is the first of series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

1. [Translation](webgpu-translation.html)  ⬅ you are here
2. [Rotation](webgpu-rotation.html)
3. [Scaling](webgpu-scale.html)
4. [Matrix Math](webgpu-matrix-math.html)
5. [Orthographic Projection](webgpu-orthographic-projection.html)
6. [Perspective Projection](webgpu-perspective-projection.html)
7. [Cameras](webgpu-cameras.html)
8. [Matrix Stacks](webgpu-matrix-stacks.html)
9. [Scene Graphs](webgpu-scene-graphs.html)

We are going to start code similar to the examples from [the article on vertex-buffers](webgpu-vertex-buffers.html)
but instead of a bunch of circles we're going to draw a single F and we'll use an [index buffer](webgpu-vertex-buffers.html#a-index-buffers) to keep the data
smaller.

Let's work in pixel space instead of clip space, just like the [Canvas 2D API](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D)
We'll make an F and we'll build it from 6 triangles like this

<div class="webgpu_center"><img src="resources/f-polygons.svg" style="width: 600px;"></div>

Here's the data for the F

```rust
#[rustfmt::skip]
fn create_f_vertices() -> (Vec<f32>, Vec<u32>, u32) {
    let vertex_data: Vec<f32> = vec![
        // left column
        0.0, 0.0,
        30.0, 0.0,
        0.0, 150.0,
        30.0, 150.0,

        // top rung
        30.0, 0.0,
        100.0, 0.0,
        30.0, 30.0,
        100.0, 30.0,

        // middle rung
        30.0, 60.0,
        70.0, 60.0,
        30.0, 90.0,
        70.0, 90.0,
    ];

    let index_data: Vec<u32> = vec![
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top run
        8,  9, 10,   10,  9, 11,  // middle run
    ];

    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}
```

The vertex data above is in pixel space so we need to translate that to clip space.
We can do that by passing the resolution into the shader and doing some math.
Here it is spelled out one step at a time.

```wgsl
struct Uniforms {
  color: vec4f,
  resolution: vec2f,
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
  
  let position = vert.position;

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

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return uni.color;
}
```

You can see we take a vertex position and divide it by the resolution. 
This gives us a value that goes from 0 to 1 across the canvas.
We then multiply by 2 to get a value that goes from 0 to 2 across the canvas.
We subtract 1. Now our value is in clip space but it's flipped because
the clip space goes positive Y up where as canvas 2d goes positive Y down.
So we multiply Y by -1 to flip it. Now we have our needed clip space value
which we can output from the shader.

We've only got one attribute so our pipeline looks like this

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("just 2d position"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
*        array_stride: (2) * 4, // (2) floats, 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
*        attributes: &[
*          // position
*          wgpu::VertexAttribute {
*            shader_location: 0,
*            offset: 0,
*            format: wgpu::VertexFormat::Float32x2,
*          },
*        ],
      })],
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

We need to setup a buffer for our uniforms

```rust
  // color, resolution, padding
*  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2) * 4 + 8;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
*  const K_COLOR_OFFSET: usize = 0;
*  const K_RESOLUTION_OFFSET: usize = 4;
*
*  // The color will not change so let's set it once at init time
*  uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
*    rand(0.0, 1.0),
*    rand(0.0, 1.0),
*    rand(0.0, 1.0),
*    1.0,
*  ]);
```

(In JavaScript you'd make TypedArray *views* into one buffer; in Rust we just
index one `[f32]` array with the offset constants.)

At render time we need to set the resolution

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

    // Set the uniform values in our Rust side array
    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
        .copy_from_slice(&[frame.width as f32, frame.height as f32]);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

Before we run it lets make the background of the canvas look like
graph paper. We'll set it's scale so each grid cell of the graph
paper is 10x10 pixels and every 100x100 pixels we'll draw a bolder
line.

```css
:root {
  --bg-color: #fff;
  --line-color-1: #AAA;
  --line-color-2: #DDD;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg-color: #000;
    --line-color-1: #666;
    --line-color-2: #333;
  }
}
canvas {
  display: block;  /* make the canvas act like a block   */
  width: 100%;     /* make the canvas fill its container */
  height: 100%;
  background-color: var(--bg-color);
  background-image: linear-gradient(var(--line-color-1) 1.5px, transparent 1.5px),
      linear-gradient(90deg, var(--line-color-1) 1.5px, transparent 1.5px),
      linear-gradient(var(--line-color-2) 1px, transparent 1px),
      linear-gradient(90deg, var(--line-color-2) 1px, transparent 1px);
  background-position: -1.5px -1.5px, -1.5px -1.5px, -1px -1px, -1px -1px;
  background-size: 100px 100px, 100px 100px, 10px 10px, 10px 10px;  
}
```

The CSS above should handle both light and dark cases.

All our examples to this point have used an opaque canvas. To make it transparent,
so we can see the background we just setup, we need to make a few changes.

First we need to set the alpha mode of the canvas to premultiplied.
It defaults to opaque.

```rust
  let mut app = App::new("WebGPU Translation").await;
  app.auto_resize = true;
+  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
```

Then we need to clear the canvas to 0, 0, 0, 0 in our render pass descriptor,
so the page shows through.

```rust
      ops: wgpu::Operations {
-        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
+        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
        store: wgpu::StoreOp::Store,
      },
```

And with that, here's our F

{{{example url="../webgpu-translation-prep.html"}}}

Notice the F's size relative to the grid behind it.
The vertex positions of the F data make an F that is 100 pixels
wide and 150 pixels tall and that matches what we displayed.
The F starts at 0,0 and extends right to 100,0 and down to 0,150

Now that we have the basics in place, let's add *translation*.

Translation is just the process of moving things so all we need
to do is add translation to our uniforms and add that to our
position

```wgsl
struct Uniforms {
  color: vec4f,
  resolution: vec2f,
+  translation: vec2f,
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
  
+  // Add in the translation
-  let position = vert.position;
+  let position = vert.position + uni.translation;

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

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return uni.color;
}
```

We need to add room to our uniform buffer

```rust
-  // color, resolution, padding
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2) * 4 + 8;
+  // color, resolution, translation
+  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2) * 4;
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
+  const K_TRANSLATION_OFFSET: usize = 6;
```

And then we need to set a translation at render time

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

    // Set the uniform values in our Rust side array
    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
+    let translation = [
+        wgpu_fun::setting_f64("translationX", 0.0) as f32,
+        wgpu_fun::setting_f64("translationY", 0.0) as f32,
+    ];
+    uniform_values[K_TRANSLATION_OFFSET..K_TRANSLATION_OFFSET + 2]
+        .copy_from_slice(&translation);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

Finally let's add a UI so we can adjust the translation. The settings panel
stays in the example's page JavaScript (like the GUI examples in
[the article on textures](webgpu-textures.html)); its onChange handlers push
values into the wasm module, which is where `wgpu_fun::setting_f64` above
reads them from.

```js
+import GUI from '../3rdparty/muigui-0.x.module.js';

...
  const settings = {
    translation: [0, 0],
  };

+  const gui = new GUI();
+  gui.add(settings.translation, '0', 0, 1000).name('translation.x')
+     .onChange(v => wasm.set_setting_num('translationX', v));
+  gui.add(settings.translation, '1', 0, 1000).name('translation.y')
+     .onChange(v => wasm.set_setting_num('translationY', v));
```

And now we've added translation

{{{example url="../webgpu-translation.html"}}}

Notice it matches our pixel grid. If we set the translation to 200,300 the F
is drawn with its 0,0 top left vertex at 200,300.

This article might have seemed exceedingly simple. We were already using *translation*
in several examples already though we named it 'offset'.
This article is part of series. Though it was simple, hopefully its point will make
sense in context as we continue the series.

Next up is [rotation](webgpu-rotation.html).
