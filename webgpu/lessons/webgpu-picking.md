Title: WebGPU Picking
Description: Clicking on Objects 
TOC: Picking

This article is the 3nd in a short series about making parts for a 3D editor.
Each one builds on the previous lesson so you may find them easiest to
understand by reading them in order.

1. [Highlighting](webgpu-highlighting.html)
2. [Camera Controls](webgpu-camera-controls.html)
3. [Picking](webgpu-picking.html) ⬅ you are here

Picking is the act of selecting objects by clicking on the screen
and then figuring out which objects were clicked on.

**A note on how this port handles selection:** in the JavaScript originals,
picking calls `setCurrentSceneGraphNode(node)` which updates both the GUI
and the selection. In our port the selection state lives in the Rust wasm
module (a `selected_node: Option<NodeNdx>`), because that's where picking
happens. The page's GUI still selects nodes — it sends an `"id ndx"`
`selectNode` setting (the id makes each click apply exactly once, since
picks also change the selection), and it forwards its `showMeshNodes`
checkbox so the Rust side knows whether a picked `-mesh` node should be
walked up to its parent. What the wasm module can't do is reach back into
the page, so after you pick by clicking, the highlight updates but the
GUI's arrow marker and orientation folder keep showing the GUI's own last
selection.

```rust
+    // The selected node. The page's GUI changes it via the `selectNode`
+    // setting; picking (below) changes it directly. This is the state
+    // behind the JS version's `setCurrentSceneGraphNode(node)`.
+    let mut selected_node: Option<NodeNdx> = Some(23); // cabinets.children[1]
+    // id of the last selection the page sent
+    let mut last_select_id = 0.0f64;

    app.run(RenderMode::Once, move |frame: &Frame| {
+        // The page's GUI selects nodes by sending an "id ndx" string (ndx
+        // -1 = none); the id makes each click apply exactly once, since
+        // picking below also changes the selection.
+        let select_node = wgpu_fun::setting_str("selectNode", "");
+        let parts: Vec<f64> = select_node
+            .split_whitespace()
+            .filter_map(|v| v.parse().ok())
+            .collect();
+        if let [id, ndx] = parts[..] {
+            if id != last_select_id {
+                last_select_id = id;
+                selected_node = (ndx >= 0.0).then(|| ndx as usize);
+            }
+        }

        ...

        // Gather the meshes the selected node (or any of its children)
        // uses.
        let selected_meshes: Vec<&Mesh> = meshes
            .iter()
-            .filter(|mesh| mesh_uses_node(mesh, &scene, node_ndx))
+            .filter(|mesh| selected_node.is_some_and(|node| mesh_uses_node(mesh, &scene, node)))
            .collect();
```

One more porting note: pointer interaction can't be driven headlessly, so
in native test mode the examples on this page simulate a single click in
the center of the canvas on the first frame and print the picked node with
`wgpu_fun::print`. That code is `#[cfg(not(target_arch = "wasm32"))]` and
doesn't exist in the browser builds.

## CPU Based Picking

In our series on 3D math we learned how to use matrices to
project 3D vertex positions into clip space positions. For picking
we can do the reverse. We can take where the user clicked on the
screen, convert that to clip space positions, then using the inverse
of the matrix that converted vertex positions to clip space, we can
convert clip space positions to vertex space.

Once they are in the same space it's relatively easy to check
if the ray from the front of the current frustum to the back of
the current frustum, intersects any objects.

Let's work down. First we need to decide when the pick. Because
we also use the pointer to move the camera, let's pick on
pointer up, if the user hasn't moved the pointer.

```rust
    let mut start_x = 0.0f32;
    let mut start_y = 0.0f32;
+    let mut moved = false;
    let mut last_mode: Option<Mode> = None;
    let mut cam_helper: Option<UpdateHelper> = None;

    ...

        for event in wgpu_fun::drain_pointer_events() {
            match event {
                PointerEvent::Down { x, y, button } => {
                    const K_DOUBLE_CLICK_TIME_MS: f64 = 300.0;
                    // canvas.setPointerCapture(e.pointerId);
                    pointer_count += 1;
                    drag_button = button;
                    if pointer_count == 1 {
+                        moved = false;
                        if !double_tap_mode {
                            ...
                        }
                    } else {
                        double_tap_mode = false;
                    }
                    ...
                }
                PointerEvent::Move { x, y } => {
                    ...

                    let delta_x = x - start_x;
                    let delta_y = y - start_y;

+                    if pointer_count == 1 && delta_x.hypot(delta_y) > 1.0 {
+                        moved = true;
+                    }

                    ...
                }
-                PointerEvent::Up { .. } => {
+                PointerEvent::Up { x, y, .. } => {
+                    let num_pointers = pointer_count;
                    // pointerToLastPosition.delete(e.pointerId);
                    pointer_count = (pointer_count - 1).max(0);
                    // canvas.releasePointerCapture(e.pointerId);
                    cam_helper = None;
-                    if pointer_count == 0 {
+                    if num_pointers == 1 && pointer_count == 0 {
                        double_tap_mode = false;
+                        if !moved {
+                            // pickMeshes(e, cam). The world matrices are up
+                            // to date from the previous render in the JS
+                            // version; make sure they are here too.
+                            scene.update_world_matrix(root);
+                            if let Some(node) = pick_meshes(
+                                x,
+                                y,
+                                frame.width,
+                                frame.height,
+                                &orbit_camera,
+                                &scene,
+                                &meshes,
+                                &vertex_sets,
+                                field_of_view,
+                            ) {
+                                // setCurrentSceneGraphNode(node)
+                                selected_node = Some(node);
+                            }
+                        }
                    }
                }
```

With that we're calling `pick_meshes` if the user hasn't moved
the pointer. We need to supply that function, but before that
we're going to need a view projection matrix so let's pull out
the current view project matrix code.

```rust
+fn get_view_projection_matrix(
+    cam: &OrbitCamera,
+    scene: &SceneGraph,
+    field_of_view: f32,
+    width: u32,
+    height: u32,
+) -> [f32; 16] {
+    let aspect = width as f32 / height as f32;
+    let projection = m4::perspective(
+        field_of_view,
+        aspect,
+        1.0,    // zNear
+        2000.0, // zFar
+    );
+
+    let view_matrix = m4::inverse(&cam.get_camera_matrix(scene));
+
+    // combine the view and projection matrixes
+    m4::multiply(&projection, &view_matrix)
+}

   ...

    app.run(RenderMode::Once, move |frame: &Frame| {
        ...

-        let aspect = frame.width as f32 / frame.height as f32;
-        let projection = m4::perspective(
-            field_of_view,
-            aspect,
-            1.0,    // zNear
-            2000.0, // zFar
-        );
-
        scene.update_world_matrix(root);
-
-        // make a view matrix from the camera's
-        let view_matrix = m4::inverse(&orbit_camera.get_camera_matrix(&scene));
-
-        // combine the view and projection matrixes
-        let view_projection_matrix = m4::multiply(&projection, &view_matrix);
+        let view_projection_matrix = get_view_projection_matrix(
+            &orbit_camera,
+            &scene,
+            field_of_view,
+            frame.width,
+            frame.height,
+        );
```

Now we can use that to start making `pick_meshes`. Pointer coordinates
from wgpu_fun are already in device pixels relative to the canvas, so
converting to clip space is just a divide by the canvas size.

```rust
+// pickMeshes(e, cam) in the JS version. Returns the picked node, if any.
+fn pick_meshes(
+    x: f32,
+    y: f32,
+    width: u32,
+    height: u32,
+    cam: &OrbitCamera,
+    scene: &SceneGraph,
+    meshes: &[Mesh],
+    vertex_sets: &[Vertices],
+    field_of_view: f32,
+) -> Option<NodeNdx> {
+    let clip_x = x / width as f32 * 2.0 - 1.0;
+    let clip_y = y / height as f32 * -2.0 + 1.0;
+
+    let view_projection_value = get_view_projection_matrix(cam, scene, field_of_view, width, height);
+    let intersecting_meshes =
+        get_intersecting_meshes(clip_x, clip_y, &view_projection_value, meshes, scene, vertex_sets);
+    ???
+}
```

`pick_meshes` computes a clip space X and Y, a view projection matrix,
and passes them to `get_intersecting_meshes` expecting an array of
meshes.

Let's make `get_intersecting_meshes`. Since our meshes live in a `Vec` and
refer to things by index, the results refer to the hit meshes by index
too.

```rust
// the result of an intersection: where it hit (in clip space) and the
// index of the mesh that was hit
struct IntersectingMesh {
    position: [f32; 3],
    mesh: usize,
}

fn get_intersecting_meshes(
    clip_x: f32,
    clip_y: f32,
    view_projection: &[f32; 16],
    meshes: &[Mesh],
    scene: &SceneGraph,
    vertex_sets: &[Vertices],
) -> Vec<IntersectingMesh> {
    let clip_near = [clip_x, clip_y, 0.0];
    let clip_far = [clip_x, clip_y, 1.0];

    let mut verts = [[0.0f32; 3]; 3];

    let mut intersecting_meshes = Vec::new();
    for (mesh_ndx, mesh) in meshes.iter().enumerate() {
        // put mat in model space (the space of the vertex data)
        let world_view_projection =
            m4::multiply(view_projection, &scene.nodes[mesh.node].world_matrix);

        // invert it so putting in clip space coords will transform them
        // to model space.
        let mat = m4::inverse(&world_view_projection);

        // now transform the clip space coords to model space
        // so we can compare them to the model vertices and AABB
        let near = vec3::transform_mat4(clip_near, &mat);
        let far = vec3::transform_mat4(clip_far, &mat);

        let Vertices {
            vertex_data,
            num_vertices,
            ..
        } = &vertex_sets[mesh.vertices];

        let num_triangles = num_vertices / 3;
        let mut closest: Option<[f32; 3]> = None;
        for t in 0..num_triangles {
            for (i, v) in verts.iter_mut().enumerate() {
                // get the 3 positions for the triangle
                let offset = (t as usize * 3 + i) * 4;
                v[0] = vertex_data[offset];
                v[1] = vertex_data[offset + 1];
                v[2] = vertex_data[offset + 2];
            }

            let result =
                intersect_line_segment_and_triangle(near, far, verts[0], verts[1], verts[2]);
            if let Some(result) = result {
                // Convert back to clip space so we can check Z to keep
                // the closest hit.
                let result = vec3::transform_mat4(result, &world_view_projection);
                if closest.is_none_or(|closest| result[2] < closest[2]) {
                    closest = Some(result);
                }
            }
        }

        if let Some(closest) = closest {
            intersecting_meshes.push(IntersectingMesh {
                position: closest,
                mesh: mesh_ndx,
            });
        }
    }

    intersecting_meshes
}
```

I hope this code is relatively straight forward. It creates `clip_near`
and `clip_far`. These are easy as they're just the `clip_x` and `clip_y`
that were passed in with `clip_near` z set to 0 and `clip_far` set to 1.

Then, for each mesh we get its `world_matrix` and multiply with our
camera's view projection. We then take the inverse. This lets us
convert `clip_near` and `clip_far` to the same positions but in the
same space as the vertex data. We call the results `near` and `far`.

We then walk the triangles of the vertex data and for each one
call `intersect_line_segment_and_triangle` which will return `None`
if the `near` `far` line segment does not intersect, or, it returns
where the intersection happened if it did.

We convert back to clip space so the positions are oriented back
relative to the viewer. This lets us keep the closest point relative
to the camera.

If we found any one of the triangles interested then we push that
mesh onto our results.

With that in place we can go back and finish `pick_meshes`

```rust
// pickMeshes(e, cam) in the JS version. Returns the picked node, if any.
fn pick_meshes(
    x: f32,
    y: f32,
    width: u32,
    height: u32,
    cam: &OrbitCamera,
    scene: &SceneGraph,
    meshes: &[Mesh],
    vertex_sets: &[Vertices],
    field_of_view: f32,
) -> Option<NodeNdx> {
    let clip_x = x / width as f32 * 2.0 - 1.0;
    let clip_y = y / height as f32 * -2.0 + 1.0;

    let view_projection_value = get_view_projection_matrix(cam, scene, field_of_view, width, height);
    let mut intersecting_meshes =
        get_intersecting_meshes(clip_x, clip_y, &view_projection_value, meshes, scene, vertex_sets);

    // sort the results by their z
    intersecting_meshes.sort_by(|a, b| a.position[2].total_cmp(&b.position[2]));

    // pick the first one
    if !intersecting_meshes.is_empty() {
        let mut node = meshes[intersecting_meshes[0].mesh].node;
        if !wgpu_fun::setting_bool("showMeshNodes", false) {
            while scene.nodes[node].name.contains("mesh") {
                node = scene.nodes[node].parent.unwrap();
            }
        }
        Some(node)
    } else {
        None
    }
}
```

We still have a few more things we need to do. We need to
supply `intersect_line_segment_and_triangle`. This is called
[The Möller–Trumbore ray-triangle intersection algorithm](https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm).

```rust
// https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
fn intersect_line_segment_and_triangle(
    p0: [f32; 3],
    p1: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<[f32; 3]> {
    let edge1 = vec3::subtract(v1, v0);
    let edge2 = vec3::subtract(v2, v0);
    let dir = vec3::subtract(p1, p0); // Line segment direction

    let h = vec3::cross(dir, edge2);
    let a = vec3::dot(edge1, h);

    // If 'a' is near zero, the line is parallel
    // to the triangle's plane
    if a.abs() < 0.00001 {
        return None;
    }

    let f = 1.0 / a;
    let s = vec3::subtract(p0, v0);
    let u = f * vec3::dot(s, h);

    // Check if the intersection point is outside
    // the triangle's U parameter range [0, 1]
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q = vec3::cross(s, edge1);
    let v = f * vec3::dot(dir, q);

    // Check if the intersection point is outside
    // the triangle's V parameter range [0, 1] or S+T range [0, 1]
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // At this stage, the intersection point lies on
    // the infinite line and within the triangle
    let t = f * vec3::dot(edge2, q);

    // Check if the intersection point lies within
    // the line segment's T parameter range [0, 1]
    if !(0.0..=1.0).contains(&t) {
        return None;
    }

    // Return the intersection point
    Some(vec3::add_scaled(p0, dir, t))
}
```

That calls `vec3::dot` so we need to supply it.

```rust
mod vec3 {
  ...

+    pub fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
+        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
+    }

}
```

We've used `dot` in [the articles on lighting](webgpu-lighting-directional.html) among other places. It multiplies corresponding components
of 2 vec3s and adds the results.

We also need to keep around the vertex data.

```rust
struct Vertices {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    aabb: Aabb,
+    vertex_data: Vec<f32>,
}

fn create_vertices(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    (vertex_data, num_vertices, aabb): (Vec<f32>, u32, Aabb),
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
        aabb,
+        vertex_data,
    }
}
```

And with that we can pick!

{{{example url="../webgpu-picking-cpu-step-01.html"}}}

It would be nice if we click no where we unselect
whatever is currently selected. Let's do that

```rust
        // pick the first one
        if !intersecting_meshes.is_empty() {
            ...
            Some(node)
        } else {
            None
        }

    ...

                        if !moved {
                            scene.update_world_matrix(root);
-                            if let Some(node) = pick_meshes(
-                                ...
-                            ) {
-                                // setCurrentSceneGraphNode(node)
-                                selected_node = Some(node);
-                            }
+                            // setCurrentSceneGraphNode(node or undefined)
+                            selected_node = pick_meshes(
+                                ...
+                            );
                        }
```

`pick_meshes` already returns `None` on a miss so all we change is
assigning the result directly instead of only assigning on a hit.
The page-side GUI gets the small tweaks from the JS original — a
`--none--` label and a stand-in for the missing `TRS` — and starts with
nothing selected, which on the Rust side is just a different default:

```rust
-    let mut selected_node: Option<NodeNdx> = Some(23); // cabinets.children[1]
+    let mut selected_node: Option<NodeNdx> = None; // setCurrentSceneGraphNode(undefined)
```

{{{example url="../webgpu-picking-cpu-step-02.html"}}}

A problem we have right now is we can only select the closest object.
A good thing about our code is we get a list of all objects that are under
the user's pointer. It's common in an editor that on the first click
the closest object is picked. On a 2nd click, if the pointer has not moved,
then the next object is picked. This repeats until we've cycled through all
the objects under the pointer. Let's do that.

```rust
+// the lastPickX/lastPickY/lastPickNdx/lastIntersectingMeshes variables
+// from the JS version
+#[derive(Default)]
+struct LastPick {
+    x: f32,
+    y: f32,
+    ndx: usize,
+    intersecting_meshes: Option<Vec<IntersectingMesh>>,
+}

// pickMeshes(e, cam) in the JS version. Returns the picked node, if any.
fn pick_meshes(
    x: f32,
    y: f32,
    width: u32,
    height: u32,
    cam: &OrbitCamera,
    scene: &SceneGraph,
    meshes: &[Mesh],
    vertex_sets: &[Vertices],
    field_of_view: f32,
+    last_pick: &mut LastPick,
) -> Option<NodeNdx> {
+    if last_pick.intersecting_meshes.is_none() || last_pick.x != x || last_pick.y != y {
+        last_pick.ndx = 0;
+        last_pick.x = x;
+        last_pick.y = y;
+
        let clip_x = x / width as f32 * 2.0 - 1.0;
        let clip_y = y / height as f32 * -2.0 + 1.0;

        let view_projection_value =
            get_view_projection_matrix(cam, scene, field_of_view, width, height);
-        let mut intersecting_meshes =
-            get_intersecting_meshes(clip_x, clip_y, &view_projection_value, meshes, scene, vertex_sets);
-
-        // sort the results by their z
-        intersecting_meshes.sort_by(|a, b| a.position[2].total_cmp(&b.position[2]));
-    }
-
-    // pick the first one
-    if !intersecting_meshes.is_empty() {
-        let mut node = meshes[intersecting_meshes[0].mesh].node;
+        let mut intersecting_meshes = get_intersecting_meshes(
+            clip_x,
+            clip_y,
+            &view_projection_value,
+            meshes,
+            scene,
+            vertex_sets,
+        );
+        intersecting_meshes.sort_by(|a, b| a.position[2].total_cmp(&b.position[2]));
+        last_pick.intersecting_meshes = Some(intersecting_meshes);
+    }
+
+    // Cycle through the results
+    let intersecting_meshes = last_pick.intersecting_meshes.as_ref().unwrap();
+    if !intersecting_meshes.is_empty() {
+        let mut node = meshes[intersecting_meshes[last_pick.ndx].mesh].node;
+        last_pick.ndx = (last_pick.ndx + 1) % intersecting_meshes.len();
        if !wgpu_fun::setting_bool("showMeshNodes", false) {
            while scene.nodes[node].name.contains("mesh") {
                node = scene.nodes[node].parent.unwrap();
            }
        }
        Some(node)
    } else {
        None
    }
}
```

Now if you click a drawer you'll select the drawer. If you click again
without moving the pointer, you'll select the cabinet behind the drawer

{{{example url="../webgpu-picking-cpu-step-03.html"}}}

A common optimization we can make is to check if the ray intersects
the AABB of the vertex data. If it does not intersect then there's
no reason to check all of the triangles.

We added an AABB in
[the previous article](webgpu-camera-controls.html#a-frame-selected) in
order to implement "frame selected" so we have the data. All we need
to do is add the check.

```rust
fn get_intersecting_meshes(
    ...

        let Vertices {
            vertex_data,
            num_vertices,
+            aabb,
            ..
        } = &vertex_sets[mesh.vertices];

+        // check if the ray passes through the AABB.
+        if intersect_segment_aabb(near, far, aabb).is_none() {
+            // no so skip checking every triangle
+            continue;
+        }

    ...
}
```

Here's the code for checking the a ray with an AABB.

```rust
// Branchless slab ray/segment–AABB intersection (Williams et al.)
// note: unoptimized for JS.
const K_EPSILON: f32 = 1e-12;
fn intersect_segment_aabb(p0: [f32; 3], p1: [f32; 3], aabb: &Aabb) -> Option<(f32, f32)> {
    let delta = vec3::subtract(p1, p0);

    let inv_delta: [f32; 3] = std::array::from_fn(|i| {
        let v = delta[i];
        1.0 / if v.abs() > K_EPSILON {
            v
        } else {
            v.signum() * K_EPSILON
        }
    });

    let t0 = vec3::multiply(vec3::subtract(aabb.min, p0), inv_delta);
    let t1 = vec3::multiply(vec3::subtract(aabb.max, p0), inv_delta);

    let min = vec3::min(t0, t1);
    let max = vec3::max(t0, t1);

    let t_min = min.iter().fold(0.0f32, |a, &b| a.max(b));
    let t_max = max.iter().fold(1.0f32, |a, &b| a.min(b));

    for c in 0..3 {
        if delta[c].abs() <= K_EPSILON && (p0[c] < aabb.min[c] || p0[c] > aabb.max[c]) {
            return None;
        }
    }

    if t_min > t_max {
        None
    } else {
        Some((t_min, t_max))
    }
}
```

We need to add `vec3::multiply`

```rust
mod vec3 {
  ...

+    pub fn multiply(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0] * b[0];
+        dst[1] = a[1] * b[1];
+        dst[2] = a[2] * b[2];
+
+        dst
+    }

  ...
};
```

Because our cabinets are made from scaled unit cubes, our bounding
box perfect matches our cubes. So, just to make sure it's all working
let's add our F back in that we used in other articles.

```rust
+fn compute_aabb_for_vertices(vertex_data: &[f32], stride: usize) -> Aabb {
+    let num_vertices = vertex_data.len() / stride;
+    let mut min = [vertex_data[0], vertex_data[1], vertex_data[2]];
+    let mut max = min;
+
+    for i in 1..num_vertices {
+        let offset = i * stride;
+        let p = [
+            vertex_data[offset],
+            vertex_data[offset + 1],
+            vertex_data[offset + 2],
+        ];
+        min = vec3::min(min, p);
+        max = vec3::max(max, p);
+    }
+    Aabb { min, max }
+}
+
+fn create_f_vertices() -> (Vec<f32>, u32, Aabb) {
  ...

+    let aabb = compute_aabb_for_vertices(&vertex_data, 4);
+    (vertex_data, num_vertices, aabb)
}
```

We just needed to compute the F's AABB

Now let's add it to the scene just before we add the cabinets.

```rust
+const K_F_VERTICES: usize = 1;

    ...

    let vertex_sets = vec![
        create_vertices(&app.device, &app.queue, create_cube_vertices(), "cube"),
+        create_vertices(&app.device, &app.queue, create_f_vertices(), "f"),
    ];

    ...

+    {
+        let node = add_trs_scene_graph_node(
+            &mut scene,
+            "f",
+            Some(root),
+            TRS {
+                translation: [100.0, 75.0, 30.0],
+                rotation: [std::f32::consts::PI, std::f32::consts::PI * 0.33, 0.0],
+                scale: [0.5, 0.5, 0.5],
+            },
+        );
+        add_mesh(&mut meshes, node, K_F_VERTICES, [1.0, 1.0, 1.0, 1.0]);
+    }

    let cabinets = add_trs_scene_graph_node(&mut scene, "cabinets", Some(root), TRS::default());
    // Add cabinets
    for cabinet_ndx in 0..K_NUM_CABINETS {
        add_cabinet(&mut scene, &mut meshes, cabinets, cabinet_ndx);
    }
```

There's not really anything to see. It's just slightly optimized.

{{{example url="../webgpu-picking-cpu-step-04.html"}}}

The problem with CPU based picking is it's potentially slow and it's a bunch
of work to make it keep up with any new GPU based rendering features we add.
It also requires we keep access to the vertex data for the CPU.

## <a id="a-gpu-picking"></a> GPU Picking

We can also pick with the GPU. We do it by, instead of drawing each object
with a color, we draw each object with an integer ID. We then look at the texel under
the pointer. Whatever ID we see is the ID of the object that was clicked on.

<div class="webgpu_center">
  <div data-diagram="id-render" style="width: 1200px; max-width: 80%;"></div>
  <div>drag to rotate</div>
</div>

Above is a render of a cube, a sphere, an a pyramid. Each has its id rendered over it.

To do that we need a way to render the objects with ids. We have a few options. 

1. ## We could add a 2nd output to our shader

   Our fragment shader is currently returning a single color

   ```wgsl
   @fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
      return vsOut.color * uni.color;
   }
   ```

   We could change it to return both a color and an id.

   ```wgsl
    struct Uniforms {
      matrix: mat4x4f,
      color: vec4f,
   +   id: u32,
    };

   +struct MyOutput {
   +  @location(0) color: vec4f,
   +  @location(1) id: vec4u,
   +};

   -@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
   -   return vsOut.color * uni.color;
   +@fragment fn fs(vsOut: VSOutput) -> MyOutput {
   +   return MyOutput(
   +     vsOut.color * uni.color,
   +     uni.id,
   +   );
   }
   ```

   This method has the advantage that we only need to render once and we get
   both the image and ids.

2. ## We could render twice, once for color, once for ids

   I'm going to choose this method for now for reasons that will hopefully become clear after this step. [^render-twice]

   [^render-twice]: Method 2 was chosen because we needed a way to selectively render for picking in order to implement cycling
   through all objects under the pointer.

So, first let's add the id to our uniforms and create a fragment shader
that outputs ids.

```wgsl
struct Uniforms {
  matrix: mat4x4f,
  color: vec4f,
+  id: u32,
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
  return vsOut.color * uni.color;
}

+@fragment fn fsPicking(vsOut: VSOutput) -> @location(0) vec4u {
+  return vec4u(uni.id);
+}
```

As we mentioned early on, bindGroups made from pipelines that use `layout: None`
(JavaScript's `layout: 'auto'`) can not be shared. We'd like to use the same
bindGroups with both fragment shaders so we need to manually create a
bindGroupLayout and pipelineLayout.

```rust
    let bind_group_layout = app
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(96),
                },
                count: None,
            }],
        });

    let pipeline_layout = app
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
```

We can then update our existing pipeline and also create a new one for rendering
the ids.

```rust
    let pipeline = app
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("2 attributes with color"),
-            layout: None,
+            layout: Some(&pipeline_layout),
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
-                entry_point: None,
+                entry_point: Some("fs"),
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

+    let pick_pipeline = app
+        .device
+        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
+            label: Some("2 attributes with id for picking"),
+            layout: Some(&pipeline_layout),
+            vertex: wgpu::VertexState {
+                module: &module,
+                entry_point: None,
+                compilation_options: Default::default(),
+                buffers: &[Some(wgpu::VertexBufferLayout {
+                    array_stride: (4) * 4, // (3) floats 4 bytes each + one 4 byte color
+                    step_mode: wgpu::VertexStepMode::Vertex,
+                    attributes: &[
+                        // position
+                        wgpu::VertexAttribute {
+                            shader_location: 0,
+                            offset: 0,
+                            format: wgpu::VertexFormat::Float32x3,
+                        },
+                        // color
+                        wgpu::VertexAttribute {
+                            shader_location: 1,
+                            offset: 12,
+                            format: wgpu::VertexFormat::Unorm8x4,
+                        },
+                    ],
+                })],
+            },
+            fragment: Some(wgpu::FragmentState {
+                module: &module,
+                entry_point: Some("fsPicking"),
+                compilation_options: Default::default(),
+                targets: &[Some(wgpu::TextureFormat::R32Uint.into())],
+            }),
+            primitive: wgpu::PrimitiveState {
+                cull_mode: Some(wgpu::Face::Back),
+                ..Default::default()
+            },
+            depth_stencil: Some(wgpu::DepthStencilState {
+                depth_write_enabled: Some(true),
+                depth_compare: Some(wgpu::CompareFunction::Less),
+                format: wgpu::TextureFormat::Depth24Plus,
+                stencil: Default::default(),
+                bias: Default::default(),
+            }),
+            multisample: Default::default(),
+            multiview_mask: None,
+            cache: None,
+        });
```

We need to update our per object uniform buffers so they have
room for the id and a way to set them.

```rust
-// matrix and color
-const UNIFORM_BUFFER_SIZE: u64 = (16 + 4) * 4;
+// matrix, color, id, padding
+const UNIFORM_BUFFER_SIZE: u64 = (16 + 4 + 1 + 3) * 4;

// offsets to the various uniform values in float32 indices
const K_MATRIX_OFFSET: usize = 0;
const K_COLOR_OFFSET: usize = 16;
+const K_ID_OFFSET: usize = 20;
```

and we need to update the rendering code to include the id.
Where the JavaScript version makes a `Uint32Array` view of the
uniform values, we cast the same bytes with bytemuck.

```rust
fn draw_object(ctx: &mut Ctx, vertices: &Vertices, matrix: [f32; 16], color: [f32; 4]) {
    ...

    let matrix_value = m4::multiply(&ctx.view_projection_matrix, &matrix);
    object_info.uniform_values[K_MATRIX_OFFSET..K_MATRIX_OFFSET + 16]
        .copy_from_slice(&matrix_value);
    object_info.uniform_values[K_COLOR_OFFSET..K_COLOR_OFFSET + 4].copy_from_slice(&color);
+    // set the id (a u32 view of the same uniform data)
+    let as_u32: &mut [u32] = bytemuck::cast_slice_mut(&mut object_info.uniform_values);
+    as_u32[K_ID_OFFSET] = ctx.object_ndx as u32;

    ...
}
```

We need to make it possible to render twice so let's
refactor the scene rendering part of the frame callback into
`render_to_texture`.
We'll pass it a `CommandEncoder`, a `target` view
to render to, a `pipeline` so we can pass on the drawing
pipeline or the id rendering pipeline, and the `view_projection_matrix`.
Two differences from the JavaScript: wgpu_fun hands the frame callback a
`TextureView` for the canvas (not the texture itself), so we pass a view
plus a size, and since Rust has no globals to lean on the function takes
the meshes to draw and returns the number of objects it drew (the JS
version's global `objectNdx`) so later passes can keep allocating
`ObjectInfo`s where it left off.

```rust
+// renderToTexture in the JS version.
+fn render_to_texture(
+    device: &wgpu::Device,
+    queue: &wgpu::Queue,
+    encoder: &mut wgpu::CommandEncoder,
+    target: &wgpu::TextureView,
+    size: (u32, u32),
+    pipeline: &wgpu::RenderPipeline,
+    view_projection_matrix: [f32; 16],
+    meshes: &[&Mesh],
+    scene: &SceneGraph,
+    vertex_sets: &[Vertices],
+    object_infos: &mut Vec<ObjectInfo>,
+    depth_texture: &mut Option<wgpu::Texture>,
+) -> usize {
+    *depth_texture = Some(make_new_texture_if_size_different(
+        device,
+        depth_texture.take(),
+        size, // for size
+        wgpu::TextureFormat::Depth24Plus,
+        wgpu::TextureUsages::RENDER_ATTACHMENT,
+    ));
+    let depth_view = depth_texture
+        .as_ref()
+        .unwrap()
+        .create_view(&Default::default());
+
+    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
+        label: Some("our basic canvas renderPass"),
+        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
+            view: target,
+            resolve_target: None,
+            ops: wgpu::Operations {
+                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
+                store: wgpu::StoreOp::Store,
+            },
+            depth_slice: None,
+        })],
+        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
+            view: &depth_view,
+            depth_ops: Some(wgpu::Operations {
+                load: wgpu::LoadOp::Clear(1.0),
+                store: wgpu::StoreOp::Store,
+            }),
+            stencil_ops: None,
+        }),
+        ..Default::default()
+    });
+    pass.set_pipeline(pipeline);
+
+    let mut ctx = Ctx {
+        pass: &mut pass,
+        view_projection_matrix,
+        device,
+        queue,
+        pipeline,
+        object_infos,
+        object_ndx: 0,
+    };
+    for mesh in meshes {
+        draw_mesh(&mut ctx, mesh, scene, vertex_sets);
+    }
+    ctx.object_ndx
+}
```

and the frame callback's main render becomes

```rust
        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

+        // render the scene to the canvas (renderToTexture in the JS
+        // version; it takes care of the depth texture)
+        let all_meshes: Vec<&Mesh> = meshes.iter().collect();
+        let object_ndx = render_to_texture(
+            frame.device,
+            frame.queue,
+            &mut encoder,
+            frame.view,
+            (frame.width, frame.height),
+            &pipeline,
+            view_projection_matrix,
+            &all_meshes,
+            &scene,
+            &vertex_sets,
+            &mut object_infos,
+            &mut depth_texture,
+        );
```

Now in order to render the pick texture let's make a `pick`
function. Here's where the port differs the most from the JavaScript:
the JS `pick` is an `async` function that ends with
`await pickBuffer.mapAsync(GPUMapMode.READ)`. Our frame callback can't
await, so `pick` hands the mapAsync callback a shared slot; the callback
reads the id, stores it there, and calls `wgpu_fun::request_redraw()`,
and the frame callback finishes the pick when the value shows up. On
native, mapAsync callbacks only fire when the device is polled, so the
frame callback polls while a pick is in flight — which makes the whole
pick resolve within the same frame (in the browser, the browser polls for
us and the redraw request gets us back into the frame callback).

```rust
+    let pick_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+        label: None,
+        size: 4,
+        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
+        mapped_at_creation: false,
+    });
+
+    let mut pick_texture: Option<wgpu::Texture> = None;
+    // where the mapAsync callback leaves the picked id for the frame
+    // callback (the value of the JS version's `await pick(...)`)
+    let pick_result: Arc<Mutex<Option<u32>>> = Arc::new(Mutex::new(None));
+    let mut pick_in_flight = false;
```

```rust
+// The JS `pick` function.
+fn pick(
+    device: &wgpu::Device,
+    queue: &wgpu::Queue,
+    clip_x: f32,
+    clip_y: f32,
+    view_projection_matrix: [f32; 16],
+    canvas_size: (u32, u32),
+    pick_pipeline: &wgpu::RenderPipeline,
+    pick_texture: &mut Option<wgpu::Texture>,
+    depth_texture: &mut Option<wgpu::Texture>,
+    pick_buffer: &wgpu::Buffer,
+    meshes: &[&Mesh],
+    scene: &SceneGraph,
+    vertex_sets: &[Vertices],
+    object_infos: &mut Vec<ObjectInfo>,
+    pick_result: &Arc<Mutex<Option<u32>>>,
+) {
+    let x = ((clip_x * 0.5 + 0.5) * canvas_size.0 as f32).round() as u32;
+    let y = ((clip_y * -0.5 + 0.5) * canvas_size.1 as f32).round() as u32;
+    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
+    *pick_texture = Some(make_new_texture_if_size_different(
+        device,
+        pick_texture.take(),
+        canvas_size, // for size
+        wgpu::TextureFormat::R32Uint,
+        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
+    ));
+
+    render_to_texture(
+        device,
+        queue,
+        &mut encoder,
+        &pick_texture
+            .as_ref()
+            .unwrap()
+            .create_view(&Default::default()),
+        canvas_size,
+        pick_pipeline,
+        view_projection_matrix,
+        meshes,
+        scene,
+        vertex_sets,
+        object_infos,
+        depth_texture,
+    );
+
+    // Copy the texel under the pointer to pickBuffer
+    encoder.copy_texture_to_buffer(
+        wgpu::TexelCopyTextureInfo {
+            texture: pick_texture.as_ref().unwrap(),
+            mip_level: 0,
+            origin: wgpu::Origin3d { x, y, z: 0 },
+            aspect: wgpu::TextureAspect::All,
+        },
+        wgpu::TexelCopyBufferInfo {
+            buffer: pick_buffer,
+            layout: wgpu::TexelCopyBufferLayout::default(),
+        },
+        wgpu::Extent3d {
+            width: 1,
+            height: 1,
+            depth_or_array_layers: 1,
+        },
+    );
+
+    let command_buffer = encoder.finish();
+    queue.submit([command_buffer]);
+
+    // Get the value from the pickBuffer
+    // (the JS version's `await pickBuffer.mapAsync(GPUMapMode.READ)`)
+    let buffer = pick_buffer.clone();
+    let pick_result = pick_result.clone();
+    pick_buffer.map_async(wgpu::MapMode::Read, .., move |result| {
+        result.expect("failed to map pick buffer");
+        let id = {
+            let view = buffer.slice(..).get_mapped_range().unwrap();
+            let ids: &[u32] = bytemuck::cast_slice(&view);
+            ids[0]
+        };
+        buffer.unmap();
+        *pick_result.lock().unwrap() = Some(id);
+        wgpu_fun::request_redraw();
+    });
+}
```

It's pretty straight forward. We convert `clip_x` and `clip_y`
into the texel coordinate under the pointer. We then create
a an `r32uint` texture the same size as the canvas. We render
the scene to this texture using `render_to_texture`. We then
copy the single texel under the pointer to `pick_buffer`.
Then map it and read the value.

To use it we replace the old CPU `pick_meshes` call in the `Up` handler
with starting a pick, and finish it — the code after the `await` in the
JS version — once the id arrives.

```rust
                        if !moved {
-                            scene.update_world_matrix(root);
-                            // setCurrentSceneGraphNode(node or undefined)
-                            selected_node = pick_meshes(...);
+                            // pickMeshes(e, cam) — start the async pick.
+                            scene.update_world_matrix(root);
+                            let clip_x = x / frame.width as f32 * 2.0 - 1.0;
+                            let clip_y = y / frame.height as f32 * -2.0 + 1.0;
+                            let view_projection_matrix = get_view_projection_matrix(
+                                &orbit_camera,
+                                &scene,
+                                field_of_view,
+                                frame.width,
+                                frame.height,
+                            );
+                            let all_meshes: Vec<&Mesh> = meshes.iter().collect();
+                            pick(
+                                frame.device,
+                                frame.queue,
+                                clip_x,
+                                clip_y,
+                                view_projection_matrix,
+                                (frame.width, frame.height),
+                                &pick_pipeline,
+                                &mut pick_texture,
+                                &mut depth_texture,
+                                &pick_buffer,
+                                &all_meshes,
+                                &scene,
+                                &vertex_sets,
+                                &mut object_infos,
+                                &pick_result,
+                            );
+                            pick_in_flight = true;
                        }

        ...

+        // On native, mapAsync callbacks only fire when the device is
+        // polled; poll here so a click's pick finishes this frame (in the
+        // browser, the browser polls for us and the callback's redraw
+        // request gets us back here).
+        #[cfg(not(target_arch = "wasm32"))]
+        if pick_in_flight {
+            frame
+                .device
+                .poll(wgpu::PollType::wait_indefinitely())
+                .expect("device poll failed");
+        }
+
+        // finish a resolved pick: the rest of the JS version's
+        // `pickMeshes` after `const id = await pick(...)`
+        if let Some(id) = pick_result.lock().unwrap().take() {
+            pick_in_flight = false;
+            if id > 0 {
+                let mut node = meshes[id as usize - 1].node;
+                if !wgpu_fun::setting_bool("showMeshNodes", false) {
+                    while scene.nodes[node].name.contains("mesh") {
+                        node = scene.nodes[node].parent.unwrap();
+                    }
+                }
+                // setCurrentSceneGraphNode(node)
+                selected_node = Some(node);
+            } else {
+                // setCurrentSceneGraphNode(undefined)
+                selected_node = None;
+            }
+        }
```

That was quite a few changes but with that we have GPU picking.

{{{example url="../webgpu-picking-gpu-step-01.html"}}}

Unfortunately we lost the ability to cycle though all the
objects under the pointer. Let's fix that. We'll do it
by making a `pickable_meshes` list that is all of the
meshes it's possible to pick. Each time we pick a mesh
we'll remove that mesh from `pickable_meshes`. That means
the next time we click the previously picked mesh won't
be rendered and so we'll get whatever id it was overwriting.
If we don't get any id we'll put all of the meshes back in
`pickable_meshes` and try a 2nd time.

The JS version first changes `renderToTexture` and `pick` to take an
array of meshes; our Rust versions already take one, so all that's left
is the `pickMeshes` logic. `pickable_meshes` keeps mesh *indices*, and
because the "try a 2nd time" happens after an `await` in the JS version,
here it happens when the first pick's result arrives — we keep the pick's
clip coordinates and view projection matrix around so we can issue the
retry.

```rust
+    // lastPickX/lastPickY/pickableMeshes in the JS version. The pickable
+    // meshes are indices into `meshes`.
+    let mut last_pick_x = f32::NAN;
+    let mut last_pick_y = f32::NAN;
+    let mut pickable_meshes: Option<Vec<usize>> = None;
+    // where we are in the JS version's `pickMeshes`: false = the first
+    // `await pick(...)`, true = the retry with all the meshes
+    let mut pick_second_try = false;
+    // the arguments to pass to `pick` again if we need to retry
+    let mut pick_clip = (0.0f32, 0.0f32);
+    let mut pick_view_projection = m4::identity();

    ...

                        if !moved {
                            // pickMeshes(e, cam) — start the async pick.
+                            // if we have no meshes OR the pointer moved
+                            if pickable_meshes.is_none() || last_pick_x != x || last_pick_y != y {
+                                last_pick_x = x;
+                                last_pick_y = y;
+
+                                // get all the meshes.
+                                pickable_meshes = Some((0..meshes.len()).collect());
+                            }
+                            pick_second_try = false;

                            scene.update_world_matrix(root);
                            let clip_x = x / frame.width as f32 * 2.0 - 1.0;
                            let clip_y = y / frame.height as f32 * -2.0 + 1.0;
                            let view_projection_matrix = get_view_projection_matrix(
                                &orbit_camera,
                                &scene,
                                field_of_view,
                                frame.width,
                                frame.height,
                            );
+                            pick_clip = (clip_x, clip_y);
+                            pick_view_projection = view_projection_matrix;
+                            // pick from the available meshes
-                            let all_meshes: Vec<&Mesh> = meshes.iter().collect();
+                            let list: Vec<&Mesh> = pickable_meshes
+                                .as_ref()
+                                .unwrap()
+                                .iter()
+                                .map(|&m| &meshes[m])
+                                .collect();
                            pick(
                                frame.device,
                                frame.queue,
                                clip_x,
                                clip_y,
                                view_projection_matrix,
                                (frame.width, frame.height),
                                &pick_pipeline,
                                &mut pick_texture,
                                &mut depth_texture,
                                &pick_buffer,
-                                &all_meshes,
+                                &list,
                                &scene,
                                &vertex_sets,
                                &mut object_infos,
                                &pick_result,
                            );
                            pick_in_flight = true;
                        }
```

Then we need the adjust the pick-finishing code like
we mentioned above.

```rust
        // finish a resolved pick: the rest of the JS version's
        // `pickMeshes` after `let id = await pick(...)`
-        if let Some(id) = pick_result.lock().unwrap().take() {
+        loop {
+            let resolved = pick_result.lock().unwrap().take();
+            let Some(id) = resolved else {
+                break;
+            };
            pick_in_flight = false;
+            if id == 0 && !pick_second_try {
+                // if we didn't find one, try all of them again
+                pick_second_try = true;
+                pickable_meshes = Some((0..meshes.len()).collect());
+                let list: Vec<&Mesh> = pickable_meshes
+                    .as_ref()
+                    .unwrap()
+                    .iter()
+                    .map(|&m| &meshes[m])
+                    .collect();
+                pick(
+                    frame.device,
+                    frame.queue,
+                    pick_clip.0,
+                    pick_clip.1,
+                    pick_view_projection,
+                    (frame.width, frame.height),
+                    &pick_pipeline,
+                    &mut pick_texture,
+                    &mut depth_texture,
+                    &pick_buffer,
+                    &list,
+                    &scene,
+                    &vertex_sets,
+                    &mut object_infos,
+                    &pick_result,
+                );
+                pick_in_flight = true;
+                // on native, poll so the retry resolves this frame too
+                #[cfg(not(target_arch = "wasm32"))]
+                frame
+                    .device
+                    .poll(wgpu::PollType::wait_indefinitely())
+                    .expect("device poll failed");
+                continue;
+            }
-            if id > 0 {
-                let mut node = meshes[id as usize - 1].node;
+            if id == 0 {
+                // If we still didn't find one there was nothing under the
+                // pointer
+                // setCurrentSceneGraphNode(undefined)
+                selected_node = None;
+            } else {
+                // remove the picked mesh and get its node
+                let mesh_ndx = pickable_meshes.as_mut().unwrap().remove(id as usize - 1);
+                let mut node = meshes[mesh_ndx].node;
                if !wgpu_fun::setting_bool("showMeshNodes", false) {
                    while scene.nodes[node].name.contains("mesh") {
                        node = scene.nodes[node].parent.unwrap();
                    }
                }
                // setCurrentSceneGraphNode(node)
                selected_node = Some(node);
-            } else {
-                // setCurrentSceneGraphNode(undefined)
-                selected_node = None;
            }
+            break;
        }
```

<sup>Those changes might be hard to see. Consider clicking "hide deleted".</sup>

With that, we're back to being able to click cycle through
the objects under the pointer.

{{{example url="../webgpu-picking-gpu-step-02.html"}}}

Some advantages to GPU picking:

* All GPU vertex effects are applied

  A good example is skinning. [Skinning](webgpu-skinning.html) is often only
  applied on the GPU. To do CPU picking on a skinned object you need to
  reproduce all of the skinning logic on the CPU. Similarly for
  [blend targets](webgpu-blend-targets.html) you would need to make a CPU
  version of that as well. Even in our current code, in the CPU picking we had 
  to walk the vertices knowing what their formats and stride were. We hard
  coded our solution to our one vertex format. It's not uncommon for an app to only
  have one vertex format. But, if it had more than one, we'd need to update the CPU
  code to support each format.

* Transparency can be taken into account if appropriate

  Imagine you have a plane and to that plane is applied a leaf texture
  where areas outside of the leaf are 100% transparent so you can see
  things behind. With CPU picking, as we implemented it, all the picking
  code sees is the 2 triangles making the leaf plane.

  With GPU picking we could easy check the alpha value for the texture
  and `discard` writing the object id if it's below some threshold. 
  This would let us pick things we can see through transparent parts of
  the leaf plane which would feel more natural.

An issue compared to the CPU one we wrote above is that it only gives
us the front most object. To implement clicking to rotate through all objects,
if the pointer hasn't moved, then don't draw the last selected object when
doing the picking. This will make the next closest object be the result.

## Optimizations

There are 3 relatively simple optimizations we could make
though at the moment these will be left as exercises for
the reader 😛

1. Set the scissor to the texel under the pointer

   We can call `pass.set_scissor_rect(clip_x, clip_y, 1, 1)`
   and this would make the GPU render only to that 1 pixel.
   That would be faster than rendering a millions of id
   pixels since in the end we're only reading a single pixel
   anyway.

2. Use frustum culling or other "potential visible set" culling

   If you can easily determine if an object is definitely not in front of the
   camera then you can skip asking the GPU to look at all of that object's triangles.

   This isn't special to picking,
   drawing benefits from frustum culling as well.
   Checking if an object is inside the view frustum,
   helps the next item so it was worth mentioning.

3. Use a 1x1 pixel texture and a different projection matrix.

   It's possible to make a projection matrix that represents just the frustum
   that includes the pixel under the cursor. If we did that we could just use a
   1x1 pixel texture for picking. This has 2 benefits. First, we only need a 1x1
   pixel texture which is a lot less memory than a canvas sized texture. Second,
   the same frustum culling check mentioned above will have much smaller frustum
   and so reject even more objects.


<!-- keep this at the bottom of the article -->
<link href="webgpu-picking.css" rel="stylesheet">
<script type="module" src="webgpu-picking.js"></script>
