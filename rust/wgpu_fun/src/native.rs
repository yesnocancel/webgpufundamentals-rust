use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::{Frame, RenderMode};

/// Native entry point: block on the async run function.
pub fn start(fut: impl std::future::Future<Output = ()>) {
    env_logger::init();
    pollster::block_on(fut);
}

pub struct App {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The texture format frames render to. Fixed per platform so pipelines
    /// can be created before the window exists.
    pub format: wgpu::TextureFormat,
    /// When true the drawing buffer resolution follows the window/canvas
    /// size. False by default, like a raw `<canvas>`. (Only observable in
    /// the browser; native surfaces always match the window.)
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
    /// Browser-only low-res-canvas trick; ignored natively (surfaces must
    /// match the window size). Kept so examples compile on both platforms.
    pub resize_divisor: u32,
    /// Usage flags for the surface's textures (JS `context.configure`'s
    /// `usage`). Defaults to RENDER_ATTACHMENT.
    pub usage: wgpu::TextureUsages,
    instance: wgpu::Instance,
    title: String,
    test: Option<TestConfig>,
}

struct TestConfig {
    width: u32,
    height: u32,
    out_path: std::path::PathBuf,
}

impl App {
    pub async fn new(title: &str) -> App {
        Self::new_with_features(title, wgpu::Features::empty()).await
    }

    /// Like [`App::new`], but enables the given **optional** features on the
    /// device if (and only if) the adapter supports them — the JS
    /// `adapter.features.has(...)` pattern. Check `app.device.features()`
    /// to see which ones you actually got.
    pub async fn new_with_features(title: &str, optional_features: wgpu::Features) -> App {
        crate::settings::load_settings_from_env();
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("no compatible GPU adapter found");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: adapter.features() & optional_features,
                ..Default::default()
            })
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
            TestConfig { width: w, height: h, out_path }
        });

        let format = if test.is_some() {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Bgra8Unorm
        };

        App {
            device,
            queue,
            format,
            auto_resize: false,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            alpha_mode_fn: None,
            resize_divisor: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            instance,
            title: title.to_string(),
            test,
        }
    }

    fn current_alpha_mode(&self) -> wgpu::CompositeAlphaMode {
        match &self.alpha_mode_fn {
            Some(f) => f(),
            None => self.alpha_mode,
        }
    }

    /// Hand control to the render loop. In test mode this renders a few
    /// frames offscreen, writes a PNG and returns instead.
    pub fn run(self, mode: RenderMode, frame_fn: impl FnMut(&Frame) + 'static) {
        if self.test.is_some() {
            self.run_test(mode, frame_fn);
            return;
        }
        let event_loop = EventLoop::new().expect("failed to create event loop");
        let mut handler = WinitApp {
            app: self,
            mode,
            frame_fn: Box::new(frame_fn),
            window: None,
            surface: None,
            configured_alpha_mode: None,
            start_time: Instant::now(),
            cursor: (0.0, 0.0),
        };
        event_loop.run_app(&mut handler).expect("event loop error");
    }

    fn run_test(self, mode: RenderMode, mut frame_fn: impl FnMut(&Frame)) {
        let test = self.test.as_ref().unwrap();
        let (width, height) = (test.width, test.height);
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("test render target"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: self.usage | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());

        // Render a few frames at fixed timestamps for animated examples so
        // the PNG shows a mid-animation state, one frame for static ones.
        let times: &[f64] = match mode {
            RenderMode::Once => &[0.0],
            RenderMode::Continuous => &[0.0, 0.25, 0.5],
        };
        for &time in times {
            frame_fn(&Frame {
                device: &self.device,
                queue: &self.queue,
                view: &view,
                format: self.format,
                width,
                height,
                time,
            });
        }

        // Read the last frame back and write it as a PNG.
        write_texture_png(&self.device, &self.queue, &texture, width, height, &test.out_path);
        println!("TEST-OK {}", test.out_path.display());
    }
}

/// Read an Rgba8 texture back and write it as a PNG (test mode).
pub(crate) fn write_texture_png(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    out_path: &std::path::Path,
) {
    let bytes_per_row = (width * 4 + 255) & !255;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map failed"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll failed");

    let data = slice.get_mapped_range().expect("failed to map readback");
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * bytes_per_row) as usize;
        pixels.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    drop(data);

    if let Some(dir) = out_path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    let file = std::fs::File::create(out_path).expect("failed to create test PNG");
    let mut png_encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    png_encoder.set_color(png::ColorType::Rgba);
    png_encoder.set_depth(png::BitDepth::Eight);
    png_encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&pixels))
        .expect("failed to write test PNG");
}

struct WinitApp {
    app: App,
    mode: RenderMode,
    frame_fn: Box<dyn FnMut(&Frame)>,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    configured_alpha_mode: Option<wgpu::CompositeAlphaMode>,
    start_time: Instant,
    cursor: (f32, f32),
}

impl WinitApp {
    fn configure_surface(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &self.surface) else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let alpha_mode = self.app.current_alpha_mode();
        surface.configure(
            &self.app.device,
            &wgpu::SurfaceConfiguration {
                usage: self.app.usage,
                format: self.app.format,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::default(),
                desired_maximum_frame_latency: 2,
                alpha_mode,
                view_formats: vec![],
            },
        );
        self.configured_alpha_mode = Some(alpha_mode);
    }
}

impl ApplicationHandler for WinitApp {
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
        let caps = surface.get_capabilities(
            // The adapter is gone by now; capabilities were effectively chosen
            // when App::new picked the format. Check support at configure time.
            &pollster::block_on(self.app.instance.request_adapter(
                &wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                },
            ))
            .expect("no adapter for surface"),
        );
        if !caps.formats.contains(&self.app.format) {
            log::warn!(
                "surface does not support {:?} (supported: {:?})",
                self.app.format,
                caps.formats
            );
        }
        self.window = Some(window.clone());
        self.surface = Some(surface);
        self.configure_surface();
        crate::settings::set_redraw_hook(move || window.request_redraw());
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
                crate::events::push_event(crate::PointerEvent::Move {
                    x: self.cursor.0,
                    y: self.cursor.1,
                });
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    winit::event::MouseButton::Left => 0,
                    winit::event::MouseButton::Middle => 1,
                    winit::event::MouseButton::Right => 2,
                    _ => 3,
                };
                let (x, y) = self.cursor;
                crate::events::push_event(match state {
                    winit::event::ElementState::Pressed => {
                        crate::PointerEvent::Down { x, y, button }
                    }
                    winit::event::ElementState::Released => {
                        crate::PointerEvent::Up { x, y, button }
                    }
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => (x * -100.0, y * -100.0),
                    winit::event::MouseScrollDelta::PixelDelta(p) => {
                        (-p.x as f32, -p.y as f32)
                    }
                };
                crate::events::push_event(crate::PointerEvent::Wheel { delta_x, delta_y });
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
                if self.configured_alpha_mode != Some(self.app.current_alpha_mode()) {
                    self.configure_surface();
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
                let view = frame.texture.create_view(&Default::default());
                (self.frame_fn)(&Frame {
                    device: &self.app.device,
                    queue: &self.app.queue,
                    view: &view,
                    format: self.app.format,
                    width: size.width,
                    height: size.height,
                    time: self.start_time.elapsed().as_secs_f64(),
                });
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
