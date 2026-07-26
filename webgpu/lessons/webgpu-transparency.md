Title: WebGPU Transparency and Blending
Description: Blending Pixels in WebGPU
TOC: Transparency and Blending

It's hard to cover transparency and blending because often, what you need
to do for one situation is different than for another. So, this article
will mostly be a tour of WebGPU features so we can refer back here when
we cover specific techniques.

## <a href="a-alphamode"></a> Canvas `alphaMode`

The first thing we need to be aware of, there is transparent and blending within WebGPU
but there is also transparency and blending with a WebGPU canvas and the HTML page.

By default a WebGPU canvas is opaque. Its alpha channel is ignored. To make it not
ignored we have to set its `alphaMode` to `'premultiplied'`. In wgpu that option is
the *composite alpha mode* of the surface configuration; `wgpu_fun` passes its
`alpha_mode` field along when it configures the surface (the equivalent of calling
`configure` on the canvas context). The default, `Auto`, behaves like `'opaque'`.

```rust
  let mut app = App::new("WebGPU Canvas alphaMode premultiplied").await;
  app.auto_resize = true;
+  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
```

It's important to understand what `alphaMode: 'premultiplied'` means. It means,
the colors you put in the canvas must have their color values already multiplied
by the alpha value.

Let's make the smallest example we can. We'll just create a render pass and set
the clear color.

```rust
use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
  let mut app = App::new("WebGPU Canvas alphaMode premultiplied").await;
  app.auto_resize = true;
+  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

  app.run(RenderMode::Once, move |frame: &Frame| {
    let clear_value = [1.0, 0.0, 0.0, 0.01];

    let mut encoder = frame.device.create_command_encoder(
      &wgpu::CommandEncoderDescriptor { label: Some("clear encoder") });
    {
      let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: clear_value[0],
              g: clear_value[1],
              b: clear_value[2],
              a: clear_value[3],
            }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
}

fn main() {
  wgpu_fun::start(run());
}
```

Let's also set the canvas's CSS background to a gray checkerboard

```css
canvas {
  background-color: #404040;
  background-image:
     linear-gradient(45deg, #808080 25%, transparent 25%),
     linear-gradient(-45deg, #808080 25%, transparent 25%),
     linear-gradient(45deg, transparent 75%, #808080 75%),
     linear-gradient(-45deg, transparent 75%, #808080 75%);
  background-size: 32px 32px;
  background-position: 0 0, 0 16px, 16px -16px, -16px 0px;
}
```

To that let's add a UI so we can set the alpha and color of
the clear value as well as whether or not it's premultiplied.

The settings panel itself is plain DOM UI, so on the converted pages it stays
in the page's JavaScript. Its change handlers hand the values to our Rust
code through the wasm module's `set_setting_*` functions

```js
+import GUI from '../3rdparty/muigui-0.x.module.js';
+import init, * as wasm from './wasm/webgpu-canvas-alphamode-premultiplied/webgpu-canvas-alphamode-premultiplied.js';
+await init();
+
+const color = [1, 0, 0];
+const settings = {
+  premultiply: false,
+  color,
+  alpha: 0.01,
+};
+
+// send the current settings to the wasm module (which re-renders)
+function update() {
+  wasm.set_setting_bool('premultiply', settings.premultiply);
+  wasm.set_setting_num('alpha', settings.alpha);
+  wasm.set_setting_num('color0', color[0]);
+  wasm.set_setting_num('color1', color[1]);
+  wasm.set_setting_num('color2', color[2]);
+}
+
+const gui = new GUI().onChange(update);
+gui.add(settings, 'premultiply');
+gui.add(settings, 'alpha', 0, 1);
+gui.addColor(settings, 'color');
```

and in Rust we read the current values with `wgpu_fun::setting_bool` and
friends. Changing a setting automatically triggers a re-render.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
-    let clear_value = [1.0, 0.0, 0.0, 0.01];
+    // read the settings the GUI on the page sets
+    let premultiply = wgpu_fun::setting_bool("premultiply", false);
+    let alpha = wgpu_fun::setting_f64("alpha", 0.01);
+    let color = [
+      wgpu_fun::setting_f64("color0", 1.0),
+      wgpu_fun::setting_f64("color1", 0.0),
+      wgpu_fun::setting_f64("color2", 0.0),
+    ];
+
+    let mut clear_value = [0.0, 0.0, 0.0, alpha];
+    if premultiply {
+      // premultiply the colors by the alpha
+      clear_value[0] = color[0] * alpha;
+      clear_value[1] = color[1] * alpha;
+      clear_value[2] = color[2] * alpha;
+    } else {
+      // use un-premultiplied colors
+      clear_value[0] = color[0];
+      clear_value[1] = color[1];
+      clear_value[2] = color[2];
+    }

    let mut encoder = frame.device.create_command_encoder(
      &wgpu::CommandEncoderDescriptor { label: Some("clear encoder") });
    ...
```

If we run that I hope you'll see an issue

{{{example url="../webgpu-canvas-alphamode-premultiplied.html"}}}

What colors appear here is **UNDEFINED**!!!

On my machine I got these colors

<img src="resources/canvas-invalid-color.png" class="center" style="width: 440px">

Do you see what's wrong? We have the alpha set to 0.01. The background colors
are supposed to be medium and dark gray. The color is set to red (1, 0, 0).
Putting 0.01 amount of red on top of a medium/dark gray checkerboard should be
nearly imperceptible so why is it 2 bright shades of pink?

The reason is, **THIS IS AN ILLEGAL COLOR!**. The color of
our canvas is `1, 0, 0, 0.01` but that is not a premultiplied
color. "premultiplied" means the colors we put in the canvas
must already be multiplied by the alpha value. Given an alpha
value of 0.01, no other value should be greater than 0.01.

If you click the 'premultiplied' checkbox then the code will
premultiply the color. The value put in the canvas will be
`0.01, 0, 0, 0.01` and it will look correct, almost imperceptible.

With 'premultiplied' checked, adjust the alpha and
you'll see it fades to red as the alpha approaches 1.

> Note: Because the example `1, 0, 0, 0.01` is an illegal color,
> how it is displayed is undefined. It's up to the browser what
> happens with illegal colors so don't use illegal colors and
> expect the same results across devices.

Let's say our color is 1, 0.5, 0.25 which is orange and we want it to be 33%
transparent so our alpha is 0.33. Then, our "premultiplied color" would be

```
                      premultiplied
   ---------------------------------
   r = 1    * 0.33   = 0.33
   g = 0.5  * 0.33   = 0.165
   g = 0.25 * 0.33   = 0.0825
   a = 0.33          = 0.33
```

How you get a pre-multiplied color is up to you. If you have un-premultiplied
colors then in the shader you could premultiply with code like this.

```wgsl
   return vec4f(color.rgb * color.a, color.a)`;
```

The JavaScript API's `copyExternalImageToTexture` function, which we covered in
[the article on importing textures](webgpu-importing-textures.html),
takes a `premultipliedAlpha: true` option. ([see below](#copyExternalImageToTexture))
This means when you load the image into the texture
you can tell WebGPU to premultiply the colors for
you as it copies them to the texture. In Rust we upload pixels with
`write_texture`, so we do that multiplication ourselves as we copy — we'll do
exactly that in the blending example below. Either way, when you call
`textureSample` the value you get will already be premultiplied.

The point of this section was

1. To explain `alphaMode: 'premultiplied'` WebGPU canvas configuration option.

   This lets a WebGPU canvas have transparency

2. To introduce the concept of premultiplied alpha colors 

   How you get premultiplied colors is up to you. In the 
   example above we created a premultiplied `clear_value`
   in Rust.

   We can also return colors from fragment shaders (and/or)
   other shaders. We might provide premultiplied colors
   to those shaders. We might do the multiplication in
   the shader itself. We might run a post processing pass
   to premultiply the colors. What's important is that
   the colors in the canvas, one way or another, end up
   premultiplied if we're using `alphaMode: 'premultiplied'`

   A good reference for other premultiplied vs un-premultiplied
   colors is this article:
   [GPUs prefer premultiplication](https://www.realtimerendering.com/blog/gpus-prefer-premultiplication/).

## <a href="a-discard"></a> Discard

`discard` is a WGSL statement that you can use in a fragment
shader to discard the current fragment or in other words, to
not draw a pixel.

Let's take our example that draws a checkerboard in the fragment
shader using the `@builtin(position)` from [the article on inter-stage variables](webgpu-inter-stage-variables.html#a-builtin-position).

Instead of drawing a 2 color checkerboard, we'll discard
for one of the two cases.

```wgsl
@fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
-  let red = vec4f(1, 0, 0, 1);
  let cyan = vec4f(0, 1, 1, 1);

  let grid = vec2u(fsInput.position.xy) / 8;
  let checker = (grid.x + grid.y) % 2 == 1;

+        if (checker) {
+          discard;
+        }
+
+        return cyan;

-  return select(red, cyan, checker);
}
```

A few other changes, we'll add in the CSS above to make the
canvas have a CSS checkerboard background. We'll also set
the alpha mode to premultiplied. And we'll set the clear value
to `0, 0, 0, 0`

```rust
  let mut app = App::new("WebGPU Fragment Shader Discard").await;
  app.auto_resize = true;
+  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

  ...

      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
-            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
+            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });
...

```

{{{example url="../webgpu-transparency-fragment-shader-discard.html"}}}

You should see that every other square is "transparent" in that
it wasn't even drawn.

It's common in a shader used for transparency to discard based
on the alpha value. Something like

```wgsl
@fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
    let color = ... compute a color ....

    if (color.a < threshold) {
      discard;
    }

    return color;
}
```

Where `threshold` might be a value from a uniform or a constant
or whatever is appropriate.

This is probably most commonly used for sprites and for foliage like grass and
leaves because, if we are drawing and we're using a depth texture, like we
introduced in [the article on orthographic projection](webgpu-orthograpic-projection.html#a-depth-textures),
then when we draw a sprite, leaf, or blade of grass, none of the sprites,
leaves, or grass behind the thing we're currently drawing will be drawn, even if
the alpha value is 0 because we'll still be updating the depth texture. So,
instead of drawing we discard. We'll go over this more in another article.

## <a href="a-blending"></a> Blend Settings

Finally we get to blend settings. When you create a render pipeline, for each
`target` in the fragment shader, you can set blending state. In other words,
here's a typical pipeline from our other examples so far

```rust
    let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("hardcoded textured quad pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[],
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

And here it is with blending added to `targets[0]`. The `app.format.into()`
shorthand made a `ColorTargetState` with no blending; now we write it out in
full so we can fill in the `blend` field.

```rust
      fragment: Some(wgpu::FragmentState {
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
-        targets: &[Some(app.format.into())],
+        targets: &[Some(wgpu::ColorTargetState {
+          format: app.format,
+          blend: Some(wgpu::BlendState {
+            color: wgpu::BlendComponent {
+              src_factor: wgpu::BlendFactor::One,
+              dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
+              ..Default::default()
+            },
+            alpha: wgpu::BlendComponent {
+              src_factor: wgpu::BlendFactor::One,
+              dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
+              ..Default::default()
+            },
+          }),
+          write_mask: wgpu::ColorWrites::ALL,
+        })],
      }),
```

The full list of default settings are:

```rust
blend: Some(wgpu::BlendState {
  color: wgpu::BlendComponent {
    operation: wgpu::BlendOperation::Add,
    src_factor: wgpu::BlendFactor::One,
    dst_factor: wgpu::BlendFactor::Zero,
  },
  alpha: wgpu::BlendComponent {
    operation: wgpu::BlendOperation::Add,
    src_factor: wgpu::BlendFactor::One,
    dst_factor: wgpu::BlendFactor::Zero,
  },
}),
```

Where `color` is what happens to the `rgb` portion of a color and `alpha` is
what happens to the `a` (alpha) portion.

`operation` (a `wgpu::BlendOperation`) can be one of

  * `Add`
  * `Subtract`
  * `ReverseSubtract`
  * `Min`
  * `Max`

`src_factor` and `dst_factor` (`wgpu::BlendFactor`) can each be one of

  * `Zero`
  * `One`
  * `Src`
  * `OneMinusSrc`
  * `SrcAlpha`
  * `OneMinusSrcAlpha`
  * `Dst`
  * `OneMinusDst`
  * `DstAlpha`
  * `OneMinusDstAlpha`
  * `SrcAlphaSaturated`
  * `Constant`
  * `OneMinusConstant`

Most of them are relatively straight forward to understand. Think of it as

```
   result = operation((src * srcFactor),  (dst * dstFactor))
```

Where `src` is the value returned by your fragment shader and `dst` is the value
already in the texture you are drawing to.

Consider the default where `operation` is `Add`, `src_factor` is `One` and
`dst_factor` is `Zero`. This gives us

```
   result = add((src * 1), (dst * 0))
   result = add(src * 1, dst * 0)
   result = add(src, 0)
   result = src;
```

As you can see, the default result ends up being just `src`.

Of the blend factors above, 2 mention a constant, `Constant` and
`OneMinusConstant`. The constant referred to here is set in a render pass
with the `set_blend_constant` command and defaults to `0, 0, 0, 0`. This lets
you change it between draws.

Probably the most common setting for blending is

```rust
wgpu::BlendComponent {
  operation: wgpu::BlendOperation::Add,
  src_factor: wgpu::BlendFactor::One,
  dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
}
```

This mode is used most often with "premultiplied alpha" meaning it expects that
the "src" has already had its RGB colors "premultiplied" by the alpha value as
we covered above.

Let's make an example that shows these options. 

The original JavaScript version of this article first draws two images with
some alpha using the canvas 2D API, and loads those 2 canvases into WebGPU
textures. We don't have a 2D canvas in Rust, so we'll compute the same two
images pixel by pixel and get the same result.

First, some code for making an image we'll use for our dst texture: a diagonal
rainbow gradient with diagonal stripes erased from it. The canvas version fills
a linear gradient of `hsl` colors, then rotates by -45° and, using the
'destination-out' composite mode, fills rectangles to erase stripes. We compute
the same gradient and stripe coverage directly.

```rust
// The JS version makes its colors with CSS `hsl(...)`/`hsla(...)` strings.
// This is the same conversion CSS does (s and l fixed at 1 and 0.5 like the
// examples use them; h is in turns, `h * 360` is degrees).
fn hsl(h: f32) -> [f32; 3] {
  let h = ((h * 360.0) as i32 as f32).rem_euclid(360.0) / 60.0; // `h * 360 | 0`
  let x = 1.0 - (h % 2.0 - 1.0).abs();
  match h as u32 {
    0 => [1.0, x, 0.0],
    1 => [x, 1.0, 0.0],
    2 => [0.0, 1.0, x],
    3 => [0.0, x, 1.0],
    4 => [x, 0.0, 1.0],
    _ => [1.0, 0.0, x],
  }
}

// Reproduces the JS createDestinationImage: a diagonal rainbow linear
// gradient with diagonal stripes erased with the 'destination-out'
// composite mode.
fn create_destination_image(size: u32) -> SourceImage {
  let sizef = size as f32;

  // the 7 gradient color stops, hsl(0 / -6) ... hsl(6 / -6)
  let stops: Vec<[f32; 3]> = (0..=6).map(|i| hsl(i as f32 / -6.0)).collect();

  let mut pixels = Vec::with_capacity((size * size) as usize);
  for y in 0..size {
    for x in 0..size {
      // the linear gradient runs from the top-left corner (0, 0) to
      // the bottom-right corner (size, size)
      let t = ((x as f32 + 0.5) + (y as f32 + 0.5)) / (sizef * 2.0);
      let seg = (t * 6.0).clamp(0.0, 6.0);
      let ndx = (seg as usize).min(5);
      let f = seg - ndx as f32;
      let color: [f32; 3] =
          std::array::from_fn(|c| stops[ndx][c] + (stops[ndx + 1][c] - stops[ndx][c]) * f);

      // erase 16 pixel tall stripes every 32 pixels, rotated by
      // PI / -4, like the rotate + fillRect loop (4x4 supersampled
      // to keep the anti-aliased edges the canvas gives us)
      let mut coverage = 0.0;
      for sy in 0..4 {
        for sx in 0..4 {
          let px = x as f32 + (sx as f32 + 0.5) / 4.0;
          let py = y as f32 + (sy as f32 + 0.5) / 4.0;
          let stripe = (px + py) / std::f32::consts::SQRT_2;
          if stripe.rem_euclid(32.0) < 16.0 {
            coverage += 1.0 / 16.0;
          }
        }
      }
      pixels.push([color[0], color[1], color[2], 1.0 - coverage]);
    }
  }
  to_rgba8(pixels, size, size)
}
```

where `SourceImage` is our stand-in for a canvas: tightly packed
un-premultiplied rgba8 pixels

```rust
struct SourceImage {
  data: Vec<u8>,
  width: u32,
  height: u32,
}

fn to_rgba8(pixels: Vec<[f32; 4]>, width: u32, height: u32) -> SourceImage {
  let data = pixels
    .iter()
    .flat_map(|p| p.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8))
    .collect();
  SourceImage { data, width, height }
}
```

And here's the canvas 2D version it reproduces, running.

{{{example url="../webgpu-blend-dest-canvas.html"}}}

Here's some code for making an image we'll use for our
src texture: three circles filled with radial gradients that are opaque in
the middle and fade out to transparent, drawn with the 'screen' composite
mode. We loop over the 3 circles and apply the screen compositing math
ourselves.

```rust
// Reproduces the JS createSourceImage: three circles with radial
// hsla gradients (opaque in the middle, transparent at the edge),
// drawn with the 'screen' composite mode.
fn create_source_image(size: u32) -> SourceImage {
  let sizef = size as f32;
  let mut pixels = vec![[0.0f32; 4]; (size * size) as usize];

  const NUM_CIRCLES: u32 = 3;
  for i in 0..NUM_CIRCLES {
    // the canvas version rotates PI * 2 / numCircles each time and
    // translates by size / 6; these are the resulting circle centers
    let angle = std::f32::consts::PI * 2.0 * (i + 1) as f32 / NUM_CIRCLES as f32;
    let center_x = sizef / 2.0 + angle.cos() * sizef / 6.0;
    let center_y = sizef / 2.0 + angle.sin() * sizef / 6.0;

    let radius = sizef / 3.0;
    let color = hsl(i as f32 / NUM_CIRCLES as f32);

    for y in 0..size {
      for x in 0..size {
        let dx = x as f32 + 0.5 - center_x;
        let dy = y as f32 + 0.5 - center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        // the radial gradient: alpha 1 from the center to half way
        // between radius / 2 and radius, fading to 0 at radius
        let t = ((dist - radius / 2.0) / (radius / 2.0)).clamp(0.0, 1.0);
        let src_alpha = ((1.0 - t) * 2.0).clamp(0.0, 1.0);
        if src_alpha <= 0.0 {
          continue;
        }

        // composite onto what's already there with the canvas
        // 'screen' blend mode
        let [dst_r, dst_g, dst_b, dst_a] = pixels[(y * size + x) as usize];
        let out_a = src_alpha + dst_a * (1.0 - src_alpha);
        let screen = |cb: f32, cs: f32| cb + cs - cb * cs;
        let blend = |cb: f32, cs: f32| {
          (src_alpha * (1.0 - dst_a) * cs
              + src_alpha * dst_a * screen(cb, cs)
              + (1.0 - src_alpha) * dst_a * cb)
              / out_a
        };
        pixels[(y * size + x) as usize] = [
          blend(dst_r, color[0]),
          blend(dst_g, color[1]),
          blend(dst_b, color[2]),
          out_a,
        ];
      }
    }
  }
  to_rgba8(pixels, size, size)
}
```

And here's the canvas 2D version of that running.

{{{example url="../webgpu-blend-src-canvas.html"}}}

Now that we have both, we can modify the canvas importing example from
[the article on importing textures](webgpu-import-textures.html#a-loading-canvas).

First, let's make the 2 images

```rust
let size = 300;
let src_image = create_source_image(size);
let dst_image = create_destination_image(size);
```

Let's modify the shader so it doesn't multiply
the texture coords by 50 since we will not be trying to
draw a long plane into the distance.

```wgsl
@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32
) -> OurVertexShaderOutput {
  let pos = array(
    // 1st triangle
    vec2f( 0.0,  0.0),  // center
    vec2f( 1.0,  0.0),  // right, center
    vec2f( 0.0,  1.0),  // center, top

    // 2nd triangle
    vec2f( 0.0,  1.0),  // center, top
    vec2f( 1.0,  0.0),  // right, center
    vec2f( 1.0,  1.0),  // right, top
  );

  var vsOutput: OurVertexShaderOutput;
  let xy = pos[vertexIndex];
  vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
-  vsOutput.texcoord = xy * vec2f(1, 50);
+  vsOutput.texcoord = xy;
  return vsOutput;
}
```

<a id="copyExternalImageToTexture"></a>Next, let's update the
`copy_source_to_texture` function so we can pass `premultiplied_alpha:
true/false` to it. This is where the JavaScript version passes
`premultipliedAlpha: true` to `copyExternalImageToTexture`; since we upload
with `write_texture` we do the multiplication ourselves as we copy.

```rust
+// The premultipliedAlpha: true option of copyExternalImageToTexture,
+// done on the CPU: multiply the colors by the alpha value as we copy.
+fn premultiply_alpha(source: &SourceImage) -> SourceImage {
+  let data = source
+    .data
+    .chunks_exact(4)
+    .flat_map(|p| {
+      let a = p[3] as f32 / 255.0;
+      [
+        (p[0] as f32 * a).round() as u8,
+        (p[1] as f32 * a).round() as u8,
+        (p[2] as f32 * a).round() as u8,
+        p[3],
+      ]
+    })
+    .collect();
+  SourceImage { data, width: source.width, height: source.height }
+}

fn copy_source_to_texture(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  mip_gen: &mut MipGenerator,
  texture: &wgpu::Texture,
  source: &SourceImage,
+  premultiplied_alpha: bool,
) {
+  let image = if premultiplied_alpha {
+    premultiply_alpha(source)
+  } else {
+    SourceImage {
+      data: source.data.clone(),
+      width: source.width,
+      height: source.height,
+    }
+  };
  queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
-    &source.data,
+    &image.data,
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(image.width * 4),
      rows_per_image: None,
    },
    wgpu::Extent3d {
      width: image.width,
      height: image.height,
      depth_or_array_layers: 1,
    },
  );

  if texture.mip_level_count() > 1 {
    mip_gen.generate_mips(device, queue, texture);
  }
}
```

(`MipGenerator` is the render-pass based `generateMips` from
[the article on importing textures](webgpu-importing-textures.html),
wrapped in a struct that caches its shader module, sampler and per-format
pipelines.)

Then, let's use that to create two versions of each texture, one premultiplied, one "un-premultiplied" or "not premultiplied"

```rust
  let src_texture_unpremultiplied_alpha = create_texture_from_source(
      &app.device, &app.queue, &mut mip_gen, &src_image,
      true, false);
  let dst_texture_unpremultiplied_alpha = create_texture_from_source(
      &app.device, &app.queue, &mut mip_gen, &dst_image,
      true, false);

  let src_texture_premultiplied_alpha = create_texture_from_source(
      &app.device, &app.queue, &mut mip_gen, &src_image,
      true, true);
  let dst_texture_premultiplied_alpha = create_texture_from_source(
      &app.device, &app.queue, &mut mip_gen, &dst_image,
      true, true);
```

where the last 2 parameters are `mips` and `premultiplied_alpha`.

Note: We could add an option to premultiply in the shader but that's
arguably not common. Rather it's more common
to decide, based on your needs, whether all textures containing color are premultiplied
or not premultiplied. So, we'll stick with different textures and add UI options to
select the premultiplied ones or un-premultiplied ones.

We need a uniform buffer for each of our 2 draws just in case we want to draw
in 2 different places or the textures are 2 different sizes.

```rust
  fn make_uniform_buffer_and_values(device: &wgpu::Device) -> (wgpu::Buffer, [f32; 16]) {
    // create a buffer for the uniform values
    const UNIFORM_BUFFER_SIZE: u64 = 16 * 4; // matrix is 16 32bit floats (4bytes each)
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms for quad"),
      size: UNIFORM_BUFFER_SIZE,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    // create an array of f32s to hold the matrix for the uniforms in Rust
    let values = [0.0f32; 16];
    (buffer, values)
  }
  let (src_uniform_buffer, mut src_uniform_values) = make_uniform_buffer_and_values(&app.device);
  let (dst_uniform_buffer, mut dst_uniform_values) = make_uniform_buffer_and_values(&app.device);
```

We need a sampler and we need a bindGroup for each texture. This brings up an issue.
A bindGroup needs a bindGroup layout. Most of the examples on this site
get their layout from a pipeline by calling `some_pipeline.get_bind_group_layout(group_number)`.
In our case though, we're going to create a pipeline based on the blend state settings
we choose. So, we won't have the pipeline to get a bindGroupLayout from until render
time.

We could create the bindGroups at render time. OR, we could create our own
bindGroupLayout and tell the pipelines to use it. This way we can create the bindGroups
at init time and they'll be compatible with any pipeline that uses the same bindGroupLayout.

The details of creating a [bindGroupLayout](GPUBindGroupLayout) and [pipelineLayout](GPUPipelineLayout)
are covered [in another article](webgpu-bind-group-layouts.html). For now, here's the code to create
them that matches our shader module

```rust
  let bind_group_layout = app.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
    ],
  });

  let pipeline_layout = app.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&bind_group_layout)],
    immediate_size: 0,
  });
```

With the bindGroupLayout created, we can use it to make bindGroups.

```rust
  let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    mipmap_filter: wgpu::MipmapFilterMode::Linear,
    ..Default::default()
  });

  let make_bind_group = |texture: &wgpu::Texture, uniform_buffer: &wgpu::Buffer| {
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &bind_group_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::Sampler(&sampler),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&texture_view),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: uniform_buffer.as_entire_binding(),
        },
      ],
    })
  };

  let src_bind_group_unpremultiplied_alpha =
      make_bind_group(&src_texture_unpremultiplied_alpha, &src_uniform_buffer);
  let dst_bind_group_unpremultiplied_alpha =
      make_bind_group(&dst_texture_unpremultiplied_alpha, &dst_uniform_buffer);
  let src_bind_group_premultiplied_alpha =
      make_bind_group(&src_texture_premultiplied_alpha, &src_uniform_buffer);
  let dst_bind_group_premultiplied_alpha =
      make_bind_group(&dst_texture_premultiplied_alpha, &dst_uniform_buffer);
```

Now that we have bindGroups and textures let's make an array of
the premultiplied textures vs the un-premultiplied textures so we can
easily select one set or the other

```rust
  struct TextureSet {
    src_texture: wgpu::Texture,
    dst_texture: wgpu::Texture,
    src_bind_group: wgpu::BindGroup,
    dst_bind_group: wgpu::BindGroup,
  }

  let texture_sets = [
    TextureSet {
      src_texture: src_texture_premultiplied_alpha,
      dst_texture: dst_texture_premultiplied_alpha,
      src_bind_group: src_bind_group_premultiplied_alpha,
      dst_bind_group: dst_bind_group_premultiplied_alpha,
    },
    TextureSet {
      src_texture: src_texture_unpremultiplied_alpha,
      dst_texture: dst_texture_unpremultiplied_alpha,
      src_bind_group: src_bind_group_unpremultiplied_alpha,
      dst_bind_group: dst_bind_group_unpremultiplied_alpha,
    },
  ];
```

We'll need 2 render pipelines. One to draw the dest texture, this one will
not use blending. Notice we're passing in the `pipeline_layout` instead of using
`None` (the 'auto' layout) as we've done in most examples so far.

```rust
  let dst_pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("hardcoded textured quad pipeline"),
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[],
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

The other pipeline will be created at render time, inside our frame callback,
with whatever blend options we choose

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    let color = wgpu::BlendComponent {
      operation: wgpu::BlendOperation::Add,
      src_factor: wgpu::BlendFactor::One,
      dst_factor: wgpu::BlendFactor::OneMinusSrc,
    };
    let alpha = wgpu::BlendComponent {
      operation: wgpu::BlendOperation::Add,
      src_factor: wgpu::BlendFactor::One,
      dst_factor: wgpu::BlendFactor::OneMinusSrc,
    };

    let src_pipeline = frame.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("hardcoded textured quad pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[],
      },
      fragment: Some(wgpu::FragmentState {
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        targets: &[Some(wgpu::ColorTargetState {
          format: frame.format,
          blend: Some(wgpu::BlendState { color, alpha }),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: Default::default(),
      depth_stencil: None,
      multisample: Default::default(),
      multiview_mask: None,
      cache: None,
    });

    ...
```

To render we choose a texture set and then render the dst texture
with the `dst_pipeline` (no blending), and then on top of that we render
the src texture with the `src_pipeline` (with blending)

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
+    let texture_set_ndx =
+        (wgpu_fun::setting_f64("textureSet", 0.0) as usize).min(texture_sets.len() - 1);

    ...

    let src_pipeline = frame.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      ...
    });

+    let TextureSet {
+      src_texture,
+      dst_texture,
+      src_bind_group,
+      dst_bind_group,
+    } = &texture_sets[texture_set_ndx];

+    let update_uniforms =
+        |uniform_buffer: &wgpu::Buffer, values: &mut [f32; 16], texture: &wgpu::Texture| {
+      let projection_matrix = glam::camera::rh::proj::directx::orthographic(
+          0.0, frame.width as f32, frame.height as f32, 0.0, -1.0, 1.0);
+
+      let matrix = projection_matrix
+          * Mat4::from_scale(vec3(texture.width() as f32, texture.height() as f32, 1.0));
+      values.copy_from_slice(&matrix.to_cols_array());
+
+      // copy the values from Rust to the GPU
+      frame.queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(values));
+    };
+    update_uniforms(&src_uniform_buffer, &mut src_uniform_values, src_texture);
+    update_uniforms(&dst_uniform_buffer, &mut dst_uniform_values, dst_texture);

    let mut encoder = frame.device.create_command_encoder(
      &wgpu::CommandEncoderDescriptor { label: Some("render quad encoder") });
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });

+      // draw dst
+      pass.set_pipeline(&dst_pipeline);
+      pass.set_bind_group(0, dst_bind_group, &[]);
+      pass.draw(0..6, 0..1);  // call our vertex shader 6 times
+
+      // draw src
+      pass.set_pipeline(&src_pipeline);
+      pass.set_bind_group(0, src_bind_group, &[]);
+      pass.draw(0..6, 0..1);  // call our vertex shader 6 times
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

Now let's make some UI to set these values. Like before, the muigui panel
stays in the page's JavaScript

```js
+  const operations = [
+    'add',
+    'subtract',
+    'reverse-subtract',
+    'min',
+    'max',
+  ];
+
+  const factors = [
+    'zero',
+    'one',
+    'src',
+    'one-minus-src',
+    'src-alpha',
+    'one-minus-src-alpha',
+    'dst',
+    'one-minus-dst',
+    'dst-alpha',
+    'one-minus-dst-alpha',
+    'src-alpha-saturated',
+    'constant',
+    'one-minus-constant',
+  ];

  const color = {
    operation: 'add',
    srcFactor: 'one',
    dstFactor: 'one-minus-src',
  };

  const alpha = {
    operation: 'add',
    srcFactor: 'one',
    dstFactor: 'one-minus-src',
  };

  const settings = {
    textureSet: 0,
  };

+  const gui = new GUI().onChange(update);
+  gui.add(settings, 'textureSet', ['premultiplied alpha', 'un-premultiplied alpha']);
+  const colorFolder = gui.addFolder('color');
+  colorFolder.add(color, 'operation', operations);
+  colorFolder.add(color, 'srcFactor', factors);
+  colorFolder.add(color, 'dstFactor', factors);
+  const alphaFolder = gui.addFolder('alpha');
+  alphaFolder.add(alpha, 'operation', operations);
+  alphaFolder.add(alpha, 'srcFactor', factors);
+  alphaFolder.add(alpha, 'dstFactor', factors);
```

where `update` sends each value to the wasm module with
`wasm.set_setting_str('colorOperation', color.operation)` etc... On the Rust
side we read the strings and convert them to the wgpu enums with two small
lookup functions

```rust
fn blend_operation(name: &str) -> wgpu::BlendOperation {
  match name {
    "add" => wgpu::BlendOperation::Add,
    "subtract" => wgpu::BlendOperation::Subtract,
    "reverse-subtract" => wgpu::BlendOperation::ReverseSubtract,
    "min" => wgpu::BlendOperation::Min,
    "max" => wgpu::BlendOperation::Max,
    _ => wgpu::BlendOperation::Add,
  }
}

fn blend_factor(name: &str) -> wgpu::BlendFactor {
  match name {
    "zero" => wgpu::BlendFactor::Zero,
    "one" => wgpu::BlendFactor::One,
    "src" => wgpu::BlendFactor::Src,
    "one-minus-src" => wgpu::BlendFactor::OneMinusSrc,
    "src-alpha" => wgpu::BlendFactor::SrcAlpha,
    "one-minus-src-alpha" => wgpu::BlendFactor::OneMinusSrcAlpha,
    "dst" => wgpu::BlendFactor::Dst,
    "one-minus-dst" => wgpu::BlendFactor::OneMinusDst,
    "dst-alpha" => wgpu::BlendFactor::DstAlpha,
    "one-minus-dst-alpha" => wgpu::BlendFactor::OneMinusDstAlpha,
    "src-alpha-saturated" => wgpu::BlendFactor::SrcAlphaSaturated,
    "constant" => wgpu::BlendFactor::Constant,
    "one-minus-constant" => wgpu::BlendFactor::OneMinusConstant,
    _ => wgpu::BlendFactor::One,
  }
}
```

and use them when we make the blend components in the frame callback

```rust
-    let color = wgpu::BlendComponent {
-      operation: wgpu::BlendOperation::Add,
-      src_factor: wgpu::BlendFactor::One,
-      dst_factor: wgpu::BlendFactor::OneMinusSrc,
-    };
-    let alpha = wgpu::BlendComponent {
-      operation: wgpu::BlendOperation::Add,
-      src_factor: wgpu::BlendFactor::One,
-      dst_factor: wgpu::BlendFactor::OneMinusSrc,
-    };
+    // read the settings the GUI on the page sets
+    let mut color = wgpu::BlendComponent {
+      operation: blend_operation(&wgpu_fun::setting_str("colorOperation", "add")),
+      src_factor: blend_factor(&wgpu_fun::setting_str("colorSrcFactor", "one")),
+      dst_factor: blend_factor(&wgpu_fun::setting_str("colorDstFactor", "one-minus-src")),
+    };
+    let mut alpha = wgpu::BlendComponent {
+      operation: blend_operation(&wgpu_fun::setting_str("alphaOperation", "add")),
+      src_factor: blend_factor(&wgpu_fun::setting_str("alphaSrcFactor", "one")),
+      dst_factor: blend_factor(&wgpu_fun::setting_str("alphaDstFactor", "one-minus-src")),
+    };
```

If the operation is `Min` or `Max` we must set `src_factor` and `dst_factor` to
`One` or else we'll get an error

```rust
+// if the operation is min or max, srcFactor and dstFactor must be one or
+// we'll get an error
+fn make_blend_component_valid(blend: &mut wgpu::BlendComponent) {
+  if blend.operation == wgpu::BlendOperation::Min || blend.operation == wgpu::BlendOperation::Max
+  {
+    blend.src_factor = wgpu::BlendFactor::One;
+    blend.dst_factor = wgpu::BlendFactor::One;
+  }
+}

  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

+    make_blend_component_valid(&mut color);
+    make_blend_component_valid(&mut alpha);

    ...
```

(the page's `update` function applies the same fix to the GUI's own values and
calls `gui.updateDisplay()` so the panel shows what's actually used).

Let's also make it possible to set the blend constant for when we pick
`Constant` or `OneMinusConstant` as a factor. In the page

```js
+  const constant = {
+    color: [1, 0.5, 0.25],
+    alpha: 1,
+  };

  ...

+  const constantFolder = gui.addFolder('constant');
+  constantFolder.addColor(constant, 'color');
+  constantFolder.add(constant, 'alpha', 0, 1);
```

and in the frame callback

```rust
+    let constant_color = [
+      wgpu_fun::setting_f64("constantColor0", 1.0),
+      wgpu_fun::setting_f64("constantColor1", 0.5),
+      wgpu_fun::setting_f64("constantColor2", 0.25),
+    ];
+    let constant_alpha = wgpu_fun::setting_f64("constantAlpha", 1.0);

    ...

      // draw dst
      pass.set_pipeline(&dst_pipeline);
      pass.set_bind_group(0, dst_bind_group, &[]);
      pass.draw(0..6, 0..1);  // call our vertex shader 6 times

      // draw src
      pass.set_pipeline(&src_pipeline);
      pass.set_bind_group(0, src_bind_group, &[]);
+      pass.set_blend_constant(wgpu::Color {
+        r: constant_color[0],
+        g: constant_color[1],
+        b: constant_color[2],
+        a: constant_alpha,
+      });
      pass.draw(0..6, 0..1);  // call our vertex shader 6 times
```

As there are 13 * 13 * 5 * 13 * 13 * 5 possible settings there are
just too many to explore so let's provide a list of presets. If
there is no `alpha` setting we'll just repeat the `color` setting.
The presets only set GUI values, so they live entirely in the page

```js
+  const presets = {
+    'default (copy)': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one',
+        dstFactor: 'zero',
+      },
+    },
+    'premultiplied blend (source-over)': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one',
+        dstFactor: 'one-minus-src-alpha',
+      },
+    },
+    'un-premultiplied blend': {
+      color: {
+        operation: 'add',
+        srcFactor: 'src-alpha',
+        dstFactor: 'one-minus-src-alpha',
+      },
+    },
+    'destination-over': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one-minus-dst-alpha',
+        dstFactor: 'one',
+      },
+    },
+    'source-in': {
+      color: {
+        operation: 'add',
+        srcFactor: 'dst-alpha',
+        dstFactor: 'zero',
+      },
+    },
+    'destination-in': {
+      color: {
+        operation: 'add',
+        srcFactor: 'zero',
+        dstFactor: 'src-alpha',
+      },
+    },
+    'source-out': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one-minus-dst-alpha',
+        dstFactor: 'zero',
+      },
+    },
+    'destination-out': {
+      color: {
+        operation: 'add',
+        srcFactor: 'zero',
+        dstFactor: 'one-minus-src-alpha',
+      },
+    },
+    'source-atop': {
+      color: {
+        operation: 'add',
+        srcFactor: 'dst-alpha',
+        dstFactor: 'one-minus-src-alpha',
+      },
+    },
+    'destination-atop': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one-minus-dst-alpha',
+        dstFactor: 'src-alpha',
+      },
+    },
+    'additive (lighten)': {
+      color: {
+        operation: 'add',
+        srcFactor: 'one',
+        dstFactor: 'one',
+      },
+    },
+  };

  ...

  const settings = {
    textureSet: 0,
+    preset: 'default (copy)',
  };

  const gui = new GUI().onChange(update);
  gui.add(settings, 'textureSet', ['premultiplied alpha', 'un-premultiplied alpha']);
+  gui.add(settings, 'preset', Object.keys(presets))
+    .name('blending preset')
+    .onChange(presetName => {
+      const preset = presets[presetName];
+      Object.assign(color, preset.color);
+      Object.assign(alpha, preset.alpha || preset.color);
+      gui.updateDisplay();
+    });

  ...
```

Let's also let you choose the canvas configuration for `alphaMode`. The JS
version calls `context.configure` with the new mode every time it renders. In
our setup the surface configuration lives inside `wgpu_fun`, so `App` has an
`alpha_mode_fn` hook: it's consulted every frame and the surface is
reconfigured whenever the returned mode changes.

```js
  const settings = {
+    alphaMode: 'premultiplied',
    textureSet: 0,
    preset: 'default (copy)',
  };

  const gui = new GUI().onChange(update);
+  gui.add(settings, 'alphaMode', ['opaque', 'premultiplied']).name('canvas alphaMode');
  gui.add(settings, 'textureSet', ['premultiplied alpha', 'un-premultiplied alpha']);
```

```rust
  let mut app = App::new("WebGPU Blend").await;
  app.auto_resize = true;
+  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
+  app.alpha_mode_fn = Some(Box::new(|| {
+    match wgpu_fun::setting_str("alphaMode", "premultiplied").as_str() {
+      "opaque" => wgpu::CompositeAlphaMode::Auto,
+      _ => wgpu::CompositeAlphaMode::PreMultiplied,
+    }
+  }));
```

And finally, lets let you pick the clear value for the render pass.

```js
+  const clear = {
+    color: [0, 0, 0],
+    alpha: 0,
+    premultiply: true,
+  };

  ...

+  const clearFolder = gui.addFolder('clear color');
+  clearFolder.add(clear, 'premultiply');
+  clearFolder.add(clear, 'alpha', 0, 1);
+  clearFolder.addColor(clear, 'color');
```

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

+    let clear_color = [
+      wgpu_fun::setting_f64("clearColor0", 0.0),
+      wgpu_fun::setting_f64("clearColor1", 0.0),
+      wgpu_fun::setting_f64("clearColor2", 0.0),
+    ];
+    let clear_alpha = wgpu_fun::setting_f64("clearAlpha", 0.0);
+    let clear_premultiply = wgpu_fun::setting_bool("clearPremultiply", true);

    ...

+    let mult = if clear_premultiply { clear_alpha } else { 1.0 };
+    let clear_value = wgpu::Color {
+      r: clear_color[0] * mult,
+      g: clear_color[1] * mult,
+      b: clear_color[2] * mult,
+      a: clear_alpha,
+    };

    ...

          ops: wgpu::Operations {
-            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }),
+            load: wgpu::LoadOp::Clear(clear_value),
            store: wgpu::StoreOp::Store,
          },
```

That was a lot of options. Maybe too many 😅. In any case, we now have an
example where we can play around with the blend settings

{{{example url="../webgpu-blend.html"}}}

Given our source images

<div class="webgpu_center">
  <div data-diagram="original"></div>
</div>

Here's some known useful blend settings

<div class="webgpu_center">
  <div data-diagram="blend-premultiplied blend (source-over)"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-destination-over"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-additive (lighten)"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-source-in"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-destination-in"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-source-out"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-destination-out"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-source-atop"></div>
</div>

<div class="webgpu_center">
  <div data-diagram="blend-destination-atop"></div>
</div>

<hr>

These blend setting names are from the Canvas 2D
[`globalCompositeOperation`](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/globalCompositeOperation)
options. There are more options listed in that spec but most of the rest require
more math than can be done with only these base blending settings and so require
different solutions.

Now that we have these fundamentals of blending in WebGPU we can refer to them as we
cover various techniques.

<!-- keep this at the bottom of the article -->
<link href="webgpu-transparency.css" rel="stylesheet">
<script type="module" src="webgpu-transparency.js"></script>
