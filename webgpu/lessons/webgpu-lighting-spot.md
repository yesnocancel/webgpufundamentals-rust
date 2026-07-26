Title: WebGPU - Spot Lighting
Description: How to implement spot lights in WebGPU
TOC: Spot Lighting


This article is a continuation of [the article on Point
Lighting](webgpu-lighting-point.html).  If you haven't read that I
suggest [you start there](webgpu-lighting-point.html).

In the last article we covered point lighting where for every point
on the surface of our object we compute the direction from the light
to that point on the surface. We then do the same thing we did for
[directional lighting](webgpu-lighting-directional.html) which is
we took the dot product of the surface normal (the direction the surface
is facing) and the light direction. This gave us a value of
1 if the two directions matched and should therefore be fully lit. 0 if
the two directions were perpendicular and -1 if they were opposite.
We used that value directly to multiply the color of the surface
which gave us lighting.

Spot lighting is only a very small change. In fact if you think
creatively about the stuff we've done so far you might be able
to derive your own solution.

You can imagine a point light as a point with light going in all
directions from that point.
To make a spot light all we need to do is choose a direction from
that point, this is the direction of our spotlight. Then, for every
direction the light is going we could take the dot product of
that direction with our chosen spotlight direction. We'd pick some arbitrary
limit and decide if we're within that limit we light. If we're not within
that limit we don't light.

{{{diagram url="resources/spot-lighting.html" width="700" height="400" className="noborder" }}}

In the diagram above we can see a light with rays going in all directions and
printed on them is their dot product relative to the direction.
We then have a specific **direction** that is the direction of the spotlight.
We choose a limit (above it's in degrees). From the limit we compute a *dot limit*, we just take the cosine of the limit. If the dot product of our chosen direction of the spotlight to
the direction of each ray of light is above the dot limit then we do the lighting. Otherwise no lighting.

To say this another way, let's say the limit is 20 degrees. We can convert
that to radians and from that to a value for -1 to 1 by taking the cosine. Let's call that dot space.
In other words here's a small table for limit values

              limits in
     degrees | radians | dot space
     --------+---------+----------
        0    |   0.0   |    1.0
        22   |    .38  |     .93
        45   |    .79  |     .71
        67   |   1.17  |     .39
        90   |   1.57  |    0.0
       180   |   3.14  |   -1.0

Then we can the just check

    dotFromDirection = dot(surfaceToLight, -lightDirection)
    if (dotFromDirection >= limitInDotSpace) {
       // do the lighting
    }

Let's do that

First let's modify our fragment shader from
[the last article](webgpu-lighting-point.html).

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
+  lightDirection: vec3f,
+  limit: f32,
};

...

@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  // Because vsOut.normal is an inter-stage variable 
  // it's interpolated so it will not be a unit vector.
  // Normalizing it will make it a unit vector again
  let normal = normalize(vsOut.normal);

  let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
  let surfaceToViewDirection = normalize(vsOut.surfaceToView);
  let halfVector = normalize(
    surfaceToLightDirection + surfaceToViewDirection);


+  var light = 0.0;
+  var specular = 0.0;
+
+  let dotFromDirection = dot(surfaceToLightDirection, -uni.lightDirection);
+  if (dotFromDirection > uni.limit) {
    // Compute the light by taking the dot product
    // of the normal with the direction to the light
-    let light = dot(normal, surfaceToLightDirection);
+    light = dot(normal, surfaceToLightDirection);

    specular = dot(normal, halfVector);
    specular = select(
        0.0,                           // value if condition is false
        pow(specular, uni.shininess),  // value if condition is true
        specular > 0.0);               // condition
+  }

  // Lets multiply just the color portion (not the alpha)
  // by the light
  let color = uni.color.rgb * light + specular;
  return vec4f(color, uni.color.a);
}
```

Of course we need to add space for the new values in our uniform buffer.

```rust
-    // normalMatrix + worldViewProjection + world + color + light position +
-    // view position + shininess
-    const UNIFORM_BUFFER_SIZE: u64 = (12 + 16 + 16 + 4 + 4 + 4) * 4;
+    // normalMatrix + worldViewProjection + world + color + light position +
+    // view position + shininess + light direction + limit
+    const UNIFORM_BUFFER_SIZE: u64 = (12 + 16 + 16 + 4 + 4 + 4 + 4) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_NORMAL_MATRIX_OFFSET: usize = 0;
    const K_WORLD_VIEW_PROJECTION_OFFSET: usize = 12;
    const K_WORLD_OFFSET: usize = 28;
    const K_COLOR_OFFSET: usize = 44;
    const K_LIGHT_WORLD_POSITION_OFFSET: usize = 48;
    const K_VIEW_WORLD_POSITION_OFFSET: usize = 52;
    const K_SHININESS_OFFSET: usize = 55;
+    const K_LIGHT_DIRECTION_OFFSET: usize = 56;
+    const K_LIMIT_OFFSET: usize = 59;
```

and we need to set them

```rust
        uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4]
            .copy_from_slice(&[0.2, 1.0, 0.2, 1.0]); // green
+        let light_world_position = [-10.0, 30.0, 100.0];
        uniform_values[K_LIGHT_WORLD_POSITION_OFFSET..K_LIGHT_WORLD_POSITION_OFFSET + 3]
-            .copy_from_slice(&[-10.0, 30.0, 100.0]);
+            .copy_from_slice(&light_world_position);
        uniform_values[K_VIEW_WORLD_POSITION_OFFSET..K_VIEW_WORLD_POSITION_OFFSET + 3]
            .copy_from_slice(&eye);
        uniform_values[K_SHININESS_OFFSET] = shininess;
+        uniform_values[K_LIMIT_OFFSET] = limit.cos();

+        // Since we don't have a plane like most spotlight examples
+        // let's point the spot light at the F
+        {
+            let mat = m4::aim(
+                light_world_position,
+                [target[0] + aim_offset_x, target[1] + aim_offset_y, 0.0],
+                up,
+            );
+            // get the zAxis from the matrix
+            // negate it because lookAt looks down the -Z axis
+            uniform_values[K_LIGHT_DIRECTION_OFFSET..K_LIGHT_DIRECTION_OFFSET + 3]
+                .copy_from_slice(&mat[8..11]);
+        }
```

Above we're using `m4::aim` which we covered in
[the article on cameras](webgpu-cameras.html). Specifically
our F is `target`. The spot light is at `-10, 30, 100`. We add some
offsets to the target so we can easily aim the spotlight. We then
just pull out the *z axis* since that's the direction aim points
something.

We just need to add some UI code. On the example page's JavaScript
side

```js
  const settings = {
    rotation: degToRad(0),
    shininess: 30,
+    limit: degToRad(15),
+    aimOffsetX: -10,
+    aimOffsetY: 10,
  };

  const radToDegOptions = { min: -360, max: 360, step: 1, converters: GUI.converters.radToDeg };
+  const limitOptions = { min: 0, max: 90, step: 1, minRange: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
  gui.add(settings, 'rotation', radToDegOptions)
     .onChange(v => wasm.set_setting_num('rotation', v));
  gui.add(settings, 'shininess', { min: 1, max: 250 })
     .onChange(v => wasm.set_setting_num('shininess', v));
+  gui.add(settings, 'limit', limitOptions)
+     .onChange(v => wasm.set_setting_num('limit', v));
+  gui.add(settings, 'aimOffsetX', -50, 50)
+     .onChange(v => wasm.set_setting_num('aimOffsetX', v));
+  gui.add(settings, 'aimOffsetY', -50, 50)
+     .onChange(v => wasm.set_setting_num('aimOffsetY', v));
```

and in the Rust render code we read the new settings

```rust
        let rotation = wgpu_fun::setting_f64("rotation", 0.0) as f32;
        let shininess = wgpu_fun::setting_f64("shininess", 30.0) as f32;
+        let limit = wgpu_fun::setting_f64("limit", 15.0f64.to_radians()) as f32;
+        let aim_offset_x = wgpu_fun::setting_f64("aimOffsetX", -10.0) as f32;
+        let aim_offset_y = wgpu_fun::setting_f64("aimOffsetY", 10.0) as f32;
```

And here it is

{{{example url="../webgpu-lighting-spot.html" }}}

One note is we're negating `uni.lightDirection` in the shader.
That's a [*six of one, half dozen of another*](https://en.wiktionary.org/wiki/six_of_one,_half_a_dozen_of_the_other)
type of thing. We want the 2 directions we're comparing to point in
the same direction when they match. That means we need to compare
the surfaceToLightDirection to the opposite of the spotlight direction.

Right now the spotlight is super harsh. We're
either inside the spotlight or not and things just turn black.

To fix this we could use 2 limits instead of one,
an inner limit and an outer limit.
If we're inside the inner limit then use 1.0. If we're outside the outer
limit then use 0.0. If we're between the inner limit and the outer limit
then lerp between 1.0 and 0.0.

Here's one way we could do this

```wgsl
struct Uniforms {
  normalMatrix: mat3x3f,
  worldViewProjection: mat4x4f,
  world: mat4x4f,
  color: vec4f,
  lightWorldPosition: vec3f,
  viewWorldPosition: vec3f,
  shininess: f32,
  lightDirection: vec3f,
-  limit: f32,
+  innerLimit: f32,
+  outerLimit: f32,
};

...

-  var light = 0.0;
-  var specular = 0.0;
-
-  let dotFromDirection = dot(surfaceToLightDirection, -uni.lightDirection);
-  if (dotFromDirection > uni.limit) {
-    // Compute the light by taking the dot product
-    // of the normal with the direction to the light
-    light = dot(normal, surfaceToLightDirection);
-    specular = dot(normal, halfVector);
-    specular = select(
-        0.0,                           // value if condition false
-        pow(specular, uni.shininess),  // value if condition is true
-        specular > 0.0);               // condition
-  }

    let dotFromDirection = dot(surfaceToLightDirection, -uni.lightDirection);
    let limitRange = uni.innerLimit - uni.outerLimit;
    let inLight = saturate((dotFromDirection - uni.outerLimit) / limitRange);

    // Compute the light by taking the dot product
    // of the normal with the direction to the light
    let light = inLight * dot(normal, surfaceToLightDirection);

    var specular = dot(normal, halfVector);
    specular = inLight * select(
        0.0,                           // value if condition false
        pow(specular, uni.shininess),  // value if condition is true
        specular > 0.0);               // condition

```

We're using `saturate`. Saturate clamps a value between 0 and 1.
This means `inLight` will be 0 if we're outside of the `outerLimit`.
It will be 1 if we're inside the `innerLimit`. And, it will be between
0 and 1 between those 2 limits. We then multiply the light and specular
calculations by `inLight`.

And again we need to update our uniform buffer setup

```rust
-    // normalMatrix + worldViewProjection + world + color + light position +
-    // view position + shininess + light direction + limit
-    const UNIFORM_BUFFER_SIZE: u64 = (12 + 16 + 16 + 4 + 4 + 4 + 4) * 4;
+    // normalMatrix + worldViewProjection + world + color + light position +
+    // view position + shininess + light direction + inner/outer limit
+    const UNIFORM_BUFFER_SIZE: u64 = (12 + 16 + 16 + 4 + 4 + 4 + 4 + 4) * 4;
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("uniforms"),
        size: UNIFORM_BUFFER_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut uniform_values = [0.0f32; UNIFORM_BUFFER_SIZE as usize / 4];

    // offsets to the various uniform values in float32 indices
    const K_NORMAL_MATRIX_OFFSET: usize = 0;
    const K_WORLD_VIEW_PROJECTION_OFFSET: usize = 12;
    const K_WORLD_OFFSET: usize = 28;
    const K_COLOR_OFFSET: usize = 44;
    const K_LIGHT_WORLD_POSITION_OFFSET: usize = 48;
    const K_VIEW_WORLD_POSITION_OFFSET: usize = 52;
    const K_SHININESS_OFFSET: usize = 55;
    const K_LIGHT_DIRECTION_OFFSET: usize = 56;
-    const K_LIMIT_OFFSET: usize = 59;
+    const K_INNER_LIMIT_OFFSET: usize = 59;
+    const K_OUTER_LIMIT_OFFSET: usize = 60;
```

and where we set them. On the example page's JavaScript side the two
limits become a min/max pair of sliders

```js
  const radToDegOptions = { min: -360, max: 360, step: 1, converters: GUI.converters.radToDeg };
+  const limitOptions = { min: 0, max: 90, minRange: 1, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
  gui.add(settings, 'rotation', radToDegOptions)
     .onChange(v => wasm.set_setting_num('rotation', v));
  gui.add(settings, 'shininess', { min: 1, max: 250 })
     .onChange(v => wasm.set_setting_num('shininess', v));
-  gui.add(settings, 'limit', limitOptions)
-     .onChange(v => wasm.set_setting_num('limit', v));
+  GUI.makeMinMaxPair(gui, settings, 'innerLimit', 'outerLimit', limitOptions);
+  gui.onChange(() => {
+    wasm.set_setting_num('innerLimit', settings.innerLimit);
+    wasm.set_setting_num('outerLimit', settings.outerLimit);
+  });
  gui.add(settings, 'aimOffsetX', -50, 50)
     .onChange(v => wasm.set_setting_num('aimOffsetX', v));
  gui.add(settings, 'aimOffsetY', -50, 50)
     .onChange(v => wasm.set_setting_num('aimOffsetY', v));
```

and in the Rust render code

```rust
        let rotation = wgpu_fun::setting_f64("rotation", 0.0) as f32;
        let shininess = wgpu_fun::setting_f64("shininess", 30.0) as f32;
-        let limit = wgpu_fun::setting_f64("limit", 15.0f64.to_radians()) as f32;
+        let inner_limit = wgpu_fun::setting_f64("innerLimit", 15.0f64.to_radians()) as f32;
+        let outer_limit = wgpu_fun::setting_f64("outerLimit", 25.0f64.to_radians()) as f32;

    ...

        uniform_values[K_SHININESS_OFFSET] = shininess;
-        uniform_values[K_LIMIT_OFFSET] = limit.cos();
+        uniform_values[K_INNER_LIMIT_OFFSET] = inner_limit.cos();
+        uniform_values[K_OUTER_LIMIT_OFFSET] = outer_limit.cos();

    ...
```

And that works

{{{example url="../webgpu-lighting-spot-w-linear-falloff.html" }}}

Now we're getting something that looks more like a spotlight!

One thing to be aware of is if `innerLimit` is equal to `outerLimit`
then `limitRange` will be 0.0. We divide by `limitRange` and dividing by
zero is bad/undefined. There's nothing to do in the shader here. We just
need to make sure that `innerLimit` is never equal to
`outerLimit` which, in this case, the gui on the example page does for us.

WGSL also has a function we could use to slightly simplify this. It's
called `smoothstep` it returns a value from 0 to 1 but
it takes both an lower and upper bound and lerps between 0 and 1 between
those bounds.

```wgsl
     smoothstep(lowerBound, upperBound, value)
```

Let's do that

```wgsl
    let dotFromDirection = dot(surfaceToLightDirection, -uni.lightDirection);
-    let limitRange = uni.innerLimit - uni.outerLimit;
-    let inLight = saturate((dotFromDirection - uni.outerLimit) / limitRange);
+    let inLight = smoothStep(uni.outerLimit, uni.innerLimit, dotFromDirection);
```

That works too

{{{example url="../webgpu-lighting-spot-w-smoothstep-falloff.html" }}}

The difference is `smoothstep` uses a *hermite interpolation* instead of a
linear interpolation. That means between `lowerBound` and `upperBound`
it interpolates like the image below on the right whereas a linear interpolation is like the image on the left.

<img class="webgpu_center invertdark" src="resources/linear-vs-hermite.png" />

It's up to you if you think the difference matters.

One other thing to be aware is the `smoothstep` function has undefined
results if the `lowerBound` is greater than or equal to `upperBound`. Having
them be equal is the same issue we had above. The added issue of not being
defined if `lowerBound` is greater than `upperBound` is new but for the
purpose of a spotlight that should never be true.
