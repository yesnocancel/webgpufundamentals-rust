Title: WebGPU Environment Maps (reflections)
Description: How to implement environment maps.
TOC: Environment maps

This article continues from [the article on cube maps](webgpu-cube-maps.html).
This article also uses concepts covered in [the article on lighting](webgpu-lighting-directional.html).
If you have not read those articles already you might want to read them first.

An *environment map* represents the environment of the objects you're drawing.
If the you're drawing an outdoor scene it would represent the outdoors. If
you're drawing people on a stage it would represent the venue. If you're drawing
an outer space scene it would be the stars. We can implement an environment map
with a cube map if we have 6 images that show the environment from a point in
space in the 6 directions of the cubemap.

Here's an environment map from the lobby of the Leadenhall Market in London.

<div class="webgpu_center">
  <div class="side-by-side center-by-margin" style="max-width: 800px">
    <div><img src="../resources/images/leadenhall_market/pos-x.jpg" style="min-width: 256px; width: 256px" class="border"><div>positive x</div></div>
    <div><img src="../resources/images/leadenhall_market/neg-x.jpg" style="min-width: 256px; width: 256px" class="border"><div>negative x</div></div>
    <div><img src="../resources/images/leadenhall_market/pos-y.jpg" style="min-width: 256px; width: 256px" class="border"><div>positive y</div></div>
    <div><img src="../resources/images/leadenhall_market/pos-z.jpg" style="min-width: 256px; width: 256px" class="border"><div>positive z</div></div>
    <div><img src="../resources/images/leadenhall_market/neg-z.jpg" style="min-width: 256px; width: 256px" class="border"><div>negative z</div></div>
    <div><img src="../resources/images/leadenhall_market/neg-y.jpg" style="min-width: 256px; width: 256px" class="border"><div>positive y</div></div>
  </div>
</div>
<div class="webgpu_center">
  <a href="https://polyhaven.com/a/leadenhall_market">Leadenhall Market</a>, CC0 by: <a href="https://www.artstation.com/andreasmischok">Andreas Mischok</a>
</div>

Based on [the code in the previous article](webgpu-cube-maps.html) let's load those 6 images instead of the pre-made face images we used there.
From [the article on importing textures](webgpu-importing-textures.html) we had these two pieces. `wgpu_fun::load_image` to load
an image and a function to create a texture from an image.

```rust
  async fn create_texture_from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    url: &str,
    mips: bool,
  ) -> wgpu::Texture {
    let source = wgpu_fun::load_image(url).await;
    create_texture_from_source(device, queue, &source, mips)
  }
```

Let's add and one to load multiple images

```rust
+  async fn create_texture_from_images(
+    device: &wgpu::Device,
+    queue: &wgpu::Queue,
+    urls: &[&str],
+    mips: bool,
+  ) -> wgpu::Texture {
+    let mut images = Vec::new();
+    for url in urls {
+      images.push(wgpu_fun::load_image(url).await);
+    }
+    create_texture_from_sources(device, queue, &images, mips)
+  }

  async fn create_texture_from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    url: &str,
    mips: bool,
  ) -> wgpu::Texture {
-    let source = wgpu_fun::load_image(url).await;
-    create_texture_from_source(device, queue, &source, mips)
+    create_texture_from_images(device, queue, &[url], mips).await
  }
```

While we were at it we also changed the existing function to use
the new one. Now we can use the new one to load the six images.

```rust
-  let texture = create_texture_from_sources(
-      &app.device, &app.queue, &face_sources, true);
+  let texture = create_texture_from_images(
+      &app.device,
+      &app.queue,
+      &[
+        "resources/images/leadenhall_market/pos-x.jpg",
+        "resources/images/leadenhall_market/neg-x.jpg",
+        "resources/images/leadenhall_market/pos-y.jpg",
+        "resources/images/leadenhall_market/neg-y.jpg",
+        "resources/images/leadenhall_market/pos-z.jpg",
+        "resources/images/leadenhall_market/neg-z.jpg",
+      ],
+      true, // mips
+  )
+  .await;
```

In fragment shader we want to know, for each fragment to be drawn, given a vector from
the eye/camera to that position on the surface of the object, which direction
will it reflect off the that surface. We can then use that direction to get a
color from the cubemap.

The formula to reflect is

    reflectionDir = eyeToSurfaceDir –
        2 ∗ dot(surfaceNormal, eyeToSurfaceDir) ∗ surfaceNormal

Thinking about what we can see it's true. Recall from the [lighting articles](webgpu-lighting-directional.html)
that a dot product of 2 vectors returns the cosine of the angle between the 2
vectors. Adding vectors gives us a new vector so let's take the example of an eye
looking directly perpendicular to a flat surface.

<div class="webgpu_center"><img src="resources/reflect-180-01.svg" style="width: 400px"></div>

Let's visualize the formula above. First off recall the dot product of 2 vectors
pointing in exactly opposite directions is -1 so visually

<div class="webgpu_center"><img src="resources/reflect-180-02.svg" style="width: 400px"></div>

Plugging in that dot product with the <span style="color:black; font-weight:bold;">eyeToSurfaceDir</span>
and <span style="color:green;">normal</span> in the reflection formula gives us this

<div class="webgpu_center"><img src="resources/reflect-180-03.svg" style="width: 400px"></div>

Which multiplying -2 by -1 makes it positive 2.

<div class="webgpu_center"><img src="resources/reflect-180-04.svg" style="width: 400px"></div>

So adding the vectors by connecting them up gives us the <span style="color: red">reflected vector</span>

<div class="webgpu_center"><img src="resources/reflect-180-05.svg" style="width: 400px"></div>

We can see above given 2 normals, one completely cancels out the direction from
the eye and the second one points the reflection directly back towards the eye.
Which if we put back in the original diagram is exactly what we'd expect

<div class="webgpu_center"><img src="resources/reflect-180-06.svg" style="width: 400px"></div>

Let's rotate the surface 45 degrees to the right.

<div class="webgpu_center"><img src="resources/reflect-45-01.svg" style="width: 400px"></div>

The dot product of 2 vectors 135 degrees apart is -0.707

<div class="webgpu_center"><img src="resources/reflect-45-02.svg" style="width: 400px"></div>

So plugging everything into the formula

<div class="webgpu_center"><img src="resources/reflect-45-03.svg" style="width: 400px"></div>

Again multiplying 2 negatives gives us a positive but the <span style="color: green">vector</span> is now about 30% shorter.

<div class="webgpu_center"><img src="resources/reflect-45-04.svg" style="width: 400px"></div>

Adding up the vectors gives us the <span style="color: red">reflected vector</span>

<div class="webgpu_center"><img src="resources/reflect-45-05.svg" style="width: 400px"></div>

Which if we put back in the original diagram seems correct.

<div class="webgpu_center"><img src="resources/reflect-45-06.svg" style="width: 400px"></div>

We use that  <span style="color: red">reflected direction</span> to look at the cubemap to color the surface of the object.

Here's a diagram where you can set the rotation of the surface and see the
various parts of the equation. You can also see the reflection vectors point to
the different faces of the cubemap and effect the color of the surface.

{{{diagram url="resources/environment-mapping.html" width="700" height="500" }}}

Now that we know how reflection works and that we can use it to look up values
from the cubemap let's change the shaders to do that.

First in the vertex shader we'll compute the world position and world oriented
normal of the vertices and pass those to the fragment shader as inter-stage variables. This
is similar to what we did in [the article on spotlights](webgpu-3d-lighting-spot.html).

```wgsl
struct Uniforms {
-  matrix: mat4x4f,
+  projection: mat4x4f,
+  view: mat4x4f,
+  world: mat4x4f,
+  cameraPosition: vec3f,
};

struct Vertex {
  @location(0) position: vec4f,
+  @location(1) normal: vec3f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
-  @location(0) normal: vec3f,
+  @location(0) worldPosition: vec3f,
+  @location(1) worldNormal: vec3f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(0) @binding(1) var ourSampler: sampler;
@group(0) @binding(2) var ourTexture: texture_cube<f32>;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
-  vsOut.position = uni.matrix * vert.position;
-  vsOut.normal = normalize(vert.position.xyz);
+  vsOut.position = uni.projection * uni.view * uni.world * vert.position;
+  vsOut.worldPosition = (uni.world * vert.position).xyz;
+  vsOut.worldNormal = (uni.world * vec4f(vert.normal, 0)).xyz;
  return vsOut;
}
```

Then in the fragment shader we normalize the `worldNormal` since it's being
interpolated across the surface between vertices. Based on the matrix math
from [the article on cameras](webgpu-cameras.html) we can get the world position
of the camera by getting the 3rd row of the view matrix and negating it and by subtracting that from the world position of the surface we
get the `eyeToSurfaceDir`.

And finally we use `reflect` which is a built in WGSL function that implements
the formula we went over above. We use the result to get a color from the
cubemap.

```wgsl
@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
+  let worldNormal = normalize(vsOut.worldNormal);
+  let eyeToSurfaceDir = normalize(vsOut.worldPosition - uni.cameraPosition);
+  let direction = reflect(eyeToSurfaceDir, worldNormal);

-  return textureSample(ourTexture, ourSampler, normalize(vsOut.normal));
+  return textureSample(ourTexture, ourSampler, direction);
}
```

We also need real normals for this example. We need real normals so the faces of
the cube appear flat. In the previous example, just to see the cubemap work, we
repurposed the cube's positions but in this case we need actual normals for a
cube like we covered in [the article on lighting](webgpu-lighting-directional.html)

```rust
  let vertex_data: Vec<f32> = vec![
-     // front face
-    -1.0,  1.0,  1.0,
-    -1.0, -1.0,  1.0,
-     1.0,  1.0,  1.0,
-     1.0, -1.0,  1.0,
-     // right face
-     1.0,  1.0, -1.0,
-     1.0,  1.0,  1.0,
-     1.0, -1.0, -1.0,
-     1.0, -1.0,  1.0,
-     // back face
-     1.0,  1.0, -1.0,
-     1.0, -1.0, -1.0,
-    -1.0,  1.0, -1.0,
-    -1.0, -1.0, -1.0,
-    // left face
-    -1.0,  1.0,  1.0,
-    -1.0,  1.0, -1.0,
-    -1.0, -1.0,  1.0,
-    -1.0, -1.0, -1.0,
-    // bottom face
-     1.0, -1.0,  1.0,
-    -1.0, -1.0,  1.0,
-     1.0, -1.0, -1.0,
-    -1.0, -1.0, -1.0,
-    // top face
-    -1.0,  1.0,  1.0,
-     1.0,  1.0,  1.0,
-    -1.0,  1.0, -1.0,
-     1.0,  1.0, -1.0,
+     //  position   |  normals
+     //-------------+----------------------
+     // front face      positive z
+    -1.0,  1.0,  1.0,    0.0,  0.0,  1.0,
+    -1.0, -1.0,  1.0,    0.0,  0.0,  1.0,
+     1.0,  1.0,  1.0,    0.0,  0.0,  1.0,
+     1.0, -1.0,  1.0,    0.0,  0.0,  1.0,
+     // right face      positive x
+     1.0,  1.0, -1.0,    1.0,  0.0,  0.0,
+     1.0,  1.0,  1.0,    1.0,  0.0,  0.0,
+     1.0, -1.0, -1.0,    1.0,  0.0,  0.0,
+     1.0, -1.0,  1.0,    1.0,  0.0,  0.0,
+     // back face       negative z
+     1.0,  1.0, -1.0,    0.0,  0.0, -1.0,
+     1.0, -1.0, -1.0,    0.0,  0.0, -1.0,
+    -1.0,  1.0, -1.0,    0.0,  0.0, -1.0,
+    -1.0, -1.0, -1.0,    0.0,  0.0, -1.0,
+    // left face        negative x
+    -1.0,  1.0,  1.0,   -1.0,  0.0,  0.0,
+    -1.0,  1.0, -1.0,   -1.0,  0.0,  0.0,
+    -1.0, -1.0,  1.0,   -1.0,  0.0,  0.0,
+    -1.0, -1.0, -1.0,   -1.0,  0.0,  0.0,
+    // bottom face      negative y
+     1.0, -1.0,  1.0,    0.0, -1.0,  0.0,
+    -1.0, -1.0,  1.0,    0.0, -1.0,  0.0,
+     1.0, -1.0, -1.0,    0.0, -1.0,  0.0,
+    -1.0, -1.0, -1.0,    0.0, -1.0,  0.0,
+    // top face         positive y
+    -1.0,  1.0,  1.0,    0.0,  1.0,  0.0,
+     1.0,  1.0,  1.0,    0.0,  1.0,  0.0,
+    -1.0,  1.0, -1.0,    0.0,  1.0,  0.0,
+     1.0,  1.0, -1.0,    0.0,  1.0,  0.0,
  ];
```

And of course we need to change our pipeline to provide the normals

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("2 attributes"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (3) * 4, // (3) floats 4 bytes each
+        array_stride: (3 + 3) * 4, // (6) floats 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
+          // normal
+          wgpu::VertexAttribute {
+            shader_location: 1,
+            offset: 12,
+            format: wgpu::VertexFormat::Float32x3,
+          },
        ],
      })],
    },

```

As usual we need to setup our uniform buffer. Where the JavaScript version
makes `Float32Array` views into one larger `Float32Array`, in Rust we'll keep
one `[f32; N]` array and some offsets into it, and copy each value into its
slice of the array.

```rust
-  // matrix
-  let uniform_buffer_size = 16 * 4;
+  // projection, view, world, cameraPosition, pad
+  const UNIFORM_BUFFER_SIZE: u64 = (16 + 16 + 16 + 3 + 1) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
-    size: uniform_buffer_size,
+    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

+  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];
+
+  // offsets to the various uniform values in float32 indices
+  const K_PROJECTION_OFFSET: usize = 0;
+  const K_VIEW_OFFSET: usize = 16;
+  const K_WORLD_OFFSET: usize = 32;
+  const K_CAMERA_POSITION_OFFSET: usize = 48;
```

And we need to set them at render time

```rust
    let aspect = frame.width as f32 / frame.height as f32;
-    let matrix = Mat4::perspective_rh(
+    let projection = Mat4::perspective_rh(
        60.0f32.to_radians(),
        aspect,
        0.1,  // zNear
        10.0, // zFar
-    ) * Mat4::look_at_rh(
-        Vec3::new(0.0, 1.0, 5.0), // camera position
-        Vec3::new(0.0, 0.0, 0.0), // target
-        Vec3::new(0.0, 1.0, 0.0), // up
-    ) * Mat4::from_rotation_x(rotation[0])
-        * Mat4::from_rotation_y(rotation[1])
-        * Mat4::from_rotation_z(rotation[2]);
+    );
+    let camera_position = Vec3::new(0.0, 0.0, 4.0); // camera position
+    let view = Mat4::look_at_rh(
+        camera_position,
+        Vec3::new(0.0, 0.0, 0.0), // target
+        Vec3::new(0.0, 1.0, 0.0), // up
+    );
+    let world = Mat4::from_rotation_x(time * -0.1) * Mat4::from_rotation_y(time * -0.2);
+
+    uniform_values[K_PROJECTION_OFFSET..K_PROJECTION_OFFSET + 16]
+        .copy_from_slice(&projection.to_cols_array());
+    uniform_values[K_VIEW_OFFSET..K_VIEW_OFFSET + 16].copy_from_slice(&view.to_cols_array());
+    uniform_values[K_WORLD_OFFSET..K_WORLD_OFFSET + 16].copy_from_slice(&world.to_cols_array());
+    uniform_values[K_CAMERA_POSITION_OFFSET..K_CAMERA_POSITION_OFFSET + 3]
+        .copy_from_slice(&camera_position.to_array());

    // upload the uniform values to the uniform buffer
    frame
        .queue
-        .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&matrix.to_cols_array()));
+        .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

Let's also change the rendering to a continuous animation. The previous
example rendered once and had a settings panel to rotate the cube; this one
rotates by time instead so we drop the settings and switch the render mode to
`Continuous`, which renders every frame like a JavaScript
`requestAnimationFrame` loop. `frame.time` is the seconds since the example
started.

```rust
-  app.run(RenderMode::Once, move |frame: &Frame| {
+  app.run(RenderMode::Continuous, move |frame: &Frame| {
+    let time = frame.time as f32;

     ...

-    let rotation = [
-        wgpu_fun::setting_f64("rotationX", 20.0f64.to_radians()) as f32,
-        wgpu_fun::setting_f64("rotationY", 25.0f64.to_radians()) as f32,
-        wgpu_fun::setting_f64("rotationZ", 0.0) as f32,
-    ];
```

And with that we get.

{{{example url="../webgpu-environment-map-backward.html" }}}

If you look closely you might see a small problem.

<div class="webgpu_center"><img src="resources/environment-map-backward.png" class="nobg" style="width: 600px;"></div>

## <a id="a-flipped"></a> Correcting the reflection direction

Our cube with an environment map applied
to it represents a mirrored cube. But a mirror normally shows
things flipped horizontally. What's going on?

The issue is, we're on the inside of the cube looking out, but
recall from [the previous article](webgpu-cube-maps.html), when
we mapped textures to each side of the cube they mapped correctly
when viewed from the outside.

<div class="webgpu_center">
  <div data-diagram="show-cube-map" class="center-by-margin" style="width: 700px; height: 400px"></div>
</div>

Another way to look at this is, from inside the cube we're in a "y-up right handed coordinate system".
This means positive-z is forward. Where as all of our 3d math so far uses a "y-up left handed coordinate system" [^xxx-handed]
where negative-z is forward. A simple solution is to flip the Z coordinate when we sample the
texture.

[^xxx-handed]: To be honest I find this talk of "left handed" vs "right handed" coordinate systems to be super confusing
and I'd much rather say "+x to the right, +y up, -z forward", which leaves zero ambiguity. If you want to know more
though you can [google it](https://www.google.com/search?q=right+handed+vs+left+handed+coordinate+system&tbm=isch) 😄

```wgsl
-  return textureSample(ourTexture, ourSampler, direction);
+  return textureSample(ourTexture, ourSampler, direction * vec3f(1, 1, -1));
```

Now the reflection is flipped, just like in a mirror.

{{{example url="../webgpu-environment-map.html" }}}

Next let's show [how to use a cubemap for a skybox](webgpu-skybox.html).

## Finding and Making Cube Maps

You can find hundreds of free panoramas at [polyhaven.com](https://polyhaven.com/hdris).
Download a jpg or png of any one of them (click the ≡ menu in the top right). Then, go to
[this page](https://greggman.github.io/panorama-to-cubemap/) and drag and drop the .jpg or .png
file there. Select the size and format you want and click the button to save the images
as cubemap faces.

<!-- keep this at the bottom of the article -->
<script type="module" src="webgpu-environment-maps.js"></script>
