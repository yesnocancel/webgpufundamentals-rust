//! Native backend for examples that draw to *several* canvases at once
//! (the multiple-canvases lesson). In the browser each [`Canvas`] wraps a
//! real `<canvas>` element with its own surface; natively there is no such
//! thing as many surfaces in one window, so each [`Canvas`] is an offscreen
//! texture and the harness composites them into one window as a scrollable
//! grid (scroll with the mouse wheel), or into the test PNG in test mode.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::{MultiFrame, RenderMode};

/// One "canvas": natively, an offscreen texture of a fixed size plus the
/// resources needed to composite it into the window.
#[derive(Clone)]
pub struct Canvas(Rc<CanvasInner>);

struct CanvasInner {
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    visible: Cell<bool>,
    rect_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Canvas {
    /// The canvas's drawing buffer size, like JS `canvas.width`.
    pub fn width(&self) -> u32 {
        self.0.width
    }

    /// The canvas's drawing buffer size, like JS `canvas.height`.
    pub fn height(&self) -> u32 {
        self.0.height
    }

    /// Whether the canvas currently intersects the viewport — the
    /// `IntersectionObserver` visibility set from the lesson. Natively this
    /// is whether the canvas's grid cell intersects the window.
    pub fn is_visible(&self) -> bool {
        self.0.visible.get()
    }

    /// View of the canvas's current texture to render to, like JS
    /// `context.getCurrentTexture().createView()`.
    pub fn current_view(&self) -> wgpu::TextureView {
        self.0.view.clone()
    }
}

pub struct MultiApp {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The texture format the canvases use, like
    /// `navigator.gpu.getPreferredCanvasFormat()`.
    pub format: wgpu::TextureFormat,
    /// Browser only: when true each canvas's drawing buffer resolution
    /// follows its displayed size (the ResizeObserver code in the lesson).
    /// Natively canvases keep the size they were created with.
    pub auto_resize: bool,
    #[allow(dead_code)]
    instance: wgpu::Instance,
    title: String,
    test: Option<(u32, u32, std::path::PathBuf)>,
    canvases: RefCell<Vec<Canvas>>,
    composite_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
}

/// Spacing of the native grid, roughly matching the lesson's CSS
/// (1em padding + 1em margin around each product card).
const GRID_GAP: f32 = 24.0;

impl MultiApp {
    pub async fn new(title: &str) -> MultiApp {
        crate::settings::load_settings_from_env();
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no compatible GPU adapter found");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to create GPU device");

        let test = std::env::var("WGPU_FUN_TEST").ok().map(|_| {
            let size = std::env::var("WGPU_FUN_TEST_SIZE").unwrap_or_default();
            let (w, h) = size
                .split_once('x')
                .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                .unwrap_or((640, 480));
            let out_path = std::env::var("WGPU_FUN_TEST_OUT")
                .unwrap_or_else(|_| {
                    let name = std::env::args()
                        .next()
                        .and_then(|p| {
                            std::path::Path::new(&p)
                                .file_stem()
                                .map(|s| s.to_string_lossy().into_owned())
                        })
                        .unwrap_or_else(|| "example".to_string());
                    format!("test_frames/{name}.png")
                })
                .into();
            (w, h, out_path)
        });

        let format = if test.is_some() {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Bgra8Unorm
        };

        // Pipeline that composites the canvas textures into the window /
        // test target as quads.
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("canvas composite shaders"),
            source: wgpu::ShaderSource::Wgsl(
                /* wgsl */ r#"
      // x, y, w, h of the canvas in normalized (0..1, y-down) window coords
      @group(0) @binding(0) var<uniform> rect: vec4f;
      @group(0) @binding(1) var t: texture_2d<f32>;
      @group(0) @binding(2) var s: sampler;

      struct VSOut {
        @builtin(position) position: vec4f,
        @location(0) uv: vec2f,
      };

      @vertex fn vs(@builtin(vertex_index) i: u32) -> VSOut {
        let corners = array(
          vec2f(0, 0), vec2f(1, 0), vec2f(0, 1),
          vec2f(0, 1), vec2f(1, 0), vec2f(1, 1));
        let c = corners[i];
        let p = rect.xy + c * rect.zw;
        var out: VSOut;
        out.position = vec4f(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0);
        out.uv = c;
        return out;
      }

      @fragment fn fs(v: VSOut) -> @location(0) vec4f {
        return textureSample(t, s, v.uv);
      }
    "#
                .into(),
            ),
        });
        let composite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("canvas composite pipeline"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: None,
                    compilation_options: Default::default(),
                    targets: &[Some(format.into())],
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        MultiApp {
            device,
            queue,
            format,
            auto_resize: false,
            instance,
            title: title.to_string(),
            test,
            canvases: RefCell::new(Vec::new()),
            composite_pipeline,
            sampler,
        }
    }

    /// Get one [`Canvas`] per entry of `sizes`. In the browser this wraps
    /// the page's existing `<canvas>` elements (and `sizes` only supplies
    /// the native fallback); natively it creates offscreen canvases of the
    /// given pixel sizes.
    pub fn canvases(&self, sizes: &[(u32, u32)]) -> Vec<Canvas> {
        let canvases: Vec<Canvas> = sizes
            .iter()
            .map(|&(width, height)| {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("canvas texture"),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let rect_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("canvas composite rect"),
                    size: 16,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("canvas composite bind group"),
                    layout: &self.composite_pipeline.get_bind_group_layout(0),
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: rect_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                });
                Canvas(Rc::new(CanvasInner {
                    view,
                    width,
                    height,
                    visible: Cell::new(false),
                    rect_buffer,
                    bind_group,
                }))
            })
            .collect();
        self.canvases.borrow_mut().extend(canvases.iter().cloned());
        canvases
    }

    /// Lay the canvases out as a wrapping grid (like the inline-block
    /// product cards in the lesson), update each canvas's visibility, and
    /// return each canvas's (x, y) in view coordinates.
    fn layout(&self, view_width: f32, view_height: f32, scroll_y: f32) -> Vec<(f32, f32)> {
        let canvases = self.canvases.borrow();
        let mut positions = Vec::with_capacity(canvases.len());
        let mut x = GRID_GAP;
        let mut y = GRID_GAP - scroll_y;
        let mut row_height = 0.0f32;
        for canvas in canvases.iter() {
            let (w, h) = (canvas.width() as f32, canvas.height() as f32);
            if x + w > view_width - GRID_GAP && x > GRID_GAP {
                x = GRID_GAP;
                y += row_height + GRID_GAP;
                row_height = 0.0;
            }
            canvas
                .0
                .visible
                .set(y + h > 0.0 && y < view_height && x < view_width);
            positions.push((x, y));
            x += w + GRID_GAP;
            row_height = row_height.max(h);
        }
        positions
    }

    /// Composite every visible canvas into `target` (the window's surface
    /// texture or the test texture).
    fn composite(&self, target: &wgpu::TextureView, view_width: f32, view_height: f32, scroll_y: f32) {
        let positions = self.layout(view_width, view_height, scroll_y);
        let canvases = self.canvases.borrow();
        for (canvas, &(x, y)) in canvases.iter().zip(positions.iter()) {
            if !canvas.is_visible() {
                continue;
            }
            let rect = [
                x / view_width,
                y / view_height,
                canvas.width() as f32 / view_width,
                canvas.height() as f32 / view_height,
            ];
            self.queue
                .write_buffer(&canvas.0.rect_buffer, 0, bytemuck_cast(&rect));
        }
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("canvas composite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.15,
                            g: 0.15,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.composite_pipeline);
            for canvas in canvases.iter() {
                if !canvas.is_visible() {
                    continue;
                }
                pass.set_bind_group(0, &canvas.0.bind_group, &[]);
                pass.draw(0..6, 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
    }

    /// Hand control to the render loop. In test mode this renders a few
    /// frames offscreen, composites the canvas grid and writes a PNG.
    pub fn run(self, mode: RenderMode, frame_fn: impl FnMut(&MultiFrame) + 'static) {
        if self.test.is_some() {
            self.run_test(mode, frame_fn);
            return;
        }
        let event_loop = EventLoop::new().expect("failed to create event loop");
        let mut handler = MultiWinitApp {
            app: self,
            mode,
            frame_fn: Box::new(frame_fn),
            window: None,
            surface: None,
            start_time: Instant::now(),
            scroll_y: 0.0,
        };
        event_loop.run_app(&mut handler).expect("event loop error");
    }

    fn run_test(self, mode: RenderMode, mut frame_fn: impl FnMut(&MultiFrame)) {
        let (width, height, out_path) = self.test.clone().unwrap();
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test render target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());

        // Compute visibility for the test viewport before the example
        // renders (so `is_visible()` works like the browser's
        // IntersectionObserver would).
        self.layout(width as f32, height as f32, 0.0);

        let times: &[f64] = match mode {
            RenderMode::Once => &[0.0],
            RenderMode::Continuous => &[0.0, 0.25, 0.5],
        };
        for &time in times {
            frame_fn(&MultiFrame {
                device: &self.device,
                queue: &self.queue,
                format: self.format,
                time,
            });
        }
        self.composite(&view, width as f32, height as f32, 0.0);
        crate::native::write_texture_png(&self.device, &self.queue, &target, width, height, &out_path);
        println!("TEST-OK {}", out_path.display());
    }
}

// bytemuck is not a wgpu_fun dependency; a [f32; 4] is plain-old-data so a
// manual byte view is fine here.
fn bytemuck_cast(v: &[f32; 4]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, 16) }
}

struct MultiWinitApp {
    app: MultiApp,
    mode: RenderMode,
    frame_fn: Box<dyn FnMut(&MultiFrame)>,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    start_time: Instant,
    scroll_y: f32,
}

impl MultiWinitApp {
    fn configure_surface(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &self.surface) else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        surface.configure(
            &self.app.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.app.format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::default(),
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            },
        );
    }
}

impl ApplicationHandler for MultiWinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(&self.app.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(800, 600)),
                )
                .expect("failed to create window"),
        );
        let surface = self
            .app
            .instance
            .create_surface(window.clone())
            .expect("failed to create surface");
        self.window = Some(window.clone());
        self.surface = Some(surface);
        self.configure_surface();
        crate::settings::set_redraw_hook(move || window.request_redraw());
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 60.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.scroll_y = (self.scroll_y - dy).max(0.0);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(_) => {
                self.configure_surface();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(window) = self.window.clone() else {
                    return;
                };
                if self.surface.is_none() {
                    return;
                }
                let size = window.inner_size();
                if size.width == 0 || size.height == 0 {
                    return;
                }
                use wgpu::CurrentSurfaceTexture as Cst;
                let mut retried = false;
                let frame = loop {
                    match self.surface.as_ref().unwrap().get_current_texture() {
                        Cst::Success(frame) | Cst::Suboptimal(frame) => break frame,
                        Cst::Timeout | Cst::Occluded => return,
                        Cst::Outdated | Cst::Lost | Cst::Validation => {
                            if retried {
                                return;
                            }
                            retried = true;
                            self.configure_surface();
                        }
                    }
                };
                let (vw, vh) = (size.width as f32, size.height as f32);
                // Update visibility before the example renders, like the
                // browser's IntersectionObserver.
                self.app.layout(vw, vh, self.scroll_y);
                (self.frame_fn)(&MultiFrame {
                    device: &self.app.device,
                    queue: &self.app.queue,
                    format: self.app.format,
                    time: self.start_time.elapsed().as_secs_f64(),
                });
                let view = frame.texture.create_view(&Default::default());
                self.app.composite(&view, vw, vh, self.scroll_y);
                window.pre_present_notify();
                self.app.queue.present(frame);
                if self.mode == RenderMode::Continuous {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
