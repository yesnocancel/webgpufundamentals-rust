Title: WebGPU Camera Controls
Description: Controlling the Camera
TOC: Camera Controls

This article is the 2nd in a short series about making parts for a 3D editor.
Each one builds on the previous lesson so you may find them easiest to
understand by reading them in order.

{{{toc-steps list="editor.hanson"}}}

# Orbit Camera

An orbit camera is the camera that most 3D modeling packages like Blender,
Unity, Maya, 3DSMax, Unreal use in the editor. You can press some icon or hold
some key and then dragging the pointer orbits some point in the world.

There are some words that AFAIK, come from film and others from aviation

* "Pan" is turning the camera left and right at it's current location.

  When you take a panorama picture on your phone you "pan" the camera.

* "Tilt" is turning the camera up and down

  If you're standing you might tilt a camera down to take a picture
  of a flower or tilt it up to take a picture of an airplane.

* "Roll" is like tilting your head left or right.

  The horizon is no longer flat.

* "Dolly" is moving the camera closer or further

  This is often considered "zooming" but zoom with a camera lens is instead
  changing the field of view where as "dollying" is moving the camera closer
  or further from the target.

* "Track" is moving the camera perpendicular to the way it's facing.

  I'm only guessing this comes from
  [actually having a "track" to roll a movie camera on](https://en.wikipedia.org/wiki/Tracking_shot).

In any case, one way to solve many issues like this is to build a "rig".
A "rig" in 3D terms generally refers to some hierarchy of scene graph nodes,
potentially with some constraints added.

We could build a hierarchy like this

```
+-camTarget (anchors the center of rotation)
  +-camPitch (lets us "pan" around the target)
    +-camTilt (lets us "tilt" above or below the target)
      +-camExtend (lets us "dolly" the camera closer or further from the target)
        +-cam (gives us a camera matrix)
```

You can almost picture this as a actual mechanical rig made of physical parts.
I don't know if this is a good analogy but if you had a military tank, the tank itself would be the `camTarget`. The head that rotates on top of the
tank would be the `camPitch`. The part that lets the barrel rotate up and down
is the `camTilt`. The barrel itself is the `camExtend`. Ideally imagine a telescoping
barrel that can change length. You then attach the camera to the end of the barrel
**aimed back toward the tank**.

<div class="webgpu_center">
  <div data-diagram="camera-rig" style="width: 600px;"></div>
</div>

In the diagram above:

* the blue base is the `camTarget`
* the green head is the `camPitch`
* the red hinge is the `camTilt`
* the pink/purple barrel is the `camExtend`
* the white frame frustum represents a camera at `cam` looking back toward the `camTarget`

By default the pieces in the diagram are stacked up to make them easy to see but in our
actual rig they'd all sit on top of each other. Check "collapse" to put them where they should be.

In any case, let's make that camera rig.

First some minor UI tweaks. Since eventually
we want the user to be able to drag on the
scene to update the camera, lets make the controls
more like a 3D editor where instead of hovering
over the the scene, they fit some space on the right. We'll also make it so if the user closes
the controls the scene expands to fill the space.

First some HTML changes

```html
+<div id="split">
*  <canvas></canvas>
+  <div id="ui"></div>
+</div>
```

and the corresponding CSS

```css
#split {
  display: flex;
  height: 100%;
}
#ui {
  border-left: 1px solid #888;
}
#ui.hide-ui {
  right: 0;
  position: absolute;
}
#split > :nth-child(1) {
  flex: 1 1 auto;
  min-width: 0;
}
```

Then finally we'll move the UI inside this `#ui` div and update
the div's css classes based on the UI state. This is all page
JavaScript in our port; the wasm module re-renders on its own so there's
no `render()` to call.

```js
-  const gui = new GUI();
+  const uiElem = document.querySelector('#ui');
+  const gui = new GUI({
+    parent: uiElem,
+  });
+  gui.onChange(() => {
+    uiElem.classList.toggle('hide-ui', !gui.isOpen());
+  });
```

Now let's start making an orbit camera based on scene graph nodes.

Here's the our orbit camera rig. In JavaScript this was a class whose
private fields held direct references to its rig nodes; like the
`SceneGraph` itself, in Rust the rig holds node *indices* and its methods
take the scene graph. The JavaScript getters and setters
(`get pan()` / `set pan(v)`) become `pan(&scene)` / `set_pan(&mut scene, v)`
methods that read and write the rig nodes' `TRS` values.

```rust
// The camera rig. In JavaScript this was a class holding direct references
// to its rig nodes and a `nodeToUISettings` map for the page's GUI (the map
// stays page-side in this port); here the rig holds node indices and its
// methods take the SceneGraph.
struct OrbitCamera {
    cam_target: NodeNdx,
    cam_pan: NodeNdx,
    cam_tilt: NodeNdx,
    cam_extend: NodeNdx,
    cam: NodeNdx,
}

impl OrbitCamera {
    fn new(scene: &mut SceneGraph) -> Self {
        // Create Camera Rig
        let cam_target = add_trs_scene_graph_node(scene, "cam-target", None, TRS::default());
        let cam_pan = add_trs_scene_graph_node(scene, "cam-pan", Some(cam_target), TRS::default());
        let cam_tilt = add_trs_scene_graph_node(scene, "cam-tilt", Some(cam_pan), TRS::default());
        let cam_extend =
            add_trs_scene_graph_node(scene, "cam-extend", Some(cam_tilt), TRS::default());
        let cam = add_trs_scene_graph_node(scene, "cam", Some(cam_extend), TRS::default());

        OrbitCamera {
            cam_target,
            cam_pan,
            cam_tilt,
            cam_extend,
            cam,
        }
    }

    fn set_parent(&self, scene: &mut SceneGraph, parent: NodeNdx) {
        scene.set_parent(self.cam_target, Some(parent));
    }

    fn get_camera_matrix(&self, scene: &SceneGraph) -> [f32; 16] {
        scene.nodes[self.cam].world_matrix
    }

    fn pan(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_pan].source.as_ref().unwrap().rotation[1]
    }
    fn set_pan(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_pan].source.as_mut().unwrap().rotation[1] = v;
    }
    fn tilt(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_tilt].source.as_ref().unwrap().rotation[0]
    }
    fn set_tilt(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_tilt].source.as_mut().unwrap().rotation[0] = v;
    }
    fn radius(&self, scene: &SceneGraph) -> f32 {
        scene.nodes[self.cam_extend].source.as_ref().unwrap().translation[2]
    }
    fn set_radius(&self, scene: &mut SceneGraph, v: f32) {
        scene.nodes[self.cam_extend].source.as_mut().unwrap().translation[2] = v;
    }
    fn target(&self, scene: &SceneGraph) -> [f32; 3] {
        scene.nodes[self.cam_target].source.as_ref().unwrap().translation
    }
    fn set_target(&self, scene: &mut SceneGraph, v: [f32; 3]) {
        scene.nodes[self.cam_target].source.as_mut().unwrap().translation = v;
    }
}
```

The JavaScript version needed to add a `vec3.copy` function here so its
getter could return a copy of the target instead of a live reference.
In Rust `[f32; 3]` is a `Copy` type so returning it already returns a copy
and no helper is needed.

then we need to use the `OrbitCamera`

```rust
    let root = scene.add_node("root", None);

+    let orbit_camera = OrbitCamera::new(&mut scene);
+    orbit_camera.set_parent(&mut scene, root);
+    orbit_camera.set_target(&mut scene, [120.0, 80.0, 0.0]);
+    orbit_camera.set_tilt(&mut scene, std::f32::consts::PI * -0.2);
+    orbit_camera.set_radius(&mut scene, 300.0);

    // Add cabinets
    for cabinet_ndx in 0..K_NUM_CABINETS {
        add_cabinet(&mut scene, &mut meshes, root, cabinet_ndx);
    }
```

and in the frame callback we replace the old camera math with the camera
matrix from the rig. Note `update_world_matrix` moves up, before we compute
the view matrix, because the camera's matrix now comes from the scene graph.

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {
        ...

-        let camera_rotation =
-            wgpu_fun::setting_f64("cameraRotation", (-45.0f64).to_radians()) as f32;

        let aspect = frame.width as f32 / frame.height as f32;
        let projection = m4::perspective(
            60.0f32.to_radians(), // fieldOfView,
            aspect,
            1.0,    // zNear
            2000.0, // zFar
        );

-        // Compute a camera matrix
-        let mut camera_matrix = m4::identity();
-        camera_matrix = m4::translate(&camera_matrix, [120.0, 100.0, 0.0]);
-        camera_matrix = m4::rotate_y(&camera_matrix, camera_rotation);
-        camera_matrix = m4::translate(&camera_matrix, [60.0, 0.0, 300.0]);
-
-        // Compute a view matrix
-        let view_matrix = m4::inverse(&camera_matrix);
+        scene.update_world_matrix(root);
+
+        // make a view matrix from the camera's
+        let view_matrix = m4::inverse(&orbit_camera.get_camera_matrix(&scene));

        // combine the view and projection matrixes
        let view_projection_matrix = m4::multiply(&projection, &view_matrix);
```

Notice that a whole bunch of math disappeared. There is no math
in the `OrbitCamera` code, just rig nodes. This is because
all the math has been buried in the rig itself.

The page's mirror of the node tree needs the same nodes in the same order
so the GUI's node indices keep matching the Rust arena; the `cameraRotation`
setting goes away.

```js
+const nodeToUISettings = new Map();

const root = addNode('root');
+// mirror of the Rust module's OrbitCamera rig and its initial settings
+const camTarget = addNode('cam-target', root, { translation: [120, 80, 0] });
+const camPan = addNode('cam-pan', camTarget, {});
+const camTilt = addNode('cam-tilt', camPan, { rotation: [Math.PI * -0.2, 0, 0] });
+const camExtend = addNode('cam-extend', camTilt, { translation: [0, 0, 300] });
+const cam = addNode('cam', camExtend, {});
// Add cabinets
for (let cabinetNdx = 0; cabinetNdx < kNumCabinets; ++cabinetNdx) {
  addCabinet(root, cabinetNdx);
}

const settings = {
-  cameraRotation: degToRad(-45),
  showMeshNodes: false,
  showAllTRS: false,
};
```

We could run it as is but it would be difficult to change any
camera settings since our UI, by default, displays translation x,y,z
only OR all 9 translation, rotation, and scale settings per node.

Let's hack the UI so we can make the camera nodes show only relevant
settings. We'll do this by adding a map of scene graph nodes to
settings just to keep it simple and terse we'll provide an array
of controls by index we want to appear where 0, 1, 2 are translation
x, y, z. 3, 4, 5 are rotation x, y, z, and 6, 7, 8 are scale.
If no settings for the node exist then they'll follow the existing
rules. This is all page JavaScript, operating on the mirror.

```js
+nodeToUISettings.set(camTarget, { trs: [0, 1, 2] });
+nodeToUISettings.set(camPan, { trs: [4] });
+nodeToUISettings.set(camTilt, { trs: [3] });
+nodeToUISettings.set(camExtend, { trs: [2] });
+nodeToUISettings.set(cam, { trs: [] });

  ...

  function setCurrentSceneGraphNode(node) {
    currentNode = node;
    wasm.set_setting_num('nodeNdx', node.ndx);
    trsFolder.name(`orientation: ${node.name}`);
    trsFolder.updateDisplay();

+    showTRS();

    // Mark which node is selected.
    for (const b of nodeButtons) {
      const name = b.button.getName().replace(prefixRE, '');
      b.button.name(`${b.node === node ? kSelected : kUnelected}${name}`);
    }
  }

  ...

  const alwaysShow = new Set([0, 1, 2]);
-  function showTRS(show) {
+  function showTRS() {
+    const ui = nodeToUISettings.get(currentNode);
    trsControls.forEach((trs, i) => {
-      trs.show(show || alwaysShow.has(i));
+      const showThis = ui
+        ? ui.trs?.indexOf(i) >= 0
+        : (settings.showAllTRS || alwaysShow.has(i));
+      trs.show(showThis);
    });
  }
-  showTRS(false);
```

With those changes we've replaced the old camera code with
our new `OrbitCamera`, removed a bunch of math, and made the
camera's rig nodes show up in the UI with their settings
visible and editable.

{{{example url="../webgpu-camera-controls-scene-graph-step-01.html"}}}

Now that we have the basics in place, lets add some pointer controls.

**A note on how this port handles pointer input:** the JavaScript originals
attach `pointerdown`, `pointermove`, `pointerup` and `wheel` listeners to
the canvas. In this port, wgpu_fun forwards those events (and the matching
mouse events of a native window) into a queue that the example drains with
`wgpu_fun::drain_pointer_events()` inside the frame callback. The events
are `PointerEvent::Down { x, y, button }`, `Move { x, y }`,
`Up { x, y, button }` and `Wheel { delta_x, delta_y }`, with coordinates in
device pixels, and any event triggers a re-render so `RenderMode::Once`
still works. The camera math stays the same; a few event-plumbing details
change, all following from the queue being a single merged pointer stream:

* There's no pointer capture. A `Down` starts a drag and an `Up` ends it —
  we track that with the presence of the update helper.
* Events don't carry keyboard modifiers, so where the originals check
  `e.shiftKey || (e.buttons & 4) !== 0` to pick "track" mode we check
  whether the drag started with the middle mouse button (`button == 1`).
* Events don't carry pointer ids, so multi-touch gestures (the pinch
  section below) can't be reconstructed — with 2 or more pointers down we
  give up, like the originals do for 3 or more.
* Camera changes happen inside the wasm module, so the page's mirrored TRS
  values (and therefore the numbers in the GUI) don't live-update while you
  drag; they only reflect edits made through the GUI itself.

## <a id="a-pan-and-tilt"></a> Pan and Tilt

Lets adjust pan and tilt when you drag the pointer.

First, we need to make minor CSS tweak so that dragging doesn't
select the canvas among other things.

```css
canvas {
  display: block;  /* make the canvas act like a block   */
  width: 100%;     /* make the canvas fill its container */
  height: 100%;
+  touch-action: none;
}
```

Then, let's add some code to the camera to encapsulate these
changes a little. We'll make a function `get_update_helper` that
records some relevant but kind of private camera state, and the
helper will provide functions to modify the camera state by
deltas the input code will pass in. In JavaScript the helper was an object
of closures that captured the starting camera state; in Rust it's a struct
of the starting values with methods that take the camera and scene.

```rust
  impl OrbitCamera {

   ...

+    fn get_update_helper(&self, scene: &SceneGraph) -> UpdateHelper {
+        UpdateHelper {
+            start_tilt: self.tilt(scene),
+            start_pan: self.pan(scene),
+        }
+    }

   ...

  }

+struct UpdateHelper {
+    start_tilt: f32,
+    start_pan: f32,
+}
+
+impl UpdateHelper {
+    fn pan_and_tilt(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_pan: f32, delta_tilt: f32) {
+        cam.set_tilt(scene, self.start_tilt - delta_tilt);
+        cam.set_pan(scene, self.start_pan - delta_pan);
+    }
+}
```

Then, we can add some code to connect pointer input to create
the helper and pass in deltas. Where the JavaScript version's
`addOrbitCameraEventListeners` adds listeners, we keep a little state
across frames and drain the event queue at the top of the frame callback.

```rust
+    // state for the pointer events (addOrbitCameraEventListeners in the
+    // JS version)
+    let mut start_x = 0.0f32;
+    let mut start_y = 0.0f32;
+    // Some(...) while a drag is in progress; this stands in for the JS
+    // version's pointer capture check.
+    let mut cam_helper: Option<UpdateHelper> = None;

    app.run(RenderMode::Once, move |frame: &Frame| {
        ...

+        // The JS version attaches pointerdown/pointermove/pointerup
+        // listeners to the canvas; here we drain wgpu_fun's pointer event
+        // queue instead (coordinates are in device pixels).
+        for event in wgpu_fun::drain_pointer_events() {
+            match event {
+                PointerEvent::Down { x, y, .. } => {
+                    // canvas.setPointerCapture(e.pointerId);
+                    // updateStartPosition(e);
+                    start_x = x;
+                    start_y = y;
+                    cam_helper = Some(orbit_camera.get_update_helper(&scene));
+                }
+                PointerEvent::Move { x, y } => {
+                    // if (!canvas.hasPointerCapture(e.pointerId)) return;
+                    let Some(helper) = &cam_helper else {
+                        continue;
+                    };
+
+                    let delta_x = x - start_x;
+                    let delta_y = y - start_y;
+
+                    helper.pan_and_tilt(
+                        &orbit_camera,
+                        &mut scene,
+                        delta_x * 0.01,
+                        delta_y * 0.01,
+                    );
+                }
+                PointerEvent::Up { .. } => {
+                    // canvas.releasePointerCapture(e.pointerId);
+                    cam_helper = None;
+                }
+                PointerEvent::Wheel { .. } => {}
+            }
+        }
```

The code is pretty straight forward. On `Down` we call
`get_update_helper` which records the current `pan` and `tilt`. We also record
the current pointer position. On `Move` we compute the delta from
where the pointer started and pass it into the helper to adjust `pan` and
`tilt`. That's basically it. There's no `render()` call because pushing a
pointer event already requests a re-render.

One more small change on the page, the original makes the GUI check for
updates to the values with `.listen()`. We keep it, though as noted above,
in this port the mirrored values only change for GUI-driven edits.

```js
-  const trsFolder = gui.addFolder('orientation');
+  const trsFolder = gui.addFolder('orientation').listen();
```

Give it try, drag your finger on the canvas.

{{{example url="../webgpu-camera-controls-scene-graph-step-02.html"}}}

## <a id="a-track"></a> Tracking

It's common that if you hold some modifying key, like shift, while dragging,
instead of adjusting the pan or tilt, you instead "track" the camera (translate it).

Let's add that. First off we need a few new math functions.

```rust
mod vec3 {

+    pub fn create() -> [f32; 3] {
+        [0.0; 3]
+    }
+
+    pub fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0] + b[0];
+        dst[1] = a[1] + b[1];
+        dst[2] = a[2] + b[2];
+
+        dst
+    }
+
+    pub fn transform_mat3(v: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        let x = v[0];
+        let y = v[1];
+        let z = v[2];
+
+        dst[0] = x * m[0] + y * m[4] + z * m[8];
+        dst[1] = x * m[1] + y * m[5] + z * m[9];
+        dst[2] = x * m[2] + y * m[6] + z * m[10];
+
+        dst
+    }

    ...
}
```

`create` just creates a vec3 with 3 zeros. `add` adds two vec3s.
Finally, `transform_mat3` multiplies a vector by a 3x3 matrix. This was
mentioned [when we covered normals for lighting](webgpu-lighting-directional.html#a-normals). There, we multiplied a normal (vec3f) by a normal matrix (mat3x3f) in WGSL. Here, we're essentially doing the same thing but in Rust but instead of re-orienting a normal we're reorienting the pointer
movement.

We can now update the helper

```rust
  impl OrbitCamera {

    ...

    fn get_update_helper(&self, scene: &SceneGraph) -> UpdateHelper {
        UpdateHelper {
            start_tilt: self.tilt(scene),
            start_pan: self.pan(scene),
+            start_camera_matrix: self.get_camera_matrix(scene),
+            start_target: self.target(scene),
        }
    }
  }

  struct UpdateHelper {
      start_tilt: f32,
      start_pan: f32,
+      start_camera_matrix: [f32; 16],
+      start_target: [f32; 3],
  }

  impl UpdateHelper {
      fn pan_and_tilt(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_pan: f32, delta_tilt: f32) {
          cam.set_tilt(scene, self.start_tilt - delta_tilt);
          cam.set_pan(scene, self.start_pan - delta_pan);
      }
+
+      fn track(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_x: f32, delta_y: f32) {
+          let direction = vec3::transform_mat3([delta_x, delta_y, 0.0], &self.start_camera_matrix);
+          cam.set_target(scene, vec3::add(self.start_target, direction));
+      }
  }
```

`track` takes an xy delta  multiplies it by the upper left 3x3 matrix of our
camera matrix. This has the effect of orienting the direction perpendicular to
the way the camera is facing. We can then just add that to our target

We then `track` from the pointer event code. The JS version uses strings
for the modes; in Rust an enum is the natural fit.

```rust
+// The JS version uses strings for the modes ('track', 'panAndTilt', ...);
+// in Rust an enum is the natural fit.
+#[derive(Clone, Copy, PartialEq)]
+enum Mode {
+    Track,
+    PanAndTilt,
+}

    ...

    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;
+    let mut last_mode: Option<Mode> = None;
    let mut cam_helper: Option<UpdateHelper> = None;
+    // wgpu_fun's event queue doesn't carry keyboard modifiers, so where the
+    // JS version checks `e.shiftKey || (e.buttons & 4) !== 0` we check
+    // which button started the drag (1 = middle).
+    let mut drag_button = 0u32;

    ...

        for event in wgpu_fun::drain_pointer_events() {
            match event {
-                PointerEvent::Down { x, y, .. } => {
+                PointerEvent::Down { x, y, button } => {
                    // canvas.setPointerCapture(e.pointerId);
+                    drag_button = button;
                    // updateStartPosition(e);
                    start_x = x;
                    start_y = y;
                    cam_helper = Some(orbit_camera.get_update_helper(&scene));
                }
                PointerEvent::Move { x, y } => {
                    // if (!canvas.hasPointerCapture(e.pointerId)) return;
-                    let Some(helper) = &cam_helper else {
-                        continue;
-                    };
+                    if cam_helper.is_none() {
+                        continue;
+                    }

+                    let mode = if drag_button == 1 {
+                        Mode::Track
+                    } else {
+                        Mode::PanAndTilt
+                    };
+
+                    if Some(mode) != last_mode {
+                        last_mode = Some(mode);
+                        // updateStartPosition(e);
+                        start_x = x;
+                        start_y = y;
+                        cam_helper = Some(orbit_camera.get_update_helper(&scene));
+                    }

                    let delta_x = x - start_x;
                    let delta_y = y - start_y;

-                    helper.pan_and_tilt(
-                        &orbit_camera,
-                        &mut scene,
-                        delta_x * 0.01,
-                        delta_y * 0.01,
-                    );
+                    let helper = cam_helper.as_ref().unwrap();
+                    match mode {
+                        Mode::Track => {
+                            let s = orbit_camera.radius(&scene) * 0.001;
+                            helper.track(&orbit_camera, &mut scene, -delta_x * s, delta_y * s);
+                        }
+                        Mode::PanAndTilt => {
+                            helper.pan_and_tilt(
+                                &orbit_camera,
+                                &mut scene,
+                                delta_x * 0.01,
+                                delta_y * 0.01,
+                            );
+                        }
+                    }
                }
```

Our event code above, computes a mode based on which button the drag
started with (in the original, whether or not the user is holding the shift
key or the middle mouse button — see the adaptation note above). If the
mode switches then we need to record starting values. It then switches on
the mode.

Our `Mode::Track` passes the pointer delta to the helper's `track`
function. We scale the delta by the radius (our distance from the
target), that way we'll move in smaller steps if we're really close up.

Now you can hold the mouse wheel (middle button) down and move your mouse
to track.

{{{example url="../webgpu-camera-controls-scene-graph-step-03.html"}}}

## <a id="a-dolly-by-wheel"></a> Dolly by Wheel

Next let's add zooming or "dolly" with the scroll wheel which is pretty common.

First let's update our helper.

```rust
    fn get_update_helper(&self, scene: &SceneGraph) -> UpdateHelper {
        UpdateHelper {
            start_tilt: self.tilt(scene),
            start_pan: self.pan(scene),
+            start_radius: self.radius(scene),
            start_camera_matrix: self.get_camera_matrix(scene),
            start_target: self.target(scene),
        }
    }

  struct UpdateHelper {
      start_tilt: f32,
      start_pan: f32,
+      start_radius: f32,
      start_camera_matrix: [f32; 16],
      start_target: [f32; 3],
  }

  impl UpdateHelper {

      ...

+      fn dolly(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta: f32) {
+          cam.set_radius(scene, self.start_radius + delta);
+      }
  }
```

And then let's use it.

```rust
        for event in wgpu_fun::drain_pointer_events() {
            match event {

  ...

-                PointerEvent::Wheel { .. } => {}
+                // Dolly when the user uses the wheel
+                PointerEvent::Wheel { delta_y, .. } => {
+                    // (e.preventDefault() happens inside wgpu_fun)
+                    let helper = orbit_camera.get_update_helper(&scene);
+                    let radius = orbit_camera.radius(&scene);
+                    helper.dolly(&orbit_camera, &mut scene, radius * 0.001 * delta_y);
+                }
            }
        }
```

With that small change you should be able to zoom in/out (dolly) with
the mouse wheel (or with 2 fingers on a laptop).

The code is adjusting by 1000th of the radius. This has not been tested
with lots of scenes but it seems reasonable that we don't want to
move the same speed if we're too close.

{{{example url="../webgpu-camera-controls-scene-graph-step-04.html"}}}

## <a id="a-dolly-by-pinch"></a> Dolly by Pinch

On mobile it's common to pinch to zoom.

The JavaScript original implements this by keeping a
`Map` of pointer id → last position. When exactly 2 pointers are down it's
a pinch: it records the distance between them when the pinch starts
(`computePinchDistance`), and as they move it dollies by how much that
distance has changed. With more than 2 pointers it gives up.

As covered in the adaptation note above, wgpu_fun's event queue merges all
pointers into one stream with no pointer ids, so we can't tell which finger
a `Move` belongs to and can't compute a pinch distance. What we *can* do is
count pointers (the JS version's `pointerToLastPosition.size`) and give up
on 2 or more, the way the JS gives up on 3 or more.

```rust
    let mut drag_button = 0u32;
+    // The JS version keeps a Map of pointer id -> last position so it can
+    // compute the distance between 2 fingers (a pinch). wgpu_fun's event
+    // queue merges all pointers into one stream with no ids, so we can only
+    // count them (the JS version's pointerToLastPosition.size) and give up
+    // on 2 or more, like the JS gives up on 3 or more.
+    let mut pointer_count = 0i32;

    ...

#[derive(Clone, Copy, PartialEq)]
enum Mode {
+    Undefined,
    Track,
    PanAndTilt,
}

    ...

                PointerEvent::Down { x, y, button } => {
                    // canvas.setPointerCapture(e.pointerId);
+                    pointer_count += 1;
                    drag_button = button;
                    ...
                }
                PointerEvent::Move { x, y } => {
                    ...

-                    let mode = if drag_button == 1 {
+                    let mode = if pointer_count >= 2 {
+                        // more than one pointer; without pointer ids we
+                        // can't compute a pinch distance, so give up.
+                        Mode::Undefined
+                    } else if drag_button == 1 {
                        Mode::Track
                    } else {
                        Mode::PanAndTilt
                    };

                    ...

                    match mode {
+                        Mode::Undefined => {}
                        Mode::Track => {
                        ...
                    }
                }
                PointerEvent::Up { .. } => {
+                    // pointerToLastPosition.delete(e.pointerId);
+                    pointer_count = (pointer_count - 1).max(0);
                    // canvas.releasePointerCapture(e.pointerId);
                    cam_helper = None;
                }
```

So, unlike the original, this version doesn't dolly on pinch — but it also
won't misbehave when a second finger comes down. If you need real pinch
support you'd extend wgpu_fun's event plumbing to carry pointer ids, which
would be a straight port of the JavaScript shown in the original article.

{{{example url="../webgpu-camera-controls-scene-graph-step-05.html"}}}

## <a id="a-dolly-by-double-tab-drag"></a> Dolly by Double Tap Drag

Let's do one more. It's common on some apps that if you double tap the screen
and then drag your finger it zooms. Google Maps does this for example. Let's add
that. This one only needs a single pointer and a clock, so it ports
directly — we use the frame time where the JavaScript uses
`performance.now()`.

```rust
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Undefined,
+    DoubleTapZoom,
    Track,
    PanAndTilt,
}

    ...

    let mut pointer_count = 0i32;
+    let mut double_tap_mode = false;
+    // performance.now() in the JS version; we use the frame time instead.
+    let mut last_single_tap_time = f64::NEG_INFINITY;

    ...

                PointerEvent::Down { x, y, button } => {
+                    const K_DOUBLE_CLICK_TIME_MS: f64 = 300.0;
                    // canvas.setPointerCapture(e.pointerId);
                    pointer_count += 1;
                    drag_button = button;
+                    if pointer_count == 1 {
+                        if !double_tap_mode {
+                            let now = frame.time;
+                            let delta_time = (now - last_single_tap_time) * 1000.0;
+                            if delta_time < K_DOUBLE_CLICK_TIME_MS {
+                                double_tap_mode = true;
+                            }
+                            last_single_tap_time = now;
+                        }
+                    } else {
+                        double_tap_mode = false;
+                    }
                    // updateStartPosition(e);
                    start_x = x;
                    start_y = y;
                    cam_helper = Some(orbit_camera.get_update_helper(&scene));
                }
                PointerEvent::Move { x, y } => {
                    ...

                    let mode = if pointer_count >= 2 {
                        Mode::Undefined
+                    } else if double_tap_mode {
+                        Mode::DoubleTapZoom
                    } else if drag_button == 1 {
                        Mode::Track
                    } else {
                        Mode::PanAndTilt
                    };

                    ...

                    match mode {
                        ...
+                        Mode::DoubleTapZoom => {
+                            let radius = orbit_camera.radius(&scene);
+                            helper.dolly(&orbit_camera, &mut scene, radius * 0.002 * delta_y);
+                        }
                    }
                }
                PointerEvent::Up { .. } => {
                    // pointerToLastPosition.delete(e.pointerId);
                    pointer_count = (pointer_count - 1).max(0);
                    // canvas.releasePointerCapture(e.pointerId);
                    cam_helper = None;
+                    if pointer_count == 0 {
+                        double_tap_mode = false;
+                    }
                }
```

The code checks if there is a single `Down` and checks the time between that and
the last single `Down`. If it's below `K_DOUBLE_CLICK_TIME_MS` then we're in `double_tap_mode`
and we can adjust the zoom based on the distance from where the 2nd tap started.

ATM, this will work with the mouse or a touch screen. Is it appropriate for a mouse?
Give it a try.

{{{example url="../webgpu-camera-controls-scene-graph-step-06.html"}}}

## <a id="a-camera-not-at-root"></a> Camera not at root

An issue we have not covered is what if our OrbitCamera, which exists
in the scene graph, is not based at the root of the graph.

For example, lets say it was a camera in the scene on a fallen tower.
Since the tower is fallen the camera is not level with ground.

For tilt, pan, and dolly, nothing needs to change as all of these are
relative to the camera itself but for track, we need to do some extra
work since the target of the camera is relative to its parent node.

To fix this, first, we should probably replace the plain `set_target`
setter as it's mis-leading. We'll make it take a world position and take
the camera's parent into account.

```rust
    fn target(&self, scene: &SceneGraph) -> [f32; 3] {
        scene.nodes[self.cam_target].source.as_ref().unwrap().translation
    }
-    fn set_target(&self, scene: &mut SceneGraph, v: [f32; 3]) {
-        scene.nodes[self.cam_target].source.as_mut().unwrap().translation = v;
-    }
+    fn set_target(&self, scene: &mut SceneGraph, world_position: [f32; 3]) {
+        // this.#camTarget.parent?.worldMatrix ?? mat4.identity()
+        let parent_world_matrix = match scene.nodes[self.cam_target].parent {
+            Some(parent) => scene.nodes[parent].world_matrix,
+            None => m4::identity(),
+        };
+        let inv = m4::inverse(&parent_world_matrix);
+        scene.nodes[self.cam_target].source.as_mut().unwrap().translation =
+            vec3::transform_mat4(world_position, &inv);
+    }
```

We also need to add `vec3::transform_mat4` which is the same math
we use in our vertex shader for `uni.matrix * vert.position` just
translated to Rust.

```rust
mod vec3 {
  ...

+    pub fn transform_mat4(v: [f32; 3], m: &[f32; 16]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        let x = v[0];
+        let y = v[1];
+        let z = v[2];
+        let w = m[3] * x + m[7] * y + m[11] * z + m[15];
+        let w = if w == 0.0 { 1.0 } else { w }; // the JS version's `|| 1`
+
+        dst[0] = (m[0] * x + m[4] * y + m[8] * z + m[12]) / w;
+        dst[1] = (m[1] * x + m[5] * y + m[9] * z + m[13]) / w;
+        dst[2] = (m[2] * x + m[6] * y + m[10] * z + m[14]) / w;
+
+        dst
+    }
}
```

We also need to refactor the helper's `track` function to
take into account it might not be at the root and adjust the delta
to be relative to the camera's parent.

```rust
    fn track(&self, cam: &OrbitCamera, scene: &mut SceneGraph, delta_x: f32, delta_y: f32) {
-        let direction = vec3::transform_mat3([delta_x, delta_y, 0.0], &self.start_camera_matrix);
-        cam.set_target(scene, vec3::add(self.start_target, direction));
+        let world_direction =
+            vec3::transform_mat3([delta_x, delta_y, 0.0], &self.start_camera_matrix);
+        // this.#camTarget.parent?.worldMatrix ?? mat4.identity()
+        let parent_world_matrix = match scene.nodes[cam.cam_target].parent {
+            Some(parent) => scene.nodes[parent].world_matrix,
+            None => m4::identity(),
+        };
+        let inv = m4::inverse(&parent_world_matrix);
+        let camera_direction = vec3::transform_mat3(world_direction, &inv);
+        scene.nodes[cam.cam_target].source.as_mut().unwrap().translation =
+            vec3::add(self.start_target, camera_direction);
    }
```

The direction we were computing before was a direction in world space.
That worked when the camera was at the root. Now though, we multiply
by the inverse of the camera's parent worldMatrix. This effectively
changes the delta to be relative to the that parent which is what
we need.

Let's put the camera on some extra scene graph nodes

```rust
    let orbit_camera = OrbitCamera::new(&mut scene);
-    orbit_camera.set_parent(&mut scene, root);
+    let extra_rot = add_trs_scene_graph_node(
+        &mut scene,
+        "extra-rot",
+        Some(root),
+        TRS {
+            rotation: [0.0, 0.0, std::f32::consts::PI * 0.35],
+            ..Default::default()
+        },
+    );
+    let extra_mov = add_trs_scene_graph_node(
+        &mut scene,
+        "extra-mov",
+        Some(extra_rot),
+        TRS {
+            translation: [-30.0, -90.0, 40.0],
+            ..Default::default()
+        },
+    );
+    orbit_camera.set_parent(&mut scene, extra_mov);
    orbit_camera.set_target(&mut scene, [120.0, 80.0, 0.0]);
```

(and the page's mirror gets the same `extra-rot` and `extra-mov` nodes so
the node indices keep matching.)

You should set tracking still works.

{{{example url="../webgpu-camera-controls-scene-graph-step-07.html"}}}

## <a id="a-frame-selected"></a> Frame Selected

One more important feature is being able to select an object and then pick "Frame Selected"
to move the camera to show that object. To do that requires knowing how large each
object is. For this specific case, we happen to know everything on the screen is a unit cube.
We can store some extents on our data but for now just set them all to cover our cube.

```rust
+struct Aabb {
+    min: [f32; 3],
+    max: [f32; 3],
+}

#[rustfmt::skip]
-fn create_cube_vertices() -> (Vec<f32>, u32) {
+fn create_cube_vertices() -> (Vec<f32>, u32, Aabb) {
    let positions: Vec<f32> = vec![
        // left
        0.0, 0.0,  0.0,
        0.0, 0.0, -1.0,
        0.0, 1.0,  0.0,
        0.0, 1.0, -1.0,

        // right
        1.0, 0.0,  0.0,
        1.0, 0.0, -1.0,
        1.0, 1.0,  0.0,
        1.0, 1.0, -1.0,
    ];

    ...

-    (vertex_data, num_vertices)
+    (vertex_data, num_vertices, Aabb {
+        min: [ 0.0,  0.0, -1.0],
+        max: [ 1.0,  1.0,  0.0],
+    })
}
```

`Aabb` stands for Axis Aligned Bounding Box. We can easily see
this matches our cube. If we had different data we'd have to scan it
for the min and max values.

We need to bubble this data up to our mesh vertices

```rust
struct Vertices {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
+    aabb: Aabb,
}

fn create_vertices(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
-    (vertex_data, num_vertices): (Vec<f32>, u32),
+    (vertex_data, num_vertices, aabb): (Vec<f32>, u32, Aabb),
    name: &str,
) -> Vertices {
    let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(&format!("{name}: vertex buffer vertices")),
        size: (vertex_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
    Vertices {
        vertex_buffer,
        num_vertices,
+        aabb,
    }
}
```

We need a function that given a mesh, computes the AABB for that
mesh in world space since it will have been oriented by our scene graph.

```rust
fn compute_aabb_for_mesh(mesh: &Mesh, scene: &SceneGraph, vertex_sets: &[Vertices]) -> Aabb {
    let mat = &scene.nodes[mesh.node].world_matrix;
    let p0 = vertex_sets[mesh.vertices].aabb.min;
    let p1 = vertex_sets[mesh.vertices].aabb.max;
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for i in 0..8 {
        let p = [
            if i & 1 != 0 { p0[0] } else { p1[0] },
            if i & 2 != 0 { p0[1] } else { p1[1] },
            if i & 4 != 0 { p0[2] } else { p1[2] },
        ];
        let p = vec3::transform_mat4(p, mat);
        if i == 0 {
            min = p;
            max = p;
        } else {
            min = vec3::min(min, p);
            max = vec3::max(max, p);
        }
    }
    Aabb { min, max }
}
```

This used 2 more `vec3` functions we need to add. `min`, and `max`
that return the a `vec3` that contains the min or max of each component
of 2 vec3s.

```rust
mod vec3 {
  ...

+    pub fn min(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0].min(b[0]);
+        dst[1] = a[1].min(b[1]);
+        dst[2] = a[2].min(b[2]);
+
+        dst
+    }
+
+    pub fn max(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0].max(b[0]);
+        dst[1] = a[1].max(b[1]);
+        dst[2] = a[2].max(b[2]);
+
+        dst
+    }

  ...
}
```

Then, we need a function to go through the selected meshes and gives
us their combined AABB.

```rust
fn expand_aabb_in_place(aabb: &mut Aabb, other_aabb: &Aabb) {
    aabb.min = vec3::min(aabb.min, other_aabb.min);
    aabb.max = vec3::max(aabb.max, other_aabb.max);
}

fn get_aabb_for_selected_meshes(
    selected_meshes: &[&Mesh],
    scene: &SceneGraph,
    vertex_sets: &[Vertices],
) -> Option<Aabb> {
    if selected_meshes.is_empty() {
        return None;
    }
    let mut aabb = compute_aabb_for_mesh(selected_meshes[0], scene, vertex_sets);
    for mesh in &selected_meshes[1..] {
        expand_aabb_in_place(
            &mut aabb,
            &compute_aabb_for_mesh(mesh, scene, vertex_sets),
        );
    }
    Some(aabb)
}
```

With that we can write the code that frames the selected meshes. In the
JavaScript version this is a `frameSelected` function called by a GUI
button. In our port the page's button just bumps a `frameSelected` setting
and the Rust side runs the code once per press, in the frame callback,
right after `selected_meshes` has been gathered.

```rust
+        // The page's "frame selected" button bumps the `frameSelected`
+        // setting; run the JS version's frameSelected() once per press.
+        let frame_selected_id = wgpu_fun::setting_f64("frameSelected", 0.0);
+        if frame_selected_id != last_frame_selected_id {
+            last_frame_selected_id = frame_selected_id;
+            if !selected_meshes.is_empty() {
+                // In the JS version the world matrices are up to date from
+                // the previous render; make sure they are here too.
+                scene.update_world_matrix(root);
+
+                // get aabb bounds for the selected objects.
+                let aabb =
+                    get_aabb_for_selected_meshes(&selected_meshes, &scene, &vertex_sets).unwrap();
+
+                let extent = vec3::subtract(aabb.max, aabb.min);
+                let diameter = vec3::distance(aabb.min, aabb.max);
+
+                // compute how far we need to set the radius for the selected
+                // objects to be framed.
+                let aspect = frame.width as f32 / frame.height as f32;
+                let field_of_view_h = 2.0 * (field_of_view.tan() * aspect).atan();
+                let fov = field_of_view_h.min(field_of_view);
+                let zoom_scale = 1.5; // make it 1.5 times as large for some padding.
+                let half_size = diameter * zoom_scale * 0.5;
+                let distance = half_size / (fov * 0.5).tan();
+
+                orbit_camera.set_radius(&mut scene, distance);
+
+                // point the camera at the center
+                let center = vec3::add_scaled(aabb.min, extent, 0.5);
+                orbit_camera.set_target(&mut scene, center);
+            }
+        }
```

The code above gets the AABB for the selected meshes. The diameter
of a sphere that would contain this AABB is just the distance between
2 opposite corners. Once we have that diameter we compute how far away
a camera needs to be give its current `field_of_view`. The field of view
setting of our `m4::perspective` function is the vertical field of view.
so based on that and the aspect we horizontal field of view and use
whichever is smaller and then use that to compute how far away we need
to be so our sphere would fit. We use `zoom_scale` to make our sphere 1.5x
as large as the sphere that contains our AABB so we'll get some padding.
We then just the radius of the camera to that distance.

Finally we point the camera's target at the AABB's center point.

We need to supply a few more `vec3` functions, `distance` and `add_scaled`

```rust
mod vec3 {
  ...

+    pub fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
+        let dx = a[0] - b[0];
+        let dy = a[1] - b[1];
+        let dz = a[2] - b[2];
+        (dx * dx + dy * dy + dz * dz).sqrt()
+    }

  ...

+    pub fn add_scaled(a: [f32; 3], b: [f32; 3], scale: f32) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0] + b[0] * scale;
+        dst[1] = a[1] + b[1] * scale;
+        dst[2] = a[2] + b[2] * scale;
+
+        dst
+    }

  ...
}
```

`distance` computes the distance between 2 `vec3`s. `add_scaled` effectively
does `a + b * scale`. It makes it easy to add some portion of `b` to `a`.

We need a `field_of_view` we can share between the projection matrix and
the framing code

```rust
+    let field_of_view = 60.0f32.to_radians();

    ...

        let aspect = frame.width as f32 / frame.height as f32;
        let projection = m4::perspective(
-            60.0f32.to_radians(), // fieldOfView,
+            field_of_view,
            aspect,
            1.0,    // zNear
            2000.0, // zFar
        );
```

We also need to add a "frame selected" button on the page

```js
  gui.add(settings, 'showMeshNodes').onChange(showMeshNodes);
  gui.add(settings, 'showAllTRS').onChange(showTRS);
+  // each press bumps the frameSelected setting so the Rust module runs
+  // frameSelected() once per press
+  let frameSelectedId = 0;
+  gui.addButton('frame selected', () => wasm.set_setting_num('frameSelected', ++frameSelectedId));
  const trsFolder = gui.addFolder('orientation').listen();
```

Let's also add a parent node that contains
all 4 cabinets. That way we'll have something to
select that we can frame the entire thing.

```rust
+    let cabinets = add_trs_scene_graph_node(&mut scene, "cabinets", Some(root), TRS::default());
    // Add cabinets
    for cabinet_ndx in 0..K_NUM_CABINETS {
-        add_cabinet(&mut scene, &mut meshes, root, cabinet_ndx);
+        add_cabinet(&mut scene, &mut meshes, cabinets, cabinet_ndx);
    }
```

(with the matching change on the page's mirror, where the default selected
node also becomes `cabinets.children[1]`.)

And while we're at it lets remove the extra rotation and translation

```rust
-    let extra_rot = add_trs_scene_graph_node(
-        &mut scene,
-        "extra-rot",
-        Some(root),
-        TRS {
-            rotation: [0.0, 0.0, std::f32::consts::PI * 0.35],
-            ..Default::default()
-        },
-    );
-    let extra_mov = add_trs_scene_graph_node(
-        &mut scene,
-        "extra-mov",
-        Some(extra_rot),
-        TRS {
-            translation: [-30.0, -90.0, 40.0],
-            ..Default::default()
-        },
-    );
+    let extra_rot = add_trs_scene_graph_node(&mut scene, "extra-rot", Some(root), TRS::default());
+    let extra_mov =
+        add_trs_scene_graph_node(&mut scene, "extra-mov", Some(extra_rot), TRS::default());
```

Try selecting an object and the picking "Frame selected".

{{{example url="../webgpu-camera-controls-scene-graph-step-08.html"}}}

## <a id="a-ux"></a> UX decisions

There are TONs of UX decisions related to an orbit camera that you'll need to make.
Some off of them include:

* Should it allow roll?

  Roll is like when you tilt your head left / right.

  Adding roll would just be a matter of adding one more node at the end
  with a z rotation of our current rig between `cam_extend` and `cam`.

* Should it be like we have it, just letting you drag, or should you it require some other way to adjust
  the camera.

  In Unity, you have to hold a key or switch to camera controlling mode by
  clicking an icon. In Blender you click and drag on certain icons or using the
  middle mouse button and modifier keys. Dragging on the "track camera" icon
  tracks the camera. Dragging the "orbit camera" icon orbits the camera.
  Dragging on the zoom icon zooms (dollies) the camera.

  For a viewer it's nice to be able to just drag with no keys or icons. For an
  editor where most activity is editing 3d content it's probably better to use
  an icon, add a mode, or have the user hold a key.

* What should happen on mobile?

  We didn't provide a solution for tracking the camera on mobile. Our only current method requires the middle mouse button. Using an icon to drag on would
  work. I think some viewers use 2 fingers to track.

* Should it allow tilting past 90 degrees?

  We allowed going past 90 degrees which means the camera can go upside down.
  Some apps prevent that.

* Should "frame" keep the same orientation?

  Most 3D editors let you select an object and pick "Frame" which centers that object
  in the camera AND makes the camera orbit that object. The question is, does
  the orientation of the camera reset, like say, view from the front of the object. Or maybe it always switches to looking along positive Z.
  Or, does it keep whatever orientation it was before picking "frame". For example, if
  you were looking down on object A and the selected B, should it still be looking down?

* Which way does the camera move relative to the pointer?

  In other words, if you drag the pointer from left to right should the camera
  rotate clockwise or counterclockwise. counterclockwise makes it seem like
  your orbiting the camera. clockwise makes it seem like your turing the world
  under the camera. This is similar to dragging two fingers on a trackpad to
  scroll. If you drag down, should the content go up, because you're dragging
  the view over the content. Or should the content down, as though you're dragging
  the content itself.

  With touch screens you generally want it to look like your dragging the content
  but scrollbars existed before touch screens. Dragging the handle on the scroll bar
  drags the view, not the content. Scroll wheels moved that handle. Two fingers
  on a trackpad was a shortcut for that scroll wheel.

## <a id="a-no-scene-graph"></a> Implementing an OrbitCamera without a scene graph.

If you understood how a scene graph works from [the article on scene graphs](webgpu-scene-graphs.html)
then it should be pretty clear. We just need code like

```rust
// An OrbitCamera that is not based on scene graph nodes. It keeps its own
// target/pan/tilt/radius and does the math itself.
struct OrbitCamera {
    target: [f32; 3],
    pan: f32,
    tilt: f32,
    radius: f32,
}

impl OrbitCamera {
    fn new() -> Self {
        OrbitCamera {
            target: vec3::create(),
            pan: 0.0,
            tilt: 0.0,
            radius: 0.0,
        }
    }

    fn get_camera_matrix(&self, parent_matrix: Option<&[f32; 16]>) -> [f32; 16] {
        let mut mat = match parent_matrix {
            Some(m) => *m,
            None => m4::identity(),
        };
        mat = m4::translate(&mat, self.target);
        mat = m4::rotate_y(&mat, self.pan);
        mat = m4::rotate_x(&mat, self.tilt);
        mat = m4::translate(&mat, [0.0, 0.0, self.radius]);
        mat
    }

    fn set_target(&mut self, world_position: [f32; 3], parent_matrix: Option<&[f32; 16]>) {
        let inv = m4::inverse(&match parent_matrix {
            Some(m) => *m,
            None => m4::identity(),
        });
        self.target = vec3::transform_mat4(world_position, &inv);
    }

    fn get_update_helper(&self) -> UpdateHelper {
        UpdateHelper {
            start_tilt: self.tilt,
            start_pan: self.pan,
            start_radius: self.radius,
            start_camera_matrix: self.get_camera_matrix(None),
            start_target: self.target,
        }
    }
}

struct UpdateHelper {
    start_tilt: f32,
    start_pan: f32,
    start_radius: f32,
    start_camera_matrix: [f32; 16],
    start_target: [f32; 3],
}

impl UpdateHelper {
    fn pan_and_tilt(&self, cam: &mut OrbitCamera, delta_pan: f32, delta_tilt: f32) {
        cam.tilt = self.start_tilt - delta_tilt;
        cam.pan = self.start_pan - delta_pan;
    }

    fn track(
        &self,
        cam: &mut OrbitCamera,
        delta_x: f32,
        delta_y: f32,
        parent_matrix: Option<&[f32; 16]>,
    ) {
        let world_direction =
            vec3::transform_mat3([delta_x, delta_y, 0.0], &self.start_camera_matrix);
        let inv = m4::inverse(&match parent_matrix {
            Some(m) => *m,
            None => m4::identity(),
        });
        let camera_direction = vec3::transform_mat3(world_direction, &inv);
        cam.target = vec3::add(self.start_target, camera_direction);
    }

    fn dolly(&self, cam: &mut OrbitCamera, delta: f32) {
        cam.radius = self.start_radius + delta;
    }
}
```

The JavaScript version's private fields and getters/setters become plain
struct fields, and since the camera is no longer read and written through
the scene graph the helper methods take `&mut OrbitCamera` directly.

Popping that in our example we need one more minor change. Since it's not in the scene graph
we need to not add it to the scene graph.

```rust
-    let orbit_camera = OrbitCamera::new(&mut scene);
-    let extra_rot = add_trs_scene_graph_node(&mut scene, "extra-rot", Some(root), TRS::default());
-    let extra_mov =
-        add_trs_scene_graph_node(&mut scene, "extra-mov", Some(extra_rot), TRS::default());
-    orbit_camera.set_parent(&mut scene, extra_mov);
-    orbit_camera.set_target(&mut scene, [120.0, 80.0, 0.0]);
-    orbit_camera.set_tilt(&mut scene, std::f32::consts::PI * -0.2);
-    orbit_camera.set_radius(&mut scene, 300.0);
+    let mut orbit_camera = OrbitCamera::new();
+    orbit_camera.set_target([120.0, 80.0, 0.0], None);
+    orbit_camera.tilt = std::f32::consts::PI * -0.2;
+    orbit_camera.radius = 300.0;
```

And it works 

{{{example url="../webgpu-camera-controls-raw.html"}}}

Now that we have a camera, let's make it so you can
[click on objects directly to select them](webgpu-picking.html).

<!-- keep this at the bottom of the article -->
<link href="webgpu-camera-controls.css" rel="stylesheet">
<script type="module" src="webgpu-camera-controls.js"></script>
