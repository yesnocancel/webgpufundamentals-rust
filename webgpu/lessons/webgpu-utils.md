Title: Utility crates for wgpu
Description: Utils and Math for WebGPU in Rust
TOC: Utility Crates and Math

> ## What you should take away from this article
>
> Using WebGPU is very verbose. So verbose that it gets easier to understand
> if you use some helpers so that you can concentrate on the higher level concepts.
>
> For example, say you were learning math. Your teacher teaches you what "average"
> means and how to compute the average of some set of numbers. Once they've taught
> you, they then move on to other things and just say "here you compute the average".
> For example:
>
> > To compute the standard deviation
> > 
> > * Calculate the Average of all your data
> > * For each number in your data set, calculate the difference between that number and the average.
> > * After finding each difference, square it.
> > * Take the Square root of the average the squared differences
>
> They don't re-explain how to calculate an average. You've already learned it and
> they can now just refer to what you've already learned
>
> Similarly in WebGPU we have the concept of creating structures for uniforms in WGSL.
> Then creating one or more uniform buffers, and
> filling those buffers with data through `bytemuck`. We've covered this extensively
> in the first 20-30 articles on this site and in [the article on memory layout](webgpu-memory-layout.html).
>
> At some point though, it becomes harder
> to understand the code dealing with these details instead of just saying
> "set the uniform" and you, having learned previously that "set the uniforms" means
> "compute the offset to the various pieces of data, write the values there, and,
> before rendering, upload the values to the GPU".
>
> As such, don't be afraid of the crates used on this site — or of the wider
> Rust ecosystem's helpers. Almost all of their
> functionality is explained extensively in the first articles on the site.
> Some more details are provided below.

The examples on this site deliberately stay close to the raw wgpu API, but
they lean on a few crates, and there are a few more you'll almost certainly
want in real projects.

## glam

[glam](https://docs.rs/glam) is the math library the 3D examples use. It's a
collection of the same kinds of functions we wrote by hand in
[the article on matrix math](webgpu-matrix-math.html) through
[the article on perspective projection](webgpu-perspective-projection.html) as well
as [the article on lighting](webgpu-lighting-directional.html).

There's nothing special happening here. If you want to know how the math
works you can go read the articles listed above. A rough mapping from the
hand-written functions (and the JS site's wgpu-matrix library) to glam:

<div class="webgpu_center">

| articles / wgpu-matrix | glam |
| ---------------------- | ---- |
| `mat4.identity()` | `Mat4::IDENTITY` |
| `mat4.perspective(fov, aspect, near, far)` | `Mat4::perspective_rh(fov, aspect, near, far)` |
| `mat4.ortho(l, r, b, t, n, f)` | `Mat4::orthographic_rh(l, r, b, t, n, f)` |
| `mat4.lookAt(eye, target, up)` | `Mat4::look_at_rh(eye, target, up)` |
| `mat4.translation(v)` / `mat4.translate(m, v)` | `Mat4::from_translation(v)` / `m * Mat4::from_translation(v)` |
| `mat4.rotationX(a)` / `mat4.rotateX(m, a)` | `Mat4::from_rotation_x(a)` / `m * Mat4::from_rotation_x(a)` |
| `mat4.scaling(v)` / `mat4.scale(m, v)` | `Mat4::from_scale(v)` / `m * Mat4::from_scale(v)` |
| `mat4.multiply(a, b)` | `a * b` |
| `mat4.inverse(m)` | `m.inverse()` |
| `mat4.transpose(m)` | `m.transpose()` |
| `vec3.cross(a, b)`, `vec3.normalize(v)`, ... | `a.cross(b)`, `v.normalize()`, ... |

</div>

Two things to keep in mind:

* WebGPU clip space Z goes from 0 to 1, so use glam's `_rh` projection
  functions (**not** the `_gl` variants, which target OpenGL's -1 to 1).
* To upload a matrix, `matrix.to_cols_array()` gives you the 16 floats in
  column-major order, which is what WGSL's `mat4x4f` expects, and
  `bytemuck::cast_slice` turns them into bytes.

## bytemuck

[bytemuck](https://docs.rs/bytemuck) is how we turn typed data (`[f32]`,
structs) into the `&[u8]` that `queue.write_buffer` and
`queue.write_texture` want, without copying — the Rust equivalent of viewing
an `ArrayBuffer` through a `Float32Array`. We covered it in
[the article on memory layout](webgpu-memory-layout.html).

## The texture helpers from the articles

The functions

* `num_mip_levels`
* `load_image`
* `copy_source_to_texture`
* `create_texture_from_source`
* `create_texture_from_image`
* `generate_mips`

were all created in [the article on importing textures](webgpu-importing-textures.html),
and their multi-layer versions

* `copy_sources_to_texture`
* `create_textures_from_sources`

in [the article on cubemaps](webgpu-cube-maps.html). The examples on this
site keep them inline so each example is self-contained, but in your own
project you'd put them in a module or use a crate that provides them.

The JS version of this site uses the
[webgpu-utils](https://github.com/greggman/webgpu-utils) library for the same
purpose.

## wgpu::util

wgpu itself ships a small utility module,
[`wgpu::util`](https://docs.rs/wgpu/latest/wgpu/util/index.html). The most
useful piece is `DeviceExt::create_buffer_init`, which creates a buffer and
fills it with data in one call:

```rust
use wgpu::util::DeviceExt;

let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("vertex buffer vertices"),
    contents: bytemuck::cast_slice(&vertex_data),
    usage: wgpu::BufferUsages::VERTEX,
});
```

The examples on this site spell out `create_buffer` + `write_buffer`
because that's what the articles teach, but `create_buffer_init` is what
you'll usually reach for.

## encase and crevice — automatic WGSL memory layout

As you've seen in all the [fundamental articles](webgpu-fundamentals.html),
as well as the [articles on matrix math](webgpu-matrix-math.html) and
[the articles on lighting](webgpu-lighting-directional.html), when we make
a structure in WGSL, we then usually have to make a uniform buffer or storage
buffer, work out each field's byte offset — including WGSL's alignment
padding — and put data in it at the right offsets.

Every time the WGSL struct changes, all the offsets change, and keeping the
Rust constants in sync with the WGSL by hand is tedious and easy to get
wrong. The JS site solves this with webgpu-utils'
`makeShaderDataDefinitions`/`makeStructuredView`, which parse the WGSL at
runtime. In Rust the same problem is usually solved at compile time:

* [encase](https://docs.rs/encase) — derive `ShaderType` on a Rust struct
  and it lays the data out with WGSL's uniform/storage rules for you:

  ```rust
  #[derive(encase::ShaderType)]
  struct Uniforms {
      world_view_projection: glam::Mat4,
      color: glam::Vec4,
      light_direction: glam::Vec3,
  }
  ```

  `UniformBuffer::write` then produces correctly padded bytes, whatever
  fields you add or remove.

* [crevice](https://docs.rs/crevice) — a similar idea with `AsStd140`.

If you find yourself with more than a couple of uniform structs, use one of
these. The articles on this site do the layout by hand because the layout
rules are exactly what those articles teach.

## What the examples' own helper does

Every example on this site uses the tiny
[`wgpu_fun`](https://github.com/REPO_OWNER/webgpufundamentals-rust/tree/main/rust/wgpu_fun)
crate for window/canvas setup, device creation, the render loop, and the
settings plumbing for the GUI panels — all of it explained in
[the first article](webgpu-fundamentals.html). It's not a library to build
on; it exists so the examples can focus on WebGPU. For real applications,
[winit](https://docs.rs/winit) (windowing) plus the crates above are the
usual foundation.

## More helpers in the articles

Other articles build more helpers you can reuse the same way: premultiplied
alpha support in [the article on transparency and blending](webgpu-transparency.html),
loading 6 images for [cubemaps](webgpu-cubemaps.html) and
[environment maps](webgpu-environment-maps.html), normalized 8-bit vertex
data in [the article on vertex buffers](webgpu-vertex-buffers.html#a-normalized-attributes),
lighting math in the articles from
[directional lighting](webgpu-lighting-direction.html) through
[spot lights](webgpu-lighting-spot.html), and the textured cube used in
examples like [this one](../webgpu-cube.html).
