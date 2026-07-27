Title: WebGPU Compute Shaders - Image Histogram Part 2
Description: Using an image histogram to adjust video in real time.
TOC: Image Histogram Part 2

In [the previous article](webgpu-compute-shaders-histogram.html) we covered
how to make an image histogram on the CPU in Rust and then converted it to use
WebGPU and went through several steps of optimizing it.

Let's do a few more things with it

## Generate 4 histograms at once.

Given an image like this

<div class="webgpu_center">
  <div>
    <div><img src="../resources/images/pexels-chevanon-photography-1108099.jpg" style="max-width: 700px;"></div>
    <div style="text-align: center;"><a href="https://www.pexels.com/photo/two-yellow-labrador-retriever-puppies-1108099/">Photo by Chevanon Photography</a></div>
  </div>
</div>

It's common to generate multiple histograms

<div class="webgpu_center side-by-side">
  <div>
    <div><img src="resources/histogram-colors-photoshop-02.png" style="width: 237px;" class="nobg"></div>
  </div>
  <div>
    <div><img src="resources/histogram-luminosity-photoshop-02.png" style="width: 237px;" class="nobg"> </div>
  </div>
</div>

On the left we have 3 histograms, one for red values, one for green, and one for blue. They're drawn
to overlap. On the right we have a luminance histogram like the one we generated in  [the previous article](webgpu-compute-shaders-histogram.html).

It's a tiny change to generate all 4 at once.

Here's the changes to our `compute_histogram` function to generate 4 histograms
at once

```rust
fn compute_histogram(num_bins: usize, img_data: &ImageData) -> Vec<u32> {
  let ImageData { width, height, data } = img_data;
-  let mut bins = vec![0u32; num_bins];
+  let mut bins = vec![0u32; num_bins * 4];
  for y in 0..*height {
    for x in 0..*width {
      let offset = ((y * width + x) * 4) as usize;

      let r = data[offset] as f32 / 255.0;
      let g = data[offset + 1] as f32 / 255.0;
      let b = data[offset + 2] as f32 / 255.0;
-      let v = srgb_luminance(r, g, b);
-
-      let bin = (v * num_bins as f32) as usize;
-      bins[bin.min(num_bins - 1)] += 1;
+      let channels = [r, g, b, srgb_luminance(r, g, b)];
+      for (ch, v) in channels.iter().enumerate() {
+        let bin = ((v * num_bins as f32) as usize).min(num_bins - 1);
+        bins[bin * 4 + ch] += 1;
+      }
    }
  }
  bins
}
```

This will generate the histograms interleaved, r, g, b, l, r, g, b, l, r, g, b, l ....

Now we need to update the code that draws them. The JavaScript version draws
with the canvas 2D API. It picks a color for each channel and sets
`globalCompositeOperation = 'screen'` so that overlapping bars add up like
light. Our `histogram_to_image` function generates the graph as an `ImageData`
on the CPU, so we do the same 'screen' compositing ourselves.

```rust
+// Like the JS version's drawHistogram: draws the chosen channels with
+// 'screen' compositing (red, green, blue, white for luminance).
-fn histogram_to_image(histogram: &[u32], num_entries: u32, height: usize) -> ImageData {
-  let num_bins = histogram.len();
-  let max = *histogram.iter().max().unwrap();
-  let scale = (1.0 / max as f32).max(0.2 * num_bins as f32 / num_entries as f32);
+fn histogram_to_image(
+  histogram: &[u32],
+  num_entries: u32,
+  channels: &[usize],
+  height: usize,
+) -> ImageData {
+  // find the highest value for each channel
+  let num_bins = histogram.len() / 4;
+  let mut max = [0u32; 4];
+  for (ndx, v) in histogram.iter().enumerate() {
+    let ch = ndx % 4;
+    max[ch] = max[ch].max(*v);
+  }
+  let scale =
+    max.map(|max| (1.0 / max as f32).max(0.2 * num_bins as f32 / num_entries as f32));
+
+  let colors: [[f32; 3]; 4] = [
+    [1.0, 0.0, 0.0],
+    [0.0, 1.0, 0.0],
+    [0.0, 0.0, 1.0],
+    [1.0, 1.0, 1.0],
+  ];

  let mut data = vec![0u8; num_bins * height * 4];
  for x in 0..num_bins {
-    let v = (histogram[x] as f32 * scale * height as f32) as usize;
-    for y in (height - v.min(height))..height {
-      let o = (y * num_bins + x) * 4;
-      data[o..o + 4].copy_from_slice(&[255, 255, 255, 255]);
-    }
+    let offset = x * 4;
+    for y in 0..height {
+      // 'screen' composite the channels whose bar covers this pixel
+      let mut acc = [0.0f32; 3];
+      for &ch in channels {
+        let v = (histogram[offset + ch] as f32 * scale[ch] * height as f32) as usize;
+        if height - y <= v {
+          for c in 0..3 {
+            acc[c] = 1.0 - (1.0 - acc[c]) * (1.0 - colors[ch][c]);
+          }
+        }
+      }
+      let o = (y * num_bins + x) * 4;
+      data[o] = (acc[0] * 255.0) as u8;
+      data[o + 1] = (acc[1] * 255.0) as u8;
+      data[o + 2] = (acc[2] * 255.0) as u8;
+      data[o + 3] = 255;
+    }
  }
  ImageData {
    data,
    width: num_bins as u32,
    height: height as u32,
  }
}
```

There's now a per channel `max` and per channel `scale` since each channel's
histogram gets its own scale. 'screen' compositing is
`1 - (1 - dst) * (1 - src)`: where only the red bar covers a pixel we get pure
red, where the red and green bars overlap we get yellow, and where all 3 color
bars overlap we get white.

And then call that function twice, once to render the
color histograms and once for the luminance histogram

```rust
  let histogram = compute_histogram(num_bins, &img);

  let num_entries = texture.width() * texture.height();
-  let histogram_image = histogram_to_image(&histogram, num_entries, 100);
-  let histogram_texture = create_texture_from_source(&app.device, &app.queue, &histogram_image);
+  // draw the red, green, and blue channels
+  let color_histogram = create_texture_from_source(
+    &app.device, &app.queue, &histogram_to_image(&histogram, num_entries, &[0, 1, 2], 100));
+
+  // draw the luminosity channel
+  let luminosity_histogram = create_texture_from_source(
+    &app.device, &app.queue, &histogram_to_image(&histogram, num_entries, &[3], 100));

-  show_images(app, vec![texture, histogram_texture]);
+  show_images(app, vec![texture, color_histogram, luminosity_histogram]);
```

And now we get these results.

{{{example url="../webgpu-compute-shaders-histogram-4ch-javascript.html"}}}

Doing the same to our WGSL examples is even simpler

For example the our first example that was too slow would
change like this

```wgsl
-@group(0) @binding(0) var<storage, read_write> bins: array<u32>;
+@group(0) @binding(0) var<storage, read_write> bins: array<vec4u>;
@group(0) @binding(1) var ourTexture: texture_2d<f32>;

// from: https://www.w3.org/WAI/GL/wiki/Relative_luminance
const kSRGBLuminanceFactors = vec3f(0.2126, 0.7152, 0.0722);
fn srgbLuminance(color: vec3f) -> f32 {
  return saturate(dot(color, kSRGBLuminanceFactors));
}

@compute @workgroup_size(1, 1, 1) fn cs() {
  let size = textureDimensions(ourTexture, 0);
  let numBins = f32(arrayLength(&bins));
  let lastBinIndex = u32(numBins - 1);
  for (var y = 0u; y < size.y; y++) {
    for (var x = 0u; x < size.x; x++) {
      let position = vec2u(x, y);
-      let color = textureLoad(ourTexture, position, 0);
-      let v = srgbLuminance(color.rgb);
-      let bin = min(u32(v * numBins), lastBinIndex);
-      bins[bin] += 1;
+      var channels = textureLoad(ourTexture, position, 0);
+      channels.w = srgbLuminance(channels.rgb);
+      for (var ch = 0; ch < 4; ch++) {
+        let v = channels[ch];
+        let bin = min(u32(v * numBins), lastBinIndex);
+        bins[bin][ch] += 1;
+      }
    }
  }
}
```

We needed to make room for all 4 channels by changing bins
from `array<u32>` to `array<vec4u>`.

Then we pulled out the color from the texture, computed a
luminance and put it in the `w` element of `channels`

```wgsl
  var channels = textureLoad(ourTexture, position, 0);
  channels.w = srgbLuminance(channels.rgb);
```

This way we could just loop over the 4 channels and increment
the correct bin.

The only other change we need is allocating 4x the memory
for our buffer

```rust
  let num_bins = 256u32;
  let histogram_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
-    size: (num_bins * 4) as u64, // 256 entries * 4 bytes per (u32)
+    size: (num_bins * 4 * 4) as u64, // 256 entries * 4 (rgba) * 4 bytes per (u32)
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    mapped_at_creation: false,
  });
```

And here's our slow WebGPU version generating 4 histograms

{{{example url="../webgpu-compute-shaders-histogram-4ch-slow.html"}}}

Making similar changes to our fastest version:

```wgsl
const chunkWidth = 256;
const chunkHeight = 1;
const chunkSize = chunkWidth * chunkHeight;
-var<workgroup> bins: array<atomic<u32>, chunkSize>;
-@group(0) @binding(0) var<storage, read_write> chunks: array<array<u32, chunkSize>>;
+var<workgroup> bins: array<array<atomic<u32>, 4>, chunkSize>;
+@group(0) @binding(0) var<storage, read_write> chunks: array<array<vec4u, chunkSize>>;
@group(0) @binding(1) var ourTexture: texture_2d<f32>;

const kSRGBLuminanceFactors = vec3f(0.2126, 0.7152, 0.0722);
fn srgbLuminance(color: vec3f) -> f32 {
  return saturate(dot(color, kSRGBLuminanceFactors));
}

@compute @workgroup_size(chunkWidth, chunkHeight, 1)
fn cs(
  @builtin(global_invocation_id) global_invocation_id: vec3u,
  @builtin(workgroup_id) workgroup_id: vec3u,
  @builtin(local_invocation_id) local_invocation_id: vec3u,
) {
  let size = textureDimensions(ourTexture, 0);
  let position = global_invocation_id.xy;
  if (all(position < size)) {
    let numBins = f32(chunkSize);
    let lastBinIndex = u32(numBins - 1);
-    let color = textureLoad(ourTexture, position, 0);
-    let v = srgbLuminance(color.rgb);
-    let bin = min(u32(v * numBins), lastBinIndex);
-    atomicAdd(&bins[bin], 1u);
+    var channels = textureLoad(ourTexture, position, 0);
+    channels.w = srgbLuminance(channels.rgb);
+    for (var ch = 0; ch < 4; ch++) {
+      let v = channels[ch];
+      let bin = min(u32(v * numBins), lastBinIndex);
+      atomicAdd(&bins[bin][ch], 1u);
+    }
  }

  workgroupBarrier();

  let chunksAcross = (size.x + chunkWidth - 1) / chunkWidth;
  let chunk = workgroup_id.y * chunksAcross + workgroup_id.x;
  let bin = local_invocation_id.y * chunkWidth + local_invocation_id.x;

-  chunks[chunk][bin] = atomicLoad(&bins[bin]);
+  chunks[chunk][bin] = vec4u(
+    atomicLoad(&bins[bin][0]),
+    atomicLoad(&bins[bin][1]),
+    atomicLoad(&bins[bin][2]),
+    atomicLoad(&bins[bin][3]),
+  );
}
```

And for our reduce shader

```wgsl
const chunkWidth = 256;
const chunkHeight = 1;
const chunkSize = chunkWidth * chunkHeight;

struct Uniforms {
  stride: u32,
};

-@group(0) @binding(0) var<storage, read_write> chunks: array<array<u32, chunkSize>>;
+@group(0) @binding(0) var<storage, read_write> chunks: array<array<vec4u, chunkSize>>;
@group(0) @binding(1) var<uniform> uni: Uniforms;

@compute @workgroup_size(chunkSize, 1, 1) fn cs(
  @builtin(local_invocation_id) local_invocation_id: vec3u,
  @builtin(workgroup_id) workgroup_id: vec3u,
) {
  let chunk0 = workgroup_id.x * uni.stride * 2;
  let chunk1 = chunk0 + uni.stride;

  let sum = chunks[chunk0][local_invocation_id.x] +
            chunks[chunk1][local_invocation_id.x];
  chunks[chunk0][local_invocation_id.x] = sum;
}
```

Like the previous example, we need to increase the buffer sizes

```rust
  let chunks_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
-    size: (num_chunks * chunk_size * 4) as u64, // 4 bytes per (u32)
+    size: (num_chunks * chunk_size * 4 * 4) as u64, // 16 bytes per (vec4u)
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    mapped_at_creation: false,
  });

  let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
-    size: (chunk_size * 4) as u64, // 4 bytes per (u32)
+    size: (chunk_size * 4 * 4) as u64, // 16 bytes per (vec4u)
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
```

That's it.

{{{example url="../webgpu-compute-shaders-histogram-4ch-optimized-more.html"}}}

There were 2 other steps we tried in the previous article.
One used a single workgroup per pixel. Another summed the
chunks with an invocation per bin instead of reducing the bins.

Here's some timing info I got testing these 4 channel versions.

<div class="webgpu_center data-table">
  <div data-diagram="timings4ch"></div>
</div>

You can compare to the 1 channel versions from the previous
article.

<div class="webgpu_center data-table">
  <div data-diagram="timings"></div>
</div>

## Drawing the histogram on the GPU

Let's draw the histogram on the GPU. So far we've read the results back to the
CPU and generated the graph as an `ImageData` in `histogram_to_image`, one
1-by-height bar per bin, which was very easy. We could keep doing that but
I think there's a better approach for the particular issue of drawing a
histogram with the GPU.

Let's instead just draw a rectangle.
Drawing rectangles we've covered in many places. For example, most of
the examples from [the articles on textures](webgpu-textures.html) use
a rectangle.

For a histogram, in the fragment shader, we could
pass in a texture coordinate and 
convert the horizontal part from 0 -> 1 to 0 -> numBins - 1.
We could then look up the value in that bin and compute a height
in the 0 to 1 range. We could then compare that to our vertical
texture coordinate. If texture coordinate is above
the height then we could draw 0, if it's below the height we'll
could draw some color.

This would work for 1 channel but we'd like to draw multiple channels.
So instead, we'll set a bit, one for each channel that is above the
height and then use those 4 bits to look up one of 16 colors. This
will also let us select the colors we want to represent each channel and their combinations.

Here's a fragment shader that does this

```wgsl
struct Uniforms {
  matrix: mat4x4f,  // <- used by the vertex shader
  colors: array<vec4f, 16>,
  channelMult: vec4u,
};

@group(0) @binding(0) var<storage, read> bins: array<vec4u>;
@group(0) @binding(1) var<uniform> uni: Uniforms;
@group(0) @binding(2) var<storage, read_write> scale: vec4f;

@fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
  let numBins = arrayLength(&bins);
  let lastBinIndex = u32(numBins - 1);
  let bin = clamp(
      u32(fsInput.texcoord.x * f32(numBins)),
      0,
      lastBinIndex);
  let heights = vec4f(bins[bin]) * scale;
  let bits = heights > vec4f(fsInput.texcoord.y);
  let ndx = dot(select(vec4u(0), uni.channelMult, bits), vec4u(1));
  return uni.colors[ndx];
}
```

The first part is computing which bin based off the horizontal texture coordinate

```wgsl
  let numBins = arrayLength(&bins);
  let lastBinIndex = u32(numBins - 1);
  let bin = clamp(
      u32(fsInput.texcoord.x * f32(numBins)),
      0,
      lastBinIndex);
```

The next part is getting the heights for all 4 channels.
We're multiplying by `scale` just like we did on the
CPU. We'll need to supply that later.

```wgsl
  let heights = vec4f(bins[bin]) * scale;
```

Next we set 4 booleans in a `vec4<bool>`,
one for each channel. They'll be true the height of the bin is
higher than the texture coordinate.

```wgsl
    let bits = heights > vec4f(fsInput.texcoord.y);
```

The next part will then select values from `uni.channelMult` based on those 4 bools and then add
the 4 values.
Being able to pass in `uni.channelMult` is the similar to what we did with
the `channels` argument of `histogram_to_image`, letting us choose which
channels get drawn. For example
if we set `channelMult` to `1, 2, 4, 0` then we'll get the red, green,
and blue histograms.

```wgsl
  let ndx = dot(select(vec4u(0), uni.channelMult, bits), vec4u(1));
```

This last part looks up one of our 16 colors.

```wgsl
  return uni.colors[ndx];
```

We also need a shader to compute `scale`. On the CPU we
did this

```rust
  let num_bins = histogram.len() / 4;
  let mut max = [0u32; 4];
  for (ndx, v) in histogram.iter().enumerate() {
    let ch = ndx % 4;
    max[ch] = max[ch].max(*v);
  }
  let scale =
    max.map(|max| (1.0 / max as f32).max(0.2 * num_bins as f32 / num_entries as f32));
```

To do the same thing in a compute shaders we could do something like this

```wgsl
@group(0) @binding(0) var<storage, read> bins: array<vec4u>;
@group(0) @binding(1) var<storage, read_write> scale: vec4f;
@group(0) @binding(2) var ourTexture: texture_2d<f32>;

@compute @workgroup_size(1, 1, 1) fn cs() {
  let size = textureDimensions(ourTexture, 0);
  let numEntries = f32(size.x * size.y);
  var m = vec4u(0);
  let numBins = arrayLength(&bins);
  for (var i = 0u ; i < numBins; i++) {
    m = max(m, bins[i]);
  }
  scale = max(1.0 / vec4f(m), vec4f(0.2 * f32(numBins) / numEntries));
}
```

Note that the only reason we pass in `ourTexture` is to get its size so
we can compute `numEntries` where as on the CPU we passed in `num_entries`.
We could also use a uniform to pass in `numEntries` but then we'd have to
create a uniform buffer, update it with the value for `numEntries`, bind
it, etc... It seemed easier to just reference the texture itself.

Another thing to consider is this is another place where we're using only
a single core. We could reduce here too but there are only `numBins` steps
which is only 256. The overhead of dispatching a bunch of reduce steps
would *probably* outweigh the parallelization. I did time it and was told
it was around 0.1ms, at least on one machine.

So, what's left to do is put the parts together

Since we're going to draw with the GPU we need the canvas format but our
`App` helper already provides that as `app.format` so, unlike the JavaScript
version which calls `navigator.gpu.getPreferredCanvasFormat()`, there's
nothing new to look up.

We need to create the shader modules with the 2 shaders above
and create pipelines for each one. 

```rust
  let scale_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("histogram scale shader"),
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      @group(0) @binding(0) var<storage, read> bins: array<vec4u>;
      @group(0) @binding(1) var<storage, read_write> scale: vec4f;
      @group(0) @binding(2) var ourTexture: texture_2d<f32>;

      @compute @workgroup_size(1, 1, 1) fn cs() {
        let size = textureDimensions(ourTexture, 0);
        let numEntries = f32(size.x * size.y);

        var m = vec4u(0);
        let numBins = arrayLength(&bins);
        for (var i = 0u ; i < numBins; i++) {
          m = max(m, bins[i]);
        }
        scale = max(1.0 / vec4f(m), vec4f(0.2 * f32(numBins) / numEntries));
      }
    "#.into()),
  });

  let draw_histogram_module = app.device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("draw histogram shader"),
    source: wgpu::ShaderSource::Wgsl(/* wgsl */ r#"
      struct OurVertexShaderOutput {
        @builtin(position) position: vec4f,
        @location(0) texcoord: vec2f,
      };

      struct Uniforms {
        matrix: mat4x4f,
        colors: array<vec4f, 16>,
        channelMult: vec4u,
      };

      @group(0) @binding(0) var<storage, read> bins: array<vec4u>;
      @group(0) @binding(1) var<uniform> uni: Uniforms;
      @group(0) @binding(2) var<storage, read_write> scale: vec4f;

      @vertex fn vs(
        @builtin(vertex_index) vertexIndex : u32
      ) -> OurVertexShaderOutput {
        let pos = array(

          vec2f( 0.0,  0.0),  // center
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 0.0,  1.0),  // center, top

          // 2st triangle
          vec2f( 0.0,  1.0),  // center, top
          vec2f( 1.0,  0.0),  // right, center
          vec2f( 1.0,  1.0),  // right, top
        );

        var vsOutput: OurVertexShaderOutput;
        let xy = pos[vertexIndex];
        vsOutput.position = uni.matrix * vec4f(xy, 0.0, 1.0);
        vsOutput.texcoord = xy;
        return vsOutput;
      }

      @fragment fn fs(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
        let numBins = arrayLength(&bins);
        let lastBinIndex = u32(numBins - 1);
        let bin = clamp(
            u32(fsInput.texcoord.x * f32(numBins)),
            0,
            lastBinIndex);
        let heights = vec4f(bins[bin]) * scale;
        let bits = heights > vec4f(fsInput.texcoord.y);
        let ndx = dot(select(vec4u(0), uni.channelMult, bits), vec4u(1));
        return uni.colors[ndx];
      }
    "#.into()),
  });

  let scale_pipeline = app.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("scale"),
    layout: None,
    module: &scale_module,
    entry_point: None,
    compilation_options: Default::default(),
    cache: None,
  });

  let draw_histogram_pipeline = app.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: Some("draw histogram"),
    layout: None,
    vertex: wgpu::VertexState {
      module: &draw_histogram_module,
      entry_point: None,
      compilation_options: Default::default(),
      buffers: &[],
    },
    fragment: Some(wgpu::FragmentState {
      module: &draw_histogram_module,
      entry_point: None,
      compilation_options: Default::default(),
      targets: &[Some(app.format.into())],
    }),
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
```

We no longer need the result buffer since we're not going
to read the values back but we need a scale buffer to
store the scale we're going to compute.

```rust
-  let result_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
-    label: None,
-    size: (chunk_size * 4 * 4) as u64,
-    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
-    mapped_at_creation: false,
-  });
+  let scale_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
+    label: Some("scale buffer"),
+    size: 4 * 4,
+    usage: wgpu::BufferUsages::STORAGE,
+    mapped_at_creation: false,
+  });
```

We need a bind group for our scale pipeline that has the chunks,
the scale buffer, and the texture. For the chunks we don't want to
bind the whole buffer, just the first chunk, so we make a binding
resource with an explicit size. We'll `.clone()` it because we'll
use it again below when we draw.

```rust
  let chunks_binding = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
    buffer: &chunks_buffer,
    offset: 0,
    size: wgpu::BufferSize::new((chunk_size * 4 * 4) as u64),
  });

  let scale_bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("scale bindGroup"),
    layout: &scale_pipeline.get_bind_group_layout(0),
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: chunks_binding.clone(),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: scale_buffer.as_entire_binding(),
      },
      wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::TextureView(
          &texture.create_view(&Default::default()),
        ),
      },
    ],
  });
```

Above we set the size of the binding for the `chunks_buffer`
to just the size of the first chunk. This way, in the
shader this code

```wgsl
      @group(0) @binding(0) var<storage, read> bins: array<vec4u>;

      ...

        let numBins = arrayLength(&bins);
```

will get the correct value. If we didn't specify the size
then the entire `chunks_buffer` size would be available and
`numBins` would be calculated from all of the chunks, not just
the first one.

Now, after we've reduced the chunks into one chunk we can
run our scale compute shader to compute the scale and,
since we no longer have a result buffer we no longer need to copy
the first chunk into it, nor do we need to map it,
nor do we need to pass `num_entries` since we were using that
to compute a scale but we've already done that. We also
are not going to pass `histogram` which is the data we got
from the result buffer. Our data is already in the `chunks_buffer`.


```rust
+    // compute scales for the channels
+    pass.set_pipeline(&scale_pipeline);
+    pass.set_bind_group(0, &scale_bind_group, &[]);
+    pass.dispatch_workgroups(1, 1, 1);
  }

-  encoder.copy_buffer_to_buffer(&chunks_buffer, 0, &result_buffer, 0, result_buffer.size());
  app.queue.submit([encoder.finish()]);

-  wgpu_fun::map_async(&app.device, &result_buffer, wgpu::MapMode::Read).await;
-  let histogram: Vec<u32> = {
-    let data = result_buffer.slice(..).get_mapped_range().unwrap();
-    bytemuck::cast_slice(&data).to_vec()
-  };
-  result_buffer.unmap();

-  let num_entries = texture.width() * texture.height();
  // draw the red, green, and blue channels
-  let color_histogram = create_texture_from_source(
-    &app.device, &app.queue, &histogram_to_image(&histogram, num_entries, &[0, 1, 2], 100));
+  let color_histogram = draw_histogram(&[0, 1, 2], 100);

  // draw the luminosity channel
-  let luminosity_histogram = create_texture_from_source(
-    &app.device, &app.queue, &histogram_to_image(&histogram, num_entries, &[3], 100));
+  let luminosity_histogram = draw_histogram(&[3], 100);

  show_images(app, vec![texture, color_histogram, luminosity_histogram]);
```

Now we need to write that `draw_histogram` function, which replaces
`histogram_to_image` and renders with the GPU. The JavaScript version
creates one canvas per histogram and renders into it. We have a single
canvas so instead each histogram renders into its own texture and we
composite those textures into our one canvas with `show_images`, exactly
like we've been doing with the `ImageData` based textures.

First we need to make a uniform buffer to pass our uniforms.
For reference here's the uniforms from the shaders we'll draw
the histogram with

```wgsl
struct Uniforms {
  matrix: mat4x4f,
  colors: array<vec4f, 16>,
  channelMult: vec4u,
};
```

So, here's the code to create a buffer for and fill out the
channelMult and colors.

```rust
  // Draw a histogram entirely on the GPU into its own texture.
  let draw_histogram = |channels: &[usize], height: u32| -> wgpu::Texture {
    let num_bins = chunk_size;

    //  matrix: mat4x4f;
    //  colors: array<vec4f, 16>;
    //  channelMult; vec4u,
    let mut uniform_values_f32 = [0.0f32; 16 + 64 + 4 + 4];
    let uniform_buffer = app.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("draw histogram uniform buffer"),
      size: (uniform_values_f32.len() * 4) as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    uniform_values_f32[16..16 + 64].copy_from_slice(&[
      0.0, 0.0, 0.0, 1.0,
      1.0, 0.0, 0.0, 1.0,
      0.0, 1.0, 0.0, 1.0,
      1.0, 1.0, 0.0, 1.0,
      0.0, 0.0, 1.0, 1.0,
      1.0, 0.0, 1.0, 1.0,
      0.0, 1.0, 1.0, 1.0,
      0.5, 0.5, 0.5, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
      1.0, 1.0, 1.0, 1.0,
    ]);
    let channel_mult: [u32; 4] =
      std::array::from_fn(|i| if channels.contains(&i) { 2u32.pow(i as u32) } else { 0 });
    uniform_values_f32[16 + 64..16 + 64 + 4]
      .copy_from_slice(bytemuck::cast_slice(&channel_mult));
```

The JavaScript version makes typed-array views into one `ArrayBuffer`
for `matrix`, `colors`, and `channelMult`. In Rust we just slice one
`[f32]` array at the same offsets, using `bytemuck` to store the `u32`
`channelMult` values in it.

We also need to compute a matrix using matrix math like we covered
in [the series of articles about matrix math](webgpu-translation.html).
We use `glam` where the JavaScript version uses its `mat4` helpers.

In particular, our shader has a hard coded unit quad that goes
from 0 to 1 in X and Y. If we scale it by 2 in both X and Y and
subtract 1 we'll get a quad that goes from -1 to +1 in both direction that covers clip space. This way of using a single
unit quad is common as then we can just use a little matrix
math to draw rectangles in any position and orientation without
having to make special vertex data.

```rust
    // matrix: cover clip space
    let matrix = glam::Mat4::from_translation(glam::Vec3::new(-1.0, -1.0, 0.0))
      * glam::Mat4::from_scale(glam::Vec3::new(2.0, 2.0, 1.0));
    uniform_values_f32[0..16].copy_from_slice(&matrix.to_cols_array());
    app.queue.write_buffer(&uniform_buffer, 0, bytemuck::cast_slice(&uniform_values_f32));
```

We need a bindGroup for all of this. Note how we reuse the
`chunks_binding` from above so the shader only sees the first chunk.

```rust
    let bind_group = app.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: None,
      layout: &draw_histogram_pipeline.get_bind_group_layout(0),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: chunks_binding.clone(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: uniform_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: scale_buffer.as_entire_binding(),
        },
      ],
    });
```

The JavaScript version creates a canvas configured for WebGPU here.
Instead we create the texture this histogram will be rendered into.

```rust
    // In the JS version each histogram gets its own canvas; we render
    // into a texture and composite them into our one canvas below.
    let target = app.device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width: num_bins,
        height,
        depth_or_array_layers: 1,
      },
      format: app.format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      view_formats: &[],
    });
```

and finally we can render

```rust
    let mut encoder = app.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
      label: Some("render histogram"),
    });
    {
      let view = target.create_view(&Default::default());
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("our basic canvas renderPass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.3, g: 0.3, b: 0.3, a: 1.0 }),
            store: wgpu::StoreOp::Store,
          },
          depth_slice: None,
        })],
        ..Default::default()
      });
      pass.set_pipeline(&draw_histogram_pipeline);
      pass.set_bind_group(0, &bind_group, &[]);
      pass.draw(0..6, 0..1); // call our vertex shader 6 times
    }
    app.queue.submit([encoder.finish()]);
    target
  };
```

And with all of that we're rendering on the GPU

{{{example url="../webgpu-compute-shaders-histogram-4ch-optimized-more-gpu-draw.html"}}}

Let's do one last thing, let's get a histogram of video.
We're effectively going to merge the example from the
[the article on using external video](webgpu-textures-external-video.html) and our previous example.

<div class="warn">
Video decoding is provided by the browser and, as covered in
<a href="webgpu-textures-external-video.html">the article on using external
video</a>, <code>importExternalTexture</code> is a browser-only API that wgpu
does not expose. So this final section is about the browser path: the code
shown is the original JavaScript version of the examples above (of which our
Rust examples are translations) and the example below runs that JavaScript.
The histogram computing and drawing techniques are exactly the ones we just
wrote; only the video import has no native equivalent.
</div>

We need to update our HTML and CSS to match the video example

```html
    <style>
      @import url(resources/webgpu-lesson.css);
+html, body {
+  margin: 0;       /* remove the default margin          */
+  height: 100%;    /* make the html,body fill the page   */
+}
canvas {
+  display: block;  /* make the canvas act like a block   */
+  width: 100%;     /* make the canvas fill its container */
+  height: 100%;
-  max-width: 256px;
-  border: 1px solid #888;
}
+#start {
+  position: fixed;
+  left: 0;
+  top: 0;
+  width: 100%;
+  height: 100%;
+  display: flex;
+  justify-content: center;
+  align-items: center;
+}
+#start>div {
+  font-size: 200px;
+  cursor: pointer;
+}
    </style>
  </head>
  <body>
+    <canvas></canvas>
+    <div id="start">
+      <div>▶️</div>
+    </div>
  </body>
```

We'll setup one canvas right at the beginning

```js
  // Get a WebGPU context from the canvas and configure it
  const canvas = document.querySelector('canvas');
  const context = canvas.getContext('webgpu');
  const presentationFormat = navigator.gpu.getPreferredCanvasFormat();
  context.configure({
    device,
    format: presentationFormat,
  });
```

Because we're using an external texture we need to change
our shaders for that kind of texture. For example the histogram
chunk making shader needs these changes

```wgsl
const chunkSize = chunkWidth * chunkHeight;
var<workgroup> bins: array<array<atomic<u32>, 4>, chunkSize>;
@group(0) @binding(0) var<storage, read_write> chunks: array<array<vec4u, chunkSize>>;
-@group(0) @binding(1) var ourTexture: texture_2d<f32>;
+@group(0) @binding(1) var ourTexture: texture_external;

const kSRGBLuminanceFactors = vec3f(0.2126, 0.7152, 0.0722);
fn srgbLuminance(color: vec3f) -> f32 {
  return saturate(dot(color, kSRGBLuminanceFactors));
}

@compute @workgroup_size(chunkWidth, chunkHeight, 1)
fn cs(
  @builtin(workgroup_id) workgroup_id: vec3u,
  @builtin(local_invocation_id) local_invocation_id: vec3u,
) {
-  let size = textureDimensions(ourTexture, 0);
+  let size = textureDimensions(ourTexture);
  let position = workgroup_id.xy * vec2u(chunkWidth, chunkHeight) + 
                 local_invocation_id.xy;
  if (all(position < size)) {
    let numBins = f32(chunkSize);
    let lastBinIndex = u32(numBins - 1);
-    var channels = textureLoad(ourTexture, position, 0);
+    var channels = textureLoad(ourTexture, position);
    channels.w = srgbLuminance(channels.rgb);
    for (var ch = 0; ch < 4; ch++) {
      let v = channels[ch];
      let bin = min(u32(v * numBins), lastBinIndex);
      atomicAdd(&bins[bin][ch], 1u);
    }
  }

...
```

Our scale calculating shader has similar changes

```wgsl
@group(0) @binding(0) var<storage, read> bins: array<vec4u>;
@group(0) @binding(1) var<storage, read_write> scale: vec4f;
-@group(0) @binding(2) var ourTexture: texture_2d<f32>;
+@group(0) @binding(2) var ourTexture: texture_external;

@compute @workgroup_size(1, 1, 1) fn cs() {
-  let size = textureDimensions(ourTexture, 0);
+  let size = textureDimensions(ourTexture);
  let numEntries = f32(size.x * size.y);

  ...
```

The shader module to draw the video is copied directly from
the video article as is the creation of a render pipeline
to use it and a sampler for the video and a uniform buffer
and render pass to draw with.
We have the same code to wait for a click and start playing
the video.

After the video starts we can setup for computing
a histogram. The only change is we don't get our size from
the texture but instead from the video.

```js
-  const imgBitmap = await loadImageBitmap('resources/images/pexels-francesco-ungaro-96938-mid.jpg');
-  const texture = createTextureFromSource(device, imgBitmap);

-  const chunksAcross = Math.ceil(texture.width / k.chunkWidth);
-  const chunksDown = Math.ceil(texture.height / k.chunkHeight);
+  const chunksAcross = Math.ceil(video.videoWidth / k.chunkWidth);
+  const chunksDown = Math.ceil(vide.videoHeight / k.chunkHeight);
```

We had our code to draw the histograms in `drawHistogram`
but that code created its own canvas and created other things
that were only used once. We'll get rid of `drawHistogram`
and make some code to setup a uniform buffer and bind group
for each of the 2 histograms we want to draw

```js
  const histogramDrawInfos = [
    [0, 1, 2],
    [3],
  ].map(channels => {
    //        matrix: mat4x4f;
    //        colors: array<vec4f, 16>;
    //        channelMult; vec4u,
    const uniformValuesAsF32 = new Float32Array(16 + 64 + 4 + 4);
    const uniformValuesAsU32 = new Uint32Array(uniformValuesAsF32.buffer);
    const uniformBuffer = device.createBuffer({
      label: 'draw histogram uniform buffer',
      size: uniformValuesAsF32.byteLength,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });
    thingsToDestroy.push(uniformBuffer);
    const subpart = (view, offset, length) => view.subarray(offset, offset + length);
    const matrix = subpart(uniformValuesAsF32, 0, 16);
    const colors = subpart(uniformValuesAsF32, 16, 64);
    const channelMult = subpart(uniformValuesAsU32, 16 + 64, 4);
    colors.set([
      [0, 0, 0, 1],
      [1, 0, 0, 1],
      [0, 1, 0, 1],
      [1, 1, 0, 1],
      [0, 0, 1, 1],
      [1, 0, 1, 1],
      [0, 1, 1, 1],
      [0.5, 0.5, 0.5, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
      [1, 1, 1, 1],
    ].flat());

    const drawHistogramBindGroup = device.createBindGroup({
      layout: drawHistogramPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: chunksBuffer, size: chunkSize * 4 * 4 }},
        { binding: 1, resource: uniformBuffer  },
        { binding: 2, resource: scaleBuffer },
      ],
    });

    return {
      drawHistogramBindGroup,
      matrix,
      uniformBuffer,
      uniformValuesAsF32,
    };
  });
```

At render time, first we import the video texture. Remember, it's only valid for this one JavaScript event so we have to create
the bind groups that reference the texture every frame

```js
  function render() {
    const texture = device.importExternalTexture({source: video});

    // make a bind group for to make a histogram from this video texture
    const histogramBindGroup = device.createBindGroup({
      layout: histogramChunkPipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: chunksBuffer },
        { binding: 1, resource: texture },
      ],
    });

    const scaleBindGroup = device.createBindGroup({
      layout: scalePipeline.getBindGroupLayout(0),
      entries: [
        { binding: 0, resource: { buffer: chunksBuffer, size: chunkSize * 4 * 4 }},
        { binding: 1, resource: scaleBuffer },
        { binding: 2, resource: texture },
      ],
    });

    ... insert histogram computing code here ...
```

As for render, rendering the video is similar to the article
about rendering external video. The one difference is the code
that computes a matrix. We're doing the scale by 2, subtract 1
thing like we mentioned above for the histogram but we're using -2
for y and adding 1 so that we flip Y. We're also scaling to
get a [cover effect](https://developer.mozilla.org/en-US/docs/Web/CSS/background-size#cover) so the video always fills the canvas but keeps the correct aspect ratio.

```js
    // Draw to canvas
    {
      const canvasTexture = context.getCurrentTexture().createView();
      renderPassDescriptor.colorAttachments[0].view = canvasTexture;
      const pass = encoder.beginRenderPass(renderPassDescriptor);

      // Draw video
      const bindGroup = device.createBindGroup({
        layout: videoPipeline.getBindGroupLayout(0),
        entries: [
          { binding: 0, resource: videoSampler },
          { binding: 1, resource: texture },
          { binding: 2, resource: videoUniformBuffer },
        ],
      });

      // 'cover' canvas
      const canvasAspect = canvas.clientWidth / canvas.clientHeight;
      const videoAspect = video.videoWidth / video.videoHeight;
      const scale = canvasAspect > videoAspect
         ? [1, canvasAspect / videoAspect, 1]
         : [videoAspect / canvasAspect, 1, 1];

      const matrix = mat4.identity(videoMatrix);
      mat4.scale(matrix, scale, matrix);
      mat4.translate(matrix, [-1, 1, 0], matrix);
      mat4.scale(matrix, [2, -2, 1], matrix);

      device.queue.writeBuffer(videoUniformBuffer, 0, videoUniformValues);

      pass.setPipeline(videoPipeline);
      pass.setBindGroup(0, bindGroup);
      pass.draw(6);  // call our vertex shader 6 times
```

To draw the histograms is just moving up the code from
`drawHistogram`

```js
      // Draw Histograms
      histogramDrawInfos.forEach(({
        matrix,
        uniformBuffer,
        uniformValuesAsF32,
        drawHistogramBindGroup,
      }, i) => {
        mat4.identity(matrix);
        mat4.translate(matrix, [-0.95 + i, -1, 0], matrix);
        mat4.scale(matrix, [0.9, 0.5, 1], matrix);

        device.queue.writeBuffer(uniformBuffer, 0, uniformValuesAsF32);

        pass.setPipeline(drawHistogramPipeline);
        pass.setBindGroup(0, drawHistogramBindGroup);
        pass.draw(6);  // call our vertex shader 6 times
      });

      pass.end();
    }

    const commandBuffer = encoder.finish();
    device.queue.submit([commandBuffer]);

    requestAnimationFrame(render);
  }
  requestAnimationFrame(render);
```

The matrix math above draws a quad on the left or right that is 90%
the width of half of the canvas, centered on that half,
and ¼ of the canvas tall.

{{{example url="../webgpu-compute-shaders-histogram-video.html"}}}

<div class="webgpu_center">
   <div>Video by <a href="https://www.pexels.com/video/timelapse-video-of-the-city-5750980/">Ekaterina Martynova</a>
   </div>
</div>

Ok, so why compute a histogram?
There are several things you can do with a histogram

* show it to the user so they can make informed decisions
  on image adjustments
* apply [histogram equalization](https://www.google.com/search?q=histogram+equalization) to the image
* apply [adaptive histogram equalization](https://www.google.com/search?q=adaptive+histogram+equalization) to the image
* Use it for [image segmentation](https://www.google.com/search?q=histogram+based+image+segmentation)
* Posterize using [histogram thresholding](https://www.google.com/search?q=histogram+thresholding)

And a bunch of other techniques. Maybe we can cover some later.
My hope is these have been useful examples. We went from CPU code
that computed a histogram and CPU code that drew a histogram
to having all the work done on the GPU, including rendering
that is hopefully fast enough to run in real time.

<!-- keep this at the bottom of the article -->
<link rel="stylesheet" href="webgpu-compute-shaders-histogram.css">
<script type="module" src="webgpu-compute-shaders-histogram.js"></script>
