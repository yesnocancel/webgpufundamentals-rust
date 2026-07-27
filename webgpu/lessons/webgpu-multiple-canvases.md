Title: WebGPU Multiple Canvases
Description: Multiple Canvases
TOC: Multiple Canvases

Drawing to multiple canvases in WebGPU is super easy.
In [the article on fundamentals](webgpu-fundamentals.html)
our helper's `App` looked up the page's canvas, got a context,
and configured it. In JavaScript that looks like

```js
  // Get a WebGPU context from the canvas and configure it
  const canvas = document.querySelector('canvas');
  const context = canvas.getContext('webgpu');
  const presentationFormat = navigator.gpu.getPreferredCanvasFormat();
  context.configure({
    device,
    format: presentationFormat,
  });
```

and in our Rust examples it's hidden inside `wgpu_fun::App::new`, which
wraps the page's single `<canvas>` (or the window, natively) in a wgpu
*surface*.

To draw to the canvas we got a texture for the canvas from that surface
and set that texture as the first color attachment of a render pass

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    // make a command encoder to start encoding commands
    let mut encoder = frame
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("our encoder"),
      });

    // make a render pass encoder to encode render specific commands
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
*          view: frame.view,  // <- the canvas's current texture
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });
```

All we have to do to draw to a different canvas is follow the same steps for
that canvas.

1. Lookup the canvas (or create one)
2. Create a surface for it (JS: get a "webgpu" context)
3. Configure it
4. When we want to render to that canvas, get its current texture
   and use that texture as a color attachment in a render pass

`App` only handles one canvas, so for this lesson `wgpu_fun` has a second
entry point, `MultiApp`. It creates the device and queue exactly like
`App`, but leaves the canvases to you: `app.canvases(...)` hands back one
`Canvas` per canvas, and each `Canvas` does steps 1 to 3 and exposes step
4 as `canvas.current_view()` — the equivalent of the JS
`context.getCurrentTexture().createView()`.

Let's take our very first example and render to 3 canvases

First let's add 2 more canvases

```html
  <body>
    <canvas></canvas>
+    <canvas></canvas>
+    <canvas></canvas>
  </body>
```

Next let's get a surface for each canvas

```rust
-  let app = App::new("WebGPU Multiple Canvases").await;
+  let app = MultiApp::new("WebGPU Multiple Canvases").await;
+
+  // Get a canvas surface for each canvas and configure it
+  // (browser: the page's three <canvas> elements; native: three
+  // offscreen 300x150 canvases shown as a grid in one window)
+  let infos = app.canvases(&[(300, 150); 3]);
```

In the browser, `app.canvases` wraps every `<canvas>` element on the
page, in document order, and configures a surface for each with the
preferred canvas format (`app.format`). There is no such thing as one
native window with several surfaces, so natively the argument gives the
pixel sizes of "canvases" to create — offscreen textures the helper
composites into the window as a grid you can scroll with the mouse
wheel. 300x150 is the default size of an HTML `<canvas>`.

And finally let's render to all of them

```rust
  app.run(RenderMode::Once, move |frame: &MultiFrame| {
*    // make a command encoder to start encoding commands
*    let mut encoder = frame
*      .device
*      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
*        label: Some("our encoder"),
*      });

+    for canvas in &infos {
      // Get the current texture from the canvas context and
      // set it as the texture to render to.
+      let view = canvas.current_view();

      // make a render pass encoder to encode render specific commands
      {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: Some("our basic canvas renderPass"),
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
*            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
              store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
          })],
          ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);  // call our vertex shader 3 times.
      }
+    }

*    let command_buffer = encoder.finish();
*    frame.queue.submit([command_buffer]);
  });
```

Changes we made are (1) where we create our command encoder so it
can be shared to render all 3 canvases. (2) looping over the
canvases. Note that each frame's `MultiFrame` no longer has a single
`view` — the views come from the canvases.

And with that we've rendered to 3 canvases

{{{example url="../webgpu-multiple-canvases.html" }}}

Note: It's not strictly necessary to make a single command encoder but it
is slightly more efficient.

So what else is left?

## Optimizing Lots of Canvases

Let's say we wanted to show spinning products. To keep this simple
let's stick with our hard coded triangle but let's make it spin
by passing in a matrix [like we covered in the articles on matrix math](webgpu-matrix-math.html).
and let's also pass in a color so we can make each one appear slightly
different.

```wgsl
+  struct Uniforms {
+    matrix: mat4x4f,
+    color: vec4f,
+  };
+
+  @group(0) @binding(0) var<uniform> uni: Uniforms;

  @vertex fn vs(
    @builtin(vertex_index) vertexIndex : u32
  ) -> @builtin(position) vec4f {
    let pos = array(
      vec2f( 0.0,  0.5),  // top center
      vec2f(-0.5, -0.5),  // bottom left
      vec2f( 0.5, -0.5)   // bottom right
    );

-    return vec4f(pos[vertexIndex], 0.0, 1.0);
+    return uni.matrix * vec4f(pos[vertexIndex], 0.0, 1.0);
  }

  @fragment fn fs() -> @location(0) vec4f {
-    return vec4f(1, 0, 0, 1);
+    return uni.color;
  }
```


We'll need a [uniform buffer](webgpu-uniforms.html) for each as well
as a bind group and related things

Let's make 200 canvases. Making 200 product cards is plain DOM work so it
stays in the page's JavaScript, before the wasm module starts

```js
  // Make the 200 product cards (plain DOM work, so it stays in page JS —
  // the wasm module then finds and renders every <canvas>).
  const numProducts = 200;
  for (let i = 0; i < numProducts; ++i) {
    // making this
    // <div class="product size?">
    //   <canvas></canvas>
    //   <div>Product#: ?</div>
    // </div>
    const canvas = document.createElement('canvas');

    const container = document.createElement('div');
    container.className = `product size${i % 4}`;

    const description = document.createElement('div');
    description.textContent = `product#: ${i + 1}`;

    container.appendChild(canvas);
    container.appendChild(description);
    document.body.appendChild(container);
  }
```

We need some CSS to go along with this

```css
  .product {
    display: inline-block;
    padding: 1em;
    background: #888;
    margin: 1em;
  }
  .size0>canvas {
    width: 200px;
    height: 200px;
  }
  .size1>canvas {
    width: 250px;
    height: 200px;
  }
  .size2>canvas {
    width: 300px;
    height: 200px;
  }
  .size3>canvas {
    width: 100px;
    height: 200px;
  }
```

The 4 sizes are just to make sure we're doing things correctly. If we
made them all the same size we might hide a mistake.

On the Rust side we wrap all 200 canvases. Natively the same four sizes
become the pixel sizes of the offscreen canvases

```rust
+  // One canvas per product card. On the page the cards (and their CSS
+  // sizes) are made by the page's JS; natively these are the four
+  // .size0-.size3 CSS sizes from the original.
+  const NUM_PRODUCTS: usize = 200;
+  let sizes: Vec<(u32, u32)> = (0..NUM_PRODUCTS)
+    .map(|i| [(200, 200), (250, 200), (300, 200), (100, 200)][i % 4])
+    .collect();
+
+  let canvases = app.canvases(&sizes);
```

We need a uniform buffer and bind group for each one. We won't change
the color later so we'll pick one now. Let's pick a rand clearValue as well (why not? 🤷‍♂️)

```rust
+  fn random_color() -> [f32; 4] {
+    [rand(0.0, 1.0), rand(0.0, 1.0), rand(0.0, 1.0), 1.0]
+  }

+  // Everything we need per canvas: the JS `infos` array entries.
+  struct Info {
+    canvas: Canvas,
+    clear_value: [f32; 4],
+    uniform_values: [f32; 16 + 4],
+    uniform_buffer: wgpu::Buffer,
+    bind_group: wgpu::BindGroup,
+  }

  let mut infos = Vec::new();
  for canvas in app.canvases(&sizes) {
+    // Make a uniform buffer and values for our uniforms.
+    let mut uniform_values = [0.0f32; 16 + 4];
+    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+      label: None,
+      size: (uniform_values.len() * 4) as u64,
+      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+      mapped_at_creation: false,
+    });
+    const K_COLOR_OFFSET: usize = 16;
+    uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&random_color());
+
+    // Make a bind group for this uniform
+    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
+      label: None,
+      layout: &pipeline.get_bind_group_layout(0),
+      entries: &[wgpu::BindGroupEntry {
+        binding: 0,
+        resource: uniform_buffer.as_entire_binding(),
+      }],
+    });

    infos.push(Info {
      canvas,
+      clear_value: random_color(),
+      uniform_values,
+      uniform_buffer,
+      bind_group,
    });
  }
```

(`rand` is the little deterministic helper we've used since
[the article on fundamentals](webgpu-fundamentals.html); JS just uses
`Math.random`.)

The original JavaScript also adds a `ResizeObserver` to
[resize each canvas](webgpu-fundamentals.html#a-resizing) so its drawing
buffer matches its displayed size

```js
  const resizeObserver = new ResizeObserver(entries => {
    for (const entry of entries) {
      const canvas = entry.target;
      const width = entry.contentBoxSize[0].inlineSize;
      const height = entry.contentBoxSize[0].blockSize;
      canvas.width = Math.max(1, Math.min(width, device.limits.maxTextureDimension2D));
      canvas.height = Math.max(1, Math.min(height, device.limits.maxTextureDimension2D));
    }
  });
```

`MultiApp` has that code built in — we just have to turn it on before
wrapping the canvases

```rust
  let mut app = MultiApp::new("WebGPU Multiple Canvases - 200").await;
+  // Each canvas's drawing buffer follows its displayed size, like the
+  // ResizeObserver in the original.
+  app.auto_resize = true;
```

At render time, we'll use `RenderMode::Continuous` (a
requestAnimationFrame loop in the browser) to animate.

```rust
-  app.run(RenderMode::Once, move |frame: &MultiFrame| {
+  app.run(RenderMode::Continuous, move |frame: &MultiFrame| {
```

And, we need to update the matrix for each canvas, upload the new values
to the uniform buffer, and set the bind group.

```rust
  app.run(RenderMode::Continuous, move |frame: &MultiFrame| {
+    let time = frame.time;

    // make a command encoder to start encoding commands
    let mut encoder = frame
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("our encoder"),
      });

    for Info {
      canvas,
      clear_value,
      uniform_values,
      uniform_buffer,
      bind_group,
    } in infos.iter_mut()
    {
      // Get the current texture from the canvas context and
      // set it as the texture to render to.
      let view = canvas.current_view();

+      let aspect = canvas.width() as f32 / canvas.height() as f32;
+      let matrix = glam::camera::rh::proj::directx::orthographic(-aspect, aspect, -1.0, 1.0, -1.0, 1.0)
+        * Mat4::from_rotation_z(time as f32 * 0.1);
+      uniform_values[0..16].copy_from_slice(&matrix.to_cols_array());
+
+      // Upload our uniform values.
+      frame
+        .queue
+        .write_buffer(uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

      // make a render pass encoder to encode render specific commands
      {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: Some("our basic canvas renderPass"),
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
-              load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
+              load: wgpu::LoadOp::Clear(wgpu::Color {
+                r: clear_value[0] as f64,
+                g: clear_value[1] as f64,
+                b: clear_value[2] as f64,
+                a: clear_value[3] as f64,
+              }),
              store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
          })],
          ..Default::default()
        });
        pass.set_pipeline(&pipeline);
+        pass.set_bind_group(0, &*bind_group, &[]);
        pass.draw(0..3, 0..1);  // call our vertex shader 3 times.
      }
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

We also give each product a random starting rotation so they don't all
spin in lock step

```rust
  infos.push(Info {
    canvas,
    clear_value: random_color(),
    uniform_values,
    uniform_buffer,
    bind_group,
+    rotation: rand(0.0, std::f32::consts::PI * 2.0),
  });
```

```rust
-      * Mat4::from_rotation_z(time as f32 * 0.1);
+      * Mat4::from_rotation_z(time as f32 * 0.1 + *rotation);
```

Let's add a few more things. We'll get to why below.

Let's add a way to stop and start the entire thing. First
we'll add a button

```html
  <body>
+    <button type="button" id="stop">Stop/Start</button>
  </body>
```

And some CSS for it.

```css
  #stop {
    position: fixed;
    right: 0;
    top: 0;
    margin: 0.5em;
    z-index: 1;
  }
```

The button lives on the page, so the click handler is page JavaScript.
Where the original cancels and restarts requestAnimationFrame, our page
flips a setting the Rust code reads (the same mechanism the GUI examples
use)

```js
  let running = true;
  document.querySelector('#stop').addEventListener('click', () => {
    running = !running;
    wasm.set_setting_bool('running', running);
  });
```

```rust
  app.run(RenderMode::Continuous, move |frame: &MultiFrame| {
+    // The page's Stop/Start button toggles this setting (instead of
+    // cancelling requestAnimationFrame like the JS version).
+    if !wgpu_fun::setting_bool("running", true) {
+      return;
+    }
```

This would work but, all the objects would jump after we pause
and then later unpause. That's because `frame.time` is the time since
the example started, and it keeps advancing while we're paused, even
though it's used to compute our rotation.

So, let's fix that by keeping our own time that only advances
when we're animating.

```rust
+  // Our own time that only advances while we're animating, so nothing
+  // jumps when the Stop/Start button pauses us.
+  let mut time = 0.0;
+  let mut then = 0.0;
  app.run(RenderMode::Continuous, move |frame: &MultiFrame| {
-    let time = frame.time;
+    let now = frame.time;
+    let delta_time = now - then;
+    then = now;
    if !wgpu_fun::setting_bool("running", true) {
      return;
    }
+    time += delta_time;

  ...
```

Note that we update `then` *before* checking whether we're paused: that
way, the first frame after unpausing sees a tiny `delta_time` instead of
the whole time we spent paused — the same thing the original achieves by
resetting `then` when the animation restarts.

And now we have 200 canvases.

{{{example url="../webgpu-multiple-canvases-x200.html"}}}

You might notice this example is HEAVY! The problem is, we're rendering
all 200 canvases even though only a few are visible. It would be
much much worse if we were drawing detailed product models instead
of just a single triangle per canvas. This is why we added the stop/start
button. This page might be too heavy if the example is running so you
might want to stop it now, before continuing.

> Note: This site tries to make the examples only render and animate if the example
> itself is visible.

One way we can potentially solve this problem is by using `IntersectionObserver`.

## <a id="a-intersection-observer"></a> Using `IntersectionObserver`

`IntersectionObserver` was designed specifically for this kind of
situation. An `IntersectionObserver` does what it says, it observes
intersections. By default it observes the intersection of an element
with the browser window. Using this, we can keep track of which
canvases are actually visible and only render those canvases.

In JavaScript you create one much like a `ResizeObserver`: it takes a
function that gets called when an observed element starts or stops
intersecting the window

```js
  const visibleCanvasSet = new Set();
  const intersectionObserver = new IntersectionObserver((entries) => {
    for (const { target, isIntersecting } of entries) {
      if (isIntersecting) {
        visibleCanvasSet.add(target);
      } else {
        visibleCanvasSet.delete(target);
      }
    }
  });
```

`wgpu_fun`'s `MultiApp` already observes every canvas it wraps with
exactly this code, and each `Canvas` exposes the result as
`canvas.is_visible()`. (Natively, "visible" means the canvas's cell in
the grid currently intersects the window, which you can try by
scrolling the example's window with the mouse wheel.) The original
JavaScript keeps a `Map` from canvas to per-canvas info and iterates the
visible `Set`; since each of our `Canvas`es carries its own visibility
flag we can keep our `infos` `Vec` and just skip the canvases that
aren't visible.

In our render function, we can just only render the visible canvases

```rust
    for Info {
      canvas,
      clear_value,
      uniform_values,
      uniform_buffer,
      bind_group,
      rotation,
    } in infos.iter_mut()
    {
+      // Only render the canvases that are actually visible — the
+      // IntersectionObserver visibility set from the lesson.
+      if !canvas.is_visible() {
+        continue;
+      }

      // Get the current texture from the canvas context and
      // set it as the texture to render to.
      let view = canvas.current_view();

      ...
```

And with that, we're only drawing the canvases that are actually visible, which should
hopefully be much lighter.

{{{example url="../webgpu-multiple-canvases-x200-optimized.html"}}}

`IntersectionObserver` will probably not cover every case. If you are drawing very heavy
things in each canvas then you might want to only animate canvases the user selects.
In any case, hopefully you have one more tool in your toolbox.
