Title: WebGPU Compatibility Mode
Description: Running on older machines
TOC: Compatibility Mode

WebGPU Compatibility mode is a version of WebGPU that,
with some limits, can run on older devices. The idea is,
if you can make your app run within some extra limits and
restrictions then you can request a webgpu compatibility adapter
and have your app run in more places.

> Note: Compatibility mode is shipping in Chrome 146. (2026-02-23) It may be available in
> your browser as an experiment. In, [Chrome Canary](https://www.google.com/chrome/canary/),
> as of version 136.0.7063.0
> (2025-03-11), you can allow compatibility mode by enabling the flag
> "enable-unsafe-webgpu" by going to
> `chrome://flags/#enable-unsafe-webgpu`.

To give some idea what what you can do in compatibility mode,
effectively *nearly* all WebGL2 programs could be converted to
run on compatibility mode.

In the browser, a JavaScript WebGPU app opts in like this.

```js
const adapter = await navigator.gpu.requestAdapter({
  featureLevel: 'compatibility',
});
const device = await adapter.requestDevice();
```

Simple! With Rust and wgpu though, there is no way to opt in:
`wgpu::RequestAdapterOptions` has no equivalent of `featureLevel` and the
wgpu documentation states plainly that "wgpu does not support
compatibility-level adapters per se". Requesting an adapter with wgpu
always gets you core WebGPU behavior.

So why should a wgpu user care? Two reasons:

* If you ship your app as wasm, the adapter your app gets comes from the
  browser. Something outside your code — a future browser default on old
  hardware, or a tool like the
  [webgpu-dev-extension](https://github.com/greggman/webgpu-dev-extension)
  covered at the bottom of this article — can put the page's adapters in
  compatibility mode, and then the restrictions below are enforced on
  your code.

* If you want your app to run on the lowest common denominator of devices,
  these restrictions describe what older GPUs actually can not do. Staying
  within them keeps your code portable, whether or not anything enforces
  them.

The good news: every app that follows all the
limits of compatibility mode is a valid "core"
webgpu app and will run anywhere WebGPU is already
running. Everything below, including the compatibility-friendly
`generate_mips` we'll write, is directly usable from Rust.

# Major limits and restrictions

## Possibly 0 storage buffers in vertex shaders.

The major restriction that is most likely to affect WebGPU apps is that ~45%
of these old devices do not support storage buffers in vertex shaders.

We used this feature in [the article on storage buffers](webgpu-storage-buffers.html)
which is the 3nd article on this site. After that article we
[switched to using vertex buffers](webgpu-vertex-buffers.html).
Using vertex buffers is common and works everywhere but certain solutions are easier
with storage buffers. One example is
[this example of drawing wireframes](https://webgpu.github.io/webgpu-samples/?sample=wireframe). 
It uses storage buffers to generate triangles from vertex data.

With vertex data stored in storage buffers we can randomly access the vertex
data. With the vertex data in vertex buffer we can not. Of course there are
always other solutions.

## Medium limits and restrictions

## Only a single view dimension is allowed for a texture as a `TEXTURE_BINDING`

In normal WebGPU you can make a 2d texture like this

```rust
let my_texture = device.create_texture(&wgpu::TextureDescriptor {
  size: wgpu::Extent3d { width, height, depth_or_array_layers: 6 },
  usage: ...
  format: ...
  ...
});
```

You can then view it 3 different view dimensions

```rust
// a view of my_texture as a 2d array with 6 layers
let as_2d_array = my_texture.create_view(&Default::default());

// view layer 3 of my_texture as a 2d texture
let as_2d = my_texture.create_view(&wgpu::TextureViewDescriptor {
  dimension: Some(wgpu::TextureViewDimension::D2),
  base_array_layer: 3,
  array_layer_count: Some(1),
  ..Default::default()
});

// view of my_texture as a cubemap
let as_cube = my_texture.create_view(&wgpu::TextureViewDescriptor {
  dimension: Some(wgpu::TextureViewDimension::Cube),
  ..Default::default()
});
```

In compatibility mode you can only use one view dimension and you have to
choose which view dimension when you create the texture. A 2D texture with
1 layer defaults to only being usable as a `'2d'` view. A 2D texture with
more than 1 layer defaults to only being usable as a `'2d-array`' view.
If you want something other than the default you must tell WebGPU. For example,
If you want a cube map then you must tell WebGPU when you create the texture.
In JavaScript that looks like this

```js
const cubeTexture = device.createTexture({
  size: [width, height, 6],
  usage: ...
  format: ...
  textureBindingViewDimension: 'cube', 
});
```

Note, this extra parameter is called `textureBindingViewDimension` because
it relates to using the texture with usage `TEXTURE_BINDING`. You can still
use a single layer of a cubemap or 2d-array as a 2d texture as a `RENDER_ATTACHMENT`.

`wgpu::TextureDescriptor` has no `textureBindingViewDimension` field — it's
a compatibility-mode-only parameter, and, as covered at the top of this
article, wgpu doesn't do compatibility-level adapters. So this declaration
is one more thing you can't express from Rust. What we *can* do from Rust
is follow the rule the parameter implies: pick one view dimension per
texture and stick to it, which is what the code below will do.

To put it another way, you must use this same view dimension when using the
texture in a bind group. You can still use the `2d` dimension, even if the
`textureBindingViewDimension` is `2d-array` or `cube` when using the texture
in as a render target.

In compatibility mode, using the texture in a bind group with another type of view will
generate a validation error.

```rust
// a view of cube_texture as a 2d array with 6 layers
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
  ...
  entries: &[
    wgpu::BindGroupEntry {
      binding,
      // ERROR in compatibility mode: texture is a cubemap not a 2d-array
      // (the default for a texture with more than 1 layer)
      resource: wgpu::BindingResource::TextureView(
        &cube_texture.create_view(&Default::default()),
      ),
    },
  ],
});
```

```rust
// view layer 3 of cube_texture as a 2d texture
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
  ...
  entries: &[
    wgpu::BindGroupEntry {
      binding,
      // ERROR in compatibility mode: texture is a cubemap not 2d
      resource: wgpu::BindingResource::TextureView(
        &cube_texture.create_view(&wgpu::TextureViewDescriptor {
          dimension: Some(wgpu::TextureViewDimension::D2),
          base_array_layer: 3,
          array_layer_count: Some(1),
          ..Default::default()
        }),
      ),
    },
  ],
});
```

```rust
// view of cube_texture as a cubemap
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
  ...
  entries: &[
    wgpu::BindGroupEntry {
      binding,
      // GOOD!
      resource: wgpu::BindingResource::TextureView(
        &cube_texture.create_view(&wgpu::TextureViewDescriptor {
          dimension: Some(wgpu::TextureViewDimension::Cube),
          ..Default::default()
        }),
      ),
    },
  ],
});
```

This restriction is not that big of a deal.
Few programs want to use a texture with different kinds of views.

## When calling `texture.createView` you can not select a subset of layers in a bindGroup

In core WebGPU we can create a texture with some layers

```rust
let texture = device.create_texture(&wgpu::TextureDescriptor {
  size: wgpu::Extent3d { width: 64, height: 128, depth_or_array_layers: 8 },  // 8 layers,
  ...
});
```

We can then select a subset of layers

```rust
let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
  ...
  entries: &[
    wgpu::BindGroupEntry {
      binding,
      // ERROR  in compatibility mode - select layers 3 and 4
      resource: wgpu::BindingResource::TextureView(
        &cube_texture.create_view(&wgpu::TextureViewDescriptor {
          base_array_layer: 3,
          array_layer_count: Some(2),
          ..Default::default()
        }),
      ),
    },
  ],
});
```

This restriction is also not that big of a deal. Few programs
want to select a subset of layers from a texture.

## <a id="a-generating-mipmaps"></a> Generating Mipmaps in compatibility mode.

There is one place though both of these restrictions comes up and that is when generating
mipmaps, which is a common use-case.

Recall that we made a gpu based mipmap generator in 
[the article in importing images into textures](webgpu-importing-textures.html#a-generating-mips-on-the-gpu).
We modified that function to generate mipmaps for 2d-array and cubemaps in
[the article on cube maps](webgpu-cube-maps.html#a-texture-helpers). In that version
we always view each layer of the texture with a `'2d'` dimension to reference
just one layer of the texture.
This won't work in compatibility mode for the reasons above. We can't use a `'2d'`
view of `'2d-array'` or `'cube'` texture. We also can not select individual layers
in a bind group to select which layer to read from.

To make the code work in compatibility mode we have to work with textures
with the same view dimension they were created with and we need to pass in the texture
with access to all layers and select the layer we want in the shader itself, rather
than selecting the layer via `create_view` as we were doing.

So let's do that! We'll start with the code for `generate_mips` from [the article on cubemaps](webgpu-cube-maps.html#a-texture-helpers).

```rust
fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    use std::cell::RefCell;
    use std::collections::HashMap;
    thread_local! {
        static CACHE: RefCell<Option<(wgpu::ShaderModule, wgpu::Sampler)>> = const { RefCell::new(None) };
        static PIPELINE_BY_FORMAT: RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>> =
            RefCell::new(HashMap::new());
    }

    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let (module, sampler) = cache.get_or_insert_with(|| {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("textured quad shaders for mip level generation"),
                source: wgpu::ShaderSource::Wgsl(
                    /* wgsl */ r#"
            struct VSOutput {
              @builtin(position) position: vec4f,
              @location(0) texcoord: vec2f,
            };

            @vertex fn vs(
              @builtin(vertex_index) vertexIndex : u32
            ) -> VSOutput {
              let pos = array(

                vec2f( 0.0,  0.0),  // center
                vec2f( 1.0,  0.0),  // right, center
                vec2f( 0.0,  1.0),  // center, top

                // 2st triangle
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
          "#
                    .into(),
                ),
            });

            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                min_filter: wgpu::FilterMode::Linear,
                mag_filter: wgpu::FilterMode::Linear,
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
                for layer in 0..texture.depth_or_array_layers() {
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
                                resource: wgpu::BindingResource::TextureView(
                                    &texture.create_view(&wgpu::TextureViewDescriptor {
                                        dimension: Some(wgpu::TextureViewDimension::D2),
                                        base_mip_level: base_mip_level - 1,
                                        mip_level_count: Some(1),
                                        base_array_layer: layer,
                                        array_layer_count: Some(1),
                                        ..Default::default()
                                    }),
                                ),
                            },
                        ],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
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
            }

            let command_buffer = encoder.finish();
            queue.submit([command_buffer]);
        });
    });
}
```

We need to change the WGSL so for each type of texture (2d, 2d-array, cube, etc...) we
use a different fragment shader and we need to be able to pass in a layer to read from.

```wgsl
+const faceMat = array(
+  mat3x3f( 0,  0,  -2,  0, -2,   0,  1,  1,   1),   // pos-x
+  mat3x3f( 0,  0,   2,  0, -2,   0, -1,  1,  -1),   // neg-x
+  mat3x3f( 2,  0,   0,  0,  0,   2, -1,  1,  -1),   // pos-y
+  mat3x3f( 2,  0,   0,  0,  0,  -2, -1, -1,   1),   // neg-y
+  mat3x3f( 2,  0,   0,  0, -2,   0, -1,  1,   1),   // pos-z
+  mat3x3f(-2,  0,   0,  0, -2,   0,  1,  1,  -1));  // neg-z

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) texcoord: vec2f,
+  @location(1) @interpolate(flat, either) baseArrayLayer: u32,
};

@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32,
+  @builtin(instance_index) baseArrayLayer: u32,
) -> VSOutput {
  var pos = array<vec2f, 3>(
    vec2f(-1.0, -1.0),
    vec2f(-1.0,  3.0),
    vec2f( 3.0, -1.0),
  );

  var vsOutput: VSOutput;
  let xy = pos[vertexIndex];
  vsOutput.position = vec4f(xy, 0.0, 1.0);
  vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
+  vsOutput.baseArrayLayer = baseArrayLayer;
  return vsOutput;
}

@group(0) @binding(0) var ourSampler: sampler;
-@group(0) @binding(1) var ourTexture: texture_2d<f32>;

+@group(0) @binding(1) var ourTexture2d: texture_2d<f32>;
@fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
-  return textureSample(ourTexture, ourSampler, fsInput.texcoord);
+  return textureSample(ourTexture2d, ourSampler, fsInput.texcoord);
}

+@group(0) @binding(1) var ourTexture2dArray: texture_2d_array<f32>;
+@fragment fn fs2darray(fsInput: VSOutput) -> @location(0) vec4f {
+  return textureSample(
+    ourTexture2dArray,
+    ourSampler,
+    fsInput.texcoord,
+    fsInput.baseArrayLayer);
+}
+
+@group(0) @binding(1) var ourTextureCube: texture_cube<f32>;
+@fragment fn fscube(fsInput: VSOutput) -> @location(0) vec4f {
+  return textureSample(
+    ourTextureCube,
+    ourSampler,
+    faceMat[fsInput.baseArrayLayer] * vec3f(fract(fsInput.texcoord), 1));
+}
```

This code has 3 fragment shaders, one for each of `'2d'`, `'2d-array'`, `'cube'`.
It uses the [large triangle to cover clip space](webgpu-large-triangle-to-cover-clip-space.html) technique
[covered elsewhere](webgpu-large-triangle-to-cover-clip-space.html) to draw.
It also uses `@builtin(instance_index)` to select the layer. This is an interesting and quick way
to pass in a single integer value to a shader without having to use a uniform buffer.
When we call `draw`, the start of the instance range is the first instance which will be passed
to the shader as `@builtin(instance_index)`. We pass that from the vertex shader to fragment
shader via `VSOutput.baseArrayLayer` which we can reference has `fsInput.baseArrayLayer`
in the fragment shader.

The cubemap code converts a 2d-array layer and normalized UV coordinate into a
cubemap 3d coordinate. We need this because again, in compatibility mode, a cubemap
can only be viewed as a cubemap.

Back to our Rust. The JavaScript version reads a `textureBindingViewDimension`
property from the texture, which is defined when in compatibility mode and
otherwise assumed to be `'2d-array'` since in "core" webgpu `'2d-array'`
should always work. wgpu textures have no such property (just like
`wgpu::TextureDescriptor` has no such field), so we state the view dimension
ourselves. This example only makes cube maps so we use `Cube`; if you were
generating mips for plain 2d or 2d-array textures you'd pass the dimension
in, or derive it from `texture.depth_or_array_layers()`.

```rust
fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
+    // A generateMips that works in WebGPU compatibility mode: it never makes
+    // a '2d' view of a 2d-array/cube texture. It binds the texture with its
+    // own view dimension (here 'cube') and selects a fragment shader entry
+    // point to match, rendering each layer with the layer index passed as
+    // the first instance (instance_index).
+    let texture_binding_view_dimension = wgpu::TextureViewDimension::Cube;

    use std::cell::RefCell;
    use std::collections::HashMap;

    ...

            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("textured quad shaders for mip level generation"),
                source: wgpu::ShaderSource::Wgsl(
                    /* wgsl */ r#"
+            const faceMat = array(
+              mat3x3f( 0,  0,  -2,  0, -2,   0,  1,  1,   1),   // pos-x
+              mat3x3f( 0,  0,   2,  0, -2,   0, -1,  1,  -1),   // neg-x
+              mat3x3f( 2,  0,   0,  0,  0,   2, -1,  1,  -1),   // pos-y
+              mat3x3f( 2,  0,   0,  0,  0,  -2, -1, -1,   1),   // neg-y
+              mat3x3f( 2,  0,   0,  0, -2,   0, -1,  1,   1),   // pos-z
+              mat3x3f(-2,  0,   0,  0, -2,   0,  1,  1,  -1));  // neg-z

            struct VSOutput {
              @builtin(position) position: vec4f,
              @location(0) texcoord: vec2f,
+              @location(1) @interpolate(flat, either) baseArrayLayer: u32,
            };

            @vertex fn vs(
              @builtin(vertex_index) vertexIndex : u32,
+              @builtin(instance_index) baseArrayLayer: u32,
            ) -> VSOutput {
              var pos = array<vec2f, 3>(
                vec2f(-1.0, -1.0),
                vec2f(-1.0,  3.0),
                vec2f( 3.0, -1.0),
              );

              var vsOutput: VSOutput;
              let xy = pos[vertexIndex];
              vsOutput.position = vec4f(xy, 0.0, 1.0);
              vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
+              vsOutput.baseArrayLayer = baseArrayLayer;
              return vsOutput;
            }

            @group(0) @binding(0) var ourSampler: sampler;

            @group(0) @binding(1) var ourTexture2d: texture_2d<f32>;
            @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
              return textureSample(ourTexture2d, ourSampler, fsInput.texcoord);
            }

+            @group(0) @binding(1) var ourTexture2dArray: texture_2d_array<f32>;
+            @fragment fn fs2darray(fsInput: VSOutput) -> @location(0) vec4f {
+              return textureSample(
+                ourTexture2dArray,
+                ourSampler,
+                fsInput.texcoord,
+                fsInput.baseArrayLayer);
+            }
+
+            @group(0) @binding(1) var ourTextureCube: texture_cube<f32>;
+            @fragment fn fscube(fsInput: VSOutput) -> @location(0) vec4f {
+              return textureSample(
+                ourTextureCube,
+                ourSampler,
+                faceMat[fsInput.baseArrayLayer] * vec3f(fract(fsInput.texcoord), 1));
+            }
          "#
                    .into(),
                ),
            });

    ...
```

Before we tracked a pipeline per format so we could reuse the pipeline for
textures of the same format. We need to update that to be a pipeline per format
per viewDimension. Where the JavaScript version builds the entry point name
with string manipulation we use a `match`.

```rust
    thread_local! {
        static CACHE: RefCell<Option<(wgpu::ShaderModule, wgpu::Sampler)>> = const { RefCell::new(None) };
-        static PIPELINE_BY_FORMAT: RefCell<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>> =
-            RefCell::new(HashMap::new());
+        static PIPELINE_BY_FORMAT_AND_VIEW: RefCell<HashMap<String, wgpu::RenderPipeline>> =
+            RefCell::new(HashMap::new());
    }

    ...

-        PIPELINE_BY_FORMAT.with(|pipelines| {
+        PIPELINE_BY_FORMAT_AND_VIEW.with(|pipelines| {
            let mut pipelines = pipelines.borrow_mut();
-            let pipeline = pipelines.entry(texture.format()).or_insert_with(|| {
+            let id = format!("{:?}.{:?}", texture.format(), texture_binding_view_dimension);
+            let pipeline = pipelines.entry(id).or_insert_with(|| {
+                // choose a fragment shader based on the view dimension
+                let entry_point = match texture_binding_view_dimension {
+                    wgpu::TextureViewDimension::D2 => "fs2d",
+                    wgpu::TextureViewDimension::D2Array => "fs2darray",
+                    wgpu::TextureViewDimension::Cube => "fscube",
+                    _ => unreachable!(),
+                };
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
-                        entry_point: None,
+                        entry_point: Some(entry_point),
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

    ...
```

Then our loop to generate the mipmap needs to change to use the full layers, since
compatibility mode does not allow a sub-range of layers. We also need to use
our ability to pass in the instance index via draw to select the layer we want to read from.

```rust
            for base_mip_level in 1..texture.mip_level_count() {
                for layer in 0..texture.depth_or_array_layers() {
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
                                resource: wgpu::BindingResource::TextureView(
                                    &texture.create_view(&wgpu::TextureViewDescriptor {
-                                        dimension: Some(wgpu::TextureViewDimension::D2),
+                                        dimension: Some(texture_binding_view_dimension),
                                        base_mip_level: base_mip_level - 1,
                                        mip_level_count: Some(1),
-                                        base_array_layer: layer,
-                                        array_layer_count: Some(1),
                                        ..Default::default()
                                    }),
                                ),
                            },
                        ],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor {
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        base_mip_level,
                        mip_level_count: Some(1),
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                        ..Default::default()
                    });
                    {
                        let mut pass = encoder.begin_render_pass(/* renders to view */);
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &bind_group, &[]);
-                        pass.draw(0..6, 0..1); // call our vertex shader 6 times
+                        // draw 3 vertices, 1 instance, first instance (instance_index) = layer
+                        pass.draw(0..3, layer..layer + 1);
                    }
                }
            }

            let command_buffer = encoder.finish();
            queue.submit([command_buffer]);
```

With that our mipmap generation code works in compatibility mode, and it still
works in core WebGPU.

The JavaScript version has one more thing to update: its
`createTextureFromSources` takes a `textureBindingViewDimension` option and
passes it on to `createTexture`, so compatibility mode knows in advance how
the texture will be viewed.

```js
  function createTextureFromSources(device, sources, options = {}) {
    // Assume are sources all the same size so just use the first one for width and height
    const source = sources[0];
    const texture = device.createTexture({
      format: 'rgba8unorm',
      mipLevelCount: options.mips ? numMipLevels(source.width, source.height) : 1,
      size: [source.width, source.height, sources.length],
      usage: GPUTextureUsage.TEXTURE_BINDING |
             GPUTextureUsage.COPY_DST |
             GPUTextureUsage.RENDER_ATTACHMENT,
+      textureBindingViewDimension: options.textureBindingViewDimension,
    });
    copySourcesToTexture(device, texture, sources, options);
    return texture;
  }
```

As covered above, `wgpu::TextureDescriptor` has no such field, so our Rust
`create_texture_from_sources` stays exactly as it was in
[the article on cube maps](webgpu-cube-maps.html#a-texture-helpers).

Similarly, the JavaScript version requests a compatibility adapter like we
covered at the top of this article. From Rust we can't, so our example makes
no such change: it just uses the new `generate_mips`, which, being valid core
WebGPU, runs unmodified.

And with that, our cube map sample works in compatibility mode.

{{{example url="../webgpu-compatibility-mode-generatemips.html"}}}

You now have a compatibility mode friendly `generate_mips` which you could
use in any of the examples on this site. It works on both core and compatibility mode.
Set (or pass in) the texture binding view dimension if you want a cube map or if
you want a 1 layer 2d-array; otherwise 2d-array is a fine choice, since in
core WebGPU it doesn't matter.

# Minor limits and restrictions

The following are limits and restrictions *most* programs are unlikely to
run into

* ## Color blending must match on all color targets.

  In core, when you create a render pipeline, each color target
  can specify blending settings. We used blending settings in
  [the article on blending and transparency](webgpu-transparency.html).
  In compatibility mode, all the settings across all color targets
  in a single pipeline must be the same.

* ## `copy_texture_to_buffer` and `copy_texture_to_texture` do not work with compressed textures

* ## `copy_texture_to_texture` does not work with multisampled textures

* ## `cube-array` is not supported

* ## views of a textures may not differ in aspect or mip levels in a single draw/dispatch call.

  In core WebGPU you can make multiple texture views of a texture to different mip
  levels AND use them in the same draw call. This is uncommon. Note that this
  restriction is on `TEXTURE_BINDING` usage, using a texture via a bindGroup. You
  can still use a different view as a `RENDER_ATTACHMENT` as we did in the mipmap generation
  code above.

* ## `@builtin(sample_mask)` and `@builtin(sample_index)` are not supported

* ## `rg32uint`, `rg32sint` and `rg32float` texture formats can not be used as storage textures.

* ## `depthClampBias` must be 0

  This is a setting when creating a render pipeline
  (the `clamp` field of `wgpu::DepthBiasState`).

* ## `@interpolation(linear)` and `@interpolation(..., sample)` are not supported

  These were briefly mentioned in [the article on inter-stage variables](webgpu-inter-stage-variables.html#a-interpolate).

* ## <a id="flat"></a> `@interpolate(flat)` and `@interpolate(flat, first)` are not supported

  In compatibility mode you must use `@interpolate(flat, either)` when you want
  flat interpolation. `either` means the value passed to the fragment shader
  could be the value from either the first or last vertex of the triangle or line
  being drawn. It's up to the implementation.

  It is common for this not to matter. The most common use cases for passing something
  with flat interpolation from the vertex shader to the fragment shader are usually
  per model, per material, or per instance types of values. For example the mipmap
  generation code above used flat interpolation above to pass the `instance_index`
  to the fragment shader. It will be the same for all vertices of a triangle and
  so works just fine with `@interpolate(flat, either)`

* ## Texture formats can not be reinterpreted

  In core WebGPU you can create an `'rgba8unorm'` texture and view it as an `'rgba8unorm-srgb'`
  texture and visa-versa as well as other `'-srgb'` formats and their corresponding non `'-srgb'`
  formats. Compatibility mode does not allow this. Whatever format you create the texture
  is the only format it can be used as.

* ## `bgra8unorm-srgb` is not supported.

* ## `rgba16float` and `r32float` textures can not be multisampled.

* ## All integer texture formats can not be multisampled.

* ## `depthOrArrayLayers` must be compatible with `textureBindingViewDimension`

  This means a texture marked with `textureBindingViewDimension: '2d'` must
  have a `depthOrArrayLayers: 1` (the default). A texture marked with `textureBindingViewDimension: 'cube'`
  must have `depthOrArrayLayers: 6`.

* ## `textureLoad` does not work with depth textures.

  A "depth texture" is a texture referenced in WGSL with `texture_depth`,
  `texture_depth_2d_array`, or `texture_depth_cube`. Those can not be used with
  `textureLoad` in compatibility mode.

  On the other hand, `textureLoad` can be used with `texture_2d<f32>`, `texture_2d_array<f32>` and
  `texture_cube<f32>` and a texture that has a depth format can be bound to these bindings..

* ## depth textures can not be used with non-comparison samplers.

  Again, a "depth texture" is a texture referenced in WGSL with `texture_depth`,
  `texture_depth_2d_array`, or `texture_depth_cube`. Those can not be used
  with a non-comparison sampler in compatibility mode.

  This effectively means `texture_depth`, `texture_depth_2d_array`, and `texture_depth_cube`
  can only be used with `textureSampleCompare`, `textureSampleCompareLevel` and `textureGatherCompare`
  in compatibility mode.

  On the other hand, you can bind a texture that uses a depth format to a `texture_2d<f32>`, `texture_2d_array<f32>` and `texture_cube<f32>` binding,
  subject to the normal restriction that it must use a non-filtering sampler.

* ## fine derivatives are not supported

  The WGSL functions `dpdxFine`, `dpdyFine` and `fwidthFine` are not supported in compatibility mode.
  You can still use `dpdx` `dpdxCoarse`, `dpdy`, `dpdyCoarse`, `fwidth`, and `fwidthCoarse`

* ## The combinations of texture + sampler are more limited

  In core you can bind 16+ textures and 16+ samplers and then in your shader
  you can use all 256+ combinations.

  In compatibility mode you can only use 16 total combinations in a single stage.

  The actual rule is a little more complicated. Here it is spelled out in pseudo code.

  ```
  maxCombinationsPerStage =
     min(device.limits.max_sampled_textures_per_shader_stage, device.limits.max_samplers_per_shader_stage)
  for each stage of the pipeline:
    sum = 0
    for each texture binding in the pipeline layout which is visible to that stage:
      sum += max(1, number of texture sampler combos for that texture binding)
    for each external texture binding in the pipeline layout which is visible to that stage:
      sum += 1 // for LUT texture + LUT sampler
      sum += 3 * max(1, number of external_texture sampler combos) // for Y+U+V
    if sum > maxCombinationsPerStage
      generate a validation error.
  ```

* ## Some of the default limits are lower in compatibility mode

  | limit                                    | compat  | core      |
  | :--------------------------------------- | ------: | --------: |
  | `max_color_attachments`                  |       4 |         8 |
  | `max_compute_invocations_per_workgroup`  |     128 |       256 |
  | `max_compute_workgroup_size_x`           |     128 |       256 |
  | `max_compute_workgroup_size_y`           |     128 |       256 |
  | `max_inter_stage_shader_variables`       |      15 |        16 |
  | `max_texture_dimension_1d`               |    4096 |      8192 |
  | `max_texture_dimension_2d`               |    4096 |      8192 |
  | `max_uniform_buffer_binding_size`        |   16384 |     65536 |
  | `max_vertex_attributes`           | 16<sup>a</sup> |        16 |

  (a) In compatibility mode, using `@builtin(vertex_index)`
and/or `@builtin(instance_index)` each count as an
attribute.

  Of course the adapter may support higher limits for any of these.

* ## There are 4 new limits.

  * `maxStorageBuffersInVertexStage` (default 0)
  * `maxStorageTexturesInVertexStage` (default 0)
  * `maxStorageBuffersInFragmentStage` (default 4)
  * `maxStorageTexturesInFragmentStage` (default 4)

  Like other limits, you can check when you request an adapter
  what the adapter supports and require higher than the defaults
  if you need more. Note these are given with their WebGPU names:
  they are compatibility-mode-only limits and have no field on
  `wgpu::Limits`.

  As mentioned above, about 45% of devices support `0`
  storage buffers and storage textures in vertex shaders.

# Upgrading from compatibility mode to core

Compatibility mode was designed for you to opt-in. If you
can design your application to live with the restrictions above
then you ask for compatibility mode. If not, ask for core, the
default, if the device can't handle core it will not return
an adapter.

This opt-in dance is, as covered at the top of this article, JavaScript-only —
a wgpu app always behaves as core — but it's worth knowing if you write the
page JavaScript around your wasm app or need to reason about what a browser
will do.

A JavaScript app can also be designed to function
in compatibility mode but take advantage of all the core features
if the user has a device that supports core WebGPU.

To do this, ask for a compatibility mode adapter, then check
for and enable the `core-features-and-limits` feature. If it
exists on the adapter AND you require it on the device the
device will be a core device and none of the restrictions above
will apply.

Example:

```js
const adapter = await navigator.gpu.requestAdapter({
  featureLevel: 'compatibility',
});
const hasCore = adapter.features.has('core-features-and-limits');
const device = await adapter.requestDevice({
  requiredFeatures: [
    ...(hasCore ? ['core-features-and-limits'] : []),
  ],
});
```

If `hasCore` is true then none of the above restrictions and limits apply.

Note that other code that wants to check if the device is a core or
compatibility device should check the device's features.

```js
const isCore = device.features.has('core-features-and-limits');
```

This will always be true on a core device.

# Testing compatibility mode

On a browser that supports compatibility mode you can test that an
application follows the restrictions by requesting a compatibility
adapter and NOT requesting `'core-features-and-limits'`.
You may want to check that you actually have a compatibility
device so you can know that the restrictions and limits are
being enforced.

```js
const adapter = await navigator.gpu.requestAdapter({
  featureLevel: 'compatibility',
});
const device = await adapter.requestDevice();

const isCompatibilityMode = !device.features.has('core-features-and-limits');
```

This is a good way to test if your app will run on these older devices.

# Quick test via the webgpu-dev-extension

Using [webgpu-dev-extension](https://github.com/greggman/webgpu-dev-extension) you can
force your app to use compatibility mode as a quick test with no changes to your app.
Because the extension changes what `navigator.gpu.requestAdapter` returns, this
works for wasm builds of wgpu apps too — it's the one way to see a wgpu app run
against compatibility mode restrictions.
You can also test an app that auto-upgrades to core webgpu, works when it gets compatibility mode.

Steps:

1. Open devtools and run your app
2. In Devtools, open the settings

   <div class="webgpu_left"><img src="resources/images/webgpu-devtools-settings.png" style="width: 554px"></div>

3. Turn on 'Custom Formatters'

   <div class="webgpu_left"><img src="resources/images/webgpu-devtools-custom-formatters.png" style="width: 554px"></div>

4. In the WebGPU-Dev-Extension, select these options:

   <div class="webgpu_left"><img src="resources/images/webgpu-dev-extension-compat.png" style="width: 274px"></div>

    * ### Force Mode: 'compatibility-mode'

      This makes the app do `navigator.gpu.requestAdapter({ featureLevel: 'compatibility' });`

      Leave this at the default of your app already supports compatibility mode.

    * ### Block Features 'core-features-and-limits'

      This makes it so the app can't request core mode

    * ### DevTools Custom Formatters

      This makes so if you inspect the device in devtools it will show
      device.features as an array of strings. Without this, the devtools shows an
      opaque object so you can't see the features

    * ### Show Adapter Info

      This option makes it do console.log(adapter) and console.log(device) any time
      a new adapter or device is created. This lets you verify the device is in
      compatibility mode. You can check device.features and see that it doesn't have
      'core-features-and-limits'

5. Refresh the page
6. Verify your app is running in compatibility mode

   In the JavaScript console you should see something like this

<div class="webgpu_center"><img src="resources/images/webgpu-compat-verification.png" style="width: 1100px" class="nobg"></div>

   Look for `webgpu-dev-extension: custom-formatters` near the top to verify the formatters
   were injected into the page

   Then, look for `GPUDevice` and expand the `features`. Make sure you **DO NOT SEE**
   `"core-features-and-limits"`.

# Examples:

As of 2026-02-01, all of the local examples at [webgpu-samples](https://webgpu.github.io/webgpu-samples)
work, and 185 of the 193 webgpu examples at [threejs.org/examples](https://threejs.org/examples/)
work in compatibility mode. The remaining 8 may be updated to also work in compatibility mode in
the future with minor adjustments.
