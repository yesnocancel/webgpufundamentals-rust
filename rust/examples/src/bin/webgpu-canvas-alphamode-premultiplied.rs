use wgpu_fun::{App, Frame, RenderMode};

async fn run() {
    let mut app = App::new("WebGPU Canvas alphaMode premultiplied").await;
    app.auto_resize = true;
    app.alpha_mode = wgpu::CompositeAlphaMode::PreMultiplied;

    app.run(RenderMode::Once, move |frame: &Frame| {
        // read the settings the GUI on the page sets
        let premultiply = wgpu_fun::setting_bool("premultiply", false);
        let alpha = wgpu_fun::setting_f64("alpha", 0.01);
        let color = [
            wgpu_fun::setting_f64("color0", 1.0),
            wgpu_fun::setting_f64("color1", 0.0),
            wgpu_fun::setting_f64("color2", 0.0),
        ];

        let mut clear_value = [0.0, 0.0, 0.0, alpha];
        if premultiply {
            // premultiply the colors by the alpha
            clear_value[0] = color[0] * alpha;
            clear_value[1] = color[1] * alpha;
            clear_value[2] = color[2] * alpha;
        } else {
            // use un-premultiplied colors
            clear_value[0] = color[0];
            clear_value[1] = color[1];
            clear_value[2] = color[2];
        }

        let mut encoder = frame
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear encoder"),
            });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("our basic canvas renderPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_value[0],
                            g: clear_value[1],
                            b: clear_value[2],
                            a: clear_value[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
        }

        let command_buffer = encoder.finish();
        frame.queue.submit([command_buffer]);
    });
}

fn main() {
    wgpu_fun::start(run());
}
