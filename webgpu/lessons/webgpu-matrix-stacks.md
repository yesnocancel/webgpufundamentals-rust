Title: WebGPU Matrix Stacks
Description: Matrix Stacks
TOC: Matrix Stacks

This article is the 8th in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="matrix-math.hanson"}}}

A matrix stack is exactly what it sounds like, a [stack](https://en.wikipedia.org/wiki/Stack_(abstract_data_type)) of matrices.
It is useful for positioning and orientating things relative to each other.
To demonstrate, let's make a set of file cabinets. Using a matrix stack will make this easy.

To keep it simple we'll make them from cubes starting with
[the last example from the previous article](webgpu-cameras#a-aim-fs).

The first thing we'll do is swap the F we'be been drawing for a unit cube.

```rust
-fn create_f_vertices() -> (Vec<f32>, u32) {
+fn create_cube_vertices() -> (Vec<f32>, u32) {
*        // left
*        0.0, 0.0,  0.0,
*        0.0, 0.0, -1.0,
*        0.0, 1.0,  0.0,
*        0.0, 1.0, -1.0,
*
*        // right
*        1.0, 0.0,  0.0,
*        1.0, 0.0, -1.0,
*        1.0, 1.0,  0.0,
*        1.0, 1.0, -1.0,
*    ];
*
*    let indices: Vec<u32> = vec![
*         0,  2,  1,    2,  3,  1,   // left
*         4,  5,  6,    6,  5,  7,   // right
*         0,  4,  2,    2,  4,  6,   // front
*         1,  3,  5,    5,  3,  7,   // back
*         0,  1,  4,    4,  1,  5,   // bottom
*         2,  6,  3,    3,  6,  7,   // top
*    ];
*
*    let quad_colors: Vec<u8> = vec![
*        200,  70, 120,  // left column front
*         80,  70, 200,  // left column back
*         70, 200, 210,  // top
*        160, 160, 220,  // top rung right
*         90, 130, 110,  // top rung bottom
*        200, 200,  70,  // between top and middle rung
*    ];

  ...
```

The data above makes a cube like this.

<div class="webgpu_center"><img src="resources/unit-cube.png" class="nobg"></div>

The old code pre-created 26 "objectsInfos" where each "objectInfo" was a set of
uniform buffer, and bindGroup, one for each thing we want to draw. Let's change
the code to instead create these on demand. That way we can just draw as many
things as we want. In JavaScript this was a `createObjectInfo` function that
captured `device` and `pipeline` from the enclosing scope. Rust functions don't
capture, so we pass those in.

```rust
-    const NUM_FS: usize = 5 * 5 + 1;
    let mut object_infos: Vec<ObjectInfo> = Vec::new();
-    for _i in 0..NUM_FS {
-        let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+fn create_object_info(device: &wgpu::Device, pipeline: &wgpu::RenderPipeline) -> ObjectInfo {
+    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {

    ...

-        object_infos.push(ObjectInfo {
-            uniform_buffer,
-            uniform_values,
-            bind_group,
-        });
-    }
+    ObjectInfo {
+        uniform_buffer,
+        uniform_values,
+        bind_group,
+    }
+}
```

We're going to be using the same unit cube for everything just to keep things
simple but we need some way to change the color a little so we can tell cubes
apart. So, let's update the fragment to take a color via our uniform buffer and
we'll multiply the vertex colors by this uniform color. That will let us
slightly change the vertex colors for each cube.

```wgsl
struct Uniforms {
  matrix: mat4x4f,
+  color: vec4f,
};

struct Vertex {
  @location(0) position: vec4f,
  @location(1) color: vec4f,
};

struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) color: vec4f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex fn vs(vert: Vertex) -> VSOutput {
  var vsOut: VSOutput;
  vsOut.position = uni.matrix * vert.position;
  vsOut.color = vert.color;
  return vsOut;
}

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
-  return vsOut.color;
+  return vsOut.color * uni.color;
}
```

We need to update the uniform buffer creation to
add space for the new color.

```rust
-// matrix
-const UNIFORM_BUFFER_SIZE: u64 = (16) * 4;
+// matrix and color
+const UNIFORM_BUFFER_SIZE: u64 = (16 + 4) * 4;

 // offsets to the various uniform values in float32 indices
 const K_MATRIX_OFFSET: usize = 0;
+const K_COLOR_OFFSET: usize = 16;

fn create_object_info(device: &wgpu::Device, pipeline: &wgpu::RenderPipeline) -> ObjectInfo {
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bind group for object"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    ObjectInfo {
        uniform_buffer,
        uniform_values,
        bind_group,
    }
}
```

The JavaScript version made `matrixValue` and `colorValue` typed array views
into `uniformValues`. In Rust we just index into `uniform_values` with the
offsets when we set the values.

Now we need to extract the code that "draws" an object into a
function. In JavaScript `drawObject` was a function that captured `device`,
`objectInfos`, `objectNdx` and `numVertices` from the enclosing scope
and took a context called `ctx` that had the render pass encoder and the
current `viewProjectionMatrix`. In Rust, everything the function needs goes
into the context.

```rust
+// In JavaScript `drawObject` was a function that captured `device`,
+// `pipeline`, `objectInfos`, `objectNdx` and `numVertices` from the
+// enclosing scope. In Rust we pass those in via the context.
+struct Ctx<'a, 'b> {
+    pass: &'a mut wgpu::RenderPass<'b>,
+    view_projection_matrix: [f32; 16],
+    device: &'a wgpu::Device,
+    queue: &'a wgpu::Queue,
+    pipeline: &'a wgpu::RenderPipeline,
+    object_infos: &'a mut Vec<ObjectInfo>,
+    object_ndx: usize,
+    num_vertices: u32,
+}
+
+fn draw_object(ctx: &mut Ctx, matrix: [f32; 16], color: [f32; 4]) {
+    if ctx.object_ndx == ctx.object_infos.len() {
+        ctx.object_infos
+            .push(create_object_info(ctx.device, ctx.pipeline));
+    }
+    let object_info = &mut ctx.object_infos[ctx.object_ndx];
+    ctx.object_ndx += 1;
+
+    let matrix_value = m4::multiply(&ctx.view_projection_matrix, &matrix);
+    object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
+        .copy_from_slice(&matrix_value);
+    object_info.uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&color);
+
+    // upload the uniform values to the uniform buffer
+    ctx.queue.write_buffer(
+        &object_info.uniform_buffer,
+        0,
+        bytemuck::cast_slice(&object_info.uniform_values),
+    );
+
+    ctx.pass.set_bind_group(0, &object_info.bind_group, &[]);
+    ctx.pass.draw(0..ctx.num_vertices, 0..1);
+}
```

and in the render code we can delete the old loop that drew the Fs

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {

    ...

            pass.set_pipeline(&pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));

-            // update target X,Z based on angle
-            settings_target[0] = target_angle.cos() * radius;
-            settings_target[2] = target_angle.sin() * radius;

    ...

-            for (i, object_info) in object_infos.iter_mut().enumerate() {
-                let deep = 5;
-                let across = 5;
-                let matrix_value = if i < 25 {
-                    // compute grid positions
-                    let grid_x = i % across;
-                    let grid_z = i / across;
-
-                    // compute 0 to 1 positions
-                    let u = grid_x as f32 / (across - 1) as f32;
-                    let v = grid_z as f32 / (deep - 1) as f32;
-
-                    // center and spread out
-                    let x = (u - 0.5) * across as f32 * 150.0;
-                    let z = (v - 0.5) * deep as f32 * 150.0;
-
-                    // aim this F from it's position toward the target F
-                    let aim_matrix = m4::aim([x, 0.0, z], settings_target, up);
-                    m4::multiply(&view_projection_matrix, &aim_matrix)
-                } else {
-                    m4::translate(&view_projection_matrix, settings_target)
-                };
-                object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
-                    .copy_from_slice(&matrix_value);
-
-                // upload the uniform values to the uniform buffer
-                frame.queue.write_buffer(
-                    &object_info.uniform_buffer,
-                    0,
-                    bytemuck::cast_slice(&object_info.uniform_values),
-                );
-
-                pass.set_bind_group(0, &object_info.bind_group, &[]);
-                pass.draw(0..num_vertices, 0..1);
-            }
```

We added a function `draw_object` that will make a new "objectInfo" (a uniform
buffer, and a CPU side copy of its values) if it needs to. `draw_object` takes
a context called `ctx` that has the render pass encoder and the current
`view_projection_matrix`. It also takes a matrix and a color. It fills out the
uniform buffer for this object by multiplying the matrix passed in with the
`view_projection_matrix` and then sets the bind group to use that specific
uniform buffer and calls `draw`.

Now let's add some code to use it to draw the cube. Where the JavaScript
version reset `objectNdx = 0` before making the ctx, in Rust we just build a
fresh `Ctx` each render with `object_ndx: 0`.

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {

    ...

            pass.set_pipeline(&pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));

    ...

+            let mut ctx = Ctx {
+                pass: &mut pass,
+                view_projection_matrix,
+                device: frame.device,
+                queue: frame.queue,
+                pipeline: &pipeline,
+                object_infos: &mut object_infos,
+                object_ndx: 0,
+                num_vertices,
+            };
+            draw_object(&mut ctx, m4::rotation_y(base_rotation), [1.0, 1.0, 1.0, 1.0]);
```

Above we pass in a matrix that rotates around the y axis and the color white.
This means the cube will be drawn with its vertex colors unchanged.

We need a few more tweaks for the gui and camera. On the
example page's JavaScript side

```js
-  const settings = {
-    target: [0, 200, 300],
-    targetAngle: 0,
-  };
+  const settings = {
+    baseRotation: 0,
+  };

  const radToDegOptions = { min: -360, max: 360, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
-  gui.add(settings.target, '1', -100, 300).name('target height')
-     .onChange(v => wasm.set_setting_num('targetHeight', v));
-  gui.add(settings, 'targetAngle', radToDegOptions).name('target angle')
-     .onChange(v => wasm.set_setting_num('targetAngle', v));
+  gui.add(settings, 'baseRotation', radToDegOptions)
+     .onChange(v => wasm.set_setting_num('baseRotation', v));
```

and in the Rust render code

```rust
-            let mut settings_target = [
-                0.0,
-                wgpu_fun::setting_f64("targetHeight", 200.0) as f32,
-                300.0,
-            ];
-            let target_angle = wgpu_fun::setting_f64("targetAngle", 0.0) as f32;
+            let base_rotation = wgpu_fun::setting_f64("baseRotation", 0.0) as f32;

  ...

-            let eye = [-500.0, 300.0, -500.0];
-            let target = [0.0, -100.0, 0.0];
+            let eye = [0.0, 2.0, 3.0];
+            let target = [0.0, 1.0, 0.0];
            let up = [0.0, 1.0, 0.0];

            // Compute a view matrix
            let view_matrix = m4::look_at(eye, target, up);

```

We have a cube.

{{{example url="../webgpu-matrix-stack-cube.html" }}}

Now that we are able to render cubes, lets use a matrix stack
to help us make a set of file cabinets.

First, lets make a matrix stack struct.

```rust
struct MatrixStack {
    matrix: [f32; 16],
    stack: Vec<[f32; 16]>,
}

impl MatrixStack {
    fn new() -> Self {
        MatrixStack {
            matrix: m4::identity(),
            stack: Vec::new(),
        }
    }
    fn reset(&mut self) -> &mut Self {
        self.matrix = m4::identity();
        self.stack.clear();
        self
    }
    fn save(&mut self) -> &mut Self {
        // [f32; 16] is Copy so pushing copies the current matrix
        self.stack.push(self.matrix);
        self
    }
    fn restore(&mut self) -> &mut Self {
        self.matrix = self.stack.pop().unwrap();
        self
    }
    fn get(&self) -> [f32; 16] {
        self.matrix
    }
    fn set(&mut self, matrix: [f32; 16]) -> &mut Self {
        self.matrix = matrix;
        self
    }
    fn translate(&mut self, translation: [f32; 3]) -> &mut Self {
        self.matrix = m4::translate(&self.matrix, translation);
        self
    }
    fn rotate_x(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_x(&self.matrix, angle);
        self
    }
    fn rotate_y(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_y(&self.matrix, angle);
        self
    }
    fn rotate_z(&mut self, angle: f32) -> &mut Self {
        self.matrix = m4::rotate_z(&self.matrix, angle);
        self
    }
    fn scale(&mut self, scale: [f32; 3]) -> &mut Self {
        self.matrix = m4::scale(&self.matrix, scale);
        self
    }
}
```

The struct above is pretty straight forward. It keeps a `stack` which is
a `Vec` of matrices. And, it keeps a `matrix` which is effectively
the top matrix on the stack.

It adds a bunch of methods that use the `m4` functions
[we wrote previously](webgpu-orthograph-projection.html)
to manipulate the matrix at the top of the stack. Each method returns
`&mut Self` so calls can be chained, just like the JavaScript version
returned `this`.

Note: It's a stack but I choose the names `save` and `restore` instead of
the more traditional `push` and `pop` because `save` and `restore` match
the functions from the Canvas 2D API's
[save](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/save) and
[restore](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/restore)
which are used to manipulate its own matrix stack.

The JavaScript version needed a `mat4.copy` function so that `save` could
push a copy of the current matrix. In Rust, `[f32; 16]` is a plain array
which implements `Copy`, so pushing `self.matrix` onto the `Vec` already
makes a copy. No extra function needed.

With that, let's draw a single filing cabinet drawer with a handle.
The drawer will be a large cube. The handle will be a small
cube. First we need to add the stack to our `Ctx`

```rust
struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
+    stack: &'a mut MatrixStack,
    view_projection_matrix: [f32; 16],
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    object_infos: &'a mut Vec<ObjectInfo>,
    object_ndx: usize,
    num_vertices: u32,
}
```

then

```rust
+const K_HANDLE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
+const K_DRAWER_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
+
+const K_DRAWER_SIZE: [f32; 3] = [40.0, 30.0, 50.0];
+const K_HANDLE_SIZE: [f32; 3] = [10.0, 2.0, 2.0];
+
+const K_WIDTH: usize = 0;
+const K_HEIGHT: usize = 1;
+const K_DEPTH: usize = 2;
+
+const K_HANDLE_POSITION: [f32; 3] = [
+    (K_DRAWER_SIZE[K_WIDTH] - K_HANDLE_SIZE[K_WIDTH]) / 2.0,
+    K_DRAWER_SIZE[K_HEIGHT] * 2.0 / 3.0,
+    K_HANDLE_SIZE[K_DEPTH],
+];
+
+fn draw_drawer(ctx: &mut Ctx) {
+    ctx.stack.save();
+    ctx.stack.scale(K_DRAWER_SIZE);
+    draw_object(ctx, ctx.stack.get(), K_DRAWER_COLOR);
+    ctx.stack.restore();
+
+    ctx.stack.save();
+    ctx.stack.translate(K_HANDLE_POSITION);
+    ctx.stack.scale(K_HANDLE_SIZE);
+    draw_object(ctx, ctx.stack.get(), K_HANDLE_COLOR);
+    ctx.stack.restore();
+}

+    let mut stack = MatrixStack::new();

  ...

    app.run(RenderMode::Once, move |frame: &Frame| {
    ...

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

            let mut ctx = Ctx {
                pass: &mut pass,
+                stack: &mut stack,
                view_projection_matrix,
                device: frame.device,
                queue: frame.queue,
                pipeline: &pipeline,
                object_infos: &mut object_infos,
                object_ndx: 0,
                num_vertices,
            };
+            ctx.stack.save();
+            ctx.stack.rotate_y(base_rotation);
+            ctx.stack.translate([K_DRAWER_SIZE[K_WIDTH] * -0.5, 0.0, 0.0]);
-            draw_object(&mut ctx, m4::rotation_y(base_rotation), [1.0, 1.0, 1.0, 1.0]);
+            draw_drawer(&mut ctx);
+            ctx.stack.restore();
```

The code above creates a `MatrixStack` and adds it to the
context (ctx) passed into `draw_drawer`. It uses this to
help us compute matrices. Instead of creating a rotation
matrix directly, we do it on the stack, then translate
half the width of the drawer so as to center it.

We pass the stack into `draw_drawer` which draws 2 cubes.
One it scales to the size of `K_DRAWER_SIZE`. The other it
positions to `K_HANDLE_POSITION` and scales to the size of
`K_HANDLE_SIZE`. Because it's using the matrix stack, both
will be relative to the rotation and translation already
on the stack.

The drawer cube is drawn with color `K_DRAWER_COLOR`, which is
white, and so will leave the vertex colors unchanged. 
The handle is drawn with color `K_HANDLE_COLOR`, which is 50% gray,
and so will draw the cube darker.

A minor tweak for the camera position:

```rust
-            let eye = [0.0, 2.0, 3.0];
-            let target = [0.0, 1.0, 0.0];
+            let eye = [0.0, 20.0, 100.0];
+            let target = [0.0, 20.0, 0.0];
            let up = [0.0, 1.0, 0.0];

            // Compute a view matrix
            let view_matrix = m4::look_at(eye, target, up);
```

That gives us a filing cabinet drawer.

{{{example url="../webgpu-matrix-stack-filing-drawer.html"}}}

You might be asking, why go through all this trouble of a
matrix stack? Let's draw a filing cabinet with 4 draws and
we'll see why.

```rust
  const K_HANDLE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
  const K_DRAWER_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
+const K_CABINET_COLOR: [f32; 4] = [0.75, 0.75, 0.75, 0.75];
+const K_NUM_DRAWERS_PER_CABINET: usize = 4;

  const K_DRAWER_SIZE: [f32; 3] = [40.0, 30.0, 50.0];
  const K_HANDLE_SIZE: [f32; 3] = [10.0, 2.0, 2.0];

  const K_WIDTH: usize = 0;
  const K_HEIGHT: usize = 1;
  const K_DEPTH: usize = 2;

  const K_HANDLE_POSITION: [f32; 3] = [
      (K_DRAWER_SIZE[K_WIDTH] - K_HANDLE_SIZE[K_WIDTH]) / 2.0,
      K_DRAWER_SIZE[K_HEIGHT] * 2.0 / 3.0,
      K_HANDLE_SIZE[K_DEPTH],
  ];

+const K_DRAWER_SPACING: f32 = K_DRAWER_SIZE[K_HEIGHT] + 3.0;

  fn draw_drawer(ctx: &mut Ctx) {
      ctx.stack.save();
      ctx.stack.scale(K_DRAWER_SIZE);
      draw_object(ctx, ctx.stack.get(), K_DRAWER_COLOR);
      ctx.stack.restore();

      ctx.stack.save();
      ctx.stack.translate(K_HANDLE_POSITION);
      ctx.stack.scale(K_HANDLE_SIZE);
      draw_object(ctx, ctx.stack.get(), K_HANDLE_COLOR);
      ctx.stack.restore();
  }

+fn draw_cabinet(ctx: &mut Ctx, num_drawers_per_cabinet: usize) {
+    let k_cabinet_size = [
+        K_DRAWER_SIZE[K_WIDTH] + 6.0,
+        K_DRAWER_SPACING * num_drawers_per_cabinet as f32 + 6.0,
+        K_DRAWER_SIZE[K_DEPTH] + 4.0,
+    ];
+
+    ctx.stack.save();
+    ctx.stack.scale(k_cabinet_size);
+    draw_object(ctx, ctx.stack.get(), K_CABINET_COLOR);
+    ctx.stack.restore();
+
+    for i in 0..num_drawers_per_cabinet {
+        ctx.stack.save();
+        ctx.stack
+            .translate([3.0, i as f32 * K_DRAWER_SPACING + 5.0, 1.0]);
+        draw_drawer(ctx);
+        ctx.stack.restore();
+    }
+}

    app.run(RenderMode::Once, move |frame: &Frame| {
    ...
-            let eye = [0.0, 20.0, 100.0];
-            let target = [0.0, 20.0, 0.0];
+            let eye = [0.0, 80.0, 200.0];
+            let target = [0.0, 80.0, 0.0];
            let up = [0.0, 1.0, 0.0];

            // Compute a view matrix
            let view_matrix = m4::look_at(eye, target, up);

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

    ...

            ctx.stack.save();
            ctx.stack.rotate_y(base_rotation);
            ctx.stack.translate([K_DRAWER_SIZE[K_WIDTH] * -0.5, 0.0, 0.0]);
-            draw_drawer(&mut ctx);
+            draw_cabinet(&mut ctx, K_NUM_DRAWERS_PER_CABINET);
            ctx.stack.restore();
```

Above, `draw_cabinet` draws a cube the size of
`k_cabinet_size` which is slightly taller than the number
of cabinets we ask it to draw.

It then just uses the matrix stack to translate each
drawer to appears at the correct position and slightly
in front of the cabinet cube.

{{{example url="../webgpu-matrix-stack-filing-cabinet.html"}}}

We didn't have to change `draw_drawer` at all. Because of
the matrix stack we were able to just use it as is.

Let's keep going. Let's draw multiple cabinets.

```rust
  const K_HANDLE_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];
  const K_DRAWER_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
  const K_CABINET_COLOR: [f32; 4] = [0.75, 0.75, 0.75, 0.75];
  const K_NUM_DRAWERS_PER_CABINET: usize = 4;
+const K_NUM_CABINETS: usize = 5;

  ...

  const K_DRAWER_SPACING: f32 = K_DRAWER_SIZE[K_HEIGHT] + 3.0;
+const K_CABINET_SPACING: f32 = K_DRAWER_SIZE[K_WIDTH] + 10.0;

  ...

+fn draw_cabinets(ctx: &mut Ctx, num_cabinets: usize) {
+    for i in 0..num_cabinets {
+        ctx.stack.save();
+        ctx.stack.translate([i as f32 * K_CABINET_SPACING, 0.0, 0.0]);
+        draw_cabinet(ctx, K_NUM_DRAWERS_PER_CABINET);
+        ctx.stack.restore();
+    }
+}

    app.run(RenderMode::Once, move |frame: &Frame| {
    ...
            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

    ...

            ctx.stack.save();
            ctx.stack.rotate_y(base_rotation);
-            ctx.stack.translate([K_DRAWER_SIZE[K_WIDTH] * -0.5, 0.0, 0.0]);
+            ctx.stack.translate([
+                (K_NUM_CABINETS as f32 - 0.5) * K_CABINET_SPACING * -0.5,
+                0.0,
+                0.0,
+            ]);
-            draw_cabinet(&mut ctx, K_NUM_DRAWERS_PER_CABINET);
+            draw_cabinets(&mut ctx, K_NUM_CABINETS);
            ctx.stack.restore();
```

Now we have `draw_cabinets` that just uses `draw_cabinet`
to draw however many cabinets we specify.

Back out in `render` we translate half the width of the
cabinets to center them.

{{{example url="../webgpu-matrix-stack-filing-cabinets.html"}}}

Hopefully this gives some idea of the usefulness of a matrix
stack. It lets us easily re-use things and/or position, orient,
and scale them.

## <a id="a-recursive-tree"></a> Recursive Tree

Let's make another example. Let's create a recursive tree out
of cubes. To do this we need a function that will add a "branch" of the
tree. We'll make it recursive and pass in `tree_depth`. If the
depth is > 0 then we will recursively add 2 more branches and pass
in one lower depth.

First the new settings, on the example page's JavaScript side

```js
  const degToRad = d => d * Math.PI / 180;

  const settings = {
    baseRotation: 0,
+    scale: 0.9,
+    rotationX: degToRad(20),
+    rotationY: degToRad(10),
  };

  const radToDegOptions = { min: -180, max: 180, step: 1, converters: GUI.converters.radToDeg };
+  const treeRadToDegOptions = { min: 0, max: 90, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
+  gui.add(settings, 'scale', 0.1, 1.2)
+     .onChange(v => wasm.set_setting_num('scale', v));
+  gui.add(settings, 'rotationX', treeRadToDegOptions)
+     .onChange(v => wasm.set_setting_num('rotationX', v));
+  gui.add(settings, 'rotationY', treeRadToDegOptions)
+     .onChange(v => wasm.set_setting_num('rotationY', v));
  gui.add(settings, 'baseRotation', radToDegOptions)
     .onChange(v => wasm.set_setting_num('baseRotation', v));
```

The tree drawing code needs to read those settings so we make a small
struct for them and add it to the `Ctx`.

```rust
+#[derive(Clone, Copy)]
+struct Settings {
+    scale: f32,
+    rotation_x: f32,
+    rotation_y: f32,
+}

struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
    stack: &'a mut MatrixStack,
+    settings: Settings,
    view_projection_matrix: [f32; 16],
    ...
```

and in the render code we read the current values

```rust
            let base_rotation = wgpu_fun::setting_f64("baseRotation", 0.0) as f32;
+            let settings = Settings {
+                scale: wgpu_fun::setting_f64("scale", 0.9) as f32,
+                rotation_x: wgpu_fun::setting_f64("rotationX", 20.0f64.to_radians()) as f32,
+                rotation_y: wgpu_fun::setting_f64("rotationY", 10.0f64.to_radians()) as f32,
+            };
```

Then the tree itself

```rust
+const K_TREE_DEPTH: usize = 6;
+const K_HEIGHT: usize = 1;
+// Moves the 1 unit cube so it's center above the origin so that when it scales
+// it scales out in x and z and up (y) from the origin
+const K_BRANCH_POSITION: [f32; 3] = [-0.5, 0.0, 0.5];
+const K_BRANCH_SIZE: [f32; 3] = [20.0, 150.0, 20.0];
+
+const K_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
+
+fn draw_branch(ctx: &mut Ctx) {
+    ctx.stack
+        .save()
+        .scale(K_BRANCH_SIZE)
+        .translate(K_BRANCH_POSITION);
+    draw_object(ctx, ctx.stack.get(), K_WHITE);
+    ctx.stack.restore();
+}
+
+fn draw_tree_level(ctx: &mut Ctx, offset: f32, tree_depth: usize) {
+    let s = if offset != 0.0 { ctx.settings.scale } else { 1.0 };
+    let y = if offset != 0.0 {
+        K_BRANCH_SIZE[K_HEIGHT]
+    } else {
+        0.0
+    };
+    ctx.stack
+        .save()
+        .translate([0.0, y, 0.0])
+        .rotate_z(offset * ctx.settings.rotation_x)
+        .rotate_y(offset.abs() * ctx.settings.rotation_y)
+        .scale([s, s, s]);
+
+    draw_branch(ctx);
+
+    if tree_depth > 0 {
+        draw_tree_level(ctx, -1.0, tree_depth - 1);
+        draw_tree_level(ctx, 1.0, tree_depth - 1);
+    }
+
+    ctx.stack.restore();
+}

    app.run(RenderMode::Once, move |frame: &Frame| {
    ...

-            let eye = [0.0, 80.0, 200.0];
-            let target = [0.0, 80.0, 0.0];
+            let eye = [0.0, 450.0, 1000.0];
+            let target = [0.0, 450.0, 0.0];
            let up = [0.0, 1.0, 0.0];

            // Compute a view matrix
            let view_matrix = m4::look_at(eye, target, up);

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);

    ...

            ctx.stack.save();
            ctx.stack.rotate_y(base_rotation);
-            ctx.stack.translate([
-                (K_NUM_CABINETS as f32 - 0.5) * K_CABINET_SPACING * -0.5,
-                0.0,
-                0.0,
-            ]);
-            draw_cabinets(&mut ctx, K_NUM_CABINETS);
+            draw_tree_level(&mut ctx, 0.0, K_TREE_DEPTH);
            ctx.stack.restore();
```

`draw_tree_level` uses our matrix stack. First it calls `save` to save the current
matrix. Then `translate`s it to move the branch to the end of the current
branch. If the `offset` is `0` it's the root so no translation needed.

The `offset` is then used to `rotate_z` the current branch either clockwise or
counter-clockwise. Because of the matrix stack it will be rotated relative to
the parent branch.

The `offset` is used again to `rotate_y` the branch. This time we use the
absolute value of `offset`. Feel free to remove the `.abs()` so see the
difference.

Finally we `scale` the branch, making each one smaller (or larger) than its
parent, except for the root, the branch with an `offset` of `0`.

We then call `draw_branch`. Draw branch draws a cube that is `K_BRANCH_SIZE` big.
It also translates the original unit cube so that the cube will be centered over
and above the origin. That way, when it scales, it will grow up (along the +Y
axis).

Then, if the depth > 0 we recursively call `draw_tree_level` to add 2 more
branches. One with an offset of `-1` and one with `+1`. Each branch will start
with the matrix on the stack and so will be positioned and oriented relative
to its parent.

Finally we `restore` the stack. 

{{{example url="../webgpu-matrix-stack-tree.html"}}}

Adjust "rotationX" and you'll see the branches fan out or bunch up.
Adjust "rotationY" and you'll see the branches spread out from the x-plane.
You may need to adjust "baseRotation" to see what's happening.
Adjust "scale" and you'll see each branch get smaller or larger than its
parent.

Maybe this could give you some inspiration to make an algorithmic tree generator. [^tree-gen]

[^tree-gen]: It would likely not be normal to generate a tree from individual
cubes or cylinders. The technique of recursion and a matrix stack would be used
but instead of drawing cubes we'd use the matrices to help generate vertices and
build a single mesh for the entire tree.

Let's add an ornament to each branch. Instead of using a cube, let's use a cone
for the ornament. Here's some code to generate cone vertices.

```rust
// tip is at origin, base is below
fn create_cone_vertices(radius: f32, height: f32, subdivisions: usize) -> (Vec<f32>, u32) {
    let mut positions: Vec<f32> = Vec::new();
    let mut colors: Vec<f32> = Vec::new();

    let mut add_vertex = |angle: f32, radius: f32, height: f32, color: &[f32; 3]| {
        let c = angle.cos();
        let s = angle.sin();
        positions.extend_from_slice(&[c * radius, height, s * radius]);
        colors.extend_from_slice(color);
    };

    for i in 0..subdivisions {
        let angle0 = (i + 0) as f32 / subdivisions as f32 * std::f32::consts::PI * 2.0;
        let angle1 = (i + 1) as f32 / subdivisions as f32 * std::f32::consts::PI * 2.0;

        let u = (i + 1) as f32 / subdivisions as f32;
        let color = [u * 128.0 + 127.0, 0.0, 0.0];

        // add side
        add_vertex(angle0, 0.0, 0.0, &color);
        add_vertex(angle1, radius, -height, &color);
        add_vertex(angle0, radius, -height, &color);

        // add top
        add_vertex(angle0, radius, -height, &color);
        add_vertex(angle1, radius, -height, &color);
        add_vertex(angle0, 0.0, -height, &color);
    }

    let num_vertices = positions.len() / 3;
    let mut vertex_data = vec![0.0f32; num_vertices * 4]; // xyz + color

    for i in 0..num_vertices {
        let position = &positions[i * 3..i * 3 + 3];
        vertex_data[i * 4..i * 4 + 3].copy_from_slice(position);

        let color = &colors[i * 3..i * 3 + 3];
        // set RGB in the first 3 bytes of the 4th float, set A to 255
        vertex_data[i * 4 + 3] =
            f32::from_ne_bytes([color[0] as u8, color[1] as u8, color[2] as u8, 255]);
    }

    (vertex_data, num_vertices as u32)
}
```

The code above walks around a circle and adds a triangle on each side and a
corresponding triangle on top. It sets each face to a shade of red. Like the
cube function it returns the vertex data and the number of vertices. We'll go
over [making various geometric primitives in another
article](webgpu-primitives.html).

Let's wrap our code that makes a vertex buffer into a function so we can call it
twice, once for the cube and once for the cone.

```rust
+struct Vertices {
+    vertex_buffer: wgpu::Buffer,
+    num_vertices: u32,
+}

-    let (vertex_data, num_vertices) = create_cube_vertices();
+fn create_vertices(
+    device: &wgpu::Device,
+    queue: &wgpu::Queue,
+    (vertex_data, num_vertices): (Vec<f32>, u32),
+    name: &str,
+) -> Vertices {
*    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
-        label: Some("vertex buffer vertices"),
+        label: Some(&format!("{name}: vertex buffer vertices")),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
+    Vertices {
+        vertex_buffer,
+        num_vertices,
+    }
*}

+    let cube_vertices = create_vertices(&app.device, &app.queue, create_cube_vertices(), "cube");
+    let ornament_vertices = create_vertices(
+        &app.device,
+        &app.queue,
+        create_cone_vertices(
+            20.0, // radius
+            60.0, // height
+            6,    // subdivisions
+        ),
+        "ornament",
+    );
```

Then let's update are `draw_object` function to take a vertices parameter.
The `Ctx` keeps references to both sets of vertices instead of a single
`num_vertices`.

```rust
struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
    stack: &'a mut MatrixStack,
    settings: Settings,
    view_projection_matrix: [f32; 16],
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    object_infos: &'a mut Vec<ObjectInfo>,
    object_ndx: usize,
-    num_vertices: u32,
+    cube_vertices: &'a Vertices,
+    ornament_vertices: &'a Vertices,
}

-fn draw_object(ctx: &mut Ctx, matrix: [f32; 16], color: [f32; 4]) {
+fn draw_object(ctx: &mut Ctx, vertices: &Vertices, matrix: [f32; 16], color: [f32; 4]) {
+    let Vertices {
+        vertex_buffer,
+        num_vertices,
+    } = vertices;
    if ctx.object_ndx == ctx.object_infos.len() {
        ctx.object_infos
            .push(create_object_info(ctx.device, ctx.pipeline));
    }
    let object_info = &mut ctx.object_infos[ctx.object_ndx];
    ctx.object_ndx += 1;

    let matrix_value = m4::multiply(&ctx.view_projection_matrix, &matrix);
    object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
        .copy_from_slice(&matrix_value);
    object_info.uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&color);

    // upload the uniform values to the uniform buffer
    ctx.queue.write_buffer(
        &object_info.uniform_buffer,
        0,
        bytemuck::cast_slice(&object_info.uniform_values),
    );

+    ctx.pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    ctx.pass.set_bind_group(0, &object_info.bind_group, &[]);
-    ctx.pass.draw(0..ctx.num_vertices, 0..1);
+    ctx.pass.draw(0..*num_vertices, 0..1);
}
```

and update the code that draws a branch to pass in the cube vertices

```rust
fn draw_branch(ctx: &mut Ctx) {
    ctx.stack
        .save()
        .scale(K_BRANCH_SIZE)
        .translate(K_BRANCH_POSITION);
-    draw_object(ctx, ctx.stack.get(), K_WHITE);
+    draw_object(ctx, ctx.cube_vertices, ctx.stack.get(), K_WHITE);
    ctx.stack.restore();
}
```

And we no longer need to set the vertex buffer early.

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {

    ...

            pass.set_pipeline(&pipeline);
-            pass.set_vertex_buffer(0, vertex_buffer.slice(..));

    ...
```

And then, let's add some code to `draw_tree_level` to draw an ornament when
depth equals zero.

```rust
fn draw_tree_level(ctx: &mut Ctx, offset: f32, tree_depth: usize) {
    let s = if offset != 0.0 { ctx.settings.scale } else { 1.0 };
    let y = if offset != 0.0 {
        K_BRANCH_SIZE[K_HEIGHT]
    } else {
        0.0
    };
    ctx.stack
        .save()
        .translate([0.0, y, 0.0])
        .rotate_z(offset * ctx.settings.rotation_x)
        .rotate_y(offset.abs() * ctx.settings.rotation_y)
        .scale([s, s, s]);

    draw_branch(ctx);

    if tree_depth > 0 {
        draw_tree_level(ctx, -1.0, tree_depth - 1);
        draw_tree_level(ctx, 1.0, tree_depth - 1);
    }

+    if tree_depth == 0 && offset > 0.0 {
+        let position = vec3::get_translation(&ctx.stack.get());
+        draw_object(
+            ctx,
+            ctx.ornament_vertices,
+            m4::translation(position),
+            K_WHITE,
+        );
+    }

    ctx.stack.restore();
}
```

We're using a function `vec3::get_translation` which we need to supply.

```rust
mod vec3 {
  ...
+    pub fn get_translation(m: &[f32; 16]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = m[12];
+        dst[1] = m[13];
+        dst[2] = m[14];
+
+        dst
+    }
}
```

`get_translation` gets the current translation from a matrix like we covered in
[the article on 3d math](webgpu-orthographic-projection.html).

Above, the code we added to draw an ornament, calls `get_translation` to get the
current translation of the matrix stack. This will be the base of the last
branch. We can not just draw an ornament directly from the matrix stack because
it would be oriented and scaled with the branch and we want the ornaments to
hang down. So, instead, we get the current translation from the stack and then
pass in a matrix with that translation. Because the translation is at the base
of the branch we only need to draw one which is why we only draw if `offset >
0`. Otherwise we'd draw 2 ornaments at the exact same location.

{{{example url="../webgpu-matrix-stack-tree-with-ornaments.html"}}}

Next Up, [Scene graphs](webgpu-scene-graphs.html).
