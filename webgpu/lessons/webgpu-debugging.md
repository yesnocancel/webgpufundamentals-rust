Title: WebGPU Debugging and Errors
Description: Tips for debugging WebGPU
TOC: Debugging and Errors

Some tips on debugging WebGPU and dealing with errors.

With wgpu our examples run in two places: in the browser (compiled to
WebAssembly, going through the browser's WebGPU implementation) and
natively (going straight to Vulkan/Metal/DirectX). Errors show up in
different places in each, so the tips below cover both.

## Keep the JavaScript console open to see WebGPU errors

Most browsers have a JavaScript console. When you run the wasm builds
of the examples, keep it open. WebGPU should generally print errors
there. Browsers also have a GPU status page, `about://gpu` in Chrome,
which can tell you if WebGPU is even enabled and which adapter it's
using.

Running natively, "the console" is your terminal. wgpu reports
problems two ways:

* An uncaptured validation error **panics** by default with the full
  error message. Run with `RUST_BACKTRACE=1` and the backtrace points
  at the exact call that failed — arguably better than the browser,
  where the error appears asynchronously with no useful stack.

* Warnings and informational messages go through the standard Rust
  [`log`](https://docs.rs/log) crate. wgpu_fun installs `env_logger`,
  so you can turn them on from the environment:

  ```sh
  RUST_LOG=warn cargo run --bin webgpu-simple-triangle
  RUST_LOG=wgpu_core=info cargo run --bin webgpu-simple-triangle  # much chattier
  ```

## Consider logging uncaught errors

You can install a handler for uncaptured WebGPU errors and log them
yourself. In JavaScript that's
`device.addEventListener('uncapturederror', ...)`; in wgpu it's
`on_uncaptured_error`. For example

```rust
let (device, queue) = adapter
    .request_device(&wgpu::DeviceDescriptor::default())
    .await
    .expect("failed to create GPU device");
device.on_uncaptured_error(std::sync::Arc::new(|error: wgpu::Error| {
    wgpu_fun::log(&format!("{error}"));
}));
```

The JS version of this article suggests `alert`; you can log the
message, put it in an element, or in some way make it visible. Our
examples use `wgpu_fun::log`, which appends the message to the page in
the browser and prints to stdout natively. I find this useful because I
often forget the advice above, to open the JavaScript console, and then
I don't see the errors. 😅

Errors that WebGPU emits itself go to the JavaScript console (browser)
or panic (native) but errors that you capture go where you tell them
to. Note that on native, installing an `on_uncaptured_error` handler
**replaces** the default panic — you get the browser-like behavior of
logging the error and continuing. The panic default is actually great
for debugging, so only install a handler when you want your program to
keep going.

## Help WebGPU report errors

Errors in WebGPU are reported asynchronously. This is to keep WebGPU
fast and efficient. But, it sometimes means you might not get an error
at the time you expect it or at all, unless you help WebGPU.

Here's some code using the advice from above, adding a handler to
show uncaptured errors. It then compiles a shader module that
should get an error.

```rust
async fn main() {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .expect("need a browser that supports WebGPU");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .expect("need a browser that supports WebGPU");

    device.on_uncaptured_error(std::sync::Arc::new(|error: wgpu::Error| {
        log(&format!("{error}"));
    }));

    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            /* wgsl */ r#"
      this shader won't compile
    "#
            .into(),
        ),
    });

    log("--done--");
}
```

In the live example below, at least in Chrome 129, you probably won't
get an error.

{{{example url="../webgpu-debugging-help-webgpu-report-errors.html"}}}

Note: that's the browser. Native wgpu validates on the calling thread
and delivers uncaptured errors synchronously, so if you run this
example natively you see the error immediately, printed *before*
`--done--`, even without the fix below. The asynchronous delivery
described here is how the WebGPU spec works and is what the wasm build
gets in the browser.

The reason is, in this case, Chrome in WebGPU doesn't process certain
errors until you call certain functions. One such function is
`submit`

```rust
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            /* wgsl */ r#"
      this shader won't compile
    "#
            .into(),
        ),
    });

+    // pump WebGPU
+    queue.submit([]);

    log("--done--");
```

Now it should show the error.

{{{example url="../webgpu-debugging-help-webgpu-report-errors-fixed.html"}}}

This issue rarely comes up because if you never call `submit` then you really
aren't using WebGPU yet. But, it can come up in special situations, like
when you're trying to make a minimal complete verifiable example for a
tech support question or a bug report. Or if you're stepping through the
code and you pass a line you know is supposed to cause an error and yet
no error has appeared yet.

Note: In the browser the error will also go to the JavaScript console.
The JS API can suppress that with `event.preventDefault()`; wgpu's
`on_uncaptured_error` doesn't expose that.

## Manually catching errors.

Above we showed a message for "uncaptured errors" which implies there's
such a thing as a "captured error". To capture an error you push an
*error scope* with `device.push_error_scope`. Submit commands, then pop
the error scope to see if there were any errors between the time you
pushed and the time you popped.

Example:

```rust
  device.on_uncaptured_error(std::sync::Arc::new(|error: wgpu::Error| {
*    log(&format!("uncaptured error: {error}"));
  }));

+  let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
  device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
      this shader won't compile
    "#
      .into(),
    ),
  });
+  let error = error_scope.pop().await;
+  if let Some(error) = error {
+    log(&format!("captured error: {error}"));
+  }

+  device.create_shader_module(wgpu::ShaderModuleDescriptor {
+    label: None,
+    source: wgpu::ShaderSource::Wgsl(
+      /* wgsl */ r#"
+      also, this shader won't compile
+    "#
+      .into(),
+    ),
+  });

  queue.submit([]);

  log("--done--");
```

`device.push_error_scope` takes one of three filters
(`'validation'`, `'out-of-memory'` and `'internal'` in JS).

* `wgpu::ErrorFilter::Validation`

  Errors related to using the API incorrectly

* `wgpu::ErrorFilter::OutOfMemory`

  Errors related to trying to allocate too much memory.

* `wgpu::ErrorFilter::Internal`

  Errors where you did nothing wrong but the driver complained.
  For example, this might happen if your shader is too complex.

{{{example url="../webgpu-debugging-push-pop-error-scope.html"}}}

In wgpu, `push_error_scope` returns a guard and `guard.pop()` returns a
*future* that resolves to `Some(error)` or `None` if there was no
error — the equivalent of JS `popErrorScope`'s promise. Above we
`await` it right away, but that makes our program wait. You can also
hold on to the future and await it later:

```rust
  let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
  device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      /* wgsl */ r#"
      this shader won't compile
    "#
      .into(),
    ),
  });
+  let error_future = error_scope.pop();
+
+  // ... queue up more work here ...
+
+  if let Some(error) = error_future.await {
+    log(&format!("captured error: {error}"));
+  }
```

This way our program doesn't pause and wait for the GPU to get back
to us on whether or not there was an error. (On native the answer is
already recorded by the time you pop, so the await is instant; in the
browser it's a genuine round trip.)

## Different kinds of Errors

Some errors in WebGPU are checked when you call a function. Others are checked
later. The WebGPU spec defines timelines. Two of them are the "content
timeline" and the "device timeline". The "content timeline" is the same
timeline as JavaScript itself. The device timeline is separate and generally
runs in a separate process. Yet other errors are checked by the rules of the
language itself.

* Example of a language-level Error: Passing the wrong type

  ```rust
  queue.write_buffer(&some_texture, ...);
  ```

  In JavaScript this line would throw at runtime because the first
  argument of `writeBuffer` must be a `GPUBuffer`. In Rust it doesn't
  even compile — `write_buffer` takes a `&wgpu::Buffer` and the type
  system enforces it. A whole class of WebGPU errors becomes compile
  errors in wgpu.

* Example of a "content timeline" error

  ```js
  device.createTexture({
    size: [],
    format: 'rgba8unorm',
    usage: GPUTextureUsage.TEXTURE_BINDING,
  });
  ```

  `size` as provided above, is an error in JavaScript, it must have at
  least 1 element, and it throws an exception immediately. In wgpu
  `size` is a `wgpu::Extent3d` struct so this particular mistake can't
  be expressed; the nearest equivalent (say, a zero width) is checked
  on the device timeline like any other validation error.

* Example of a device error

  The examples at the start of the page are device errors. Device
  errors are what `push_error_scope`, `pop`, and the uncaptured
  error handler process.

Where errors happen is detailed in [the spec](https://www.w3.org/TR/webgpu/)
but it's important to know that language and content timeline errors happen
immediately (an exception in JS, a compile error or panic in Rust) whereas
device timeline errors happen asynchronously — except that native wgpu,
as noted above, reports them at the call site.

## WGSL errors

If you get an error compiling a shader module you can ask for more
detailed info by calling `get_compilation_info`.

Example:

```rust
  let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
  let code = r#"
      // This function
      // calls a function
      // that does not
      // exist.

      fn foo() -> vec3f {
        return someFunction(1, 2);
      }
    "#;
  let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(code.into()),
  });
  let error = error_scope.pop().await;
  if error.is_some() {
    let info = module.get_compilation_info().await;

    // Split the code into lines
    let mut lines: Vec<String> = code.split('\n').map(String::from).collect();

    // Sort the messages by line numbers in reverse order
    // so that as we insert the messages they won't affect
    // the line numbers.
    let mut msgs: Vec<_> = info
      .messages
      .iter()
      .filter_map(|msg| msg.location.map(|loc| (loc, msg.message.clone())))
      .collect();
    msgs.sort_by(|a, b| b.0.line_number.cmp(&a.0.line_number));

    // Insert the error messages between lines
    for (loc, message) in msgs {
      lines.insert(loc.line_number as usize, message);
      lines.insert(
        loc.line_number as usize,
        format!(
          "{}{}",
          " ".repeat(loc.line_position as usize - 1),
          "^".repeat(loc.length as usize),
        ),
      );
    }

    log(&lines.join("\n"));
  }
```

The code above effectively interleaves any error messages
into the full shader code.

{{{example url="../webgpu-debugging-get-compilation-info.html"}}}

`get_compilation_info` returns a `wgpu::CompilationInfo` that contains a
`Vec` of `wgpu::CompilationMessage`s, each of which has the following
fields

* `message`: a string error message
* `message_type`: `Error` or `Warning` or `Info`
* `location`: an `Option<wgpu::SourceLocation>` with
  * `line_number`: the line number of the error, 1 based
  * `line_position`: the position in the line of the error, 1 based
  * `offset`: the position in the string of the error, 0 based.
    (this is effectively the same info as line_position, line_number)
  * `length`: the length to highlight

One difference from JavaScript: the WebGPU spec counts these in UTF-16
code units; wgpu counts in UTF-8 bytes. For ASCII shaders they're the
same.

Note that natively, wgpu's shader errors already come nicely
formatted — naga (wgpu's shader compiler) prints the offending source
line with a `^^^` marker and the message, so you often get this
interleaved view for free in your terminal.

## <a id="webgpu-dev-extension"></a> WebGPU-Dev-Extension

*(browser only)* The [WebGPU-Dev-Extension](https://github.com/greggman/webgpu-dev-extension)
provides features to help debug. It hooks the browser's JS WebGPU API,
which is exactly what our wasm builds call, so it works with the Rust
examples running in the browser too.

Some things it can do

* Show a stack trace where errors happened.

  As we showed above, errors in WebGPU happen asynchronously. In the
  first example we used the uncaptured error handler to see that we
  got a WebGPU error but there was no info about where in the code
  that error happened.

  The webgpu-dev-extension provides this info by trying to add calls
  to `pushErrorScope` and `popErrorScope` around all of the WebGPU
  functions that generate errors. Inside it creates an `Error` object
  which holds a stack trace. If it gets an error it can then print
  that `Error` object and you'll see the error stack of where the
  error was originally generated.

  (Natively you get this for free: the uncaptured-error panic happens
  at the erroring call, so `RUST_BACKTRACE=1` shows the stack.)

* Show errors for command encoders

  In WebGPU, command encoders, like `GPUCommandEncoder`, `GPURenderPassEncoder`,
  `GPUComputePassEncoder`, and `GPURenderBundleEncoder` do
  not generate device timeline errors. Instead, the errors
  are saved up until you call `encoder.finish`. The same is true in
  wgpu.

  For example:

  ```rust
  let mut encoder = device.create_command_encoder(&Default::default());
  {
    let mut pass = encoder.begin_render_pass(&render_pass_desc);
    pass.set_pipeline(&some_pipeline);
    pass.set_bind_group(0, &some_bind_group_incompatible_with_some_pipeline, &[]); // oops!
    pass.set_vertex_buffer(0, position_buffer.slice(..));
    pass.set_vertex_buffer(1, normal_buffer.slice(..));
    pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..4, 0, 0..1);
  }
  let cb = encoder.finish();  // Error above is generated here
  ```

  The problem here is, at best you'll get an error message
  that the bind group bound to group 0 is incompatible with
  the pipeline but you won't know which line the error happened on.
  In a small example like this it should be pretty obvious but in
  a large app, it might be hard to track down which specific line
  caused the error.

  The webgpu-dev-extension can try to throw an error at the line
  that caused the error.

* Show WGSL errors interleaved with the full shader source

  Like the example above, the webgpu-dev-extension has an option
  to show the errors interleaved with the source WGSL, rather than
  just a terse error message. (the default)

* <a id="check-for-multiple-updates"></a> Check for Multiple Updates

  Checks if you updated a buffer or texture more than once per submit.
  [See below](#multiple-updates) for what that means.

## WebGPU-Inspector

*(browser only)* [The WebGPU-Inspector](https://github.com/brendan-duncan/webgpu_inspector)
will attempt to capture all of your WebGPU commands and can let you
inspect buffers, textures, calls, and generally try to see what's
happening in your WebGPU code.

<div class="webgpu_center"><img src="resources/images/frame_capture_commands.jpg"style="width: 1200px;"></div>

## Native tools

Running natively you get to use the regular GPU debugging ecosystem:

* **[RenderDoc](https://renderdoc.org/)** — launch your example from
  RenderDoc (or attach) and capture a frame. You can inspect every
  draw call, buffer, texture, and pipeline state. The `label:` strings
  the lessons put on every object show up here, which is one more
  reason to always fill them in.

* **Xcode's Metal frame capture** (macOS) and **PIX** (Windows/D3D12)
  do the same for wgpu's Metal and D3D12 backends.

* **Vulkan validation layers** — in debug builds wgpu enables the
  backend's validation/debug layers when they're installed (on Vulkan,
  `VK_LAYER_KHRONOS_validation`). Their complaints are forwarded
  through the `log` crate, so run with `RUST_LOG=warn` (see above) to
  see them. They catch a class of driver-level mistakes wgpu's own
  validation doesn't.

* **`RUST_LOG` + `RUST_BACKTRACE`** — worth repeating: `RUST_LOG=warn`
  makes wgpu's warnings visible and `RUST_BACKTRACE=1` turns the
  panic on an uncaptured error into an exact source location.

## Tips for debugging shaders

### Simplify:

Get your shader to a working state by cutting out as much as possible.
Once it's working, add stuff back in little by little

### Show a solid color

For render passes, the first thing I often do is show a solid color.

Here is last shader from [the article on spot lights](webgpu-lighitng-spot.html).

```wgsl
@fragment fn fs(vsOut: VSOutput) -> @location(0) vec4f {
  // Because vsOut.normal is an inter-stage variable 
  // it's interpolated so it will not be a unit vector.
  // Normalizing it will make it a unit vector again
  let normal = normalize(vsOut.normal);

  let surfaceToLightDirection = normalize(vsOut.surfaceToLight);
  let surfaceToViewDirection = normalize(vsOut.surfaceToView);
  let halfVector = normalize(
    surfaceToLightDirection + surfaceToViewDirection);

  let dotFromDirection = dot(surfaceToLightDirection, -uni.lightDirection);
  let inLight = smoothstep(uni.outerLimit, uni.innerLimit, dotFromDirection);

  // Compute the light by taking the dot product
  // of the normal with the direction to the light
  let light = inLight * dot(normal, surfaceToLightDirection);

  var specular = dot(normal, halfVector);
  specular = inLight * select(
      0.0,                           // value if condition false
      pow(specular, uni.shininess),  // value if condition is true
      specular > 0.0);               // condition

  // Lets multiply just the color portion (not the alpha)
  // by the light
  let color = uni.color.rgb * light + specular;
  return vec4f(color, uni.color.a);
}
```

The example is supposed to render a green F with a small portion lit by a
spotlight. Here's a version with a bug. Let's debug it.

{{{example url="../webgpu-debugging-spot-light-01.html"}}}

We ran it and nothing appeared on the screen and there were
no WebGPU errors. The first thing I might do is change it to return solid red

```wgsl
  let color = uni.color.rgb * light + specular;
-  return vec4f(color, uni.color.a);
+  //return vec4f(color, uni.color.a);
+  return vec4f(1, 0, 0, 1);  // solid red
```

If I see a red F then I know I should start looking in the fragment shader since
clearly enough of the vertex shader was correct to draw the triangles that make the F.
If I don't see a red F then I should start looking in the vertex shader.

Trying it:

{{{example url="../webgpu-debugging-spot-light-02.html"}}}

We see a red F. Ok, lets try to visualize the normals.
To do so, change the end of the fragment shader to:

```wgsl
  let color = uni.color.rgb * light + specular;
  //return vec4f(color, uni.color.a);
-   return vec4f(1, 0, 0, 1);  // solid red
+   //return vec4f(1, 0, 0, 1);  // solid red
+   return vec4f(vsOut.normal * 0.5 + 0.5, 1);  // normal
```

Normals go from -1.0 to +1.0 but colors go from 0.0 to 1.0 so by multiplying
by 0.5 and adding 0.5 we convert the normals to something that can be visualized
with colors.

Trying that:

{{{example url="../webgpu-debugging-spot-light-03.html"}}}

Hmmm, that's not right. That looks suspiciously like all the normals are 0,0,0.
Clearly something is wrong the normals in the fragment shader. Those normals
come from the vertex shader after being multiplied by `normalMatrix`. Let's try
passing the normals straight through, without multiplying by `normalMatrix`. If
the F appears then we know the bug is in `normalMatrix`. If the F doesn't appear
then the bug in the data being supplied to the vertex shader.

```wgsl
  // Orient the normals and pass to the fragment shader
-  vsOut.normal = uni.normalMatrix * vert.normal;
+  //vsOut.normal = uni.normalMatrix * vert.normal;
+  vsOut.normal = vert.normal;
```

Running that:

{{{example url="../webgpu-debugging-spot-light-04.html"}}}

That looks more like it. So apparently something is wrong with
`normalMatrix`

Checking the code it was commented out which left the matrix all zeros. 
Someone must have checking something and forgot to uncomment it.😅

```rust
    // Inverse and transpose it into the worldInverseTranspose value
-    //mat3::from_mat4(
-    //    &m4::transpose(&m4::inverse(&world)),
-    //    &mut uniform_values[K_NORMAL_MATRIX_OFFSET..K_NORMAL_MATRIX_OFFSET + 12],
-    //);
+    mat3::from_mat4(
+        &m4::transpose(&m4::inverse(&world)),
+        &mut uniform_values[K_NORMAL_MATRIX_OFFSET..K_NORMAL_MATRIX_OFFSET + 12],
+    );
```

Let's un-comment it. Then let's put the vertex shader back the way it was

```wgsl
  // Orient the normals and pass to the fragment shader
-  //vsOut.normal = uni.normalMatrix * vert.normal;
-  vsOut.normal = vert.normal;
+  vsOut.normal = uni.normalMatrix * vert.normal;
```

That gives us:

{{{example url="../webgpu-debugging-spot-light-05.html"}}}

If you rotate the F you'll see the colors change showing the normals
are being re-oriented by `normalMatrix`. Compare that to the one above
where the colors don't change as we rotate.

With that we can finally restore the fragment shader.

```wgsl
  let color = uni.color.rgb * light + specular;
-  //return vec4f(color, uni.color.a);
-  //return vec4f(1, 0, 0, 1);  // solid red
-  return vec4f(vsOut.normal * 0.5 + 0.5, 1);  // normal
+  return vec4f(color, uni.color.a);
```

And it's working as it's supposed to.

{{{example url="../webgpu-debugging-spot-light-06.html"}}}

Finding ways to visualize your data is a good way to check it.
For example, to check [texture coordinates](webpgu-textures.html)
you might do something like

```wgsl
   return vec4f(fract(textureCoord), 0, 1);
```

Texture coordinates generally go from 0.0 to 1.0 but if you're repeating
the texture they might go higher so `fract` covers that. 

To give an idea of what texture coordinates look like, here's a few objects with their texture coordinates visualized.

<div class="webgpu_center">
   <div data-diagram="texcoords" style="width: 1024px; height: 400px;"></div>
   <div class="caption">texture coordinates visualized</div>
</div>

Texture coordinates are generally smooth over some surface.

Here are the same texture coordinates visualized with a bug.

<div class="webgpu_center">
   <div data-diagram="texcoords-bad"  style="width: 1024px; height: 400px;"></div>
   <div class="caption">bad texture coordinates</div>
</div>

They are no longer smooth so something is probably off.

Following the same procedures as above we'd conclude that the data coming into
the vertex shader must be bad. And indeed, this example is uploading the
vertex data as `f32` values, three per position (`VertexFormat::Float32x3`),
but mistakenly specified them as `VertexFormat::Float16x2` in the render
pipeline descriptor.

## Other common issues

### <a id="multiple-updates"></a> Remembering that command buffers don't execute until you submit them

It's common to run into a version of this problem (this gotcha is the same
in wgpu)

```rust
  let mut encoder = device.create_command_encoder(&Default::default());
  let mut pass = encoder.begin_render_pass(&render_pass_desc);
  pass.set_pipeline(&some_pipeline);
  pass.set_bind_group(0, &some_bind_group_that_uses_some_buffer, &[]);
  queue.write_buffer(&some_buffer, 0, data0_for_buffer);
  pass.draw(0..num_vertices, 0..1);
  queue.write_buffer(&some_buffer, 0, data1_for_buffer); // ERROR!?
  pass.draw(0..num_vertices, 0..1);
  drop(pass);
  queue.submit([encoder.finish()]);
```

The code above is updating `some_buffer` by calling `queue.write_buffer`. That function
executes immediately where as the `draw` function does not execute, it just adds
a command to a command buffer. That command buffer gets executed later.

Usually this type of situation is not as obvious as the code above. It's more common
to call some various functions and have one update a buffer that's already being used
earlier.

One way to find this kind of bug is to use
[the WebGPU-Inspector](https://github.com/brendan-duncan/webgpu_inspector)
which either defaults to, or has an option, to order the commands in
the order they will be executed rather then the order they were created
in your code. This can be helpful to see what your code is actually doing
vs what you thought it was doing.

Another is to use the [WebGPU-Dev-Extension](#check-for-multiple-updates)

### Remembering that Canvas Textures are only valid until the next event

You call `someWebGPUContext.getCurrentTexture` to get the texture for a canvas.
That texture only exists for the current event. So for example, code like this
will fail

```js
  const texture = context.getCurrentTexture();

  // This ends the current event and waits for another
  await someAsyncFunction();

  // ERROR: The canvas texture no longer exist
  encoder.beginRenderPass({
    colorTarget: { view: texture, ...},  
  });
  ...
```

You need to call `getCurrentTexture` at the last possible moment and then apply
all uses without `async` or any other events or callbacks.

### External textures expire at the next event

`importExternalTexture` imports a texture from a video. Like the canvas texture,
the external texture is only valid until the end of the current event.

### Waiting for images to load

This issue is not unique to WebGPU but images (and videos) load asynchronously
so if you're going to pass them to WebGPU you need to make sure they're loaded.

```js
   const img = new Image();
   img.src = 'someUrl';
   device.write.copyExternalImageToTexture({ source: img }, ...); // ERROR!
```

The code above will fail because `img` has not loaded yet.

How you wait is up to you. One example:

```js
   const img = new Image();
   img.src = 'someUrl';
   await img.decode();
   device.write.copyExternalImageToTexture({ source: img }, ...);
```

<!-- keep this at the bottom of the article -->
<link href="webgpu-debugging.css" rel="stylesheet">
<script type="module" src="webgpu-debugging.js"></script>
