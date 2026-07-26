Title: WebGPU Orthographic Projection
Description: Orthographic Projection (no perspective)
TOC: Orthographic Projection

This article is the 5th in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

1. [Translation](webgpu-translation.html)
2. [Rotation](webgpu-rotation.html)
3. [Scaling](webgpu-scale.html)
4. [Matrix Math](webgpu-matrix-math.html)
5. [Orthographic Projection](webgpu-orthographic-projection.html) ⬅ you are here
6. [Perspective Projection](webgpu-perspective-projection.html)
7. [Cameras](webgpu-cameras.html)
8. [Matrix Stacks](webgpu-matrix-stacks.html)
9. [Scene Graphs](webgpu-scene-graphs.html)

In the last post we went over how matrices work. We talked
about how translation, rotation, scaling, and even projecting from
pixels into clip space can all be done by 1 matrix and some magic
matrix math. To do 3D is only a small step from there.

In our previous 2D examples we had 2D points (x, y) that we multiplied by
a 3x3 matrix. To do 3D we need 3D points (x, y, z) and a 4x4 matrix.

Let's take our last example and change it to 3D. We'll use an F again
but this time a 3D 'F'.

The first thing we need to do is change the vertex shader to handle 3D.
Here's the old vertex shader.

```wgsl
struct Uniforms {
  color: vec4f,
-  matrix: mat3x3f,
+  matrix: mat4x4f,
};

struct Vertex {
-  @location(0) position: vec2f,
+  @location(0) position: vec4f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
-
-  let clipSpace = (uni.matrix * vec3f(vert.position, 1)).xy;
-  vsOut.position = vec4f(clipSpace, 0.0, 1.0);
  vsOut.position = uni.matrix * vert.position;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return uni.color;
}
```

It got even simpler! Just like in 2D we provided `x` and `y` and then
set `z` to 1, in 3D we will provide `x`, `y`, and `z` and we need `w`
to be 1 but we can take advantage of the fact that for attributes
`w` defaults to 1.

Then we need to provide 3D data.

```rust
#[rustfmt::skip]
fn create_f_vertices() -> (Vec<f32>, Vec<u32>, u32) {
    let vertex_data: Vec<f32> = vec![
        // left column
*        0.0, 0.0, 0.0,
*        30.0, 0.0, 0.0,
*        0.0, 150.0, 0.0,
*        30.0, 150.0, 0.0,

        // top rung
*        30.0, 0.0, 0.0,
*        100.0, 0.0, 0.0,
*        30.0, 30.0, 0.0,
*        100.0, 30.0, 0.0,

        // middle rung
*        30.0, 60.0, 0.0,
*        70.0, 60.0, 0.0,
*        30.0, 90.0, 0.0,
*        70.0, 90.0, 0.0,
    ];

    let index_data: Vec<u32> = vec![
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top run
        8,  9, 10,   10,  9, 11,  // middle run
    ];

    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}
```

Above we just added a ` 0.0,` to the end of each line

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("2 attributes"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (2) * 4, // (2) floats, 4 bytes each
+        array_stride: (3) * 4, // (3) floats, 4 bytes each
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
-            format: wgpu::VertexFormat::Float32x2,
+            format: wgpu::VertexFormat::Float32x3,
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
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

Next we need to change all the matrix math from 2D to 3D

<div class="webgpu_center local-compare" style="align-items: end;">
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">1</td>
          <td class="m12">0</td>
          <td class="m13">tx</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">1</td>
          <td class="m23">ty</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">1</td>
        </tr>
      </table>
    </div>
    <div>2D translation matrix</div>
  </div>
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">1</td>
          <td class="m12">0</td>
          <td class="m13">0</td>
          <td class="m14">tx</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">1</td>
          <td class="m23">0</td>
          <td class="m24">ty</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">1</td>
          <td class="m34">tz</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </table>
    </div>
    <div>3D translation matrix</div>
  </div>
</div>

<div class="webgpu_center local-compare" style="align-items: end;">
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">c</td>
          <td class="m12">-s</td>
          <td class="m13">0</td>
        </tr>
        <tr>
          <td class="m21">s</td>
          <td class="m22">c</td>
          <td class="m23">0</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">1</td>
        </tr>
      </table>
    </div>
    <div>2D rotation matrix</div>
  </div>
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">c</td>
          <td class="m12">-s</td>
          <td class="m13">0</td>
          <td class="m14">0</td>
        </tr>
        <tr>
          <td class="m21">s</td>
          <td class="m22">c</td>
          <td class="m23">0</td>
          <td class="m24">0</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">1</td>
          <td class="m34">0</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </table>
    </div>
    <div>3D rotation Z matrix</div>
  </div>
</div>

<div class="webgpu_center local-compare" style="align-items: end;">
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">sx</td>
          <td class="m12">0</td>
          <td class="m13">0</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">sy</td>
          <td class="m23">0</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">1</td>
        </tr>
      </table>
    </div>
    <div>2D scaling matrix</div>
  </div>
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">sx</td>
          <td class="m12">0</td>
          <td class="m13">0</td>
          <td class="m14">0</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">sy</td>
          <td class="m23">0</td>
          <td class="m24">0</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">0</td>
          <td class="m33">sz</td>
          <td class="m34">0</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </table>
    </div>
    <div>3D scaling matrix</div>
  </div>
</div>

We can also make X and Y rotation matrices

<div class="webgpu_center local-compare" style="align-items: end;">
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">1</td>
          <td class="m12">0</td>
          <td class="m13">0</td>
          <td class="m14">0</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">c</td>
          <td class="m23">-s</td>
          <td class="m24">0</td>
        </tr>
        <tr>
          <td class="m31">0</td>
          <td class="m32">s</td>
          <td class="m33">c</td>
          <td class="m34">0</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </table>
    </div>
    <div>3D rotation X matrix</div>
  </div>
  <div>
    <div class="glocal-center">
      <table class="glocal-center-content glocal-mat">
        <tr>
          <td class="m11">c</td>
          <td class="m12">0</td>
          <td class="m13">s</td>
          <td class="m14">0</td>
        </tr>
        <tr>
          <td class="m21">0</td>
          <td class="m22">1</td>
          <td class="m23">0</td>
          <td class="m24">0</td>
        </tr>
        <tr>
          <td class="m31">-s</td>
          <td class="m32">0</td>
          <td class="m33">c</td>
          <td class="m34">0</td>
        </tr>
        <tr>
          <td class="m41">0</td>
          <td class="m42">0</td>
          <td class="m43">0</td>
          <td class="m44">1</td>
        </tr>
      </table>
    </div>
    <div>3D rotation Y matrix</div>
  </div>
</div>

We now have 3 rotation matrices.  We only needed one in 2D as we
were effectively only rotating around the Z axis.  Now though, to do 3D we
also want to be able to rotate around the X axis and Y axis as well.  You
can see from looking at them they are all very similar.  If we were to
work them out you'd see them simplify just like before

Z rotation

<div class="webgpu_center"><pre class="webgpu_math">
newX = x * c + y * -s;
newY = x * s + y *  c;
</pre></div>

Y rotation

<div class="webgpu_center"><pre class="webgpu_math">
newX = x *  c + z * s;
newZ = x * -s + z * c;
</pre></div>

X rotation

<div class="webgpu_center"><pre class="webgpu_math">
newY = y * c + z * -s;
newZ = y * s + z *  c;
</pre></div>

which gives you these rotations.

<iframe class="external_diagram" src="resources/axis-diagram.html" style="width: 540px; height: 280px;"></iframe>

Here's the 2D (before) versions of `m3::translation` and `m3::rotation` and `m3::scaling`

```rust
mod m3 {
    ...
    #[rustfmt::skip]
    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 12] {
        let mut dst = [0.0; 12];
        dst[0] = 1.0;  dst[1] = 0.0;  dst[2] = 0.0;
        dst[4] = 0.0;  dst[5] = 1.0;  dst[6] = 0.0;
        dst[8] = tx;   dst[9] = ty;   dst[10] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation(angle_in_radians: f32) -> [f32; 12] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 12];
        dst[0] = c;    dst[1] = s;   dst[2] = 0.0;
        dst[4] = -s;   dst[5] = c;   dst[6] = 0.0;
        dst[8] = 0.0;  dst[9] = 0.0; dst[10] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn scaling([sx, sy]: [f32; 2]) -> [f32; 12] {
        let mut dst = [0.0; 12];
        dst[0] = sx;   dst[1] = 0.0;  dst[2] = 0.0;
        dst[4] = 0.0;  dst[5] = sy;   dst[6] = 0.0;
        dst[8] = 0.0;  dst[9] = 0.0;  dst[10] = 1.0;
        dst
    }
    ...
```

And here are the updated 3D versions

```rust
mod m4 {
    ...
    #[rustfmt::skip]
    pub fn translation([tx, ty, tz]: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        dst[ 0] = 1.0;  dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = 1.0;  dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = 1.0;  dst[11] = 0.0;
        dst[12] = tx;   dst[13] = ty;   dst[14] = tz;   dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_x(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = 1.0;  dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = c;    dst[ 6] = s;    dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = -s;   dst[10] = c;    dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_y(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = c;    dst[ 1] = 0.0;  dst[ 2] = -s;   dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = 1.0;  dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = s;    dst[ 9] = 0.0;  dst[10] = c;    dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn rotation_z(angle_in_radians: f32) -> [f32; 16] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        let mut dst = [0.0; 16];
        dst[ 0] = c;    dst[ 1] = s;    dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = -s;   dst[ 5] = c;    dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = 1.0;  dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }

    #[rustfmt::skip]
    pub fn scaling([sx, sy, sz]: [f32; 3]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        dst[ 0] = sx;   dst[ 1] = 0.0;  dst[ 2] = 0.0;  dst[ 3] = 0.0;
        dst[ 4] = 0.0;  dst[ 5] = sy;   dst[ 6] = 0.0;  dst[ 7] = 0.0;
        dst[ 8] = 0.0;  dst[ 9] = 0.0;  dst[10] = sz;   dst[11] = 0.0;
        dst[12] = 0.0;  dst[13] = 0.0;  dst[14] = 0.0;  dst[15] = 1.0;
        dst
    }
    ...
```

(In JavaScript these functions take an optional `dst` Float32Array to write
into; in Rust we just return a `[f32; 16]` array.)

Similarly we'll make our simplified functions. Here's the 2D ones.

```rust
    pub fn translate(m: &[f32; 12], translation: [f32; 2]) -> [f32; 12] {
        multiply(m, &self::translation(translation))
    }

    pub fn rotate(m: &[f32; 12], angle_in_radians: f32) -> [f32; 12] {
        multiply(m, &rotation(angle_in_radians))
    }

    pub fn scale(m: &[f32; 12], scale: [f32; 2]) -> [f32; 12] {
        multiply(m, &scaling(scale))
    }
```

And now the 3D ones. Not much has changed except naming them `m4` and adding
the 2 more rotation functions.

```rust
    pub fn translate(m: &[f32; 16], translation: [f32; 3]) -> [f32; 16] {
        multiply(m, &self::translation(translation))
    }

    pub fn rotate_x(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_x(angle_in_radians))
    }

    pub fn rotate_y(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_y(angle_in_radians))
    }

    pub fn rotate_z(m: &[f32; 16], angle_in_radians: f32) -> [f32; 16] {
        multiply(m, &rotation_z(angle_in_radians))
    }

    pub fn scale(m: &[f32; 16], scale: [f32; 3]) -> [f32; 16] {
        multiply(m, &scaling(scale))
    }
    ...
```

And we need a 4x4 matrix multiplication function

```rust
    pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut dst = [0.0; 16];
        let b00 = b[0 * 4 + 0];
        let b01 = b[0 * 4 + 1];
        let b02 = b[0 * 4 + 2];
        let b03 = b[0 * 4 + 3];
        let b10 = b[1 * 4 + 0];
        let b11 = b[1 * 4 + 1];
        let b12 = b[1 * 4 + 2];
        let b13 = b[1 * 4 + 3];
        let b20 = b[2 * 4 + 0];
        let b21 = b[2 * 4 + 1];
        let b22 = b[2 * 4 + 2];
        let b23 = b[2 * 4 + 3];
        let b30 = b[3 * 4 + 0];
        let b31 = b[3 * 4 + 1];
        let b32 = b[3 * 4 + 2];
        let b33 = b[3 * 4 + 3];
        let a00 = a[0 * 4 + 0];
        let a01 = a[0 * 4 + 1];
        let a02 = a[0 * 4 + 2];
        let a03 = a[0 * 4 + 3];
        let a10 = a[1 * 4 + 0];
        let a11 = a[1 * 4 + 1];
        let a12 = a[1 * 4 + 2];
        let a13 = a[1 * 4 + 3];
        let a20 = a[2 * 4 + 0];
        let a21 = a[2 * 4 + 1];
        let a22 = a[2 * 4 + 2];
        let a23 = a[2 * 4 + 3];
        let a30 = a[3 * 4 + 0];
        let a31 = a[3 * 4 + 1];
        let a32 = a[3 * 4 + 2];
        let a33 = a[3 * 4 + 3];

        dst[0] = b00 * a00 + b01 * a10 + b02 * a20 + b03 * a30;
        dst[1] = b00 * a01 + b01 * a11 + b02 * a21 + b03 * a31;
        dst[2] = b00 * a02 + b01 * a12 + b02 * a22 + b03 * a32;
        dst[3] = b00 * a03 + b01 * a13 + b02 * a23 + b03 * a33;

        dst[4] = b10 * a00 + b11 * a10 + b12 * a20 + b13 * a30;
        dst[5] = b10 * a01 + b11 * a11 + b12 * a21 + b13 * a31;
        dst[6] = b10 * a02 + b11 * a12 + b12 * a22 + b13 * a32;
        dst[7] = b10 * a03 + b11 * a13 + b12 * a23 + b13 * a33;

        dst[8] = b20 * a00 + b21 * a10 + b22 * a20 + b23 * a30;
        dst[9] = b20 * a01 + b21 * a11 + b22 * a21 + b23 * a31;
        dst[10] = b20 * a02 + b21 * a12 + b22 * a22 + b23 * a32;
        dst[11] = b20 * a03 + b21 * a13 + b22 * a23 + b23 * a33;

        dst[12] = b30 * a00 + b31 * a10 + b32 * a20 + b33 * a30;
        dst[13] = b30 * a01 + b31 * a11 + b32 * a21 + b33 * a31;
        dst[14] = b30 * a02 + b31 * a12 + b32 * a22 + b33 * a32;
        dst[15] = b30 * a03 + b31 * a13 + b32 * a23 + b33 * a33;

        dst
    }
```

We also need to update the projection function. Here's the old one

```rust
    #[rustfmt::skip]
    pub fn projection(width: f32, height: f32) -> [f32; 12] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        let mut dst = [0.0; 12];
        dst[0] = 2.0 / width;  dst[1] = 0.0;            dst[2] = 0.0;
        dst[4] = 0.0;          dst[5] = -2.0 / height;  dst[6] = 0.0;
        dst[8] = -1.0;         dst[9] = 1.0;            dst[10] = 1.0;
        dst
    }
```

which converted from pixels to clip space. For our first attempt at
expanding it to 3D let's try


```rust
    #[rustfmt::skip]
    pub fn projection(width: f32, height: f32, depth: f32) -> [f32; 16] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        let mut dst = [0.0; 16];
        dst[ 0] = 2.0 / width;  dst[ 1] = 0.0;            dst[ 2] = 0.0;           dst[ 3] = 0.0;
        dst[ 4] = 0.0;          dst[ 5] = -2.0 / height;  dst[ 6] = 0.0;           dst[ 7] = 0.0;
        dst[ 8] = 0.0;          dst[ 9] = 0.0;            dst[10] = 0.5 / depth;   dst[11] = 0.0;
        dst[12] = -1.0;         dst[13] = 1.0;            dst[14] = 0.5;           dst[15] = 1.0;
        dst
    }
```

Just like we needed to convert from pixels to clip space for X and Y, for
Z we need to do the same thing.  In this case we making the Z axis "pixel
units" as well?. We'll pass in some value similar to `width` for the `depth`
so our space will be 0 to `width` pixels wide, 0 to `height` pixels tall, but
for `depth` it will be `-depth / 2` to `+depth / 2`.

We need to provide a 4x4 matrix in our uniforms

```rust
  // color, matrix
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 12) * 4;
+  const UNIFORM_BUFFER_SIZE: u64 = (4 + 16) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_MATRIX_OFFSET: usize = 4;
```

(The matrix now takes 16 floats starting at `K_MATRIX_OFFSET` instead of 12.)

And we need to to update the code that computes the matrix. The settings
live in the example page's JavaScript, where the GUI pushes them into the
wasm module.

```js
 const settings = {
-    translation: [150, 100],
-    rotation: degToRad(30),
-    scale: [1, 1],
+    translation: [45, 100, 0],
+    rotation: [degToRad(40), degToRad(25), degToRad(325)],
+    scale: [1, 1, 1],
  };
```

and in the render code we read them and compute a 4x4 matrix

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

    let translation = [
        wgpu_fun::setting_f64("translationX", 45.0) as f32,
        wgpu_fun::setting_f64("translationY", 100.0) as f32,
+        wgpu_fun::setting_f64("translationZ", 0.0) as f32,
    ];
-    let rotation = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
+    let rotation = [
+        wgpu_fun::setting_f64("rotationX", 40.0f64.to_radians()) as f32,
+        wgpu_fun::setting_f64("rotationY", 25.0f64.to_radians()) as f32,
+        wgpu_fun::setting_f64("rotationZ", 325.0f64.to_radians()) as f32,
+    ];
    let scale = [
        wgpu_fun::setting_f64("scaleX", 1.0) as f32,
        wgpu_fun::setting_f64("scaleY", 1.0) as f32,
+        wgpu_fun::setting_f64("scaleZ", 1.0) as f32,
    ];

-    let mut matrix_value = m3::projection(frame.width as f32, frame.height as f32);
-    matrix_value = m3::translate(&matrix_value, translation);
-    matrix_value = m3::rotate(&matrix_value, rotation);
-    matrix_value = m3::scale(&matrix_value, scale);
+    let mut matrix_value = m4::projection(frame.width as f32, frame.height as f32, 400.0);
+    matrix_value = m4::translate(&matrix_value, translation);
+    matrix_value = m4::rotate_x(&matrix_value, rotation[0]);
+    matrix_value = m4::rotate_y(&matrix_value, rotation[1]);
+    matrix_value = m4::rotate_z(&matrix_value, rotation[2]);
+    matrix_value = m4::scale(&matrix_value, scale);
+    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16].copy_from_slice(&matrix_value);
```

{{{example url="../webgpu-orthographic-projection-step-1-flat-f.html"}}}

The first problem we have is that our data is a flat F which makes it
hard to see any 3D.  To fix that let's expand the data to 3D.  Our
current F is made of 3 rectangles, 2 triangles each.  To make it 3D will
require a total of 16 rectangles.  The 3 rectangles on the front, 3 on the
back, 1 on the left, 4 on the right, 2 on the tops, 3 on the bottoms.

<img class="webgpu_center noinvertdark" style="width: 400px;" src="resources/3df.svg" />

We just need to take all of our current vertex positions and duplicate them
but move them in Z. Then connect them all with indices

```rust
#[rustfmt::skip]
fn create_f_vertices() -> (Vec<f32>, Vec<u32>, u32) {
    let vertex_data: Vec<f32> = vec![
        // left column
        0.0, 0.0, 0.0,
        30.0, 0.0, 0.0,
        0.0, 150.0, 0.0,
        30.0, 150.0, 0.0,

        // top rung
        30.0, 0.0, 0.0,
        100.0, 0.0, 0.0,
        30.0, 30.0, 0.0,
        100.0, 30.0, 0.0,

        // middle rung
        30.0, 60.0, 0.0,
        70.0, 60.0, 0.0,
        30.0, 90.0, 0.0,
        70.0, 90.0, 0.0,

+        // left column back
+        0.0, 0.0, 30.0,
+        30.0, 0.0, 30.0,
+        0.0, 150.0, 30.0,
+        30.0, 150.0, 30.0,
+
+        // top rung back
+        30.0, 0.0, 30.0,
+        100.0, 0.0, 30.0,
+        30.0, 30.0, 30.0,
+        100.0, 30.0, 30.0,
+
+        // middle rung back
+        30.0, 60.0, 30.0,
+        70.0, 60.0, 30.0,
+        30.0, 90.0, 30.0,
+        70.0, 90.0, 30.0,
    ];

    let index_data: Vec<u32> = vec![
+        // front
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top run
        8,  9, 10,   10,  9, 11,  // middle run

+        // back
+        12,  13,  14,   14, 13, 15,  // left column back
+        16,  17,  18,   18, 17, 19,  // top run back
+        20,  21,  22,   22, 21, 23,  // middle run back
+
+        0, 5, 12,   12, 5, 17,   // top
+        5, 7, 17,   17, 7, 19,   // top rung right
+        6, 7, 18,   18, 7, 19,   // top rung bottom
+        6, 8, 18,   18, 8, 20,   // between top and middle rung
+        8, 9, 20,   20, 9, 21,   // middle rung top
+        9, 11, 21,  21, 11, 23,  // middle rung right
+        10, 11, 22, 22, 11, 23,  // middle rung bottom
+        10, 3, 22,  22, 3, 15,   // stem right
+        2, 3, 14,   14, 3, 15,   // bottom
+        0, 2, 12,   12, 2, 14,   // left
    ];

    let num_vertices = index_data.len() as u32;
    (vertex_data, index_data, num_vertices)
}
```

And here's that version

{{{example url="../webgpu-orthographic-projection-step-2-3d-f.html"}}}

Moving the sliders it's pretty hard to tell that it's 3D.  Let's try
coloring each rectangle a different color.  To do this we will add another
attribute to our vertex shader and pass it from the vertex
shader to the fragment shader via an [inter-stage variable](webgpu-inter-stage-variables.html).

First we update the shader

```wgsl
struct Uniforms {
-  color: vec4f,
  matrix: mat4x4f,
};

struct Vertex {
  @location(0) position: vec4f,
+  @location(1) color: vec4f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
+  @location(0) color: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = uni.matrix * vert.position;
+  vsOut.color = vert.color;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
-  return uni.color;
+  return vsOut.color;
}
```

We need to add colors do our vertex data but there's a problem.
Currently we are using indices in order to share vertices. But, if
we want to draw each face a different color, those vertices can not be
shared because they only get 1 color each.

<img src="resources/cube-faces-vertex-no-texture.svg" class="webgpu_center" style="width:400px;" />

The corner vertex in above needs to be used once for each of the 3 faces
it shares but each time it needs a different color so using indices is problematic.
[^flat-interpolation]

[^flat-interpolation]: it's possible with creative arrangement of
the indices we could use `@interpolate(flat)` as mentioned in
[the article on inter-stage variables](webgpu-inter-stage-varaibles.html#a-interpolate)
and still use indices.

So, let's expand our data from indexed to non-index and while we're at
it we'll add vertex colors so that each part of the F gets a different
color.

```rust
#[rustfmt::skip]
-fn create_f_vertices() -> (Vec<f32>, Vec<u32>, u32) {
+fn create_f_vertices() -> (Vec<f32>, u32) {
-    let vertex_data: Vec<f32> = vec![
+    let positions: Vec<f32> = vec![
        // left column
        0.0, 0.0, 0.0,
        30.0, 0.0, 0.0,
        0.0, 150.0, 0.0,
        30.0, 150.0, 0.0,

        // top rung
        30.0, 0.0, 0.0,
        100.0, 0.0, 0.0,
        30.0, 30.0, 0.0,
        100.0, 30.0, 0.0,

        // middle rung
        30.0, 60.0, 0.0,
        70.0, 60.0, 0.0,
        30.0, 90.0, 0.0,
        70.0, 90.0, 0.0,

        // left column back
        0.0, 0.0, 30.0,
        30.0, 0.0, 30.0,
        0.0, 150.0, 30.0,
        30.0, 150.0, 30.0,

        // top rung back
        30.0, 0.0, 30.0,
        100.0, 0.0, 30.0,
        30.0, 30.0, 30.0,
        100.0, 30.0, 30.0,

        // middle rung back
        30.0, 60.0, 30.0,
        70.0, 60.0, 30.0,
        30.0, 90.0, 30.0,
        70.0, 90.0, 30.0,
    ];

-    let index_data: Vec<u32> = vec![
+    let indices: Vec<u32> = vec![
        // front
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top rung
        8,  9, 10,   10,  9, 11,  // middle rung

        // back
        12,  13,  14,   14, 13, 15,  // left column back
        16,  17,  18,   18, 17, 19,  // top rung back
        20,  21,  22,   22, 21, 23,  // middle rung back

        0, 5, 12,   12, 5, 17,   // top
        5, 7, 17,   17, 7, 19,   // top rung right
        6, 7, 18,   18, 7, 19,   // top rung bottom
        6, 8, 18,   18, 8, 20,   // between top and middle rung
        8, 9, 20,   20, 9, 21,   // middle rung top
        9, 11, 21,  21, 11, 23,  // middle rung right
        10, 11, 22, 22, 11, 23,  // middle rung bottom
        10, 3, 22,  22, 3, 15,   // stem right
        2, 3, 14,   14, 3, 15,   // bottom
        0, 2, 12,   12, 2, 14,   // left
    ];

+    let quad_colors: Vec<u8> = vec![
+        200,  70, 120,  // left column front
+        200,  70, 120,  // top rung front
+        200,  70, 120,  // middle rung front
+
+         80,  70, 200,  // left column back
+         80,  70, 200,  // top rung back
+         80,  70, 200,  // middle rung back
+
+         70, 200, 210,  // top
+        160, 160, 220,  // top rung right
+         90, 130, 110,  // top rung bottom
+        200, 200,  70,  // between top and middle rung
+        210, 100,  70,  // middle rung top
+        210, 160,  70,  // middle rung right
+         70, 180, 210,  // middle rung bottom
+        100,  70, 210,  // stem right
+         76, 210, 100,  // bottom
+        140, 210,  80,  // left
+    ];
+
+    let num_vertices = indices.len() as u32;
+    let mut vertex_data = vec![0.0f32; indices.len() * 4]; // xyz + color
+    for (i, index) in indices.iter().enumerate() {
+        let position_ndx = (index * 3) as usize;
+        let position = &positions[position_ndx..position_ndx + 3];
+        vertex_data[i * 4..i * 4 + 3].copy_from_slice(position);
+
+        let quad_ndx = (i / 6) * 3;
+        let color = &quad_colors[quad_ndx..quad_ndx + 3];
+        // set RGB in the first 3 bytes of the 4th float, set A to 255
+        vertex_data[i * 4 + 3] = f32::from_ne_bytes([color[0], color[1], color[2], 255]);
+    }

-    let num_vertices = index_data.len() as u32;
-    (vertex_data, index_data, num_vertices)
+    (vertex_data, num_vertices)
}
```

We walk each index, get the position for that index and put the position values
in `vertex_data`. The colors are 4 unsigned bytes but our vertex data is `f32`s,
so, where JavaScript would make a `Uint8Array` view *on the same data*, in Rust
we pack the 4 color bytes into the bits of the 4th float with `f32::from_ne_bytes`.
We pull out the colors by quad index (one quad every 6 vertices)
and insert the same color for each vertex of that quad. The data will end up like this.

<img class="webgpu_center" style="background-color: transparent; width: 1024px;" src="resources/vertex-buffer-f32x3-u8x4.svg" />

The colors we added are unsigned bytes with values from 0 to 255, similar to
[a css `rgb()` color](https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/rgb).
By setting the attribute type in the pipeline to `Unorm8x4` (unsigned normalized 8 bit value x 4),
the GPU will pull the values out of the buffer and *normalize* them when supplying them to the
shader. This which means it will make them go from 0 to 1, in this case by dividing them by 255.

Now that we have the data, we need to change our pipeline to use it.

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("2 attributes"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[Some(wgpu::VertexBufferLayout {
-        array_stride: (3) * 4, // (3) floats, 4 bytes each
+        array_stride: (4) * 4, // (3) floats 4 bytes each + one 4 byte color
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
          // position
          wgpu::VertexAttribute {
            shader_location: 0,
            offset: 0,
            format: wgpu::VertexFormat::Float32x3,
          },
+          // color
+          wgpu::VertexAttribute {
+            shader_location: 1,
+            offset: 12,
+            format: wgpu::VertexFormat::Unorm8x4,
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
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

We need to remove the old color data from our uniform.

```rust
-  // color, matrix
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 16) * 4;
+  // matrix
+  const UNIFORM_BUFFER_SIZE: u64 = (16) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
-  const K_COLOR_OFFSET: usize = 0;
-  const K_MATRIX_OFFSET: usize = 4;
+  const K_MATRIX_OFFSET: usize = 0;

-  // The color will not change so let's set it once at init time
-  uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
-    rand(0.0, 1.0),
-    rand(0.0, 1.0),
-    rand(0.0, 1.0),
-    1.0,
-  ]);
```

We no longer need to make an index buffer.

```rust
-  let (vertex_data, index_data, num_vertices) = create_f_vertices();
+  let (vertex_data, num_vertices) = create_f_vertices();
  let vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("vertex buffer vertices"),
    size: (vertex_data.len() * 4) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  app.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
-  let index_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-    label: Some("index buffer"),
-    size: (index_data.len() * 4) as u64,
-    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
-    mapped_at_creation: false,
-  });
-  app.queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&index_data));
```

and we need to draw without indices

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...
    pass.set_pipeline(&pipeline);
    pass.set_vertex_buffer(0, vertex_buffer.slice(..));
-    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);

    ...

    pass.set_bind_group(0, &bind_group, &[]);
-    pass.draw_indexed(0..num_vertices, 0, 0..1);
+    pass.draw(0..num_vertices, 0..1);

    ...
  });
```

Now we get this.

{{{example url="../webgpu-orthographic-projection-step-3-colored-3d-f.html"}}}

Uh oh, what's that mess?  Well, it turns out all the various parts of
that 3D 'F', front, back, sides, etc get drawn in the order they appear in
our geometry data.  That doesn't give us quite the desired results as sometimes
the ones in the back get drawn after the ones in the front.

<img class="webgpu_center" style="background-color: transparent; width: 163px;" src="resources/polygon-drawing-order.gif" />

The <span style="background: rgb(200, 70, 120); color: white; padding: 0.25em">reddish part</span> is
the **front** of the 'F'  but because it's the first part of our data
it is drawn first and then the other triangles behind it get drawn
after, covering it up. For example the  <span style="background: rgb(80, 70, 200); color: white; padding: 0.25em">purple part</span>
is actually the back of the 'F'. It gets drawn 2nd because it comes 2nd in our data.

Triangles in WebGPU have the concept of front facing and back facing.  By default a
front facing triangle has its vertices go in a counter clockwise direction in clip space.  A
back facing triangle has its vertices go in a clockwise direction in clip space.

<img src="resources/triangle-winding.svg" class="webgpu_center" style="width: 400px;" />

The gpu has the ability to draw only forward facing or only back facing
triangles.  We can turn that feature on by modifying the pipeline

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
-    primitive: Default::default(),
+    primitive: wgpu::PrimitiveState {
+      cull_mode: Some(wgpu::Face::Back),
+      ..Default::default()
+    },
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

With `cull_mode` set to `Back`, "back facing" triangles will be culled.
"Culling" in this case is a fancy word for "not drawing".
So, with `cull_mode` set to `Some(wgpu::Face::Back)`, this is what we get

{{{example url="../webgpu-orthographic-projection-step-4-cullmode-back.html"}}}

Hey!  Where did all the triangles go?  It turns out, many of them are
facing the wrong way.  Rotate it and you'll see them appear when you look
at the other side.  Fortunately it's easy to fix.  We just look at which
ones are backward and exchange 2 of their vertices.  For example if one
backward triangle has indices

<div class="webgpu_center"><pre class="webgpu_math">
6, 7, 8,
</pre></div>

We can just swap two of them to make them go the other way

<div class="webgpu_center"><pre class="webgpu_math">
6, 8, 7,
</pre></div>

Importantly, as far as WebGPU is concerned, whether or not a triangle is
considered to be going clockwise or counter clockwise depends on the
vertices of that triangle in clip space.  In other words, WebGPU figures out
whether a triangle is front or back AFTER you've applied math to the
vertices in the vertex shader.  That means for example, a clockwise
triangle that is scaled in X by -1 becomes a counter clockwise triangle or,
a clockwise triangle rotated 180 degrees becomes a counter clockwise
triangle.  Because we didn't set `cull_mode` before, we could see both
clockwise(front) and counter clockwise(back) facing triangles.  Now that we've
set `cull_mode` to `Back`,, any time a front facing triangle flips around, either because
of scaling or rotation or for whatever reason, WebGPU won't draw it.
That's a good thing since, as you turn something around in 3D, you
generally want whichever triangles are facing you to be considered front
facing.

BUT! Remember that in clip space +Y is at the top, but in our pixel space
+Y is at the bottom. In other words, our matrix is flipping all the
triangles vertically. This means that in order to draw things with +Y
at the bottom we either need to set `cull_mode` to `Front`, OR
flip all our triangles vertices. Let's set `cull_mode` to `Front`
and then also fix the vertex data so all the triangles have the same
direction.

```rust
    let indices: Vec<u32> = vec![
        // front
        0,  1,  2,    2,  1,  3,  // left column
        4,  5,  6,    6,  5,  7,  // top run
        8,  9, 10,   10,  9, 11,  // middle run

        // back
-        12,  13,  14,   14, 13, 15,  // left column back
+        12,  14,  13,   14, 15, 13,  // left column back
-        16,  17,  18,   18, 17, 19,  // top run back
+        16,  18,  17,   18, 19, 17,  // top run back
-        20,  21,  22,   22, 21, 23,  // middle run back
+        20,  22,  21,   22, 23, 21,  // middle run back

-        0, 5, 12,   12, 5, 17,   // top
+        0, 12, 5,   12, 17, 5,   // top
-        5, 7, 17,   17, 7, 19,   // top rung right
+        5, 17, 7,   17, 19, 7,   // top rung right
        6, 7, 18,   18, 7, 19,   // top rung bottom
-        6, 8, 18,   18, 8, 20,   // between top and middle rung
+        6, 18, 8,   18, 20, 8,   // between top and middle rung
-        8, 9, 20,   20, 9, 21,   // middle rung top
+        8, 20, 9,   20, 21, 9,   // middle rung top
-        9, 11, 21,  21, 11, 23,  // middle rung right
+        9, 21, 11,  21, 23, 11,  // middle rung right
        10, 11, 22, 22, 11, 23,  // middle rung bottom
-        10, 3, 22,  22, 3, 15,   // stem right
+        10, 22, 3,  22, 15, 3,   // stem right
        2, 3, 14,   14, 3, 15,   // bottom
        0, 2, 12,   12, 2, 14,   // left
    ];
```

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    ...
    primitive: wgpu::PrimitiveState {
-      cull_mode: Some(wgpu::Face::Back),
+      cull_mode: Some(wgpu::Face::Front),
      ..Default::default()
    },
    ...
  });
```

With those changes, making all the triangles face one direction gets us to this

{{{example url="../webgpu-orthographic-projection-step-5-order-fixed.html"}}}

That's closer but there's still one more problem.  Even with all the
triangles facing in the correct direction, and with the ones facing away from us
being culled, we still have places where triangles that should be in the back
are being drawn over triangles that should be in front.

## <a id="a-depth-textures"></a>Enter "Depth Textures"

A depth texture, sometimes called a depth-buffer or Z-Buffer, is a rectangle of *depth*
texels, one depth texel for each color texel in the texture we're drawing to.
If we create and bind a depth texture, then, as WebGPU draws each pixel it can also draw a depth pixel.  It does this
based on the values we return from the vertex shader for Z.  Just like we
had to convert to clip space for X and Y, Z is also in clip space. For
Z, clip space is 0 to +1.

Before WebGPU draws a color pixel it will check the corresponding depth
pixel.  If the depth (Z) value for the pixel it's about to draw does not match
some condition relative to the value of the corresponding depth pixel then WebGPU will not draw
the new color pixel. Otherwise it draws both the new color pixel with the
color from your fragment shader AND it draws the depth pixel with the new
depth value. This means, pixels that are behind other pixels won't get
drawn.

To setup and use a depth texture we need to update our pipeline

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
      cull_mode: Some(wgpu::Face::Front),
      ..Default::default()
    },
-    depth_stencil: None,
+    depth_stencil: Some(wgpu::DepthStencilState {
+      depth_write_enabled: Some(true),
+      depth_compare: Some(wgpu::CompareFunction::Less),
+      format: wgpu::TextureFormat::Depth24Plus,
+      stencil: Default::default(),
+      bias: Default::default(),
+    }),
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

Above we're setting `depth_compare` to `Less`. This means, only draw the new pixel,
if the Z value for the new pixel is "less" than the corresponding pixel in the depth
texture. Other options include `Never`, `Equal`, `LessEqual`, `Greater`, `NotEqual`,
`GreaterEqual`, `Always`.

`depth_write_enabled: Some(true)` means, if we pass the `depth_compare` test, then write
the Z value of our new pixel to the depth texture. In our case, this means
each time a pixel we're drawing has a Z value less than what's already in the depth
texture, we'll draw that pixel and update the depth texture. In this way, if we later try
to draw a pixel that's further back (has a higher Z value) it will not be drawn.

`format` is similar to the fragment target's `format`. It's the format of
the depth texture we will use. The available depth texture formats were listed
[in the article on textures](webgpu-textures.html#a-depth-stencil-formats).
`Depth24Plus` is a good default format to choose.

We also need to update our render pass descriptor so it has a depth stencil attachment.

```rust
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
+        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
+          view: &depth_view,
+          depth_ops: Some(wgpu::Operations {
+            load: wgpu::LoadOp::Clear(1.0),
+            store: wgpu::StoreOp::Store,
+          }),
+          stencil_ops: None,
+        }),
        ..Default::default()
      });
```

Depth values generally go from 0.0 to 1.0. We clear the depth to 1.
This makes sense since we set `depth_compare` to `Less`.

Finally, we need to create a depth texture. The catch is, it has to match the size the color attachments,
which in this case is the texture for the canvas. The canvas texture changes
size when the canvas is resized (wgpu_fun's `auto_resize` handles the resolution for us,
and `frame.width` / `frame.height` are whatever size the canvas currently is).
With that in mind, let's create the correct size texture
at render time.

```rust
+  let mut depth_texture: Option<wgpu::Texture> = None;

  app.run(RenderMode::Once, move |frame: &Frame| {
+    // If we don't have a depth texture OR if its size is different
+    // from the canvasTexture when make a new depth texture
+    if depth_texture
+        .as_ref()
+        .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
+    {
+      if let Some(texture) = depth_texture.take() {
+        texture.destroy();
+      }
+      depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
+        label: None,
+        size: wgpu::Extent3d {
+          width: frame.width,
+          height: frame.height,
+          depth_or_array_layers: 1,
+        },
+        format: wgpu::TextureFormat::Depth24Plus,
+        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
+        mip_level_count: 1,
+        sample_count: 1,
+        dimension: wgpu::TextureDimension::D2,
+        view_formats: &[],
+      }));
+    }
+    let depth_view = depth_texture
+        .as_ref()
+        .unwrap()
+        .create_view(&Default::default());

  ...
```

With the depth texture added we now get

{{{example url="../webgpu-orthographic-projection-step-6-depth-texture.html"}}}

Which is 3D!

## Ortho / Orthographic

One minor thing. In most 3D math libraries there is no `projection` function to
do our conversions from clip space to pixel space. Rather, there's usually a function
called `ortho` or `orthographic` that looks like this

```rust
mod m4 {
    ...
    pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
        let mut dst = [0.0; 16];

        dst[0] = 2.0 / (right - left);
        dst[1] = 0.0;
        dst[2] = 0.0;
        dst[3] = 0.0;

        dst[4] = 0.0;
        dst[5] = 2.0 / (top - bottom);
        dst[6] = 0.0;
        dst[7] = 0.0;

        dst[8] = 0.0;
        dst[9] = 0.0;
        dst[10] = 1.0 / (near - far);
        dst[11] = 0.0;

        dst[12] = (right + left) / (left - right);
        dst[13] = (top + bottom) / (bottom - top);
        dst[14] = near / (near - far);
        dst[15] = 1.0;

        dst
    }
    ...
```

Unlike our simplified `projection` function above, which only had width, height, and depth
parameters, with this more common orthographic projection function we can pass in left, right,
bottom, top, near, and far which gives as more flexibility. To use it the same as
our original projection function we'd call it with

```rust
-    let mut matrix_value = m4::projection(frame.width as f32, frame.height as f32, 400.0);
+    let mut matrix_value = m4::ortho(
+        0.0,                   // left
+        frame.width as f32,    // right
+        frame.height as f32,   // bottom
+        0.0,                   // top
+        400.0,                 // near
+        -400.0,                // far
+    );
```

{{{example url="../webgpu-orthographic-projection-step-7-ortho.html"}}}

Next we'll go over [how to make it have perspective](webgpu-perspective-projection.html).

<div class="webgpu_bottombar">
<h3>Why's it called orthographic projection</h3>
<p>
Orthographic in this case comes from the word, <i>orthogonal</i>
</p>
<blockquote>
<h2>orthogonal</h2>
<p><i>adjective</i>:</p>
<ol><li>of or involving right angles</li></ol>
</blockquote>
</div>

<!-- keep this at the bottom of the article -->
<link href="webgpu-orthographic-projection.css" rel="stylesheet">
<script type="module" src="webgpu-orthographic-projection.js"></script>
