Title: WebGPU Optional Features and Limits
Description: Optional Features
TOC: Optional Features and Limits

WebGPU has a bunch of optional features and limits. Let's go over how to check them
and request them.

When you request an adapter with

```rust
let instance = wgpu::Instance::default();
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions::default())
    .await
    .expect("this system does not support WebGPU");
```

The adapter has a set of limits, returned by `adapter.limits()` as a plain
`wgpu::Limits` struct, and a set of feature flags, returned by
`adapter.features()` as a `wgpu::Features` bitflag set. For example

```rust
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions::default())
    .await
    .expect("this system does not support WebGPU");
println!("{}", adapter.limits().max_color_attachments);
```

Might print `8` to the terminal meaning the adapter supports a maximum
of 8 color attachments.

Here is a list of all the limits, including the limits of your default adapter
as well as the minimum required limits.

<div class="webgpu_center data-table limits" data-diagram="limits"></div>

The minimum limits are the limits you can count on all devices that support WebGPU
to have.

There is also a list of optional features. For example, you could view them
like this

```rust
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions::default())
    .await
    .expect("this system does not support WebGPU");
println!("{}", adapter.features());
```

which might print something like `TEXTURE_COMPRESSION_ASTC | TEXTURE_COMPRESSION_BC` telling
you those features are available if you request them. The names are constants
on `wgpu::Features` and match the JavaScript API's feature names — where
JavaScript has `'texture-compression-bc'`, wgpu has
`wgpu::Features::TEXTURE_COMPRESSION_BC`.

Here is the list of features available on your default adapter.

<div class="webgpu_center data-table features" data-diagram="features"></div>

> Note: You can check all of your system's adapter's features and limits at [webgpureport.org](https://webgpureport.org).

> Note: When running natively, wgpu also exposes *extra* features and limits
> that go beyond the WebGPU spec (their docs mark which are which). If you use
> those, your code will still run natively but won't be able to run in a
> browser.

## Requesting limits and features

By default, when you request a device, you get the minimum limits
(the right column above) and you get no optional features. The
hope is, if you stay under the minimum limits, then your app will
run on all devices that support WebGPU.

But, given the available limits and features listed on the adapter,
you can request them when you call `request_device` by
passing your desired limits as `required_limits` and your desired features as `required_features`. For example

```rust
const K1_GIG: u64 = 1024 * 1024 * 1024;
let adapter = instance
    .request_adapter(&wgpu::RequestAdapterOptions::default())
    .await
    .expect("this system does not support WebGPU");
let (device, queue) = adapter
    .request_device(&wgpu::DeviceDescriptor {
      required_limits: wgpu::Limits {
        max_buffer_size: K1_GIG,
        ..Default::default()
      },
      required_features: wgpu::Features::FLOAT32_FILTERABLE,
      ..Default::default()
    })
    .await
    .expect("could not get the required limits and features");
```

Above we're requesting to be able to use buffers of up to 1gig and to be able to use filterable float32
textures (for example `Rgba32Float` with `min_filter` set to `FilterMode::Linear`, which by default can only be used with `FilterMode::Nearest`).
Note that `wgpu::Limits::default()` is the set of minimum limits from the
table above, so with struct update syntax we only raise the one limit we care
about.

If either of those requests can not be met `request_device` will fail (return an `Err`).

## Don't request everything

It might be temping to ask for all the limits and features and then check for the ones you need.

Example:

```rust
//
// BAD!!! ?
//
async fn main_async() {
  let instance = wgpu::Instance::default();
  let Ok(adapter) = instance
      .request_adapter(&wgpu::RequestAdapterOptions::default())
      .await
  else {
    fail("need webgpu");
    return;
  };
  let Ok((device, queue)) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        required_features: adapter.features(),
        ..Default::default()
      })
      .await
  else {
    fail("need webgpu");
    return;
  };

  let can_use_128k_uniform_buffers =
      device.limits().max_uniform_buffer_binding_size >= 128 * 1024;
  let can_store_to_bgra8unorm =
      device.features().contains(wgpu::Features::BGRA8UNORM_STORAGE);
  let can_indirect_first_instance =
      device.features().contains(wgpu::Features::INDIRECT_FIRST_INSTANCE);
}
```

This seems like a simple and clear way to check for limits and features[^objliketoobj]. The
problem with this pattern is you might be accidentally exceeding limits and not
know it. For example lets say you created an `Rgba32Float` texture and filtered it
with `linear` filtering.
It would magically just work on your desktop machine because you happened to have
enabled it.

[^objliketoobj]: It's even simpler in Rust than in JavaScript. In the
JavaScript API you can't pass `adapter.limits` straight to `requiredLimits` —
for esoteric Web-spec reasons (the limits live as getters on the object's
prototype and are not copied when converted to a
`record<DOMString, GPUSize64>`), so you have to copy them into a plain object
yourself. In wgpu, `adapter.limits()` returns a plain `Limits` struct that
you can pass directly, which makes this tempting pattern even more tempting.

On the user's phone, your program fails mysteriously because the `FLOAT32_FILTERABLE`
feature didn't exist and you happened to be using it without realizing that it's
an optional feature.

Or you might allocate a buffer larger the minimum `max_buffer_size` and again
not be aware you went over the limit. You ship and a bunch of users can't run
your page.

## Recommended Way to Request Features and Limits

The recommended way to use features and limits is to decide on what you absolutely
must have and only request those limits.

For example

```rust
  let Ok(adapter) = instance
      .request_adapter(&wgpu::RequestAdapterOptions::default())
      .await
  else {
    fail("need webgpu");
    return;
  };

  let can_use_128k_uniform_buffers =
      adapter.limits().max_uniform_buffer_binding_size >= 128 * 1024;
  let can_store_to_bgra8unorm =
      adapter.features().contains(wgpu::Features::BGRA8UNORM_STORAGE);
  let can_indirect_first_instance =
      adapter.features().contains(wgpu::Features::INDIRECT_FIRST_INSTANCE);

  // if we absolutely need one or more of these features then fail now if they
  // are not available
  if !can_use_128k_uniform_buffers {
    fail("Sorry, your device is probably too old or underpowered");
    return;
  }

  // Request the available features and limits we need
  let mut required_features = wgpu::Features::empty();
  if can_store_to_bgra8unorm {
    required_features |= wgpu::Features::BGRA8UNORM_STORAGE;
  }
  if can_indirect_first_instance {
    required_features |= wgpu::Features::INDIRECT_FIRST_INSTANCE;
  }
  let (device, queue) = adapter
      .request_device(&wgpu::DeviceDescriptor {
        required_features,
        required_limits: wgpu::Limits {
          max_uniform_buffer_binding_size: 128 * 1024,
          ..Default::default()
        },
        ..Default::default()
      })
      .await
      .expect("failed to create a device");
```

Doing it this way, if you happen to ask for a Uniform buffer larger than 128k you'll get an error.
Similarly if you happen to try to use a feature you didn't request you'll get an error
(the device validates against exactly what you requested — no more, no less, regardless
of what the adapter could have provided).
You can then make a conscience decision if you want to increase your required limits (and therefore
refuse to run on more devices) or if you want to keep the limits, or if you want to structure
your code to do different things if the features or limits are or are not available.

<!-- keep this at the bottom of the article -->
<link rel="stylesheet" href="webgpu-limits-and-features.css">
<script type="module" src="webgpu-limits-and-features.js"></script>



