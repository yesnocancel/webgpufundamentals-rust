use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{Frame, RenderMode};

/// Browser entry point: spawn the async run function on the JS event loop.
pub fn start(fut: impl std::future::Future<Output = ()> + 'static) {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Warn);
    wasm_bindgen_futures::spawn_local(fut);
}

pub struct App {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    /// When true the canvas drawing buffer resolution follows the canvas's
    /// displayed size (like the ResizeObserver code in the lessons). False
    /// by default: the canvas keeps its own resolution and CSS just
    /// stretches it, like a raw `<canvas>`.
    pub auto_resize: bool,
    /// How the surface's alpha channel is composited with what's behind the
    /// window/canvas, like the JS `alphaMode` canvas configuration option.
    /// `Auto` (the default) behaves like `'opaque'`; use
    /// `wgpu::CompositeAlphaMode::PreMultiplied` for `'premultiplied'`.
    pub alpha_mode: wgpu::CompositeAlphaMode,
    /// Optional override for `alpha_mode`, consulted every frame (examples
    /// use it to read a GUI setting). When the returned mode differs from
    /// the surface's current configuration the surface is reconfigured —
    /// the equivalent of calling `context.configure` with a new
    /// `alphaMode` at render time in the JS examples.
    pub alpha_mode_fn: Option<Box<dyn Fn() -> wgpu::CompositeAlphaMode>>,
    /// With `auto_resize`, divide the observed canvas size by this (the
    /// low-res-canvas trick some lessons use, e.g. `inlineSize / 64 | 0`).
    pub resize_divisor: u32,
    /// Usage flags for the surface's textures (JS `context.configure`'s
    /// `usage`). Defaults to RENDER_ATTACHMENT.
    pub usage: wgpu::TextureUsages,
    surface: wgpu::Surface<'static>,
    canvas: HtmlCanvasElement,
    max_texture_dimension: u32,
}

fn canvas() -> HtmlCanvasElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("canvas")
        .unwrap()
        .expect("no <canvas> element on the page")
        .dyn_into()
        .unwrap()
}

// Exports the example pages' GUI code calls to change a setting from JS
// (muigui onChange handlers). See settings.rs.
#[wasm_bindgen]
pub fn set_setting_num(name: &str, value: f64) {
    crate::settings::set_setting(name, crate::SettingValue::Number(value));
}

#[wasm_bindgen]
pub fn set_setting_str(name: &str, value: String) {
    crate::settings::set_setting(name, crate::SettingValue::Text(value));
}

#[wasm_bindgen]
pub fn set_setting_bool(name: &str, value: bool) {
    crate::settings::set_setting(name, crate::SettingValue::Bool(value));
}

/// Show a message on the page, like the JS examples' fail().
pub fn fail(msg: &str) {
    let document = web_sys::window().unwrap().document().unwrap();
    let div = document.create_element("div").unwrap();
    div.set_attribute(
        "style",
        "position:fixed;inset:0;display:flex;justify-content:center;align-items:center;\
         background:#400;color:white;font-size:x-large;font-family:sans-serif;\
         text-align:center;padding:1em;",
    )
    .unwrap();
    div.set_text_content(Some(msg));
    document.body().unwrap().append_child(&div).unwrap();
}

impl App {
    pub async fn new(title: &str) -> App {
        Self::new_with_features(title, wgpu::Features::empty()).await
    }

    /// Like [`App::new`], but enables the given **optional** features on the
    /// device if (and only if) the adapter supports them — the JS
    /// `adapter.features.has(...)` pattern. Check `app.device.features()`
    /// to see which ones you actually got.
    pub async fn new_with_features(_title: &str, optional_features: wgpu::Features) -> App {
        let canvas = canvas();
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .expect("failed to create surface from canvas");
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
        {
            Ok(adapter) => adapter,
            Err(_) => {
                fail("need a browser that supports WebGPU");
                panic!("no adapter");
            }
        };
        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: adapter.features() & optional_features,
                ..Default::default()
            })
            .await
        {
            Ok(pair) => pair,
            Err(_) => {
                fail("need a browser that supports WebGPU");
                panic!("no device");
            }
        };
        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        App {
            device,
            queue,
            format,
            auto_resize: false,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            alpha_mode_fn: None,
            resize_divisor: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            surface,
            canvas,
            max_texture_dimension,
        }
    }

    pub fn run(self, mode: RenderMode, frame_fn: impl FnMut(&Frame) + 'static) {
        let app = Rc::new(self);
        let frame_fn = Rc::new(RefCell::new(frame_fn));
        // Current canvas size in device pixels, updated by the ResizeObserver.
        let size = Rc::new(Cell::new((0u32, 0u32)));
        let configured =
            Rc::new(Cell::new((0u32, 0u32, wgpu::CompositeAlphaMode::Auto)));
        let start_time = web_sys::window().unwrap().performance().unwrap().now();

        let render = {
            let app = app.clone();
            let frame_fn = frame_fn.clone();
            let size = size.clone();
            let configured = configured.clone();
            move || {
                let (width, height) = if app.auto_resize {
                    size.get()
                } else {
                    (app.canvas.width(), app.canvas.height())
                };
                if width == 0 || height == 0 {
                    return;
                }
                let alpha_mode = match &app.alpha_mode_fn {
                    Some(f) => f(),
                    None => app.alpha_mode,
                };
                if configured.get() != (width, height, alpha_mode) {
                    if app.auto_resize {
                        app.canvas.set_width(width);
                        app.canvas.set_height(height);
                    }
                    app.surface.configure(
                        &app.device,
                        &wgpu::SurfaceConfiguration {
                            usage: app.usage,
                            format: app.format,
                            color_space: wgpu::SurfaceColorSpace::Auto,
                            width,
                            height,
                            present_mode: wgpu::PresentMode::default(),
                            desired_maximum_frame_latency: 2,
                            alpha_mode,
                            view_formats: vec![],
                        },
                    );
                    configured.set((width, height, alpha_mode));
                }
                let frame = match app.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    _ => return,
                };
                let view = frame.texture.create_view(&Default::default());
                let now = web_sys::window().unwrap().performance().unwrap().now();
                (frame_fn.borrow_mut())(&Frame {
                    device: &app.device,
                    queue: &app.queue,
                    view: &view,
                    format: app.format,
                    width,
                    height,
                    time: (now - start_time) / 1000.0,
                });
                app.queue.present(frame);
            }
        };
        let render = Rc::new(render);

        // Pointer + wheel events feed the shared event queue (see events.rs).
        {
            let canvas = app.canvas.clone();
            let to_device_px = move |e: &web_sys::MouseEvent| {
                let scale_x = canvas.width() as f32 / canvas.client_width().max(1) as f32;
                let scale_y = canvas.height() as f32 / canvas.client_height().max(1) as f32;
                (e.offset_x() as f32 * scale_x, e.offset_y() as f32 * scale_y)
            };

            let t = to_device_px.clone();
            let on_down = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let (x, y) = t(&e);
                crate::events::push_event(crate::PointerEvent::Down {
                    x,
                    y,
                    button: e.button() as u32,
                });
            });
            let t = to_device_px.clone();
            let on_move = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let (x, y) = t(&e);
                crate::events::push_event(crate::PointerEvent::Move { x, y });
            });
            let t = to_device_px;
            let on_up = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(move |e: web_sys::MouseEvent| {
                let (x, y) = t(&e);
                crate::events::push_event(crate::PointerEvent::Up {
                    x,
                    y,
                    button: e.button() as u32,
                });
            });
            let on_wheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |e: web_sys::WheelEvent| {
                e.prevent_default();
                crate::events::push_event(crate::PointerEvent::Wheel {
                    delta_x: e.delta_x() as f32,
                    delta_y: e.delta_y() as f32,
                });
            });
            let c = &app.canvas;
            c.add_event_listener_with_callback("pointerdown", on_down.as_ref().unchecked_ref())
                .unwrap();
            c.add_event_listener_with_callback("pointermove", on_move.as_ref().unchecked_ref())
                .unwrap();
            c.add_event_listener_with_callback("pointerup", on_up.as_ref().unchecked_ref())
                .unwrap();
            c.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref())
                .unwrap();
            on_down.forget();
            on_move.forget();
            on_up.forget();
            on_wheel.forget();
        }

        // Watch the canvas's device-pixel size, like the JS lessons do.
        {
            let app2 = app.clone();
            let size = size.clone();
            let render = render.clone();
            let observer_cb = Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
                let app = &app2;
                for entry in entries.iter() {
                    let entry: web_sys::ResizeObserverEntry = entry.dyn_into().unwrap();
                    let (mut width, mut height);
                    let dpr_boxes = entry.device_pixel_content_box_size();
                    if dpr_boxes.length() > 0 {
                        let box_size: web_sys::ResizeObserverSize =
                            dpr_boxes.get(0).dyn_into().unwrap();
                        width = box_size.inline_size() as u32;
                        height = box_size.block_size() as u32;
                    } else {
                        let rect = entry.content_rect();
                        let dpr = web_sys::window().unwrap().device_pixel_ratio();
                        width = (rect.width() * dpr) as u32;
                        height = (rect.height() * dpr) as u32;
                    }
                    width = (width / app.resize_divisor).clamp(1, app.max_texture_dimension);
                    height = (height / app.resize_divisor).clamp(1, app.max_texture_dimension);
                    size.set((width, height));
                }
                // Re-render on resize; the continuous loop picks the new
                // size up on its own.
                if mode == RenderMode::Once {
                    render();
                }
            });
            let observer =
                web_sys::ResizeObserver::new(observer_cb.as_ref().unchecked_ref()).unwrap();
            observer.observe(&app.canvas);
            observer_cb.forget();
        }

        {
            // Settings changes re-render Once-mode examples immediately.
            let render = render.clone();
            crate::settings::set_redraw_hook(move || {
                if mode == RenderMode::Once {
                    render();
                }
            });
        }

        if mode == RenderMode::Continuous {
            // requestAnimationFrame loop.
            let raf: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
            let raf2 = raf.clone();
            let render = render.clone();
            *raf.borrow_mut() = Some(Closure::new(move || {
                render();
                web_sys::window()
                    .unwrap()
                    .request_animation_frame(
                        raf2.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                    )
                    .unwrap();
            }));
            web_sys::window()
                .unwrap()
                .request_animation_frame(raf.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .unwrap();
            // Leak the closure: the loop runs for the page's lifetime.
            std::mem::forget(raf);
        }
    }
}
