//! Browser backend for examples that draw to *several* canvases at once
//! (the multiple-canvases lesson). Each [`Canvas`] wraps one of the page's
//! `<canvas>` elements with its own wgpu surface, a `ResizeObserver` (when
//! `auto_resize` is on) and an `IntersectionObserver` feeding
//! [`Canvas::is_visible`].

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{MultiFrame, RenderMode};

/// One "canvas": a `<canvas>` element plus its configured wgpu surface.
#[derive(Clone)]
pub struct Canvas(Rc<CanvasInner>);

struct CanvasInner {
    element: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    auto_resize: bool,
    max_texture_dimension: u32,
    /// Desired drawing-buffer size from the ResizeObserver (auto_resize).
    observed_size: Cell<(u32, u32)>,
    configured: Cell<(u32, u32)>,
    visible: Cell<bool>,
    /// The surface texture acquired this frame, presented after the frame
    /// callback returns.
    acquired: RefCell<Option<wgpu::SurfaceTexture>>,
}

impl Canvas {
    /// The canvas's drawing buffer size, like JS `canvas.width`.
    pub fn width(&self) -> u32 {
        self.desired_size().0
    }

    /// The canvas's drawing buffer size, like JS `canvas.height`.
    pub fn height(&self) -> u32 {
        self.desired_size().1
    }

    /// Whether the canvas currently intersects the viewport — the lesson's
    /// `IntersectionObserver` visibility set.
    pub fn is_visible(&self) -> bool {
        self.0.visible.get()
    }

    fn desired_size(&self) -> (u32, u32) {
        let (mut width, mut height) = if self.0.auto_resize {
            self.0.observed_size.get()
        } else {
            (self.0.element.width(), self.0.element.height())
        };
        width = width.clamp(1, self.0.max_texture_dimension);
        height = height.clamp(1, self.0.max_texture_dimension);
        (width, height)
    }

    /// View of the canvas's current surface texture, like JS
    /// `context.getCurrentTexture().createView()`. The texture is presented
    /// automatically after the frame callback returns.
    pub fn current_view(&self) -> wgpu::TextureView {
        let inner = &self.0;
        let (width, height) = self.desired_size();
        if inner.configured.get() != (width, height) {
            if inner.auto_resize {
                inner.element.set_width(width);
                inner.element.set_height(height);
            }
            inner.surface.configure(
                &inner.device,
                &wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: inner.format,
                    color_space: wgpu::SurfaceColorSpace::Auto,
                    width,
                    height,
                    present_mode: wgpu::PresentMode::default(),
                    desired_maximum_frame_latency: 2,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                },
            );
            inner.configured.set((width, height));
        }
        let mut acquired = inner.acquired.borrow_mut();
        if acquired.is_none() {
            *acquired = match inner.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame)
                | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
                _ => None,
            };
        }
        acquired
            .as_ref()
            .expect("failed to get current surface texture")
            .texture
            .create_view(&Default::default())
    }
}

pub struct MultiApp {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// `navigator.gpu.getPreferredCanvasFormat()`.
    pub format: wgpu::TextureFormat,
    /// When true each canvas's drawing buffer resolution follows its
    /// displayed size (the ResizeObserver code in the lesson). Set it
    /// before calling [`MultiApp::canvases`].
    pub auto_resize: bool,
    max_texture_dimension: u32,
    canvases: RefCell<Vec<Canvas>>,
    /// Once-mode re-render callback, installed by [`MultiApp::run`] and
    /// fired by the observers.
    render_hook: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    // Keep the instance alive: on the WebGPU backend, dropping it aborts
    // pending async operations (mapAsync etc.) with
    // "A valid external Instance reference no longer exists".
    instance: wgpu::Instance,
}

impl MultiApp {
    pub async fn new(_title: &str) -> MultiApp {
        let instance = wgpu::Instance::default();
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
        {
            Ok(adapter) => adapter,
            Err(_) => {
                crate::web::fail("need a browser that supports WebGPU");
                panic!("no adapter");
            }
        };
        let (device, queue) = match adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
        {
            Ok(pair) => pair,
            Err(_) => {
                crate::web::fail("need a browser that supports WebGPU");
                panic!("no device");
            }
        };
        // navigator.gpu.getPreferredCanvasFormat() — via js_sys since
        // web-sys's WebGPU bindings are still unstable-gated.
        let navigator = web_sys::window().unwrap().navigator();
        let gpu = js_sys::Reflect::get(navigator.as_ref(), &"gpu".into()).unwrap();
        let get_format =
            js_sys::Reflect::get(&gpu, &"getPreferredCanvasFormat".into()).unwrap();
        let format = match js_sys::Function::from(get_format)
            .call0(&gpu)
            .unwrap()
            .as_string()
            .as_deref()
        {
            Some("rgba8unorm") => wgpu::TextureFormat::Rgba8Unorm,
            _ => wgpu::TextureFormat::Bgra8Unorm,
        };
        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        MultiApp {
            device,
            queue,
            format,
            auto_resize: false,
            max_texture_dimension,
            canvases: RefCell::new(Vec::new()),
            render_hook: Rc::new(RefCell::new(None)),
            instance,
        }
    }

    /// Wrap every `<canvas>` element on the page in a [`Canvas`], in
    /// document order. `_native_sizes` is only used by the native backend
    /// (where there is no page to put canvases on).
    pub fn canvases(&self, _native_sizes: &[(u32, u32)]) -> Vec<Canvas> {
        let document = web_sys::window().unwrap().document().unwrap();
        let list = document.query_selector_all("canvas").unwrap();
        let mut canvases = Vec::new();
        for i in 0..list.length() {
            let element: HtmlCanvasElement = list.get(i).unwrap().dyn_into().unwrap();
            canvases.push(self.wrap_canvas(element));
        }
        self.canvases.borrow_mut().extend(canvases.iter().cloned());
        canvases
    }

    fn wrap_canvas(&self, element: HtmlCanvasElement) -> Canvas {
        let surface = self
            .instance
            .create_surface(wgpu::SurfaceTarget::Canvas(element.clone()))
            .expect("failed to create surface from canvas");
        let canvas = Canvas(Rc::new(CanvasInner {
            element: element.clone(),
            surface,
            device: self.device.clone(),
            format: self.format,
            auto_resize: self.auto_resize,
            max_texture_dimension: self.max_texture_dimension,
            observed_size: Cell::new((element.width(), element.height())),
            configured: Cell::new((0, 0)),
            visible: Cell::new(false),
            acquired: RefCell::new(None),
        }));

        // ResizeObserver: track the canvas's displayed size in device pixels.
        if self.auto_resize {
            let inner = canvas.0.clone();
            let render_hook = self.render_hook.clone();
            let observer_cb =
                Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
                    for entry in entries.iter() {
                        let entry: web_sys::ResizeObserverEntry = entry.dyn_into().unwrap();
                        let (width, height);
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
                        inner.observed_size.set((width, height));
                    }
                    if let Some(render) = render_hook.borrow().clone() {
                        render();
                    }
                });
            let observer =
                web_sys::ResizeObserver::new(observer_cb.as_ref().unchecked_ref()).unwrap();
            observer.observe(&element);
            observer_cb.forget();
        }

        // IntersectionObserver: track whether the canvas is on screen.
        {
            let inner = canvas.0.clone();
            let render_hook = self.render_hook.clone();
            let observer_cb =
                Closure::<dyn FnMut(js_sys::Array)>::new(move |entries: js_sys::Array| {
                    for entry in entries.iter() {
                        let entry: web_sys::IntersectionObserverEntry =
                            entry.dyn_into().unwrap();
                        inner.visible.set(entry.is_intersecting());
                    }
                    if let Some(render) = render_hook.borrow().clone() {
                        render();
                    }
                });
            let observer =
                web_sys::IntersectionObserver::new(observer_cb.as_ref().unchecked_ref())
                    .unwrap();
            observer.observe(&element);
            observer_cb.forget();
        }

        canvas
    }

    pub fn run(self, mode: RenderMode, frame_fn: impl FnMut(&MultiFrame) + 'static) {
        let app = Rc::new(self);
        let frame_fn = Rc::new(RefCell::new(frame_fn));
        let start_time = web_sys::window().unwrap().performance().unwrap().now();

        let render = {
            let app = app.clone();
            let frame_fn = frame_fn.clone();
            move || {
                let now = web_sys::window().unwrap().performance().unwrap().now();
                (frame_fn.borrow_mut())(&MultiFrame {
                    device: &app.device,
                    queue: &app.queue,
                    format: app.format,
                    time: (now - start_time) / 1000.0,
                });
                // Present every canvas the frame callback rendered to.
                for canvas in app.canvases.borrow().iter() {
                    if let Some(frame) = canvas.0.acquired.borrow_mut().take() {
                        app.queue.present(frame);
                    }
                }
            }
        };
        let render: Rc<dyn Fn()> = Rc::new(render);

        if mode == RenderMode::Once {
            // Re-render when a canvas resizes or scrolls into view, or when
            // a GUI setting changes.
            *app.render_hook.borrow_mut() = Some(render.clone());
            let render2 = render.clone();
            crate::settings::set_redraw_hook(move || render2());
            render();
        } else {
            // requestAnimationFrame loop.
            let raf: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
            let raf2 = raf.clone();
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
