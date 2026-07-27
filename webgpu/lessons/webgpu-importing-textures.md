Title: WebGPU Loading Images into Textures
Description: How to load an Image/Canvas/Video into a texture
TOC: Loading Images

This article is one in a series of the various ways to provide data
to a shader. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="passing-data.hanson"}}}

We covered some basics about using textures [in the previous article](webgpu-textures.html).
In this article we'll cover loading an image into a texture
as well as generating mipmaps on the GPU.

In the previous article we'd created a texture by calling `device.create_texture` and then
put data in the texture by calling `queue.write_texture`. To load an image we need to
get the image's pixels, and we'll keep using `write_texture` to upload them.

(In the JavaScript WebGPU API there's a browser-specific fast path,
`device.queue.copyExternalImageToTexture`, which copies an `ImageBitmap` the
browser decoded straight into a texture. wgpu exposes it on its web backend
as `queue.copy_external_image_to_texture`, but it doesn't exist natively — a
window system has no browser to decode images for us. So our examples decode
the image in Rust, which works identically in the browser and natively.)

Let's take [the magFilter example from the previous article](webgpu-textures.html#a-mag-filter) and change it to load a few images.

First we need some code to fetch and decode an image. That's what
`wgpu_fun::load_image` does:

```rust
pub struct ImageData {
    pub data: Vec<u8>,  // tightly packed RGBA8 pixels
    pub width: u32,
    pub height: u32,
}

pub async fn load_image(url: &str) -> ImageData {
    let bytes = load_binary(url).await;  // browser: fetch(); native: read the file
    let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let (width, height) = img.dimensions();
    ImageData { data: img.into_raw(), width, height }
}
```

The code above fetches the image file's bytes (an HTTP `fetch` in the
browser, a file read natively) and decodes them with the
[`image` crate](https://docs.rs/image) into RGBA8 pixels. Decoding
ourselves also means no browser color space conversion is applied. That
matters because in WebGPU we might load an image that is a normal map or a
height map or something that is not color data. In those cases we definitely
don't want anything to muck with the data in the image.

Now that we have code to create an `ImageBitmap` let's load one and create a texture of the same size.

We'll load this image

<div class="webgpu_center"><img src="../resources/images/f-texture.png"></div>

I was taught once that a texture with an `F` in it is a good example texture because we can instantly
see its orientation.

<div class="webgpu_center"><img src="resources/f-orientation.svg"></div>


```rust
-  let texture = app.device.create_texture(&wgpu::TextureDescriptor {
-    label: Some("yellow F on red"),
-    size: wgpu::Extent3d {
-      width: K_TEXTURE_WIDTH,
-      height: K_TEXTURE_HEIGHT,
-      depth_or_array_layers: 1,
-    },
-    format: wgpu::TextureFormat::Rgba8Unorm,
-    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
-    ...
-  });
+  let url = "resources/images/f-texture.png";
+  let source = wgpu_fun::load_image(url).await;
+  let texture = app.device.create_texture(&wgpu::TextureDescriptor {
+    label: Some(url),
+    format: wgpu::TextureFormat::Rgba8Unorm,
+    size: wgpu::Extent3d {
+      width: source.width,
+      height: source.height,
+      depth_or_array_layers: 1,
+    },
+    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
+    mip_level_count: 1,
+    sample_count: 1,
+    dimension: wgpu::TextureDimension::D2,
+    view_formats: &[],
+  });
```

So then we can copy the pixels to the texture. The JS version's
`copyExternalImageToTexture` has a handy `flipY: true` option; with
`write_texture` we flip the rows of pixels ourselves:

```rust
+  // wgpu's write_texture has no flipY option like the browser's
+  // copyExternalImageToTexture, so we flip the rows of pixels ourselves.
+  fn flip_image_y(source: &ImageData) -> Vec<u8> {
+    let bytes_per_row = (source.width * 4) as usize;
+    source.data.chunks(bytes_per_row).rev().flatten().copied().collect()
+  }

  app.queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture: &texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
-    bytemuck::cast_slice(&texture_data),
+    &flip_image_y(&source), // flipY: true
    wgpu::TexelCopyBufferLayout {
      offset: 0,
-      bytes_per_row: Some(K_TEXTURE_WIDTH * 4),
+      bytes_per_row: Some(source.width * 4),
      rows_per_image: None,
    },
-    wgpu::Extent3d { width: K_TEXTURE_WIDTH, height: K_TEXTURE_HEIGHT, depth_or_array_layers: 1 },
+    wgpu::Extent3d { width: source.width, height: source.height, depth_or_array_layers: 1 },
  );
```

And that works!

{{{example url="../webgpu-simple-textured-quad-import-no-mips.html"}}}

## <a id="a-generating-mips-on-the-gpu"></a>Generating mips on the GPU

In [the previous article we also generated a mipmap](webgpu-textures.html#a-mipmap-filter)
but in that case we had easy access to the image data. When loading an image, we
could draw that image into a 2D canvas, the call `getImageData` to get the data, and
finally generate mips and upload. That would be pretty slow. It would also potentially
be lossy since how canvas 2D renders is intentionally implementation dependant.

When we generated mip levels we did a bilinear interpolation which is exactly what
the GPU does with `minFilter: linear`. We can use that feature to generate mip levels
on the GPU

Let's modify the [mipmapFilter example from the previous article](webgpu-textures.html#a-mipmap-filter)
to load images and generate mips using the GPU

First, let's change the code that creates the texture to create mip levels. We need to know how many
to create which we can calculate like this

```rust
  fn num_mip_levels(sizes: &[u32]) -> u32 {
    let max_size = *sizes.iter().max().unwrap();
    1 + (max_size as f32).log2() as u32
  }
```

We can call that with 1 or more numbers and it will return the number of mips needed, so for example
`num_mip_levels(&[123, 456])` returns `9`.

> * level 0: 123, 456
> * level 1: 61, 228
> * level 2: 30, 114
> * level 3: 15, 57
> * level 4: 7, 28
> * level 5: 3, 14
> * level 6: 1, 7
> * level 7: 1, 3
> * level 8: 1, 1
> 
> 9 mip levels

`log2` tells us the power of 2 we need to make our number.
In other words, `log2(8) = 3` because 2<sup>3</sup> = 8. Another way to say the same thing is, `log2` tells us how
many times can we divide this number by 2. 

> ```
> log2(8)
>      8 / 2 = 4
>              4 / 2 = 2
>                      2 / 2 = 1
> ```

So we can divide 8 by 2 three times. That's exactly what we need to compute how many mip levels to make.
It's `log2(largest_size) + 1`. 1 for the original size mip level 0

So, we can now create the right number of mip levels

```rust
  let texture = app.device.create_texture(&wgpu::TextureDescriptor {
    label: Some(url),
    format: wgpu::TextureFormat::Rgba8Unorm,
    mip_level_count: num_mip_levels(&[source.width, source.height]),
    size: wgpu::Extent3d {
      width: source.width,
      height: source.height,
      depth_or_array_layers: 1,
    },
    usage: wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_DST
        | wgpu::TextureUsages::RENDER_ATTACHMENT,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    view_formats: &[],
  });
```

Note that rendering mip levels on the GPU requires the
`RENDER_ATTACHMENT` usage flag.

To generate the next mip level, we'll draw a textured quad, just like we've been doing, from the
existing mip level, to the next level, with `minFilter: linear`. 

Here's the code

```rust
  fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    // The JS version lazily caches the module/sampler/pipelines in a
    // closure; we cache them in thread locals.
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
      static CACHE: RefCell<Option<(wgpu::ShaderModule, wgpu::Sampler)>> =
          const { RefCell::new(None) };
      static PIPELINE_BY_FORMAT:
          RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>> =
          RefCell::new(HashMap::new());
    }

    CACHE.with(|cache| {
      let mut cache = cache.borrow_mut();
      let (module, sampler) = cache.get_or_insert_with(|| {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
          label: Some("textured quad shaders for mip level generation"),
          source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
            struct VSOutput {
              @builtin(position) position: vec4f,
              @location(0) texcoord: vec2f,
            };

            @vertex fn vs(
              @builtin(vertex_index) vertexIndex : u32
            ) -> VSOutput {
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

              var vsOutput: VSOutput;
              let xy = pos[vertexIndex];
              vsOutput.position = vec4f(xy * 2.0 - 1.0, 0.0, 1.0);
              vsOutput.texcoord = vec2f(xy.x, 1.0 - xy.y);
              return vsOutput;
            }

            @group(0) @binding(0) var ourSampler: sampler;
            @group(0) @binding(1) var ourTexture: texture_2d<f32>;

            @fragment fn fs(fsInput: VSOutput) -> @location(0) vec4f {
              return textureSample(ourTexture, ourSampler, fsInput.texcoord);
            }
          "#.into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
          min_filter: wgpu::FilterMode::Linear,
          ..Default::default()
        });
        (module, sampler)
      });

      PIPELINE_BY_FORMAT.with(|pipelines| {
        let mut pipelines = pipelines.borrow_mut();
        let pipeline = pipelines.entry(texture.format()).or_insert_with(|| {
          device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mip level generator pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
              module,
              entry_point: None,
              compilation_options: Default::default(),
              buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
              module,
              entry_point: None,
              compilation_options: Default::default(),
              targets: &[Some(texture.format().into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
          })
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
          label: Some("mip gen encoder"),
        });

        for base_mip_level in 1..texture.mip_level_count() {
          let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
              wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(sampler),
              },
              wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture.create_view(
                  &wgpu::TextureViewDescriptor {
                    base_mip_level: base_mip_level - 1,
                    mip_level_count: Some(1),
                    ..Default::default()
                  },
                )),
              },
            ],
          });

          let view = texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level,
            mip_level_count: Some(1),
            ..Default::default()
          });
          {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("our basic canvas renderPass"),
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                  load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                  store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
              })],
              ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..6, 0..1); // call our vertex shader 6 times
          }
        }

        let command_buffer = encoder.finish();
        queue.submit([command_buffer]);
      });
    });
  }
```

The code above looks long but it's almost the exact same code we've been using in our examples with textures so far.
What's changed

* We hold on to 3 cached things: `module`, `sampler`, `PIPELINE_BY_FORMAT`.
  The JS version does this with a closure; in Rust we use thread locals.
  For `module` and `sampler` we check if they have not been set
  (`get_or_insert_with`) and if not, we create a `ShaderModule`
  and `Sampler` which we can hold on to and use in the future.

* We have a pair of shaders that are almost exactly the same as all the examples so far. 
  The only difference is this part

  ```wgsl
  -  vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
  -  vsOutput.texcoord = xy * vec2f(1, 50);
  +  vsOutput.position = vec4f(xy * 2.0 - 1.0, 0.0, 1.0);
  +  vsOutput.texcoord = vec2f(xy.x, 1.0 - xy.y);
  ```

  The hard coded quad position data we have in shader goes from 0.0 to 1.0 and so, as is, would only
  cover the top right quarter texture we're drawing to, just as it does in the examples. We need it to cover the entire
  area so by multiplying by 2 and subtracting 1 we get a quad that goes from -1,-1 to +1,+1.

  We also flip the Y texture coordinate. This is because when drawing to the texture +1, +1 is at the top right
  but we want the top right of the texture we are sampling to be there. The top right of the sampled texture is +1, 0

* We have a `HashMap`, `PIPELINE_BY_FORMAT`, which we use as a map of pipelines to texture formats.
  This is because a pipeline needs to know the format to use.

* We check if we already have a pipeline for a particular format and if not create one
  
  ```rust
      let pipeline = pipelines.entry(texture.format()).or_insert_with(|| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("mip level generator pipeline"),
          layout: None,
          vertex: wgpu::VertexState {
            module,
            ...
          },
          fragment: Some(wgpu::FragmentState {
            module,
  +          targets: &[Some(texture.format().into())],
            ...
          }),
          ...
        })
      });
  ```

  The only major difference here is `targets` is set from the texture's format,
  not from the `app.format` we use when rendering to the canvas

* We finally use some parameters to `texture.createView`

  This is the first time we've passed parameters to `create_view`.
  (In wgpu, unlike the JS API, a bind group or color attachment always takes
  a `TextureView` — there's no shortcut of passing the texture directly, so
  we've been calling `texture.create_view(&Default::default())` all along,
  which means "access the entire texture".)
  With parameters, `create_view` lets you select a subset of the texture.
  In this case we use `create_view` to select the mip level we want to read from. We set this in
  the bind group. And, we use `create_view` again, to select which mip level we want
  to render to in the render pass descriptor.

  We loop over each mip level that we need to generate. 
  We create a bind group for the last mip with data in it
  and we set the renderPassDescriptor to draw to the current mip level. Then we encode
  a renderPass for that specific mip level. When we're done. All the mips will have
  been filled out.

  ```rust
      for base_mip_level in 1..texture.mip_level_count() {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
          layout: &pipeline.get_bind_group_layout(0),
          entries: &[
            wgpu::BindGroupEntry {
              binding: 0,
              resource: wgpu::BindingResource::Sampler(sampler),
            },
  +          wgpu::BindGroupEntry {
  +            binding: 1,
  +            resource: wgpu::BindingResource::TextureView(&texture.create_view(
  +              &wgpu::TextureViewDescriptor {
  +                base_mip_level: base_mip_level - 1,
  +                mip_level_count: Some(1),
  +                ..Default::default()
  +              },
  +            )),
  +          },
          ],
          ...
        });

  +      let view = texture.create_view(&wgpu::TextureViewDescriptor {
  +        base_mip_level,
  +        mip_level_count: Some(1),
  +        ..Default::default()
  +      });
        {
          let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("our basic canvas renderPass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
  +            view: &view,
              ...
            })],
            ..Default::default()
          });
          pass.set_pipeline(pipeline);
          pass.set_bind_group(0, &bind_group, &[]);
          pass.draw(0..6, 0..1); // call our vertex shader 6 times
        }
      }

      let command_buffer = encoder.finish();
      queue.submit([command_buffer]);
  ```

> Note: This function only handles 2d textures.
> [The article on cubemaps](webgpu-cube-maps.html#a-texture-helpers)
> covers how to expand this function to handle 2d-array textures and
> cube maps.

## <a id="a-texture-helpers"></a> Simple Image Loading Functions

Let's create some support functions make it simple load an image
into a texture and generate mips

Here's a function that updates the first mip level and optionally flips the image.
If the image has mip levels then we generate them.

```rust
  fn copy_source_to_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    source: &ImageData,
    flip_y: bool,
  ) {
    let data = if flip_y { flip_image_y(source) } else { source.data.clone() };
    queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &data,
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(source.width * 4),
        rows_per_image: None,
      },
      wgpu::Extent3d {
        width: source.width,
        height: source.height,
        depth_or_array_layers: 1,
      },
    );

    if texture.mip_level_count() > 1 {
      generate_mips(device, queue, texture);
    }
  }
```

<a id="a-create-texture-from-source"></a>Here's a function that given a source (an `ImageData`) will
create a texture of the matching size and then call the previous function
to fill it in with the data

```rust
  fn create_texture_from_source(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    source: &ImageData,
    mips: bool,
    flip_y: bool,
  ) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      format: wgpu::TextureFormat::Rgba8Unorm,
*      mip_level_count: if mips {
*        num_mip_levels(&[source.width, source.height])
*      } else {
*        1
*      },
      size: wgpu::Extent3d {
        width: source.width,
        height: source.height,
        depth_or_array_layers: 1,
      },
      usage: wgpu::TextureUsages::TEXTURE_BINDING
          | wgpu::TextureUsages::COPY_DST
          | wgpu::TextureUsages::RENDER_ATTACHMENT,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      view_formats: &[],
    });
    copy_source_to_texture(device, queue, &texture, source, flip_y);
    texture
  }
```

and here's a function that given a url will load the url with
`wgpu_fun::load_image` and call the previous function to create a texture
and fill it with the contents of the image.

```rust
  async fn create_texture_from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    url: &str,
    mips: bool,
    flip_y: bool,
  ) -> wgpu::Texture {
    let source = wgpu_fun::load_image(url).await;
    create_texture_from_source(device, queue, &source, mips, flip_y)
  }
```

With those setup, the only major change to the [mipmapFilter sample](webgpu-textures.html#a-mipmap-filter)
is this

```rust
-  let textures = [
-    create_texture_with_mips(&app, &create_blended_mipmap(), "blended"),
-    create_texture_with_mips(&app, &create_checked_mipmap(), "checker"),
-  ];
+  let textures = [
+    create_texture_from_image(&app.device, &app.queue,
+        "resources/images/f-texture.png", true, false).await,
+    create_texture_from_image(&app.device, &app.queue,
+        "resources/images/coins.jpg", true, false).await,
+    create_texture_from_image(&app.device, &app.queue,
+        "resources/images/Granite_paving_tileable_512x512.jpeg", true, false).await,
+  ];
```

The code above loads the F texture from above as well as these 2 tiling textures

<div class="webgpu_center side-by-side">
  <div class="separate">
    <img src="../resources/images/coins.jpg">
    <div class="copyright">
      <a href="https://renderman.pixar.com/pixar-one-thirty">CC-BY: Pixar</a>
    </div>
  </div>
  <div class="separate">
    <img src="../resources/images/Granite_paving_tileable_512x512.jpeg">
    <div class="copyright">
       <a href="https://commons.wikimedia.org/wiki/File:Granite_paving_tileable_2048x2048.jpg">CC-BY-SA: Coyau</a>
    </div>
  </div>
</div>

And here it is

{{{example url="../webgpu-simple-textured-quad-import.html"}}}

## <a id="a-loading-canvas"></a> Loading Canvas

In the browser, `copyExternalImageToTexture` takes other *sources*. One is an
`HTMLCanvasElement`. You can draw things in a 2d canvas, and then get the
result in a texture in WebGPU. Of course you can use WebGPU to draw to a
texture and use that texture you just drew to in something else you render.
In fact we just did that, rendering to a mip level and then using that mip
level a texture attachment to render to the next mip level.

But, sometimes using 2d canvas can make certain things easy. The 2d canvas
has a relatively high level API.

The 2d canvas only exists in the browser though, so for the Rust version
the more general lesson is: *any pixels you can produce, you can upload
every frame*. We'll first look at the browser's 2d canvas animation (this
first example is JavaScript, exactly as on the JS site), and then reproduce
it by computing the same pixels in Rust so it also runs natively.

So, first let's make some kind of canvas animation.

```js
const size = 256;
const half = size / 2;

const ctx = document.createElement('canvas').getContext('2d');
ctx.canvas.width = size;
ctx.canvas.height = size;

const hsl = (h, s, l) => `hsl(${h * 360 | 0}, ${s * 100}%, ${l * 100 | 0}%)`;

function update2DCanvas(time) {
  time *= 0.0001;
  ctx.clearRect(0, 0, size, size);
  ctx.save();
  ctx.translate(half, half);
  const num = 20;
  for (let i = 0; i < num; ++i) {
    ctx.fillStyle = hsl(i / num * 0.2 + time * 0.1, 1, i % 2 * 0.5);
    ctx.fillRect(-half, -half, size, size);
    ctx.rotate(time * 0.5);
    ctx.scale(0.85, 0.85);
    ctx.translate(size / 16, 0);
  }
  ctx.restore();
}

function render(time) {
  update2DCanvas(time);
  requestAnimationFrame(render);
}
requestAnimationFrame(render);
```

{{{example url="../canvas-2d-animation.html"}}}

To get the same animation into our Rust example we reproduce the drawing on
the CPU: same 20 nested squares, each rotated, scaled 0.85x and offset from
the previous, with the same cycling hues. Instead of canvas 2d transforms we
keep a small affine transform of our own and rasterize each square by
inverse-transforming pixels (see `update_2d_canvas` in
[the example's source](https://github.com/yesnocancel/webgpufundamentals-rust/blob/main/rust/examples/src/bin/webgpu-simple-textured-quad-import-canvas.rs)):

```rust
+  const SIZE: usize = 256;
+  let mut pixels = vec![0u8; SIZE * SIZE * 4];

  // a texture with mips, sized to our animation
+  let texture = /* create_texture like create_texture_from_source, SIZE x SIZE */;
```

Then we switch to `RenderMode::Continuous`, update the pixels, and
upload them to WebGPU every frame

```rust
-  app.run(RenderMode::Once, move |frame: &Frame| {
+  app.run(RenderMode::Continuous, move |frame: &Frame| {
+    let time_ms = frame.time * 1000.0;
+    update_2d_canvas(&mut pixels, time_ms);
+    copy_source_to_texture(frame.device, frame.queue, &texture, &pixels);

     ...
```

With that we're able to upload our animation AND generate mip levels for it,
every frame

{{{example url="../webgpu-simple-textured-quad-import-canvas.html"}}}

## <a id="a-loading-video"></a> Loading Video

Video decoding is provided by the browser, so this section — and its example —
is about the browser path and the code shown is the JavaScript version (the
example below runs the original JavaScript). On the wasm build of wgpu the
same approach is available through `queue.copy_external_image_to_texture`,
which accepts browser video sources; there is no portable native
equivalent short of shipping a video decoder, which is beyond this lesson.

In the browser we can create a `<video>` element and pass
it to the same functions we passed the canvas to in the previous example and it should
just work with minor adjustments

Here's a video

<div class="webgpu_center">
  <div>
     <video muted controls src="../resources/videos/Golden_retriever_swimming_the_doggy_paddle-360-no-audio.webm" style="width: 720px";></video>
     <div class="copyright"><a href="https://commons.wikimedia.org/wiki/File:Golden_retriever_swimming_the_doggy_paddle.webm">CC-BY: Golden Woofs</a></div>
  </div>
</div>

`ImageBitmap` and `HTMLCanvasElement` have their width and height as `width` and `height` properties but `HTMLVideoElement` has its width and height
on `videoWidth` and `videoHeight`. So, let's update the code to handle that difference

```js
+  function getSourceSize(source) {
+    return [
+      source.videoWidth || source.width,
+      source.videoHeight || source.height,
+    ];
+  }

  function copySourceToTexture(device, texture, source, {flipY} = {}) {
    device.queue.copyExternalImageToTexture(
      { source, flipY, },
      { texture },
-      { width: source.width, height: source.height },
+      getSourceSize(source),
    );

    if (texture.mipLevelCount > 1) {
      generateMips(device, texture);
    }
  }

  function createTextureFromSource(device, source, options = {}) {
+    const size = getSourceSize(source);
    const texture = device.createTexture({
      format: 'rgba8unorm',
-      mipLevelCount: options.mips ? numMipLevels(source.width, source.height) : 1,
-      size: [source.width, source.height],
+      mipLevelCount: options.mips ? numMipLevels(...size) : 1,
+      size,
      usage: GPUTextureUsage.TEXTURE_BINDING |
             GPUTextureUsage.COPY_DST |
             GPUTextureUsage.RENDER_ATTACHMENT,
    });
    copySourceToTexture(device, texture, source, options);
    return texture;
  }
```

So then, lets setup a video element

```js
  const video = document.createElement('video');
  video.muted = true;
  video.loop = true;
  video.preload = 'auto';
  video.src = 'resources/videos/Golden_retriever_swimming_the_doggy_paddle-360-no-audio.webm';

  const texture = createTextureFromSource(device, video, {mips: true});
```

and update it at render time

```js
-  function render(time) {
-    update2DCanvas(time);
-    copySourceToTexture(device, texture, ctx.canvas);
+  function render() {
+    copySourceToTexture(device, texture, video);
```

One complication of videos is we need to wait for them to have started
playing before we pass them to WebGPU. In modern browsers we can do
this by calling `video.requestVideoFrameCallback`. It calls us each time
a new frame is available so we can use it to find out when at least
one frame is available.

For a fallback, we can wait for the time to advance and pray 🙏 because
sadly, old browsers made it hard to know when it's safe to use a video 😅

```js
+  function startPlayingAndWaitForVideo(video) {
+    return new Promise((resolve, reject) => {
+      video.addEventListener('error', reject);
+      if ('requestVideoFrameCallback' in video) {
+        video.requestVideoFrameCallback(resolve);
+      } else {
+        const timeWatcher = () => {
+          if (video.currentTime > 0) {
+            resolve();
+          } else {
+            requestAnimationFrame(timeWatcher);
+          }
+        };
+        timeWatcher();
+      }
+      video.play().catch(reject);
+    });
+  }

  const video = document.createElement('video');
  video.muted = true;
  video.loop = true;
  video.preload = 'auto';
  video.src = 'resources/videos/Golden_retriever_swimming_the_doggy_paddle-360-no-audio.webm';
+  await startPlayingAndWaitForVideo(video);

  const texture = createTextureFromSource(device, video, {mips: true});
```

Another complication is we need to wait for the user to interact with the
page before we can start the video [^autoplay]. Let's add some HTML with
a play button.

[^autoplay]: There are various ways to get a video, usually without audio,
to autoplay without having to wait for the user to interact with the page.
They seem to change over time so we won't go into solutions here.

```html
  <body>
    <canvas></canvas>
+    <div id="start">
+      <div>▶️</div>
+    </div>
  </body>
```

And some CSS to center it

```css
#start {
  position: fixed;
  left: 0;
  top: 0;
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
}
#start>div {
  font-size: 200px;
  cursor: pointer;
}
```

Then let's write a function to wait for it to be clicked and hide it.

```js
+  function waitForClick() {
+    return new Promise(resolve => {
+      window.addEventListener(
+        'click',
+        () => {
+          document.querySelector('#start').style.display = 'none';
+          resolve();
+        },
+        { once: true });
+    });
+  }

  const video = document.createElement('video');
  video.muted = true;
  video.loop = true;
  video.preload = 'auto';
  video.src = 'resources/videos/Golden_retriever_swimming_the_doggy_paddle-360-no-audio.webm';
+  await waitForClick();
  await startPlayingAndWaitForVideo(video);

  const texture = createTextureFromSource(device, video, {mips: true});
```

Let's also add a wait to pause the video

```js
  const video = document.createElement('video');
  video.muted = true;
  video.loop = true;
  video.preload = 'auto';
  video.src = 'resources/videos/pexels-anna-bondarenko-5534310 (540p).mp4'; /* webgpufundamentals: url */
  await waitForClick();
  await startPlayingAndWaitForVideo(video);

+  canvas.addEventListener('click', () => {
+    if (video.paused) {
+      video.play();
+    } else {
+      video.pause();
+    }
+  });
```

And with that we should get video in a texture

{{{example url="../webgpu-simple-textured-quad-import-video.html"}}}

One optimization we could make. We could only update the texture when 
the video has changed.

For example

```js
  const video = document.createElement('video');
  video.muted = true;
  video.loop = true;
  video.preload = 'auto';
  video.src = 'resources/videos/Golden_retriever_swimming_the_doggy_paddle-360-no-audio.webm';
  await waitForClick();
  await startPlayingAndWaitForVideo(video);

+  let alwaysUpdateVideo = !('requestVideoFrameCallback' in video);
+  let haveNewVideoFrame = false;
+  if (!alwaysUpdateVideo) {
+    function recordHaveNewFrame() {
+      haveNewVideoFrame = true;
+      video.requestVideoFrameCallback(recordHaveNewFrame);
+    }
+    video.requestVideoFrameCallback(recordHaveNewFrame);
+  }

  ...

  function render() {
+    if (alwaysUpdateVideo || haveNewVideoFrame) {
+      haveNewVideoFrame = false;
      copySourceToTexture(device, texture, video);
+    }

    ...
```

With this change we'd only update the video for each new frame. So, for example, on a device
with a display rate of 120 frames per second we'd draw at 120 frames per second so animations,
camera movements, etc would be smooth. But, the video texture itself would only update at its own frame
rate (for example 30fps).

**BUT! WebGPU has special support for using video efficiently**

We'll cover that in [another article](webgpu-textures-external-video.html).
The way above, using `device.query.copyExternalImageToTexture` is actually
making **a copy**. Making a copy takes time. For example a 4k video's resolution
is generally 3840 × 2160 which for `rgba8unorm` is 31meg of data that needs to be
copied, **per frame**. [External textures](webgpu-textures-external-video.html)
let you use the video's data directly (no copy) but require different methods
and have some restrictions.

## <a id="a-texture-atlases"></a> Texture Atlases

From the examples above, we can see that to draw something with a texture
we have to create the texture, put data it in, bind it to bindGroup with
a sampler,
and reference it from a shader. So what would we do if we wanted
to draw multiple different textures on an object? Say we had a chair where the legs and back
are made of wood but the cushion is made of cloth. 

<div class="webgpu_center">
  <div class="center">
    <model-viewer 
      src="/webgpu/resources/models/gltf/cc0_chair.glb"
      camera-controls
      touch-action="pan-y"
      camera-orbit="45deg 70deg 2.5m"
      interaction-prompt="none"
      disable-zoom
      disable-pan
      style="width: 400px; height: 400px;"></model-viewer>
  </div>
  <div>
    <a href="https://skfb.ly/opnwY"></a>"[CC0] Chair" by adadadad5252341 <a href="http://creativecommons.org/licenses/by/4.0/">CC-BY 4.0</a>
  </div>
</div>

Or a car where the tires are rubber, the body is paint, the bumpers and hub caps
are chrome.

<div class="webgpu_center">
  <div class="center">
    <model-viewer 
      src="/webgpu/resources/models/gltf/classic_muscle_car.glb"
      camera-controls
      touch-action="pan-y"
      camera-orbit="45deg 70deg 20m"
      interaction-prompt="none"
      disable-zoom
      disable-pan
      style="width: 700px; height: 400px;"></model-viewer>
  </div>
  <div>
    <a href="https://skfb.ly/6Usqo"></a>"Classic Muscle car" by Lexyc16 <a href="http://creativecommons.org/licenses/by/4.0/">CC-BY 4.0</a>
  </div>
</div>

If we did nothing else you might think we'd have to draw 2 times for
the chair, once for the wood with a wood texture, and once for the
cushion with a cloth texture. For the car we'd have several draws, one for
the tires, one for the body, one for the bumpers, etc...

That would end up being slow as every object would require multiple
draw calls. We could try to fix that by adding more inputs to our
shader (2, 3, 4 textures) with texture coordinates for each one
but that would not be very flexible and would be slow as well
as we'd need to read all 4 textures and add code to chose between them.

The most common way to cover this case is to use what's called a
[Texture Atlas](https://www.google.com/search?q=texture+atlas). 
A Texture Atlas is a fancy name for a texture with
multiple images it in. We then use texture coordinates to select
which parts go where.

Let's wrap a cube with these 6 images

<div class="webgpu_table_div_center">
  <style>
    table.webgpu_table_center {
      border-spacing: 0.5em;
      border-collapse: separate;
    }
    table.webgpu_table_center img {
      display:block;
    }
  </style>
  <table class="webgpu_table_center">
    <tr><td><img src="resources/noodles-01.jpg" /></td><td><img src="resources/noodles-02.jpg" /></td></tr>
    <tr><td><img src="resources/noodles-03.jpg" /></td><td><img src="resources/noodles-04.jpg" /></td></tr>
    <tr><td><img src="resources/noodles-05.jpg" /></td><td><img src="resources/noodles-06.jpg" /></td></tr>
  </table>
</div>

Using some image editing software like Photoshop or [Photopea](https://photopea.com) we could put all 6 images into a single image

<img class="webgpu_center" src="../resources/images/noodles.jpg" />

We'd then make a cube and provide texture coordinates that select each
portion of the image onto a specific face of the cube. To keep it simple I put
all 6 images in the texture above in squares, 4x2. So it should be pretty
easy to compute the texture coordinates for each square. 

<div class="webgpu_center center diagram">
  <div>
    <div data-diagram="texture-atlas" style="display: inline-block; width: 600px;"></div>
  </div>
</div>

> The diagram above might be confusing because it is often suggested that texture coordinates 
> have 0,0 as the bottom left corner. Really though there is no "bottom". There is just the idea
> that texture coordinate 0,0 references the first pixel in the texture's data. The first
> pixel in the texture's data is the top left corner of the image.
> If you subscribe to the idea that 0,0 = bottom left then our texture coordinates
> would be visualized like this. **They're still the same coordinates**.

<div class="webgpu_center center diagram">
  <div>
    <div data-diagram="texture-atlas-bottom-left" style="display: inline-block; width: 600px;"></div>
    <div class="center">0,0 at bottom left</div>
  </div>
</div>


Here's the position vertices for a cube and the texture coordinates
to go with them

```rust
#[rustfmt::skip]
fn create_cube_vertices() -> (Vec<f32>, Vec<u16>, u32) {
    let vertex_data: Vec<f32> = vec![
         //  position   |  texture coordinate
         //-------------+----------------------
         // front face     select the top left image
        -1.0,  1.0,  1.0,        0.0 , 0.0,
        -1.0, -1.0,  1.0,        0.0 , 0.5,
         1.0,  1.0,  1.0,        0.25, 0.0,
         1.0, -1.0,  1.0,        0.25, 0.5,
         // right face     select the top middle image
         1.0,  1.0, -1.0,        0.25, 0.0,
         1.0,  1.0,  1.0,        0.5 , 0.0,
         1.0, -1.0, -1.0,        0.25, 0.5,
         1.0, -1.0,  1.0,        0.5 , 0.5,
         // back face      select to top right image
         1.0,  1.0, -1.0,        0.5 , 0.0,
         1.0, -1.0, -1.0,        0.5 , 0.5,
        -1.0,  1.0, -1.0,        0.75, 0.0,
        -1.0, -1.0, -1.0,        0.75, 0.5,
        // left face       select the bottom left image
        -1.0,  1.0,  1.0,        0.0 , 0.5,
        -1.0,  1.0, -1.0,        0.25, 0.5,
        -1.0, -1.0,  1.0,        0.0 , 1.0,
        -1.0, -1.0, -1.0,        0.25, 1.0,
        // bottom face     select the bottom middle image
         1.0, -1.0,  1.0,        0.25, 0.5,
        -1.0, -1.0,  1.0,        0.5 , 0.5,
         1.0, -1.0, -1.0,        0.25, 1.0,
        -1.0, -1.0, -1.0,        0.5 , 1.0,
        // top face        select the bottom right image
        -1.0,  1.0,  1.0,        0.5 , 0.5,
         1.0,  1.0,  1.0,        0.75, 0.5,
        -1.0,  1.0, -1.0,        0.5 , 1.0,
         1.0,  1.0, -1.0,        0.75, 1.0,
    ];
    let index_data: Vec<u16> = vec![
         0,  1,  2,  2,  1,  3,  // front
         4,  5,  6,  6,  5,  7,  // right
         8,  9, 10, 10,  9, 11,  // back
        12, 13, 14, 14, 13, 15,  // left
        16, 17, 18, 18, 17, 19,  // bottom
        20, 21, 22, 22, 21, 23,  // top
    ];
    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}
```

To make this example we're going to have start with an example from [the article on cameras](webgpu-cameras.html).
If you haven't read the article yet you can read it and the series it's a part of the learn how do 3D.
For now, the important part is, like we did above, we output positions and texture coordinates from our
vertex shader and use them to look up values from a texture in our fragment shader. So, here's
the changes needed from the shader in the camera example, applying what we have above.

```wgsl
struct Uniforms {
  matrix: mat4x4f,
};

struct Vertex {
  @location(0) position: vec4f,
-  @location(1) color: vec4f,
+  @location(1) texcoord: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
-  @location(0) color: vec4f,
+  @location(0) texcoord: vec2f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;
+@group(0) @binding(1) var ourSampler: sampler;
+@group(0) @binding(2) var ourTexture: texture_2d<f32>;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = uni.matrix * vert.position;
-  vsOut.color = vert.color;
+  vsOut.texcoord = vert.texcoord;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
-  return vsOut.color;
+  return textureSample(ourTexture, ourSampler, vsOut.texcoord);
}
```

All we did was switch from taking a color per vertex to a texture coordinate per vertex
and passing that texture coordinate to the fragment shader, like we did above. We then
use it, in the fragment shader, like we did above.

In Rust we need to change that example's pipeline from taking a color to taking
texture coordinates

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("2 attributes"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (4) * 4, // (3) floats 4 bytes each + one 4 byte color
+        array_stride: (3 + 2) * 4, // (3+2) floats 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
-          // color
-          wgpu::VertexAttribute {
-            shader_location: 1,
-            offset: 12,
-            format: wgpu::VertexFormat::Unorm8x4,
-          },
+          // texcoord
+          wgpu::VertexAttribute {
+            shader_location: 1,
+            offset: 12,
+            format: wgpu::VertexFormat::Float32x2,
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
    primitive: wgpu::PrimitiveState {
      cull_mode: Some(wgpu::Face::Back),
      ..Default::default()
    },
    depth_stencil: Some(wgpu::DepthStencilState {
      depth_write_enabled: Some(true),
      depth_compare: Some(wgpu::CompareFunction::Less),
      format: wgpu::TextureFormat::Depth24Plus,
      stencil: Default::default(),
      bias: Default::default(),
    }),
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

To keep the data smaller we're going to use indices like we covered in [the article on vertex buffers](webgpu-vertex-buffers.html).

```rust
-  let (vertex_data, num_vertices) = create_f_vertices();
+  let (vertex_data, index_data, num_vertices) = create_cube_vertices();
  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("vertex buffer vertices"),
    size: (vertex_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));

+  let index_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+    label: Some("index buffer"),
+    size: (index_data.len() * 2) as u64,
+    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
+    mapped_at_creation: false,
+  });
+  app.queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));
```

We need to copy all of the texture loading and mip generation code into this example
and then use it to load the texture atlas image. We also need to make a sampler
and add them our bind group

```rust
+  let texture = create_texture_from_image(&app.device, &app.queue,
+      "resources/images/noodles.jpg", true, false).await;
+
+  let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
+    mag_filter: wgpu::FilterMode::Linear,
+    min_filter: wgpu::FilterMode::Linear,
+    mipmap_filter: wgpu::MipmapFilterMode::Linear,
+    ..Default::default()
+  });

  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bind group for object"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
+      wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
+      wgpu::BindGroupEntry {
+        binding: 2,
+        resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
+      },
    ],
  });
```

We need to do some 3D math to setup a matrix for drawing in 3D. (Again, see [the camera article](webgpu-cameras.html) for
details on 3D math.)

We use [`glam`](https://docs.rs/glam) for the matrix math (the rotation
sliders come from the page GUI as usual):

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {

    ...

    let rotation = [
      wgpu_fun::setting_f64("rotationX", 20.0f64.to_radians()) as f32,
      wgpu_fun::setting_f64("rotationY", 25.0f64.to_radians()) as f32,
      wgpu_fun::setting_f64("rotationZ", 0.0) as f32,
    ];

    let aspect = frame.width as f32 / frame.height as f32;
    let matrix = Mat4::perspective_rh(
      60.0f32.to_radians(),
      aspect,
      0.1,  // zNear
      10.0, // zFar
    ) * Mat4::look_at_rh(
      Vec3::new(0.0, 1.0, 5.0), // camera position
      Vec3::new(0.0, 0.0, 0.0), // target
      Vec3::new(0.0, 1.0, 0.0), // up
    ) * Mat4::from_rotation_x(rotation[0])
      * Mat4::from_rotation_y(rotation[1])
      * Mat4::from_rotation_z(rotation[2]);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
```

And at render time we need to draw with indices

```rust
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&render_pass_descriptor);
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
+      pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);

      ...

      pass.set_bind_group(0, &bind_group, &[]);
-      pass.draw(0..num_vertices, 0..1);
+      pass.draw_indexed(0..num_vertices, 0, 0..1);
    }
```

And we get a cube, with a different image on each side, using a single texture.

{{{example url="../webgpu-texture-atlas.html"}}}

Using a texture atlas is good because there's just 1 texture to load, the shader stays simple as it only has to reference 1 texture, and it only
requires 1 draw call to draw the shape instead of 1 draw call per texture as it might if we keep the images separate.

<!-- keep this at the bottom of the article -->
<script type="module" src="/3rdparty/model-viewer.3.3.0.min.js"></script>
<script type="module" src="webgpu-importing-textures.js"></script>
