Title: WebGPU Post Processing - 3d lookup table (LUT)
Description: 3D lookup table (LUT)
TOC: 3D Lookup Table (LUT)

This is article is the 3nd in a short series
about image adjustments. Each one builds on the previous lesson so you may find
them easiest to understand by reading them in order.

1. [Image Adjustments](webgpu-image-adjustments.html)
2. [1D Lookup Tables](webgpu-1dlut.html)
3. [3D Lookup Tables](webgpu-3dlut.html) ⬅ you are here


In the last article we went over [gradient maps](webgpu-1dlut.html), which we could
also call a 1d lookup table or 1D-LUT for short. Our 1D-LUTs were n pixels wide and 1
tall. A 3D-LUT is the same idea but in 3D.

How it works is we make a cube of colors. Then we index the cube using the colors of our source image. For each pixel in the original image we look up a position in the cube based on the red, green, and blue colors of the original pixel. The value we pull out of the 3D-LUT is the new color.

In code we might do it like this. Imagine the colors are specified in integers from 0 to 255 and we have a large 3 dimensional array 256x256x256 in size. Then to translate a color through the look up table we'd do this

```js
    const newColor = lut[origColor.red][origColor.green][origColor.bue];
```

Of course a 256x256x256 array would be rather large but as we pointed out in [the article on textures](webgpu-textures.html), textures are referenced from values of 0.0 to 1.0 regardless of the dimensions of the texture.

Let's imagine an 8x8x8 cube.

<div class="webgpu_center"><img src="resources/images/3dlut-rgb.svg" class="noinvertdark" style="width: 500px"></div>

First we might fill in the corners with 0,0,0 corner being pure black, the opposite 1,1,1 corner pure white. 1,0,0 being pure <span style="color:red;">red</span>. 0,1,0 being pure <span style="color:green;">green</span> and 0,0,1 being <span style="color:blue;">blue</span>. 

<div class="webgpu_center"><img src="resources/images/3dlut-axis.svg" class="noinvertdark" style="width: 500px"></div>

We'd add in the colors down each axis.

<div class="webgpu_center"><img src="resources/images/3dlut-edges.svg" class="noinvertdark" style="width: 500px"></div>

And the colors on edges that use 2 or more channels.

<div class="webgpu_center"><img src="resources/images/3dlut-standard.svg" class="noinvertdark" style="width: 500px"></div>

And finally fill in all the colors in between. This is an "identity" 3D-LUT. It produces the exact same output as input. If you look up a color you'll get the same color out.

<div class="webgpu_center"><object type="image/svg+xml" data="resources/images/3dlut-standard-lookup.svg" class="noinvertdark" data-diagram="lookup" style="width: 600px"></object></div>

If we change the cube to shades of amber though then as we look up colors, we look up the same locations in the 3D lookup table but they produce different output.

<div class="webgpu_center"><object type="image/svg+xml" data="resources/images/3dlut-amber-lookup.svg" class="noinvertdark" data-diagram="lookup" style="width: 600px"></object></div>

Using this technique, by supplying a different lookup table we can apply all kinds of effects. Basically any effect that can be computed based only on a single color input. Those effects include all the ones we made in the previous articles. Adjusting hue, contrast, saturation, color cast, tint, brightness, exposure, levels, curves, posterization, shadows, highlights, and many others. Even better they can all be combined into a single look up table.

Here's the WGSL we need. It's very similar to the `apply1DLUT` function

```wgsl
fn apply1DLUT(
    color: vec3f,
    lut: texture_2d<f32>,
    smp: sampler) -> vec3f {
  let l = luminance(color);
  let width = f32(textureDimensions(lut, 0).x);
  let range = (width - 1) / width;
  let u = 0.5 / width + l * range;
  return textureSample(lut, smp, vec2f(u, 0.5)).rgb;
}

+fn apply3DLUT(
+    color: vec3f,
+    lut: texture_3d<f32>,
+    smp: sampler) -> vec3f {
+  let size = vec3f(textureDimensions(lut, 0));
+  let range = (size - 1) / size;
+  let uvw = 0.5 / size + color * range;
+  return textureSample(lut, smp, uvw).rgb;
+}
```

Let's apply it to our shaders. While we're at lets remove the all the other adjustments.

```wgsl
struct Uniforms {
-  brightness: f32,
-  contrast: f32,
  lutAmount: f32,
};

@group(0) @binding(0) var postTexture2d: texture_2d<f32>;
@group(0) @binding(1) var postSampler: sampler;
@group(0) @binding(2) var<uniform> uni: Uniforms;
-@group(1) @binding(0) var lut: texture_2d<f32>;
+@group(1) @binding(0) var lut: texture_3d<f32>;
@group(1) @binding(1) var lutSampler: sampler;

@fragment fn fs2d(fsInput: VSOutput) -> @location(0) vec4f {
  let color = textureSample(postTexture2d, postSampler, fsInput.texcoord);
  var rgb = color.rgb;
-  rgb = adjustBrightness(rgb, uni.brightness);
-  rgb = adjustContrast(rgb, uni.contrast);
-  rgb = mix(rgb, apply1DLUT(rgb, lut, lutSampler), uni.lutAmount);
+  rgb = mix(rgb, apply3DLUT(rgb, lut, lutSampler), uni.lutAmount);
  return vec4f(rgb, color.a);
}
```

To use it we'll need a 3D texture. The simplest 3D-LUT is a 2x2x2 identity LUT where *identity* means nothing happens. It's like multiplying by 1 or doing nothing, even though we're looking up colors in the LUT each color in maps to the same color out.

<div class="webgpu_center"><img src="resources/images/3dlut-standard-2x2.svg" class="noinvertdark" style="width: 200px"></div>

Here's the code to make a 2ˣ2ˣ2 3D texture with the colors required for an
identity LUT. Note the texture's `dimension` is `D3`.

```rust
fn make_identity_lut_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
  let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size: wgpu::Extent3d {
      width: 2,
      height: 2,
      depth_or_array_layers: 2,
    },
    dimension: wgpu::TextureDimension::D3,
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    mip_level_count: 1,
    sample_count: 1,
    view_formats: &[],
  });

  #[rustfmt::skip]
  let identity_lut: [u8; 32] = [
      0,   0,   0, 255,  // black
    255,   0,   0, 255,  // red
      0, 255,   0, 255,  // green
    255, 255,   0, 255,  // yellow
      0,   0, 255, 255,  // blue
    255,   0, 255, 255,  // magenta
      0, 255, 255, 255,  // cyan
    255, 255, 255, 255,  // white
  ];

  queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture: &texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
    &identity_lut,
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(8),
      rows_per_image: Some(2),
    },
    wgpu::Extent3d {
      width: 2,
      height: 2,
      depth_or_array_layers: 2,
    },
  );

  texture
}
```

We need some code to use it. Let's use it twice, once with linear filtering
and once without. We give each entry a name; the page's GUI dropdown shows
the same names and forwards the selected index as the `lut` setting.

```rust
  let lut_nearest_sampler = app.device.create_sampler(&Default::default());
  let lut_linear_sampler = app.device.create_sampler(&wgpu::SamplerDescriptor {
    mag_filter: wgpu::FilterMode::Linear,
    min_filter: wgpu::FilterMode::Linear,
    ..Default::default()
  });

  let make_lut_bind_group = |texture: &wgpu::Texture, sampler: &wgpu::Sampler| {
    app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &post_process_pipeline.get_bind_group_layout(1),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(
            &texture.create_view(&Default::default()),
          ),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(sampler),
        },
      ],
    })
  };

  let identity_lut_texture = make_identity_lut_texture(&app.device, &app.queue);
  // (name, bindGroup) pairs; the names fill the GUI dropdown on the page
  let lut_bind_groups: Vec<(&str, wgpu::BindGroup)> = vec![
    (
      "identity",
      make_lut_bind_group(&identity_lut_texture, &lut_linear_sampler),
    ),
    (
      "identity (nearest)",
      make_lut_bind_group(&identity_lut_texture, &lut_nearest_sampler),
    ),
  ];

  ...

    // read the settings the GUI on the page sets
-    let brightness = wgpu_fun::setting_f64("brightness", 0.0) as f32;
-    let contrast = wgpu_fun::setting_f64("contrast", 0.0) as f32;
    let lut_amount = wgpu_fun::setting_f64("lutAmount", 1.0) as f32;
    let lut = wgpu_fun::setting_f64("lut", 0.0) as usize % lut_bind_groups.len();
    frame.queue.write_buffer(
      &post_process_uniform_buffer,
      0,
-      bytemuck::cast_slice(&[brightness, contrast, lut_amount]),
+      bytemuck::cast_slice(&[lut_amount]),
    );

    ...

      pass.set_pipeline(&post_process_pipeline);
      pass.set_bind_group(0, post_process_bind_group.as_ref().unwrap(), &[]);
-      pass.set_bind_group(1, &lut_bind_groups[lut], &[]);
+      pass.set_bind_group(1, &lut_bind_groups[lut].1, &[]);
      pass.draw(0..3, 0..1);
```

and in the page we swap the gradient swatches for a dropdown

```js
+// must match the lut_bind_groups order in the Rust example
+const lutNames = [
+  'identity',
+  'identity (nearest)',
+];

const settings = {
-  brightness: 0,
-  contrast: 0,
  lutAmount: 1,
  lut: 0,
};

const gui = new GUI();
-gui.add(settings, 'brightness', -1, 1)
-   .onChange(v => wasm.set_setting_num('brightness', v));
-gui.add(settings, 'contrast', -1, 10)
-   .onChange(v => wasm.set_setting_num('contrast', v));
gui.add(settings, 'lutAmount', 0, 1)
   .onChange(v => wasm.set_setting_num('lutAmount', v));
+const keyValues = Object.fromEntries(lutNames.map((name, i) => [name, i]));
+gui.add(settings, 'lut', { keyValues })
+   .onChange(v => wasm.set_setting_num('lut', v));
```

With that we get the identity lut which has zero affect 😂 but at least
we can try it without filtering and see a strong effect.

{{{example url="../webgpu-post-processing-image-adjustments-3d-lut.html" }}}

First decide on the resolution of the LUT you want and generate the slices of the lookup cube using a simple script. This is a standalone 2d-canvas utility webpage, so it stays plain JavaScript.

```js
const ctx = document.querySelector('canvas').getContext('2d');

function drawColorCubeImage(ctx, size) {
  const canvas = ctx.canvas;
  canvas.width = size * size;
  canvas.height = size;

  for (let zz = 0; zz < size; ++zz) {
    for (let yy = 0; yy < size; ++yy) {
      for (let xx = 0; xx < size; ++xx) {
        const r = Math.floor(xx / (size - 1) * 255);
        const g = Math.floor(yy / (size - 1) * 255);
        const b = Math.floor(zz / (size - 1) * 255);
        ctx.fillStyle = `rgb(${r},${g},${b})`;
        ctx.fillRect(zz * size + xx, yy, 1, 1);
      }
    }
  }
}

drawColorCubeImage(ctx, 8);
```

and we need some html

```html
<h1>Color Cube Image Maker</h1>
<div>size:<input id="size" type="number" value="8" min="2" max="64"/></div>
<p><button type="button">Save...</button></p>
<div id="cube"><canvas></canvas></div>
<div>( note: actual image size is
<span id="width"></span>x<span id="height"></span> )</div>
</p>
```

And to JS to make a UI

```js
function update(size) {
  drawColorCubeImage(ctx, size);
  document.querySelector('#width').textContent = ctx.canvas.width;
  document.querySelector('#height').textContent = ctx.canvas.height;
}
update(8);

function handleSizeChange(event) {
  const elem = event.target;
  elem.style.background = '';
  try {
    const size = parseInt(elem.value);
    if (size >= 2 && size <= 64) {
      update(size);
    }
  } catch (e) {
    elem.style.background = 'red';
  }
}

const sizeElem = document.querySelector('#size');
sizeElem.addEventListener('change', handleSizeChange, true);

const saveData = (function() {
  const a = document.createElement('a');
  document.body.appendChild(a);
  a.style.display = 'none';
  return function saveData(blob, fileName) {
    const url = window.URL.createObjectURL(blob);
    a.href = url;
    a.download = fileName;
    a.click();
  };
}());

document.querySelector('button').addEventListener('click', () => {
  ctx.canvas.toBlob((blob) => {
    saveData(blob, `identity-lut-s${ctx.canvas.height}.png`);
  });
});
```

Now we can generate a identity 3d lookup table for any size. [^size]

[^size]: Adobe .cube files are generally 33ˣ33ˣ33

{{{example url="../3dlut-base-cube-maker.html" }}}

The larger the resolution the more fine adjustments we can make but being a cube of data the size required grows quickly. A size 8 cube only requires 2k but a size 64 cube requires 1meg. So use the smallest that reproduces the effect you want.

Let's set the size to 16 and then click save the file which gives us this file.

<div class="webgpu_center"><img src="resources/images/identity-lut-s16.png" style="image-rendering: pixelated; width: 256px;"></div>

We then go it into an image editor, in my case Photoshop, load up a sample image, and paste the 3D-LUT in the top left corner

> note: I first tried dragging and dropping the cube file on top of the image
> in Photoshop but that didn't work. Photoshop made the image twice as large.
> I'm guessing it was trying to match DPI or something. Loading the cube file
> separately and then copying and pasting it into the screen capture worked.

<div class="webgpu_center"><img class="nobg" src="resources/images/3d-lut-photoshop-before.png" style="width: 1100px"></div>

We then use any of the color based full image adjustments to adjust the image. For Photoshop most of the adjustments we can use are available on the Adjustments tab.

<div class="webgpu_center"><img class="nobg" src="resources/images/3d-lut-photoshop-after.png" style="width: 1100px"></div>

After we've adjusted the image to our liking you can see the cube slices we placed in the top left corner have the same adjustments applied.

Okay but how do we use it?

First I saved it as a png `3d-lut-orange-to-green-s16.png`. To save memory we could have cropped it to just the 256ˣ16 top left corner of the LUT table but just for fun we'll crop it after loading. The good thing about using this method is we can get some idea of the effective of the LUT just by looking at the .png file. The bad thing is of course wasted bandwidth.

Here's some code to load it. The code loads the image, copies out only the 3D-LUT part, then uploads it to the texture one slice at a time.

```rust
/// create a LUT texture from an image URL. You must pass in the size of the LUT
/// It's assumed to be in the top left corner of the image.
///
/// +---------+---------+---------+---------+---------+---------+---→
/// |         |         |         |         |         |         |
/// | layer 0 | layer 1 | layer 2 | layer 3 |   ...   | layer n |
/// |         |         |         |         |         |         |
/// +---------+---------+---------+---------+---------+---------+
/// |
/// ↓
async fn create_lut_texture_from_image(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    url: &str,
    lut_size: u32,
) -> wgpu::Texture {
    let img = wgpu_fun::load_image(url).await;
    // The JS version draws the image into a lutSize² x lutSize 2d canvas
    // and reads it back; we copy the same top left region ourselves.
    let width = lut_size * lut_size;
    let mut data = vec![0u8; (width * lut_size * 4) as usize];
    for y in 0..lut_size.min(img.height) {
        let src = (y * img.width * 4) as usize;
        let dst = (y * width * 4) as usize;
        let len = (width.min(img.width) * 4) as usize;
        data[dst..dst + len].copy_from_slice(&img.data[src..src + len]);
    }

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: lut_size,
            height: lut_size,
            depth_or_array_layers: lut_size,
        },
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1,
        sample_count: 1,
        view_formats: &[],
    });

    for z in 0..lut_size {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: (z * lut_size * 4) as u64,
                bytes_per_row: Some(width * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: lut_size,
                height: lut_size,
                depth_or_array_layers: 1,
            },
        );
    }
    texture
}
```

Let's add our custom lut to the list of existing luts.

```rust
-  let lut_bind_groups: Vec<(&str, wgpu::BindGroup)> = vec![
+  let mut lut_bind_groups: Vec<(&str, wgpu::BindGroup)> = vec![
    ...

+  let lut_textures: &[(&str, &str)] = &[
+    ("custom",          "resources/images/lut/3d-lut-orange-to-green-s16.png"),
+  ];
+
+  for &(name, url) in lut_textures {
+    // assumes filename ends in '-s<num>[n]'
+    // where <num> is the size of the 3DLUT cube
+    // and [n] means 'no filtering' or 'nearest'
+    //
+    // examples:
+    //    'foo-s16.png' = size:16, filter: true
+    //    'bar-s8n.png' = size:8, filter: false
+    let m = url.rsplit_once("-s").unwrap().1;
+    let digits: String = m.chars().take_while(|c| c.is_ascii_digit()).collect();
+    let size: u32 = digits.parse().unwrap();
+    let filter = !m[digits.len()..].starts_with('n');
+
+    let texture = create_lut_texture_from_image(&app.device, &app.queue, url, size).await;
+    let sampler = if filter {
+      &lut_linear_sampler
+    } else {
+      &lut_nearest_sampler
+    };
+    lut_bind_groups.push((name, make_lut_bind_group(&texture, sampler)));
+  }
```

Above you can see we encoded the size of the LUT into the end of the filename. This makes it easier to pass around LUTs as pngs

While we're at it, , lets load a bunch more image based 3D-luts 

```rust
  let lut_textures: &[(&str, &str)] = &[
    ("custom",          "resources/images/lut/3d-lut-orange-to-green-s16.png"),
+    ("monochrome",      "resources/images/lut/monochrome-s8.png"),
+    ("sepia",           "resources/images/lut/sepia-s8.png"),
+    ("saturated",       "resources/images/lut/saturated-s8.png"),
+    ("posterize",       "resources/images/lut/posterize-s8n.png"),
+    ("posterize-3-rgb", "resources/images/lut/posterize-3-rgb-s8n.png"),
+    ("posterize-3-lab", "resources/images/lut/posterize-3-lab-s8n.png"),
+    ("posterize-4-lab", "resources/images/lut/posterize-4-lab-s8n.png"),
+    ("posterize-more",  "resources/images/lut/posterize-more-s8n.png"),
+    ("inverse",         "resources/images/lut/inverse-s8.png"),
+    ("color negative",  "resources/images/lut/color-negative-s8.png"),
+    ("funky contrast",  "resources/images/lut/funky-contrast-s8.png"),
+    ("nightvision",     "resources/images/lut/nightvision-s8.png"),
+    ("thermal",         "resources/images/lut/thermal-s8.png"),
+    ("b/w",             "resources/images/lut/black-white-s8n.png"),
+    ("hue +60",         "resources/images/lut/hue-plus-60-s8.png"),
+    ("hue +180",        "resources/images/lut/hue-plus-180-s8.png"),
+    ("hue -60",         "resources/images/lut/hue-minus-60-s8.png"),
+    ("red to cyan",     "resources/images/lut/red-to-cyan-s8.png"),
+    ("blues",           "resources/images/lut/blues-s8.png"),
+    ("infrared",        "resources/images/lut/infrared-s8.png"),
+    ("radioactive",     "resources/images/lut/radioactive-s8.png"),
+    ("goolgey",         "resources/images/lut/googley-s8.png"),
+    ("bgy",             "resources/images/lut/bgy-s8.png"),
  ];
```

(the page's `lutNames` list gets the same names added so the dropdown
matches)

And where's a bunch of luts to try.

{{{example url="../webgpu-post-processing-image-adjustments-3d-luts.html" }}}

Here's all the luts applied to our image

<div class="webgpu_center">
   <div data-diagram="imageLuts" class="fill-container"></div>
</div>

One last thing, just for fun, it turns out there's a standard LUT format defined by Adobe. If you [search on the net you can find lots of these LUT files](https://www.google.com/search?q=lut+files). For example [this site](https://freshluts.com/) has
lots of luts.

The original JavaScript version of the next example includes a quick loader
(`resources/js/lut-reader.js`) so you can drag-and-drop an Adobe LUT file
and have it applied; that's built on browser file APIs so the converted
example omits the drag-and-drop part and just offers the built-in LUTs.

{{{example url="../webgpu-post-processing-image-adjustments-3d-luts-w-loader.html"}}}

Here's some luts I found online and applied them to an image

<div class="webgpu_center">
   <div data-diagram="cubeLuts" class="fill-container" style="max-width: 1200px"></div>
</div>

Note that Adobe LUTs are not designed for online usage. They are large files.
(~1meg). You can convert them to smaller files and save as our PNG format by dragging and dropping on the sample below (a standalone 2d-canvas JavaScript page) and clicking "Save...". The PNG files are typically ~20x smaller, around 50k.

{{{example url="../adobe-lut-to-png-converter.html" }}}

<!-- keep this at the bottom of the article -->
<link href="webgpu-3dlut.css" rel="stylesheet">
<script type="module" src="webgpu-3dlut.js"></script>
