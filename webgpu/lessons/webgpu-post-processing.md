Title: WebGPU Post Processing - Basic CRT Effect
Description: Post Processing
TOC: Basic CRT Effect

Post Processing just means to do some processing after you've created the "original" image.
Post processing can apply to a photo, a video, a 2d scene, a 3d scene. It just generally
means you have an image and you apply some effects to that image, like choosing a filter
on Instagram.

In almost every example on this site we render to the canvas texture. To do post processing
we instead render to a different texture. Then render that texture to the canvas while
applying some image processing effects.

As a simple example, let's try to post process an image to make it kind of look like a 1980s TV
with scanlines and CRT RGB elements.

<div class="webgpu_center"><img class="nobg" src="resources/gemini-generated-1980s-tv-1024.png" style="width: 700px"></div>

To do that, lets take the animated example from the top of [the article on timing](webgpu-timing.html).
The first thing we'll do is make it render to a separate texture and then render that texture
to the canvas.

Here's a shader that draws a [large clip space triangle](webgpu-large-triangle-to-cover-clip-space.html).
and passes the correct UV coordinates to let as draw a texture that covers the portion of the triangle
that fits in clip space.

```rust
  let post_process_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
        return vsOutput;
      }

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
        return vec4f(color);
      }
    "#.into()),
  });
```

It's pretty straight forward and is similar to the shader we used to generate mipmaps
in [the article on using images with textures](webgpu-importing-textures.html). The
only major difference is the original shader uses 2 triangles to cover clip space,
this one uses [1 large triangle](webgpu-large-triangle-to-cover-clip-space.html).

Then, to use these shaders we need a pipeline

```rust
  let post_process_pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: None,
    vertex: wgpu::VertexState {
      module: &post_process_module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[],
    },
    fragment: Some(wgpu::FragmentState {
      module: &post_process_module,
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

This pipeline will be rendering to the canvas so we need to
set the target format as `app.format`, the canvas format our
helper looked up for us (the JS version's `presentationFormat`).

We'll need a sampler. The JavaScript version also makes a
`postProcessRenderPassDescriptor` here that it reuses every frame but, in
wgpu, render pass descriptors borrow the texture view they render to, so
we'll just fill one out each frame when we render.

```rust
  let post_process_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
    min_filter: wgpu::FilterMode::Linear,
    mag_filter: wgpu::FilterMode::Linear,
    ..Default::default()
  });
```

Then, instead of having our original renderPass render
to the canvas, we need it to render to a separate texture.

```rust
+  let mut render_target: Option<wgpu::Texture> = None;
+  let mut post_process_bind_group: Option<wgpu::BindGroup> = None;

  let mut then = 0.0;
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let now = frame.time;
    let delta_time = (now - then) as f32;
    then = now;

+    // If we don't have a render target or it doesn't match the canvas
+    // size, make a new one (setupPostProcess in the JS version).
+    if render_target
+      .as_ref()
+      .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
+    {
+      if let Some(t) = render_target.take() {
+        t.destroy();
+      }
+      let texture = frame.device.create_texture(&wgpu::TextureDescriptor {
+        label: None,
+        size: wgpu::Extent3d {
+          width: frame.width,
+          height: frame.height,
+          depth_or_array_layers: 1,
+        },
+        format: wgpu::TextureFormat::Rgba8Unorm,
+        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
+          | wgpu::TextureUsages::TEXTURE_BINDING,
+        mip_level_count: 1,
+        sample_count: 1,
+        dimension: wgpu::TextureDimension::D2,
+        view_formats: &[],
+      });
+      post_process_bind_group =
+        Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
+          label: None,
+          layout: &post_process_pipeline.get_bind_group_layout(0),
+          entries: &[
+            wgpu::BindGroupEntry {
+              binding: 0,
+              resource: wgpu::BindingResource::TextureView(
+                &texture.create_view(&Default::default()),
+              ),
+            },
+            wgpu::BindGroupEntry {
+              binding: 1,
+              resource: wgpu::BindingResource::Sampler(&post_process_sampler),
+            },
+          ],
+        }));
+      render_target = Some(texture);
+    }
+    let render_target_view = render_target
+      .as_ref()
+      .unwrap()
+      .create_view(&Default::default());

    ...

    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
-          view: frame.view,
+          view: &render_target_view,
          resolve_target: None,
          ...
```

Above, at the start of each frame, we check if the size of our "renderTarget"
texture is the same size as the canvas. If not, we create a new texture the
same size. This is the JavaScript version's `setupPostProcess(canvasTexture)`
function.

While we're at it we also make the bind group that will let us pass the
renderTarget texture and the sampler to the post processing shader. We make
it here, rather than at init time, because it references the renderTarget's
view and so has to be remade whenever the renderTarget is remade.

We then have our original render pass render to this renderTarget texture
instead of the canvas (`frame.view`).

Since our old pipeline will render to this texture, the pipeline's target
format needs to match the texture's format. This is where the JavaScript
version switches its target from `presentationFormat` to `'rgba8unorm'`.

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("per vertex color"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      ...
    },
    fragment: Some(wgpu::FragmentState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
    ...
  });
```

We'll leave it reading `app.format`, which matches the `rgba8unorm` render
target in the configurations we run in.

These change alone would make it start rendering the original scene to this
render target texture but we still need to draw something to the canvas
or we won't see anything so lets do that. The JavaScript version wraps this
in a `postProcess(encoder, srcTexture, dstTexture)` function. In Rust it's
simplest to add a second render pass at the end of our frame callback.

```rust
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let now = frame.time;
    let delta_time = (now - then) as f32;
    then = now;

    ...

      pass.draw(0..num_vertices, 0..num_objects as u32);
    } // the render pass ends when it drops here

+    // post process the render target to the canvas
+    {
+      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
+        label: Some("post process render pass"),
+        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
+          view: frame.view,
+          resolve_target: None,
+          ops: wgpu::Operations {
+            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
+            store: wgpu::StoreOp::Store,
+          },
+          depth_slice: None,
+        })],
+        ..Default::default()
+      });
+      pass.set_pipeline(&post_process_pipeline);
+      pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
+      pass.draw(0..3, 0..1);
+    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

The only other tweak let's make. Let's get rid of the object count setting
since it's not relevant to post processing. The settings panel lives in the
example's page JavaScript so that's where we remove it.

```js
-const settings = {
-  numObjects: 100,
-};
-
-const gui = new GUI();
-gui.add(settings, 'numObjects', 0, kNumObjects, 1)
-  .onChange(v => wasm.set_setting_num('numObjects', v));
```

We could have gotten rid of the setting completely but it requires
edits in several different places and so let's leave it for now. Our Rust
code still reads it; we'll set the default to 200 just to fill the image.

```rust
    // read the settings the GUI on the page sets
-    let num_objects = wgpu_fun::setting_f64("numObjects", 100.0) as usize;
+    let num_objects = wgpu_fun::setting_f64("numObjects", 200.0) as usize;
```

If we run this there's not visible difference from our original.

{{{example url="../webgpu-post-processing-step-01.html"}}}

The difference is we're rendering to the renderTarget texture
and then rendering that texture to the canvas so now we can start
applying some effects.

The most obvious effect of an old CRT is old CRTs have visible scanlines.
This is because the way the image was projected was by using magnets
to direct a beam across the screen in a pattern of horizontal lines.

We can get a similar effect just by generating a pattern of light
and dark using a sine wave and taking the absolute value.

<div class="webgpu_center">
  <div style="width: 100%;"><img class="ddnobg" src="resources/sinewave-40.svg"></div>
  <div lass="caption">sin(x)</div>
</div>
<div class="webgpu_center">
   <div style="width: 100%;"><img class="ddnobg" src="resources/abs-sinewave-40.svg"></div>
   <div class="caption">abs(sin(x))</div>
</div>
<div class="webgpu_center">
   <div style="width: 100%;"><div data-diagram="sine" style="aspect-ratio: 981 / 50; width: 100%;"></div></div>
   <div class="caption">abs(sin(x)) as gray scale color</div>
</div>


Let's add that to the code. First let's edit the shader to apply this sine wave.

```rust
  let post_process_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
        return vsOutput;
      }

+      struct Uniforms {
+        effectAmount: f32,
+        bandMult: f32,
+      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
+      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
+        let banding = abs(sin(fsInput.position.y * uni.bandMult));
+        let effect = mix(1.0, banding, uni.effectAmount);

        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
-        return vec4f(color);
+        return vec4f(color.rgb * effect, color.a);
      }
    "#.into()),
  });
```

Our sine wave is based on `fsInput.position.y` which is the y coordinate of the pixel being
written to. In other words, for each scanline starting at 0 it will go 0.5, 1.5, 2.5, 3.5, etc....
`bendMult` will let us adjust the size of the bands and `effectAmount` will let us turn
the effect on and off so we can compare effect to no effect.

To use the new shader we need up a uniform buffer.

```rust
  let post_process_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: 8,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
```

We need to add it to our bind group

```rust
      post_process_bind_group =
        Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
          label: None,
          layout: &post_process_pipeline.get_bind_group_layout(0),
          entries: &[
            wgpu::BindGroupEntry {
              binding: 0,
              resource: wgpu::BindingResource::TextureView(
                &texture.create_view(&Default::default()),
              ),
            },
            wgpu::BindGroupEntry {
              binding: 1,
              resource: wgpu::BindingResource::Sampler(&post_process_sampler),
            },
+            wgpu::BindGroupEntry {
+              binding: 2,
+              resource: post_process_uniform_buffer.as_entire_binding(),
+            },
          ],
        }));
```

And, we need to add some settings. Like our other examples with a GUI,
the settings panel is in the example's page JavaScript; its onChange
handlers push values into the wasm module.

```js
+const settings = {
+  affectAmount: 1,
+  bandMult: 1,
+};
+
+const gui = new GUI();
+gui.add(settings, 'affectAmount', 0, 1)
+   .onChange(v => wasm.set_setting_num('affectAmount', v));
+gui.add(settings, 'bandMult', 0.01, 2.0)
+   .onChange(v => wasm.set_setting_num('bandMult', v));
```

and we need to read those settings in our frame code, with defaults that
match the page's initial `settings`, and upload them to the uniform buffer

```rust
+    // read the settings the GUI on the page sets
+    let affect_amount = wgpu_fun::setting_f64("affectAmount", 1.0) as f32;
+    let band_mult = wgpu_fun::setting_f64("bandMult", 1.0) as f32;
+    frame.queue.write_buffer(
+      &post_process_uniform_buffer,
+      0,
+      bytemuck::cast_slice(&[affect_amount, band_mult]),
+    );

    // post process the render target to the canvas
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("post process render pass"),
        ...
      });
      pass.set_pipeline(&post_process_pipeline);
      pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
      pass.draw(0..3, 0..1);
    }
```

And that gives us a CRT like scanline effect.

{{{example url="../webgpu-post-processing-step-02.html"}}}

CRTs, like LCDs, split the image into red, green, and blue areas. 
On CRTs those areas were generally larger than most LCDs today so
sometimes this stuck out. Let's add something to approximate that effect.

First let's change the shader

```rust
  let post_process_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32,
      ) -> VSOutput {
        var pos = array(
          vec2f(-1.0, -1.0),
          vec2f(-1.0,  3.0),
          vec2f( 3.0, -1.0),
        );

        var vsOutput: VSOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
        return vsOutput;
      }

      struct Uniforms {
        effectAmount: f32,
        bandMult: f32,
+        cellMult: f32,
+        cellBright: f32,
      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
        let banding = abs(sin(fsInput.position.y * uni.bandMult));

+        let cellNdx = u32(fsInput.position.x * uni.cellMult) % 3;
+        var cellColor = vec3f(0);
+        cellColor[cellNdx] = 1;
+        let cMult = cellColors[cellNdx] + uni.cellBright;

-        let effect = mix(1.0, banding, uni.effectAmount);
+        let effect = mix(vec3f(1), banding * cMult, uni.effectAmount);
        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
        return vec4f(color.rgb * effect, 1);
      }
    "#.into()),
  });
```

Above we're using `fsInput.position.x` which is the x coordinate of the
pixel being written to. By multiplying by `cellMult` we can choose a cell
size. We convert to an integer and modulo 3. This gives us a number, 0, 1, or 2
which we use to set the red, green, or blue channel of `cellColor` to 1.

We add in `cellBright` as an adjustment and then multiply both the old banding
and the new effect together. `effect` changed from an `f32` to a `vec3f` so it
can affect each channel independently.

Back in Rust we need to adjust the size of the uniform buffer

```rust
  let post_process_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
-    size: 8,
+    size: 16,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
```

And add some settings to the GUI on the page

```js
const settings = {
  affectAmount: 1,
  bandMult: 1,
+  cellMult: 0.5,
+  cellBright: 1,
};

const gui = new GUI();
gui.add(settings, 'affectAmount', 0, 1)
   .onChange(v => wasm.set_setting_num('affectAmount', v));
gui.add(settings, 'bandMult', 0.01, 2.0)
   .onChange(v => wasm.set_setting_num('bandMult', v));
+gui.add(settings, 'cellMult', 0, 1)
+   .onChange(v => wasm.set_setting_num('cellMult', v));
+gui.add(settings, 'cellBright', 0, 2)
+   .onChange(v => wasm.set_setting_num('cellBright', v));
```

and upload the new settings

```rust
    // read the settings the GUI on the page sets
    let affect_amount = wgpu_fun::setting_f64("affectAmount", 1.0) as f32;
    let band_mult = wgpu_fun::setting_f64("bandMult", 1.0) as f32;
+    let cell_mult = wgpu_fun::setting_f64("cellMult", 0.5) as f32;
+    let cell_bright = wgpu_fun::setting_f64("cellBright", 1.0) as f32;
    frame.queue.write_buffer(
      &post_process_uniform_buffer,
      0,
-      bytemuck::cast_slice(&[affect_amount, band_mult]),
+      bytemuck::cast_slice(&[affect_amount, band_mult, cell_mult, cell_bright]),
    );
```

And now we have a CRT color element *like* effect.

{{{example url="../webgpu-post-processing-step-03.html"}}}

The effects above are not meant to be perfect representations of how a CRT works.
Rather they were just meant to hint at looking like a CRT and be hopefully easy to
to understand. You can find fancier techniques all over the web.

## <a id="compute"></a> Using a Compute Shader

The topic comes up, could we use a compute shader for this, and, maybe
more importantly, should we? Let's cover "can we first".

We covered using a compute shader to render to a texture in
[the article on storage textures](webgpu-storage-textures.html).

To convert our code to use a compute shader we need to add
the `STORAGE_BINDING` usage to the canvas texture which, from
[the afore mentioned article](webgpu-storage-textures.html) requires
checking we can and choosing a texture format that supports it.
`App::new_with_features` requests the `bgra8unorm-storage` feature
*if the adapter supports it*, mirroring the JS
`adapter.features.has('bgra8unorm-storage')` check, and `app.usage`
is passed through to the surface configuration (the JS
`context.configure`'s `usage`). Then we check the combination we
ended up with.

```rust
-  let mut app = App::new("WebGPU Post Processing - Step 03 - rgb elements").await;
+  let mut app = App::new_with_features(
+    "WebGPU Post Processing - Step 03 - compute",
+    wgpu::Features::BGRA8UNORM_STORAGE,
+  ).await;
  app.auto_resize = true;
+  app.usage = wgpu::TextureUsages::RENDER_ATTACHMENT
+    | wgpu::TextureUsages::TEXTURE_BINDING
+    | wgpu::TextureUsages::STORAGE_BINDING;
+  if app.format == wgpu::TextureFormat::Bgra8Unorm
+    && !app.device.features().contains(wgpu::Features::BGRA8UNORM_STORAGE)
+  {
+    panic!("bgra8unorm-storage is not supported");
+  }
```

We need to switch our shader to write to a storage texture. The storage
texture's format has to be written in the shader itself so, like we did in
[the article on storage textures](webgpu-storage-textures.html), we splice
it into the shader string with `format!`, the way the JS version splices in
its `${presentationFormat}` template literal.

```rust
+  let format_name = match app.format {
+    wgpu::TextureFormat::Rgba8Unorm => "rgba8unorm",
+    wgpu::TextureFormat::Bgra8Unorm => "bgra8unorm",
+    f => panic!("unsupported canvas format {f:?}"),
+  };
+
-  let post_process_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
-    label: None,
-    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
+  let post_process_code = /* wgsl */ format!("
+      @group(1) @binding(0) var outTexture: texture_storage_2d<{format_name}, write>;
+") + &r#"
-      struct VSOutput {
-        @builtin(position) position: vec4f,
-        @location(0) texcoord: vec2f,
-      };
-
-      @vertex fn vs(
-        @builtin(vertex_index) vertexIndex : u32,
-      ) -> VSOutput {
-        var pos = array(
-          vec2f(-1.0, -1.0),
-          vec2f(-1.0,  3.0),
-          vec2f( 3.0, -1.0),
-        );
-
-        var vsOutput: VSOutput;
-        let xy = pos[vertexIndex];
-        vsOutput.position = vec4f(xy, 0.0, 1.0);
-        vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
-        return vsOutput;
-      }

      struct Uniforms {
        effectAmount: f32,
        bandMult: f32,
        cellMult: f32,
        cellBright: f32,
      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

-      @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
-        let banding = abs(sin(fsInput.position.y * uni.bandMult));
-
-        let cellNdx = u32(fsInput.position.x * uni.cellMult) % 3;
+      @compute @workgroup_size(1) fn cs(@builtin(global_invocation_id) gid: vec3u) {
+        let outSize = textureDimensions(outTexture);
+        let banding = abs(sin(f32(gid.y) * uni.bandMult));
+
+        let cellNdx = u32(f32(gid.x) * uni.cellMult) % 3;
        var cellColor = vec3f(0);
        cellColor[cellNdx] = 1.0;
        let cMult = cellColor + uni.cellBright;

        let effect = mix(vec3f(1), banding * cMult, uni.effectAmount);
-        let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
-        return vec4f(color.rgb * effect, color.a);
+        let uv = (vec2f(gid.xy) + 0.5) / vec2f(outSize);
+        let color = textureSampleLevel(postTexture2d, postSampler, uv, 0);
+        textureStore(outTexture, gid.xy, vec4f(color.rgb * effect, color.a));
      }
-    "#.into()),
-  });
+    "#;
+
+  let post_process_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
+    label: None,
+    source: wgpu::ShaderSource::Wgsl(post_process_code.into()),
+  });
```

Above we got rid of the vertex shader and related parts. We also no longer have `fsInput.position`
which was the coordinate of the pixel being written to. Instead we have `gid` which is
the `global_invocation_id` of an individual invocation of our compute shader. We'll use this
as our texture coordinate. It's a `vec3u` so we need to cast here and there. We also
no longer have `fsInput.texcoord` but we can get the equivalent with
`(vec2f(gid.xy) + 0.5) / vec2f(outSize)`.

We need to stop using a render pass and instead use a compute pass for our post processing.

```rust
-  let post_process_pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
-    label: None,
-    layout: None,
-    vertex: wgpu::VertexState {
-      module: &post_process_module,
-      entry_point: None,
-      compilation_options: Default::default(),
-      buffers: &[],
-    },
-    fragment: Some(wgpu::FragmentState {
-      module: &post_process_module,
-      entry_point: None,
-      compilation_options: Default::default(),
-      targets: &[Some(app.format.into())],
-    }),
-    primitive: Default::default(),
-    depth_stencil: None,
-    multisample: Default::default(),
-    multiview_mask: None,
-    cache: None,
-  });
+  let post_process_pipeline = app.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
+    label: None,
+    layout: None,
+    module: &post_process_module,
+    entry_point: None,
+    compilation_options: Default::default(),
+    cache: None,
+  });
```

```rust
    // read the settings the GUI on the page sets
    let affect_amount = wgpu_fun::setting_f64("affectAmount", 1.0) as f32;
    let band_mult = wgpu_fun::setting_f64("bandMult", 1.0) as f32;
    let cell_mult = wgpu_fun::setting_f64("cellMult", 0.5) as f32;
    let cell_bright = wgpu_fun::setting_f64("cellBright", 1.0) as f32;
    frame.queue.write_buffer(
      &post_process_uniform_buffer,
      0,
      bytemuck::cast_slice(&[affect_amount, band_mult, cell_mult, cell_bright]),
    );

-    // post process the render target to the canvas
+    // post process the render target to the canvas with a compute shader
    {
+      let out_bind_group = frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
+        label: None,
+        layout: &post_process_pipeline.get_bind_group_layout(1),
+        entries: &[wgpu::BindGroupEntry {
+          binding: 0,
+          resource: wgpu::BindingResource::TextureView(frame.view),
+        }],
+      });
+
-      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
-        label: Some("post process render pass"),
-        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
-          view: frame.view,
-          resolve_target: None,
-          ops: wgpu::Operations {
-            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
-            store: wgpu::StoreOp::Store,
-          },
-          depth_slice: None,
-        })],
-        ..Default::default()
-      });
+      let mut pass = encoder.begin_compute_pass(&Default::default());
      pass.set_pipeline(&post_process_pipeline);
      pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
+      pass.set_bind_group(1, &out_bind_group, &[]);
-      pass.draw(0..3, 0..1);
+      pass.dispatch_workgroups(frame.width, frame.height, 1);
    }
```

That works

{{{example url="../webgpu-post-processing-step-03-compute.html"}}}

Unfortunately, depending on the GPU, it's slow! We covered some of why in
[the article on optimizing compute shaders](webgpu-compute-shaders-historgram.html).
Using a workgroup size of 1 makes things easy but it's slow.

We can update to use a larger workgroup size. This requires us to skip writing
to the texture when we're out of bounds. Where the JS version splices
`${workgroupSize}` directly into `@workgroup_size(...)`, we'll splice in two
WGSL `const`s with `format!` and reference those.

```rust
+  let workgroup_size = [16u32, 16u32];
+  let [wx, wy] = workgroup_size;
  let post_process_code = /* wgsl */ format!("
      @group(1) @binding(0) var outTexture: texture_storage_2d<{format_name}, write>;
+      const workgroupSizeX = {wx};
+      const workgroupSizeY = {wy};
") + &r#"
      struct Uniforms {
        effectAmount: f32,
        bandMult: f32,
        cellMult: f32,
        cellBright: f32,
      };

      @group(0) @binding(0) var postTexture2d: texture_2d<f32>;
      @group(0) @binding(1) var postSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

-      @compute @workgroup_size(1) fn cs(@builtin(global_invocation_id) gid: vec3u) {
+      @compute @workgroup_size(workgroupSizeX, workgroupSizeY) fn cs(@builtin(global_invocation_id) gid: vec3u) {
        let outSize = textureDimensions(outTexture);
+        if (gid.x >= outSize.x || gid.y >= outSize.y) {
+          return;
+        }
        let banding = abs(sin(f32(gid.y) * uni.bandMult));

        let cellNdx = u32(f32(gid.x) * uni.cellMult) % 3;
        var cellColor = vec3f(0);
        cellColor[cellNdx] = 1.0;
        let cMult = cellColor + uni.cellBright;

        let effect = mix(vec3f(1), banding * cMult, uni.effectAmount);
        let uv = (vec2f(gid.xy) + 0.5) / vec2f(outSize);
        let color = textureSampleLevel(postTexture2d, postSampler, uv, 0);
        textureStore(outTexture, gid.xy, vec4f(color.rgb * effect, color.a));
      }
    "#;
```

And then we need to dispatch less workgroups. `div_ceil` is the Rust
equivalent of the JS `Math.ceil(a / b)` pattern.

```rust
      let mut pass = encoder.begin_compute_pass(&Default::default());
      pass.set_pipeline(&post_process_pipeline);
      pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
      pass.set_bind_group(1, &out_bind_group, &[]);
-      pass.dispatch_workgroups(frame.width, frame.height, 1);
+      pass.dispatch_workgroups(
+        frame.width.div_ceil(workgroup_size[0]),
+        frame.height.div_ceil(workgroup_size[1]),
+        1,
+      );
```

This works

{{{example url="../webgpu-post-processing-step-03-compute-workgroups.html"}}}

This is much faster! But, unfortunately, on some GPUs it is still slower than using a render pass.

<div class="webgpu_center data-table">
  <table>
    <thead>
      <tr><th>GPU</th><th>Compute pass time vs<br>Render pass time<br>(higher is worse)</th></tr>
    </thead>
    <tbody>
      <tr><td>M1 Mac                 </td><td>1x</td></tr>
      <tr><td>AMD Radeon Pro 5300M   </td><td>1x</td></tr>
      <tr><td>AMD Radeon Pro WX 32000</td><td>1.3x</td></tr>
      <tr><td>Intel UHD Graphics 630 </td><td>1.7x</td></tr>
      <tr><td>NVidia 2070 Super      </td><td>2x</td></tr>
    </tbody>
  </table>
</div>

Going into how to make it faster is too big of a topic for this particular article.
Referencing [the article on optimizing compute shaders](webgpu-compute-shaders-historgram.html),
the same rules apply. Unfortunately none of them are really relevant to this example.
If the post processing you're trying to do could benefit from shared workgroup memory
then maybe using a compute shader would be beneficial. Access patterns might be relevant
too to try to make sure the GPU isn't getting lots of cache misses. Yet another might
be taking advantage of [subgroups](webgpu-subgroups.html).

For now, it's recommended you try different techniques and checking their timing.
Or, stick with render passes unless the algorithm you're implementing could truly
benefit from the shared data of workgroups and or subgroups. GPUs have been rendering
to textures for much longer than they've been running compute shaders so many things
about that process are highly optimized.

---

This article introduced the concept of *post processing*.
In the next article we'll cover some
[common post processing image adjustments](webgpu-image-adjustments.html).

<!-- keep this at the bottom of the article -->
<link href="webgpu-post-processing.css" rel="stylesheet">
<script type="module" src="webgpu-post-processing.js"></script>
