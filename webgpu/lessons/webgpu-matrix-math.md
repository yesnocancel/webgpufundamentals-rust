Title: WebGPU Matrix Math
Description: Matrix Math Simplifies everything
TOC: Matrix Math

This article is the 4th in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="matrix-math.hanson"}}}

In the last 3 posts we went over how to [translate](webgpu-translation.html),
[rotate](webgpu-rotation.html), and [scale](webgpu-scale.html) vertex positions.
Translation, rotation and scale are each considered a type of *transformation*.
Each of these transformations required changes to the shader and each
of the 3 transformations was order dependent.

In [our previous example](webgpu-scale.html), we scaled, then rotated,
then translated. If we applied those in a different order we'd get a
different result.

For example here is a scale of 2, 1, rotation of 30 degrees,
and translation of 100, 0.

<img src="resources/f-scale-rotation-translation.svg" class="webgpu_center" width="400" />

And here is a translation of 100,0, rotation of 30 degrees and scale of 2, 1

<img src="resources/f-translation-rotation-scale.svg" class="webgpu_center" width="400" />

The results are completely different. Even worse, if we needed the
second example we'd have to write a different shader that applied
the translation, rotation, and scale in our new desired order.

Well, some smart people figured out a way that you can do
all the same stuff with matrix math. For 2D we use a 3x3 matrix.
A 3x3 matrix is like a grid with 9 boxes:

<div class="glocal-center">
  <table class="glocal-center-content glocal-mat">
    <tr>
      <td class="m11">1</td>
      <td class="m12">4</td>
      <td class="m13">7</td>
    </tr>
    <tr>
      <td class="m21">2</td>
      <td class="m22">5</td>
      <td class="m23">8</td>
    </tr>
    <tr>
      <td class="m31">3</td>
      <td class="m32">6</td>
      <td class="m33">9</td>
    </tr>
  </table>
</div>

To do the math we multiply the position across the rows of the matrix
and add up the results.

<div class="webgpu_center"><img src="resources/matrix-vector-math.svg" class="noinvertdark" style="width: 1000px;"></div>

Our positions only have 2 values, x and y, but
to do this math we need 3 values so we'll use 1 for the third value.

In this case our result would be

<div class="glocal-center">
  <p>newX = x * <span class="m11">1</span> + y * <span class="m12">4</span> + 1 * <span class="m13">7</span></p>
  <p>newY = x * <span class="m21">2</span> + y * <span class="m22">5</span> + 1 * <span class="m23">8</span></p>
  <p>newZ = x * <span class="m31">3</span> + y * <span class="m32">6</span> + 1 * <span class="m33">9</span></p>
</div>

You're probably looking at that and thinking "WHAT'S THE POINT?" Well,
let's assume we have a translation. We'll call the amount we want to
translate by tx and ty. Let's make a matrix like this

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

And now check it out

<div class="glocal-center">
  <div class="eq">
    <div>newX = x * <span class="m11">1</span> + y * <span class="m12">0</span> + 1 * <span class="m13">tx</span></div>
    <div>newY = x * <span class="m21">0</span> + y * <span class="m22">1</span> + 1 * <span class="m23">ty</span></div>
    <div>newZ = x * <span class="m31">0</span> + y * <span class="m32">0</span> + 1 * <span class="m33">1</span></div>
  </div>
</div>

If you remember your algebra, we can delete any place that multiplies
by zero. Multiplying by 1 effectively does nothing so let's simplify
to see what's happening

<div class="glocal-center">
  <div class="eq">
    <div>newX = x <div class="blk">* <span class="m11">1</span></div> + <div class="blk">y * <span class="m12">0</span> + 1 * </div><span class="m13">tx</span></div>
    <div>newY = <div class="blk">x * <span class="m21">0</span> +</div> y <div class="blk">* <span class="m22">1</span></div> + <div class="blk">1 * </div><span class="m23">ty</span></div>
    <div>newZ = <div class="blk">x * <span class="m31">0</span> + y * <span class="m32">0</span> +</div> 1 <div class="blk">* <span class="m33">1</span></div></div>
  </div>
</div>

or more succinctly

<div class="webgpu_center"><pre class="webgpu_math">
newX = x + tx;
newY = y + ty;
</pre></div>

And newZ we don't really care about.

That looks surprisingly like
[the translation code from our translation example](webgpu-translation.html).

Similarly let's do rotation. Like we pointed out in the rotation post
we just need the sine and cosine of the angle at which we want to rotate, so

<div class="webgpu_center"><pre class="webgpu_math">
s = sin(angleToRotateInRadians);
c = cos(angleToRotateInRadians);
</pre></div>

And we build a matrix like this

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

Applying the matrix we get this

<div class="glocal-center">
  <div class="eq">
    <div>newX = x * <span class="m11">c</span> + y * <span class="m12">-s</span> + 1 * <span class="m13">0</span></div>
    <div>newY = x * <span class="m21">s</span> + y * <span class="m22">c</span> + 1 * <span class="m23">0</span></div>
    <div>newZ = x * <span class="m31">0</span> + y * <span class="m32">0</span> + 1 * <span class="m33">1</span></div>
  </div>
</div>

Blacking out all multiply by 0s and 1s we get

<div class="glocal-center">
  <div class="eq">
    <div>newX = x * <span class="m11">c</span> + y * <span class="m12">-s</span><div class="blk"> + 1 * <span class="m13">0</span></div></div>
    <div>newY = x * <span class="m21">s</span> + y * <span class="m22">c</span><div class="blk"> + 1 * <span class="m23">0</span></div></div>
    <div>newZ = <div class="blk">x * <span class="m31">0</span> + y * <span class="m32">0</span> +</div> 1 <div class="blk">* <span class="m33">1</span></div></div>
  </div>
</div>

And simplifying we get

<div class="webgpu_center">
<pre class="webgpu_math">
newX = x * c - y * s;
newY = x * s + y * c;
</pre>
</div>

Which is exactly what we had in our [rotation example](webgpu-rotation.html).

And lastly scale. We'll call our 2 scale factors sx and sy

And we build a matrix like this

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

Applying the matrix we get this

<div class="glocal-center">
  <div class="eq">
    <div>newX = x * <span class="m11">sx</span> + y * <span class="m12">0</span> + 1 * <span class="m13">0</span></div>
    <div>newY = x * <span class="m21">0</span> + y * <span class="m22">sy</span> + 1 * <span class="m23">0</span></div>
    <div>newZ = x * <span class="m31">0</span> + y * <span class="m32">0</span> + 1 * <span class="m33">1</span></div>
  </div>
</div>

which is really

<div class="glocal-center">
  <div class="eq">
    <div>newX = x * <span class="m11">sx</span><div class="blk"> + y * <span class="m12">0</span> + 1 * <span class="m13">0</span></div></div>
    <div>newY = <div class="blk">x * <span class="m21">0</span> +</div> y * <span class="m22">sy</span><div class="blk"> + 1 * <span class="m23">0</span></div></div>
    <div>newZ = <div class="blk">x * <span class="m31">0</span> + y * <span class="m32">0</span> +</div> 1 <div class="blk">* <span class="m33">1</span></div></div>
  </div>
</div>

which simplified is

<div class="webgpu_center">
<pre class="webgpu_math">
newX = x * sx;
newY = y * sy;
</pre>
</div>

Which is the same as our [scaling example](webgpu-scale.html).

Now I'm sure you might still be thinking "So what? What's the point?"
That seems like a lot of work just to do the same thing we were already doing.

This is where the magic comes in. It turns out we can multiply matrices
together and apply all the transformations at once. Let's assume we have
a function, `mat3::multiply`, that takes two matrices, multiplies them and
returns the result. We'll keep each matrix in a plain 9 element array.

```rust
mod mat3 {
    pub fn multiply(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
        let a00 = a[0 * 3 + 0];
        let a01 = a[0 * 3 + 1];
        let a02 = a[0 * 3 + 2];
        let a10 = a[1 * 3 + 0];
        let a11 = a[1 * 3 + 1];
        let a12 = a[1 * 3 + 2];
        let a20 = a[2 * 3 + 0];
        let a21 = a[2 * 3 + 1];
        let a22 = a[2 * 3 + 2];
        let b00 = b[0 * 3 + 0];
        let b01 = b[0 * 3 + 1];
        let b02 = b[0 * 3 + 2];
        let b10 = b[1 * 3 + 0];
        let b11 = b[1 * 3 + 1];
        let b12 = b[1 * 3 + 2];
        let b20 = b[2 * 3 + 0];
        let b21 = b[2 * 3 + 1];
        let b22 = b[2 * 3 + 2];

        [
            b00 * a00 + b01 * a10 + b02 * a20,
            b00 * a01 + b01 * a11 + b02 * a21,
            b00 * a02 + b01 * a12 + b02 * a22,
            b10 * a00 + b11 * a10 + b12 * a20,
            b10 * a01 + b11 * a11 + b12 * a21,
            b10 * a02 + b11 * a12 + b12 * a22,
            b20 * a00 + b21 * a10 + b22 * a20,
            b20 * a01 + b21 * a11 + b22 * a21,
            b20 * a02 + b21 * a12 + b22 * a22,
        ]
    }
}
```

To make things clearer let's make functions to build matrices for
translation, rotation and scale.

```rust
mod mat3 {
    pub fn multiply(a: &[f32; 9], b: &[f32; 9]) -> [f32; 9] {
        ...
    }

    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 9] {
        [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            tx, ty, 1.0,
        ]
    }

    pub fn rotation(angle_in_radians: f32) -> [f32; 9] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        [
            c, s, 0.0,
            -s, c, 0.0,
            0.0, 0.0, 1.0,
        ]
    }

    pub fn scaling([sx, sy]: [f32; 2]) -> [f32; 9] {
        [
            sx, 0.0, 0.0,
            0.0, sy, 0.0,
            0.0, 0.0, 1.0,
        ]
    }
}
```

Now let's change our shader to use a matrix

```wgsl
struct Uniforms {
  color: vec4f,
  resolution: vec2f,
-  translation: vec2f,
-  rotation: vec2f,
-  scale: vec2f,
+  matrix: mat3x3f,
};

...

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;

-  // Scale the position
-  let scaledPosition = vert.position * uni.scale;
-
-  // Rotate the position
-  let rotatedPosition = vec2f(
-    scaledPosition.x * uni.rotation.x - scaledPosition.y * uni.rotation.y,
-    scaledPosition.x * uni.rotation.y + scaledPosition.y * uni.rotation.x
-  );
-
-  // Add in the translation
-  let position = rotatedPosition + uni.translation;
+  // Multiply by a matrix
+  let position = (uni.matrix * vec3f(vert.position, 1)).xy;

  ...
```

As you can see above we passed in 1 for z. Multiplied the position
by the matrix, then just kept x and y from the result.

Again we need to update our uniform buffer size and offsets

```rust
-  // color, resolution, translation, rotation, scale
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 2 + 2) * 4;
+  // color, resolution, padding, matrix
+  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 12) * 4;
  let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("uniforms"),
    size: UNIFORM_BUFFER_SIZE,
    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_RESOLUTION_OFFSET: usize = 4;
-  const K_TRANSLATION_OFFSET: usize = 6;
-  const K_ROTATION_OFFSET: usize = 8;
-  const K_SCALE_OFFSET: usize = 10;
+  const K_MATRIX_OFFSET: usize = 8;
```

And finally we need to do some *matrix math* at render time

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...
    let translation = [
        wgpu_fun::setting_f64("translationX", 150.0) as f32,
        wgpu_fun::setting_f64("translationY", 100.0) as f32,
    ];
-    let angle = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
-    let rotation = [angle.cos(), angle.sin()];
+    let rotation = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
    let scale = [
        wgpu_fun::setting_f64("scaleX", 1.0) as f32,
        wgpu_fun::setting_f64("scaleY", 1.0) as f32,
    ];

+    let translation_matrix = mat3::translation(translation);
+    let rotation_matrix = mat3::rotation(rotation);
+    let scale_matrix = mat3::scaling(scale);
+
+    let mut matrix = mat3::multiply(&translation_matrix, &rotation_matrix);
+    matrix = mat3::multiply(&matrix, &scale_matrix);

    // Set the uniform values in our Rust side array
    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
-    uniform_values[K_TRANSLATION_OFFSET..K_TRANSLATION_OFFSET + 2]
-        .copy_from_slice(&translation);
-    uniform_values[K_ROTATION_OFFSET..K_ROTATION_OFFSET + 2]
-        .copy_from_slice(&rotation);
-    uniform_values[K_SCALE_OFFSET..K_SCALE_OFFSET + 2]
-        .copy_from_slice(&scale);
+    // copy each column of 3 values, followed by 1 float of padding
+    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 3].copy_from_slice(&matrix[0..3]);
+    uniform_values[K_MATRIX_OFFSET + 4..K_MATRIX_OFFSET + 7].copy_from_slice(&matrix[3..6]);
+    uniform_values[K_MATRIX_OFFSET + 8..K_MATRIX_OFFSET + 11].copy_from_slice(&matrix[6..9]);
```

Here's it is using our new code. The sliders are the same, translation,
rotation and scale. But the way they get used in the shader is much simpler.

{{{example url="../webgpu-matrix-math-transform-trs-3x3.html"}}}

## <a id="a-columns-are-rows"></a> Columns are Rows

In the description of how a matrix works we talked about multiplying by columns.
As one example we showed this matrix as an example of a translation matrix.

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

But when we actually built the matrix in code we did this

```rust
    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 9] {
        [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            tx, ty, 1.0,
        ]
    }
```

The `tx, ty, 1.0` part is in the bottom row, not the last column.

```rust
    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 9] {
        [
            1.0, 0.0, 0.0,  // <-- 1st column
            0.0, 1.0, 0.0,  // <-- 2nd column
            tx, ty, 1.0,    // <-- 3rd column
        ]
    }
```


The way some graphics gurus resolve this is they call these columns.
Sadly, it's just something you have to get used to. Math books and math articles on the net will show matrices like the diagram above
where `tx, ty, 1` are in the last column but when we put them in code, at least in WebGPU, we specify them as above.

## Matrix Math is Flexible

Still, you might be asking, so what? That doesn't seem like much of a benefit.
The benefit is, now, if we want to change the order of operations, we don't have to write a new shader.
We can just change the math in Rust

```rust
-    let mut matrix = mat3::multiply(&translation_matrix, &rotation_matrix);
-    matrix = mat3::multiply(&matrix, &scale_matrix);
+    let mut matrix = mat3::multiply(&scale_matrix, &rotation_matrix);
+    matrix = mat3::multiply(&matrix, &translation_matrix);
```

Above we switched from applying translation→rotation→scale to scale→rotation→translation

{{{example url="../webgpu-matrix-math-transform-srt-3x3.html"}}}

Play with the sliders and you'll see the react differently now what we're composing the
matrices in a different order. For example, translation is happening after rotation

<div class="webgpu_center compare" style="justify-content: space-evenly;">
  <div style="flex: 0 0 auto;">
    <div>translation→rotation→scale</div>
    <div><div data-diagram="trs"></div></div>
  </div>
  <div style="flex: 0 0 auto;">
    <div>scale→rotation→translation</div>
    <div><div data-diagram="srt"></div></div>
  </div>
</div>

The one on the left could be described as a scaled and rotated F, translated left and right.
Where as the one on the right could better be described as the translation itself
has been rotated and scaled. The movement is not left↔right, it's diagonal. The further,
the F on th right not moving nearly as far because the translate itself has been scaled.

This flexibility is why matrix math is a core component of
all most all computer graphics.

Being able to apply matrices like this is especially important for
hierarchical animation like arms and legs on a body, moons around a planet around a
sun, or branches on a tree. For a simple example of hierarchical
matrix application lets draw the 'F' five times, but each time lets start with the
matrix from the previous 'F'.

To do this we need 5 uniform buffers, 5 uniform values, and 5 bindGroups

```rust
  // color, resolution, padding, matrix
  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 12) * 4;

  // offsets to the various uniform values in float32 indices
  const K_COLOR_OFFSET: usize = 0;
  const K_RESOLUTION_OFFSET: usize = 4;
  const K_MATRIX_OFFSET: usize = 8;

+  struct ObjectInfo {
+      uniform_buffer: wgpu::Buffer,
+      uniform_values: [f32; UNIFORM_BUFFER_SIZE as usize / 4],
+      bind_group: wgpu::BindGroup,
+  }
+
+  const NUM_OBJECTS: usize = 5;
+  let mut object_infos: Vec<ObjectInfo> = Vec::new();
+  for _i in 0..NUM_OBJECTS {
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("uniforms"),
      size: UNIFORM_BUFFER_SIZE,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // The color will not change so let's set it once at init time
    uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&[
      rand(0.0, 1.0),
      rand(0.0, 1.0),
      rand(0.0, 1.0),
      1.0,
    ]);

    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("bind group for object"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: uniform_buffer.as_entire_binding(),
      }],
    });

+    object_infos.push(ObjectInfo {
+      uniform_buffer,
+      uniform_values,
+      bind_group,
+    });
+  }
```

At render time we loop through the them and multiply the previous matrix
by our translation, rotation, and scale matrices.

```rust
app.run(RenderMode::Once, move |frame: &Frame| {
  ...

  let translation_matrix = mat3::translation(translation);
  let rotation_matrix = mat3::rotation(rotation);
  let scale_matrix = mat3::scaling(scale);

-  let mut matrix = mat3::multiply(&translation_matrix, &rotation_matrix);
-  matrix = mat3::multiply(&matrix, &scale_matrix);

+  // Starting Matrix.
+  let mut matrix = mat3::identity();
+
+  for object_info in object_infos.iter_mut() {
+    matrix = mat3::multiply(&matrix, &translation_matrix);
+    matrix = mat3::multiply(&matrix, &rotation_matrix);
+    matrix = mat3::multiply(&matrix, &scale_matrix);

    // Set the uniform values in our Rust side array
    object_info.uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
    // copy each column of 3 values, followed by 1 float of padding
    object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 3]
        .copy_from_slice(&matrix[0..3]);
    object_info.uniform_values[K_MATRIX_OFFSET + 4..K_MATRIX_OFFSET + 7]
        .copy_from_slice(&matrix[3..6]);
    object_info.uniform_values[K_MATRIX_OFFSET + 8..K_MATRIX_OFFSET + 11]
        .copy_from_slice(&matrix[6..9]);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(
        &object_info.uniform_buffer,
        0,
        bytemuck::cast_slice(&object_info.uniform_values),
    );

    pass.set_bind_group(0, &object_info.bind_group, &[]);
    pass.draw_indexed(0..num_vertices, 0, 0..1);
+  }
```

To make this work we introduced the function, `mat3::identity`, that makes an
identity matrix.  An identity matrix is a matrix that effectively
represents 1.0 so that if you multiply by the identity nothing happens.
Just like

<div class="webgpu_center"><div class="webgpu_math">X * 1 = X</div></div>

so too

<div class="webgpu_center"><div class="webgpu_math">matrixX * identity = matrixX</div></div>

Here's the code to make an identity matrix.

```rust
mod mat3 {
    ...
    pub fn identity() -> [f32; 9] {
        [
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ]
    }

    ...
```

Here's the five Fs.

{{{example url="../webgpu-matrix-math-transform-five-fs-3x3.html"}}}

Drag the sliders and see how each subsequent 'F' is drawn relative to
the previous 'F''s size and orientation. This is how an arm on a CG human
works where the rotation of the arm affects the forearm, and the rotation
of the forearm affects than hand, and the rotation of the hand affects the
fingers, etc...

## Changing the Center of Rotation or Scaling

Let's see one more example.  In every example so far, our 'F' rotates around
its top left corner (well except for the example were we reversed the order above).
This is because the math we are using always rotates around the origin and
the top left corner of our 'F' is at the origin, (0, 0).

But now, because we can do matrix math, and we can choose the order that
transforms are applied, we can move the origin.

```rust
    let translation_matrix = mat3::translation(translation);
    let rotation_matrix = mat3::rotation(rotation);
    let scale_matrix = mat3::scaling(scale);
+    // make a matrix that will move the origin of the 'F' to its center.
+    let move_origin_matrix = mat3::translation([-50.0, -75.0]);

    let mut matrix = mat3::multiply(&translation_matrix, &rotation_matrix);
    matrix = mat3::multiply(&matrix, &scale_matrix);
+    matrix = mat3::multiply(&matrix, &move_origin_matrix);
```

Above we had a translation to move the F -50, -75. This moves all of it's
points so 0,0 is at the center of the F.
Drag the sliders and Notice the F rotates and scales around its center.

{{{example url="../webgpu-matrix-math-transform-move-origin-3x3.html" }}}

Using that technique, you can rotate or scale from any point. Now you know
how your favorite image editing program lets you move the rotation point.

## Adding in Projection

Let's go even more crazy.  You might remember we have code in
the shader to convert from pixels to clip space that looks like this.

```wgsl
// convert the position from pixels to a 0.0 to 1.0 value
let zeroToOne = position / uni.resolution;

// convert from 0 <-> 1 to 0 <-> 2
let zeroToTwo = zeroToOne * 2.0;

// covert from 0 <-> 2 to -1 <-> +1 (clip space)
let flippedClipSpace = zeroToTwo - 1.0;

// flip Y
let clipSpace = flippedClipSpace * vec2f(1, -1);

vsOut.position = vec4f(clipSpace, 0.0, 1.0);
```

If you look at each of those steps in turn:

The first step, "convert the position from pixels to a 0.0 to 1.0 value", is really a scale operation. `zeroToOne = position / uni.resolution` is the same as `zeroToOne = position * (1 / uni.resolution)` which is scaling.

The second step, `let zeroToTwo = zeroToOne * 2.0;` is also a scale operation. It's
scaling by 2.

The 3rd step, `flippedClipSpace = zeroToTwo - 1.0;` is a translation.

The 4th step, `clipSpace = flippedClipSpace * vec2f(1, -1);` is a scale.

So, we could add this to our math

```rust
+  let scale_by_1_over_resolution_matrix =
+      mat3::scaling([1.0 / frame.width as f32, 1.0 / frame.height as f32]);
+  let scale_by_2_matrix = mat3::scaling([2.0, 2.0]);
+  let translate_by_minus_1 = mat3::translation([-1.0, -1.0]);
+  let scale_by_1_minus_1 = mat3::scaling([1.0, -1.0]);

  let translation_matrix = mat3::translation(translation);
  let rotation_matrix = mat3::rotation(rotation);
  let scale_matrix = mat3::scaling(scale);

-  let mut matrix = mat3::multiply(&translation_matrix, &rotation_matrix);
+  let mut matrix = mat3::multiply(&scale_by_1_minus_1, &translate_by_minus_1);
+  matrix = mat3::multiply(&matrix, &scale_by_2_matrix);
+  matrix = mat3::multiply(&matrix, &scale_by_1_over_resolution_matrix);
+  matrix = mat3::multiply(&matrix, &translation_matrix);
+  matrix = mat3::multiply(&matrix, &rotation_matrix);
  matrix = mat3::multiply(&matrix, &scale_matrix);
```

Then our shader would could change to this

```wgsl
struct Uniforms {
  color: vec4f,
-  resolution: vec2f,
  matrix: mat3x3f,
};

struct Vertex {
  @location(0) position: vec2f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;

-  let position = (uni.matrix * vec3f(vert.position, 1)).xy;
-
-  // convert the position from pixels to a 0.0 to 1.0 value
-  let zeroToOne = position / uni.resolution;
-
-  // convert from 0 <-> 1 to 0 <-> 2
-  let zeroToTwo = zeroToOne * 2.0;
-
-  // covert from 0 <-> 2 to -1 <-> +1 (clip space)
-  let flippedClipSpace = zeroToTwo - 1.0;
-
-  // flip Y
-  let clipSpace = flippedClipSpace * vec2f(1, -1);
-
-  vsOut.position = vec4f(clipSpace, 0.0, 1.0);
+  let clipSpace = (uni.matrix * vec3f(vert.position, 1)).xy;
+
+  vsOut.position = vec4f(clipSpace, 0.0, 1.0);
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  return uni.color;
}
```

Our shader is super simple now and we've lost no functionally.
In fact it's become more flexible! We're no longer hard coded to
representing pixels. We could choose different units from outside the shader.
All because we're using matrix math.

Rather than make those 4 extra matrices though, we could just make
a function that generates the same result

```rust
mod mat3 {
    pub fn projection(width: f32, height: f32) -> [f32; 9] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        [
            2.0 / width, 0.0, 0.0,
            0.0, -2.0 / height, 0.0,
            -1.0, 1.0, 1.0,
        ]
    }

    ...
```

And our Rust would change to this

```rust
-  let scale_by_1_over_resolution_matrix =
-      mat3::scaling([1.0 / frame.width as f32, 1.0 / frame.height as f32]);
-  let scale_by_2_matrix = mat3::scaling([2.0, 2.0]);
-  let translate_by_minus_1 = mat3::translation([-1.0, -1.0]);
-  let scale_by_1_minus_1 = mat3::scaling([1.0, -1.0]);
  let projection_matrix = mat3::projection(frame.width as f32, frame.height as f32);
  let translation_matrix = mat3::translation(translation);
  let rotation_matrix = mat3::rotation(rotation);
  let scale_matrix = mat3::scaling(scale);

-  let mut matrix = mat3::multiply(&scale_by_1_minus_1, &translate_by_minus_1);
-  matrix = mat3::multiply(&matrix, &scale_by_2_matrix);
-  matrix = mat3::multiply(&matrix, &scale_by_1_over_resolution_matrix);
-  matrix = mat3::multiply(&matrix, &translation_matrix);
  let mut matrix = mat3::multiply(&projection_matrix, &translation_matrix);
  matrix = mat3::multiply(&matrix, &rotation_matrix);
  matrix = mat3::multiply(&matrix, &scale_matrix);
  matrix = mat3::multiply(&matrix, &move_origin_matrix);
```

We also removed the code that made space for the resolution in our uniform buffer
and the code that set it. 

With this last step
we've gone from a rather complicated shader with 6-7 steps to a very
simple shader with only 1 step that is more flexible, all due to the magic of matrix math.

{{{example url="../webgpu-matrix-math-transform-just-matrix-3x3.html" }}}

## Matrix Multiply as we go

Before we move on let's simplify a little bit. While it's common to generate
various matrices and separately multiply them together it's also common to just
multiply them as we go. Effectively we could write functions like this

```rust
mod mat3 {

    ...

    pub fn translate(m: &[f32; 9], t: [f32; 2]) -> [f32; 9] {
        multiply(m, &translation(t))
    }

    pub fn rotate(m: &[f32; 9], angle_in_radians: f32) -> [f32; 9] {
        multiply(m, &rotation(angle_in_radians))
    }

    pub fn scale(m: &[f32; 9], s: [f32; 2]) -> [f32; 9] {
        multiply(m, &scaling(s))
    }

    ...

}
```

This would let us change 7 lines of matrix code above to just 4 lines like this

```rust
let projection_matrix = mat3::projection(frame.width as f32, frame.height as f32);
-let translation_matrix = mat3::translation(translation);
-let rotation_matrix = mat3::rotation(rotation);
-let scale_matrix = mat3::scaling(scale);
-
-let mut matrix = mat3::multiply(&projection_matrix, &translation_matrix);
-matrix = mat3::multiply(&matrix, &rotation_matrix);
-matrix = mat3::multiply(&matrix, &scale_matrix);
+let mut matrix = mat3::translate(&projection_matrix, translation);
+matrix = mat3::rotate(&matrix, rotation);
+matrix = mat3::scale(&matrix, scale);
```

## mat3x3 is 3 padded vec3fs

As pointed out in [the article on memory layout](webgpu-memory-layout.md), `vec3f`s often take the space of 4 floats, not 3.

This is what a `mat3x3f` looks like in memory

<div class="webgpu_center" data-diagram="mat3x3f"></div>

This is why we needed this code to copy it into the uniform values

```rust
    // copy each column of 3 values, followed by 1 float of padding
    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 3].copy_from_slice(&matrix[0..3]);
    uniform_values[K_MATRIX_OFFSET + 4..K_MATRIX_OFFSET + 7].copy_from_slice(&matrix[3..6]);
    uniform_values[K_MATRIX_OFFSET + 8..K_MATRIX_OFFSET + 11].copy_from_slice(&matrix[6..9]);
```

We could fix that by changing the matrix functions to expect/handle the padding.
Each matrix becomes 12 floats: 3 columns of 4, where the 4th float of each
column is unused padding.

```rust
mod mat3 {
    pub fn projection(width: f32, height: f32) -> [f32; 12] {
        // Note: This matrix flips the Y axis so that 0 is at the top.
        [
-            2.0 / width, 0.0, 0.0,
-            0.0, -2.0 / height, 0.0,
-            -1.0, 1.0, 1.0,
+            2.0 / width, 0.0, 0.0, 0.0,
+            0.0, -2.0 / height, 0.0, 0.0,
+            -1.0, 1.0, 1.0, 0.0,
        ]
    }

    pub fn identity() -> [f32; 12] {
        [
-            1.0, 0.0, 0.0,
-            0.0, 1.0, 0.0,
-            0.0, 0.0, 1.0,
+            1.0, 0.0, 0.0, 0.0,
+            0.0, 1.0, 0.0, 0.0,
+            0.0, 0.0, 1.0, 0.0,
        ]
    }

    pub fn multiply(a: &[f32; 12], b: &[f32; 12]) -> [f32; 12] {
-        let a00 = a[0 * 3 + 0];
-        let a01 = a[0 * 3 + 1];
-        let a02 = a[0 * 3 + 2];
-        let a10 = a[1 * 3 + 0];
-        let a11 = a[1 * 3 + 1];
-        let a12 = a[1 * 3 + 2];
-        let a20 = a[2 * 3 + 0];
-        let a21 = a[2 * 3 + 1];
-        let a22 = a[2 * 3 + 2];
-        let b00 = b[0 * 3 + 0];
-        let b01 = b[0 * 3 + 1];
-        let b02 = b[0 * 3 + 2];
-        let b10 = b[1 * 3 + 0];
-        let b11 = b[1 * 3 + 1];
-        let b12 = b[1 * 3 + 2];
-        let b20 = b[2 * 3 + 0];
-        let b21 = b[2 * 3 + 1];
-        let b22 = b[2 * 3 + 2];
+        let a00 = a[0 * 4 + 0];
+        let a01 = a[0 * 4 + 1];
+        let a02 = a[0 * 4 + 2];
+        let a10 = a[1 * 4 + 0];
+        let a11 = a[1 * 4 + 1];
+        let a12 = a[1 * 4 + 2];
+        let a20 = a[2 * 4 + 0];
+        let a21 = a[2 * 4 + 1];
+        let a22 = a[2 * 4 + 2];
+        let b00 = b[0 * 4 + 0];
+        let b01 = b[0 * 4 + 1];
+        let b02 = b[0 * 4 + 2];
+        let b10 = b[1 * 4 + 0];
+        let b11 = b[1 * 4 + 1];
+        let b12 = b[1 * 4 + 2];
+        let b20 = b[2 * 4 + 0];
+        let b21 = b[2 * 4 + 1];
+        let b22 = b[2 * 4 + 2];

        [
            b00 * a00 + b01 * a10 + b02 * a20,
            b00 * a01 + b01 * a11 + b02 * a21,
            b00 * a02 + b01 * a12 + b02 * a22,
+            0.0,
            b10 * a00 + b11 * a10 + b12 * a20,
            b10 * a01 + b11 * a11 + b12 * a21,
            b10 * a02 + b11 * a12 + b12 * a22,
+            0.0,
            b20 * a00 + b21 * a10 + b22 * a20,
            b20 * a01 + b21 * a11 + b22 * a21,
            b20 * a02 + b21 * a12 + b22 * a22,
+            0.0,
        ]
    }

    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 12] {
        [
-            1.0, 0.0, 0.0,
-            0.0, 1.0, 0.0,
-            tx, ty, 1.0,
+            1.0, 0.0, 0.0, 0.0,
+            0.0, 1.0, 0.0, 0.0,
+            tx, ty, 1.0, 0.0,
        ]
    }

    pub fn rotation(angle_in_radians: f32) -> [f32; 12] {
        let c = angle_in_radians.cos();
        let s = angle_in_radians.sin();
        [
-            c, s, 0.0,
-            -s, c, 0.0,
-            0.0, 0.0, 1.0,
+            c, s, 0.0, 0.0,
+            -s, c, 0.0, 0.0,
+            0.0, 0.0, 1.0, 0.0,
        ]
    }

    pub fn scaling([sx, sy]: [f32; 2]) -> [f32; 12] {
        [
-            sx, 0.0, 0.0,
-            0.0, sy, 0.0,
-            0.0, 0.0, 1.0,
+            sx, 0.0, 0.0, 0.0,
+            0.0, sy, 0.0, 0.0,
+            0.0, 0.0, 1.0, 0.0,
        ]
    }
}
```

Now we can change the part that sets our matrix

```rust
-    // copy each column of 3 values, followed by 1 float of padding
-    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 3].copy_from_slice(&matrix[0..3]);
-    uniform_values[K_MATRIX_OFFSET + 4..K_MATRIX_OFFSET + 7].copy_from_slice(&matrix[3..6]);
-    uniform_values[K_MATRIX_OFFSET + 8..K_MATRIX_OFFSET + 11].copy_from_slice(&matrix[6..9]);
+    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 12].copy_from_slice(&matrix);
```

## Updating Matrices in place

Another thing we can do is have our matrix functions write their result
into a destination slice instead of returning a new array. This lets us
update a matrix in place, instead of copying it. In particular, we can pass
in the matrix's slice of the uniform values and operate directly on it.

To take 3 examples

```rust
mod mat3 {
-    pub fn multiply(a: &[f32; 12], b: &[f32; 12]) -> [f32; 12] {
+    pub fn multiply(a: &[f32], b: &[f32], dst: &mut [f32]) {
        let a00 = a[0 * 4 + 0];
        let a01 = a[0 * 4 + 1];
        let a02 = a[0 * 4 + 2];
        let a10 = a[1 * 4 + 0];
        let a11 = a[1 * 4 + 1];
        let a12 = a[1 * 4 + 2];
        let a20 = a[2 * 4 + 0];
        let a21 = a[2 * 4 + 1];
        let a22 = a[2 * 4 + 2];
        let b00 = b[0 * 4 + 0];
        let b01 = b[0 * 4 + 1];
        let b02 = b[0 * 4 + 2];
        let b10 = b[1 * 4 + 0];
        let b11 = b[1 * 4 + 1];
        let b12 = b[1 * 4 + 2];
        let b20 = b[2 * 4 + 0];
        let b21 = b[2 * 4 + 1];
        let b22 = b[2 * 4 + 2];

-        [
-            b00 * a00 + b01 * a10 + b02 * a20,
-            b00 * a01 + b01 * a11 + b02 * a21,
-            b00 * a02 + b01 * a12 + b02 * a22,
-            0.0,
-            b10 * a00 + b11 * a10 + b12 * a20,
-            b10 * a01 + b11 * a11 + b12 * a21,
-            b10 * a02 + b11 * a12 + b12 * a22,
-            0.0,
-            b20 * a00 + b21 * a10 + b22 * a20,
-            b20 * a01 + b21 * a11 + b22 * a21,
-            b20 * a02 + b21 * a12 + b22 * a22,
-            0.0,
-        ]
+        dst[0] = b00 * a00 + b01 * a10 + b02 * a20;
+        dst[1] = b00 * a01 + b01 * a11 + b02 * a21;
+        dst[2] = b00 * a02 + b01 * a12 + b02 * a22;
+
+        dst[4] = b10 * a00 + b11 * a10 + b12 * a20;
+        dst[5] = b10 * a01 + b11 * a11 + b12 * a21;
+        dst[6] = b10 * a02 + b11 * a12 + b12 * a22;
+
+        dst[8] = b20 * a00 + b21 * a10 + b22 * a20;
+        dst[9] = b20 * a01 + b21 * a11 + b22 * a21;
+        dst[10] = b20 * a02 + b21 * a12 + b22 * a22;
    }
-    pub fn translation([tx, ty]: [f32; 2]) -> [f32; 12] {
-        [
-            1.0, 0.0, 0.0, 0.0,
-            0.0, 1.0, 0.0, 0.0,
-            tx, ty, 1.0, 0.0,
-        ]
+    pub fn translation([tx, ty]: [f32; 2], dst: &mut [f32]) {
+        dst[0] = 1.0;  dst[1] = 0.0;  dst[2] = 0.0;
+        dst[4] = 0.0;  dst[5] = 1.0;  dst[6] = 0.0;
+        dst[8] = tx;   dst[9] = ty;   dst[10] = 1.0;
    }
-    pub fn translate(m: &[f32; 12], t: [f32; 2]) -> [f32; 12] {
-        multiply(m, &translation(t))
+    // `m` is both the input and the destination. We read `m` out into a
+    // local copy first, so it's safe to update a matrix with itself.
+    pub fn translate(m: &mut [f32], t: [f32; 2]) {
+        let mut tm = [0.0f32; 12];
+        translation(t, &mut tm);
+        let a: [f32; 12] = m[..12].try_into().unwrap();
+        multiply(&a, &tm, m);
    }

    ...
```

Doing the same for the other functions and now our code can change
to this

```rust
-    let projection_matrix = mat3::projection(frame.width as f32, frame.height as f32);
-    let mut matrix = mat3::translate(&projection_matrix, translation);
-    matrix = mat3::rotate(&matrix, rotation);
-    matrix = mat3::scale(&matrix, scale);
-    uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 12].copy_from_slice(&matrix);
+    let matrix_value = &mut uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 12];
+    mat3::projection(frame.width as f32, frame.height as f32, matrix_value);
+    mat3::translate(matrix_value, translation);
+    mat3::rotate(matrix_value, rotation);
+    mat3::scale(matrix_value, scale);
```

We no longer need to copy the matrix into the uniform values. Instead we
can operate directly on `matrix_value`, the matrix's slice of the uniform
values.

{{{example url="../webgpu-matrix-math-transform-trs.html"}}}

## Transform the Points vs Transform the Space

One last thing, we saw above order matters. In the first example we had

    translation * rotation * scale

and in the second we had

    scale * rotation * translation

And we saw how they are different.

The are 2 ways to look at matrices. Given the expression

    projectionMat * translationMat * rotationMat * scaleMat * position

The first way which many people find natural is to start on the right and work
to the left

First we multiply the position by the scale matrix to get a scaled position

    scaledPosition = scaleMat * position

Then we multiply the scaledPosition by the rotation matrix to get a rotatedScaledPosition

    rotatedScaledPosition = rotationMat * scaledPosition

Then we multiply the rotatedScaledPosition by the translation matrix to get a
translatedRotatedScaledPosition

    translatedRotatedScaledPosition = translationMat * rotatedScaledPosition

And finally we multiply that by the projection matrix to get clip space positions

    clipSpacePosition = projectionMatrix * translatedRotatedScaledPosition

The 2nd way to look at matrices is reading from left to right. In that case
each matrix changes the *space* represented by the texture we're drawing to.
The texture starts with representing clip space (-1 to +1) in each direction. Each matrix applied
from left to right changes the space represented by the canvas.

Step 1:  no matrix (or the identity matrix)

> <div data-diagram="space-change-0" data-caption="clip space"></div>
>
> The white area is the texture. Blue is outside the texture. We're in clip space.
> Positions passed in need to be in clip space. The green area in the top right
> is the top left corner of the F. It's upside down because in clip space +Y is up but
> the F was designed in pixel space which is +Y down. Further, clip space shows
> only 2x2 units but the F is 100x150 units big so we just see one unit's worth.

Step 2:  `mat3::projection(frame.width as f32, frame.height as f32, matrix_value);`

> <div data-diagram="space-change-1" data-caption="from clip space to pixel space"></div>
>
> We're now in pixel space. X = 0 to textureWidth, Y = 0 to textureHeight with 0,0 at the top left.
> Positions passed using this matrix in need to be in pixel space. The flash you see
> is when the space flips from positive Y = up to positive Y = down.

Step 3:  `mat3::translate(matrix_value, translation);`

> <div data-diagram="space-change-2" data-caption="move origin to tx, ty"></div>
>
> The origin of the space has now been moved to tx, ty (150, 100).

Step 4:  `mat3::rotate(matrix_value, rotation);`

> <div data-diagram="space-change-3" data-caption="rotate 33 degrees"></div>
>
> The space has been rotated around tx, ty

Step 5:  `mat3::scale(matrix_value, scale);`

> <div data-diagram="space-change-4" data-caption="scale the space"></div>
>
> The previously rotated space with its center at tx, ty has been scaled 2 in x, 1.5 in y

In the shader we then do `clipSpace = uni.matrix * vert.position;`. The `vert.position` values are effectively applied in this final space.

Use which ever way you feel is easier to understand.

I hope these articles have helped demystify matrix math. 
Next [we'll move on to 3D](webgpu-orthographic-projection.html).
In 3D the matrix math follows the same principles and usage.
We started with 2D to hopefully keep it simple to understand.

Also, if you really want to become an expert
in matrix math [check out this amazing video](https://www.youtube.com/watch?v=kjBOesZCoqc&list=PLZHQObOWTQDPD3MizzM2xVFitgF8hE_ab).

<div class="webgpu_bottombar">
<h3>What are <code>clientWidth</code> and <code>clientHeight</code>?</h3>
<p>Up until this point, whenever we referred to the canvas's dimensions we used the frame's
<code>width</code> and <code>height</code>, which are the canvas's <code>canvas.width</code> and <code>canvas.height</code>
in the browser. But there is another pair of canvas sizes, <code>canvas.clientWidth</code> and <code>canvas.clientHeight</code>. Why do they matter?</p>
<p>Projection matrices are concerned with how to take clip space (-1 to +1 in each dimension) and convert it back
to pixels. But, in the browser, there are 2 types of pixels we are dealing with. One is the number of pixels in
the canvas itself. So for example a canvas defined like this.</p>
<pre class="prettyprint">
  &lt;canvas width="400" height="300"&gt;&lt;/canvas&gt;
</pre>
<p>or one defined like this</p>
<pre class="prettyprint">
  const canvas = document.createElement("canvas");
  canvas.width = 400;
  canvas.height = 300;
</pre>
<p>both contain an image 400 pixels wide by 300 pixels tall. But, that size is separate from what size
the browser actually displays that 400x300 pixel canvas. CSS defines what size the canvas is displayed.
For example if we made a canvas like this.</p>
<pre class="prettyprint">
  &lt;style&gt;
    canvas {
      width: 100%;
      height: 100%;
    }
  &lt;/style&gt;
  ...
  &lt;canvas width="400" height="300">&lt;/canvas&gt;
</pre>
<p>The canvas will be displayed whatever size its container is. That's likely not 400x300.</p>
<p>Here are two examples that set the canvas's CSS display size to 100% so the canvas is stretched
out to fill the page. Both leave <code>auto_resize</code> off so the drawing buffer stays 400x300.
The first one uses the frame's <code>width</code> and <code>height</code> (that is, <code>canvas.width</code> and <code>canvas.height</code>) when calling <code>mat3::projection</code>. Open it in a new
window and resize the window. Notice how the 'F' doesn't have the correct aspect. It gets
distorted. It's also not in the correct place. The code says the top left corner should be at 150, 25 but as the canvas is stretched and shrunk the position where something we want to appear at 150, 25 moves.</p>
{{{example url="../webgpu-canvas-width-height.html" width="500" height="150" }}}
<p>This second example queries <code>canvas.clientWidth</code> and <code>canvas.clientHeight</code> and uses those when calling <code>mat3::projection</code>. <code>canvas.clientWidth</code> and <code>canvas.clientHeight</code> report
the size the canvas is actually being displayed by the browser so in this case, even though the canvas still only has 400x300 pixels
since we're defining our aspect ratio based on the size the canvas is being displayed the <code>F</code> always looks correct and the F is in the correct place.</p>
{{{example url="../webgpu-canvas-clientwidth-clientheight.html" width="500" height="150" }}}
<p>Most apps that allow their canvases to be resized try to make the <code>canvas.width</code> and <code>canvas.height</code> match
the <code>canvas.clientWidth</code> and <code>canvas.clientHeight</code> because they want there to be
one pixel in the canvas for each pixel displayed by the browser. That's exactly what wgpu_fun's
<code>auto_resize</code> option does, which is why the other examples on this page can pass the frame's
<code>width</code> and <code>height</code> to <code>mat3::projection</code>. But, as we've seen above, that's not
the only option. That means, in almost all cases, it's more technically correct to compute a
projection matrix's aspect ratio using <code>canvas.clientHeight</code> and <code>canvas.clientWidth</code>.
</p>
</div>

<!-- keep this at the bottom of the article -->
<link href="webgpu-matrix-math.css" rel="stylesheet">
<script type="module" src="webgpu-matrix-math.js"></script>
