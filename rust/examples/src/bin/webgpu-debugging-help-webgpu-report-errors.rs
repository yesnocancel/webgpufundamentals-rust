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
    let Ok((device, _queue)) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
    else {
        fail("need a browser that supports WebGPU");
        return;
    };

    // like JS `device.addEventListener('uncapturederror', ...)`.
    // Note: in the browser, uncaptured errors arrive asynchronously and this
    // demo shows no error because nothing "pumps" WebGPU. Native wgpu calls
    // the handler synchronously, at the call that errored, so natively the
    // error shows up even in this "broken" version. (Without a handler,
    // native wgpu would panic instead.)
    device.on_uncaptured_error(Arc::new(|error: wgpu::Error| {
        log(&format!("{error}"));
    }));

    let _module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            r#"
      this shader won't compile
    "#
            .into(),
        ),
    });

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
