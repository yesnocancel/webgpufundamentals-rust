Title: WebGPU Rotation
Description: Rotating an object
TOC: Rotation

This article is the 2nd in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

{{{toc-steps list="matrix-math.hanson"}}}

I'm going to admit right up front I have no idea if how I explain this
 will make sense but what the heck, might as well try.

First I want to introduce you to what's called a "unit circle". If you
remember your junior high school math (don't go to sleep on me!) a
circle has a radius. The radius of a circle is the distance from the center
of the circle to the edge. A unit circle is a circle with a radius of 1.0.

Here's a unit circle. [^ydown]

[^ydown]: This unit circle has +Y going down to match our pixel space which
is also Y down. WebGPU's normal clip space is +Y up. As we went over in the
previous article we've flipped Y in the shader.

<div class="webgpu_center"><div data-diagram="unit-circle" style="display: inline-block; width: 500px;"></div></div>

Notice as you drag the blue handle around the circle the X and Y positions
change. Those represent the position of that point on the circle. At the
top Y is 1 and X is 0. On the right X is 1 and Y is 0.

If you remember from basic 3rd grade math if you multiply something by 1
it stays the same. So 123 * 1 = 123. Pretty basic, right? Well, a unit circle,
a circle with a radius of 1.0 is also a form of 1. It's a rotating 1.
So you can multiply something by this unit circle and in a way it's kind
of like multiplying by 1 except magic happens and things rotate.

We're going to take that X and Y value from any point on the unit circle
and we'll multiply our vertex positions by them from [our previous example](webgpu-translation.html).

Here are the updates to our shader.


```wgsl
struct Uniforms {
  color: vec4f,
  resolution: vec2f,
  translation: vec2f,
+  rotation: vec2f,
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

+  // Rotate the position
+  let rotatedPosition = vec2f(
+    vert.position.x * uni.rotation.x - vert.position.y * uni.rotation.y,
+    vert.position.x * uni.rotation.y + vert.position.y * uni.rotation.x
+  );

  // Add in the translation
-  let position = vert.position + uni.translation;
+  let position = rotatedPosition + uni.translation;

  // convert the position from pixels to a 0.0 to 1.0 value
  let zeroToOne = position / uni.resolution;

  // convert from 0 <-> 1 to 0 <-> 2
  let zeroToTwo = zeroToOne * 2.0;

  // covert from 0 <-> 2 to -1 <-> +1 (clip space)
  let flippedClipSpace = zeroToTwo - 1.0;

  // flip Y
  let clipSpace = flippedClipSpace * vec2f(1, -1);

  vsOut.position = vec4f(clipSpace, 0.0, 1.0);
  return vsOut;
}
```

And we update the Rust to add space to the new uniform value.

```rust
-  // color, resolution, translation
-  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2) * 4;
+  // color, resolution, translation, rotation, padding
+  const UNIFORM_BUFFER_SIZE: u64 = (4 + 2 + 2 + 2) * 4 + 8;
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
  const K_TRANSLATION_OFFSET: usize = 6;
+  const K_ROTATION_OFFSET: usize = 8;
```

And we need some kind of UI. This isn't a tutorial about making UIs so
I'm just going to use one. First some HTML to give it a place to be

```html
  <body>
    <canvas></canvas>
+    <div id="circle"></div>
  </body>
```

Then some CSS to put it somewhere

```css
#circle {
  position: fixed;
  right: 0;
  bottom: 0;
  width: 300px;
  background-color: var(--bg-color);
}
```

and finally the page JavaScript to use it, feeding the circle's x and y into
the wasm module,

```js
+import UnitCircle from './resources/js/unit-circle.js';

...

+  const unitCircle = new UnitCircle();
+  document.querySelector('#circle').appendChild(unitCircle.domElement);
+  unitCircle.onChange(() => {
+    wasm.set_setting_num('rotationX', unitCircle.x);
+    wasm.set_setting_num('rotationY', unitCircle.y);
+  });
```

and the Rust to read it.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    ...

    // Set the uniform values in our Rust side array
    uniform_values[K_RESOLUTION_OFFSET..K_RESOLUTION_OFFSET + 2]
        .copy_from_slice(&[frame.width as f32, frame.height as f32]);
    uniform_values[K_TRANSLATION_OFFSET..K_TRANSLATION_OFFSET + 2]
        .copy_from_slice(&translation);
+    // x, y from the unit circle widget on the page
+    let rotation = [
+        wgpu_fun::setting_f64("rotationX", 1.0) as f32,
+        wgpu_fun::setting_f64("rotationY", 0.0) as f32,
+    ];
+    uniform_values[K_ROTATION_OFFSET..K_ROTATION_OFFSET + 2]
+        .copy_from_slice(&rotation);

    // upload the uniform values to the uniform buffer
    frame.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values));
```

And here's the result. Drag the handle on the circle to rotate
or the sliders to translate.

{{{example url="../webgpu-rotation-via-unit-circle.html"}}}

Why does it work? Well, look at the math.

<div class="webgpu_center">
<pre class="webgpu_math">
rotatedX = a_position.x * u_rotation.x - a_position.y * u_rotation.y;
rotatedY = a_position.x * u_rotation.y + a_position.y * u_rotation.x;
</pre>
</div>

Let's say you have a rectangle and you want to rotate it.
Before you start rotating it, the top right corner is at 3.0, -9.0.
Let's pick a point on the unit circle 30 degrees clockwise from 3 o'clock.

<div class="webgpu_center"><div data-diagram="static-circle-30" style="display: inline-block; width: 400px;"></div></div>

The position on the circle there is x = 0.87, y = 0.50

<div class="webgpu_center">
<pre class="webgpu_math">
 3.0 * 0.87 - -9.0 * 0.50 =  7.1
 3.0 * 0.50 + -9.0 * 0.87 = -6.3
</pre>
</div>

That's exactly where we need it to be

<img src="resources/rotation-drawing.svg" width="500" class="webgpu_center" style="width: 1000px"/>

The same for 60 degrees clockwise

<div class="webgpu_center"><div data-diagram="static-circle-60" style="display: inline-block; width: 400px;"></div></div>

The position on the circle there is 0.87 and 0.50

<div class="webgpu_center">
<pre class="webgpu_math">
 3.0 * 0.50 - -9.0 * 0.87 =  9.3
 3.0 * 0.87 + -9.0 * 0.50 = -1.9
</pre>
</div>

You can see that as we rotate that point clockwise, the X
value gets bigger and the Y gets smaller. If we kept going past 90 degrees
X would start getting smaller again and Y would start getting bigger.
That pattern gives us rotation.

There's another name for the points on a unit circle. They're called
the sine and cosine. So for any given angle we can just look up the
sine and cosine like this.

    fn print_sine_and_cosine_for_an_angle(angle_in_degrees: f32) {
      let angle_in_radians = angle_in_degrees.to_radians();
      let s = angle_in_radians.sin();
      let c = angle_in_radians.cos();
      println!("s = {s} c = {c}");
    }

If you call `print_sine_and_cosine_for_an_angle(30.0)` it prints
`s = 0.50 c = 0.87` (note: I rounded off the numbers)

If you put it all together you can rotate your vertex positions to any angle
you desire. Just set the rotation to the sine and cosine of the angle
you want to rotate to.

      ...
      let angle_in_radians = angle_in_degrees.to_radians();
      rotation[0] = angle_in_radians.cos();
      rotation[1] = angle_in_radians.sin();

Let's change things to just have an rotation setting. In the page:

```js
+  const degToRad = d => d * Math.PI / 180;

  const settings = {
    translation: [150, 100],
+    rotation: degToRad(30),
  };

  const radToDegOptions = { min: -360, max: 360, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
  gui.add(settings.translation, '0', 0, 1000).name('translation.x')
     .onChange(v => wasm.set_setting_num('translationX', v));
  gui.add(settings.translation, '1', 0, 1000).name('translation.y')
     .onChange(v => wasm.set_setting_num('translationY', v));
+  gui.add(settings, 'rotation', radToDegOptions)
+     .onChange(v => wasm.set_setting_num('rotation', v));

-  const unitCircle = new UnitCircle();
-  document.querySelector('#circle').appendChild(unitCircle.domElement);
-  unitCircle.onChange(...);
```

And in the Rust:

```rust
    ...
-    // x, y from the unit circle widget on the page
-    let rotation = [
-        wgpu_fun::setting_f64("rotationX", 1.0) as f32,
-        wgpu_fun::setting_f64("rotationY", 0.0) as f32,
-    ];
+    let angle = wgpu_fun::setting_f64("rotation", 30.0f64.to_radians()) as f32;
+    let rotation = [angle.cos(), angle.sin()];
    uniform_values[K_ROTATION_OFFSET..K_ROTATION_OFFSET + 2]
        .copy_from_slice(&rotation);
```

Drag the sliders to translate or rotate.

{{{example url="../webgpu-rotation.html"}}}

I hope that made some sense. [Next up a simpler one. Scale](webgpu-scale.html).

<div class="webgpu_bottombar"><h3>What are radians?</h3>
<p>
Radians are a unit of measurement used with circles, rotation and angles.
Just like we can measure distance in inches, yards, meters, etc we can
measure angles in degrees or radians.
</p>
<p>
You're probably aware that math with metric measurements is easier than
math with imperial measurements. To go from inches to feet we divide by 12.
To go from inches to yards we divide by 36. I don't know about you but I
can't divide by 36 in my head. With metric it's much easier. To go from
millimeters to centimeters we divide by 10. To go from millimeters to meters
we divide by 1000. I **can** divide by 1000 in my head.
</p>
<p>
Radians vs degrees are similar. Degrees make the math hard. Radians make
the math easy. There are 360 degrees in a circle but there are only 2π radians.
So a full turn is 2π radians. A half turn is 1π radian. A 1/4 turn, ie 90 degrees
is 1/2π radians. So if you want to rotate something 90 degrees just use
<code>PI * 0.5</code>. If you want to rotate it 45 degrees use
<code>PI * 0.25</code> etc. (in Rust, <code>std::f32::consts::PI</code>,
or use <code>90.0f32.to_radians()</code>).
</p>
<p>
Nearly all math involving angles, circles or rotation works very simply
if you start thinking in radians. So give it try. Use radians, not degrees,
except in UI displays.
</p>
</div>

<!-- keep this at the bottom of the article -->
<script type="module" src="webgpu-rotation.js"></script>

