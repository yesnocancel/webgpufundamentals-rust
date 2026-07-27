Title: WebGPU Bind Group Layouts
Description: Explicit Bind Group Layouts
TOC: Bind Group Layouts

Bind Group Layouts are used to make it easy and efficient
for WebGPU to match Bind Groups to Compute and Render Pipelines.

## How it works: 

A Pipeline, like a `ComputePipeline` or `RenderPipeline`
uses a `PipelineLayout` which defines 0 or more
`BindGroupLayout`s. Each `BindGroupLayout` is assigned
to a specific group index.

<div class="webgpu_center"><img src="resources/webgpu-bind-group-layouts.svg" style="width: 900px;"></div>

Bind Groups are each created with a specific `BindGroupLayout`
as well.

When you go to `draw` or to `dispatch_workgroups`, WebGPU only
needs to check, does the `BindGroupLayout` for each group index
on the current pipeline's `PipelineLayout` match the
currently bound bind groups, the ones set with `set_bind_group`.
This check is trivially simple. Most of the detailed checking
happens when you create the bind group. That way, when you're
actually drawing or computing, there's almost nothing left to
check.

Pipelines will generate their own `PipelineLayout` and
populate it with `BindGroupLayout`s automatically if you
create the pipeline with `layout: None` (JavaScript's
`layout: 'auto'`) which is what
most of the samples on this website do.

There are 2 main reasons to **NOT** use `layout: None`.

1. **You want a layout that's different than the default auto layout**

   For example you want to use a `rgba32float` as a texture
   but you get an error when you try. (see below)

2. **You want to use a bind group with more than 1 pipeline**

   You can not use a bind group made from a bindGroupLayout
   that was made from a pipeline with `layout: None` with a
   different pipeline.

## <a id="a-rgba32float"></a> Using a bind group layout different than `layout: None` - `'rgba32float'`

The rules for how a bind group layout is automatically created are
[detailed in the spec](https://www.w3.org/TR/webgpu/#abstract-opdef-default-pipeline-layout), but, as one example...

Let's say we want to use an `rgba32float` texture. Let's take
[our first example of using a texture from the article on textures](webgpu-textures.html) which drew an upside down 5x7 texel 'F'.  Let's update it to use an `rgba32float` texture.

Here are the changes.

```rust
  const K_TEXTURE_WIDTH: u32 = 5;
  const K_TEXTURE_HEIGHT: u32 = 7;
-  let r: [u8; 4] = [255, 0, 0, 255]; // red
-  let y: [u8; 4] = [255, 255, 0, 255]; // yellow
-  let b: [u8; 4] = [0, 0, 255, 255]; // blue
+  let r: [f32; 4] = [1.0, 0.0, 0.0, 1.0]; // red
+  let y: [f32; 4] = [1.0, 1.0, 0.0, 1.0]; // yellow
+  let b: [f32; 4] = [0.0, 0.0, 1.0, 1.0]; // blue
  let texture_data = [
    b, r, r, r, r,
    r, y, y, y, r,
    r, y, r, r, r,
    r, y, y, r, r,
    r, y, r, r, r,
    r, y, r, r, r,
    r, r, r, r, r,
  ]
  .concat();

  let texture = app.device.create_texture(&wgpu::TextureDescriptor {
    label: Some("yellow F on red"),
    size: wgpu::Extent3d {
      width: K_TEXTURE_WIDTH,
      height: K_TEXTURE_HEIGHT,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
-    format: wgpu::TextureFormat::Rgba8Unorm,
+    format: wgpu::TextureFormat::Rgba32Float,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  app.queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture: &texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
-    &texture_data,
+    bytemuck::cast_slice(&texture_data),
    wgpu::TexelCopyBufferLayout {
      offset: 0,
-      bytes_per_row: Some(K_TEXTURE_WIDTH * 4),
+      bytes_per_row: Some(K_TEXTURE_WIDTH * 4 * 4),
      rows_per_image: None,
    },
    wgpu::Extent3d {
      width: K_TEXTURE_WIDTH,
      height: K_TEXTURE_HEIGHT,
      depth_or_array_layers: 1,
    },
  );
```

When we run it we'll get an error. In the JavaScript API the error
just gets logged to the console. wgpu is stricter: by default an
uncaptured validation error will *panic*. To mirror the JavaScript
behavior of the sample, we push an *error scope* around the call that
fails, print the error like the console would, and skip rendering.

```rust
  let error_scope = app.device.push_error_scope(wgpu::ErrorFilter::Validation);
  let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    ...
  });
  let error = error_scope.pop().await;
  if let Some(error) = &error {
    print(&format!("WebGPU GPUValidationError: {error}"));
  }
```

{{{example url="../webgpu-bind-group-layouts-rgba32float-broken.html"}}}

The error I got was:

> - WebGPU GPUValidationError: Validation Error`<br>
> - Caused by: In Device::create_bind_group`<br>
> - Texture binding 1 expects sample type Float { filterable: true }, but was given a view with format Rgba32Float (sample type Float { filterable: false })`

What's up with that? It turns out that `rgba32float` (and all `xxx32float`)
textures are not filterable by default. There is an [optional feature](webgpu-limits-and-features.html) (`Features::FLOAT32_FILTERABLE`) to make them filterable but, that
feature might not be available everywhere. This is especially likely on
mobile devices, at least in 2024.

By default, when you declare a binding with a `texture_2d<f32>` like
this:

```wgsl
      @group(0) @binding(1) var ourTexture: texture_2d<f32>;
```

And you use `layout: None` when creating your pipeline, WebGPU creates
a bind group layout that specifically requires filterable textures. If
you try to bind an unfilterable one you get an error.

If you want to use a texture that can not be filtered then you'll need
to manually create a bind group layout.

There's a tool, [here](resources/wgsl-offset-computer.html), that if you
paste your shaders, it will generate the auto layout for you. It emits
the JavaScript form; pasting
in the shader from the example above and translating to wgpu it gives me

```rust
let bind_group_layout_descriptors = [
  wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      },
    ],
  },
];
```

This is an array of `BindGroupLayoutDescriptor`s. Above you can
see the texture binding uses `TextureSampleType::Float { filterable: true }`.
That's the type for
`Rgba8Unorm` but it's not the type for `Rgba32Float`. You can read
the sample types a particular texture format works with in
[this table in the spec](https://www.w3.org/TR/webgpu/#texture-format-caps).

To fix the example we need to adjust both the texture binding and the
sampler binding. The sampler binding needs to be changed into a
non-filtering sampler. The texture binding needs to be changed to
an unfilterable float.

So, first, we need to create a `BindGroupLayout`

```rust
  let bind_group_layout = app
    .device
    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::FRAGMENT,
*          ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
*            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
      ],
    });
```

The two changes are marked above.

Then we need to create a `PipelineLayout` which holds the list
of the `BindGroupLayout`s used by a pipeline.

```rust
  let pipeline_layout = app
    .device
    .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[Some(&bind_group_layout)],
      immediate_size: 0,
    });
```

`create_pipeline_layout` takes a slice of `Option<&BindGroupLayout>`s.
They are ordered by group index so the first entry becomes `@group(0)`,
the 2nd entry becomes `@group(1)`, etc... If you need
to skip one you'll need to add a `None` element. (`immediate_size` is
for a wgpu-specific extension; `0` is the WebGPU-compatible value.)

Finally, when we create the pipeline, we pass in the pipeline layout

```rust
  let pipeline = app
    .device
    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("hardcoded textured quad pipeline"),
-      layout: None,
+      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &module,
        ...
      },
      fragment: Some(wgpu::FragmentState {
        module: &module,
        ...
        targets: &[Some(app.format.into())],
      }),
      ...
    });
```

With that, our example works again but now it's using an `rgba32float`
texture.

{{{example url="../webgpu-bind-group-layouts-rgba32float-fixed.html"}}}

Note: the example works both because we did the work above to make
a bind group layout that accepted an unfilterable float but it also happens
to work because the example uses a `Sampler` using only nearest
filtering. If we set any of the filters, `mag_filter`, `min_filter` or
`mipmap_filter` to `wgpu::FilterMode::Linear` we'd get an error saying that we tried
to use a filtering sampler on a non-filtering sampler binding.

## Using a bind group layout different than `layout: None` - dynamic offsets

By default, when you make a bind group and you bind a uniform or storage buffer, the entire buffer is bound. You can also pass in an offset and length when creating your bind group. In both cases, once set, they can not
be changed.

WebGPU has an option to let you change the offset when you call
`set_bind_group`. To use this feature, you have to manually create bind group
layouts and set `has_dynamic_offset: true` for each binding you want to be
able to set later.

To keep this simple, let's use the simple compute example
from [the article on fundamentals](webgpu-fundamentals.html#a-run-computations-on-the-gpu). We'll modify it to add
2 sets of values from the same buffer and we'll choose which
set using dynamic offsets.

First lets change the shader to this

```wgsl
@group(0) @binding(0) var<storage, read_write> a: array<f32>;
@group(0) @binding(1) var<storage, read_write> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> dst: array<f32>;

@compute @workgroup_size(1) fn computeSomething(
  @builtin(global_invocation_id) id: vec3u
) {
  let i = id.x;
  dst[i] = a[i] + b[i];
}
```

you can see it just adds `a` to `b` and writes into `dst`.

Next let's make the bind group layout

```rust
  let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: true,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: true,
          min_binding_size: None,
        },
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 2,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Storage { read_only: false },
          has_dynamic_offset: true,
          min_binding_size: None,
        },
        count: None,
      },
    ],
  });
```

All of them are marked as `has_dynamic_offset: true`

now let's use it to create our pipeline

```rust
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&bind_group_layout)],
    immediate_size: 0,
  });

  let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
-    label: Some("double compute pipeline"),
-    layout: None,
+    label: Some("add elements compute pipeline"),
+    layout: Some(&pipeline_layout),
    module: &module,
    entry_point: None,
    compilation_options: Default::default(),
    cache: None,
  });
```

Let's setup the buffer. Offset must be a multiple of 256 [^minStorageBufferOffsetAlignment] so, let's create a buffer
256 * 3 bytes large so we have at least 3 valid offsets, 0, 256, and 512.

[^minStorageBufferOffsetAlignment]: It's possible your device
supports smaller offsets. See the `min_storage_buffer_offset_alignment`
or `min_uniform_buffer_offset_alignment` limits in [limits and features](webgpu-limits-and-features.html).

```rust
-  let input: [f32; 3] = [1.0, 3.0, 5.0];
+  let mut input = [0.0f32; 64 * 3];
+  input[0..3].copy_from_slice(&[1.0, 3.0, 5.0]);
+  input[64..64 + 3].copy_from_slice(&[11.0, 12.0, 13.0]);

  // create a buffer on the GPU to hold our computation
  // input and output
  let work_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("work buffer"),
    size: std::mem::size_of_val(&input) as u64,
    usage: wgpu::BufferUsages::STORAGE
      | wgpu::BufferUsages::COPY_SRC
      | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  // Copy our input data to that buffer
  queue.write_buffer(&work_buffer, 0, bytemuck::cast_slice(&input));
```

The code above makes an array of `64 * 3` 32bit floats. That's 768 bytes.

Since our original example read and wrote to the same buffer
we'll just bind the same buffer 3 times.

```rust
  // Setup a bindGroup to tell the shader which
  // buffers to use for the computation
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bindGroup for work buffer"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[
-      wgpu::BindGroupEntry {
-        binding: 0,
-        resource: work_buffer.as_entire_binding(),
-      },
+      wgpu::BindGroupEntry {
+        binding: 0,
+        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
+          buffer: &work_buffer,
+          offset: 0,
+          size: Some(wgpu::BufferSize::new(256).unwrap()),
+        }),
+      },
+      wgpu::BindGroupEntry {
+        binding: 1,
+        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
+          buffer: &work_buffer,
+          offset: 0,
+          size: Some(wgpu::BufferSize::new(256).unwrap()),
+        }),
+      },
+      wgpu::BindGroupEntry {
+        binding: 2,
+        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
+          buffer: &work_buffer,
+          offset: 0,
+          size: Some(wgpu::BufferSize::new(256).unwrap()),
+        }),
+      },
    ],
  });
```

Note, we must specify the size, otherwise (`size: None`) it will default to
the rest of the buffer. If we were to then set an offset > 0 we'd get an
error since we'd be specifying a portion of the buffer that's out of range.

In `set_bind_group` we now pass in 1 offset for each buffer that has dynamic offsets. Since we marked all 3 entries in the bind group layout as
`has_dynamic_offset: true` we need 3 offsets in the order of their binding slot.

```rust
  ...
  pass.set_pipeline(&pipeline);
-  pass.set_bind_group(0, &bind_group, &[]);
+  pass.set_bind_group(0, &bind_group, &[0, 256, 512]);
  pass.dispatch_workgroups(3, 1, 1);
```

Finally, we need to change the code to show the result

```rust
-  print(&format!("input {input:?}"));
-  print(&format!("result {result:?}"));
+  print(&format!("a {:?}", &input[0..3]));
+  print(&format!("b {:?}", &input[64..64 + 3]));
+  print(&format!("dst {:?}", &result[128..128 + 3]));
```

{{{example url="../webgpu-bind-group-layouts-dynamic-offsets.html"}}}

Note that, using dynamic offsets is slightly slower than non-dynamic offsets. The reason is, with non-dynamic offsets, whether the offset and size are in range of the buffer is checked when you create the bind group. With dynamic offsets, that check can not be made until you call `set_bind_group`. If you're only calling `set_bind_group` a few hundred times
that difference probably won't matter. If you're calling `set_bind_group`
1000s of times it might be more noticeable.

## <a id="a-sharing-bind-groups"></a> Using a bind group with more than 1 pipeline

Another reason to create bind group layouts manually is so we
can use the same bind group with more than one pipeline.

A common places you might want to be able to reuse a bind group is in a basic 3d scene renderer with shadows.

In a basic 3d scene renderer it's common to separate bindings
into

* globals (like the perspective and view matrices)
* materials (the textures, colors)
* locals (like the model matrix)

You then render like this

```
set_bind_group(0, globals_bg)
for each material
  set_bind_group(1, material_bg)
  for each object that uses material
    set_bind_group(2, local_bg)
    draw(...)
```

When you add [shadows](webgpu-shadows.html), you need to first
draw the shadow maps with a shadow map pipeline. Rather than
having separate bind groups of all of those things, ones to work
with the pipeline that draws and different bind groups to work
with the pipeline that renders the shadow map, it would be much
easier to just make one set of bind groups and use the same ones
for both cases.

That's a rather large sample to write, just to show off sharing
bind groups. Although, [the article on shadows](webgpu-shadows.html)
uses shared bind groups we'll take the simple compute example from [the article on fundamentals](webgpu-fundamentals.html#a-run-computations-on-the-gpu) again and make it use 2 compute pipelines with one bind group.

First let's add another shader module that adds 3

```rust
-  let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
+  let module_times2 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("doubling compute module"),
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
      @group(0) @binding(0) var<storage, read_write> data: array<f32>;

      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        let i = id.x;
        data[i] = data[i] * 2.0;
      }
    "#
      .into(),
    ),
  });

+  let module_plus3 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
+    label: Some("adding 3 compute module"),
+    source: wgpu::ShaderSource::Wgsl(
+      /* wgsl */ r#"
+      @group(0) @binding(0) var<storage, read_write> data: array<f32>;
+
+      @compute @workgroup_size(1) fn computeSomething(
+        @builtin(global_invocation_id) id: vec3u
+      ) {
+        let i = id.x;
+        data[i] = data[i] + 3.0;
+      }
+    "#
+      .into(),
+    ),
+  });
```

Then let's create a `BindGroupLayout` and `PipelineLayout`
we can use to make the 2 pipelines share the same `BindGroup`.

```rust
  let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: false },
        has_dynamic_offset: false,
        min_binding_size: None,
      },
      count: None,
    }],
  });

  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&bind_group_layout)],
    immediate_size: 0,
  });
```

Now let's use them when creating the pipelines.

```rust
-  let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
+  let pipeline_times2 = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("doubling compute pipeline"),
-    layout: None,
+    layout: Some(&pipeline_layout),
    module: &module_times2,
    entry_point: None,
    compilation_options: Default::default(),
    cache: None,
  });

+  let pipeline_plus3 = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
+    label: Some("plus 3 compute pipeline"),
+    layout: Some(&pipeline_layout),
+    module: &module_plus3,
+    entry_point: None,
+    compilation_options: Default::default(),
+    cache: None,
+  });
```

When we setup the bind group, let's use the `bind_group_layout`
directly

```rust
  // Setup a bindGroup to tell the shader which
  // buffer to use for the computation
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bindGroup for work buffer"),
-    layout: &pipeline.get_bind_group_layout(0),
+    layout: &bind_group_layout,
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: work_buffer.as_entire_binding(),
    }],
  });
```

Finally let's use both pipelines

```rust
  // Encode commands to do the computation
  let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
  {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
-    pass.set_pipeline(&pipeline);
+    pass.set_pipeline(&pipeline_times2);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.dispatch_workgroups(input.len() as u32, 1, 1);
+    pass.set_pipeline(&pipeline_plus3);
+    pass.dispatch_workgroups(input.len() as u32, 1, 1);
  }
```

The result is we multiply by 2 and add 3 with one bind group.

{{{example url="../webgpu-bind-group-layouts-multiple-pipelines.html"}}}

Not very exciting but at least it's a working and simple example.

When to manually make bind group layouts and when to not is really up
to you. In the example above it would arguably have been easier to
just make 2 bind groups, 1 for each pipeline.

For simple situations it's often not necessary to manually make bind group layouts but, as your
WebGPU programs get more complex, it's likely making bind group layouts
will be a technique you reach for.

## <a id="a-bind-group-layout-notes"></a> Bind Group Layout notes:

Some things to note about creating a `BindGroupLayout`

* ## Each entry must declare which `binding` it is for

* ## Each entry must declare which stages it will be visible in.

  In our examples above we declared just one visibility.
  If, for example, we wanted to reference the bind group both
  the vertex and the fragment shader we'd use:

  ```rust
     visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX
  ```

  or all 3 stages:

  ```rust
     visibility: wgpu::ShaderStages::COMPUTE
       | wgpu::ShaderStages::FRAGMENT
       | wgpu::ShaderStages::VERTEX
  ```

* ## There are no defaults in wgpu:

  In the JavaScript API most of the fields of a bind group layout
  entry have defaults so, in the most common sampler and texture
  usages, you can declare an entry as just `sampler: {}` or
  `texture: {}`. In wgpu every field is spelled out explicitly.
  The equivalents of those JavaScript defaults are:

  For `BindingType::Texture` bindings:

  ```rust
  wgpu::BindingType::Texture {
    sample_type: wgpu::TextureSampleType::Float { filterable: true },
    view_dimension: wgpu::TextureViewDimension::D2,
    multisampled: false,
  }
  ```

  For a `BindingType::Sampler` binding:

  ```rust
  wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
  ```

* ## buffer entries should declare a `min_binding_size` when possible.

  When you declare a buffer binding you can specify a `min_binding_size`.

  A good example might be you make a struct for uniforms. For example
  in [the article on uniforms](webgpu-uniforms.html) we had this struct:

  ```wgsl
  struct OurStruct {
    color: vec4f,
    scale: vec2f,
    offset: vec2f,
  };

  @group(0) @binding(0) var<uniform> ourStruct: OurStruct;
  ``` 

  It requires 32 bytes so, we should declare it's `min_binding_size` like
  this:

  ```rust
  let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[wgpu::BindGroupLayoutEntry {
      binding: 0,
      visibility: wgpu::ShaderStages::COMPUTE,
      ty: wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: Some(wgpu::BufferSize::new(32).unwrap()),
      },
      count: None,
    }],
  });
  ```

  The reason to declare a `min_binding_size` is it lets WebGPU check
  if your buffer size/offset is the correct size when you call
  `create_bind_group`.  If you don't set a `min_binding_size`, then
  WebGPU will have to check at draw/dispatch_workgroups time that
  the buffer is the correct size of the pipeline. Checking every
  draw calls is slower than checking once when you create a bind
  group.

  On the the other hand, in our example above that used a storage
  buffer to double numbers etc, we didn't declare a `min_binding_size`.
  That's because, since the storage buffer is declared as an `array`,
  are able to bind different size buffers depending on how
  many values you pass in.


[This part of the spec](https://www.w3.org/TR/webgpu/#dictdef-gpubindgrouplayoutentry) details all the options for making
bind group layouts.

[This article](https://toji.dev/webgpu-best-practices/bind-groups) also
has some advice on bind groups and bind group layouts.

[This Library](https://greggman.github.io/webgpu-utils) will compute
struct sizes and default bind group layouts for you.
