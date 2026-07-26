Title: WebGPU Scene Graphs
Description: Scene Graphs
TOC: Scene Graphs

This article is the 9th in a series of articles that will hopefully teach
you about 3D math. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

1. [Translation](webgpu-translation.html)
2. [Rotation](webgpu-rotation.html)
3. [Scaling](webgpu-scale.html)
4. [Matrix Math](webgpu-matrix-math.html)
5. [Orthographic Projection](webgpu-orthographic-projection.html)
6. [Perspective Projection](webgpu-perspective-projection.html)
7. [Cameras](webgpu-cameras.html)
8. [Matrix Stacks](webgpu-matrix-stacks.html)
9. [Scene Graphs](webgpu-scene-graphs.html) ⬅ you are here

In the last article we covered a matrix stack. It allowed us
to build up a stack of matrix changes which was helpful for positioning,
orienting, and scaling things relative to others.

A Scene Graph is in a sense, the same thing, except instead of using
code, we use data. We build up a graph of parents and children where
the children compute their matrix based on the matrix of their parent.

The scene graph for the filing cabinets would look something like this

```
root
  +-cabinet0
  |  +-cabinet0-mesh
  |  +-drawer0
  |  |  +-drawer0-drawer-mesh
  |  |  +-drawer0-handle-mesh
  |  +-drawer1
  |  |  +-drawer1-drawer-mesh
  |  |  +-drawer1-handle-mesh
  |  +-drawer2
  |  |  +-drawer2-drawer-mesh
  |  |  +-drawer2-handle-mesh
  |  +-drawer3
  |     +-drawer3-drawer-mesh
  |     +-drawer3-handle-mesh
  +-cabinet1
  |  ...
  +-cabinet2
  |  ...
  +-cabinet3
  |  ...
  +-cabinet4
     +-cabinet4-mesh
     +-drawer0
     |  +-drawer0-drawer-mesh
     |  +-drawer0-handle-mesh
     +-drawer1
     |  +-drawer1-drawer-mesh
     |  +-drawer1-handle-mesh
     +-drawer2
     |  +-drawer2-drawer-mesh
     |  +-drawer2-handle-mesh
     +-drawer3
        +-drawer3-drawer-mesh
        +-drawer3-handle-mesh
```

The advantage to a scene graph is it stores data as nodes in a graph
so you can easily manipulate some sub portion of the graph without
having to recurse in code.

## Let's switch the file cabinet example from the previous article to use a scene graph.

The first thing we need is a type to represent our scene graph.

In JavaScript this was a `SceneGraphNode` class where each node held direct
references to its parent and to its children. That kind of doubly-linked
object graph is painful to express with Rust's ownership rules: two nodes
can't both own each other. There are two common ways out. One is
`Rc<RefCell<SceneGraphNode>>` — shared, dynamically checked references that
read closest to the JavaScript, at the cost of `Weak` parent pointers and
`.borrow_mut()` noise on every access. The other is an *arena*: a
`SceneGraph` struct owns a `Vec` of all the nodes and nodes refer to each
other **by index**. We'll use the arena. It keeps every operation a plain
`&mut self` method, and — as we'll see when we add a GUI — a plain index is
exactly the kind of node handle we can copy around, keep in a list of
meshes, or receive from the page's GUI. The tradeoff is that the node
methods move onto `SceneGraph` and take a node index: JavaScript's
`child.setParent(parent)` becomes `scene.set_parent(child, Some(parent))`.
One more difference: nodes removed from the graph stay in the `Vec` — there
is no garbage collector to reclaim them — which is fine for examples like
ours.

```rust
// A node in the graph is identified by its index in the SceneGraph's Vec of
// nodes. In JavaScript nodes held direct references to their parent and
// children; in Rust we use indices into an arena instead.
type NodeNdx = usize;

struct SceneGraphNode {
    #[allow(dead_code)] // shown in the page's GUI; used by find() later
    name: String,
    children: Vec<NodeNdx>,
    parent: Option<NodeNdx>,
    local_matrix: [f32; 16],
    world_matrix: [f32; 16],
    source: Option<TRS>,
}

struct SceneGraph {
    nodes: Vec<SceneGraphNode>,
}

#[allow(dead_code)]
impl SceneGraph {
    fn new() -> Self {
        SceneGraph { nodes: Vec::new() }
    }

    // the JS version's `new SceneGraphNode(name, source)`
    fn add_node(&mut self, name: &str, source: Option<TRS>) -> NodeNdx {
        self.nodes.push(SceneGraphNode {
            name: name.to_string(),
            children: Vec::new(),
            parent: None,
            local_matrix: m4::identity(),
            world_matrix: m4::identity(),
            source,
        });
        self.nodes.len() - 1
    }

    fn add_child(&mut self, parent: NodeNdx, child: NodeNdx) {
        self.set_parent(child, Some(parent));
    }

    fn remove_child(&mut self, _parent: NodeNdx, child: NodeNdx) {
        self.set_parent(child, None);
    }

    fn set_parent(&mut self, node: NodeNdx, parent: Option<NodeNdx>) {
        // remove us from our parent
        if let Some(old_parent) = self.nodes[node].parent {
            let children = &mut self.nodes[old_parent].children;
            if let Some(ndx) = children.iter().position(|&c| c == node) {
                children.remove(ndx);
            }
        }

        // Add us to our new parent
        if let Some(parent) = parent {
            self.nodes[parent].children.push(node);
        }
        self.nodes[node].parent = parent;
    }

    fn update_world_matrix(&mut self, node: NodeNdx) {
        // update the local matrix from its source if it has one.
        if let Some(source) = &self.nodes[node].source {
            self.nodes[node].local_matrix = source.get_matrix();
        }

        if let Some(parent) = self.nodes[node].parent {
            // we have a parent so do the math
            self.nodes[node].world_matrix =
                m4::multiply(&self.nodes[parent].world_matrix, &self.nodes[node].local_matrix);
        } else {
            // we have no parent so just copy local to world
            self.nodes[node].world_matrix = self.nodes[node].local_matrix;
        }

        // now process all the children
        for i in 0..self.nodes[node].children.len() {
            let child = self.nodes[node].children[i];
            self.update_world_matrix(child);
        }
    }
}
```

The `SceneGraph` above is pretty straight forward. Each node has a `Vec` of
`children` (as node indices). There are functions to add and remove children
as well as set a node's parent. Each node has a `local_matrix` which
represents the position, orientation, and scale of this node relative to its
parent. Each node has a `world_matrix` that represents this node's position,
orientation, and scale relative to "the world" or more specifically,
relative to the outside of the scene graph. And finally there's
`update_world_matrix` which updates the `world_matrix` of a node and all of
its children. Each node also has an optional `source` which is something that
provides a `get_matrix` function. We can use this to provide different ways
to compute a local matrix for a particular node.

Note the loop at the bottom of `update_world_matrix` indexes the children by
position instead of iterating with a `for child in &self.nodes[node].children`
loop. That's because the recursive call needs `&mut self`, which the borrow
checker won't allow while we hold a reference into `self.nodes`.

Let's provide a source.

```rust
#[derive(Clone, Copy)]
struct TRS {
    translation: [f32; 3],
    rotation: [f32; 3],
    scale: [f32; 3],
}

impl Default for TRS {
    fn default() -> Self {
        TRS {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl TRS {
    fn get_matrix(&self) -> [f32; 16] {
        let mut dst = m4::translation(self.translation);
        dst = m4::rotate_x(&dst, self.rotation[0]);
        dst = m4::rotate_y(&dst, self.rotation[1]);
        dst = m4::rotate_z(&dst, self.rotation[2]);
        m4::scale(&dst, self.scale)
    }
}
```

`TRS` is short for Translation, Rotation, Scale. This is a common way to
compute a local matrix in a scene graph. Often, some implementations use
"position" instead of "translation". For this tutorial, I thought it might
be better to use "translation" since it matches what we do in `get_matrix`.

The JavaScript version stored these as `Float32Array`s so it could use their
`set` function to copy new values in. In Rust `[f32; 3]` is a plain `Copy`
array so assignment already copies. We implement `Default` (with a scale of
1) so a `TRS` can be created by specifying only the fields we care about,
like the JavaScript version's default parameters:
`TRS { translation: [1.0, 2.0, 3.0], ..Default::default() }`.

You can see `get_matrix` computes a matrix by using effectively

```
translation * rotationX * rotationY * rotationZ * scale
```

It's common to have options to change the order of applying rotation.
Instead of XYZ it might by ZYX or YZX or whatever. It's also common
to use a [quaternion](https://google.com/search?quaternion) and it's getting
more common to use [geometric algebra](https://www.youtube.com/watch?v=Idlv83CxP-8).

In any case, we're going to start with what's above.

Now that we have a `SceneGraph` and `TRS` source, let's build our
scene graph.

First let's make a function that adds both a scene graph node and a `TRS`
source to some parent.

```rust
fn add_trs_scene_graph_node(
    scene: &mut SceneGraph,
    name: &str,
    parent: Option<NodeNdx>,
    trs: TRS,
) -> NodeNdx {
    let node = scene.add_node(name, Some(trs));
    if let Some(parent) = parent {
        scene.set_parent(node, Some(parent));
    }
    node
}
```

Let's add a function that makes a "mesh". I'm not sure what to call this
but it will be a list of things to draw. Each "thing to draw" will be a
combination of a scene graph node, the vertices for the thing we want to
draw, and a color to draw it with. In JavaScript a mesh held a direct
reference to its vertices; we use the same trick as the nodes and refer to
an entry in a `Vec` of `Vertices` by index.

```rust
// Like the nodes, each mesh refers to its vertices by index (into a Vec of
// Vertices) instead of holding a direct reference.
struct Mesh {
    node: NodeNdx,
    vertices: usize,
    color: [f32; 4],
}

fn add_mesh(meshes: &mut Vec<Mesh>, node: NodeNdx, vertices: usize, color: [f32; 4]) {
    meshes.push(Mesh {
        node,
        vertices,
        color,
    });
}
```

Now, since we only have a cube, let's make a function that adds a cube
to the scene graph and adds a "mesh" to render the cube.

```rust
fn add_cube_node(
    scene: &mut SceneGraph,
    meshes: &mut Vec<Mesh>,
    name: &str,
    parent: NodeNdx,
    trs: TRS,
    color: [f32; 4],
) {
    let node = add_trs_scene_graph_node(scene, name, Some(parent), trs);
    add_mesh(meshes, node, K_CUBE_VERTICES, color);
}
```

`K_CUBE_VERTICES` is the index of the cube's vertices in our `Vec` of
`Vertices`. So far there's only one entry.

```rust
+const K_CUBE_VERTICES: usize = 0;

-    let cube_vertices = create_vertices(&app.device, &app.queue, create_cube_vertices(), "cube");
+    let vertex_sets = vec![create_vertices(
+        &app.device,
+        &app.queue,
+        create_cube_vertices(),
+        "cube",
+    )];
```

With those in place, lets build the graph for the filing cabinets. First let's
make a "root" node. The root doesn't need a "source".

```rust
    let mut scene = SceneGraph::new();
    let mut meshes: Vec<Mesh> = Vec::new();

    let root = scene.add_node("root", None);
```

Then let's add cabinets

```rust
    let root = scene.add_node("root", None);
+    // Add cabinets
+    for cabinet_ndx in 0..K_NUM_CABINETS {
+        add_cabinet(&mut scene, &mut meshes, root, cabinet_ndx);
+    }
```

Let's write `add_cabinet`.

```rust
fn add_cabinet(scene: &mut SceneGraph, meshes: &mut Vec<Mesh>, parent: NodeNdx, cabinet_ndx: usize) {
    let cabinet_name = format!("cabinet{cabinet_ndx}");

    // add a node for the entire cabinet
    let cabinet = add_trs_scene_graph_node(
        scene,
        &cabinet_name,
        Some(parent),
        TRS {
            translation: [cabinet_ndx as f32 * K_CABINET_SPACING, 0.0, 0.0],
            ..Default::default()
        },
    );

    // add a node with a cube for the cabinet
    let k_cabinet_size = [
        K_DRAWER_SIZE[K_WIDTH] + 6.0,
        K_DRAWER_SPACING * K_NUM_DRAWERS_PER_CABINET as f32 + 6.0,
        K_DRAWER_SIZE[K_DEPTH] + 4.0,
    ];
    add_cube_node(
        scene,
        meshes,
        &format!("{cabinet_name}-mesh"),
        cabinet,
        TRS {
            scale: k_cabinet_size,
            ..Default::default()
        },
        K_CABINET_COLOR,
    );

    // Add the drawers
    for drawer_ndx in 0..K_NUM_DRAWERS_PER_CABINET {
        add_drawer(scene, meshes, cabinet, drawer_ndx);
    }
}
```

And, let's write `add_drawer`.

```rust
fn add_drawer(scene: &mut SceneGraph, meshes: &mut Vec<Mesh>, parent: NodeNdx, drawer_ndx: usize) {
    let drawer_name = format!("drawer{drawer_ndx}");

    // add a node for the entire drawer
    let drawer = add_trs_scene_graph_node(
        scene,
        &drawer_name,
        Some(parent),
        TRS {
            translation: [3.0, drawer_ndx as f32 * K_DRAWER_SPACING + 5.0, 1.0],
            ..Default::default()
        },
    );

    // add a node with a cube for the drawer cube.
    add_cube_node(
        scene,
        meshes,
        &format!("{drawer_name}-drawer-mesh"),
        drawer,
        TRS {
            scale: K_DRAWER_SIZE,
            ..Default::default()
        },
        K_DRAWER_COLOR,
    );

    // add a node with a cube for the handle
    add_cube_node(
        scene,
        meshes,
        &format!("{drawer_name}-handle-mesh"),
        drawer,
        TRS {
            translation: K_HANDLE_POSITION,
            scale: K_HANDLE_SIZE,
            ..Default::default()
        },
        K_HANDLE_COLOR,
    );
}
```

With our scene graph in place, we need to update our render function. The
`MatrixStack` is gone, so it comes out of the `Ctx`, and `draw_object` gets
the vertices to draw as a parameter. A small `draw_mesh` function draws one
mesh by looking up its node's world matrix and its vertices.

```rust
struct Ctx<'a, 'b> {
    pass: &'a mut wgpu::RenderPass<'b>,
-    stack: &'a mut MatrixStack,
    view_projection_matrix: [f32; 16],
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    pipeline: &'a wgpu::RenderPipeline,
    object_infos: &'a mut Vec<ObjectInfo>,
    object_ndx: usize,
-    num_vertices: u32,
}

-fn draw_object(ctx: &mut Ctx, matrix: [f32; 16], color: [f32; 4]) {
+fn draw_object(ctx: &mut Ctx, vertices: &Vertices, matrix: [f32; 16], color: [f32; 4]) {
+    let Vertices {
+        vertex_buffer,
+        num_vertices,
+    } = vertices;

    ...

+    ctx.pass.set_vertex_buffer(0, vertex_buffer.slice(..));
    ctx.pass.set_bind_group(0, &object_info.bind_group, &[]);
-    ctx.pass.draw(0..ctx.num_vertices, 0..1);
+    ctx.pass.draw(0..*num_vertices, 0..1);
}

+fn draw_mesh(ctx: &mut Ctx, mesh: &Mesh, scene: &SceneGraph, vertex_sets: &[Vertices]) {
+    let Mesh {
+        node,
+        vertices,
+        color,
+    } = mesh;
+    draw_object(
+        ctx,
+        &vertex_sets[*vertices],
+        scene.nodes[*node].world_matrix,
+        *color,
+    );
+}
```

and in the render code, instead of walking the cabinets with the matrix
stack, we update the world matrices and draw all the meshes.

```rust
-            ctx.stack.save();
-            ctx.stack.rotate_y(base_rotation);
-            ctx.stack.translate([
-                (K_NUM_CABINETS as f32 - 0.5) * K_CABINET_SPACING * -0.5,
-                0.0,
-                0.0,
-            ]);
-            draw_cabinets(&mut ctx, K_NUM_CABINETS);
-            ctx.stack.restore();
+            scene.update_world_matrix(root);
+            for mesh in &meshes {
+                draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
+            }
```

And let's tweak the camera code. On the example page's JavaScript side

```js
  const settings = {
-    baseRotation: 0,
+    cameraRotation: 0,
  };

  const radToDegOptions = { min: -180, max: 180, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
-  gui.add(settings, 'baseRotation', radToDegOptions)
-     .onChange(v => wasm.set_setting_num('baseRotation', v));
+  gui.add(settings, 'cameraRotation', radToDegOptions)
+     .onChange(v => wasm.set_setting_num('cameraRotation', v));
```

and in the Rust render code

```rust
-            let base_rotation = wgpu_fun::setting_f64("baseRotation", 0.0) as f32;
+            let camera_rotation = wgpu_fun::setting_f64("cameraRotation", 0.0) as f32;

  ...

-            let eye = [0.0, 80.0, 200.0];
-            let target = [0.0, 80.0, 0.0];
-            let up = [0.0, 1.0, 0.0];
-
-            // Compute a view matrix
-            let view_matrix = m4::look_at(eye, target, up);
+            // Compute a camera matrix
+            let mut camera_matrix = m4::identity();
+            camera_matrix = m4::translate(&camera_matrix, [120.0, 100.0, 0.0]);
+            camera_matrix = m4::rotate_y(&camera_matrix, camera_rotation);
+            camera_matrix = m4::translate(&camera_matrix, [0.0, 0.0, 300.0]);
+
+            // Compute a view matrix
+            let view_matrix = m4::inverse(&camera_matrix);

            // combine the view and projection matrixes
            let view_projection_matrix = m4::multiply(&projection, &view_matrix);
```

And that gives us the same filing cabinets but using a scene graph.

{{{example url="../webgpu-scene-graphs-file-cabinets.html"}}}

## <a id="a-gui"></a> Add a GUI

A major point of a scene graph is that, because it's just data, we can
manipulate it. Let's add a UI to adjust and tweak the graph.

**A note on how the port splits this up:** in the JavaScript original the
GUI and the scene graph live in the same script, so the GUI can hold direct
references to `TRS` objects. In our port the scene graph lives in the Rust
wasm module while the muigui panel is page JavaScript. So the page keeps a
small *mirror* of the node tree — names and TRS values, created in exactly
the same order as the Rust code creates its nodes, so the page's array
indices match the Rust arena's `NodeNdx` values. The mirror is only for
display; when you pick a node the page sends its index with
`set_setting_num('nodeNdx', ndx)`, and when you drag a slider the page sends
`set_setting_str('trsEdit', 'id axis value')` which the Rust side applies to
the selected node's `TRS` (the `id` just makes each edit apply exactly
once).

Here's the page-side mirror. It's the same `addCabinet`/`addDrawer` logic
you saw above, minus the meshes.

```js
// The scene graph itself lives in the Rust module. The page keeps a mirror
// of the node tree (names and TRS values, in the same order the Rust code
// creates its nodes) so the GUI can display and edit it; edits are sent to
// the wasm module as settings.
const nodes = [];
function addNode(name, parent, trs) {
  const node = {
    name,
    children: [],
    hasTRS: trs !== undefined,
    trs: {
      translation: [...(trs?.translation ?? [0, 0, 0])],
      rotation: [...(trs?.rotation ?? [0, 0, 0])],
      scale: [...(trs?.scale ?? [1, 1, 1])],
    },
    ndx: nodes.length,
  };
  nodes.push(node);
  if (parent) {
    parent.children.push(node);
  }
  return node;
}

function addDrawer(parent, drawerNdx) {
  const drawerName = `drawer${drawerNdx}`;
  const drawer = addNode(drawerName, parent, {
    translation: [3, drawerNdx * kDrawerSpacing + 5, 1],
  });
  addNode(`${drawerName}-drawer-mesh`, drawer, { scale: kDrawerSize });
  addNode(`${drawerName}-handle-mesh`, drawer, {
    translation: kHandlePosition,
    scale: kHandleSize,
  });
}

function addCabinet(parent, cabinetNdx) {
  const cabinetName = `cabinet${cabinetNdx}`;
  const cabinet = addNode(cabinetName, parent, {
    translation: [cabinetNdx * kCabinetSpacing, 0, 0],
  });
  const kCabinetSize = [
    kDrawerSize[kWidth] + 6,
    kDrawerSpacing * kNumDrawersPerCabinet + 6,
    kDrawerSize[kDepth] + 4,
  ];
  addNode(`${cabinetName}-mesh`, cabinet, { scale: kCabinetSize });
  for (let drawerNdx = 0; drawerNdx < kNumDrawersPerCabinet; ++drawerNdx) {
    addDrawer(cabinet, drawerNdx);
  }
}

const root = addNode('root');
// Add cabinets
for (let cabinetNdx = 0; cabinetNdx < kNumCabinets; ++cabinetNdx) {
  addCabinet(root, cabinetNdx);
}
```

Now, lets add some controls for translation, rotation, and scale.
We'll make a helper the UI will look at to adjust a `TRS` but will
allow us to change which `TRS` is being edited. Each edit updates the
page's mirror and is forwarded to the wasm module.

```js
let currentNode = root;
let editId = 0;
// TRS edits are sent to the wasm module as "id axis value"; the id makes
// sure each edit is applied exactly once. Axis 0-2 is translation,
// 3-5 rotation, 6-8 scale.
function sendTRSEdit(axis, v) {
  wasm.set_setting_str('trsEdit', `${++editId} ${axis} ${v}`);
}

// Presents the current node's TRS to the UI, forwarding edits to the
// wasm module.
const trsUIHelper = {
  get translationX() { return currentNode.trs.translation[0]; },
  set translationX(v) { currentNode.trs.translation[0] = v; sendTRSEdit(0, v); },
  get translationY() { return currentNode.trs.translation[1]; },
  set translationY(v) { currentNode.trs.translation[1] = v; sendTRSEdit(1, v); },
  get translationZ() { return currentNode.trs.translation[2]; },
  set translationZ(v) { currentNode.trs.translation[2] = v; sendTRSEdit(2, v); },

  get rotationX() { return currentNode.trs.rotation[0]; },
  set rotationX(v) { currentNode.trs.rotation[0] = v; sendTRSEdit(3, v); },
  get rotationY() { return currentNode.trs.rotation[1]; },
  set rotationY(v) { currentNode.trs.rotation[1] = v; sendTRSEdit(4, v); },
  get rotationZ() { return currentNode.trs.rotation[2]; },
  set rotationZ(v) { currentNode.trs.rotation[2] = v; sendTRSEdit(5, v); },

  get scaleX() { return currentNode.trs.scale[0]; },
  set scaleX(v) { currentNode.trs.scale[0] = v; sendTRSEdit(6, v); },
  get scaleY() { return currentNode.trs.scale[1]; },
  set scaleY(v) { currentNode.trs.scale[1] = v; sendTRSEdit(7, v); },
  get scaleZ() { return currentNode.trs.scale[2]; },
  set scaleZ(v) { currentNode.trs.scale[2] = v; sendTRSEdit(8, v); },
};
```

```js
  const settings = {
-    cameraRotation: 0,
+    cameraRotation: degToRad(-45),
  };

-  const radToDegOptions = { min: -180, max: 180, step: 1, converters: GUI.converters.radToDeg };
+  const radToDegOptions = { min: -90, max: 90, step: 1, converters: GUI.converters.radToDeg };
+  const cameraRadToDegOptions = { min: -180, max: 180, step: 1, converters: GUI.converters.radToDeg };

  const gui = new GUI();
-  gui.add(settings, 'cameraRotation', radToDegOptions)
+  gui.add(settings, 'cameraRotation', cameraRadToDegOptions)
     .onChange(v => wasm.set_setting_num('cameraRotation', v));
+  const trsFolder = gui.addFolder('orientation');
+  trsFolder.add(trsUIHelper, 'translationX', -200, 200, 1),
+  trsFolder.add(trsUIHelper, 'translationY', -200, 200, 1),
+  trsFolder.add(trsUIHelper, 'translationZ', -200, 200, 1),
+  trsFolder.add(trsUIHelper, 'rotationX', radToDegOptions),
+  trsFolder.add(trsUIHelper, 'rotationY', radToDegOptions),
+  trsFolder.add(trsUIHelper, 'rotationZ', radToDegOptions),
+  trsFolder.add(trsUIHelper, 'scaleX', 0.1, 100),
+  trsFolder.add(trsUIHelper, 'scaleY', 0.1, 100),
+  trsFolder.add(trsUIHelper, 'scaleZ', 0.1, 100),
```

Now we need a way to select a node so let's walk the (mirrored) scene graph
and make a button for each node.

```js
import GUI from '../3rdparty/muigui-0.x.module.js';
+import { addButtonLeftJustified } from './resources/js/gui-helpers.js';

...
+  const kUnelected = '\u3000'; // full-width space
+  const kSelected = '➡️';
+  const prefixRE = new RegExp(`^(?:${kUnelected}|${kSelected})`);
+
+  function setCurrentSceneGraphNode(node) {
+    currentNode = node;
+    wasm.set_setting_num('nodeNdx', node.ndx);
+    trsFolder.name(`orientation: ${node.name}`);
+    trsFolder.updateDisplay();
+
+    // Mark which node is selected.
+    for (const b of nodeButtons) {
+      const name = b.button.getName().replace(prefixRE, '');
+      b.button.name(`${b.node === node ? kSelected : kUnelected}${name}`);
+    }
+  }
+
+  //   is non-breaking space.
+  const threeSpaces = '\u00a0\u00a0\u00a0';
+  const barTwoSpaces = '\u00a0|\u00a0';
+  const plusDash = '\u00a0+-';
+  // add a scene graph node to the GUI and adds the appropriate
+  // prefix so it looks something like
+  //
+  // +-root
+  // | +-child
+  // | | +-child
+  // | +-child
+  // +-child
+  function addSceneGraphNodeToGUI(gui, node, last, prefix) {
+    const nodes = [];
+    if (node.hasTRS) {
+      const label = `${prefix === undefined ? '' : `${prefix}${plusDash}`}${node.name}`;
+      nodes.push({
+        button: addButtonLeftJustified(
+          gui, label, () => setCurrentSceneGraphNode(node)),
+        node,
+      });
+    }
+    const childPrefix = prefix === undefined
+      ? ''
+      : `${prefix}${last ? threeSpaces : barTwoSpaces}`;
+    nodes.push(...node.children.map((child, i) => {
+      const childLast = i === node.children.length - 1;
+      return addSceneGraphNodeToGUI(gui, child, childLast, childPrefix);
+    }));
+    return nodes.flat();
+  }

  const gui = new GUI();
  ...
+  const nodesFolder = gui.addFolder('nodes');
+  const nodeButtons = addSceneGraphNodeToGUI(nodesFolder, root);
+
+  setCurrentSceneGraphNode(root.children[0]);
```

Above we made a button for each node that has a `TRS`.
When a button is clicked it calls
`setCurrentSceneGraphNode` and passes it the node for that button.
`setCurrentSceneGraphNode` updates the folder name, sends the node's index
to the wasm module, and then calls `trsFolder.updateDisplay` to update the
UI with the data from the newly selected `TRS`.

On the Rust side we read the selected node and apply any pending edit
before rendering:

```rust
+    // id of the last TRS edit we applied from the page's GUI
+    let mut last_trs_edit_id = 0.0f64;

    app.run(RenderMode::Once, move |frame: &Frame| {
+        // The page's GUI selects a node (`nodeNdx`) and sends TRS edits as a
+        // "id axis value" string; axis 0-2 is translation, 3-5 rotation,
+        // 6-8 scale. Apply each edit once, to the selected node's TRS.
+        let node_ndx = wgpu_fun::setting_f64("nodeNdx", 1.0) as usize;
+        let trs_edit = wgpu_fun::setting_str("trsEdit", "");
+        let parts: Vec<f64> = trs_edit
+            .split_whitespace()
+            .filter_map(|v| v.parse().ok())
+            .collect();
+        if let [id, axis, value] = parts[..] {
+            if id != last_trs_edit_id {
+                last_trs_edit_id = id;
+                if let Some(trs) = scene
+                    .nodes
+                    .get_mut(node_ndx)
+                    .and_then(|node| node.source.as_mut())
+                {
+                    let (axis, value) = (axis as usize, value as f32);
+                    match axis {
+                        0..=2 => trs.translation[axis] = value,
+                        3..=5 => trs.rotation[axis - 3] = value,
+                        _ => trs.scale[axis - 6] = value,
+                    }
+                }
+            }
+        }
```

Because the scene graph is just data, "select node 1 and set its rotation"
is nothing more than indexing into the arena and poking a value into its
`TRS`. Every settings change automatically triggers a re-render for
`RenderMode::Once` examples, so there's nothing else to do.

This works but I found the UI is a little cluttered for our small windows so
here's a few more tweaks, both purely on the page's JavaScript side.

1. Reduce the translate, rotation, scale controls.

   For the file cabinets, although we can set any of the 9 settings of
   translation, rotation, and scale on each node. The only one that's really
   relevant is "translation z". So, lets hide all but translation by default.

   ```js
    const settings = {
      cameraRotation: degToRad(-45),
   +   showAllTRS: false,
    };

    const gui = new GUI();
    gui.add(settings, 'cameraRotation', cameraRadToDegOptions)
       .onChange(v => wasm.set_setting_num('cameraRotation', v));
   + gui.add(settings, 'showAllTRS').onChange(showTRS);
    const trsFolder = gui.addFolder('orientation');
   + const trsControls = [
   *   trsFolder.add(trsUIHelper, 'translationX', -200, 200, 1),
   *   trsFolder.add(trsUIHelper, 'translationY', -200, 200, 1),
   *   trsFolder.add(trsUIHelper, 'translationZ', -200, 200, 1),
   *   trsFolder.add(trsUIHelper, 'rotationX', radToDegOptions),
   *   trsFolder.add(trsUIHelper, 'rotationY', radToDegOptions),
   *   trsFolder.add(trsUIHelper, 'rotationZ', radToDegOptions),
   *   trsFolder.add(trsUIHelper, 'scaleX', 0.1, 100),
   *   trsFolder.add(trsUIHelper, 'scaleY', 0.1, 100),
   *   trsFolder.add(trsUIHelper, 'scaleZ', 0.1, 100),
   + ];
   const nodesFolder = gui.addFolder('nodes');
   addSceneGraphNodeToGUI(nodesFolder, root);

   +const alwaysShow = new Set([0, 1, 2]);
   +function showTRS(show) {
   +  trsControls.forEach((trs, i) => {
   +    trs.show(show || alwaysShow.has(i));
   +  });
   +}
   +showTRS(false);
   ```

   This code collects the translation, rotation, scale controls into an array
   and shows all or just the first 3.

2. Don't show the meshes

   We have a '-mesh' node in the graph for each cube which we don't really need
   to move the cabinets or the drawers so lets hide them by default.

   ```js
     const settings = {
       cameraRotation: degToRad(-45),
   +    showMeshNodes: false,
       showAllTRS: false,
     };

     const gui = new GUI();
     gui.add(settings, 'cameraRotation', cameraRadToDegOptions)
        .onChange(v => wasm.set_setting_num('cameraRotation', v));
   +  gui.add(settings, 'showMeshNodes').onChange(showMeshNodes);
     gui.add(settings, 'showAllTRS').onChange(showTRS);

      ...

     const nodesFolder = gui.addFolder('nodes');
     const nodeButtons = addSceneGraphNodeToGUI(nodesFolder, root);

   + function showMeshNodes(show) {
   +   for (const {node, button} of nodeButtons) {
   +     if (node.name.includes('mesh')) {
   +       button.show(show);
   +     }
   +   }
   + }
   + showMeshNodes(false);
   ```

Try selecting a "drawer" and adjusting "translation z".

{{{example url="../webgpu-scene-graphs-file-cabinets-w-gui.html"}}}

As you can see, by having data for each node it makes it easy to change the
position, rotation, and scale of any individual node.

## <a id="a-animate"></a> Animate

For fun, let's animate the drawers.

First lets make a list of the drawer nodes.

```rust
fn add_drawer(
    scene: &mut SceneGraph,
    meshes: &mut Vec<Mesh>,
+    anim_nodes: &mut Vec<NodeNdx>,
    parent: NodeNdx,
    drawer_ndx: usize,
) {
    let drawer_name = format!("drawer{drawer_ndx}");

    // add a node for the entire drawer
    let drawer = add_trs_scene_graph_node(
        scene,
        &drawer_name,
        Some(parent),
        TRS {
            translation: [3.0, drawer_ndx as f32 * K_DRAWER_SPACING + 5.0, 1.0],
            ..Default::default()
        },
    );
+    anim_nodes.push(drawer);

    ...
}
```

Then let's write some code to animate the drawers based on the time.
Because the nodes are just indices, the list of nodes to animate is a
`Vec<NodeNdx>`.

```rust
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn animate(time: f64, scene: &mut SceneGraph, anim_nodes: &[NodeNdx]) {
    for (i, &node) in anim_nodes.iter().enumerate() {
        let source = scene.nodes[node].source.as_mut().unwrap();
        let t = time + i as f64 * 1.0;
        let l = (t.sin() * 0.5 + 0.5) as f32;
        source.translation[2] = lerp(1.0, K_DRAWER_SIZE[2] * 0.8, l);
    }
}
```

The JavaScript version made a demand-driven render loop with a
`requestRender` function that requested an animation frame only if one
hadn't already been requested, and every place that used to call `render`
called `requestRender` instead. Our `wgpu_fun` helper offers two render
modes and its `Once` mode can't restart itself from inside the frame
callback, so we take the simpler path and switch the example to
`RenderMode::Continuous`, which renders every frame like the browser's
`requestAnimationFrame` loop.

```rust
-    app.run(RenderMode::Once, move |frame: &Frame| {
+    app.run(RenderMode::Continuous, move |frame: &Frame| {
```

Finally lets setup some code to let us turn the animation on/off. On the
page's JavaScript side we add a checkbox that disables the TRS sliders
while animating

```js
  const settings = {
    cameraRotation: degToRad(-45),
+    animate: false,
    showMeshNodes: false,
    showAllTRS: false,
  };

  const gui = new GUI();
  gui.add(settings, 'cameraRotation', cameraRadToDegOptions)
     .onChange(v => wasm.set_setting_num('cameraRotation', v));
+  gui.add(settings, 'animate').onChange(v => {
+    trsFolder.enable(!v);
+    wasm.set_setting_bool('animate', v);
+  });
  gui.add(settings, 'showMeshNodes').onChange(showMeshNodes);
  gui.add(settings, 'showAllTRS').onChange(showTRS);
```

and in the Rust render code

```rust
+    // clock for the animation; it only advances while animating
+    let mut then = 0.0f64;
+    let mut time = 0.0f64;
+    let mut was_running = false;

    app.run(RenderMode::Continuous, move |frame: &Frame| {

    ...

+        // The animation clock only advances while "animate" is checked.
+        let settings_animate = wgpu_fun::setting_bool("animate", false);
+        let is_running = settings_animate;
+        let now = frame.time;
+        let delta_time = if was_running { now - then } else { 0.0 };
+        then = now;
+
+        if is_running {
+            time += delta_time;
+        }
+        was_running = is_running;
+
+        if settings_animate {
+            animate(time, &mut scene, &anim_nodes);
+        }
```

A complication above is that we'd prefer to only run the clock if "animate" is
checked. So we check if it `was_running` last frame. If not then we set
`delta_time` to 0. That way the clock won't jump forward the amount of time we
were not animating.

We disable the translation, rotation, scale controls if we're animating.
One small difference from the JavaScript version: it called
`trsFolder.updateDisplay()` every animated frame so the sliders followed
the animation. Our sliders live on the page and can't see the values the
Rust side is animating, so they simply stay put while the animation runs.

{{{example url="../webgpu-scene-graphs-file-cabinets-w-animation.html"}}}

Another advantage to a scene graph is it makes it easy to apply animation.
We just apply it to the nodes. We don't have to care in advance how they were
created.

## <a id="a-hand"></a> Making a Hand

Let's make a new example of a hand. To keep it simple we'll stick with
cubes.

Here's a diagram of what the scene graph will look like

```
oot
 +-wrist
    +-palm
    |  +-thumb
    |  |  +-thumb-mesh
    |  |  +-thumb-1
    |  |     +-thumb-1-mesh
    |  +-index finger
    |  |  +-index finger-mesh
    |  |  +-index finger-1
    |  |     +-index finger-1-mesh
    |  |     +-index finger-2
    |  |        +-index finger-2-mesh
    |  +-middle finger
    |  |  +-middle finger-mesh
    |  |  +-middle finger-1
    |  |     +-middle finger-1-mesh
    |  |     +-middle finger-2
    |  |        +-middle finger-2-mesh
    |  +-ring finger
    |  |  +-ring finger-mesh
    |  |  +-ring finger-1
    |  |     +-ring finger-1-mesh
    |  |     +-ring finger-2
    |  |        +-ring finger-2-mesh
    |  +-pinky
    |     +-pinky-mesh
    |     +-pinky-1
    |        +-pinky-1-mesh
    |        +-pinky-2
    |           +-pinky-2-mesh
    +-palm-mesh
```

First, let's move the cube vertices so they are centered above the XZ plane. We
could do this by adding more nodes in the scene graph or by applying it in each
'-mesh' node but it would be less cluttered to just do it in the vertices
themselves.

```rust
fn create_cube_vertices() -> (Vec<f32>, u32) {
    let positions: Vec<f32> = vec![
        // left
-        0.0, 0.0,  0.0,
-        0.0, 0.0, -1.0,
-        0.0, 1.0,  0.0,
-        0.0, 1.0, -1.0,
+        -0.5, 0.0,  0.5,
+        -0.5, 0.0, -0.5,
+        -0.5, 1.0,  0.5,
+        -0.5, 1.0, -0.5,

        // right
-        1.0, 0.0,  0.0,
-        1.0, 0.0, -1.0,
-        1.0, 1.0,  0.0,
-        1.0, 1.0, -1.0,
+         0.5, 0.0,  0.5,
+         0.5, 0.0, -0.5,
+         0.5, 1.0,  0.5,
+         0.5, 1.0, -0.5,
    ];

  ...
```

Now let's make the scene graph. We delete all the code
related to creating the file cabinets scene graph and replace it
with this.

```rust
+const K_WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
+
+fn add_finger(
+    scene: &mut SceneGraph,
+    meshes: &mut Vec<Mesh>,
+    name: &str,
+    parent: NodeNdx,
+    segments: usize,
+    segment_height: f32,
+    trs: TRS,
+) -> Vec<NodeNdx> {
+    let mut nodes = Vec::new();
+    let base_name = name;
+    let mut name = name.to_string();
+    let mut parent = parent;
+    let mut trs = trs;
+    for i in 0..segments {
+        let node = add_trs_scene_graph_node(scene, &name, Some(parent), trs);
+        nodes.push(node);
+        let mesh_node = add_trs_scene_graph_node(
+            scene,
+            &format!("{name}-mesh"),
+            Some(node),
+            TRS {
+                scale: [10.0, segment_height, 10.0],
+                ..Default::default()
+            },
+        );
+        add_mesh(meshes, mesh_node, K_CUBE_VERTICES, K_WHITE);
+        parent = node;
+        name = format!("{base_name}-{}", i + 1);
+        trs = TRS {
+            translation: [0.0, segment_height, 0.0],
+            rotation: [15.0f32.to_radians(), 0.0, 0.0],
+            ..Default::default()
+        };
+    }
+    nodes
+}

    let root = scene.add_node("root", None);
+    let wrist = add_trs_scene_graph_node(&mut scene, "wrist", Some(root), TRS::default());
+    let palm = add_trs_scene_graph_node(
+        &mut scene,
+        "palm",
+        Some(wrist),
+        TRS {
+            translation: [0.0, 100.0, 0.0],
+            ..Default::default()
+        },
+    );
+    let palm_mesh = add_trs_scene_graph_node(
+        &mut scene,
+        "palm-mesh",
+        Some(wrist),
+        TRS {
+            scale: [100.0, 100.0, 10.0],
+            ..Default::default()
+        },
+    );
+    add_mesh(&mut meshes, palm_mesh, K_CUBE_VERTICES, K_WHITE);
+    let rotation = [15.0f32.to_radians(), 0.0, 0.0];
+    let mut anim_nodes: Vec<NodeNdx> = vec![
+        wrist,
+        palm,
+    ];
+    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "thumb",         palm, 2, 20.0, TRS { translation: [-50.0, 0.0, 0.0], rotation, ..Default::default() }));
+    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "index finger",  palm, 3, 30.0, TRS { translation: [-25.0, 0.0, 0.0], rotation, ..Default::default() }));
+    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "middle finger", palm, 3, 35.0, TRS { translation: [ -0.0, 0.0, 0.0], rotation, ..Default::default() }));
+    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "ring finger",   palm, 3, 33.0, TRS { translation: [ 25.0, 0.0, 0.0], rotation, ..Default::default() }));
+    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "pinky",         palm, 3, 25.0, TRS { translation: [ 45.0, 0.0, 0.0], rotation, ..Default::default() }));
```

We create a wrist, to which we attach a palm and a palm-mesh. To the palm we attach 5 fingers
using `add_finger`. Add finger adds the segments of a finger, each a certain length.

> Yes, this is not even remotely correct for a human hand 😂

The example page's mirror of the scene graph gets the same update, and where
as for the file cabinets we only really cared about `translation z`, the most
important transformation for the hand is `rotation x` so let's adjust which
controls are shown by default

```js
-  const alwaysShow = new Set([0, 1, 2]);
+  const alwaysShow = new Set([0, 1, 3]);
  function showTRS(show) {
    trsControls.forEach((trs, i) => {
      trs.show(show || alwaysShow.has(i));
    });
  }
  showTRS(false);
```

The animation for the hand needs to rotate x instead of translate z.

```rust
fn animate(time: f64, scene: &mut SceneGraph, anim_nodes: &[NodeNdx]) {
    for (i, &node) in anim_nodes.iter().enumerate() {
        let source = scene.nodes[node].source.as_mut().unwrap();
-        let t = time + i as f64 * 1.0;
+        let t = time + i as f64 * 0.1;
        let l = (t.sin() * 0.5 + 0.5) as f32;
-        source.translation[2] = lerp(1.0, K_DRAWER_SIZE[2] * 0.8, l);
+        source.rotation[0] = lerp(0.0, std::f32::consts::PI * 0.25, l);
    }
}
```

Finally, ket's adjust the camera slightly.

```rust
    // Compute a camera matrix.
    let mut camera_matrix = m4::identity();
-    camera_matrix = m4::translate(&camera_matrix, [120.0, 100.0, 0.0]);
+    camera_matrix = m4::translate(&camera_matrix, [0.0, 100.0, 0.0]);
    camera_matrix = m4::rotate_y(&camera_matrix, camera_rotation);
-    camera_matrix = m4::translate(&camera_matrix, [60.0, 0.0, 300.0]);
+    camera_matrix = m4::translate(&camera_matrix, [100.0, 0.0, 300.0]);
```

{{{example url="../webgpu-scene-graphs-hand.html"}}}

Select a finger and just 'rotation x' and you'll see the segments
further down all rotate with it.

## <a id="a-shoot"></a> Let's shoot a projectile from the index finger.

Another advantage of a scene graph is that you can easily ask for the
position and orientation of any node in the graph.

So, to shoot a from the index finger we need to know the node for the
tip of the finger.

Many scene graph APIs have functions to find nodes by name. Let's add
one to ours. Like `update_world_matrix` it recurses down from a node.

```rust
impl SceneGraph {
    ...

+    fn find(&self, node: NodeNdx, name: &str) -> Option<NodeNdx> {
+        if self.nodes[node].name == name {
+            return Some(node);
+        }
+        for &child in &self.nodes[node].children {
+            if let Some(found) = self.find(child, name) {
+                return Some(found);
+            }
+        }
+        None
+    }

  ...
}
```

With that added we can find last segment of the index finger by name.
That node represents the base of the last segment of the index finger,
the point at which it rotates, not the tip. So, lets add another node
as a child of that last index finger segment that actually does represent
the tip.

```rust
    anim_nodes.extend(add_finger(&mut scene, &mut meshes, "pinky",         palm, 3, 25.0, TRS { translation: [ 45.0, 0.0, 0.0], rotation, ..Default::default() }));
+    let index_finger_2 = scene.find(root, "index finger-2");
+    let finger_tip = add_trs_scene_graph_node(
+        &mut scene,
+        "finger-tip",
+        index_finger_2,
+        TRS {
+            translation: [0.0, 30.0, 0.0],
+            ..Default::default()
+        },
+    );
```

Now we need a projectile. We'll use the cone we created for ornaments
in [the previous article](webgpu-matrix-stacks.html).

```rust
    let vertex_sets = vec![
        create_vertices(&app.device, &app.queue, create_cube_vertices(), "cube"),
+        create_vertices(
+            &app.device,
+            &app.queue,
+            create_cone_vertices(
+                10.0, // radius
+                20.0, // height
+                6,    // subdivisions
+            ),
+            "shot",
+        ),
    ];

  const K_CUBE_VERTICES: usize = 0;
+const K_SHOT_VERTICES: usize = 1;
```

Now let's add some code to shoot projectiles. Each shot keeps its node, a
velocity, and an end time.

```rust
+const K_SHOT_VELOCITY: f32 = 100.0; // units per second
+
+struct Shot {
+    node: NodeNdx,
+    velocity: [f32; 3],
+    end_time: f64,
+}
```

On the page, the Fire! button just bumps a counter setting

```js
+  let fireCount = 0;
+  gui.addButton('Fire!', () => wasm.set_setting_num('fire', ++fireCount));
```

In JavaScript `fireShot` ran when the button was clicked, between frames.
Here we notice the counter changed inside the frame callback. We check it
after the world matrices were updated, so the finger tip's world matrix is
current, and we update the new node's world matrix ourselves since the
graph-wide update already ran.

```rust
+    // the shots in flight, and the value of the page's "fire" counter the
+    // last time we looked (the Fire! button bumps it)
+    let mut shots: Vec<Shot> = Vec::new();
+    let mut shot_id = 0;
+    let mut last_fire = 0.0f64;

    app.run(RenderMode::Continuous, move |frame: &Frame| {

    ...

            scene.update_world_matrix(root);

+            let fire = wgpu_fun::setting_f64("fire", 0.0);
+            if fire != last_fire {
+                last_fire = fire;
+                // fireShot
+                let node = scene.add_node(&format!("shot-{shot_id}"), None);
+                shot_id += 1;
+                scene.set_parent(node, Some(root));
+                scene.nodes[node].local_matrix = m4::translate(
+                    &scene.nodes[finger_tip].world_matrix,
+                    [0.0, 20.0, 0.0],
+                );
+                add_mesh(&mut meshes, node, K_SHOT_VERTICES, K_WHITE);
+                let velocity = vec3::mul_scalar(
+                    vec3::normalize(vec3::get_axis(
+                        &scene.nodes[finger_tip].world_matrix,
+                        1,
+                    )),
+                    K_SHOT_VELOCITY,
+                );
+                shots.push(Shot {
+                    node,
+                    velocity,
+                    end_time: now + 5.0,
+                });
+                scene.update_world_matrix(node);
+            }

            for mesh in &meshes {
                draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
            }
```

This code adds a `Shot` to the `shots` list. This includes a `node`,
a `velocity`, and an `end_time`.

The `node` is positioned 20 units out on the Y axis. This is because the
code to make a cone vertices makes the tip 20 units out so we need to
compensate. We could go modify the cone vertex code instead but this was
less work 😅.  Notice we are not adding a `TRS` source for this node.
We will update the local matrix directly.

`velocity` is the direction and speed to move the shot. We call `vec3::get_axis`
to get the y axis as the direction to shoot as that's the axis the fingers point. As we covered in
[the article on 3d math](webgpu-orthographic-projection.html), the y
axis is the 2nd row of the matrix or elements 4,5,6 so `vec3::get_axis`
can be implemented like this

```rust
mod vec3 {
  ...
+    // 0 = x, 1 = y, 2 = z;
+    pub fn get_axis(m: &[f32; 16], axis: usize) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        let offset = axis * 4;
+        dst[0] = m[offset + 0];
+        dst[1] = m[offset + 1];
+        dst[2] = m[offset + 2];
+
+        dst
+    }
  ...
}
```

Or code gets that y axis and normalizes that direction and then uses
`vec3::mul_scalar` to get it to our desired velocity.

We need to supply `vec3::mul_scalar`

```rust
mod vec3 {
  ...
+    pub fn mul_scalar(a: [f32; 3], scale: f32) -> [f32; 3] {
+        let mut dst = [0.0; 3];
+
+        dst[0] = a[0] * scale;
+        dst[1] = a[1] * scale;
+        dst[2] = a[2] * scale;
+
+        dst
+    }
  ...
}
```

Finally the `end_time` is some time in the future to remove the shot.

With that, let's add some code to move the projectiles. It goes at the very
end of the frame callback, after submitting the commands, just like the
JavaScript version called `processShots` at the end of `render`.

```rust
        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);

+        // processShots
+        if !shots.is_empty() {
+            while !shots.is_empty() && shots[0].end_time <= now {
+                let shot = shots.remove(0);
+                scene.set_parent(shot.node, None);
+                remove_mesh(&mut meshes, shot.node);
+            }
+            for shot in shots.iter() {
+                let v = vec3::mul_scalar(shot.velocity, delta_time as f32);
+                scene.nodes[shot.node].local_matrix =
+                    m4::multiply(&m4::translation(v), &scene.nodes[shot.node].local_matrix);
+            }
+        }
    });
```

That code checks if the shot's time has expired. If so it removes the shot's node
from the scene graph and it removes the mesh from the list of things to render.

Otherwise, for each shot in the list, it adds the velocity to the shot's matrix,
scaling it by the `delta_time` so it's framerate independent.

We need to supply `remove_mesh`. The JavaScript version removed a mesh by
object identity; each of our shot nodes has exactly one mesh so we can
remove it by its node index.

```rust
+// The JS version removed a mesh by object identity; each of our shot nodes
+// has exactly one mesh so we can remove it by its node index.
+fn remove_mesh(meshes: &mut Vec<Mesh>, node: NodeNdx) {
+    if let Some(ndx) = meshes.iter().position(|mesh| mesh.node == node) {
+        meshes.remove(ndx);
+    }
+}
```

Lastly, we want the animation clock to keep running while there are shots
in flight so they keep moving even when "animate" is off.

```rust
-        let is_running = settings_animate;
+        let is_running = settings_animate || !shots.is_empty();
```

The JavaScript version also had to `requestRender` while shots were in
flight; since our example is `RenderMode::Continuous` that happens on its
own.

{{{example url="../webgpu-scene-graphs-hand-shoot.html"}}}

Try selecting one of the index fingers, adjusting the rotation x, and then
pressing 'Fire!'. Or click 'Fire!' while it's animating.

This article should have given you some idea of what a scene graph is and how to
use one. Unity, Blender, Unreal, Maya, 3DSMax, Three.js, all have a scene graph.
They can take different forms. Some put the meshes in the graph itself making it
non-homogenous. Others are more "pure" and keep them separate. Some have fairly
complex "source" classes. Having a scene graph is generally the start of a 3d
engine. Not every 3d engine has one but most do.

In our code above we kept the camera itself outside of the scene graph but it's
more common for the camera to be part of the graph itself. That's how you can
see and manipulate multiple cameras in programs like Unity, Unreal, Blender,
etc...

By putting it in the graph itself we can have the camera be a child of some node
and therefore have it affected by it's parent. For example, a camera from the
perspective of the driver of a car or a camera on a rotating security camera.

Similarly, scene graphs can help with implementing 3d manipulators like many
3d editors have. These are the UI elements that let you translate, rotate,
and scale objects in the 3D view rather than from some separate GUI like we
used above. Maybe we can cover 3D manipulators in another article.
