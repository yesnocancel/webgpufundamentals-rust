Title: WebGPU Timing Performance
Description: Timing operations in WebGPU
TOC: Timing Performance

Let's go over various things you might want
to time for performance. We'll time 3 things:

* The frame rate in frames per second (fps)
* The time spent on the CPU per frame (the original
  JavaScript examples call this "js" time and we'll keep
  that label so the numbers are easy to compare)
* The time spent on the GPU per frame

First, let's take a circle example from
[the article on vertex buffers](webgpu-vertex-buffers.html)
and lets animate them so we have something that's easy
to see changes in how much time things take.

In that example we had 3 vertex buffers. One was for
the positions and brightness of the vertices for a circle.
One was for things that are per instance but static
which included the circle's offset and color. And, the last
one was for things that change each time we render, in this
case it was the scale so we could keep the aspect ratio of
the circles correct so they stayed circles and not ellipses
as the user changed the size of the window.

We want to animate them moving so let's move the offset
to the same buffer as the scale. First we'll change the
render pipeline to move the offset to the same buffer
as the scale.

```rust
  let pipeline = app
    .device
    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("per vertex color"),
      layout: None,
      vertex: wgpu::VertexState {
        module: &module,
        entry_point: None,
        compilation_options: Default::default(),
        buffers: &[
          Some(wgpu::VertexBufferLayout {
            array_stride: 2 * 4 + 4, // 2 floats, 4 bytes each + 4 bytes
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
              // position
              wgpu::VertexAttribute {
                shader_location: 0,
                offset: 0,
                format: wgpu::VertexFormat::Float32x2,
              },
              // perVertexColor
              wgpu::VertexAttribute {
                shader_location: 4,
                offset: 8,
                format: wgpu::VertexFormat::Unorm8x4,
              },
            ],
          }),
          Some(wgpu::VertexBufferLayout {
-            array_stride: 4 + 2 * 4, // 4 bytes + 2 floats, 4 bytes each
+            array_stride: 4, // 4 bytes
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
              // color
              wgpu::VertexAttribute {
                shader_location: 1,
                offset: 0,
                format: wgpu::VertexFormat::Unorm8x4,
              },
-              // offset
-              wgpu::VertexAttribute {
-                shader_location: 2,
-                offset: 4,
-                format: wgpu::VertexFormat::Float32x2,
-              },
            ],
          }),
          Some(wgpu::VertexBufferLayout {
-            array_stride: 2 * 4, // 2 floats, 4 bytes each
+            array_stride: 4 * 4, // 4 floats, 4 bytes each
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
-              // scale
-              wgpu::VertexAttribute {
-                shader_location: 3,
-                offset: 0,
-                format: wgpu::VertexFormat::Float32x2,
-              },
+              // offset
+              wgpu::VertexAttribute {
+                shader_location: 2,
+                offset: 0,
+                format: wgpu::VertexFormat::Float32x2,
+              },
+              // scale
+              wgpu::VertexAttribute {
+                shader_location: 3,
+                offset: 8,
+                format: wgpu::VertexFormat::Float32x2,
+              },
            ],
          }),
        ],
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

Then we'll change the part that sets up the vertex buffers
to move the offsets together with the scales. Each object
now also gets a velocity, and since the offset changes every
frame it moves from the static data into `ObjectInfo`.

```rust
struct ObjectInfo {
  scale: f32,
+  offset: [f32; 2],
+  velocity: [f32; 2],
}
```

```rust
  // create 2 vertex buffers
-  let static_unit_size = 4 + // color is 4 bytes
-    2 * 4; // offset is 2 32bit floats (4bytes each)
-  let changing_unit_size = 2 * 4; // scale is 2 32bit floats (4bytes each)
+  let static_unit_size = 4; // color is 4 bytes
+  let changing_unit_size = 2 * 4 + // offset is 2 32bit floats (4bytes each)
+    2 * 4; // scale is 2 32bit floats (4bytes each)
  let static_vertex_buffer_size = static_unit_size * k_num_objects;
  let changing_vertex_buffer_size = changing_unit_size * k_num_objects;

  let static_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("static vertex for objects"),
    size: static_vertex_buffer_size as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  let changing_vertex_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("changing storage for objects"),
    size: changing_vertex_buffer_size as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });

  // offsets to the various uniform values in float32 indices
  let k_color_offset = 0;
-  let k_offset_offset = 1;
+
-  let k_scale_offset = 0;
+  let k_offset_offset = 0;
+  let k_scale_offset = 2;

  {
-    let mut static_vertex_values_f32 = vec![0.0f32; static_vertex_buffer_size / 4];
+    let mut static_vertex_values_u8 = vec![0u8; static_vertex_buffer_size];
    for i in 0..k_num_objects {
      let static_offset_u8 = i * static_unit_size;
-      let static_offset_f32 = static_offset_u8 / 4;

      // These are only set once so set them now
-      // a u8 view of the same data as static_vertex_values_f32
-      let static_vertex_values_u8: &mut [u8] =
-        bytemuck::cast_slice_mut(&mut static_vertex_values_f32);
      static_vertex_values_u8[static_offset_u8 + k_color_offset..][..4].copy_from_slice(&[
        (rand(0.0, 1.0) * 255.0) as u8,
        (rand(0.0, 1.0) * 255.0) as u8,
        (rand(0.0, 1.0) * 255.0) as u8,
        255,
      ]); // set the color

-      static_vertex_values_f32[static_offset_f32 + k_offset_offset..][..2]
-        .copy_from_slice(&[rand(-0.9, 0.9), rand(-0.9, 0.9)]); // set the offset

      object_infos.push(ObjectInfo {
        scale: rand(0.2, 0.5),
+        offset: [rand(-0.9, 0.9), rand(-0.9, 0.9)],
+        velocity: [rand(-0.1, 0.1), rand(-0.1, 0.1)],
      });
    }
-    app.queue.write_buffer(
-      &static_vertex_buffer,
-      0,
-      bytemuck::cast_slice(&static_vertex_values_f32),
-    );
+    app.queue
+      .write_buffer(&static_vertex_buffer, 0, &static_vertex_values_u8);
  }
```

At render time we can update the offsets of the circles based on their velocity and then upload those to the GPU.

```rust
+  let euclidean_modulo = |x: f32, a: f32| x - a * (x / a).floor();

+  let mut then = 0.0;
-  app.run(RenderMode::Once, move |frame: &Frame| {
+  app.run(RenderMode::Continuous, move |frame: &Frame| {
+    let now = frame.time;
+    let delta_time = (now - then) as f32;
+    then = now;

...
      // set the scales for each object
-    for (ndx, ObjectInfo { scale }) in object_infos.iter().enumerate() {
+    for ndx in 0..object_infos.len() {
+      let ObjectInfo {
+        scale,
+        offset,
+        velocity,
+      } = &mut object_infos[ndx];
+      // -1.5 to 1.5
+      offset[0] = euclidean_modulo(offset[0] + velocity[0] * delta_time + 1.5, 3.0) - 1.5;
+      offset[1] = euclidean_modulo(offset[1] + velocity[1] * delta_time + 1.5, 3.0) - 1.5;

+      let off = ndx * (changing_unit_size / 4);
+      vertex_values[off + k_offset_offset..][..2].copy_from_slice(offset);
+      vertex_values[off + k_scale_offset..][..2]
+        .copy_from_slice(&[*scale / aspect, *scale]);
    }

...

  });
```

We also switched to a rAF loop[^rAF] by passing
`RenderMode::Continuous` — our helper's render loop calls our
frame function via `requestAnimationFrame` in the browser, and
`frame.time` gives us the time in seconds, so there's nothing
left to convert. (In the JavaScript version this is where
you'd call `requestAnimationFrame(render)` yourself and
multiply the passed-in milliseconds by 0.001.)

[^rAF]: `rAF` is short for `requestAnimationFrame`

<a id="a-euclidianModulo"></a>The code above uses `euclidean_modulo` to update the offset.
`euclidean_modulo` returns the remainder of a division where
the remainder is always positive, whereas Rust's `%` operator returns the remainder in the same direction as the value.
For example

<div class="webgpu_center">
  <div class="center">
    <div class="data-table center" data-table='{
  "cols": ["value", "% operator", "euclideanModulo"],
  "classNames": ["a", "b", "c"],
  "rows": [
    [ "0.3", "0.3", "0.3" ],
    [ "2.3", "0.3", "0.3" ],
    [ "4.3", "0.3", "0.3" ],
    [ "-1.7", "-1.7", "0.3" ],
    [ "-3.7", "-1.7", "0.3" ]
  ]
}'>
     </div>
  </div>
  <div>modulo 2 of % vs euclideanModulo</div>
</div>

To put it another way, here's a graph of the `%` operator vs `euclideanModulo`

<div class="webgpu_center">
  <img style="width: 700px" src="resources/euclidean-modulo.svg">
  <div>euclideanModule(v, 2)</div>
</div>
<div class="webgpu_center">
  <img  style="width: 700px" src="resources/modulo.svg">
  <div>v % 2</div>
</div>

So, the code above takes the offset, which is in clip space, and adds 1.5. It then takes the `euclidean_modulo`
by 3 which will give us a number that is wrapped between 0.0 and 3.0
and then subtracts 1.5.  This gives us numbers
that stay between -1.5 and +1.5 and lets them wrap
around to the other side. We use -1.5 to +1.5 so that
the circles don't wrap until they are off the screen. [^offscreen]

[^offscreen]: This only works if the radius of the circle is less than 0.5
but it seemed best not to bloat the code with complicated checks for size.

To give us something to adjust, lets make it so we can
set how many circles to draw. Like the original JavaScript
examples, our page keeps a settings GUI in page JavaScript;
its onChange handler calls into the wasm module, and our
frame code reads the current value with
`wgpu_fun::setting_f64` (natively there's no panel and the
default is used).

```rust
-  let k_num_objects = 100;
+  let k_num_objects = 10000;


...

    // read the settings the GUI on the page sets
+    let num_objects = wgpu_fun::setting_f64("numObjects", 100.0) as usize;

  ...

    // set the scale and offset for each object
-    for ndx in 0..object_infos.len() {
+    for ndx in 0..num_objects {
      let ObjectInfo {
        scale,
        offset,
        velocity,
      } = &mut object_infos[ndx];

      // -1.5 to 1.5
      offset[0] = euclidean_modulo(offset[0] + velocity[0] * delta_time + 1.5, 3.0) - 1.5;
      offset[1] = euclidean_modulo(offset[1] + velocity[1] * delta_time + 1.5, 3.0) - 1.5;

      let off = ndx * (changing_unit_size / 4);
      vertex_values[off + k_offset_offset..][..2].copy_from_slice(offset);
      vertex_values[off + k_scale_offset..][..2]
        .copy_from_slice(&[*scale / aspect, *scale]);
    }

    // upload all offsets and scales at once
-    frame.queue.write_buffer(
-      &changing_vertex_buffer,
-      0,
-      bytemuck::cast_slice(&vertex_values),
-    );
+    frame.queue.write_buffer(
+      &changing_vertex_buffer,
+      0,
+      bytemuck::cast_slice(&vertex_values[..num_objects * (changing_unit_size / 4)]),
+    );

-    pass.draw(0..num_vertices, 0..k_num_objects as u32);
+    pass.draw(0..num_vertices, 0..num_objects as u32);
```

So now we should have something that animates
and we can adjust how much work is done by setting
the number of circles.

{{{example url="../webgpu-timing-animated.html"}}}

To that, let's add frames per second (fps) and
time spent on the CPU

First we need a way to display this info so lets
add an `<pre>` element positioned on top of the canvas.

```html
  <body>
    <canvas></canvas>
+    <pre id="info"></pre>
  </body>
```

```css
html, body {
  margin: 0;       /* remove the default margin          */
  height: 100%;    /* make the html,body fill the page   */
}
canvas {
  display: block;  /* make the canvas act like a block   */
  width: 100%;     /* make the canvas fill its container */
  height: 100%;
}
+#info {
+  position: absolute;
+  top: 0;
+  left: 0;
+  margin: 0;
+  padding: 0.5em;
+  background-color: rgba(0, 0, 0, 0.8);
+  color: white;
+}
```

Our Rust code fills that element with
`wgpu_fun::set_info_text` (a tiny helper: in the browser it
sets the text of `#info`; natively it prints to stdout, at
most about once a second).

We already have the data needed to display
frames per second. It's the `delta_time` we
computed above.

For CPU time, we can record the time
our frame callback started and the
time it ended. JavaScript uses `performance.now()`
for this, which returns milliseconds on a monotonic
clock; `wgpu_fun::now_ms` is the same thing
(`performance.now()` in the browser, `std::time::Instant`
natively).

```rust
  let mut then = 0.0;
  app.run(RenderMode::Continuous, move |frame: &Frame| {
    let now = frame.time;
    let delta_time = (now - then) as f32;
    then = now;

+    let start_time = wgpu_fun::now_ms();

    ...

+    let js_time = wgpu_fun::now_ms() - start_time;

+    wgpu_fun::set_info_text(&format!(
+      "\
+fps: {:.1}
+js: {:.1}ms
+",
+      1.0 / delta_time,
+      js_time,
+    ));
  });
```

And that gives us our first two timing measurements.

{{{example url="../webgpu-timing-with-fps-js-time.html"}}}

## <a id="a-timestamp-query"></a> Timing the GPU

WebGPU provides an **optional** `'timestamp-query'` feature for checking how long an operation takes on the GPU.
Since it's an optional feature we need to see if it
exists and request it like we covered in [the article on limits and features](webgpu-limits-and-features.html).
In wgpu the feature is `wgpu::Features::TIMESTAMP_QUERY`, and
the JS pattern of checking `adapter.features.has(...)` before
requiring the feature on the device is what
`App::new_with_features` does for us: it requests the given
optional features only if the adapter supports them.

```rust
async fn run() {
-  let mut app = App::new("WebGPU Timing - Step 2 - FPS/JS Time").await;
+  // ask for the timestamp-query feature if the adapter supports it
+  let mut app = App::new_with_features(
+    "WebGPU Timing - w/timestamp",
+    wgpu::Features::TIMESTAMP_QUERY,
+  )
+  .await;
  app.auto_resize = true;
+  let can_timestamp = app
+    .device
+    .features()
+    .contains(wgpu::Features::TIMESTAMP_QUERY);
```

Above, we set `can_timestamp` to true or false based on if the device ended up
with the `TIMESTAMP_QUERY` feature, which it only does if the adapter supports
it.

With the feature enabled we can ask WebGPU for *timestamps* for a render pass or
compute pass. You do this by making a `QuerySet` and adding it to your
compute or render pass. A `QuerySet` is effectively an array of query
results. You tell WebGPU which element in the array to record the time the pass started
and which element in the array to record when the pass ended. You can then copy those
timestamps to a buffer and map the buffer to read the results.[^mapping-not-necessary]

[^mapping-not-necessary]: Copying the query results to mappable buffer is only for
the purpose of reading the values from Rust. If your use-case only needs the
results to stay on the GPU, for example as input to something else, then you don't need
to copy the results to a mappable buffer.

So, first we create a query set.

```rust
  let query_set = app.device.create_query_set(&wgpu::QuerySetDescriptor {
    label: None,
    ty: wgpu::QueryType::Timestamp,
    count: 2,
  });
```

We need count to be at least 2 so we can write
both a start and end timestamp.

We need a buffer to convert the querySet info
into data we can access.

```rust
  let resolve_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: query_set.count() as u64 * 8,
    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
    mapped_at_creation: false,
  });
```

Each element in a querySet takes 8 bytes.
We need to give it a usage of `QUERY_RESOLVE`
and, if we want be able to read the results
back in Rust we need the `COPY_SRC` usage
so we can copy the result to a mappable buffer.

Finally we create a mappable buffer to read the
results.

```rust
  let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: resolve_buffer.size(),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
```

We need to wrap this code in a way that only
creates these things if the feature exists, otherwise we'll
get an error trying to make a `Timestamp` querySet. In Rust
that's what `Option` is for: `bool::then` gives us a `Some`
with all three resources when `can_timestamp` is true and a
`None` when it isn't.

```rust
+  let query_resources = can_timestamp.then(|| {
    let query_set = app.device.create_query_set(&wgpu::QuerySetDescriptor {
      label: None,
      ty: wgpu::QueryType::Timestamp,
      count: 2,
    });
    let resolve_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: query_set.count() as u64 * 8,
      usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
      mapped_at_creation: false,
    });
    let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: None,
      size: resolve_buffer.size(),
      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
      mapped_at_creation: false,
    });
+    (query_set, resolve_buffer, result_buffer)
+  });
```

In our render pass descriptor we tell it the
querySet to use and the index of the elements
in the querySet to write the start and ending
timestamps.

```rust
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass with timing"),
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
+        timestamp_writes: query_resources.as_ref().map(|(query_set, _, _)| {
+          wgpu::RenderPassTimestampWrites {
+            query_set,
+            beginning_of_pass_write_index: Some(0),
+            end_of_pass_write_index: Some(1),
+          }
+        }),
        ..Default::default()
      });
```

Above, if the feature exists, `query_resources` is a `Some` and we map it to a
`timestamp_writes` section for our render pass descriptor, passing in the
querySet and telling it to write the start to
element 0 of the set and the end to element 1. If it doesn't exist the `map`
produces `None` and no timestamps are written.

After we end the pass, we need to call `resolve_query_set`. This takes the results
of the query and puts them in a buffer. We pass it the querySet, the range
in the query set to resolve, a
buffer to resolve to, and an offset in that buffer where to store the result.

```rust
    } // the pass ends when it drops here

+    if let Some((query_set, resolve_buffer, result_buffer)) = &query_resources {
+      encoder.resolve_query_set(query_set, 0..query_set.count(), resolve_buffer, 0);
+    }
```

We also want to copy the `resolve_buffer` to our `result_buffer` so we can map it
and look at the results in Rust. We have an issue though. We can not copy
to our `result_buffer` while it's mapped. In JavaScript, buffers have a
`mapState` property you can check for this. wgpu buffers don't expose one, so
we track it ourselves: an `AtomicBool` that is true whenever the buffer is
in the JS `'unmapped'` state — the value it starts with — and false from the
moment we call `map_async` until we `unmap` it. (It's atomic and wrapped in
an `Arc` because, as we'll see below, the map callback that flips it back
may run outside our frame function.)

```rust
+  // wgpu buffers have no JS-style `mapState` property, so we track
+  // whether resultBuffer is 'unmapped' (safe to copy to / map) ourselves.
+  let result_buffer_unmapped = Arc::new(AtomicBool::new(true));
```

```rust
    if let Some((query_set, resolve_buffer, result_buffer)) = &query_resources {
      encoder.resolve_query_set(query_set, 0..query_set.count(), resolve_buffer, 0);
+      if result_buffer_unmapped.load(Ordering::Relaxed) {
+        encoder.copy_buffer_to_buffer(
+          resolve_buffer,
+          0,
+          result_buffer,
+          0,
+          result_buffer.size(),
+        );
+      }
    }
```

After we've submitted the command buffer we can map the `result_buffer`. Like
above, we only want to map it if it's unmapped.

```rust
+  let gpu_time = Arc::new(Mutex::new(0.0f64));

   ...

   app.run(RenderMode::Continuous, move |frame: &Frame| {

    ...

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);

+    if let Some((_, _, result_buffer)) = &query_resources {
+      if result_buffer_unmapped.load(Ordering::Relaxed) {
+        result_buffer_unmapped.store(false, Ordering::Relaxed);
+        let result_buffer = result_buffer.clone();
+        let result_buffer_unmapped = result_buffer_unmapped.clone();
+        let gpu_time = gpu_time.clone();
+        result_buffer.clone().map_async(wgpu::MapMode::Read, .., move |result| {
+          result.expect("failed to map result buffer");
+          {
+            let view = result_buffer.slice(..).get_mapped_range().unwrap();
+            let times: &[i64] = bytemuck::cast_slice(&view);
+            *gpu_time.lock().unwrap() =
+              (times[1] - times[0]) as f64 * timestamp_period;
+          }
+          result_buffer.unmap();
+          result_buffer_unmapped.store(true, Ordering::Relaxed);
+        });
+      }
+    }
+    // mapAsync results are delivered when the device is polled; the
+    // browser does that for us, natively we poll once per frame.
+    let _ = frame.device.poll(wgpu::PollType::Poll);
```

This is the Rust translation of JavaScript's
`resultBuffer.mapAsync(GPUMapMode.READ).then(...)`: `map_async` takes a
callback that runs when the mapping is complete, some frames later. There's
one platform difference: in the browser, the browser delivers those callbacks
on its own; natively they're delivered when we poll the device, so we add a
non-blocking `device.poll` once per frame (it's a no-op on the web).

In WebGPU, query set results are in nanoseconds and are stored in 64bit
integers. wgpu is a little lower-level: on native, timestamps are in GPU
"ticks", and `queue.get_timestamp_period()` tells us how many nanoseconds
one tick is (on the web it's always 1.0). So we grab that once, at init
time:

```rust
+  // timestamps are in GPU ticks; this many nanoseconds each (1.0 on the web)
+  let timestamp_period = app.queue.get_timestamp_period() as f64;
```

To read the timestamps, where JavaScript needs a `BigUint64Array` view of the
mapped data, we `bytemuck::cast_slice` the mapped bytes to a `&[i64]`. Note:
*signed* 64bit integers — it's legal for a query's beginning time to be
greater than its end time, and subtracting two `u64`s would panic in that
case, just like JavaScript first subtracts the two `bigint`s before
converting to a `number` to avoid losing precision. We subtract, convert to
`f64` and multiply by the timestamp period to get nanoseconds.

In the code above, we are are only copying the results to `result_buffer` some
times, when it's not mapped. That means we'll only be reading the time on some
frames. Most likely every other frame but there is no strict guarantee how long
it will take until the map completes. Because of that, we update `gpu_time` —
shared between the frame function and the map callback via `Arc<Mutex<f64>>` —
which we can use at anytime to get the last recorded time.

```rust
    wgpu_fun::set_info_text(&format!(
      "\
fps: {:.1}
js: {:.1}ms
+gpu: {}
",
      1.0 / delta_time,
      js_time,
+      if can_timestamp {
+        format!("{:.1}µs", *gpu_time.lock().unwrap() / 1000.0)
+      } else {
+        "N/A".to_string()
+      },
    ));
```

And with that we get a GPU time from WebGPU

{{{example url="../webgpu-timing-with-timestamp.html"}}}

For me, the numbers change too often to see anything
useful. One way to fix that is to compute a rolling
average. Here's a struct to help compute a rolling
average.

```rust
// Note: We disallow negative values as this is used for timestamp queries
// where it's possible for a query to return a beginning time greater than the
// end time. See: https://gpuweb.github.io/gpuweb/#timestamp
struct NonNegativeRollingAverage {
  total: f64,
  samples: Vec<f64>,
  cursor: usize,
  num_samples: usize,
}

impl NonNegativeRollingAverage {
  fn new() -> Self {
    Self {
      total: 0.0,
      samples: Vec::new(),
      cursor: 0,
      num_samples: 30,
    }
  }

  fn add_sample(&mut self, v: f64) {
    if !v.is_nan() && v.is_finite() && v >= 0.0 {
      if self.samples.len() <= self.cursor {
        self.samples.push(0.0);
      }
      self.total += v - self.samples[self.cursor];
      self.samples[self.cursor] = v;
      self.cursor = (self.cursor + 1) % self.num_samples;
    }
  }

  fn get(&self) -> f64 {
    self.total / self.samples.len() as f64
  }
}
```

It keeps an array of values and a total. When a new value is added the
oldest value is subtracted from the total as the new value is added.

We can use it like this.

```rust
+  let mut fps_average = NonNegativeRollingAverage::new();
+  let mut js_average = NonNegativeRollingAverage::new();
+  let gpu_average = Arc::new(Mutex::new(NonNegativeRollingAverage::new()));

  app.run(RenderMode::Continuous, move |frame: &Frame| {
  ...

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);

    if let Some((_, _, result_buffer)) = &query_resources {
      if result_buffer_unmapped.load(Ordering::Relaxed) {
        result_buffer_unmapped.store(false, Ordering::Relaxed);
        let result_buffer = result_buffer.clone();
        let result_buffer_unmapped = result_buffer_unmapped.clone();
        let gpu_time = gpu_time.clone();
+        let gpu_average = gpu_average.clone();
        result_buffer.clone().map_async(wgpu::MapMode::Read, .., move |result| {
          result.expect("failed to map result buffer");
          {
            let view = result_buffer.slice(..).get_mapped_range().unwrap();
            let times: &[i64] = bytemuck::cast_slice(&view);
-            *gpu_time.lock().unwrap() =
-              (times[1] - times[0]) as f64 * timestamp_period;
+            let time = (times[1] - times[0]) as f64 * timestamp_period;
+            *gpu_time.lock().unwrap() = time;
+            gpu_average.lock().unwrap().add_sample(time / 1000.0);
          }
          result_buffer.unmap();
          result_buffer_unmapped.store(true, Ordering::Relaxed);
        });
      }
    }
    // mapAsync results are delivered when the device is polled; the
    // browser does that for us, natively we poll once per frame.
    let _ = frame.device.poll(wgpu::PollType::Poll);

    let js_time = wgpu_fun::now_ms() - start_time;

+    fps_average.add_sample(1.0 / delta_time as f64);
+    js_average.add_sample(js_time);

    wgpu_fun::set_info_text(&format!(
      "\
fps: {:.1}
js: {:.1}ms
gpu: {}
",
-      1.0 / delta_time,
-      js_time,
-      if can_timestamp {
-        format!("{:.1}µs", *gpu_time.lock().unwrap() / 1000.0)
-      } else {
-        "N/A".to_string()
-      },
+      fps_average.get(),
+      js_average.get(),
+      if can_timestamp {
+        format!("{:.1}µs", gpu_average.lock().unwrap().get())
+      } else {
+        "N/A".to_string()
+      },
    ));
  });
```

And now the numbers are a little more stable.

{{{example url="../webgpu-timing-with-timestamp-w-average.html"}}}

## <a id="a-timing-helper"></a> Using a helper

For me, I find all of this a little tedious and probably easy to get something
wrong. We had to make 3 things, a querySet and 2 buffers. We had to change our
renderPassDescriptor. We had to resolve the results and copy to a mappable
buffer.

One way to make this less tedious would be to make a struct that helps us do the
timing. Here's one example of a helper that might help with some of these issues.

The original JavaScript helper does two things Rust can't do: it *replaces*
`pass.end` and `encoder.finish` at runtime so resolving happens automatically,
and it patches `GPUQueue.prototype.submit` to track unsubmitted command
buffers so it can complain if you read a result before submitting. In Rust
there's no monkeypatching, and a pass "ends" by being dropped, so our version
makes the resolve step an explicit call and keeps the same state machine and
assertions for everything else.

```rust
// See https://webgpufundamentals.org/webgpu/lessons/webgpu-timing.html
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
  Free,
  NeedResolve,
  WaitForResult,
}

struct TimingHelper {
  can_timestamp: bool,
  device: wgpu::Device,
  // timestamps are in GPU ticks; this many nanoseconds each (1.0 on the web)
  timestamp_period: f64,
  query_set: Option<wgpu::QuerySet>,
  resolve_buffer: Option<wgpu::Buffer>,
  result_buffer: Option<wgpu::Buffer>,
  result_buffers: Arc<Mutex<Vec<wgpu::Buffer>>>,
  // state can be Free, NeedResolve, WaitForResult
  state: State,
}

impl TimingHelper {
  fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
    let can_timestamp = device
      .features()
      .contains(wgpu::Features::TIMESTAMP_QUERY);
    let (query_set, resolve_buffer) = if can_timestamp {
      let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: None,
        ty: wgpu::QueryType::Timestamp,
        count: 2,
      });
      let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: query_set.count() as u64 * 8,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
      });
      (Some(query_set), Some(resolve_buffer))
    } else {
      (None, None)
    };
    Self {
      can_timestamp,
      device: device.clone(),
      timestamp_period: queue.get_timestamp_period() as f64,
      query_set,
      resolve_buffer,
      result_buffer: None,
      result_buffers: Arc::new(Mutex::new(Vec::new())),
      state: State::Free,
    }
  }

  fn begin_render_pass<'encoder>(
    &mut self,
    encoder: &'encoder mut wgpu::CommandEncoder,
    descriptor: &wgpu::RenderPassDescriptor<'_>,
  ) -> wgpu::RenderPass<'encoder> {
    if self.can_timestamp {
      assert!(self.state == State::Free, "state not free");
      self.state = State::NeedResolve;

      encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
          query_set: self.query_set.as_ref().unwrap(),
          beginning_of_pass_write_index: Some(0),
          end_of_pass_write_index: Some(1),
        }),
        ..descriptor.clone()
      })
    } else {
      encoder.begin_render_pass(descriptor)
    }
  }

  // In JS this runs automatically when pass.end() is called. In Rust a
  // pass ends when it's dropped, so call this right after.
  fn resolve_timing(&mut self, encoder: &mut wgpu::CommandEncoder) {
    if !self.can_timestamp {
      return;
    }
    assert!(
      self.state == State::NeedResolve,
      "you must use timing_helper.begin_render_pass or timing_helper.begin_compute_pass",
    );
    self.state = State::WaitForResult;

    let query_set = self.query_set.as_ref().unwrap();
    let resolve_buffer = self.resolve_buffer.as_ref().unwrap();
    let result_buffer = self.result_buffers.lock().unwrap().pop().unwrap_or_else(|| {
      self.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: resolve_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
      })
    });

    encoder.resolve_query_set(query_set, 0..query_set.count(), resolve_buffer, 0);
    encoder.copy_buffer_to_buffer(resolve_buffer, 0, &result_buffer, 0, result_buffer.size());
    self.result_buffer = Some(result_buffer);
  }

  // In JS this is async and returns the duration; in Rust the mapping
  // completes through a callback, so we pass the duration (in
  // nanoseconds) to a callback. Call after submitting the command buffer.
  fn get_result(&mut self, callback: impl FnOnce(f64) + Send + 'static) {
    if !self.can_timestamp {
      callback(0.0);
      return;
    }
    assert!(
      self.state == State::WaitForResult,
      "you must call resolve_timing and submit the command buffer before you can read the result",
    );
    self.state = State::Free;

    let result_buffer = self.result_buffer.take().unwrap();
    let result_buffers = self.result_buffers.clone();
    let timestamp_period = self.timestamp_period;
    result_buffer.clone().map_async(wgpu::MapMode::Read, .., move |result| {
      result.expect("failed to map result buffer");
      let duration = {
        let view = result_buffer.slice(..).get_mapped_range().unwrap();
        let times: &[i64] = bytemuck::cast_slice(&view);
        (times[1] - times[0]) as f64 * timestamp_period
      };
      result_buffer.unmap();
      result_buffers.lock().unwrap().push(result_buffer);
      callback(duration);
    });
  }
}
```

The asserts are there to helps us not use this struct wrong. For example if we
begin a pass but don't resolve it or, if we resolve it and try to read the result
twice.

Some notes on the mechanics: instead of a single `result_buffer` that we
skip on frames where it's still mapped, the helper keeps a free-list of
result buffers (`result_buffers`) and takes — or creates — one per pass, so
every pass gets timed. The list is behind `Arc<Mutex<...>>` because the map
callback, which returns a buffer to the list, may run outside our frame
function. And since wgpu timestamps are in GPU ticks, the helper takes the
`Queue` in `new` so it can grab `get_timestamp_period()` and hand results
back already converted to nanoseconds like the JavaScript version.

With this struct, we can remove much of the code we had before. 

```rust
async fn run() {
  // ask for the timestamp-query feature if the adapter supports it
  let mut app = App::new_with_features(
    "WebGPU Timing - w/TimingHelper",
    wgpu::Features::TIMESTAMP_QUERY,
  )
  .await;
  app.auto_resize = true;
  let can_timestamp = app
    .device
    .features()
    .contains(wgpu::Features::TIMESTAMP_QUERY);

+  let mut timing_helper = TimingHelper::new(&app.device, &app.queue);

  ...

-  let query_resources = can_timestamp.then(|| {
-    let query_set = app.device.create_query_set(&wgpu::QuerySetDescriptor {
-      label: None,
-      ty: wgpu::QueryType::Timestamp,
-      count: 2,
-    });
-    let resolve_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-      label: None,
-      size: query_set.count() as u64 * 8,
-      usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
-      mapped_at_creation: false,
-    });
-    let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-      label: None,
-      size: resolve_buffer.size(),
-      usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
-      mapped_at_creation: false,
-    });
-    (query_set, resolve_buffer, result_buffer)
-  });
-
-  // wgpu buffers have no JS-style `mapState` property, so we track
-  // whether resultBuffer is 'unmapped' (safe to copy to / map) ourselves.
-  let result_buffer_unmapped = Arc::new(AtomicBool::new(true));
-
-  // timestamps are in GPU ticks; this many nanoseconds each (1.0 on the web)
-  let timestamp_period = app.queue.get_timestamp_period() as f64;
-
-  let gpu_time = Arc::new(Mutex::new(0.0f64));

  ...

  app.run(RenderMode::Continuous, move |frame: &Frame| {

    ...

-      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
-        label: Some("our basic canvas renderPass with timing"),
+      let mut pass = timing_helper.begin_render_pass(&mut encoder, &wgpu::RenderPassDescriptor {
+        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          ...
        })],
-        timestamp_writes: query_resources.as_ref().map(|(query_set, _, _)| {
-          wgpu::RenderPassTimestampWrites {
-            query_set,
-            beginning_of_pass_write_index: Some(0),
-            end_of_pass_write_index: Some(1),
-          }
-        }),
        ..Default::default()
      });

    ...

    } // the pass ends when it drops here

-    if let Some((query_set, resolve_buffer, result_buffer)) = &query_resources {
-      encoder.resolve_query_set(query_set, 0..query_set.count(), resolve_buffer, 0);
-      if result_buffer_unmapped.load(Ordering::Relaxed) {
-        encoder.copy_buffer_to_buffer(
-          resolve_buffer,
-          0,
-          result_buffer,
-          0,
-          result_buffer.size(),
-        );
-      }
-    }
+    timing_helper.resolve_timing(&mut encoder);

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);

-    if let Some((_, _, result_buffer)) = &query_resources {
-      if result_buffer_unmapped.load(Ordering::Relaxed) {
-        result_buffer_unmapped.store(false, Ordering::Relaxed);
-        let result_buffer = result_buffer.clone();
-        let result_buffer_unmapped = result_buffer_unmapped.clone();
-        let gpu_time = gpu_time.clone();
-        let gpu_average = gpu_average.clone();
-        result_buffer.clone().map_async(wgpu::MapMode::Read, .., move |result| {
-          result.expect("failed to map result buffer");
-          {
-            let view = result_buffer.slice(..).get_mapped_range().unwrap();
-            let times: &[i64] = bytemuck::cast_slice(&view);
-            let time = (times[1] - times[0]) as f64 * timestamp_period;
-            *gpu_time.lock().unwrap() = time;
-            gpu_average.lock().unwrap().add_sample(time / 1000.0);
-          }
-          result_buffer.unmap();
-          result_buffer_unmapped.store(true, Ordering::Relaxed);
-        });
-      }
-    }
+    {
+      let gpu_average = gpu_average.clone();
+      timing_helper.get_result(move |gpu_time| {
+        gpu_average.lock().unwrap().add_sample(gpu_time / 1000.0);
+      });
+    }
    // mapAsync results are delivered when the device is polled; the
    // browser does that for us, natively we poll once per frame.
    let _ = frame.device.poll(wgpu::PollType::Poll);

    ...
```

{{{example url="../webgpu-timing-with-timing-helper.html"}}}

A few points about the `TimingHelper` struct:

* You still have to manually request the `TIMESTAMP_QUERY` feature when you
  create your device but, the struct handles whether it exists or not on the
  device.

* When you call `timing_helper.begin_render_pass` (or a
  `begin_compute_pass` you'd write the same way) it automatically adds the
  appropriate `timestamp_writes` to the pass descriptor. Unlike the
  JavaScript version it can't hook the pass's `end`, so you call
  `timing_helper.resolve_timing(&mut encoder)` yourself after the pass drops.

* It's designed so if you use it wrong it will complain.

* It only handles 1 pass.

  There are a bunch of tradeoffs here and without more exploration it's not
  clear what would be best.

  A struct that handles multiple passes could be useful but, ideally, you'd use a
  single `QuerySet` that has enough space for all of your passes, rather than
  1 `QuerySet` per pass.

  But, in order to do that you'd either need to have the user tell you up front
  the maximum number of passes they'll use. Or, you need to make the code more
  complicated where it starts with a small `QuerySet` and deletes it and
  makes a new larger one if you use more. But then, at least for 1 frame, you'd
  need to handle having multiple `QuerySet`s

  All of that seemed overkill so for now it seemed best to make it handle one
  pass and you can build on top of it until you decide it needs to be changed.

You could also make a `NoTimingHelper`.

```rust
struct NoTimingHelper;

impl NoTimingHelper {
  fn new(_device: &wgpu::Device, _queue: &wgpu::Queue) -> Self {
    Self
  }

  fn begin_render_pass<'encoder>(
    &mut self,
    encoder: &'encoder mut wgpu::CommandEncoder,
    descriptor: &wgpu::RenderPassDescriptor<'_>,
  ) -> wgpu::RenderPass<'encoder> {
    encoder.begin_render_pass(descriptor)
  }

  fn resolve_timing(&mut self, _encoder: &mut wgpu::CommandEncoder) {}

  fn get_result(&mut self, callback: impl FnOnce(f64) + Send + 'static) {
    callback(0.0);
  }
}
```

As one possible way to make so you can add timing and turn it off without having
to change too much code.

In any case, I've used the `TimingHelper` class to time the various
examples from [the articles on using compute shaders to compute image histograms](webgpu-compute-shaders-histogram.html). Here's
a list of them (the original JavaScript versions). Since only the video example runs continuously it's probably
the best example

* <a target="_blank" href="../webgpu-compute-shaders-histogram-video-w-timing.html">4 channel video histogram</a>

The rest just run once and print their result to the JavaScript console.

* <a target="_blank" href="../webgpu-compute-shaders-histogram-4ch-optimized-more-w-timing.html">4 channel workgroup per chunk histogram with reduce</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-4ch-race-fixed-w-timing.html">4 channel workgroup per pixel histogram</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-4ch-javascript-w-timing.html">4 channel JavaScript histogram</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-optimized-more-w-timing.html">1 channel workgroup per chunk histogram with reduce</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-optimized-w-timing.html">1 channel workgroup per chunk histogram with sum</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-race-fixed-w-timing.html">1 channel workgroup per pixel histogram </a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-slow-w-timing.html">1 channel single core histogram</a>
* <a target="_blank" href="../webgpu-compute-shaders-histogram-javascript-w-timing.html">1 channel JavaScript histogram</a>

# <a id="a-implementation-defined"></a> Important: `timestamp-query` results are implementation-defined

This effectively means you can use them for debugging and for comparing techniques but you can not trust them to return similar results for all of your users.
You can't not even assume relative results. Different GPUs work in different ways
and are able to optimize rendering and computing across passes. That means
on one machine a first pass might take 200µs to draw 100 things and the 2nd pass
might also take 200µs to 200 things but, another GPU might take 100µs to draw the first 100 things and 200µs to draw the 2nd 100 things so where as the first GPU
had a relative difference of 0µs, the 2nd had a relative difference of 100µs
even though both GPUs were asked to draw the same thing.

# <a id="a-implementation-defined"></a> Important: `timestamp-query` results are not a good measure of performance

Timestamp queries are not a good measure of performance as there are many other factors that determine
overall performance. To give a concrete example. We wrote a render pass based mipmap generator in
[the article on loading images into textures](webgpu-importing-textures.html#a-generating-mips-on-the-gpu).
I wrote a compute pass based mipmap generator as well. When I used timestamp-query to time both it
told me the compute pass method was 5x faster than the render pass based method. Yay! But, then I switched to a throughput test. Instead of using timestamp-query, I wrote a test that let me increase
the number of 2048x2048 textures to generate mipmaps for at 60 frames a second. I'd increase the
number until the frame rate dropped below 60fps. Using this method showed the render pass method
was 20% faster than the compute pass method on one machine, and 8% faster on another.

The point is, you can't just use timestamp-query in isolation to tell you how fast something
will run.

<div class="webgpu_bottombar">In the browser, by default the <code>'timestamp-query'</code> time values
are quantized to 100µ seconds. In Chrome, if you enable <a href="chrome://flags/#enable-webgpu-developer-features" target="_blank">"enable-webgpu-developer-features"</a> in <a href="chrome://flags/#enable-webgpu-developer-features" target="_blank">about:flags</a>, the time values may not be quantized. This would
theoretically give you more accurate timings. (Running natively, wgpu gives you
the un-quantized values.) That said, normally 100µ second quantized values should be enough for you to compare shaders techniques for performance.
</div>
