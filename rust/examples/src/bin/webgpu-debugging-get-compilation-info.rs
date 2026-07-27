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

    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let code = /* wgsl */ r#"
      // This function
      // calls a function
      // that does not
      // exist.

      fn foo() -> vec3f {
        return someFunction(1, 2);
      }
    "#;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(code.into()),
    });
    let error = error_scope.pop().await;
    if error.is_some() {
        let info = module.get_compilation_info().await;

        // Split the code into lines
        let mut lines: Vec<String> = code.split('\n').map(String::from).collect();

        // Sort the messages by line numbers in reverse order
        // so that as we insert the messages they won't affect
        // the line numbers.
        let mut msgs: Vec<_> = info
            .messages
            .iter()
            .filter_map(|msg| msg.location.map(|loc| (loc, msg.message.clone())))
            .collect();
        msgs.sort_by(|a, b| b.0.line_number.cmp(&a.0.line_number));

        // Insert the error messages between lines
        for (loc, message) in msgs {
            lines.insert(loc.line_number as usize, message);
            lines.insert(
                loc.line_number as usize,
                format!(
                    "{}{}",
                    " ".repeat(loc.line_position as usize - 1),
                    "^".repeat(loc.length as usize),
                ),
            );
        }

        log(&lines.join("\n"));
    }
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
