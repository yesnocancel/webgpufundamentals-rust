Title: WebGPU Highlighting
Description: Highlighting Selected Objects
TOC: Highlighting

This article is the 1st in a short series
about making parts of a 3D editor. Each one builds on the previous lesson so you may find them easiest to understand by reading them in order.
These article assumes you've already read
[the article on scene graphs](webgpu-scene-graphs.html) as well as
[the article on post processing](webgpu-post-processing.html).

{{{toc-steps list="editor.hanson"}}}

Let's assume we want to make a kind of simple 3D editor with inspiration from 
Blender or Maya or Unity or Unreal. We want something that lets us select and
manipulate objects in 3D. We kind of started this path in
[the article on scene graphs](webgpu-scene-graphics.html) where we had nodes
and we could select one from buttons in the UI and edit that node's translation,
rotation, and scale. It would be nice if we could see visually, which one was
selected. Let's do that.

Starting with [the example where we first added the ability to select nodes](webgpu-scene-graphs.html#a-gui), we started with a scene like this

<div class="webgpu_center center">
  <div data-diagram="standardPass" style="width: 600px"></div>
</div>

To highlight what's selected we could render just what's selected
to a separate texture.

<div class="webgpu_center center">
  <div data-diagram="selectedPass" style="width: 600px"></div>
</div>

The alpha values would effectively make a silhouette of the selected objects.

<div class="webgpu_center center">
  <div data-diagram="alpha" style="width: 600px"></div>
</div>

We could then use that alpha mask as input to a post process like pass where
we draw the highlight color if the mask's alpha is 0 but there's a non-zero
value nearby. This would effectively give us an outline.

<div class="webgpu_center center">
  <div data-diagram="outline" style="width: 600px"></div>
</div>

Here's a post processing like shader that given the alpha mask will draw an outline

```wgsl
struct VSOutput {
  @builtin(position) position: vec4f,
  @location(0) texcoord: vec2f,
};

@vertex fn vs(
  @builtin(vertex_index) vertexIndex : u32,
) -> VSOutput {
  var pos = array(
    vec2f(-1.0, -1.0),
    vec2f(-1.0,  3.0),
    vec2f( 3.0, -1.0),
  );

  var vsOutput: VSOutput;
  let xy = pos[vertexIndex];
  vsOutput.position = vec4f(xy, 0.0, 1.0);
  vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
  return vsOutput;
}

@group(0) @binding(0) var mask: texture_2d<f32>;

fn isOnEdge(pos: vec2i) -> bool {
  // Note: we need to make sure we don't use out of bounds
  // texel coordinates with textureLoad as that returns
  // different results on different GPUs
  let size = vec2i(textureDimensions(mask, 0));
  let start = max(pos - 2, vec2i(0));
  let end = min(pos + 2, size);

  for (var y = start.y; y <= end.y; y++) {
    for (var x = start.x; x <= end.x; x++) {
      let s = textureLoad(mask, vec2i(x, y), 0).a;
      if (s > 0) {
        return true;
      }
    }
  }
  return false;
};

@fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
  let pos = vec2i(fsInput.position.xy);

  // Get the current texel.
  // If it's not 0 we're inside the selected objects
  let s = textureLoad(mask, pos, 0).a;
  if (s > 0) {
    discard;
  }

  let hit = isOnEdge(pos);
  if (!hit) {
    discard;
  }
  return vec4f(1, 0.5, 0, 1); // orange
}
```

The shader first checks if the pixel in the mask is > 0. If it is
then it's inside the mask which represent the selected objects and
so we don't want to draw anything and so we `discard`.

Otherwise, it calls `isOnEdge` to check neighboring pixels.
If non of them are > 0 then it's not the edge and we don't draw
anything via `discard`.

Otherwise we were at an edge and draw orange.

Now that we have a shader we need the post processing setup code
from [the article on post processing](webgpu-post-processing.html).
Compared to that article we can drop the sampler and the uniform
buffer — the outline shader only needs the mask texture.

```rust
  let post_process_module = app
      .device
      .create_shader_module(wgpu::ShaderModuleDescriptor {
          label: None,
          source: wgpu::ShaderSource::Wgsl(
              /* wgsl */ r#"
    struct VSOutput {
      @builtin(position) position: vec4f,
      @location(0) texcoord: vec2f,
    };

    @vertex fn vs(
      @builtin(vertex_index) vertexIndex : u32,
    ) -> VSOutput {
      var pos = array(
        vec2f(-1.0, -1.0),
        vec2f(-1.0,  3.0),
        vec2f( 3.0, -1.0),
      );

      var vsOutput: VSOutput;
      let xy = pos[vertexIndex];
      vsOutput.position = vec4f(xy, 0.0, 1.0);
      vsOutput.texcoord = xy * vec2f(0.5, -0.5) + vec2f(0.5);
      return vsOutput;
    }

    @group(0) @binding(0) var mask: texture_2d<f32>;

    fn isOnEdge(pos: vec2i) -> bool {
      // Note: we need to make sure we don't use out of bounds
      // texel coordinates with textureLoad as that returns
      // different results on different GPUs
      let size = vec2i(textureDimensions(mask, 0));
      let start = max(pos - 2, vec2i(0));
      let end = min(pos + 2, size);

      for (var y = start.y; y <= end.y; y++) {
        for (var x = start.x; x <= end.x; x++) {
          let s = textureLoad(mask, vec2i(x, y), 0).a;
          if (s > 0) {
            return true;
          }
        }
      }
      return false;
    };

    @fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
      let pos = vec2i(fsInput.position.xy);

      // get the current. If it's not 0 we're inside the selected objects
      let s = textureLoad(mask, pos, 0).a;
      if (s > 0) {
        discard;
      }

      let hit = isOnEdge(pos);
      if (!hit) {
        discard;
      }
      return vec4f(1, 0.5, 0, 1);
    }
  "#
              .into(),
          ),
      });

  let post_process_pipeline = app
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
          label: None,
          layout: None,
          vertex: wgpu::VertexState {
              module: &post_process_module,
              entry_point: None,
              compilation_options: Default::default(),
              buffers: &[],
          },
          fragment: Some(wgpu::FragmentState {
              module: &post_process_module,
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

-  let post_process_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
-      min_filter: wgpu::FilterMode::Linear,
-      mag_filter: wgpu::FilterMode::Linear,
-      ..Default::default()
-  });
```

In the post processing article the scene was rendered to a render target
texture and post processed from there to the canvas. Here it's the other
way around: the scene is rendered normally to the canvas, only the
*selected* objects are rendered to a texture, and the outline pass reads
that texture while drawing **on top of** what's already on the canvas.
That means the post process render pass must `load` the existing canvas
contents instead of clearing them, and its bind group needs only the mask.

```rust
-  let mut render_target: Option<wgpu::Texture> = None;
+  let mut post_texture: Option<wgpu::Texture> = None;
  let mut post_process_bind_group: Option<wgpu::BindGroup> = None;
```

and when we (re)create the texture we (re)create the bind group.
This is the JS version's `setupPostProcess(texture)`.

```rust
      // setupPostProcess in the JS version: if the texture changed,
      // remake the bind group.
      if post_process_bind_group.is_none() || size_changed {
          post_process_bind_group =
              Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
                  label: None,
                  layout: &post_process_pipeline.get_bind_group_layout(0),
                  entries: &[wgpu::BindGroupEntry {
                      binding: 0,
                      resource: wgpu::BindingResource::TextureView(
                          &post_texture.as_ref().unwrap().create_view(&Default::default()),
                      ),
                  }],
-                 wgpu::BindGroupEntry {
-                     binding: 1,
-                     resource: wgpu::BindingResource::Sampler(&post_process_sampler),
-                 },
-                 wgpu::BindGroupEntry {
-                     binding: 2,
-                     resource: post_process_uniform_buffer.as_entire_binding(),
-                 },
              }));
      }
```

The post process pass itself becomes

```rust
      // Draw outline based on alpha of postTexture
      // on to the canvasTexture
      {
          let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
              label: Some("post process render pass"),
              color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                  view: frame.view,
                  resolve_target: None,
                  ops: wgpu::Operations {
-                     load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
+                     load: wgpu::LoadOp::Load,
                      store: wgpu::StoreOp::Store,
                  },
                  depth_slice: None,
              })],
              ..Default::default()
          });
          pass.set_pipeline(&post_process_pipeline);
          pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
          pass.draw(0..3, 0..1);
      }
```

We also need to use the post processing objects when rendering.
First we pull the view projection matrix computation out in front of the
encoder so both render passes can use it, and, since the second pass keeps
allocating `ObjectInfo`s where the first one stopped, we pass the object
count out of the first pass's block.

```rust
    app.run(RenderMode::Once, move |frame: &Frame| {

        ...

+        let camera_rotation =
+            wgpu_fun::setting_f64("cameraRotation", (-45.0f64).to_radians()) as f32;
+
+        let aspect = frame.width as f32 / frame.height as f32;
+        let projection = m4::perspective(
+            60.0f32.to_radians(), // fieldOfView,
+            aspect,
+            1.0,    // zNear
+            2000.0, // zFar
+        );
+
+        // Compute a camera matrix
+        let mut camera_matrix = m4::identity();
+        camera_matrix = m4::translate(&camera_matrix, [120.0, 100.0, 0.0]);
+        camera_matrix = m4::rotate_y(&camera_matrix, camera_rotation);
+        camera_matrix = m4::translate(&camera_matrix, [60.0, 0.0, 300.0]);
+
+        // Compute a view matrix
+        let view_matrix = m4::inverse(&camera_matrix);
+
+        // combine the view and projection matrixes
+        let view_projection_matrix = m4::multiply(&projection, &view_matrix);

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
-        {
+        let object_ndx = {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                ...
            });
            pass.set_pipeline(&pipeline);

            let mut ctx = Ctx {
                pass: &mut pass,
                view_projection_matrix,
                device: frame.device,
                queue: frame.queue,
                pipeline: &pipeline,
                object_infos: &mut object_infos,
                object_ndx: 0,
            };
            scene.update_world_matrix(root);
            for mesh in &meshes {
                draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
            }
+            ctx.object_ndx
        };

+        // draw selected objects to postTexture
+        {
+            let size_changed = post_texture
+                .as_ref()
+                .is_none_or(|t| t.width() != frame.width || t.height() != frame.height);
+            if size_changed {
+                if let Some(texture) = post_texture.take() {
+                    texture.destroy();
+                }
+                post_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
+                    label: None,
+                    size: wgpu::Extent3d {
+                        width: frame.width,
+                        height: frame.height,
+                        depth_or_array_layers: 1,
+                    },
+                    format: frame.format,
+                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
+                        | wgpu::TextureUsages::TEXTURE_BINDING,
+                    mip_level_count: 1,
+                    sample_count: 1,
+                    dimension: wgpu::TextureDimension::D2,
+                    view_formats: &[],
+                }));
+            }
+            // setupPostProcess in the JS version: if the texture changed,
+            // remake the bind group.
+            if post_process_bind_group.is_none() || size_changed {
+                post_process_bind_group =
+                    Some(frame.device.create_bind_group(&wgpu::BindGroupDescriptor {
+                        label: None,
+                        layout: &post_process_pipeline.get_bind_group_layout(0),
+                        entries: &[wgpu::BindGroupEntry {
+                            binding: 0,
+                            resource: wgpu::BindingResource::TextureView(
+                                &post_texture.as_ref().unwrap().create_view(&Default::default()),
+                            ),
+                        }],
+                    }));
+            }
+
+            let post_texture_view = post_texture
+                .as_ref()
+                .unwrap()
+                .create_view(&Default::default());
+            {
+                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
+                    label: Some("our basic canvas renderPass"),
+                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
+                        view: &post_texture_view,
+                        resolve_target: None,
+                        ops: wgpu::Operations {
+                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
+                            store: wgpu::StoreOp::Store,
+                        },
+                        depth_slice: None,
+                    })],
+                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
+                        view: &depth_view,
+                        depth_ops: Some(wgpu::Operations {
+                            load: wgpu::LoadOp::Clear(1.0),
+                            store: wgpu::StoreOp::Store,
+                        }),
+                        stencil_ops: None,
+                    }),
+                    ..Default::default()
+                });
+                pass.set_pipeline(&pipeline);
+
+                let mut ctx = Ctx {
+                    pass: &mut pass,
+                    view_projection_matrix,
+                    device: frame.device,
+                    queue: frame.queue,
+                    pipeline: &pipeline,
+                    object_infos: &mut object_infos,
+                    object_ndx,
+                };
+                for mesh in &selected_meshes {
+                    draw_mesh(&mut ctx, mesh, &scene, &vertex_sets);
+                }
+            }
+
+            // Draw outline based on alpha of postTexture
+            // on to the canvasTexture
+            {
+                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
+                    label: Some("post process render pass"),
+                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
+                        view: frame.view,
+                        resolve_target: None,
+                        ops: wgpu::Operations {
+                            load: wgpu::LoadOp::Load,
+                            store: wgpu::StoreOp::Store,
+                        },
+                        depth_slice: None,
+                    })],
+                    ..Default::default()
+                });
+                pass.set_pipeline(&post_process_pipeline);
+                pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
+                pass.draw(0..3, 0..1);
+            }
+        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
```

The code above draws the original scene. Then it draws `selected_meshes`
to `post_texture`. We pass that `post_texture` to the post processing code
to draw the outline onto the canvas.

Since we have 2 pieces of code recreating a texture if the size of another has changed
we could simplify the code a little by adding a helper.

```rust
+fn make_new_texture_if_size_different(
+    device: &wgpu::Device,
+    texture: Option<wgpu::Texture>,
+    (width, height): (u32, u32),
+    format: wgpu::TextureFormat,
+    usage: wgpu::TextureUsages,
+) -> wgpu::Texture {
+    if let Some(texture) = texture {
+        if texture.width() == width && texture.height() == height {
+            return texture;
+        }
+        texture.destroy();
+    }
+    device.create_texture(&wgpu::TextureDescriptor {
+        label: None,
+        size: wgpu::Extent3d {
+            width,
+            height,
+            depth_or_array_layers: 1,
+        },
+        format,
+        usage,
+        mip_level_count: 1,
+        sample_count: 1,
+        dimension: wgpu::TextureDimension::D2,
+        view_formats: &[],
+    })
+}

...

    app.run(RenderMode::Once, move |frame: &Frame| {
        ...

        // If we don't have a depth texture OR if its size is different
        // from the canvasTexture when make a new depth texture
-        if depth_texture
-            .as_ref()
-            .is_none_or(|t| t.width() != frame.width || t.height() != frame.height)
-        {
-            if let Some(texture) = depth_texture.take() {
-                texture.destroy();
-            }
-            depth_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
-                label: None,
-                size: wgpu::Extent3d {
-                    width: frame.width,
-                    height: frame.height,
-                    depth_or_array_layers: 1,
-                },
-                format: wgpu::TextureFormat::Depth24Plus,
-                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
-                mip_level_count: 1,
-                sample_count: 1,
-                dimension: wgpu::TextureDimension::D2,
-                view_formats: &[],
-            }));
-        }
+        depth_texture = Some(make_new_texture_if_size_different(
+            frame.device,
+            depth_texture.take(),
+            (frame.width, frame.height), // for size
+            wgpu::TextureFormat::Depth24Plus,
+            wgpu::TextureUsages::RENDER_ATTACHMENT,
+        ));

...

        // draw selected objects to postTexture
        {
            let size_changed = post_texture
                .as_ref()
                .is_none_or(|t| t.width() != frame.width || t.height() != frame.height);
-            if size_changed {
-                if let Some(texture) = post_texture.take() {
-                    texture.destroy();
-                }
-                post_texture = Some(frame.device.create_texture(&wgpu::TextureDescriptor {
-                    label: None,
-                    size: wgpu::Extent3d {
-                        width: frame.width,
-                        height: frame.height,
-                        depth_or_array_layers: 1,
-                    },
-                    format: frame.format,
-                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
-                        | wgpu::TextureUsages::TEXTURE_BINDING,
-                    mip_level_count: 1,
-                    sample_count: 1,
-                    dimension: wgpu::TextureDimension::D2,
-                    view_formats: &[],
-                }));
-            }
+            post_texture = Some(make_new_texture_if_size_different(
+                frame.device,
+                post_texture.take(),
+                (frame.width, frame.height), // for size
+                frame.format,
+                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
+            ));
            // setupPostProcess in the JS version: if the texture changed,
            // remake the bind group.
            if post_process_bind_group.is_none() || size_changed {
```

What's left is we need a way to fill out `selected_meshes`.
This is slightly complicated by the fact that we we made everything
out of cubes and by default we hide some of those nodes. Well take
that hiding into account when setting `selected_meshes`  by checking
all the children of a node for more meshes.

In the JavaScript original this happened on the page, in
`setCurrentSceneGraphNode`, because the GUI and the scene graph lived in
the same script. In our port [the page only mirrors the node tree and
sends the selected node's index as the `nodeNdx`
setting](webgpu-scene-graphs.html#a-gui), so the filtering happens on the
Rust side, where the real scene graph lives. Since nodes are indices into
the arena, `meshUsesNode` translates to

```rust
+fn mesh_uses_node(mesh: &Mesh, scene: &SceneGraph, node: NodeNdx) -> bool {
+    if mesh.node == node {
+        return true;
+    }
+    for &child in &scene.nodes[node].children {
+        if mesh_uses_node(mesh, scene, child) {
+            return true;
+        }
+    }
+    false
+}
```

and, at the top of the frame callback, right after we've read `nodeNdx`
and applied any pending TRS edit, we gather the selected meshes.

```rust
        let node_ndx = wgpu_fun::setting_f64("nodeNdx", 1.0) as usize;

        ...

+        // The page's GUI sends the selected node as `nodeNdx`; gather the
+        // meshes that node (or any of its children) uses. This is the JS
+        // version's `selectedMeshes = meshes.filter(...)` from
+        // `setCurrentSceneGraphNode`.
+        let selected_meshes: Vec<&Mesh> = meshes
+            .iter()
+            .filter(|mesh| mesh_uses_node(mesh, &scene, node_ndx))
+            .collect();
```

There is nothing to change on the page: clicking a node button already
sends `set_setting_num('nodeNdx', ndx)` and any settings change triggers a
re-render, which recomputes `selected_meshes`.

And with that the selected objects are highlighted.

{{{example url="../webgpu-highlighting.html"}}}

Now that we can highlight a selection, let's make it possible
to [move the camera by dragging](webgpu-camera-controls.html)
instead of having to use the buttons in the UI.

<!-- keep this at the bottom of the article -->
<link href="webgpu-highlighting.css" rel="stylesheet">
<script type="module" src="webgpu-highlighting.js"></script>
