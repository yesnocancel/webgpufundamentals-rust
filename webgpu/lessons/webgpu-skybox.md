Title: WebGPU SkyBox
Description: Show the sky with a skybox!
TOC: Skyboxes


This article continues from [the article on environment maps](webgpu-environment-maps.html).

An *skybox* is a box with textures on it to look like the sky in all directions
or rather to look like what is very far away including the horizon. Imagine
you're standing in a room and on each wall is a full size poster of some view,
add in a poster to cover the ceiling showing the sky and one for the floor
showing the ground and that's a skybox.

Lots of 3D games do this by just making a cube, making it really large, putting
a texture on it of the sky.

This works but it has issues. One issue is that you have a cube that you need to
view in multiple directions, Whatever direction the camera is facing. You want
everything to draw far away but you don't want the corners of the cube to go out
of the clipping plane. Complicating that issue, for performance reasons you want
to draw close things before far things because the GPU, using a [depth
texture](webgpu-orthographic.html), can skip drawing pixels it knows will fail
the test. So ideally you should draw the skybox last with the depth test on but
if you actually use a box, as the camera looks in different directions, the
corners of the box will be further away than the sides causing issues.

<div class="webgpu_center"><img src="resources/skybox-issues.svg" style="width: 500px"></div>

You can see above we need to make sure the furthest point of the cube is inside
the frustum but because of that some edges of the cube might end up covering up
objects that we don't want covered up.

The typical solution is to turn off the depth test and draw the skybox first but
then we don't get the performance benefit from the depth test not drawing pixels
that we'll later cover with stuff in our scene.

Instead of using a cube lets just [draw a triangle that covers the entire canvas](webgpu-large-triangle-to-cover-clip-space.html) and
use [a cubemap](webgpu-cube-maps.html). Normally we use a view projection matrix
to project geometry in 3D space. In this case we'll do the opposite. We'll use the
inverse of the view projection matrix to work backward and get the direction the
camera is looking for each pixel being drawn. This will give us directions to
look into the cubemap.

Starting with the [environment map example](webgpu-environment-maps.html)
since it already loads a cubemap and generates mips for it. 
Let's use a hard coded triangle. Here's the shader

```wgsl
struct Uniforms {
  viewDirectionProjectionInverse: mat4x4f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) pos: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;
@group(0) @binding(1) var ourSampler: sampler;
@group(0) @binding(2) var ourTexture: texture_cube<f32>;

@vertex fn vs(@builtin(vertex_index) vNdx: u32) -> VSOutput {
  let pos = array(
    vec2f(-1, 3),
    vec2f(-1,-1),
    vec2f( 3,-1),
  );
  var vsOut: VSOutput;
  vsOut.position = vec4f(pos[vNdx], 1, 1);
  vsOut.pos = vsOut.position;
  return vsOut;
}
```

You can see above, first we set `@builtin(position)` via `vsOut.position`
to the our vertex position and we explicitly set z to 1 so the
quad will be dawn at the furthest z value. We also pass the vertex position
to the fragment shader.

In the fragment shader we multiply the position by the inverse view projection
matrix and divide by w to go from 4D space to 3D space. This is the same divide
happens to `@builtin(position)` in the vertex shader but here we're doing it
ourselves.

```glsl
@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  let t = uni.viewDirectionProjectionInverse * vsOut.pos;
  return textureSample(ourTexture, ourSampler, normalize(t.xyz / t.w) * vec3f(1, 1, -1));
}
```

Note: We multiply the z direction by -1 for
[the reasons we covered in the previous article](webgpu-environment-maps.html#a-flipped).

The pipeline has no buffers in the vertex stage

```rust
  let pipeline = app
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: Some("no attributes"),
          layout: None,
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
          depth_stencil: Some(wgpu::DepthStencilState {
              depth_write_enabled: Some(true),
              depth_compare: Some(wgpu::CompareFunction::LessEqual),
              format: wgpu::TextureFormat::Depth24Plus,
              stencil: Default::default(),
              bias: Default::default(),
          }),
          multisample: Default::default(),
          multiview_mask: None,
          cache: None,
      });
```

Notice we set the `depth_compare` to `LessEqual` instead of `Less` because
we clear the depth texture to 1.0 and we're rendering at 1.0. 1.0 is not less
than 1.0 so we'd render nothing if we didn't change this to `LessEqual`.

Again we need to setup a uniform buffer

```rust
  // viewDirectionProjectionInverse
  const UNIFORM_BUFFER_SIZE: u64 = (16) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms"),
      size: UNIFORM_BUFFER_SIZE,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
  const K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET: usize = 0;
```

and set it at render time

```rust
    let aspect = frame.width as f32 / frame.height as f32;
    let projection = Mat4::perspective_rh(
        60.0f32.to_radians(),
        aspect,
        0.1,  // zNear
        10.0, // zFar
    );
    // Camera going in circle from origin looking at origin
    let camera_position = Vec3::new((time * 0.1).cos(), 0.0, (time * 0.1).sin());
    let mut view = Mat4::look_at_rh(
        camera_position,
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );
    // We only care about direction so remove the translation
    view.w_axis.x = 0.0;
    view.w_axis.y = 0.0;
    view.w_axis.z = 0.0;

    let view_projection = projection * view;
    let view_direction_projection_inverse = view_projection.inverse();

    uniform_values[K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET
        ..K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET + 16]
        .copy_from_slice(&view_direction_projection_inverse.to_cols_array());

    // upload the uniform values to the uniform buffer
    frame
        .queue
        .write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

Notice above we're spinning the camera around the origin where we compute
`camera_position`. Then, after make a `view` matrix we
zero out the translation (the first 3 elements of the matrix's `w_axis`
column) since we only care which way the camera is facing, not
where it is.

From that we multiply with the projection matrix, take the inverse, and then set
the matrix.

{{{example url="../webgpu-skybox.html" }}}

Let's combine the environment mapped cube back into this sample.
First off lets's rename a bunch of variables

From the skybox example

```
module -> sky_box_module
pipeline -> sky_box_pipeline
uniform_buffer -> sky_box_uniform_buffer
uniform_values -> sky_box_uniform_values
bind_group -> sky_box_bind_group
```

Similarly from the environment map example

```
module -> env_map_module
pipeline -> env_map_pipeline
uniform_buffer -> env_map_uniform_buffer
uniform_values -> env_map_uniform_values
bind_group -> env_map_bind_group
```

With those renamed we just have to update our rendering code. First we
update the uniform values for both

```rust
    let aspect = frame.width as f32 / frame.height as f32;
    let projection = Mat4::perspective_rh(
        60.0f32.to_radians(),
        aspect,
        0.1,  // zNear
        10.0, // zFar
    );
    // Camera going in circle from origin looking at origin
    let camera_position = Vec3::new((time * 0.1).cos() * 5.0, 0.0, (time * 0.1).sin() * 5.0);
    let mut view = Mat4::look_at_rh(
        camera_position,
        Vec3::new(0.0, 0.0, 0.0), // target
        Vec3::new(0.0, 1.0, 0.0), // up
    );
    // Copy the view into the uniform values since we're going
    // to zero out the view's translation
    env_map_uniform_values[K_VIEW_OFFSET..K_VIEW_OFFSET + 16]
        .copy_from_slice(&view.to_cols_array());

    // We only care about direction so remove the translation
    view.w_axis.x = 0.0;
    view.w_axis.y = 0.0;
    view.w_axis.z = 0.0;
    let view_projection = projection * view;
    let view_direction_projection_inverse = view_projection.inverse();

    // Rotate the cube
    let world = Mat4::from_rotation_x(time * -0.1) * Mat4::from_rotation_y(time * -0.2);

    env_map_uniform_values[K_PROJECTION_OFFSET..K_PROJECTION_OFFSET + 16]
        .copy_from_slice(&projection.to_cols_array());
    env_map_uniform_values[K_WORLD_OFFSET..K_WORLD_OFFSET + 16]
        .copy_from_slice(&world.to_cols_array());
    env_map_uniform_values[K_CAMERA_POSITION_OFFSET..K_CAMERA_POSITION_OFFSET + 3]
        .copy_from_slice(&camera_position.to_array());
    sky_box_uniform_values[K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET
        ..K_VIEW_DIRECTION_PROJECTION_INVERSE_OFFSET + 16]
        .copy_from_slice(&view_direction_projection_inverse.to_cols_array());

    // upload the uniform values to the uniform buffers
    frame.queue.write_buffer(
        &env_map_uniform_buffer,
        0,
        bytemuck::cast_slice(&env_map_uniform_values),
    );
    frame.queue.write_buffer(
        &sky_box_uniform_buffer,
        0,
        bytemuck::cast_slice(&sky_box_uniform_values),
    );
```

Then we render both. The environment mapped cube first and the skybox second
to show that drawing it second works.

```rust
    // Draw the cube
    pass.set_pipeline(&env_map_pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    pass.set_bind_group(0, &env_map_bind_group, &[]);
    pass.draw_indexed(0..num_vertices, 0, 0..1);

    // Draw the skyBox
    pass.set_pipeline(&sky_box_pipeline);
    pass.set_bind_group(0, &sky_box_bind_group, &[]);
    pass.draw(0..3, 0..1);
```

{{{example url="../webgpu-skybox-plus-environment-map.html" }}}

I hope these last 2 articles have given you some idea of how to use a cubemap.
It's common for example to take the code [from computing lighting](webgpu-lighting-spot.html)
and combine that result with results from
an environment map to make materials like the hood of a car or polished floor.

