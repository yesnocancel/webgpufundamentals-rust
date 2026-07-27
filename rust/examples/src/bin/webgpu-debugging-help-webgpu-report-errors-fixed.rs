use std::sync::Arc;

use wgpu_fun::log;

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

    // like JS `device.addEventListener('uncapturederror', ...)`.
    // (Without a handler, native wgpu would panic instead.)
    device.on_uncaptured_error(Arc::new(|error: wgpu::Error| {
        log(&format!("{error}"));
    }));

    let _module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            /* wgsl */ r#"
      this shader won't compile
    "#
            .into(),
        ),
    });

    // pump WebGPU
    queue.submit([]);

    log("--done--");
}

fn fail(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    wgpu_fun::fail(msg);
    #[cfg(not(target_arch = "wasm32"))]
    panic!("{msg}");
}

fn main() {
    wgpu_fun::start(main_async());
}
