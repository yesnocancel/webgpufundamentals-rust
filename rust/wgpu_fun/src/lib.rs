//! Tiny helper shared by all the WebGPU Fundamentals in Rust examples.
//!
//! It does the things every example needs and that are explained once in the
//! first lesson so they don't clutter every listing:
//!
//! * open a window (native) or attach to the page's `<canvas>` (browser/wasm)
//! * create the wgpu `Instance`, `Adapter`, `Device` and `Queue`
//! * configure the surface and keep it sized to the window/canvas
//! * drive the render loop (`requestAnimationFrame` in the browser)
//!
//! Examples look like this:
//!
//! ```no_run
//! async fn run() {
//!     let app = wgpu_fun::App::new("my example").await;
//!     // ... build pipelines with app.device / app.format ...
//!     app.run(wgpu_fun::RenderMode::Once, move |frame| {
//!         // encode and submit commands for one frame
//!     });
//! }
//!
//! fn main() {
//!     wgpu_fun::start(run());
//! }
//! ```
//!
//! When the environment variable `WGPU_FUN_TEST` is set (native only) the
//! frame callback renders into an offscreen texture instead of a window and
//! the result is written to a PNG so the examples can be verified headlessly.

pub use wgpu;

/// Everything a frame callback needs to draw one frame.
pub struct Frame<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    /// View of the texture to render to (the surface texture, or an
    /// offscreen texture in test mode).
    pub view: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
    /// Seconds since the example started.
    pub time: f64,
}

/// How the frame callback is driven.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    /// Render when needed: once at startup and again whenever the
    /// window/canvas is resized.
    Once,
    /// Render every frame (animations).
    Continuous,
}

/// Map a buffer for reading or writing and wait until it is ready, like
/// JavaScript's `await buffer.mapAsync(mode)`.
///
/// wgpu's `map_async` reports completion through a callback; this wraps it in
/// a future, and on native also polls the device (in the browser the browser
/// polls for us).
pub async fn map_async(device: &wgpu::Device, buffer: &wgpu::Buffer, mode: wgpu::MapMode) {
    let (sender, receiver) = futures_channel::oneshot::channel();
    buffer.slice(..).map_async(mode, move |result| {
        sender.send(result).ok();
    });
    #[cfg(not(target_arch = "wasm32"))]
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");
    #[cfg(target_arch = "wasm32")]
    let _ = device;
    receiver
        .await
        .expect("map_async callback dropped")
        .expect("failed to map buffer");
}

/// A decoded image: tightly packed RGBA8 pixels.
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Load and decode a PNG or JPEG, like the JS lessons' `loadImageBitmap(url)`.
///
/// URLs are relative to the `webgpu/` site folder (same as the original
/// examples). In the browser this fetches over HTTP; natively it reads the
/// file from the repository (override the base dir with `WGPU_FUN_ASSET_DIR`).
pub async fn load_image(url: &str) -> ImageData {
    let bytes = load_binary(url).await;
    let img = image::load_from_memory(&bytes)
        .unwrap_or_else(|e| panic!("failed to decode image {url}: {e}"))
        .to_rgba8();
    let (width, height) = img.dimensions();
    ImageData {
        data: img.into_raw(),
        width,
        height,
    }
}

/// Fetch a file's bytes (browser: HTTP fetch; native: file read relative to
/// the `webgpu/` folder).
pub async fn load_binary(url: &str) -> Vec<u8> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let base = std::env::var("WGPU_FUN_ASSET_DIR")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../webgpu").to_string());
        let path = std::path::Path::new(&base).join(url.trim_start_matches("./"));
        std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let response = wasm_bindgen_futures::JsFuture::from(
            web_sys::window().unwrap().fetch_with_str(url),
        )
        .await
        .expect("fetch failed");
        let response: web_sys::Response = response.dyn_into().unwrap();
        let buffer = wasm_bindgen_futures::JsFuture::from(
            response.array_buffer().expect("no response body"),
        )
        .await
        .expect("failed to read response body");
        js_sys::Uint8Array::new(&buffer).to_vec()
    }
}

/// Print a message: stdout on native, the devtools console in the browser
/// (like `console.log` in the JS examples).
pub fn print(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    println!("{msg}");
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&msg.into());
}

/// Log a line of output where the user can see it: appends a `<pre>` element
/// to the page in the browser (like several JS examples' `log()` helper),
/// prints to stdout natively.
pub fn log(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    println!("{msg}");
    #[cfg(target_arch = "wasm32")]
    {
        let document = web_sys::window().unwrap().document().unwrap();
        let elem = document.create_element("pre").unwrap();
        elem.set_text_content(Some(msg));
        document.body().unwrap().append_child(&elem).unwrap();
    }
}

mod settings;
pub use settings::{
    request_redraw, set_setting, setting_bool, setting_f64, setting_str, SettingValue,
};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{start, App};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::{start, App};
