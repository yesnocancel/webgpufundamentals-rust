Title: WebGPU Cubemaps
Description: How to use cubemaps in WebGPU
TOC: Cube Maps

This article is one in a series of the various ways to provide data
to a shader. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="passing-data.hanson"}}}

This article assumes you've read [the article on textures](webgpu-textures.html) and [the article on importing images into textures](webgpu-importing-textures.html).
This article also uses concepts covered in [the article on directional lighting](webgpu-lighting-directional.html).
If you have not read those articles already you might want to read them first.

In a [previous article](webgpu-textures.html) we covered how to use textures,
how they are referenced by texture coordinates that go from 0 to 1 across and up
the texture, and how they are filtered optionally using mips.

Another kind of texture is a *cubemap*. A cubemap consists of 6 faces representing
the 6 faces of a cube. Instead of the traditional texture coordinates that
have 2 dimensions, a cubemap uses a normal or in other words a 3D direction.
Depending on the direction the normal points one of the 6 faces of the cube
is selected and then within that face the pixels are sampled to produce a color.

Let's make a simple example. The JS version of this site uses a 2D canvas to
make the images used in each of the 6 faces; there's no 2D canvas outside the
browser, so for the Rust version we made those same 6 images ahead of time
(colored squares with a centered label) and we simply load them.

For reference, here's the JS code that draws one face with a canvas, filling
it with a color and a centered message

```js
function generateFace(size, {faceColor, textColor, text}) {
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d');
  ctx.fillStyle = faceColor;
  ctx.fillRect(0, 0, size, size);
  ctx.font = `${size * 0.7}px sans-serif`;
  ctx.fillStyle = textColor;
  ctx.textAlign = 'left';
  ctx.textBaseline = 'top';
  const m = ctx.measureText(text);
  ctx.fillText(
    text,
    (size - m.actualBoundingBoxRight + m.actualBoundingBoxLeft) / 2,
    (size - m.actualBoundingBoxDescent + m.actualBoundingBoxAscent) / 2
  );
  return canvas;
}
```

These are the 6 face images, 128x128 each, stored under
`resources/images/cube-faces/`

<div class="webgpu_center">

| file | face color | text color | text |
| ---- | ---------- | ---------- | ---- |
| pos-x.png | red     | cyan    | +X |
| neg-x.png | yellow  | blue    | -X |
| pos-y.png | green   | magenta | +Y |
| neg-y.png | cyan    | red     | -Y |
| pos-z.png | blue    | yellow  | +Z |
| neg-z.png | magenta | green   | -Z |

</div>

In the Rust code we load them like any other images

```rust
let face_urls = [
    "resources/images/cube-faces/pos-x.png", // +X
    "resources/images/cube-faces/neg-x.png", // -X
    "resources/images/cube-faces/pos-y.png", // +Y
    "resources/images/cube-faces/neg-y.png", // -Y
    "resources/images/cube-faces/pos-z.png", // +Z
    "resources/images/cube-faces/neg-z.png", // -Z
];
let mut face_sources = Vec::new();
for url in face_urls {
    face_sources.push(wgpu_fun::load_image(url).await);
}
```

{{{example url="../webgpu-cube-faces.html" }}}

Now let's apply those to a cube using a cubemap. We'll start with the code
from the texture atlas example [in the article on importing textures](webgpu-importing-textures.html#a-texture-atlases).

First off let's change the shaders to use a cube map

```wgsl
struct Uniforms {
  matrix: mat4x4f,
};

struct Vertex {
  @location(0) position: vec4f,
-  @location(1) texcoord: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
-  @location(0) texcoord: vec2f,
+  @location(0) normal: vec3f,
};

...

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = uni.matrix * vert.position;
-  vsOut.texcoord = vert.texcoord;
+  vsOut.normal = normalize(vert.position.xyz);
  return vsOut;
}
```

We've removed the texture coordinates from the shader and
changed the inter-stage variable to pass a normal to the fragment shader.
Since the positions of our cube are perfectly centered around the origin
we can just use them as our normals.

Recall from [the article on lighting](webgpu-lighting-directional.html) that
normals are a direction and are usually used to specify the direction of
the surface of some vertex. Because we are using the normalized positions
for our normals if we were to light this we'd get smooth lighting across
the cube.

{{{diagram url="resources/cube-normals.html" caption="standard cube normals vs this cube's normals" width="700" height="400"}}}

Since we're not using texture coordinates we can remove all code related to
setting up the texture coordinates.

```rust
  let vertex_data: Vec<f32> = vec![
-     // front face     select the top left image
-    -1,  1,  1,        0   , 0  ,
-    -1, -1,  1,        0   , 0.5,
-     1,  1,  1,        0.25, 0  ,
-     1, -1,  1,        0.25, 0.5,
-     // right face     select the top middle image
-     1,  1, -1,        0.25, 0  ,
-     1,  1,  1,        0.5 , 0  ,
-     1, -1, -1,        0.25, 0.5,
-     1, -1,  1,        0.5 , 0.5,
-     // back face      select to top right image
-     1,  1, -1,        0.5 , 0  ,
-     1, -1, -1,        0.5 , 0.5,
-    -1,  1, -1,        0.75, 0  ,
-    -1, -1, -1,        0.75, 0.5,
-    // left face       select the bottom left image
-    -1,  1,  1,        0   , 0.5,
-    -1,  1, -1,        0.25, 0.5,
-    -1, -1,  1,        0   , 1  ,
-    -1, -1, -1,        0.25, 1  ,
-    // bottom face     select the bottom middle image
-     1, -1,  1,        0.25, 0.5,
-    -1, -1,  1,        0.5 , 0.5,
-     1, -1, -1,        0.25, 1  ,
-    -1, -1, -1,        0.5 , 1  ,
-    // top face        select the bottom right image
-    -1,  1,  1,        0.5 , 0.5,
-     1,  1,  1,        0.75, 0.5,
-    -1,  1, -1,        0.5 , 1  ,
-     1,  1, -1,        0.75, 1  ,
+     // front face
+    -1,  1,  1,
+    -1, -1,  1,
+     1,  1,  1,
+     1, -1,  1,
+     // right face
+     1,  1, -1,
+     1,  1,  1,
+     1, -1, -1,
+     1, -1,  1,
+     // back face
+     1,  1, -1,
+     1, -1, -1,
+    -1,  1, -1,
+    -1, -1, -1,
+    // left face
+    -1,  1,  1,
+    -1,  1, -1,
+    -1, -1,  1,
+    -1, -1, -1,
+    // bottom face
+     1, -1,  1,
+    -1, -1,  1,
+     1, -1, -1,
+    -1, -1, -1,
+    // top face
+    -1.0,  1.0,  1.0,
+     1.0,  1.0,  1.0,
+    -1.0,  1.0, -1.0,
+     1.0,  1.0, -1.0,
  ];

  ...

  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("1 attribute"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (3 + 2) * 4, // (3+2) floats 4 bytes each
+        array_stride: (3) * 4, // (3) floats 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
-          // texcoord
-          wgpu::VertexAttribute {
-            shader_location: 1,
-            offset: 12,
-            format: wgpu::VertexFormat::Float32x2,
-          },
        ],
      })],
    },
    ...
  });
```

In the fragment shader we need to use a `texture_cube` instead of a `texture_2d`
and `textureSample` when used with a `texture_cube` takes a `vec3f` direction
so we pass the normal. Since the normal is a inter-stage variable and will be interpolated
we need to normalize it.

```wgsl
@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(0) @binding(1) var ourSampler: sampler;
-@group(0) @binding(2) var ourTexture: texture_2d<f32>;
+@group(0) @binding(2) var ourTexture: texture_cube<f32>;

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
-  return textureSample(ourTexture, ourSampler, vsOut.texcoord);
+  return textureSample(ourTexture, ourSampler, normalize(vsOut.normal));
}
```

To actually make a cube map we make a 2D texture with 6 layers. Let's change all our helpers
so they handle multiple sources.

## <a id="a-texture-helpers"></a> Making our texture helpers handle multiple layers

First let's take our `create_texture_from_source` and change it to `create_texture_from_sources`
where it takes a slice of sources

```rust
-  fn create_texture_from_source(
+  fn create_texture_from_sources(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
-    source: &ImageData,
+    sources: &[ImageData],
    mips: bool,
  ) -> wgpu::Texture {
+    // Assume all sources are the same size so just use the first one for width and height
+    let source = &sources[0];
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      format: wgpu::TextureFormat::Rgba8Unorm,
      mip_level_count: if mips { num_mip_levels(&[source.width, source.height]) } else { 1 },
      size: wgpu::Extent3d {
        width: source.width,
        height: source.height,
-        depth_or_array_layers: 1,
+        depth_or_array_layers: sources.len() as u32,
      },
      usage: wgpu::TextureUsages::TEXTURE_BINDING
          | wgpu::TextureUsages::COPY_DST
          | wgpu::TextureUsages::RENDER_ATTACHMENT,
      ...
    });
-    copy_source_to_texture(device, queue, &texture, source);
+    copy_sources_to_texture(device, queue, &texture, sources);
    texture
  }
```

The code above makes a texture where multiple layers, one for each source.
It also assumes all the sources are the same size. This seems like a good bet
because it would be very rare for them to be different sizes for layers of the same texture.

Now we need to update `copy_source_to_texture` to handle multiple sources.

```rust
-  fn copy_source_to_texture(device, queue, texture, source: &ImageData) {
+  fn copy_sources_to_texture(device, queue, texture, sources: &[ImageData]) {
+    for (layer, source) in sources.iter().enumerate() {
*      queue.write_texture(
*        wgpu::TexelCopyTextureInfo {
*          texture,
*          mip_level: 0,
-          origin: wgpu::Origin3d::ZERO,
+          origin: wgpu::Origin3d { x: 0, y: 0, z: layer as u32 },
*          aspect: wgpu::TextureAspect::All,
*        },
*        &source.data,
*        ...
*      );
+    }

    if texture.mip_level_count() > 1 {
      generate_mips(device, queue, texture);
    }
  }
```

Above, the only major difference is we added a loop to loop over the sources
and we set an `origin` for where in the texture to copy the source so that
we copy each source to its respective layer.

Now we need to update `generateMips` to handle multiple sources.

```rust
  fn generate_mips(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    // module / sampler / pipeline caching unchanged ...

    for base_mip_level in 1..texture.mip_level_count() {
+      for layer in 0..texture.depth_or_array_layers() {
*        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
*          layout: &pipeline.get_bind_group_layout(0),
*          entries: &[
*            wgpu::BindGroupEntry {
*              binding: 0,
*              resource: wgpu::BindingResource::Sampler(sampler),
*            },
-            wgpu::BindGroupEntry {
-              binding: 1,
-              resource: wgpu::BindingResource::TextureView(&texture.create_view(
-                &wgpu::TextureViewDescriptor {
-                  base_mip_level: base_mip_level - 1,
-                  mip_level_count: Some(1),
-                  ..Default::default()
-                },
-              )),
-            },
+            wgpu::BindGroupEntry {
+              binding: 1,
+              resource: wgpu::BindingResource::TextureView(&texture.create_view(
+                &wgpu::TextureViewDescriptor {
+                  dimension: Some(wgpu::TextureViewDimension::D2),
+                  base_mip_level: base_mip_level - 1,
+                  mip_level_count: Some(1),
+                  base_array_layer: layer,
+                  array_layer_count: Some(1),
+                  ..Default::default()
+                },
+              )),
+            },
*          ],
*        });
*
-        let view = texture.create_view(&wgpu::TextureViewDescriptor {
-          base_mip_level,
-          mip_level_count: Some(1),
-          ..Default::default()
-        });
+        let view = texture.create_view(&wgpu::TextureViewDescriptor {
+          dimension: Some(wgpu::TextureViewDimension::D2),
+          base_mip_level,
+          mip_level_count: Some(1),
+          base_array_layer: layer,
+          array_layer_count: Some(1),
+          ..Default::default()
+        });
*        {
*          let mut pass = encoder.begin_render_pass(/* renders to view */);
*          pass.set_pipeline(pipeline);
*          pass.set_bind_group(0, &bind_group, &[]);
*          pass.draw(0..6, 0..1);  // call our vertex shader 6 times
*        }
+      }
    }

    let command_buffer = encoder.finish();
    queue.submit([command_buffer]);
  }
```

We added a loop to handle each layer of the texture.
We changed the views so they select a single layer. We also had to explicitly choose
`dimension: '2d'` for our views because by default, a view of a 2d texture with more than
1 layer gets the `dimension: '2d-array'` which for the purpose of generating
mipmaps is not what we want.

> Note: [The article on compatibility mode](webgpu-compatibility-mode.html) provides
> a version of `generateMips` that works in compatibility mode.

Although we won't use them here, our original `create_texture_from_source` and
`copy_source_to_texture` functions can easily be replaced with

```rust
  fn copy_source_to_texture(device, queue, texture, source: &ImageData) {
    copy_sources_to_texture(device, queue, texture, std::slice::from_ref(source));
  }

  fn create_texture_from_source(device, queue, source: &ImageData, mips: bool) -> wgpu::Texture {
    create_texture_from_sources(device, queue, std::slice::from_ref(source), mips)
  }
```

Now that we have these ready we can use the face images from the top of the article

```rust
  let texture = create_texture_from_sources(
      &app.device, &app.queue, &face_sources, true);
```

All that's left to do is change our texture's view in the bind group

```rust
  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bind group for object"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
      wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
-      wgpu::BindGroupEntry {
-        binding: 2,
-        resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())),
-      },
+      wgpu::BindGroupEntry {
+        binding: 2,
+        resource: wgpu::BindingResource::TextureView(&texture.create_view(
+          &wgpu::TextureViewDescriptor {
+            dimension: Some(wgpu::TextureViewDimension::Cube),
+            ..Default::default()
+          },
+        )),
+      },
    ],
  });
```

And poof

{{{example url="../webgpu-cube-map.html" }}}

Note the order of the faces as layers of the texture

* layer 0 => positive x
* layer 1 => negative x
* layer 2 => positive y
* layer 3 => negative y
* layer 4 => positive z
* layer 5 => negative z

Another way to think about this is if you called `textureSample` and passed
the corresponding directions it would return the center pixel(s) color for that layer
of the texture.

* `textureSample(tex, sampler, vec3f( 1, 0, 0))` => center of layer 0
* `textureSample(tex, sampler, vec3f(-1, 0, 0))` => center of layer 1
* `textureSample(tex, sampler, vec3f( 0, 1, 0))` => center of layer 2
* `textureSample(tex, sampler, vec3f( 0,-1, 0))` => center of layer 3
* `textureSample(tex, sampler, vec3f( 0, 0, 1))` => center of layer 4
* `textureSample(tex, sampler, vec3f( 0, 0,-1))` => center of layer 5

Using a cubemap to texture a cube is **not** what cubemaps are normally
used for. The *correct* or rather standard way to texture a cube is
to use a texture atlas like we [mentioned before](webgpu-importing-textures.html#a-texture-atlases).
The point of this article was to introduce the concept of cube map and show how you pass it
directions (normals) and it returns the color of the cube in that direction.

Now that we've learned what a cubemap is and how to set one up what is a cubemap
used for? Probably the single most common thing a cubemap is used for is as an
[*environment map*](webgpu-environment-maps.html).

