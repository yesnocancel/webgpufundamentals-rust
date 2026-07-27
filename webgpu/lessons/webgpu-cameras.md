Title: WebGPU Cameras
Description: Cameras via Matrices
TOC: Cameras

This article is the 7th in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="matrix-math.hanson"}}}

In the last post we had to move the F in front of the frustum because the
`m4::perspective` function puts the eye at at the origin (0, 0, 0) and
that objects in the frustum are between `-zNear` to `-zFar` in front of it.
This means, anything we want to appear, needs to be placed this this space.

In the real world you usually move your camera to take a picture of a
some object

<div class="webgpu_center" style="width: 512px">
   <div data-diagram="move-camera"></div>
   <div class="caption">moving the camera to the objects</div>
</div>

But, in our last post, we came up with a projection matrix that requires things to
be in front of the origin on the -Z axis.  To achieve this, what we want to
do is, move the camera to the origin and move everything else the right
amount so it's still in the same place *relative to the camera*.

<div class="webgpu_center" style="width: 512px">
   <div data-diagram="move-world"></div>
   <div class="caption">moving the objects to the view</div>
</div>

We need to effectively move the world in front of the camera.  The easiest way
to do this is to use an "inverse" matrix.  The math to compute an inverse matrix
in the general case is complex but conceptually it's easy. The inverse is the
value you'd use to negate some other value.  For example, the inverse of a
matrix that translates in X by 123 is a matrix that translates in X by -123.
The inverse of a matrix that scales by 5 is a matrix that scales by 1/5th or
0.2.  The inverse of a matrix that rotates 30&deg; around the X axis would be
one that rotates -30&deg; around the X axis.

Up until this point we've used translation, rotation and scale to affect the
position and orientation of our 'F'.  After multiplying all the matrices
together we have a single matrix that represents how to move the 'F' from the
origin to the place, size and orientation we want it.  We can do the same for a
camera.  Once we have the matrix that tells us how to move and rotate the camera
from the origin to where we want it we can compute its inverse which will give
us a matrix that tells us how to move and rotate everything else the opposite
amount which will effectively make it so the camera is at (0, 0, 0) and we've
moved everything in front of it.

Let's make a 3D scene with a circle of 'F's like the diagrams above.

First things first, lets adjust our F vertex data. We originally started in 2D
with pixels. The top left corner of the F is at 0,0 and extends 100 pixels right
and 150 pixels down. "Pixels" probably make no sense as a unit in 3D and the
perspective projection matrix we made uses positive Y up so, let's flip our F so
positive Y is up and let's center it around the origin.

```rust
    let positions: Vec<f32> = vec![
-        // left column
-        0.0, 0.0, 0.0,
-        30.0, 0.0, 0.0,
-        0.0, 150.0, 0.0,
-        30.0, 150.0, 0.0,
-
-        // top rung
-        30.0, 0.0, 0.0,
-        100.0, 0.0, 0.0,
-        30.0, 30.0, 0.0,
-        100.0, 30.0, 0.0,
-
-        // middle rung
-        30.0, 60.0, 0.0,
-        70.0, 60.0, 0.0,
-        30.0, 90.0, 0.0,
-        70.0, 90.0, 0.0,
-
-        // left column back
-        0.0, 0.0, 30.0,
-        30.0, 0.0, 30.0,
-        0.0, 150.0, 30.0,
-        30.0, 150.0, 30.0,
-
-        // top rung back
-        30.0, 0.0, 30.0,
-        100.0, 0.0, 30.0,
-        30.0, 30.0, 30.0,
-        100.0, 30.0, 30.0,
-
-        // middle rung back
-        30.0, 60.0, 30.0,
-        70.0, 60.0, 30.0,
-        30.0, 90.0, 30.0,
-        70.0, 90.0, 30.0,
+        // left column
+        -50.0,  75.0,  15.0,
+        -20.0,  75.0,  15.0,
+        -50.0, -75.0,  15.0,
+        -20.0, -75.0,  15.0,
+
+        // top rung
+        -20.0,  75.0,  15.0,
+         50.0,  75.0,  15.0,
+        -20.0,  45.0,  15.0,
+         50.0,  45.0,  15.0,
+
+        // middle rung
+        -20.0,  15.0,  15.0,
+         20.0,  15.0,  15.0,
+        -20.0, -15.0,  15.0,
+         20.0, -15.0,  15.0,
+
+        // left column back
+        -50.0,  75.0, -15.0,
+        -20.0,  75.0, -15.0,
+        -50.0, -75.0, -15.0,
+        -20.0, -75.0, -15.0,
+
+        // top rung back
+        -20.0,  75.0, -15.0,
+         50.0,  75.0, -15.0,
+        -20.0,  45.0, -15.0,
+         50.0,  45.0, -15.0,
+
+        // middle rung back
+        -20.0,  15.0, -15.0,
+         20.0,  15.0, -15.0,
+        -20.0, -15.0, -15.0,
+         20.0, -15.0, -15.0,
    ];
```

Further, as we went over in
[the previous article](webgpu-perspective-projection.html),
because we were using positive Y = down to match most 2D pixel libraries, we had
our triangle vertex order backward for normal 3D and ended up culling the the
`'front'` facing triangles instead of the normal `'back'` facing triangles since
were scaling Y by negative 1. Now that we're doing *normal* 3D with positive Y =
up, let's flip the order of the vertices so that clockwise triangles are facing
out.

```rust
    let indices: Vec<u32> = vec![
-         0,  1,  2,    2,  1,  3,  // left column
-         4,  5,  6,    6,  5,  7,  // top run
-         8,  9, 10,   10,  9, 11,  // middle run
-
-        12, 14, 13,   14, 15, 13,  // left column back
-        16, 18, 17,   18, 19, 17,  // top run back
-        20, 22, 21,   22, 23, 21,  // middle run back
-
-         0, 12,  5,   12, 17,  5,   // top
-         5, 17,  7,   17, 19,  7,   // top rung right
-         6,  7, 18,   18,  7, 19,   // top rung bottom
-         6, 18,  8,   18, 20,  8,   // between top and middle rung
-         8, 20,  9,   20, 21,  9,   // middle rung top
-         9, 21, 11,   21, 23, 11,   // middle rung right
-        10, 11, 22,   22, 11, 23,   // middle rung bottom
-        10, 22,  3,   22, 15,  3,   // stem right
-         2,  3, 14,   14,  3, 15,   // bottom
-         0,  2, 12,   12,  2, 14,   // left
+         0,  2,  1,    2,  3,  1,   // left column
+         4,  6,  5,    6,  7,  5,   // top run
+         8, 10,  9,   10, 11,  9,   // middle run
+
+        12, 13, 14,   14, 13, 15,   // left column back
+        16, 17, 18,   18, 17, 19,   // top run back
+        20, 21, 22,   22, 21, 23,   // middle run back
+
+         0,  5, 12,   12,  5, 17,   // top
+         5,  7, 17,   17,  7, 19,   // top rung right
+         6, 18,  7,   18, 19,  7,   // top rung bottom
+         6,  8, 18,   18,  8, 20,   // between top and middle rung
+         8,  9, 20,   20,  9, 21,   // middle rung top
+         9, 11, 21,   21, 11, 23,   // middle rung right
+        10, 22, 11,   22, 23, 11,   // middle rung bottom
+        10,  3, 22,   22,  3, 15,   // stem right
+         2, 14,  3,   14, 15,  3,   // bottom
+         0, 12,  2,   12, 14,  2,   // left
    ];
```

Finally let's set the `cull_mode` to cull *back facing* triangles.

```rust
    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("2 attributes"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: None,
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: (4) * 4, // (3) floats 4 bytes each + one 4 byte color
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        // position
                        wgpu::VertexAttribute {
                            shader_location: 0,
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        // color
                        wgpu::VertexAttribute {
                            shader_location: 1,
                            offset: 12,
                            format: wgpu::VertexFormat::Unorm8x4,
                        },
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
-                cull_mode: Some(wgpu::Face::Front), // note: uncommon setting. See article
+                cull_mode: Some(wgpu::Face::Back),
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

Here's a function that given a matrix will compute its inverse matrix.

```rust
mod m4 {
  ...

+    pub fn inverse(m: &[f32; 16]) -> [f32; 16] {
+        let mut dst = [0.0; 16];
+
+        let m00 = m[0 * 4 + 0];
+        let m01 = m[0 * 4 + 1];
+        let m02 = m[0 * 4 + 2];
+        let m03 = m[0 * 4 + 3];
+        let m10 = m[1 * 4 + 0];
+        let m11 = m[1 * 4 + 1];
+        let m12 = m[1 * 4 + 2];
+        let m13 = m[1 * 4 + 3];
+        let m20 = m[2 * 4 + 0];
+        let m21 = m[2 * 4 + 1];
+        let m22 = m[2 * 4 + 2];
+        let m23 = m[2 * 4 + 3];
+        let m30 = m[3 * 4 + 0];
+        let m31 = m[3 * 4 + 1];
+        let m32 = m[3 * 4 + 2];
+        let m33 = m[3 * 4 + 3];
+
+        let tmp0 = m22 * m33;
+        let tmp1 = m32 * m23;
+        let tmp2 = m12 * m33;
+        let tmp3 = m32 * m13;
+        let tmp4 = m12 * m23;
+        let tmp5 = m22 * m13;
+        let tmp6 = m02 * m33;
+        let tmp7 = m32 * m03;
+        let tmp8 = m02 * m23;
+        let tmp9 = m22 * m03;
+        let tmp10 = m02 * m13;
+        let tmp11 = m12 * m03;
+        let tmp12 = m20 * m31;
+        let tmp13 = m30 * m21;
+        let tmp14 = m10 * m31;
+        let tmp15 = m30 * m11;
+        let tmp16 = m10 * m21;
+        let tmp17 = m20 * m11;
+        let tmp18 = m00 * m31;
+        let tmp19 = m30 * m01;
+        let tmp20 = m00 * m21;
+        let tmp21 = m20 * m01;
+        let tmp22 = m00 * m11;
+        let tmp23 = m10 * m01;
+
+        let t0 = (tmp0 * m11 + tmp3 * m21 + tmp4 * m31) - (tmp1 * m11 + tmp2 * m21 + tmp5 * m31);
+        let t1 = (tmp1 * m01 + tmp6 * m21 + tmp9 * m31) - (tmp0 * m01 + tmp7 * m21 + tmp8 * m31);
+        let t2 = (tmp2 * m01 + tmp7 * m11 + tmp10 * m31) - (tmp3 * m01 + tmp6 * m11 + tmp11 * m31);
+        let t3 = (tmp5 * m01 + tmp8 * m11 + tmp11 * m21) - (tmp4 * m01 + tmp9 * m11 + tmp10 * m21);
+
+        let d = 1.0 / (m00 * t0 + m10 * t1 + m20 * t2 + m30 * t3);
+
+        dst[0] = d * t0;
+        dst[1] = d * t1;
+        dst[2] = d * t2;
+        dst[3] = d * t3;
+
+        dst[4] = d * ((tmp1 * m10 + tmp2 * m20 + tmp5 * m30) - (tmp0 * m10 + tmp3 * m20 + tmp4 * m30));
+        dst[5] = d * ((tmp0 * m00 + tmp7 * m20 + tmp8 * m30) - (tmp1 * m00 + tmp6 * m20 + tmp9 * m30));
+        dst[6] = d * ((tmp3 * m00 + tmp6 * m10 + tmp11 * m30) - (tmp2 * m00 + tmp7 * m10 + tmp10 * m30));
+        dst[7] = d * ((tmp4 * m00 + tmp9 * m10 + tmp10 * m20) - (tmp5 * m00 + tmp8 * m10 + tmp11 * m20));
+
+        dst[8] = d * ((tmp12 * m13 + tmp15 * m23 + tmp16 * m33) - (tmp13 * m13 + tmp14 * m23 + tmp17 * m33));
+        dst[9] = d * ((tmp13 * m03 + tmp18 * m23 + tmp21 * m33) - (tmp12 * m03 + tmp19 * m23 + tmp20 * m33));
+        dst[10] = d * ((tmp14 * m03 + tmp19 * m13 + tmp22 * m33) - (tmp15 * m03 + tmp18 * m13 + tmp23 * m33));
+        dst[11] = d * ((tmp17 * m03 + tmp20 * m13 + tmp23 * m23) - (tmp16 * m03 + tmp21 * m13 + tmp22 * m23));
+
+        dst[12] = d * ((tmp14 * m22 + tmp17 * m32 + tmp13 * m12) - (tmp16 * m32 + tmp12 * m12 + tmp15 * m22));
+        dst[13] = d * ((tmp20 * m32 + tmp12 * m02 + tmp19 * m22) - (tmp18 * m22 + tmp21 * m32 + tmp13 * m02));
+        dst[14] = d * ((tmp18 * m12 + tmp23 * m32 + tmp15 * m02) - (tmp22 * m32 + tmp14 * m02 + tmp19 * m12));
+        dst[15] = d * ((tmp22 * m22 + tmp16 * m02 + tmp21 * m12) - (tmp20 * m12 + tmp23 * m22 + tmp17 * m02));
+
+        dst
+    }
...
```

Like we've done in previous examples, to draw 5 things we need 5
uniform buffers and 5 bind groups.

```rust
    // matrix
    const UNIFORM_BUFFER_SIZE: u64 = (16) * 4;

    // offsets to the various uniform values in float32 indices
    const K_MATRIX_OFFSET: usize = 0;

+    struct ObjectInfo {
+        uniform_buffer: wgpu::Buffer,
+        uniform_values: [f32; UNIFORM_BUFFER_SIZE as usize / 4],
+        bind_group: wgpu::BindGroup,
+    }
+
+    const NUM_FS: usize = 5;
+    let mut object_infos: Vec<ObjectInfo> = Vec::new();
+    for _i in 0..NUM_FS {
        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: UNIFORM_BUFFER_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

        let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bind group for object"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

+        object_infos.push(ObjectInfo {
+            uniform_buffer,
+            uniform_values,
+            bind_group,
+        });
+    }
```

Let's get rid of some of the settings to unclutter our example. On the
example page's JavaScript side

```js
  const settings = {
    fieldOfView: degToRad(100),
-    translation: [-65, 0, -120],
-    rotation: [degToRad(220), degToRad(25), degToRad(325)],
-    scale: [1, 1, 1],
  };
```

and in the Rust render code

```rust
    let field_of_view = wgpu_fun::setting_f64("fieldOfView", 100.0f64.to_radians()) as f32;
-    let translation = [
-        wgpu_fun::setting_f64("translationX", -65.0) as f32,
-        wgpu_fun::setting_f64("translationY", 0.0) as f32,
-        wgpu_fun::setting_f64("translationZ", -120.0) as f32,
-    ];
-    let rotation = [
-        wgpu_fun::setting_f64("rotationX", 220.0f64.to_radians()) as f32,
-        wgpu_fun::setting_f64("rotationY", 25.0f64.to_radians()) as f32,
-        wgpu_fun::setting_f64("rotationZ", 325.0f64.to_radians()) as f32,
-    ];
-    let scale = [
-        wgpu_fun::setting_f64("scaleX", 1.0) as f32,
-        wgpu_fun::setting_f64("scaleY", 1.0) as f32,
-        wgpu_fun::setting_f64("scaleZ", 1.0) as f32,
-    ];

  ...

-    matrix_value = m4::translate(&matrix_value, translation);
-    matrix_value = m4::rotate_x(&matrix_value, rotation[0]);
-    matrix_value = m4::rotate_y(&matrix_value, rotation[1]);
-    matrix_value = m4::rotate_z(&matrix_value, rotation[2]);
-    matrix_value = m4::scale(&matrix_value, scale);
```

Because we are drawing 5 things and they will all use the same
projection matrix we'll calculate it before the loop of drawing the Fs

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {
    ...

        let aspect = frame.width as f32 / frame.height as f32;
-        let mut matrix_value = m4::perspective(
+        let projection = m4::perspective(
            field_of_view,
            aspect,
            1.0,    // zNear
            2000.0, // zFar
        );
```

Next we'll compute a camera matrix. This matrix represents the
position and orientation of the camera in the world.  The code
below makes a matrix that rotates the camera around the origin
radius * 1.5 distance out and looking at the origin.

<div class="webgpu_center" style="width: 512px">
   <div data-diagram="camera-movement"></div>
   <div class="caption">camera movement</div>
</div>

```rust
+    let radius = 200.0f32;
```

and on the example page's JavaScript side

```js
  const settings = {
    fieldOfView: degToRad(100),
+    cameraAngle: 0,
  };
```

then in the render code

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {

     ...

+        let camera_angle = wgpu_fun::setting_f64("cameraAngle", 0.0) as f32;

+        // compute a matrix for the camera.
+        let camera_matrix = m4::rotation_y(camera_angle);
+        let camera_matrix = m4::translate(&camera_matrix, [0.0, 0.0, radius * 1.5]);
```

We then compute a "view matrix" from the camera matrix.  A "view matrix"
is the matrix that moves everything the opposite of the camera effectively
making everything relative to the camera as though the camera was at the
origin (0,0,0). We can do this by using the `inverse` function that computes
the inverse matrix (the matrix that does the exact opposite of the supplied matrix).
In this case the supplied matrix would move the camera to some position
and orientation relative to the origin. The inverse of that is a matrix
that will move everything else such that the camera is at the origin.

```rust
        // Make a view matrix from the camera matrix.
        let view_matrix = m4::inverse(&camera_matrix);
```

Now we combine the view and projection matrix into a view projection matrix.

```rust
+        // combine the view and projection matrixes
+        let view_projection_matrix = m4::multiply(&projection, &view_matrix);
```

Finally we draw a circle of Fs. For each F we start with the
view projection matrix, then compute a position on a circle and
translate to that position.

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {
    ...

        let aspect = frame.width as f32 / frame.height as f32;
        let projection = m4::perspective(
            field_of_view,
            aspect,
            1.0,    // zNear
            2000.0, // zFar
        );

        // compute a matrix for the camera.
        let camera_matrix = m4::rotation_y(camera_angle);
        let camera_matrix = m4::translate(&camera_matrix, [0.0, 0.0, radius * 1.5]);

        // Make a view matrix from the camera matrix.
        let view_matrix = m4::inverse(&camera_matrix);

        // combine the view and projection matrixes
        let view_projection_matrix = m4::multiply(&projection, &view_matrix);

+        for (i, object_info) in object_infos.iter_mut().enumerate() {
+            let angle = i as f32 / NUM_FS as f32 * std::f32::consts::PI * 2.0;
+            let x = angle.cos() * radius;
+            let z = angle.sin() * radius;
+
+            let matrix_value = m4::translate(&view_projection_matrix, [x, 0.0, z]);
+            object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
+                .copy_from_slice(&matrix_value);

            // upload the uniform values to the uniform buffer
            frame.queue.write_buffer(
                &object_info.uniform_buffer,
                0,
                bytemuck::cast_slice(&object_info.uniform_values),
            );

            pass.set_bind_group(0, &object_info.bind_group, &[]);
            pass.draw(0..num_vertices, 0..1);
+        }
```

And voila!  A camera that goes around the circle of 'F's.  Drag the
`cameraAngle` slider to move the camera around.

{{{example url="../webgpu-cameras-step-1-direct-math.html" }}}

That's all fine but using rotate and translate to move a camera where you
want it and point toward what you want to see is not always easy.  For
example if we wanted the camera to always point at a specific one of the
'F's it would take some pretty crazy math to compute how to rotate the
camera to point at that 'F' while it goes around the circle of 'F's.

Fortunately there's an easier way.  We can just decide where we want the
camera and what we want it to point at and then compute a matrix that will
put the camera there.  Based on how matrices work this is surprisingly
easy.

First we need to know where we want the camera.  We'll call this the
`eye`.  Then we need to know the position of the thing we want
to look at or aim at.  We'll call it the `target`.  If we subtract the
`target` from the `eye` we'll have a vector that points in the
direction we'd need to go from the camera to get to the target.  Let's
call it `zAxis`.  Since we know the camera points in the -Z direction we
can subtract the other way `eye - target`. We normalize the
results and copy it directly into the `z` part of a matrix.

<div class="webgpu_center">
  <div class="glocal-center">
    <table class="glocal-center-content glocal-mat">
      <tr>
        <td class="m11"> </td>
        <td class="m12"> </td>
        <td class="m13">Zx</td>
        <td class="m14"> </td>
      </tr>
      <tr>
        <td class="m21"> </td>
        <td class="m22"> </td>
        <td class="m23">Zy</td>
        <td class="m24"> </td>
      </tr>
      <tr>
        <td class="m31"> </td>
        <td class="m32"> </td>
        <td class="m33">Zz</td>
        <td class="m34"> </td>
      </tr>
      <tr>
        <td class="m41"> </td>
        <td class="m42"> </td>
        <td class="m43"> </td>
        <td class="m44"> </td>
      </tr>
    </table>
  </div>
</div>

This part of a matrix represents the Z axis.  In this case the Z-axis of
the camera.  Normalizing a vector means making it a vector that represents
1.0 unit.  If you go back to [the rotation article](webgpu-rotation.html)
where we talked about unit circles and how those helped with 2D rotation.
In 3D we need unit spheres and a normalized vector represents a point on a
unit sphere.

<div class="webgpu_center" style="width: 768px">
  <div data-diagram="cross-product-00"></div>
  <div class="caption">the <span class='z-axis'>z axis</span></div>
</div>

That's not enough info though.  Just a single vector gives us a point on a
unit sphere but which orientation from that point to orient things?  We
need to fill out the other parts of the matrix.  Specifically the X axis
and Y axis parts.  We know that in general, these 3 parts are perpendicular
to each other.  We also know that "in general", we don't point the camera
straight up.  Given that, if we know which way is up, in this case
(0,1,0), We can use that and something called a "cross product" to compute
the X axis and Y axis for the matrix.

I have no idea what a cross product means in mathematical terms.  What I
do know is that, if you have 2 unit vectors and you compute the cross
product of them you'll get a vector that is perpendicular to those 2
vectors.  In other words, if you have a vector pointing south east, and a
vector pointing up, and you compute the cross product you'll get a vector
pointing either south west or north east since those are the 2 vectors
that are perpendicular to south east and up.  Depending on which order you
compute the cross product in, you'll get the opposite answer.

In any case if we compute the cross product of our <span class="z-axis">`zAxis`</span> and
<span style="color: gray;">`up`</span> we'll get the <span class="x-axis">xAxis</span> for the camera.

<div class="webgpu_center" style="width: 768px">
  <div data-diagram="cross-product-01"></div>
  <div class="caption"><span style='color:gray;'>up</span> cross <span class='z-axis'>zAxis</span> = <span class='x-axis'>xAxis</span></div>
</div>

And now that we have the <span class="x-axis">`xAxis`</span> we can cross the <span class="z-axis">`zAxis`</span> and the <span class="x-axis">`xAxis`</span>
which will give us the camera's <span class="y-axis">`yAxis`</span>

<div class="webgpu_center" style="width: 768px">
  <div data-diagram="cross-product-02"></div>
  <div class="caption"><span class='z-axis'>zAxis</span> cross <span class='x-axis'>xAxis</span> = <span class='y-axis'>yAxis</span></div>
</div>

Now all we have to do is plug the 3 axes into a matrix. That gives us a
matrix that will orient something that points at the `target` from the
`eye`. We just need to put in the `eye` position in the final column.

<div class="webgpu_center">
  <div class="glocal-center">
    <table class="glocal-center-content glocal-mat">
      <tbody>
        <tr class="vertical-spans">
          <td><span class="x-axis">x axis →</span></td>
          <td><span class="y-axis">y axis →</span></td>
          <td><span class="z-axis">z axis →</span></td>
          <td><span>eye position →</span></td>
        </tr>
        <tr>
          <td class="m11">Xx</td>
          <td class="m12">Yx</td>
          <td class="m13">Zx</td>
          <td class="m14">Tx</td>
        </tr>
        <tr>
          <td class="m21">Xy</td>
          <td class="m22">Yy</td>
          <td class="m23">Zy</td>
          <td class="m24">Ty</td>
        </tr>
        <tr>
          <td class="m31">Xz</td>
          <td class="m32">Yz</td>
          <td class="m33">Zz</td>
          <td class="m34">Tz</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </tbody>
    </table>
  </div>
</div>

Here's the code to compute the cross product of 2 vectors.
Like our matrix code, it returns a new array.

```rust
+mod vec3 {
+    pub fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        let t0 = a[1] * b[2] - a[2] * b[1];
+        let t1 = a[2] * b[0] - a[0] * b[2];
+        let t2 = a[0] * b[1] - a[1] * b[0];
+
+        dst[0] = t0;
+        dst[1] = t1;
+        dst[2] = t2;
+
+        dst
+    }
+}
```

Here's the code to subtract two vectors.


```rust
mod vec3 {
  ...
+    pub fn subtract(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0] - b[0];
+        dst[1] = a[1] - b[1];
+        dst[2] = a[2] - b[2];
+
+        dst
+    }
```

Here's the code to normalize a vector (make it into a unit vector).

```rust
mod vec3 {
  ...
+    pub fn normalize(v: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
+        // make sure we don't divide by 0.
+        if length > 0.00001 {
+            dst[0] = v[0] / length;
+            dst[1] = v[1] / length;
+            dst[2] = v[2] / length;
+        } else {
+            dst[0] = 0.0;
+            dst[1] = 0.0;
+            dst[2] = 0.0;
+        }
+
+        dst
+    }
```

Here's the code to compute a *camera* matrix. It follows the steps described above.

```rust
mod m4 {
  ...
    #[rustfmt::skip]
    pub fn camera_aim(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let z_axis = vec3::normalize(vec3::subtract(eye, target));
        let x_axis = vec3::normalize(vec3::cross(up, z_axis));
        let y_axis = vec3::normalize(vec3::cross(z_axis, x_axis));

        dst[ 0] = x_axis[0];  dst[ 1] = x_axis[1];  dst[ 2] = x_axis[2];  dst[ 3] = 0.0;
        dst[ 4] = y_axis[0];  dst[ 5] = y_axis[1];  dst[ 6] = y_axis[2];  dst[ 7] = 0.0;
        dst[ 8] = z_axis[0];  dst[ 9] = z_axis[1];  dst[10] = z_axis[2];  dst[11] = 0.0;
        dst[12] = eye[0];     dst[13] = eye[1];     dst[14] = eye[2];     dst[15] = 1.0;

        dst
    }
  ...
```

And here is how we might use it to make the camera point at a specific 'F'
as we move it.

```rust
-        // compute a matrix for the camera.
-        let camera_matrix = m4::rotation_y(camera_angle);
-        let camera_matrix = m4::translate(&camera_matrix, [0.0, 0.0, radius * 1.5]);
+        // Compute the position of the first F
+        let f_position = [radius, 0.0, 0.0];
+
+        // Use matrix math to compute a position on a circle where
+        // the camera is
+        let temp_matrix = m4::rotation_y(camera_angle);
+        let temp_matrix = m4::translate(&temp_matrix, [0.0, 0.0, radius * 1.5]);
+
+        // Get the camera's position from the matrix we computed
+        let eye = [temp_matrix[12], temp_matrix[13], temp_matrix[14]];
+
+        let up = [0.0, 1.0, 0.0];
+
+        // Compute the camera's matrix using cameraAim
+        let camera_matrix = m4::camera_aim(eye, f_position, up);

        // Make a view matrix from the camera matrix.
        let view_matrix = m4::inverse(&camera_matrix);
```

And here's the result.

{{{example url="../webgpu-cameras-step-2-camera-aim.html" }}}

Drag the slider and notice how the camera tracks a single 'F'.

Most math libraries don't have a `cameraAim` function. Instead they have a `lookAt` function
which computes exactly what our `cameraAim` function does but ALSO converts it to a view matrix.
Functionally `look_at` could be implemented like this

```rust
mod m4 {
  ...
+    pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
+        inverse(&camera_aim(eye, target, up))
+    }
  ...
}
```

Using this `look_at` function our code would change to this

```rust
-        // Compute the camera's matrix using look at.
-        let camera_matrix = m4::camera_aim(eye, f_position, up);
-
-        // Make a view matrix from the camera matrix.
-        let view_matrix = m4::inverse(&camera_matrix);
+        // Compute a view matrix
+        let view_matrix = m4::look_at(eye, f_position, up);
```

{{{example url="../webgpu-cameras-step-3-look-at.html" }}}

Note that you can use this type of "aim" math for more than just cameras.
Common uses are making a character's head follow some target.  Making a turret aim
at a target.  Making an object follow a path.  You compute where on the path the
target is.  Then you compute where on the path the target would be a few moments
in the future.  Plug those 2 values into your `aim` function and you'll get a
matrix that makes your object follow the path and orient toward the path as
well.

Usually to "aim" something you want it to point down the positive Z axis instead
of the negative Z axis as our function above did. So, we need to 
subtract `target` from `eye` instead of `eye` from `target`

```rust
mod m4 {
  ...
+    #[rustfmt::skip]
+    pub fn aim(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
+        let mut dst = [0.0; 16];
+
+        let z_axis = vec3::normalize(vec3::subtract(target, eye));
+        let x_axis = vec3::normalize(vec3::cross(up, z_axis));
+        let y_axis = vec3::normalize(vec3::cross(z_axis, x_axis));
+
+        dst[ 0] = x_axis[0];  dst[ 1] = x_axis[1];  dst[ 2] = x_axis[2];  dst[ 3] = 0.0;
+        dst[ 4] = y_axis[0];  dst[ 5] = y_axis[1];  dst[ 6] = y_axis[2];  dst[ 7] = 0.0;
+        dst[ 8] = z_axis[0];  dst[ 9] = z_axis[1];  dst[10] = z_axis[2];  dst[11] = 0.0;
+        dst[12] = eye[0];     dst[13] = eye[1];     dst[14] = eye[2];     dst[15] = 1.0;
+
+        dst
+    }

    #[rustfmt::skip]
    pub fn camera_aim(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];

        let z_axis = vec3::normalize(vec3::subtract(eye, target));
        let x_axis = vec3::normalize(vec3::cross(up, z_axis));
        let y_axis = vec3::normalize(vec3::cross(z_axis, x_axis));

        dst[ 0] = x_axis[0];  dst[ 1] = x_axis[1];  dst[ 2] = x_axis[2];  dst[ 3] = 0.0;
        dst[ 4] = y_axis[0];  dst[ 5] = y_axis[1];  dst[ 6] = y_axis[2];  dst[ 7] = 0.0;
        dst[ 8] = z_axis[0];  dst[ 9] = z_axis[1];  dst[10] = z_axis[2];  dst[11] = 0.0;
        dst[12] = eye[0];     dst[13] = eye[1];     dst[14] = eye[2];     dst[15] = 1.0;

        dst
    }
  ...
```

<a id="a-aim-fs"></a> Let's make a bunch of Fs point at another F (yea, too many Fs but I don't want to clutter
the example with more data). We'll make a grid of 5x5 Fs + 1 more
for them to "aim" at

```rust
-    const NUM_FS: usize = 5;
+    const NUM_FS: usize = 5 * 5 + 1;
```

Then we'll hard code a camera target and change the
settings so we can move one of the Fs. On the example page's
JavaScript side

```js
  const settings = {
-    fieldOfView: degToRad(100),
-    cameraAngle: 0,
+    target: [0, 200, 300],
+    targetAngle: 0,
  };

  const radToDegOptions = { min: -360, max: 360, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
-  gui.add(settings, 'fieldOfView', {min: 1, max: 179, converters: GUI.converters.radToDeg})
-     .onChange(v => wasm.set_setting_num('fieldOfView', v));
-  gui.add(settings, 'cameraAngle', radToDegOptions)
-     .onChange(v => wasm.set_setting_num('cameraAngle', v));
+  gui.add(settings.target, '1', -100, 300).name('target height')
+     .onChange(v => wasm.set_setting_num('targetHeight', v));
+  gui.add(settings, 'targetAngle', radToDegOptions).name('target angle')
+     .onChange(v => wasm.set_setting_num('targetAngle', v));
```

And finally for the first 25 Fs we'll orient them in
a grid using `aim` and *aim* them at the 26th F

```rust
+        let mut settings_target = [
+            0.0,
+            wgpu_fun::setting_f64("targetHeight", 200.0) as f32,
+            300.0,
+        ];
+        let target_angle = wgpu_fun::setting_f64("targetAngle", 0.0) as f32;
+
+        // update target X,Z based on angle
+        settings_target[0] = target_angle.cos() * radius;
+        settings_target[2] = target_angle.sin() * radius;

        let aspect = frame.width as f32 / frame.height as f32;
        let projection = m4::perspective(
-            field_of_view,
+            60.0f32.to_radians(), // fieldOfView,
            aspect,
            1.0,    // zNear
            2000.0, // zFar
        );

-        // Compute the position of the first F
-        let f_position = [radius, 0.0, 0.0];
-
-        // Use matrix math to compute a position on a circle where
-        // the camera is
-        let temp_matrix = m4::rotation_y(camera_angle);
-        let temp_matrix = m4::translate(&temp_matrix, [0.0, 0.0, radius * 1.5]);
-
-        // Get the camera's position from the matrix we computed
-        let eye = [temp_matrix[12], temp_matrix[13], temp_matrix[14]];
+        let eye = [-500.0, 300.0, -500.0];
+        let target = [0.0, -100.0, 0.0];
        let up = [0.0, 1.0, 0.0];

        // Compute a view matrix
-        let view_matrix = m4::look_at(eye, f_position, up);
+        let view_matrix = m4::look_at(eye, target, up);

        // combine the view and projection matrixes
        let view_projection_matrix = m4::multiply(&projection, &view_matrix);

        for (i, object_info) in object_infos.iter_mut().enumerate() {
-            let angle = i as f32 / NUM_FS as f32 * std::f32::consts::PI * 2.0;
-            let x = angle.cos() * radius;
-            let z = angle.sin() * radius;
-
-            let matrix_value = m4::translate(&view_projection_matrix, [x, 0.0, z]);
+            let deep = 5;
+            let across = 5;
+            let matrix_value = if i < 25 {
+                // compute grid positions
+                let grid_x = i % across;
+                let grid_z = i / across;
+
+                // compute 0 to 1 positions
+                let u = grid_x as f32 / (across - 1) as f32;
+                let v = grid_z as f32 / (deep - 1) as f32;
+
+                // center and spread out
+                let x = (u - 0.5) * across as f32 * 150.0;
+                let z = (v - 0.5) * deep as f32 * 150.0;
+
+                // aim this F from it's position toward the target F
+                let aim_matrix = m4::aim([x, 0.0, z], settings_target, up);
+                m4::multiply(&view_projection_matrix, &aim_matrix)
+            } else {
+                m4::translate(&view_projection_matrix, settings_target)
+            };
            object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
                .copy_from_slice(&matrix_value);

            // upload the uniform values to the uniform buffer
            frame.queue.write_buffer(
                &object_info.uniform_buffer,
                0,
                bytemuck::cast_slice(&object_info.uniform_values),
            );
```

And now 25 Fs are facing (their front is positive Z), the 26th F

{{{example url="../webgpu-cameras-step-4-aim-Fs.html" }}}

Move the sliders and see all 25Fs *aim*.


<!-- keep this at the bottom of the article -->
<link href="webgpu-cameras.css" rel="stylesheet">
<script type="module" src="webgpu-cameras.js"></script>
