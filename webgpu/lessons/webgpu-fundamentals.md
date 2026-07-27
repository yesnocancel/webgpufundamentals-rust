Title: WebGPU Fundamentals
Description: The fundamentals of WebGPU with Rust and wgpu
TOC: Fundamentals

This article will try to teach you the very fundamentals of WebGPU, using
[Rust](https://www.rust-lang.org/) and [wgpu](https://wgpu.rs/).

<div class="warn">
It is expected you already know Rust before you read this article. Concepts
like
<a href="https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html">ownership and borrowing</a>,
<a href="https://doc.rust-lang.org/book/ch06-00-enums.html">enums and pattern matching</a>,
<a href="https://doc.rust-lang.org/book/ch09-00-error-handling.html">Result and error handling</a>,
<a href="https://doc.rust-lang.org/book/ch13-00-functional-features.html">closures</a>,
and <a href="https://rust-lang.github.io/async-book/">async/await</a>
will be used extensively. If you don't already know Rust and would like to
learn it, see <a href="https://doc.rust-lang.org/book/">The Rust Book</a>,
<a href="https://doc.rust-lang.org/rust-by-example/">Rust by Example</a>,
and/or <a href="https://rustlings.rust-lang.org/">Rustlings</a>.
</div>

<div class="warn">If you already know WebGL, <a href="webgpu-from-webgl.html">read this</a>.</div>

## What is wgpu?

[wgpu](https://github.com/gfx-rs/wgpu) is a Rust implementation of the WebGPU
API. The same Rust code runs

* **natively**, on Vulkan, Metal, DirectX 12, or OpenGL ES — you get a normal
  desktop program you run with `cargo run`, and

* **in the browser**, compiled to WebAssembly, where it maps directly onto the
  browser's WebGPU API.

The examples on this site are compiled to WebAssembly so they run live in the
page, and every one of them can also be run natively from the repository with
`cargo run --bin <example-name>`.

WebGPU is an API that lets you do 2 basic things.

1. [Draw triangles/points/lines to textures](#a-drawing-triangles-to-textures)

2. [Run computations on the GPU](#a-run-computations-on-the-gpu)

That is it!

Everything about WebGPU after that is up to you. It's like learning a computer
language like JavaScript, or Rust, or C++. First you learn the basics, then
it's up to you to creatively use those basics to solve your problem.

WebGPU is an extremely low-level API. While you can make some small examples,
for many apps it will likely require a large amount of code and some serious
organization of data. As an example, [three.js](https://threejs.org), a
JavaScript library that supports WebGPU, consists of ~550k bytes of minified
JavaScript for just its base library, and in the Rust world a game engine like
[Bevy](https://bevyengine.org/), which renders through wgpu, is hundreds of
thousands of lines of code. That does not include loaders, controls,
post-processing, and many other features.

The point being, if you just want to get something on the screen you're far
better off choosing a library or engine that provides the large amount of code
you're going to have to write when doing it yourself.

On the other hand, maybe you have a custom use case or maybe you want to modify
an existing library or maybe you're just curious how it all works. In those
cases, read on!

# Getting Started

It's hard to decide where to start. At a certain level, WebGPU is a very simple
system. All it does is run 3 types of functions on the GPU: Vertex Shaders,
Fragment Shaders, and Compute Shaders.

A Vertex Shader computes vertices. The shader returns vertex positions. For every group of 3 vertices the vertex shader function returns, a triangle is drawn between those 3 positions.[^primitives]

[^primitives]: There are actually 5 modes.

    * `PointList`: for each position, draw a point
    * `LineList`: for each 2 positions, draw a line
    * `LineStrip`: draw lines connecting the newest point to the previous point
    * `TriangleList`: for each 3 positions, draw a triangle (**default**)
    * `TriangleStrip`: for each new position, draw a triangle from it and the last 2 positions

A Fragment Shader computes colors.[^fragment-output] When a triangle is drawn, for each pixel
to be drawn the GPU calls your fragment shader. The fragment shader then returns a
color.

[^fragment-output]: Fragment shaders indirectly write data to textures. That data does not
have to be colors. For example, it's common to output the direction of the surface that
pixel represents.

A Compute Shader is more generic. It's effectively just a function you call and
say "execute this function N times". The GPU passes the iteration number each
time it calls your function so you can use that number to do something unique on
each iteration.

If you squint hard, you can think of these functions as similar to the closures
you pass to
[`iter().for_each`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.for_each)
or
[`iter().map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map).
The functions you run on the GPU are just functions, like Rust functions. The
part that differs is they run on the GPU, and so to run them you need to copy
all the data you want them to access to the GPU in the form of buffers and
textures and they only output to those buffers and textures.
You need to specify in the functions which bindings or locations the function
will look for the data. And, back in Rust, you need to bind the buffers and
textures holding your data to the bindings or locations. Once you've done that you tell the GPU to execute the
function.

<a id="a-draw-diagram"></a>Maybe a picture will help. Here is a *simplified* diagram of WebGPU setup to draw triangles
by using a vertex shader and a fragment shader:

<div class="webgpu_center"><img src="resources/webgpu-draw-diagram.svg" style="width: 960px;"></div>

What to notice about this diagram:

* There is a **Pipeline**. It contains the vertex shader and fragment shader the
  GPU will run. You could also have a pipeline with a compute shader.

* The shaders reference resources (buffers, textures, samplers) indirectly
  through **Bind Groups**

* The pipeline defines attributes that reference buffers indirectly through the
  internal state

* Attributes pull data out of buffers and feed the data into the vertex shader

* The vertex shader may feed data into the fragment shader

* The fragment shader writes to textures indirectly through the render pass
  description

To execute shaders on the GPU, you need to create all of these resources and
set up this state. Creation of resources is relatively straightforward. One
interesting thing is that most WebGPU resources can not be changed after creation. You
can change their contents but not their size, usage, format, etc... If you want
to change any of that stuff you create a new resource and destroy the old one.

Some of the state is set up by creating and then executing command buffers.
Command buffers are literally what their name suggests. They are a buffer of
commands. You create command encoders. The encoders encode commands into the command
buffer. You then *finish* the encoder and it gives you the command buffer it
created. You can then *submit* that command buffer to have WebGPU execute the
commands.

Here is some pseudo-code for encoding a command buffer followed by a representation of
the command buffer that was created.

<div class="webgpu_center side-by-side"><div style="min-width: 300px; max-width: 400px; flex: 1 1;"><pre class="prettyprint lang-rust"><code>{{#escapehtml}}
encoder = device.create_command_encoder(...)
// draw something
{
  pass = encoder.begin_render_pass(...)
  pass.set_pipeline(...)
  pass.set_vertex_buffer(0, …)
  pass.set_vertex_buffer(1, …)
  pass.set_index_buffer(...)
  pass.set_bind_group(0, …)
  pass.set_bind_group(1, …)
  pass.draw(...)
  drop(pass)
}
// draw something else
{
  pass = encoder.begin_render_pass(...)
  pass.set_pipeline(...)
  pass.set_vertex_buffer(0, …)
  pass.set_bind_group(0, …)
  pass.draw(...)
  drop(pass)
}
// compute something
{
  pass = encoder.begin_compute_pass(...)
  pass.set_bind_group(0, …)
  pass.set_pipeline(...)
  pass.dispatch_workgroups(...)
  drop(pass)
}
command_buffer = encoder.finish()
{{/escapehtml}}</code></pre></div>
<div><img src="resources/webgpu-command-buffer.svg" style="width: 300px;"></div>
</div>

One Rust-specific detail: in the JavaScript WebGPU API you call `pass.end()`
to end a pass. In wgpu, a pass ends when the pass encoder is *dropped*, so
you'll usually see passes inside a `{ }` block (or an explicit `drop(pass)`)
so the borrow of the encoder ends and the pass is recorded.

Once you create a command buffer, you can *submit* it to be executed:

```rust
device.queue.submit([command_buffer]);
```

The 'simplified diagram of WebGPU setup' shown previously represents the state at a *single* `draw` command in the command
buffer. Executing the commands will set up the *internal state* and then the
`draw` command will tell the GPU to execute a vertex shader (and indirectly a
fragment shader). The `dispatch_workgroups` command will tell the GPU to execute a
compute shader.

I hope that gave you some mental image of the state you need to set up. Like
mentioned above, WebGPU has 2 basic things it can do:

1. [Draw triangles/points/lines to textures](#a-drawing-triangles-to-textures)

2. [Run computations on the GPU](#a-run-computations-on-the-gpu)

We'll go over a small example of doing each of those things. Other
articles will show the various ways of providing data to these things. Note that
this will be very basic. We need to build up a foundation of these basics. Later
we'll show how to use them to do things people typically do with GPUs like 2D
graphics, 3D graphics, etc...

# <a id="a-drawing-triangles-to-textures"></a>Drawing triangles to textures

WebGPU can draw triangles to [textures](webgpu-textures.html). For the purpose
of this article, a texture is a 2D rectangle of pixels.[^textures] When our
code runs in the browser, the `<canvas>` element provides a texture we can
render to. When it runs natively, a window does the same job. Either way we
ask the *surface* for a texture and then render to that texture.

[^textures]: Textures can also be 3D rectangles of pixels, cube maps (6 squares of pixels
that form a cube), and a few other things but the most common textures are 2D rectangles of pixels.

To draw triangles with WebGPU we have to supply 2 "shaders". Again, Shaders
are functions that run on the GPU. These 2 shaders are:

1. Vertex Shaders

   Vertex shaders are functions that compute vertex positions for drawing
   triangles/lines/points

2. Fragment Shaders

   Fragment shaders are functions that compute the color (or other data)
   for each pixel to be drawn/rasterized when drawing triangles/lines/points

Let's start with a very small program to draw a triangle.

## Setting up the project

We need a Rust project with the `wgpu` crate:

```sh
cargo new my-triangle
cd my-triangle
cargo add wgpu
```

Unlike JavaScript in a webpage, a Rust program doesn't get a canvas and an
event loop for free. Opening a window, reacting to resize events, and driving
a render loop (or, in the browser, attaching to a `<canvas>` and hooking up
`requestAnimationFrame`) is ordinary Rust GUI plumbing — it has nothing to do
with WebGPU, but it's 100+ lines of code that every graphical example needs.

So the examples on this site share one tiny helper crate,
[`wgpu_fun`](https://github.com/yesnocancel/webgpufundamentals-rust/tree/main/rust/wgpu_fun),
that does exactly four jobs and nothing else:

* opens a window (native, via [winit](https://crates.io/crates/winit)) or
  attaches to the page's `<canvas>` (browser)
* requests the `Adapter` and `Device` (we'll see this code below — it's the
  same code you'd write yourself)
* configures the surface and keeps it sized to the window/canvas
* calls your frame function when it's time to draw

Everything WebGPU-related stays in the examples. The skeleton of every
example on this site looks like this:

```rust
use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
  let app = App::new("hello triangle").await;

  // ... create WebGPU resources with app.device ...

  app.run(RenderMode::Once, move |frame: &Frame| {
    // ... encode and submit commands to draw one frame ...
  });
}

fn main() {
  wgpu_fun::start(run());
}
```

`wgpu_fun::start` runs our async `run` function: natively it blocks on it
([`pollster::block_on`](https://crates.io/crates/pollster)); in the browser it
spawns it on the JavaScript event loop
([`wasm_bindgen_futures::spawn_local`](https://crates.io/crates/wasm-bindgen-futures)).
WebGPU is an asynchronous API, so it's easiest to use in an async function.

What does `App::new` do? The WebGPU part of it is exactly this — we request an
adapter, and then request a device from the adapter:

```rust
let instance = wgpu::Instance::default();
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions::default())
    .await
    .expect("this system does not support WebGPU");
let (device, queue) = adapter
    .request_device(&wgpu::DeviceDescriptor::default())
    .await
    .expect("failed to create a device");
```

The `Instance` is the root of the API. The adapter represents a specific GPU —
some devices have multiple GPUs. The device is what we create every other
resource from, and the queue is where we'll submit our commands. Both
`request_adapter` and `request_device` return a `Result` and give us their
results asynchronously, so we `await` them. If requesting the adapter fails,
the system (or browser) doesn't support WebGPU; the examples show a "need a
browser that supports WebGPU" style message in that case.

`App::new` also creates a *surface* for the window or canvas and configures
it, which is where the texture we render to will come from:

```rust
let surface = instance.create_surface(window_or_canvas).expect("no surface");
let format = surface.get_capabilities(&adapter).formats[0];
surface.configure(&device, &wgpu::SurfaceConfiguration {
    format,
    width,
    height,
    ..surface_defaults()
});
```

The surface's preferred texture format will be either `Rgba8Unorm` or
`Bgra8Unorm`. It's not really that important what it is, but using the
preferred one makes things faster. The `App` exposes it as `app.format`, and
exposes the device and queue as `app.device` and `app.queue`.

## Drawing the triangle

Next, we create a shader module. A shader module contains one or more shader
functions. In our case, we'll make 1 vertex shader function and 1 fragment shader
function.

```rust
  let module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("our hardcoded red triangle shaders"),
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );

        return vec4f(pos[vertexIndex], 0.0, 1.0);
      }

      @fragment fn fs() -> @location(0) vec4f {
        return vec4f(1.0, 0.0, 0.0, 1.0);
      }
    "#.into()),
  });
```

Shaders are written in a language called
[WebGPU Shading Language (WGSL)](https://gpuweb.github.io/gpuweb/wgsl/) which is
often pronounced wig-sil. WGSL is a strongly typed language
which we'll try to go over in more detail in [another article](webgpu-wgsl.html).
For now, I'm hoping with a little explanation you can infer some basics.

Notice the WGSL is exactly the same whether you use WebGPU from JavaScript,
Rust, C++, or anything else. And notice it's a separate language from Rust: it
lives in a Rust *string* (here a
[raw string literal](https://doc.rust-lang.org/reference/tokens.html#raw-string-literals),
`r#"..."#`, so we don't have to escape anything). Yes, its function
declarations look confusingly close to Rust's — `fn`, `let`, arrow return
types — but it is its own language with its own rules.

> Note: throughout this site, strings that store WGSL have `/* wgsl */` as a
> comment in front of them. This is a convention to help text editors try to
> syntax highlight and/or provide intellisense for WGSL.

Above we see a function called `vs` is declared with the `@vertex` attribute.
This designates it as a vertex shader function.

```wgsl
      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> @builtin(position) vec4f {
         ...
```

It accepts one parameter we named `vertexIndex`. `vertexIndex` is a `u32` which
means a *32-bit unsigned integer*. It gets its value from the builtin called
`vertex_index`. `vertex_index` is like an iteration number, similar to the
index in Rust's `iter().enumerate()`. If we tell the GPU to
execute this function 10 times by calling `draw`, the first time `vertex_index` would be `0`, the
2nd time it would be `1`, the 3rd time it would be `2`, etc...[^indices]

[^indices]: We can also use an index buffer to specify `vertex_index`.
This is covered in [the article on vertex-buffers](webgpu-vertex-buffers.html#a-index-buffers).

Our `vs` function is declared as returning a `vec4f` which is a vector of four
32-bit floating point values. Think of it as an array of 4 values or a struct
with 4 fields like `{x: 0.0, y: 0.0, z: 0.0, w: 0.0}`. This returned value will be
assigned to the `position` builtin. In "triangle-list" mode, every 3 times the
vertex shader is executed a triangle will be drawn connecting the 3 `position`
values we return.

Positions in WebGPU need to be returned in *clip space* where X goes from -1.0
on the left to +1.0 on the right, and Y goes from -1.0 at the bottom to +1.0 at the
top. This is true regardless of the size of the texture we are drawing to.

<div class="webgpu_center"><img src="resources/clipspace.svg" style="width: 500px"></div>

The `vs` function declares an array of 3 `vec2f`s. Each `vec2f` consists of two
32-bit floating point values.

```wgsl
        let pos = array(
          vec2f( 0.0,  0.5),  // top center
          vec2f(-0.5, -0.5),  // bottom left
          vec2f( 0.5, -0.5)   // bottom right
        );
```

Finally it uses `vertexIndex` to return one of the 3 values from the array.
Since the function requires 4 floating point values for its return type, and
since `pos` is an array of `vec2f`, the code supplies `0.0` and `1.0` for
the remaining 2 values.

```wgsl
        return vec4f(pos[vertexIndex], 0.0, 1.0);
```

Note that for drawing something in 2D we usually only need the x and y values
for position. The z value is used for depth testing and will come up in
[the article on orthographic projection](webgpu-orthographic-projection.html).
The w value is used for perspective divide and will come up in
[the article on perspective projection](webgpu-perspective-projection.html).
For now, setting z to 0.0 and w to 1.0 is what we need to draw
the triangle.

The shader module also declares a function called `fs` that is declared with
`@fragment` attribute making it a fragment shader function.

```wgsl
      @fragment fn fs() -> @location(0) vec4f {
```

This function takes no parameters and returns a `vec4f` at `location(0)`.
This means it will write to the first render target. We'll make the first
render target our canvas texture later.

```wgsl
        return vec4f(1, 0, 0, 1);
```

The code returns `1, 0, 0, 1` which is red. Colors in WebGPU are usually
specified as floating point values from `0.0` to `1.0` where the 4 values above
correspond to red, green, blue, and alpha respectively.

One more thing to note is the `label`. Nearly every object you can create with
WebGPU can take a label — in wgpu it's an `Option<&str>`, so we pass
`Some("...")`. Labels are entirely optional but it's considered
*best practice* to label everything you make. The reason is that when you get an
error, most WebGPU implementations will print an error message that includes the
labels of the things related to the error.

In a normal app, you'd have 100s or 1000s of buffers, textures, shader modules,
pipelines, etc... If you get an error like `"WGSL syntax error in shaderModule
at line 10"`, if you have 100 shader modules, which one got the error? If you
label the module then you'll get an error more like `"WGSL syntax error in
shaderModule('our hardcoded red triangle shaders') at line 10` which is a way
more useful error message and will save you a ton of time tracking down the
issue.

Now that we've created a shader module, we next need to make a render pipeline:

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("our hardcoded red triangle pipeline"),
    layout: None,
    vertex: wgpu::VertexState {
      entry_point: Some("vs"),
      module: &module,
      compilation_options: Default::default(),
      buffers: &[],
    },
    fragment: Some(wgpu::FragmentState {
      entry_point: Some("fs"),
      module: &module,
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

In this case, there isn't much to see. We set `layout` to `None` which means
to ask WebGPU to derive the layout of data from the shaders (the equivalent of
`layout: 'auto'` in the JavaScript API). We're not using any data though.

We then tell the render pipeline to use the `vs` function from our shader module
for a vertex shader and the `fs` function for our fragment shader. Otherwise, we
tell it the format of the first render target. "render target" means the texture
we will render to. When we create a pipeline
we have to specify the format for the texture(s) we'll use this pipeline to
eventually render to. `app.format.into()` turns a bare texture format into a
full `ColorTargetState` with default settings, which is what the `targets`
array holds.

Element 0 for the `targets` array corresponds to location 0 as we specified for
the fragment shader's return value.

The descriptor structs have quite a few fields we don't care about yet —
`primitive`, `depth_stencil`, `multisample` and friends. Rust requires every
struct field to be filled in, so we set them with `Default::default()` or
`None`. Each will get its own article later. This is one place where the
JavaScript API looks shorter — unspecified dictionary members just get default
values — but the same defaults are there in wgpu, we just ask for them
explicitly.

One shortcut: for each shader stage, `vertex` and `fragment`, if there is only
one function of the corresponding type then we can pass `None` for
`entry_point` and WebGPU will use the sole function that matches the shader
stage. So we can shorten the code above to:

```rust
  let pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("our hardcoded red triangle pipeline"),
    layout: None,
    vertex: wgpu::VertexState {
-      entry_point: Some("vs"),
+      entry_point: None,
      module: &module,
      compilation_options: Default::default(),
      buffers: &[],
    },
    fragment: Some(wgpu::FragmentState {
-      entry_point: Some("fs"),
+      entry_point: None,
      module: &module,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
    ...
```

Now it's time to render.

```rust
  app.run(RenderMode::Once, move |frame: &Frame| {
    // make a command encoder to start encoding commands
    let mut encoder = frame.device.create_command_encoder(
      &wgpu::CommandEncoderDescriptor { label: Some("our encoder") });

    // make a render pass encoder to encode render specific commands
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          // the texture view to render to
          view: frame.view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });
      pass.set_pipeline(&pipeline);
      pass.draw(0..3, 0..1);  // call our vertex shader 3 times
    }

    let command_buffer = encoder.finish();
    frame.queue.submit([command_buffer]);
  });
```

The closure we pass to `app.run` is called whenever it's time to draw a frame.
`RenderMode::Once` means "draw when needed" — once at the start, and again if
the window or canvas is resized. (The other option, `RenderMode::Continuous`,
draws every frame; we'll use it when we get to animation.) The `frame`
argument gives us `frame.view`, a *view* into the texture we're going to
render to — the helper gets the surface's current texture for us, which in the
browser is the canvas's texture.

The render pass descriptor has an array for `color_attachments` which lists
the textures we will render to and how to treat them. Element 0 of
the `color_attachments` array corresponds to `@location(0)` as we specified for
the return value of the fragment shader. We set up a clear value of semi-dark
gray, and a `load` op and `store` op. `LoadOp::Clear(...)` specifies to clear
the texture to the clear value before drawing. The other option is
`LoadOp::Load` which means load the existing contents of the texture into the
GPU so we can draw over what's already there.
`StoreOp::Store` means store the result of what we draw. We could also pass
`StoreOp::Discard` which would throw away what we draw. We'll cover why we
might want to do that in [another article](webgpu-multisampling.html).

We create a command encoder. A command encoder is used to create a command
buffer. We use it to encode commands and then "submit" the command buffer it
created to have the commands executed.

We then use the command encoder to create a render pass encoder by calling
`begin_render_pass`. A render pass encoder is a specific encoder for creating
commands related to rendering. We pass it our render pass descriptor to tell
it which texture we want to render to.

We encode the command, `set_pipeline`, to set our pipeline and then tell it to
execute our vertex shader 3 times by calling `draw` with the range `0..3`. By
default, every 3 times our vertex shader is executed a triangle will be drawn
by connecting the 3 values just returned from the vertex shader. The second
argument, `0..1`, is the range of *instances* to draw — one instance for now;
instancing comes up in [the article on storage buffers](webgpu-storage-buffers.html).

The render pass ends when `pass` is dropped at the end of the `{ }` block
(that's wgpu's version of JavaScript's `pass.end()`). Ending the pass also
ends its borrow of the encoder so we can call `encoder.finish()`. This gives
us a command buffer that represents the steps we just specified. Finally, we
submit the command buffer to be executed.

When the `draw` command is executed, this will be our state.

<div class="webgpu_center"><img src="resources/webgpu-simple-triangle-diagram.svg" style="width: 723px;"></div>

We've got no textures, no buffers, no bindGroups but we do have a pipeline, a
vertex and fragment shader, and a render pass descriptor that tells our shader
to render to the canvas texture.

The result.

{{{example url="../webgpu-simple-triangle.html"}}}

Remember, this example — like every example on this site — is this same Rust
code compiled to WebAssembly, running against your browser's WebGPU. You can
also run it natively with `cargo run --bin webgpu-simple-triangle`.

It's important to emphasize that all of these functions we called
like `set_pipeline`, and `draw` only add commands to a command buffer.
They don't actually execute the commands. The commands are executed
when we submit the command buffer to the device queue.

<a id="a-rasterization"></a>WebGPU takes every 3 vertices we return from our vertex shader and uses
them to rasterize a triangle. It does this by determining which pixels'
centers are inside the triangle. It then calls our fragment shader for
each pixel to ask what color to make it.

Imagine the texture we are rendering
to was 15x11 pixels. These are the pixels that would be drawn to

<div class="webgpu_center">
  <div data-diagram="clip-space-to-texels" style="display: inline-block; max-width: 500px; width: 100%"></div>
  <div>drag the vertices</div>
</div>

So, now we've seen a very small working WebGPU example. It should be pretty
obvious that hard coding a triangle inside a shader is not very flexible. We
need ways to provide data and we'll cover those in the following articles. The
points to take away from the code above,

* WebGPU just runs shaders. It's up to you to fill them with code to do useful things
* Shaders are specified in a shader module and then turned into a pipeline
* WebGPU can draw triangles
* WebGPU draws to textures (we happened to get a texture from the canvas)
* WebGPU works by encoding commands and then submitting them.

# <a id="a-run-computations-on-the-gpu"></a>Run computations on the GPU

Let's write a basic example for doing some computation on the GPU. Since a
compute example doesn't need a window or a canvas, we don't need the `App`
helper at all — this example is raw wgpu from top to bottom.

We start off with the code to get a WebGPU device — the same code we saw
inside `App::new` earlier:

```rust
async fn main_async() {
  let instance = wgpu::Instance::default();
  let Ok(adapter) = instance
      .request_adapter(&wgpu::RequestAdapterOptions::default())
      .await
  else {
    fail("need a browser that supports WebGPU");
    return;
  };
  let Ok((device, queue)) = adapter
      .request_device(&wgpu::DeviceDescriptor::default())
      .await
  else {
    fail("need a browser that supports WebGPU");
    return;
  };
```

Then we create a shader module.

```rust
  let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("doubling compute module"),
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      @group(0) @binding(0) var<storage, read_write> data: array<f32>;

      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        let i = id.x;
        data[i] = data[i] * 2.0;
      }
    "#.into()),
  });
```

The WGSL is, again, character for character what you'd write in JavaScript.

First, we declare a variable called `data` of type `storage` that we want to be
able to both read from and write to.

```wgsl
      @group(0) @binding(0) var<storage, read_write> data: array<f32>;
```

We declare its type as `array<f32>` which means an array of 32-bit floating point
values. We tell it we're going to specify this array on binding location 0 (the
`binding(0)`) in bindGroup 0 (the `@group(0)`).

Then we declare a function called `computeSomething` with the `@compute`
attribute which makes it a compute shader.

```wgsl
      @compute @workgroup_size(1) fn computeSomething(
        @builtin(global_invocation_id) id: vec3u
      ) {
        ...
```

Compute shaders are required to declare a workgroup size which we will cover
later. For now, we'll just set it to 1 with the attribute `@workgroup_size(1)`.
We declare it to have one parameter `id` which uses a `vec3u`. A `vec3u` is
three unsigned 32-bit integer values. Like our vertex shader above, this is the
iteration number. It's different in that compute shader iteration numbers are 3
dimensional (have 3 values). We declare `id` to get its value from the built-in
`global_invocation_id`.

You can *kind of* think of compute shaders as running like this. This is an over
simplification but it will do for now.

```rust
// pseudo code
fn dispatch_workgroups(width: u32, height: u32, depth: u32) {
  for z in 0..depth {
    for y in 0..height {
      for x in 0..width {
        let workgroup_id = (x, y, z);
        dispatch_workgroup(workgroup_id);
      }
    }
  }
}

fn dispatch_workgroup(workgroup_id: (u32, u32, u32)) {
  // from @workgroup_size in WGSL
  let (width, height, depth) = workgroup_size;
  for z in 0..depth {
    for y in 0..height {
      for x in 0..width {
        let local_invocation_id = (x, y, z);
        let global_invocation_id =
            workgroup_id * workgroup_size + local_invocation_id;
        compute_shader(global_invocation_id);
      }
    }
  }
}
```

Since we set `@workgroup_size(1)`, effectively the pseudo-code above becomes:

```rust
// pseudo code
fn dispatch_workgroups(width: u32, height: u32, depth: u32) {
  for z in 0..depth {
    for y in 0..height {
      for x in 0..width {
        let workgroup_id = (x, y, z);
        dispatch_workgroup(workgroup_id);
      }
    }
  }
}

fn dispatch_workgroup(workgroup_id: (u32, u32, u32)) {
  let global_invocation_id = workgroup_id;
  compute_shader(global_invocation_id);
}
```

Finally, we use the `x` field of `id` to index `data` and multiply each value
by 2.

```wgsl
        let i = id.x;
        data[i] = data[i] * 2.0;
```

Above, `i` is just the first of the 3 iteration numbers.

Now that we've created the shader, we need to create a pipeline.

```rust
  let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("doubling compute pipeline"),
    layout: None,
    module: &module,
    entry_point: None,
    compilation_options: Default::default(),
    cache: None,
  });
```

Here we just tell it we're using the shader `module` we
created and since there is only one `@compute` entry point WebGPU knows we want to call it. `layout` is
`None` again, telling WebGPU to figure out the layout from the shaders. [^layout-auto]

[^layout-auto]: `layout: None` (JavaScript's `layout: 'auto'`) is convenient but it's impossible to share bind groups
across pipelines using it. Most of the examples on this site
never use a bind group with multiple pipelines. We'll cover explicit layouts in [another article](webgpu-bind-group-layouts.html).

Next, we need some data.

```rust
  let input: [f32; 3] = [1.0, 3.0, 5.0];
```

That data only exists in our Rust program's memory, on the CPU. For the GPU to
use it, we need to make a buffer that exists on the GPU and copy the data to
the buffer.

```rust
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

Above, we call `device.create_buffer` to create a buffer. `size` is the size in
bytes. In this case, it will be 12 because the size in bytes of `[f32; 3]`
is 12.

Every WebGPU buffer we create has to specify a `usage`. There are a bunch of
flags we can pass for usage but not all of them can be used together. Here we
say we want this buffer to be usable as `storage` by passing
`BufferUsages::STORAGE`. This makes it compatible with `var<storage,...>` from
the shader. Further, we want to be able to copy data to this buffer so we include
the `COPY_DST` flag. And finally, we want to be able to copy data
from the buffer so we include `COPY_SRC`.

`queue.write_buffer` wants plain bytes (`&[u8]`), but we have `[f32; 3]`.
[`bytemuck::cast_slice`](https://docs.rs/bytemuck) reinterprets our slice of
floats as a slice of bytes without copying — the Rust equivalent of putting
data in a `Float32Array` and handing over its underlying bytes. We'll use
`bytemuck` every time we send typed data to the GPU; the
[article on memory layout](webgpu-memory-layout.html) goes into detail.

Note that you can not directly read the contents of a WebGPU buffer from
normal Rust code. Instead, you have to "map" it which is another way of
requesting access to the buffer from WebGPU because the buffer might be in use
and because it might only exist on the GPU.

WebGPU buffers that can be mapped can't be used for much else. In
other words, we can not map the buffer we just created above and if we try to add
the flag to make it mappable, we'll get an error that it is not compatible with
usage `STORAGE`.

So, in order to see the result of our computation, we'll need another buffer.
After running the computation, we'll copy the buffer above to this result buffer
and set its flags so we can map it.

```rust
  // create a buffer on the GPU to get a copy of the results
  let result_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("result buffer"),
    size: std::mem::size_of_val(&input) as u64,
    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
```

`MAP_READ` means we want to be able to map this buffer for reading data.

In order to tell our shader about the buffer we want it to work on, we need to
create a bindGroup.

```rust
  // Setup a bindGroup to tell the shader which
  // buffer to use for the computation
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("bindGroup for work buffer"),
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: work_buffer.as_entire_binding(),
    }],
  });
```

We get the layout for the bindGroup from the pipeline. Then we set up bindGroup
entries. The 0 in `pipeline.get_bind_group_layout(0)` corresponds to the
`@group(0)` in the shader. The `binding: 0` of the `entries` corresponds to
the `@group(0) @binding(0)` in the shader.

Now we can start encoding commands.

```rust
  // Encode commands to do the computation
  let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("doubling encoder"),
  });
  {
    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
      label: Some("doubling compute pass"),
      timestamp_writes: None,
    });
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.dispatch_workgroups(input.len() as u32, 1, 1);
  }
```

We create a command encoder. We start a compute pass. We set the pipeline, then
we set the bindGroup. Here, the `0` in `pass.set_bind_group(0, &bind_group, &[])`
corresponds to `@group(0)` in the shader. We then call `dispatch_workgroups` and in
this case, we pass it `input.len()` which is `3` telling WebGPU to run the
compute shader 3 times. Like the render pass earlier, the pass ends when
`pass` drops at the end of the block.

Here's what the situation will be when `dispatch_workgroups` is executed.

<div class="webgpu_center"><img src="resources/webgpu-simple-compute-diagram.svg" style="width: 553px;"></div>

After the computation is finished we ask WebGPU to copy from `work_buffer` to
`result_buffer`.

```rust
  // Encode a command to copy the results to a mappable buffer.
  encoder.copy_buffer_to_buffer(&work_buffer, 0, &result_buffer, 0, result_buffer.size());
```

Now we can `finish` the encoder to get a command buffer and then submit that
command buffer.

```rust
  // Finish encoding and submit the commands
  let command_buffer = encoder.finish();
  queue.submit([command_buffer]);
```

We then map the results buffer and get a copy of the data.

```rust
  // Read the results
  wgpu_fun::map_async(&device, &result_buffer, wgpu::MapMode::Read).await;
  let result: Vec<f32> = {
    let data = result_buffer.slice(..).get_mapped_range().unwrap();
    bytemuck::cast_slice(&data).to_vec()
  };

  print(&format!("input {input:?}"));
  print(&format!("result {result:?}"));

  result_buffer.unmap();
```

In the JavaScript API this is `await resultBuffer.mapAsync(GPUMapMode.READ)`.
wgpu's raw `map_async` instead takes a callback, and on native we also have to
tell the device to make progress with `device.poll(...)` (the browser does
that part for us). `wgpu_fun::map_async` is a
[12-line helper](https://github.com/yesnocancel/webgpufundamentals-rust/blob/main/rust/wgpu_fun/src/lib.rs)
that wraps the callback in a future so we can simply `await` it, just like the
JavaScript version.

Once mapped, `result_buffer.slice(..).get_mapped_range()` returns a view of
the buffer's bytes, and `bytemuck::cast_slice` lets us see those bytes as
`f32`s again. One important detail: the mapped range is only valid until we
call `unmap` — which is why we copy the data out with `to_vec()` before
unmapping. (In JavaScript, `unmap` silently makes the `ArrayBuffer` vanish
out from under you; in Rust the borrow checker won't even let us call `unmap`
while the `data` view is still alive. The `{ }` block ends the borrow.)

Running that we can see we got the result back, all the numbers have been
doubled. Natively it prints to the terminal; in the browser, open the
JavaScript console to see the output.

{{{example url="../webgpu-simple-compute.html"}}}

We'll cover how to really use compute shaders in other articles. For now, you
hopefully have gleaned some understanding of what WebGPU does. EVERYTHING ELSE
IS UP TO YOU! Think of WebGPU as similar to other programming languages. It
provides a few basic features and leaves the rest to your creativity.

What makes WebGPU programming special is these functions, vertex shaders,
fragment shaders, and compute shaders, run on your GPU. A GPU could have over
10000 processors which means they can potentially do more than 10000
calculations in parallel which is likely 3 or more orders of magnitude than your
CPU can do in parallel.

## <a id="a-resizing"></a> Simple Canvas Resizing

Before we move on, let's go back to our triangle drawing example and add some
basic support for resizing the canvas. This section is about what happens in
the browser; natively, the helper always keeps the surface matched to the
window size for us. Sizing a canvas is actually a topic that can have many
subtleties so [there is an entire article on it](webgpu-resizing-the-canvas.html).
For now though let's just add some basic support.

First, we'll add some CSS to the example's HTML page to make our canvas fill
the page.

```html
<style>
html, body {
  margin: 0;       /* remove the default margin          */
  height: 100%;    /* make the html,body fill the page   */
}
canvas {
  display: block;  /* make the canvas act like a block   */
  width: 100%;     /* make the canvas fill its container */
  height: 100%;
}
</style>
```

That CSS alone will make the canvas get displayed to cover the page but it won't change
the resolution of the canvas itself so you might notice, if you make the example below
large, like if you click the full-screen button, you'll see the edges of the triangle
are blocky.

{{{example url="../webgpu-simple-triangle-with-canvas-css.html"}}}

`<canvas>` tags, by default, have a resolution of 300x150 pixels. We'd like to
adjust the resolution of the canvas to match the size it is displayed at.
In the browser, the way to do this is with a `ResizeObserver`: you give it a
function to call whenever the elements you've asked it to observe change their
size. In JavaScript that code looks like this:

```js
const observer = new ResizeObserver(entries => {
  for (const entry of entries) {
    const canvas = entry.target;
    const width = entry.contentBoxSize[0].inlineSize;
    const height = entry.contentBoxSize[0].blockSize;
    canvas.width = Math.max(1, Math.min(width, device.limits.maxTextureDimension2D));
    canvas.height = Math.max(1, Math.min(height, device.limits.maxTextureDimension2D));
  }
  // re-render
  render();
});
observer.observe(canvas);
```

Our helper contains exactly this logic (written with Rust's DOM bindings — see
[web.rs](https://github.com/yesnocancel/webgpufundamentals-rust/blob/main/rust/wgpu_fun/src/web.rs)
if you're curious), but it only *applies* the observed size to the canvas when
we opt in, because sometimes — like the blocky example above — we don't want
it to. We opt in with one line:

```rust
-  let app = App::new("WebGPU Simple Triangle").await;
+  let mut app = App::new("WebGPU Simple Triangle with Canvas Resize").await;
+  app.auto_resize = true;
```

Note that the observed size is limited to the largest size our device supports,
otherwise WebGPU would start generating errors that we tried to make a texture
that is too large. It also can't be allowed to go to zero or again we'd get
errors. [See the longer article for details](webgpu-resizing-the-canvas.html).

Because we passed `RenderMode::Once`, the helper re-runs our frame closure
whenever the size changes — that's the `render()` call at the end of the
JavaScript above. The new-size texture arrives because a freshly configured
surface hands out a texture at the new resolution the next time we ask for
`frame.view`; there's nothing left for us to do.

{{{example url="../webgpu-simple-triangle-with-canvas-resize.html"}}}

> Note: The code above does not handle responding to zoom which may change
the resolution of the canvas. It also doesn't deal with higher resolutions
for high-res displays. For those issues, see
[the article on resizing the canvas](webgpu-resizing-the-canvas.html).

In the following articles, we'll cover various ways to pass data into shaders.

* [inter-stage variables](webgpu-inter-stage-variables.html)
* [uniforms](webgpu-uniforms.html)
* [storage buffers](webgpu-storage-buffers.html)
* [vertex buffers](webgpu-vertex-buffers.html)
* [textures](webgpu-textures.html)
* [constants](webgpu-constants.html)

Then we'll cover [the basics of WGSL](webgpu-wgsl.html).

This order is from the simplest to the most complex. Inter-stage variables
require no external setup to explain. We can see how to use them using nothing
but changes to the WGSL we used above. Uniforms are effectively global variables
and as such are used in all 3 kinds of shaders (vertex, fragment, and compute).
Going from uniform buffers to storage buffers is trivial as shown at the top of
the article on storage buffers. Vertex buffers are only used in vertex shaders.
They are more complex because they require describing the data layout to WebGPU.
Textures are the most complex as they have tons of types and options.

I'm a little bit worried these articles will be boring at first. Feel free to
jump around if you'd like. Just remember if you don't understand something you
probably need to read or review these basics. Once we get the basics down, we'll
start going over actual techniques.

One other thing. Every example page runs the Rust code you read in the
article, compiled to WebAssembly. Each example links to its full Rust source,
and you can clone [the repository]("https://github.com/yesnocancel/webgpufundamentals-rust")
and run any of them natively with `cargo run --bin <example-name>`.

<div class="webgpu_bottombar">
<p>
The code above gets a WebGPU device in a very terse way. A more verbose
way would be something like
</p>
<pre class="prettyprint showmods">{{#escapehtml}}
async fn start() {
  let instance = wgpu::Instance::default();

  let adapter = match instance
      .request_adapter(&wgpu::RequestAdapterOptions::default())
      .await {
    Ok(adapter) => adapter,
    Err(e) => {
      fail(&format!("this system does not support WebGPU: {e}"));
      return;
    }
  };

  let (device, queue) = match adapter
      .request_device(&wgpu::DeviceDescriptor::default())
      .await {
    Ok(pair) => pair,
    Err(e) => {
      fail(&format!("failed to create a WebGPU device: {e}"));
      return;
    }
  };

  device.set_device_lost_callback(|reason, message| {
    eprintln!("WebGPU device was lost: {message}");

    // reason will be DeviceLostReason::Destroyed if we intentionally
    // destroy the device.
    if !matches!(reason, wgpu::DeviceLostReason::Destroyed) {
      // try again
    }
  });

  main(device, queue);
}
{{/escapehtml}}</pre>
<p>
A device can be lost for many reasons. Maybe the user ran a really intensive
app and it crashed their GPU. Maybe the user updated their drivers. Maybe the
user has an external GPU and unplugged it. Maybe another page used a lot of
GPU, your tab was in the background and the browser decided to free up some
memory by losing the device for background tabs. The point to take away is
that for any serious apps you probably want to handle losing the device.
</p>
<p>
Note that <code>request_device</code> always returns a device. It just might start lost.
WebGPU is designed so that, for the most part, the device will appear to work,
at least from an API level. Calls to create things and use them will appear
to succeed but they won't actually function. It's up to you to take action
when the device is lost.
</p>
</div>

<!-- keep this at the bottom of the article -->
<script type="module" src="webgpu-fundamentals.js"></script>
