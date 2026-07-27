# Conversion conventions: JS/WebGPU → Rust/wgpu

How this fork converts webgpufundamentals.org lessons and examples from
JavaScript to Rust + wgpu. Read this before converting a lesson.

## Layout

- `rust/` — cargo workspace.
  - `wgpu_fun/` — tiny shared helper (window/canvas, device init, render
    loop, offscreen test mode). Explained to readers in the Fundamentals
    lesson; keep it SMALL and boring. Don't add features without a lesson
    explaining them.
  - `examples/src/bin/<page-name>.rs` — one bin per example page, named
    exactly like the original `webgpu/<page-name>.html`.
- `webgpu/<name>.html` — example page; converted ones load
  `./wasm/<name>/<name>.js` (wasm-bindgen output) instead of inline JS.
  Convert with `python build/gen-rust-example-html.py <name>...`.
- `webgpu/lessons/<name>.md` — the article.

## wgpu_fun API (all examples use this shape)

```rust
use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let app = App::new("title").await;            // device/queue/format/surface
    // app.device, app.queue, app.format
    // app.auto_resize = true;  // drawing buffer follows canvas/window size
    app.run(RenderMode::Once /* or Continuous */, move |frame: &Frame| {
        // frame.device, frame.queue, frame.view, frame.format,
        // frame.width, frame.height, frame.time (secs, f64)
    });
}
fn main() { wgpu_fun::start(run()); }
```

- Compute-only examples: no `App`; raw `wgpu::Instance::default()` etc.,
  still `wgpu_fun::start(main_async())` for the platform entry.
- `wgpu_fun::map_async(&device, &buffer, wgpu::MapMode::Read).await` = JS
  `await buffer.mapAsync(...)`.
- `wgpu_fun::print(&msg)` = `console.log` / `println!`.
- `wgpu_fun::fail(&msg)` (wasm only) shows the red failure banner.

## Examples with a settings GUI (muigui in the originals)

The original pages import muigui from `../3rdparty/muigui-0.x.module.js` and
bind a `settings` object. Converted pages KEEP the muigui panel in page JS;
its onChange handlers call the wasm module's exported setters, and the Rust
frame code reads current values from wgpu_fun's settings store:

- Page HTML (hand-written, gen script won't handle these):

  ```html
  <script type="module">
  import GUI from './3rdparty/muigui-0.x.module.js';
  import init, * as wasm from './wasm/<name>/<name>.js';
  await init();
  const settings = { scale: 1 };
  const gui = new GUI();
  gui.add(settings, 'scale', 0.1, 4).onChange(v => wasm.set_setting_num('scale', v));
  </script>
  ```

  (`set_setting_num`, `set_setting_str`, `set_setting_bool` are exported from
  every example's wasm module via wgpu_fun.)

- Rust side reads `wgpu_fun::setting_f64("scale", 1.0)` (or `setting_str`,
  `setting_bool`) inside the frame callback; defaults must match the page's
  initial `settings` object. A change automatically triggers a re-render for
  `RenderMode::Once` examples.
- Native test mode can override via `WGPU_FUN_SETTING_<name>=<value>` env
  vars — use this to verify GUI-dependent rendering paths.

## Porting rules for examples

- WGSL shader code is kept **character-for-character identical** to the
  original (including its comments and camelCase names).
- Strings that store WGSL get a `/* wgsl */` comment in front of them
  (`Wgsl(/* wgsl */ r#"..."#)`, `let shader_src = /* wgsl */ r#"..."#`),
  mirroring the JS originals' `code: /* wgsl */ \`...\`` editor-hint
  convention (explained in the fundamentals article). Articles mirror
  their original's usage: where the original article shows the marker,
  the converted article does too.
- Keep every `label:` string identical. Keep JS comments, translated to the
  matching Rust line.
- JS `camelCase` variables → Rust `snake_case`.
- Typed arrays → fixed arrays/`Vec` + `bytemuck::cast_slice`.
- Math: use `glam` where the original uses its own matrix helpers; the 3D
  math lessons build up their own matrix code — follow the lesson.
- Randomness: the JS lessons define their own `rand(min, max)` helper on
  `Math.random`. Mirror that with a small local helper in the example file
  using a deterministic xorshift32 (fixed seed), e.g.:

  ```rust
  fn rand(min: f32, max: f32) -> f32 {
      use std::cell::Cell;
      thread_local!(static STATE: Cell<u32> = const { Cell::new(0x13579bdf) });
      STATE.with(|s| {
          let mut x = s.get();
          x ^= x << 13;
          x ^= x >> 17;
          x ^= x << 5;
          s.set(x);
          min + (max - min) * (x as f32 / u32::MAX as f32)
      })
  }
  ```

  Do NOT use the `rand` crate (getrandom needs special setup on wasm), and a
  fixed seed keeps the native test PNGs reproducible.
- Keep code as close to a line-by-line translation as idiomatic Rust allows.
  The article walks through the code; if the code drifts from the article the
  lesson breaks.

## wgpu 30 API notes (differs from older wgpu / JS API)

- `layout: None` == JS `layout: 'auto'`.
- `entry_point: None` picks the sole entry point (like omitted JS entryPoint).
- `RenderPipelineDescriptor` needs `multiview_mask: None` (not `multiview`),
  `cache: None`, `compilation_options: Default::default()`.
- `RenderPassColorAttachment` needs `depth_slice: None`.
- Passes end on drop; wrap in `{ }` blocks.
- `surface.get_current_texture()` returns enum `CurrentSurfaceTexture`
  (`Success`/`Suboptimal`/...), and presenting is `queue.present(frame)`.
  (Handled inside wgpu_fun; examples never touch the surface directly.)
- `device.poll(wgpu::PollType::wait_indefinitely())` → `Result`.
- `get_mapped_range()` returns `Result` → `.unwrap()`.
- `request_adapter`/`request_device` return `Result`; single-arg
  `request_device(&DeviceDescriptor)`.
- `SurfaceConfiguration` has a `color_space: wgpu::SurfaceColorSpace` field
  (use `Auto`).
- `Instance::default()`; `RequestAdapterOptions::default()`.
- `pass.draw(0..3, 0..1)` == JS `pass.draw(3)`.
- `pass.set_bind_group(0, &bind_group, &[])` — extra `&[]` is dynamic offsets.
- `VertexState::buffers` is `&[Option<VertexBufferLayout>]` — wrap each in
  `Some(...)`; `step_mode` is a mandatory field (JS default `'vertex'`).
- `set_vertex_buffer`/`set_index_buffer` take `buffer.slice(..)`, not the
  buffer.
- JS `{ binding: 0, resource: buffer }` → `wgpu::BindGroupEntry { binding: 0,
  resource: buffer.as_entire_binding() }`.
- Pipeline-overridable constants: `PipelineCompilationOptions { constants:
  &[("name", 1.0)], ..Default::default() }` — a slice of `(&str, f64)` pairs
  (not a HashMap); `@id(123)` constants are keyed by the decimal string
  `"123"`.
- Per-object labels: `label: Some(&format!("thing {i}"))` works fine.
- `PipelineLayoutDescriptor { label, bind_group_layouts: &[Option<&BindGroupLayout>],
  immediate_size: 0 }` — layouts are Option-wrapped; no `push_constant_ranges`
  field in wgpu 30.
- Error scopes: `device.push_error_scope(wgpu::ErrorFilter::Validation)`
  returns a guard; `guard.pop().await` yields `Option<wgpu::Error>` (JS
  `pushErrorScope`/`popErrorScope`). Native wgpu PANICS on uncaptured
  validation errors — examples that intentionally error must capture the
  scope and print via wgpu_fun::print, then skip the failing work.

## wgpu_fun App knobs (mirror of JS page behaviors)

- `app.auto_resize = true` — the ResizeObserver canvas-resolution behavior.
- `app.resize_divisor = 64` — the low-res-canvas trick
  (`inlineSize / 64 | 0`); browser-only, ignored natively.
- `app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied` — JS
  `alphaMode: 'premultiplied'` (there's also `app.alpha_mode_fn` for
  per-frame GUI-driven changes).
- `app.usage = TextureUsages::...` — JS `context.configure`'s `usage`.
- `App::new_with_features(title, features)` — requests optional features if
  the adapter supports them (JS `adapter.features.has(...)` +
  requiredFeatures).
- `wgpu_fun::drain_pointer_events()` — drain queued `PointerEvent`s
  (Down/Move/Up/Wheel, device-pixel coords) inside the frame callback; the
  JS originals' pointerdown/pointermove/wheel listeners map to this. Events
  trigger a re-render in `RenderMode::Once`.

## Multiple canvases (`MultiApp`, the multiple-canvases lesson)

For examples that render to several canvases at once:

```rust
use wgpu_fun::{Canvas, MultiApp, MultiFrame, RenderMode};

let mut app = MultiApp::new("title").await;   // device/queue/format, no surface
app.auto_resize = true;                       // optional, before canvases()
let canvases: Vec<Canvas> = app.canvases(&[(300, 150); 3]);
app.run(RenderMode::Once /* or Continuous */, move |frame: &MultiFrame| {
    // frame.device / frame.queue / frame.format / frame.time — no view;
    for canvas in &canvases {
        let view = canvas.current_view();  // JS context.getCurrentTexture().createView()
        // canvas.width()/height(), canvas.is_visible() (IntersectionObserver)
    }
});
```

- Browser: `canvases()` wraps every `<canvas>` on the page in document
  order (the arg is ignored); surfaces are configured lazily and presented
  automatically after the frame callback. Pages that create canvases
  dynamically do the DOM work in page JS *before* `init()`.
- Native: there's no multi-surface window, so each entry of the arg
  creates an offscreen texture of that size; the helper composites them
  into one window as a wrapping grid (mouse-wheel scroll) or into the test
  PNG, and `is_visible()` means "grid cell intersects the window".

## Verifying examples (required — "all examples work")

Native, headless, renders offscreen and writes a PNG:

```sh
cd rust && cargo build --bin <name>
WGPU_FUN_TEST=1 WGPU_FUN_TEST_OUT=test_frames/<name>.png ./target/debug/<name>
```

- Prints `TEST-OK <path>` on success. **Read the PNG** and compare against
  the original example's appearance (run the original JS example's
  description in the lesson). RenderMode::Continuous examples render frames
  at t=0, 0.25, 0.5 and the PNG shows the last one.
- Compute examples: run plain (no env vars) and check stdout matches the
  lesson's expected output.
- wasm32 builds are verified separately in batch (`rust/build-wasm.sh`).

## Porting rules for lessons (`webgpu/lessons/*.md`)

- Keep the article's structure, anchors (`<a id=...>`), footnotes, diagrams,
  `{{{example url="..."}}}` directives, and bottom `<script>` tags exactly.
- Keep prose unless it's JS-specific; adapt JS references to Rust ones
  (e.g. `Float32Array` explanations → `bytemuck`, `?.` → `Result`).
- Code snippets: show the Rust translation, keeping the original's
  progressive-diff style (`+`/`-` prefixed lines inside ```rust fences).
- WGSL snippets unchanged.
- JS API names in prose → wgpu names (`createBuffer` → `create_buffer`,
  `GPUBufferUsage.STORAGE` → `BufferUsages::STORAGE`, etc.).
- The site templating uses `{{...}}`; never introduce stray `{{`.
- `yesnocancel` is a placeholder for the GitHub owner, replaced at publish.
