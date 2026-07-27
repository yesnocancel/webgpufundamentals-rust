Title: WebGPU Speed and Optimization
Description: How to go faster in WebGPU
TOC: Speed and Optimization

Most of the examples on this site are written to be as understandable as
possible. That means they work, and they're correct, but they don't necessarily
show the most efficient way to do something in WebGPU. Further, depending on
what you need to do, there are a myriad of possible optimizations.

In this article will cover some of the most basic optimizations and discuss a
few others. To be clear, IMO, **you don't usually need to go this far. Most of
the examples around the net using WebGPU draw a couple of hundred things and so
really wouldn't benefit from these optimizations**. Still, it's always good to
know how to make things go faster.

One note up front: the performance numbers mentioned in this article are the
ones the author of the original JavaScript article measured on his own
machines, running the JavaScript versions. The absolute numbers you get from
these Rust/wasm versions will differ, but the *relative* improvements — the
whole point of the article — apply just the same.

The basics: **The less work you do, and the less work you ask WebGPU to do the
faster things will go.**

In pretty much all of the examples to date, if we draw multiple shapes we've
done the following steps

* At Init time:
   * for each thing we want to draw
      * create a uniform buffer
      * create a bindGroup that references that buffer

* At Render time:
   * start an encoder and render pass
   * for each thing we want to draw
      * update an array with our uniform values for this object
      * copy the array to the uniform buffer for this object
      * set any pipeline, vertex and index buffers if needed
      * encode a command(s) to bind the bindGroup(s) for this object
      * encode a command to draw 
   * end the render pass, finish the encoder, submit the command buffer

Let's make an example we can optimize that follows the steps above so we can
then optimize it.

Note, this a fake example. We are only going to draw a bunch of cubes and as
such we could certainly optimize things by using *instancing* which we covered
in the articles on [storage buffers](webgpu-storage-buffers.html#a-instancing)
and [vertex buffers](webgpu-vertex-buffers.html#a-instancing). I didn't want to
clutter the code by handling tons of different kinds of objects. Instancing is
certainly a great way to optimize if your project uses lots of the same model.
Plants, trees, rocks, trash, etc are often optimized by using instancing. For
other models, it's arguably less common.

For example a table might have 4, 6 or 8 chairs around it and it would probably
be faster to use instancing to draw those chairs, except in a list of 500+
things to draw, if the chairs are the only exceptions, then it's probably not
worth the effort to figure out some optimal data organization that some how
organizes the chairs to use instancing but finds no other situations to use
instancing.

The point of the paragraph above is, use instancing when it's appropriate. If
you are going to draw hundreds or more of the same thing than instancing is
probably appropriate. If you are going to only draw a few of the same thing then
it's probably not worth the effort to special case those few things.

In any case, here's our code. We've got the initialization code we've been using
in general. (The original JavaScript asks for a `'high-performance'` adapter;
`wgpu_fun`'s `App` takes care of the adapter, device and surface setup for us.)

```rust
use glam::{Mat3, Mat4, Vec3};
use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
  let mut app = App::new("WebGPU Optimization - None").await;
  app.auto_resize = true;
  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
```

Then let's make a shader module.

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
      struct Uniforms {
        normalMatrix: mat3x3f,
        viewProjection: mat4x4f,
        world: mat4x4f,
        color: vec4f,
        lightWorldPosition: vec3f,
        viewWorldPosition: vec3f,
        shininess: f32,
      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) normal: vec3f,
        @location(2) texcoord: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) normal: vec3f,
        @location(1) surfaceToLight: vec3f,
        @location(2) surfaceToView: vec3f,
        @location(3) texcoord: vec2f,
      };

      @group(0) @binding(0) var diffuseTexture: texture_2d<f32>;
      @group(0) @binding(1) var diffuseSampler: sampler;
      @group(0) @binding(2) var<uniform> uni: Uniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = uni.viewProjection * uni.world * vert.position;

        // Orient the normals and pass to the fragment shader
        vsOut.normal = uni.normalMatrix * vert.normal;

        // Compute the world position of the surface
        let surfaceWorldPosition = (uni.world * vert.position).xyz;

        // Compute the vector of the surface to the light
        // and pass it to the fragment shader
        vsOut.surfaceToLight = uni.lightWorldPosition - surfaceWorldPosition;

        // Compute the vector of the surface to the view
        // and pass it to the fragment shader
        vsOut.surfaceToView = uni.viewWorldPosition - surfaceWorldPosition;

        // Pass the texture coord on to the fragment shader
        vsOut.texcoord = vert.texcoord;

        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        // Because vsOut.normal is an inter-stage variable 
        // it's interpolated so it will not be a unit vector.
        // Normalizing it will make it a unit vector again
        let normal = normalize(vsOut.normal);

        let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
        let surfaceToViewDirection = normalize(vsOut.surfaceToView);
        let halfVector = normalize(
          surfaceToLightDirection + surfaceToViewDirection);

        // Compute the light by taking the dot product
        // of the normal with the direction to the light
        let light = dot(normal, surfaceToLightDirection);

        var specular = dot(normal, halfVector);
        specular = select(
            0.0,                           // value if condition is false
            pow(specular, uni.shininess),  // value if condition is true
            specular > 0.0);               // condition

        let diffuse = uni.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
        // Lets multiply just the color portion (not the alpha)
        // by the light
        let color = diffuse.rgb * light + specular;
        return vec4f(color, diffuse.a);
      }
    "#
      .into(),
    ),
  });
```

This shader module is uses lighting similar to
[the point light with specular highlights covered else where](webgpu-lighting-point.html#a-specular).
It uses a texture because most 3d models use textures so I thought it best to include one.
It multiplies the texture by a color so we can adjust the colors of each cube.
And it has all of the uniform values we need to do the lighting and
[project the cube in 3d](webgpu-perspective-projection.html).

We need data for a cube and to put that data in buffers.

```rust
  fn create_buffer_with_data(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[u8],
    usage: wgpu::BufferUsages,
  ) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: data.len() as u64,
      usage: usage | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, data);
    buffer
  }

  let positions: Vec<f32> = vec![1.0, 1.0, -1.0, 1.0, 1.0, 1.0, /* ...see the full source... */ -1.0, -1.0, -1.0];
  let normals: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0, /* ...see the full source... */ 0.0, 0.0, -1.0];
  let texcoords: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0, /* ...see the full source... */ 1.0, 1.0];
  let indices: Vec<u16> = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23];

  let position_buffer = create_buffer_with_data(&app.device, &app.queue, bytemuck::cast_slice(&positions), wgpu::BufferUsages::VERTEX);
  let normal_buffer = create_buffer_with_data(&app.device, &app.queue, bytemuck::cast_slice(&normals), wgpu::BufferUsages::VERTEX);
  let texcoord_buffer = create_buffer_with_data(&app.device, &app.queue, bytemuck::cast_slice(&texcoords), wgpu::BufferUsages::VERTEX);
  let indices_buffer = create_buffer_with_data(&app.device, &app.queue, bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX);
  let num_vertices = indices.len() as u32;
```

We need a render pipeline

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("textured model with point light w/specular highlight"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[
        // position
        Some(wgpu::VertexBufferLayout {
          array_stride: 3 * 4, // 3 floats
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          }],
        }),
        // normal
        Some(wgpu::VertexBufferLayout {
          array_stride: 3 * 4, // 3 floats
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[wgpu::VertexAttribute {
            shader_location: 1,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          }],
        }),
        // uvs
        Some(wgpu::VertexBufferLayout {
          array_stride: 2 * 4, // 2 floats
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &[wgpu::VertexAttribute {
            shader_location: 2,
            offset: 0,
            format: wgpu::VertexFormat::Float32x2,
          }],
        }),
      ],
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

The pipeline above uses 1 buffer per attribute. One for position data, one for
normal data, and one for texture coordinates (UVs). It culls back facing
triangles, and it expects a depth texture for depth testing. All things we've
covered in other articles.

Let's insert a few utilities for making colors and random numbers. The
JavaScript version builds a CSS `hsl()` color string and reads the resulting
color back through a 2D canvas; there's no CSS to lean on here so we do the
same hsl → rgb conversion directly.

```rust
/// Given hue, saturation, and luminance values in the range of 0 to 1
/// returns an array of 4 values from 0 to 1.
/// (The JS version makes a CSS `hsl()` string and reads the color back
/// through a 2D canvas; we do the same hsl → rgb conversion directly.)
fn hsl_to_rgba(h: f32, s: f32, l: f32) -> [f32; 4] {
  let h = (h * 360.0).floor(); // the JS `h * 360 | 0`
  let l = (l * 100.0).floor() / 100.0; // the JS `l * 100 | 0`
  let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
  let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
  let m = l - c / 2.0;
  let (r, g, b) = match (h as u32) / 60 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  // the 2D canvas readback quantizes to 8 bits
  [
    ((r + m) * 255.0).round() / 255.0,
    ((g + m) * 255.0).round() / 255.0,
    ((b + m) * 255.0).round() / 255.0,
    1.0,
  ]
}

/// Returns a random number between min and max.
/// (The JS version also has 0 and 1 argument forms; we always pass a range.)
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

/// Selects a random array element
fn random_array_element<T>(arr: &[T]) -> &T {
  &arr[(rand(0.0, 1.0) * arr.len() as f32) as usize % arr.len()]
}
```

Hopefully they are all pretty straight forward.

Now let's make some textures and a sampler. The JavaScript version draws an
emoji into a canvas and then uses the `createTextureFromSource` function from
[the article on importing textures](webgpu-importing-textures.html) to make a
texture (with mips) from it. There's no 2D canvas to draw emoji into outside
the browser, so we load pre-made 128x128 images of the same emoji and pass
them to the `create_texture_from_source` helper we wrote in that same article.

```rust
  // The JS version draws each emoji into a 2D canvas and creates a texture
  // from it; there is no 2D canvas outside the browser so we load pre-made
  // 128x128 images of the same emoji.
  let mut textures = Vec::new();
  for url in [
    "resources/images/emoji/face-with-tears-of-joy.png", // 😂
    "resources/images/emoji/alien-monster.png",          // 👾
    "resources/images/emoji/thumbs-up.png",              // 👍
    "resources/images/emoji/eyes.png",                   // 👀
    "resources/images/emoji/sun-with-face.png",          // 🌞
    "resources/images/emoji/ring-buoy.png",              // 🛟
  ] {
    let source = wgpu_fun::load_image(url).await;
    textures.push(create_texture_from_source(&app.device, &app.queue, &source, true));
  }

  let sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    mipmap_filter: wgpu::MipmapFilterMode::Nearest,
    ..Default::default()
  });
```

Let's create a set of material info. We haven't done this anywhere else but it's
a common setup. Unity, Unreal, Blender, Three.js, Babylon,js all have a concept
of a *material*. Generally, a material holds things like the color of the
material, how shiny it is, as well as which texture to use, etc...

We'll make 20 "materials" and then pick a material at random for each cube.

```rust
#[derive(Clone)]
struct Material {
  color: [f32; 4],
  shininess: f32,
  texture: wgpu::Texture,
  sampler: wgpu::Sampler,
}

  ...

  let num_materials = 20;
  let mut materials: Vec<Material> = Vec::new();
  for _ in 0..num_materials {
    let color = hsl_to_rgba(rand(0.0, 1.0), rand(0.5, 0.8), rand(0.5, 0.7));
    let shininess = rand(10.0, 120.0);
    materials.push(Material {
      color,
      shininess,
      texture: random_array_element(&textures).clone(),
      sampler: sampler.clone(),
    });
  }
```

(cloning a `wgpu::Texture` or `wgpu::Sampler` just clones a handle to the
same GPU resource, like copying a reference in JavaScript)

Now let's make data for each thing (cube) we want to draw. We'll support a
maximum of 30000. Like we have in the past, we'll make a uniform buffer for each
object as well as an array of `f32`s we can update with uniform values. We'll
also make a bind group for each object. And we'll pick some random values we can
use to position and animate each object.

Where the JavaScript makes named `subarray` views into one `Float32Array`, in
Rust we keep one `Vec<f32>` per object and a set of offset constants to index
it with.

```rust
struct ObjectInfo {
  bind_group: wgpu::BindGroup,

  uniform_buffer: wgpu::Buffer,
  uniform_values: Vec<f32>,

  axis: Vec3,
  material: Material,
  radius: f32,
  speed: f32,
  rotation_speed: f32,
  scale: f32,
}

  ...

  let max_objects = 30000;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  // offsets to the various uniform values in float32 indices
  const K_NORMAL_MATRIX_OFFSET: usize = 0;
  const K_VIEW_PROJECTION_OFFSET: usize = 12;
  const K_WORLD_OFFSET: usize = 28;
  const K_COLOR_OFFSET: usize = 44;
  const K_LIGHT_WORLD_POSITION_OFFSET: usize = 48;
  const K_VIEW_WORLD_POSITION_OFFSET: usize = 52;
  const K_SHININESS_OFFSET: usize = 55;

  for _ in 0..max_objects {
    let uniform_buffer_size = (12 + 16 + 16 + 4 + 4 + 4) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms"),
      size: uniform_buffer_size,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    // (in JS this is a Float32Array with per-value subarray views; in
    // Rust we keep one Vec and index it with the offsets above)
    let uniform_values = vec![0.0f32; uniform_buffer_size as usize / 4];

    let material = random_array_element(&materials).clone();

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("bind group for object"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&material.texture.create_view(&Default::default())),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&material.sampler),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: uniform_buffer.as_entire_binding(),
        },
      ],
    });

    let axis = Vec3::new(rand(-1.0, 1.0), rand(-1.0, 1.0), rand(-1.0, 1.0)).normalize();
    let radius = rand(10.0, 100.0);
    let speed = rand(0.1, 0.4);
    let rotation_speed = rand(-1.0, 1.0);
    let scale = rand(2.0, 10.0);

    object_infos.push(ObjectInfo {
      bind_group,

      uniform_buffer,
      uniform_values,

      axis,
      material,
      radius,
      speed,
      rotation_speed,
      scale,
    });
  }
```

In JavaScript, a render pass descriptor is pre-created and its `view`
properties filled out each frame. Rust descriptors borrow the views they
reference so we'll just build the descriptor in the frame callback each frame
— it describes the same render pass.

We need a simple UI so we can adjust how many things we're drawing. Like the
other examples with a settings panel, the page keeps the muigui panel in page
JavaScript; its onChange handler calls into the wasm module and our frame code
reads the current value with `wgpu_fun::setting_f64`.

```js
const settings = {
  numObjects: 1000,
};

const gui = new GUI();
gui.add(settings, 'numObjects', { min: 0, max: maxObjects, step: 1})
  .onChange(v => wasm.set_setting_num('numObjects', v));
```

Now we can write our render loop.

```rust
  let mut depth_texture: Option<wgpu::Texture> = None;
  let mut then = 0.0;

  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let time = frame.time; // seconds
    let delta_time = time - then;
    then = time;
    let time = time as f32;

    // read the settings the GUI on the page sets
    let num_objects = (wgpu_fun::setting_f64("numObjects", 1000.0) as usize).min(max_objects);

    ...
  });
```

Inside the render loop we'll create a depth texture if one doesn't exist or if
the one we have has a different size then our canvas texture. We did this in
[the article on 3d](webgpu-orthographic-projection.html#a-depth-textures).

```rust
    // If we don't have a depth texture OR if its size is different
    // from the target texture make a new depth texture
    if depth_texture
      .as_ref()
      .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
    {
      if let Some(t) = depth_texture.take() {
        t.destroy();
      }
      depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
          width: frame.width,
          height: frame.height,
          depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
      }));
    }
    let depth_view = depth_texture.as_ref().unwrap().create_view(&Default::default());
```

We'll start a command buffer and a render pass and set our vertex and index buffers.
(`frame.view` is the current canvas texture's view; a pass in Rust ends when it
goes out of scope so the whole pass lives in a `{ }` block.)

```rust
    let mut encoder = frame.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.3,
              g: 0.3,
              b: 0.3,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
          view: &depth_view,
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        ..Default::default()
      });
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, position_buffer.slice(..));
      pass.set_vertex_buffer(1, normal_buffer.slice(..));
      pass.set_vertex_buffer(2, texcoord_buffer.slice(..));
      pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint16);
```

Then we'll compute a viewProjection matrix like we covered in
[the article on perspective projection](webgpu-perspective-projection.html),
using [glam](https://docs.rs/glam) for the matrix math.

```rust
+      let aspect = frame.width as f32 / frame.height as f32;
+      let projection = Mat4::perspective_rh(
+        60.0f32.to_radians(),
+        aspect,
+        1.0,    // zNear
+        2000.0, // zFar
+      );
+
+      let eye = Vec3::new(100.0, 150.0, 200.0);
+      let target = Vec3::new(0.0, 0.0, 0.0);
+      let up = Vec3::new(0.0, 1.0, 0.0);
+
+      // Compute a view matrix
+      let view_matrix = Mat4::look_at_rh(eye, target, up);
+
+      // Combine the view and projection matrixes
+      let view_projection_matrix = projection * view_matrix;
```

Now we can loop over all the objects and draw them, for each one we need
to update all of its uniform values, copy the uniform values to its uniform buffer,
bind the bind group for this object, and draw.

```rust
      for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
        let uniform_values = &mut object.uniform_values;

        // Copy the viewProjectionMatrix into the uniform values for this object
        uniform_values[K_VIEW_PROJECTION_OFFSET..][..16]
          .copy_from_slice(&view_projection_matrix.to_cols_array());

        // Compute a world matrix
        let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 3.721 + time * object.speed).sin() * object.radius))
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 9.721 + time * 0.1).sin() * object.radius))
          * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
          * Mat4::from_scale(Vec3::splat(object.scale));
        uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

        // Inverse and transpose it into the normalMatrix value
        // (a WGSL mat3x3f is 3 columns of 3 floats + 1 float of padding each)
        let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
        uniform_values[K_NORMAL_MATRIX_OFFSET..][..3].copy_from_slice(&normal_matrix.x_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3].copy_from_slice(&normal_matrix.y_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3].copy_from_slice(&normal_matrix.z_axis.to_array());

        let Material { color, shininess, .. } = object.material;

        // copy the materials values.
        uniform_values[K_COLOR_OFFSET..][..4].copy_from_slice(&color);
        uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&[-10.0, 30.0, 300.0]);
        uniform_values[K_VIEW_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&eye.to_array());
        uniform_values[K_SHININESS_OFFSET] = shininess;

        // upload the uniform values to the uniform buffer
        frame.queue.write_buffer(&object.uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

        pass.set_bind_group(0, &object.bind_group, &[]);
        pass.draw_indexed(0..num_vertices, 0, 0..1);
      }
```

> Note that the portion of the code labeled "Compute a world matrix" is not so common. It would
be more common to have a [scene graph](webgpu-scene-graphs.html) but that would have cluttered
the example even more. We needed something showing animation I threw something together.

Then we can end the pass, finish the command buffer, and submit it.

```rust
+    } // the pass "ends" here, when it goes out of scope
+
+    let command_buffer = encoder.finish();
+    frame.queue.submit([command_buffer]);
```

A few more things left to do. The original JavaScript adds a `ResizeObserver`
so the canvas resolution follows its displayed size. `wgpu_fun` implements that
same behavior when we set:

```rust
  let mut app = App::new("WebGPU Optimization - None").await;
+  app.auto_resize = true;
```

Let's also add in some timing. We'll use the `NonNegativeRollingAverage` and
`TimingHelper` structs we made in [the article on timing](webgpu-timing.html).

```rust
// see https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html
struct TimingHelper { ... }

// see https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html
struct NonNegativeRollingAverage { ... }

  ...

  let mut fps_average = NonNegativeRollingAverage::new();
  let mut js_average = NonNegativeRollingAverage::new();
  let gpu_average = Arc::new(Mutex::new(NonNegativeRollingAverage::new()));
  let mut math_average = NonNegativeRollingAverage::new();
```

Then we'll time our code from the beginning to the end of our rendering code.
The original examples call this "js" time; we'll keep the label so the numbers
are easy to compare, even though for us it's Rust (compiled to wasm in the
browser).

```rust
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    ...

+    let start_time_ms = wgpu_fun::now_ms();

    ...

+    let elapsed_time_ms = wgpu_fun::now_ms() - start_time_ms;
+    js_average.add_sample(elapsed_time_ms);
  });
```

We'll time the part of the code that does the 3D math

```rust
+    let mut math_elapsed_time_ms = 0.0;

    for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
+      let math_time_start_ms = wgpu_fun::now_ms();

      let uniform_values = &mut object.uniform_values;

      // Copy the viewProjectionMatrix into the uniform values for this object
      uniform_values[K_VIEW_PROJECTION_OFFSET..][..16]
        .copy_from_slice(&view_projection_matrix.to_cols_array());

      // Compute a world matrix
      ...

      // Inverse and transpose it into the normalMatrix value
      ...

      let Material { color, shininess, .. } = object.material;

      uniform_values[K_COLOR_OFFSET..][..4].copy_from_slice(&color);
      uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&[-10.0, 30.0, 300.0]);
      uniform_values[K_VIEW_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&eye.to_array());
      uniform_values[K_SHININESS_OFFSET] = shininess;

+      math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

      // upload the uniform values to the uniform buffer
      frame.queue.write_buffer(&object.uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

      pass.set_bind_group(0, &object.bind_group, &[]);
      pass.draw_indexed(0..num_vertices, 0, 0..1);
    }

    ...

    let elapsed_time_ms = wgpu_fun::now_ms() - start_time_ms;
    js_average.add_sample(elapsed_time_ms);
+    math_average.add_sample(math_elapsed_time_ms);
```

We'll time the time between frames.

```rust
  let mut depth_texture: Option<wgpu::Texture> = None;
  let mut then = 0.0;

  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let time = frame.time; // seconds
    let delta_time = time - then;
    then = time;

    ...

    let elapsed_time_ms = wgpu_fun::now_ms() - start_time_ms;
+    fps_average.add_sample(1.0 / delta_time);
    js_average.add_sample(elapsed_time_ms);
    math_average.add_sample(math_elapsed_time_ms);
  });
```

And we'll time our render pass

```rust
async fn run() {
-  let mut app = App::new("WebGPU Optimization - None").await;
+  // ask for the timestamp-query feature if the adapter supports it
+  let mut app = App::new_with_features(
+    "WebGPU Optimization - None",
+    wgpu::Features::TIMESTAMP_QUERY,
+  )
+  .await;
  app.auto_resize = true;
  app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;
+  let can_timestamp = app
+    .device
+    .features()
+    .contains(wgpu::Features::TIMESTAMP_QUERY);

+  let mut timing_helper = TimingHelper::new(&app.device, &app.queue);

  ...

  app.run(RenderMode::Continuous, move |frame: &Frame| {
    ...

-      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
+      let mut pass = timing_helper.begin_render_pass(&mut encoder, &wgpu::RenderPassDescriptor {

    ...

    } // the pass "ends" here, when it goes out of scope

+    timing_helper.resolve_timing(&mut encoder);

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);

+    {
+      let gpu_average = gpu_average.clone();
+      timing_helper.get_result(move |gpu_time| {
+        gpu_average.lock().unwrap().add_sample(gpu_time / 1000.0);
+      });
+    }
+    // mapAsync results are delivered when the device is polled; the
+    // browser does that for us, natively we poll once per frame.
+    let _ = frame.device.poll(wgpu::PollType::Poll);

    ...
  });
```

And we need to show the timing, which `wgpu_fun::set_info_text` puts in the
`#info` element on top of the canvas (or prints to stdout natively).

```rust
    let elapsed_time_ms = wgpu_fun::now_ms() - start_time_ms;
    fps_average.add_sample(1.0 / delta_time);
    js_average.add_sample(elapsed_time_ms);
    math_average.add_sample(math_elapsed_time_ms);

+    wgpu_fun::set_info_text(&format!(
+      "\
+js  : {:.1}ms
+math: {:.1}ms
+fps : {:.0}
+gpu : {}
+",
+      js_average.get(),
+      math_average.get(),
+      fps_average.get(),
+      if can_timestamp {
+        format!("{:.1}ms", gpu_average.lock().unwrap().get() / 1000.0)
+      } else {
+        "N/A".to_string()
+      },
+    ));
```

One more thing, just to help with better comparisons. An issue we have now is,
every visible cube has every pixel rendered or at least checked if it needs to
be rendered. Since we're not optimizing the rendering of pixels but rather
optimizing the usage of WebGPU itself, it can be useful to be able to draw to a
1x1 pixel canvas. This effectively removes nearly all of the time spent
rasterizing triangles and instead leaves only the part of our code that is doing
math and communicating with WebGPU.

So let's add an option to do that. The JavaScript version resizes the canvas
itself to 1x1; we can't resize the canvas from inside the frame callback, so
instead, when 'render' is unchecked, we render to a cached 1x1 offscreen
texture — which removes the same rasterization work.

```js
const settings = {
  numObjects: 1000,
+  render: true,
};

const gui = new GUI();
gui.add(settings, 'numObjects', { min: 0, max: maxObjects, step: 1})
  .onChange(v => wasm.set_setting_num('numObjects', v));
+gui.add(settings, 'render')
+  .onChange(v => wasm.set_setting_bool('render', v));
```

```rust
  let mut depth_texture: Option<wgpu::Texture> = None;
+  // a 1x1 texture to render to when 'render' is unchecked
+  let mut small_target: Option<wgpu::Texture> = None;
  let mut then = 0.0;

  app.run(RenderMode::Continuous, move |frame: &Frame| {
    ...

    // read the settings the GUI on the page sets
    let num_objects = (wgpu_fun::setting_f64("numObjects", 1000.0) as usize).min(max_objects);
+    let render = wgpu_fun::setting_bool("render", true);

+    let (target_width, target_height) = if render { (frame.width, frame.height) } else { (1, 1) };
+    if !render && small_target.is_none() {
+      small_target = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
+        label: None,
+        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
+        format: frame.format,
+        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
+        mip_level_count: 1,
+        sample_count: 1,
+        dimension: wgpu::TextureDimension::D2,
+        view_formats: &[],
+      }));
+    }
+    let small_target_view = small_target.as_ref().map(|t| t.create_view(&Default::default()));
+    let color_view = if render {
+      frame.view
+    } else {
+      small_target_view.as_ref().unwrap()
+    };
```

and we size the depth texture to match the render target and use `color_view`
as the render pass's color attachment.

Now, if we uncheck 'render', we'll remove almost all of the um, ahh ..., rendering.

And with that, we have our first "un-optimized" example. It's following the
steps listed near the top of the article, and it works.

{{{example url="../webgpu-optimization-none.html"}}}

Increase the number of objects and see when the framerate drops for you. The
author of the original JavaScript article, on a 75hz monitor on an M1 Mac, got
~8000 cubes before the framerate dropped.

# <a id="a-mapped-on-creation"></a> Optimization: Mapped On Creation

In the example above, and in most of the examples on this site, we've used
`write_buffer` to copy data into a vertex or index buffer. As a very minor
optimization, for this particular case, when you create a buffer you can pass in
`mapped_at_creation: true`. This has 2 benefits.

1. It's slightly faster to put the data into the new buffer

2. You don't have to add `wgpu::BufferUsages::COPY_DST` to the buffer's usage.

   This assumes you're not going to change the data later via `write_buffer` nor
   one of the copy to buffer functions.

```rust
  fn create_buffer_with_data(
    device: &wgpu::Device,
-    queue: &wgpu::Queue,
    data: &[u8],
    usage: wgpu::BufferUsages,
  ) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: data.len() as u64,
-      usage: usage | wgpu::BufferUsages::COPY_DST,
-      mapped_at_creation: false,
+      usage,
+      mapped_at_creation: true,
    });
-    queue.write_buffer(&buffer, 0, data);
+    buffer.slice(..).get_mapped_range_mut().unwrap().copy_from_slice(data);
+    buffer.unmap();
    buffer
  }
```

(`get_mapped_range_mut` is the wgpu version of JavaScript's
`buffer.getMappedRange()` — a view of the mapped memory we can copy into.)

Note that this optimization only helps at creation time so it will not affect
our performance at render time.

# <a id="a-pack-verts"></a> Optimization: Pack and interleave your vertices

In the example above we have 3 attributes, one for position, one for normals,
and one for texture coordinates. It's common to have 4 to 6 attributes where
we'd have [tangents for normal mapping](webgpu-normal-mapping.html) and, if
we had [a skinned model](webgpu-skinning.html), we'd add in weights and joints.

In the example above, each attribute is using its own buffer. This is slower both
on the CPU and GPU. It's slower on the CPU because we need to call
`set_vertex_buffer` once for each buffer for each model we want to draw.

Imagine instead of just a cube we had 100s of models. Each time we switched
which model to draw we'd have to call `set_vertex_buffer` up to 6 times. 100 * 6
calls per model = 600 calls. 

Following the rule "less work = go faster", if we merged the data for the
attributes into a single buffer then we'd only need one call to
`set_vertex_buffer` once per model. 100 calls. That's like 600% faster!

On the GPU, loading things that are together in memory is usually faster than
loading from different places in memory so on top of just putting the vertex
data for a single model into a single buffer, it's better to interleave the
data.

Let's make that change.

```rust
-  let positions: Vec<f32> = vec![1.0, 1.0, -1.0, /* ... */ -1.0, -1.0, -1.0];
-  let normals: Vec<f32> = vec![1.0, 0.0, 0.0, /* ... */ 0.0, 0.0, -1.0];
-  let texcoords: Vec<f32> = vec![1.0, 0.0, 0.0, /* ... */ 1.0, 1.0];
+  let vertex_data: Vec<f32> = vec![
+  // position           normal              texcoord
+     1.0,  1.0, -1.0,   1.0,  0.0,  0.0,    1.0, 0.0,
+     1.0,  1.0,  1.0,   1.0,  0.0,  0.0,    0.0, 0.0,
+     1.0, -1.0,  1.0,   1.0,  0.0,  0.0,    0.0, 1.0,
+     1.0, -1.0, -1.0,   1.0,  0.0,  0.0,    1.0, 1.0,
+    -1.0,  1.0,  1.0,  -1.0,  0.0,  0.0,    1.0, 0.0,
+    -1.0,  1.0, -1.0,  -1.0,  0.0,  0.0,    0.0, 0.0,
+    -1.0, -1.0, -1.0,  -1.0,  0.0,  0.0,    0.0, 1.0,
+    -1.0, -1.0,  1.0,  -1.0,  0.0,  0.0,    1.0, 1.0,
+    -1.0,  1.0,  1.0,   0.0,  1.0,  0.0,    1.0, 0.0,
+     1.0,  1.0,  1.0,   0.0,  1.0,  0.0,    0.0, 0.0,
+     1.0,  1.0, -1.0,   0.0,  1.0,  0.0,    0.0, 1.0,
+    -1.0,  1.0, -1.0,   0.0,  1.0,  0.0,    1.0, 1.0,
+    -1.0, -1.0, -1.0,   0.0, -1.0,  0.0,    1.0, 0.0,
+     1.0, -1.0, -1.0,   0.0, -1.0,  0.0,    0.0, 0.0,
+     1.0, -1.0,  1.0,   0.0, -1.0,  0.0,    0.0, 1.0,
+    -1.0, -1.0,  1.0,   0.0, -1.0,  0.0,    1.0, 1.0,
+     1.0,  1.0,  1.0,   0.0,  0.0,  1.0,    1.0, 0.0,
+    -1.0,  1.0,  1.0,   0.0,  0.0,  1.0,    0.0, 0.0,
+    -1.0, -1.0,  1.0,   0.0,  0.0,  1.0,    0.0, 1.0,
+     1.0, -1.0,  1.0,   0.0,  0.0,  1.0,    1.0, 1.0,
+    -1.0,  1.0, -1.0,   0.0,  0.0, -1.0,    1.0, 0.0,
+     1.0,  1.0, -1.0,   0.0,  0.0, -1.0,    0.0, 0.0,
+     1.0, -1.0, -1.0,   0.0,  0.0, -1.0,    0.0, 1.0,
+    -1.0, -1.0, -1.0,   0.0,  0.0, -1.0,    1.0, 1.0,
+  ];
  let indices: Vec<u16> = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 8, 9, 10, 8, 10, 11, 12, 13, 14, 12, 14, 15, 16, 17, 18, 16, 18, 19, 20, 21, 22, 20, 22, 23];

-  let position_buffer = create_buffer_with_data(&app.device, bytemuck::cast_slice(&positions), wgpu::BufferUsages::VERTEX);
-  let normal_buffer = create_buffer_with_data(&app.device, bytemuck::cast_slice(&normals), wgpu::BufferUsages::VERTEX);
-  let texcoord_buffer = create_buffer_with_data(&app.device, bytemuck::cast_slice(&texcoords), wgpu::BufferUsages::VERTEX);
+  let vertex_buffer = create_buffer_with_data(&app.device, bytemuck::cast_slice(&vertex_data), wgpu::BufferUsages::VERTEX);
  let indices_buffer = create_buffer_with_data(&app.device, bytemuck::cast_slice(&indices), wgpu::BufferUsages::INDEX);
  let num_vertices = indices.len() as u32;

  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("textured model with point light w/specular highlight"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[
-        // position
-        Some(wgpu::VertexBufferLayout {
-          array_stride: 3 * 4, // 3 floats
-          step_mode: wgpu::VertexStepMode::Vertex,
-          attributes: &[wgpu::VertexAttribute {
-            shader_location: 0,
-            offset: 0,
-            format: wgpu::VertexFormat::Float32x3,
-          }],
-        }),
-        // normal
-        Some(wgpu::VertexBufferLayout {
-          array_stride: 3 * 4, // 3 floats
-          step_mode: wgpu::VertexStepMode::Vertex,
-          attributes: &[wgpu::VertexAttribute {
-            shader_location: 1,
-            offset: 0,
-            format: wgpu::VertexFormat::Float32x3,
-          }],
-        }),
-        // uvs
-        Some(wgpu::VertexBufferLayout {
-          array_stride: 2 * 4, // 2 floats
-          step_mode: wgpu::VertexStepMode::Vertex,
-          attributes: &[wgpu::VertexAttribute {
-            shader_location: 2,
-            offset: 0,
-            format: wgpu::VertexFormat::Float32x2,
-          }],
-        }),
+        Some(wgpu::VertexBufferLayout {
+          array_stride: (3 + 3 + 2) * 4, // 8 floats
+          step_mode: wgpu::VertexStepMode::Vertex,
+          attributes: &[
+            // position
+            wgpu::VertexAttribute {
+              shader_location: 0,
+              offset: 0,
+              format: wgpu::VertexFormat::Float32x3,
+            },
+            // normal
+            wgpu::VertexAttribute {
+              shader_location: 1,
+              offset: 3 * 4,
+              format: wgpu::VertexFormat::Float32x3,
+            },
+            // texcoord
+            wgpu::VertexAttribute {
+              shader_location: 2,
+              offset: 6 * 4,
+              format: wgpu::VertexFormat::Float32x2,
+            },
+          ],
+        }),
      ],
    },
    ...
  });

  ...
-      pass.set_vertex_buffer(0, position_buffer.slice(..));
-      pass.set_vertex_buffer(1, normal_buffer.slice(..));
-      pass.set_vertex_buffer(2, texcoord_buffer.slice(..));
+      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
```

Above we put the data for all 3 attributes into a single buffer and then changed
our render pass so it expects the data interleaved into a single buffer.

Note: if you're loading gLTF files, it's arguably good to either pre-process
them so their vertex data is interleaved into a single buffer (best) or else
interleave the data at load time.

# Optimization: Split uniform buffers (shared, material, per model)

Our example right now has one uniform buffer per object.

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  viewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
};
```

Some of those uniform values like `viewProjection`, `lightWorldPosition`
and `viewWorldPosition` can be shared.

We can split these in the shader to use 2 uniform buffers. One for the shared
values and one for *per object values*.

```wgsl
struct GlobalUniforms {
  viewProjection: mat4x4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
};
struct PerObjectUniforms {
  normalMatrix: mat3x3f,
  world: mat4x4f,
  color: vec4f,
  shininess: f32,
};
```

With this change, we'll save having to copy the 
`viewProjection`, `lightWorldPosition` and `viewWorldPosition`
to every uniform buffer. We'll also copy less data per object
with `queue.write_buffer`

Here's the new shader

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
-      struct Uniforms {
-        normalMatrix: mat3x3f,
-        viewProjection: mat4x4f,
-        world: mat4x4f,
-        color: vec4f,
-        lightWorldPosition: vec3f,
-        viewWorldPosition: vec3f,
-        shininess: f32,
-      };

+      struct GlobalUniforms {
+        viewProjection: mat4x4f,
+        lightWorldPosition: vec3f,
+        viewWorldPosition: vec3f,
+      };
+      struct PerObjectUniforms {
+        normalMatrix: mat3x3f,
+        world: mat4x4f,
+        color: vec4f,
+        shininess: f32,
+      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) normal: vec3f,
        @location(2) texcoord: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) normal: vec3f,
        @location(1) surfaceToLight: vec3f,
        @location(2) surfaceToView: vec3f,
        @location(3) texcoord: vec2f,
      };

      @group(0) @binding(0) var diffuseTexture: texture_2d<f32>;
      @group(0) @binding(1) var diffuseSampler: sampler;
-      @group(0) @binding(2) var<uniform> uni: Uniforms;
+      @group(0) @binding(2) var<uniform> obj: PerObjectUniforms;
+      @group(0) @binding(3) var<uniform> glb: GlobalUniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
-        vsOut.position = uni.viewProjection * uni.world * vert.position;
+        vsOut.position = glb.viewProjection * obj.world * vert.position;

        // Orient the normals and pass to the fragment shader
-        vsOut.normal = uni.normalMatrix * vert.normal;
+        vsOut.normal = obj.normalMatrix * vert.normal;

        // Compute the world position of the surface
-        let surfaceWorldPosition = (uni.world * vert.position).xyz;
+        let surfaceWorldPosition = (obj.world * vert.position).xyz;

        // Compute the vector of the surface to the light
        // and pass it to the fragment shader
-        vsOut.surfaceToLight = uni.lightWorldPosition - surfaceWorldPosition;
+        vsOut.surfaceToLight = glb.lightWorldPosition - surfaceWorldPosition;

        // Compute the vector of the surface to the view
        // and pass it to the fragment shader
-        vsOut.surfaceToView = uni.viewWorldPosition - surfaceWorldPosition;
+        vsOut.surfaceToView = glb.viewWorldPosition - surfaceWorldPosition;

        // Pass the texture coord on to the fragment shader
        vsOut.texcoord = vert.texcoord;

        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        // Because vsOut.normal is an inter-stage variable 
        // it's interpolated so it will not be a unit vector.
        // Normalizing it will make it a unit vector again
        let normal = normalize(vsOut.normal);

        let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
        let surfaceToViewDirection = normalize(vsOut.surfaceToView);
        let halfVector = normalize(
          surfaceToLightDirection + surfaceToViewDirection);

        // Compute the light by taking the dot product
        // of the normal with the direction to the light
        let light = dot(normal, surfaceToLightDirection);

        var specular = dot(normal, halfVector);
        specular = select(
            0.0,                           // value if condition is false
-            pow(specular, uni.shininess),  // value if condition is true
+            pow(specular, obj.shininess),  // value if condition is true
            specular > 0.0);               // condition

-        let diffuse = uni.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
+        let diffuse = obj.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
        // Lets multiply just the color portion (not the alpha)
        // by the light
        let color = diffuse.rgb * light + specular;
        return vec4f(color, diffuse.a);
      }
    "#
      .into(),
    ),
  });
```

We need to create one global uniform buffer for the global uniforms.

```rust
  let global_uniform_buffer_size = (16 + 4 + 4) * 4;
  let global_uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("global uniforms"),
    size: global_uniform_buffer_size,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut global_uniform_values = vec![0.0f32; global_uniform_buffer_size as usize / 4];

  // offsets to the various global uniform values in float32 indices
  const K_VIEW_PROJECTION_OFFSET: usize = 0;
  const K_LIGHT_WORLD_POSITION_OFFSET: usize = 16;
  const K_VIEW_WORLD_POSITION_OFFSET: usize = 20;
```

Then we can remove these uniforms from our perObject uniform buffer and add the
global uniform buffer to each object's bind group.

```rust
  let max_objects = 30000;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  // offsets to the various uniform values in float32 indices
  const K_NORMAL_MATRIX_OFFSET: usize = 0;
-  const K_VIEW_PROJECTION_OFFSET: usize = 12;
-  const K_WORLD_OFFSET: usize = 28;
-  const K_COLOR_OFFSET: usize = 44;
-  const K_LIGHT_WORLD_POSITION_OFFSET: usize = 48;
-  const K_VIEW_WORLD_POSITION_OFFSET: usize = 52;
-  const K_SHININESS_OFFSET: usize = 55;
+  const K_WORLD_OFFSET: usize = 12;
+  const K_COLOR_OFFSET: usize = 28;
+  const K_SHININESS_OFFSET: usize = 32;

  for _ in 0..max_objects {
-    let uniform_buffer_size = (12 + 16 + 16 + 4 + 4 + 4) * 4;
+    let uniform_buffer_size = (12 + 16 + 4 + 4) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms"),
      size: uniform_buffer_size,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let uniform_values = vec![0.0f32; uniform_buffer_size as usize / 4];

    let material = random_array_element(&materials).clone();

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("bind group for object"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&material.texture.create_view(&Default::default())),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&material.sampler),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: uniform_buffer.as_entire_binding(),
        },
+        wgpu::BindGroupEntry {
+          binding: 3,
+          resource: global_uniform_buffer.as_entire_binding(),
+        },
      ],
    });

    let axis = Vec3::new(rand(-1.0, 1.0), rand(-1.0, 1.0), rand(-1.0, 1.0)).normalize();
    let radius = rand(10.0, 100.0);
    let speed = rand(0.1, 0.4);
    let rotation_speed = rand(-1.0, 1.0);
    let scale = rand(2.0, 10.0);

    object_infos.push(ObjectInfo {
      bind_group,

      uniform_buffer,
      uniform_values,

      axis,
      material,
      radius,
      speed,
      rotation_speed,
      scale,
    });
  }
```

Then, at render time, we update the global uniform buffer just once, outside the
loop of rendering our objects.

```rust
      let aspect = frame.width as f32 / frame.height as f32;
      let projection = Mat4::perspective_rh(
        60.0f32.to_radians(),
        aspect,
        1.0,    // zNear
        2000.0, // zFar
      );

      let eye = Vec3::new(100.0, 150.0, 200.0);
      let target = Vec3::new(0.0, 0.0, 0.0);
      let up = Vec3::new(0.0, 1.0, 0.0);

      // Compute a view matrix
      let view_matrix = Mat4::look_at_rh(eye, target, up);

      // Combine the view and projection matrixes
-      let view_projection_matrix = projection * view_matrix;
+      global_uniform_values[K_VIEW_PROJECTION_OFFSET..][..16]
+        .copy_from_slice(&(projection * view_matrix).to_cols_array());
+
+      global_uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&[-10.0, 30.0, 300.0]);
+      global_uniform_values[K_VIEW_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&eye.to_array());
+
+      frame.queue.write_buffer(&global_uniform_buffer, 0, bytemuck::cast_slice(&global_uniform_values));

      for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
        let math_time_start_ms = wgpu_fun::now_ms();

        let uniform_values = &mut object.uniform_values;

-        // Copy the viewProjectionMatrix into the uniform values for this object
-        uniform_values[K_VIEW_PROJECTION_OFFSET..][..16]
-          .copy_from_slice(&view_projection_matrix.to_cols_array());

        // Compute a world matrix
        let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 3.721 + time * object.speed).sin() * object.radius))
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 9.721 + time * 0.1).sin() * object.radius))
          * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
          * Mat4::from_scale(Vec3::splat(object.scale));
        uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

        // Inverse and transpose it into the normalMatrix value
        let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
        uniform_values[K_NORMAL_MATRIX_OFFSET..][..3].copy_from_slice(&normal_matrix.x_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3].copy_from_slice(&normal_matrix.y_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3].copy_from_slice(&normal_matrix.z_axis.to_array());

        let Material { color, shininess, .. } = object.material;
        uniform_values[K_COLOR_OFFSET..][..4].copy_from_slice(&color);
-        uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&[-10.0, 30.0, 300.0]);
-        uniform_values[K_VIEW_WORLD_POSITION_OFFSET..][..3].copy_from_slice(&eye.to_array());
        uniform_values[K_SHININESS_OFFSET] = shininess;

        math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

        // upload the uniform values to the uniform buffer
        frame.queue.write_buffer(&object.uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

        pass.set_bind_group(0, &object.bind_group, &[]);
        pass.draw_indexed(0..num_vertices, 0, 0..1);
      }
```

That didn't change the number of calls into WebGPU, in fact it added 1. But, it
reduced a bunch of the work we were doing per model.

{{{example url="../webgpu-optimization-step3-global-vs-per-object-uniforms.html"}}}

On the original author's machine, with that change, the math portion dropped
~16%.

# Optimization: Separate more uniforms

A common organization in a 3D library is to have "models" (the vertex data),
"materials" (the colors, shininess, and textures), "lights" (which lights to
use), "viewInfo" (the view and projection matrix). In particular, in our
example, `color` and `shininess` never change so it's a waste to keep copying
them to the uniform buffer every frame.

Let's make a uniform buffer per material. We'll copy the material settings into
them at init time and then just add them to our bind group.

First let's change the shaders to use another uniform buffer.

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
      struct GlobalUniforms {
        viewProjection: mat4x4f,
        lightWorldPosition: vec3f,
        viewWorldPosition: vec3f,
      };

+      struct MaterialUniforms {
+        color: vec4f,
+        shininess: f32,
+      };

      struct PerObjectUniforms {
        normalMatrix: mat3x3f,
        world: mat4x4f,
-        color: vec4f,
-        shininess: f32,
      };

      struct Vertex {
        @location(0) position: vec4f,
        @location(1) normal: vec3f,
        @location(2) texcoord: vec2f,
      };

      struct VSOutput {
        @builtin(position) position: vec4f,
        @location(0) normal: vec3f,
        @location(1) surfaceToLight: vec3f,
        @location(2) surfaceToView: vec3f,
        @location(3) texcoord: vec2f,
      };

      @group(0) @binding(0) var diffuseTexture: texture_2d<f32>;
      @group(0) @binding(1) var diffuseSampler: sampler;
      @group(0) @binding(2) var<uniform> obj: PerObjectUniforms;
      @group(0) @binding(3) var<uniform> glb: GlobalUniforms;
+      @group(0) @binding(4) var<uniform> material: MaterialUniforms;

      @vertex fn vs(vert: Vertex) -> VSOutput {
        var vsOut: VSOutput;
        vsOut.position = glb.viewProjection * obj.world * vert.position;

        // Orient the normals and pass to the fragment shader
        vsOut.normal = obj.normalMatrix * vert.normal;

        // Compute the world position of the surface
        let surfaceWorldPosition = (obj.world * vert.position).xyz;

        // Compute the vector of the surface to the light
        // and pass it to the fragment shader
        vsOut.surfaceToLight = glb.lightWorldPosition - surfaceWorldPosition;

        // Compute the vector of the surface to the view
        // and pass it to the fragment shader
        vsOut.surfaceToView = glb.viewWorldPosition - surfaceWorldPosition;

        // Pass the texture coord on to the fragment shader
        vsOut.texcoord = vert.texcoord;

        return vsOut;
      }

      @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
        // Because vsOut.normal is an inter-stage variable 
        // it's interpolated so it will not be a unit vector.
        // Normalizing it will make it a unit vector again
        let normal = normalize(vsOut.normal);

        let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
        let surfaceToViewDirection = normalize(vsOut.surfaceToView);
        let halfVector = normalize(
          surfaceToLightDirection + surfaceToViewDirection);

        // Compute the light by taking the dot product
        // of the normal with the direction to the light
        let light = dot(normal, surfaceToLightDirection);

        var specular = dot(normal, halfVector);
        specular = select(
            0.0,                           // value if condition is false
-            pow(specular, obj.shininess),  // value if condition is true
+            pow(specular, material.shininess),  // value if condition is true
            specular > 0.0);               // condition

-        let diffuse = obj.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
+        let diffuse = material.color * textureSample(diffuseTexture, diffuseSampler, vsOut.texcoord);
        // Lets multiply just the color portion (not the alpha)
        // by the light
        let color = diffuse.rgb * light + specular;
        return vec4f(color, diffuse.a);
      }
    "#
      .into(),
    ),
  });
```

Then we'll make a uniform buffer for each material.

```rust
#[derive(Clone)]
struct Material {
-  color: [f32; 4],
-  shininess: f32,
+  material_uniform_buffer: wgpu::Buffer,
  texture: wgpu::Texture,
  sampler: wgpu::Sampler,
}

  ...

  let num_materials = 20;
  let mut materials: Vec<Material> = Vec::new();
  for _ in 0..num_materials {
    let color = hsl_to_rgba(rand(0.0, 1.0), rand(0.5, 0.8), rand(0.5, 0.7));
    let shininess = rand(10.0, 120.0);

+    let mut material_values = [0.0f32; 8];
+    material_values[..4].copy_from_slice(&color);
+    material_values[4] = shininess;
+    // material_values[5..8] is padding
+    let material_uniform_buffer = create_buffer_with_data(
+      &app.device,
+      bytemuck::cast_slice(&material_values),
+      wgpu::BufferUsages::UNIFORM,
+    );

    materials.push(Material {
-      color,
-      shininess,
+      material_uniform_buffer,
      texture: random_array_element(&textures).clone(),
      sampler: sampler.clone(),
    });
  }
```

When we setup the per object info we no longer need to pass on the material
settings. Instead we just need to add the material's uniform buffer to the
object's bind group.

```rust
  let max_objects = 30000;
  let mut object_infos: Vec<ObjectInfo> = Vec::new();

  // offsets to the various uniform values in float32 indices
  const K_NORMAL_MATRIX_OFFSET: usize = 0;
  const K_WORLD_OFFSET: usize = 12;
-  const K_COLOR_OFFSET: usize = 28;
-  const K_SHININESS_OFFSET: usize = 32;

  for _ in 0..max_objects {
-    let uniform_buffer_size = (12 + 16 + 4 + 4) * 4;
+    let uniform_buffer_size = (12 + 16) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms"),
      size: uniform_buffer_size,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let uniform_values = vec![0.0f32; uniform_buffer_size as usize / 4];

    let material = random_array_element(&materials).clone();

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("bind group for object"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&material.texture.create_view(&Default::default())),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&material.sampler),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: uniform_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 3,
          resource: global_uniform_buffer.as_entire_binding(),
        },
+        wgpu::BindGroupEntry {
+          binding: 4,
+          resource: material.material_uniform_buffer.as_entire_binding(),
+        },
      ],
    });

    let axis = Vec3::new(rand(-1.0, 1.0), rand(-1.0, 1.0), rand(-1.0, 1.0)).normalize();
    let radius = rand(10.0, 100.0);
    let speed = rand(0.1, 0.4);
    let rotation_speed = rand(-1.0, 1.0);
    let scale = rand(2.0, 10.0);

    object_infos.push(ObjectInfo {
      bind_group,

      uniform_buffer,
      uniform_values,

      axis,
-      material,
      radius,
      speed,
      rotation_speed,
      scale,
    });
  }
```

We also no longer need to deal with this stuff at render time.

```rust
      for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
        let math_time_start_ms = wgpu_fun::now_ms();

        let uniform_values = &mut object.uniform_values;

        // Compute a world matrix
        let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 3.721 + time * object.speed).sin() * object.radius))
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 9.721 + time * 0.1).sin() * object.radius))
          * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
          * Mat4::from_scale(Vec3::splat(object.scale));
        uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

        // Inverse and transpose it into the normalMatrix value
        let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
        uniform_values[K_NORMAL_MATRIX_OFFSET..][..3].copy_from_slice(&normal_matrix.x_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3].copy_from_slice(&normal_matrix.y_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3].copy_from_slice(&normal_matrix.z_axis.to_array());

-        let Material { color, shininess, .. } = object.material;
-        uniform_values[K_COLOR_OFFSET..][..4].copy_from_slice(&color);
-        uniform_values[K_SHININESS_OFFSET] = shininess;

        math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

        // upload the uniform values to the uniform buffer
        frame.queue.write_buffer(&object.uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

        pass.set_bind_group(0, &object.bind_group, &[]);
        pass.draw_indexed(0..num_vertices, 0, 0..1);
      }
```

{{{example url="../webgpu-optimization-step4-material-uniforms.html"}}}

# Optimization: Use One large Uniform Buffer with buffer offsets

Right now, each object has it's own uniform buffer. At render time, for each
object, we update an array with the uniform values for that object and then
call `queue.write_buffer` to update that single uniform buffer's values.
If we're rendering 8000 objects that's 8000 calls to `queue.write_buffer`.

Instead, we could make one larger uniform buffer. We can then setup the bind
group for each object to use it's own portion of the larger buffer. At render
time, we can update all the values for all of the objects in one large `Vec`
and make just one call to `queue.write_buffer` which should be
faster.

First let's allocate a large uniform buffer and large `Vec`. Uniform
buffer offsets have a minimum alignment which defaults to 256 bytes so we'll
round up the size we need per object to 256 bytes.

```rust
+/// Rounds up v to a multiple of alignment
+fn round_up(v: u64, alignment: u64) -> u64 {
+  v.div_ceil(alignment) * alignment
+}

  ...

+  let uniform_buffer_size = (12 + 16) * 4;
+  let uniform_buffer_space = round_up(uniform_buffer_size, app.device.limits().min_uniform_buffer_offset_alignment as u64);
+  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+    label: Some("uniforms"),
+    size: uniform_buffer_space * max_objects as u64,
+    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
+    mapped_at_creation: false,
+  });
+  // one large array of uniform values for all the objects
+  let mut uniform_values = vec![0.0f32; uniform_buffer.size() as usize / 4];
```

Now we can set the bind group of each object to use the correct portion of the
large uniform buffer. In JavaScript, per-object `subarray` views into the one
big `Float32Array` are also made here; in Rust we'll index into the one big
`Vec` with each object's offset at render time instead.

```rust
-  for _ in 0..max_objects {
-    let uniform_buffer_size = (12 + 16) * 4;
-    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-      label: Some("uniforms"),
-      size: uniform_buffer_size,
-      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
-      mapped_at_creation: false,
-    });
-
-    let uniform_values = vec![0.0f32; uniform_buffer_size as usize / 4];
+  for i in 0..max_objects {
+    let uniform_buffer_offset = i as u64 * uniform_buffer_space;

    let material = random_array_element(&materials).clone();

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("bind group for object"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&material.texture.create_view(&Default::default())),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&material.sampler),
        },
-        wgpu::BindGroupEntry {
-          binding: 2,
-          resource: uniform_buffer.as_entire_binding(),
-        },
+        wgpu::BindGroupEntry {
+          binding: 2,
+          resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
+            buffer: &uniform_buffer,
+            offset: uniform_buffer_offset,
+            size: Some(wgpu::BufferSize::new(uniform_buffer_size).unwrap()),
+          }),
+        },
        wgpu::BindGroupEntry {
          binding: 3,
          resource: global_uniform_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 4,
          resource: material.material_uniform_buffer.as_entire_binding(),
        },
      ],
    });

    let axis = Vec3::new(rand(-1.0, 1.0), rand(-1.0, 1.0), rand(-1.0, 1.0)).normalize();
    let radius = rand(10.0, 100.0);
    let speed = rand(0.1, 0.4);
    let rotation_speed = rand(-1.0, 1.0);
    let scale = rand(2.0, 10.0);

    object_infos.push(ObjectInfo {
      bind_group,

-      uniform_buffer,
-      uniform_values,

      axis,
      radius,
      speed,
      rotation_speed,
      scale,
    });
  }
```

At render time we update all the objects values and then make
just one call to `queue.write_buffer`.

```rust
-      for (i, object) in object_infos.iter_mut().take(num_objects).enumerate() {
+      for (i, object) in object_infos.iter().take(num_objects).enumerate() {
        let math_time_start_ms = wgpu_fun::now_ms();

-        let uniform_values = &mut object.uniform_values;
+        // index into the one large array of uniform values
+        // (in JS these are the pre-made subarray views)
+        let f32_offset = i * uniform_buffer_space as usize / 4;
+        let uniform_values = &mut uniform_values[f32_offset..];

        // Compute a world matrix
        let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 3.721 + time * object.speed).sin() * object.radius))
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 9.721 + time * 0.1).sin() * object.radius))
          * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
          * Mat4::from_scale(Vec3::splat(object.scale));
        uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

        // Inverse and transpose it into the normalMatrix value
        let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
        uniform_values[K_NORMAL_MATRIX_OFFSET..][..3].copy_from_slice(&normal_matrix.x_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3].copy_from_slice(&normal_matrix.y_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3].copy_from_slice(&normal_matrix.z_axis.to_array());

        math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

-        // upload the uniform values to the uniform buffer
-        frame.queue.write_buffer(&object.uniform_buffer, 0, bytemuck::cast_slice(uniform_values));

        pass.set_bind_group(0, &object.bind_group, &[]);
        pass.draw_indexed(0..num_vertices, 0, 0..1);
      }

+      // upload all uniform values to the uniform buffer
+      if num_objects > 0 {
+        let size = (num_objects as u64 - 1) * uniform_buffer_space + uniform_buffer_size;
+        frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values[..size as usize / 4]));
+      }
```

{{{example url="../webgpu-optimization-step5-use-buffer-offsets.html"}}}

On the original author's machine that shaved off 40% of the JavaScript time!

# Optimization: Use Mapped Buffers

When we call `queue.write_buffer`, what happens is, WebGPU makes a copy of
the data. It copies that data to the GPU process (a separate
process that talks to the GPU for security). In the GPU process that data is
then copied to the GPU Buffer.

We can skip one of those copies by using mapped buffers instead. We'll map a
buffer, update the uniform values directly into that mapped buffer. Then we'll
unmap the buffer and issue a `copy_buffer_to_buffer` command to copy to the uniform
buffer. This will save a copy.

WebGPU mapping happens asynchronously so rather then map a buffer and wait for
it to be ready, we'll keep an array of already mapped buffers. Each frame, we
either get an already mapped buffer or create a new one that is already mapped.
After we render, we'll setup a callback to map the buffer when it's available
and put it back on the list of already mapped buffers. This way, we'll never
have to wait for a mapped buffer.

First we'll make a pool of mapped buffers and a function to either get a
pre-mapped buffer or make a new one. The pool is shared with the map-async
callback that refills it, so it goes in an `Arc<Mutex<...>>` (like the result
buffers in [the timing article](webgpu-timing.html)'s `TimingHelper`).

```rust
  // a pool of transfer buffers that are already mapped and ready to use
  let mapped_transfer_buffers: Arc<Mutex<Vec<wgpu::Buffer>>> = Arc::new(Mutex::new(Vec::new()));
  let get_mapped_transfer_buffer = {
    let device = app.device.clone();
    let mapped_transfer_buffers = mapped_transfer_buffers.clone();
    let size = uniform_buffer_space * max_objects as u64;
    move || {
      mapped_transfer_buffers.lock().unwrap().pop().unwrap_or_else(|| {
        device.create_buffer(&wgpu::BufferDescriptor {
          label: Some("transfer buffer"),
          size,
          usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
          mapped_at_creation: true,
        })
      })
    }
  };
```

Since we now write into whichever transfer buffer we got *this frame*, the one
big `uniform_values` `Vec` goes away.

One Rust-specific wrinkle: in JavaScript, `getMappedRange` returns an
`ArrayBuffer` you can make `Float32Array` views into and read *and* write. In
wgpu, `get_mapped_range_mut` hands us a **write-only** view of the mapped
memory (reading mapped memory back can be extremely slow — it's often
[write-combining memory](https://en.wikipedia.org/wiki/Write_combining)). So
instead of making views into the mapped buffer, we'll compute each object's
uniform values in a small local array and copy the bytes into the mapped
buffer.

```rust
-  // one large array of uniform values for all the objects
-  let mut uniform_values = vec![0.0f32; uniform_buffer.size() as usize / 4];
```

At render time we encode a command to copy the transfer buffer
to the uniform buffer *before* we start looping through the
objects. This is because the `copy_buffer_to_buffer` command is
a command on the `CommandEncoder`. We need it to run before
the objects are rendered but, as we loop over the objects we're
encoding render pass commands to render them. Before, we called
`queue.write_buffer` after updating the values, which
of course, executes first because we have not called `submit` yet
on our commands. In this case though, our copy actually is a command
so we have to encode it before the draw commands. This is fine because
remember, it's just a command, it will not be executed until we
submit the command buffer which means we can still update the transfer
buffer as the copy has not yet happened.

```rust
    let mut encoder = frame.device.create_command_encoder(&Default::default());

    ...

    let mut math_elapsed_time_ms = 0.0;

+    let transfer_buffer = get_mapped_transfer_buffer();
    {
+      // in JS this is `transferBuffer.getMappedRange()`; wgpu hands us
+      // a *write-only* view of the mapped memory
+      let mut mapped = transfer_buffer.slice(..).get_mapped_range_mut().unwrap();

+      // copy the uniform values from the transfer buffer to the uniform buffer
+      if num_objects > 0 {
+        // Remember, this is just encoding a command that will happen later.
+        let size = (num_objects as u64 - 1) * uniform_buffer_space + uniform_buffer_size;
+        encoder.copy_buffer_to_buffer(&transfer_buffer, 0, &uniform_buffer, 0, size);
+      }

      let mut pass = timing_helper.begin_render_pass(&mut encoder, &wgpu::RenderPassDescriptor {
        ...
      });
      pass.set_pipeline(&pipeline);
      pass.set_vertex_buffer(0, vertex_buffer.slice(..));
      pass.set_index_buffer(indices_buffer.slice(..), wgpu::IndexFormat::Uint16);

      for (i, object) in object_infos.iter().take(num_objects).enumerate() {
        let math_time_start_ms = wgpu_fun::now_ms();

-        // index into the one large array of uniform values
-        // (in JS these are the pre-made subarray views)
-        let f32_offset = i * uniform_buffer_space as usize / 4;
-        let uniform_values = &mut uniform_values[f32_offset..];
+        // The uniform values for this object.
+        // (in JS these are subarray views made into the mapped buffer;
+        // the mapped memory is write-only in wgpu so we use a local
+        // array and copy it into the mapped buffer below)
+        let mut uniform_values = [0.0f32; 12 + 16];

        // Compute a world matrix
        let world = Mat4::from_axis_angle(object.axis, i as f32 + time * object.speed)
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 3.721 + time * object.speed).sin() * object.radius))
          * Mat4::from_translation(Vec3::new(0.0, 0.0, (i as f32 * 9.721 + time * 0.1).sin() * object.radius))
          * Mat4::from_rotation_x(time * object.rotation_speed + i as f32)
          * Mat4::from_scale(Vec3::splat(object.scale));
        uniform_values[K_WORLD_OFFSET..][..16].copy_from_slice(&world.to_cols_array());

        // Inverse and transpose it into the normalMatrix value
        let normal_matrix = Mat3::from_mat4(world.inverse().transpose());
        uniform_values[K_NORMAL_MATRIX_OFFSET..][..3].copy_from_slice(&normal_matrix.x_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 4..][..3].copy_from_slice(&normal_matrix.y_axis.to_array());
        uniform_values[K_NORMAL_MATRIX_OFFSET + 8..][..3].copy_from_slice(&normal_matrix.z_axis.to_array());

+        // copy the values into the mapped transfer buffer
+        let uniform_buffer_offset = i * uniform_buffer_space as usize;
+        mapped
+          .slice(uniform_buffer_offset..uniform_buffer_offset + uniform_buffer_size as usize)
+          .copy_from_slice(bytemuck::cast_slice(&uniform_values));

        math_elapsed_time_ms += wgpu_fun::now_ms() - math_time_start_ms;

        pass.set_bind_group(0, &object.bind_group, &[]);
        pass.draw_indexed(0..num_vertices, 0, 0..1);
      }
    }
+    // the mapped range view was dropped at the end of the block above
+    transfer_buffer.unmap();

-      // upload all uniform values to the uniform buffer
-      if num_objects > 0 {
-        let size = (num_objects as u64 - 1) * uniform_buffer_space + uniform_buffer_size;
-        frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values[..size as usize / 4]));
-      }

    timing_helper.resolve_timing(&mut encoder);

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
```

Finally, as soon as we've submitted the command buffer we map the buffer again.
Mapping is asynchronous so when it's finally ready we'll add it back to the list
of already mapped buffers. Where JavaScript writes
`transferBuffer.mapAsync(GPUMapMode.WRITE).then(...)`, in wgpu `map_async`
takes a callback (delivered when the device is polled — which our frame
function already does once per frame).

```rust
    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);

+    // As soon as we've submitted the command buffer we map the buffer
+    // again. Mapping is asynchronous so when it's finally ready we'll add
+    // it back to the list of already mapped buffers.
+    {
+      let mapped_transfer_buffers = mapped_transfer_buffers.clone();
+      let buffer = transfer_buffer.clone();
+      transfer_buffer.map_async(wgpu::MapMode::Write, .., move |result| {
+        result.expect("failed to map transfer buffer");
+        mapped_transfer_buffers.lock().unwrap().push(buffer);
+      });
+    }
```

On the original author's machine, this version drew around 15000 objects at
75fps, which is about 87% more than the version we started with.

{{{example url="../webgpu-optimization-step6-use-mapped-buffers.html"}}}

With rendering unchecked, the difference was even bigger for him: 9000 at 75fps
with the original non-optimized example and 18000 at 75fps in this last
version. That's a 2x speed up!

Other things that *might* help

* **Double buffer the large uniform buffer**

  This comes up as a possible optimization because WebGPU can not update a
  buffer that is currently in use.

  So, imagine you start rendering (you call `queue.submit`). The GPU
  starts rendering using our large uniform buffer. You immediately try to update
  that buffer. In this case, WebGPU would have to pause and wait for the GPU to
  finish using the buffer for rendering.

  This is unlikely to happen in our example above. We don't directly update the
  uniform buffer. Instead we update a transfer buffer and then later, ask the
  GPU to copy it to the uniform buffer.

  This issue would be more likely to come up if we update a buffer directly on
  the GPU using a compute shader.

* **Compute matrix math with offsets**

  This one is specific to the JavaScript version of this article. The math
  library created in [the series on matrix math](webgpu-matrix-math.html)
  generates `Float32Array`s as outputs and takes in
  `Float32Array`s as inputs, but it can't update a `Float32Array` at some
  offset — which is why the JavaScript version has to create 2 temporary
  `Float32Array` views per object per frame, 40000 of them for 20000 objects.
  The original author tested a modified math library where every function
  takes offsets, i.e.

  ```js
      mat4.multiply(a, b, dst);
  ```

  becomes

  ```js
     mat4.multiply(a, aOffset, b, bOffset, dst, dstOffset);
  ```

  and [measured it to be about 7% faster](../webgpu-optimization-step6-use-mapped-buffers-math-w-offsets.html)
  (that link runs the JavaScript version).

  In Rust this issue doesn't really come up: glam computes matrices on the
  stack and we copy them into place with slice offsets, no per-object
  allocations involved. It's one of the places where the same code pattern is
  naturally cheaper in Rust.

* **Directly map the uniform buffer**

  In our example above we map a transfer buffer, a buffer that only has
  `COPY_SRC` and `MAP_WRITE` usage flags. We then have to call
  `encoder.copy_buffer_to_buffer` to copy the contents of that buffer into the
  actual uniform buffer.

  It would be much nicer if we could directly map the uniform buffer and avoid
  the copy. Unfortunately, that ability is not available in WebGPU version 1 but
  it is being considered as an optional feature sometime in the future,
  especially for *uniform memory architectures* like some ARM based devices.
  (Native wgpu has `wgpu::Features::MAPPABLE_PRIMARY_BUFFERS` which allows
  exactly this, but it's not available on the web and can be slow on discrete
  GPUs.)

* **Indirect Drawing**

  Indirect drawing refers to draw commands that take their parameters from a GPU buffer.

  ```rust
  pass.draw(vertices, instances);                       // direct
  pass.draw_indirect(&some_buffer, offset_into_buffer); // indirect
  ```

  In the indirect case above, `some_buffer` is a 16 byte portion of a GPU buffer that holds
  `[vertexCount, instanceCount, firstVertex, firstInstance]`.

  The advantage to indirect draw is that you can have the GPU itself fill out the values.
  You can even have the GPU set `vertexCount` and/or `instanceCount` to zero when you
  don't want that thing to be drawn.

  Using indirect drawing, you could do things like, for example, passing all of the
  objects' bounding boxes or bounding spheres to the GPU and then have the GPU do
  frustum culling and if the object is inside the frustum it would update that
  object's indirect drawing parameters to be drawn, otherwise it would update them
  to not be drawn. "frustum culling" is a fancy way to say "check if the object
  is possibly inside the frustum of the camera. We talked about frustums in
  [the article on perspective projection](webgpu-persective-projection.html).

* **Render Bundles**

  Render bundles let you pre-record a bunch of command buffer commands and then
  request them to be executed later. This can be useful, especially if your
  scene is relatively static, meaning you don't need to add or remove objects
  later.

  There's a great article [here](https://toji.dev/webgpu-best-practices/render-bundles)
  that combines render bundles, indirect draws, GPU frustum culling, to show
  some ideas for getting more speed in specialized situations.

* Immediates

  [Immediates](webgpu-immediates.html) where added in 2026. They
  are a fast way to send a little bit of data to a shader.
  They probably won't be as fast as some of the techniques in this
  article but they can be an easy first step to optimizing.

