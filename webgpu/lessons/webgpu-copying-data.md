Title: WebGPU Copying Data
Description: Copying Data to/from buffers and textures
TOC: Copying Data

In most of the articles to date, we've used the functions
`write_buffer` to put data in a buffer and `write_texture`
to put data in a texture. There are several ways to put
data into a buffer or a texture.

## `write_buffer`

`write_buffer` copies data from a slice of bytes in our program to a buffer.
This is arguably the most straight forward way to get data into a buffer.

`write_buffer` follows this format

```rust
queue.write_buffer(
  &dest_buffer, // the buffer to write to
  dest_offset,  // where in the destination buffer to start writing
  src_data,     // the data to write, as bytes (&[u8])
);
```

The data is always a `&[u8]`; as we've seen before, we use
`bytemuck::cast_slice` to view our typed data as bytes.

> Important: In the JavaScript API, `writeBuffer` takes 2 more optional
> parameters, `srcOffset` and `size`, which select a portion of the source
> data in **elements**. In Rust there are no extra parameters. Instead, you
> select the portion of the source data with an ordinary slice.
>
> In other words,
>
> ```rust
> queue.write_buffer(
>   &some_buffer,
>   some_offset,
>   bytemuck::cast_slice(&some_f32_data[6..6 + 7]),
> );
> ```
>
> the code above will copy from f32 #6, 7 f32s of data.
> To put it another way it will copy 28 bytes starting at byte 24
> of the data that `some_f32_data` refers to.

## `write_texture`

`write_texture` copies data from a slice of bytes in our program to a texture.

`write_texture` has this signature

```rust
queue.write_texture(
  // details of the destination
  wgpu::TexelCopyTextureInfo {
    texture: &texture,
    mip_level: 0,
    origin: wgpu::Origin3d::ZERO,
    aspect: wgpu::TextureAspect::All,
  },

  // the source data
  src_data,

  // details of the source data
  wgpu::TexelCopyBufferLayout {
    offset: 0,
    bytes_per_row: Some(bytes_per_row),
    rows_per_image: Some(rows_per_image),
  },

  // size:
  wgpu::Extent3d { width, height, depth_or_array_layers },
);
```

Things to note:

* `texture` must have a usage of `TextureUsages::COPY_DST`

* `mip_level`, `origin`, and `aspect` above are all at their default values.
  For the extremely common case of "the whole texture at mip level 0" there
  is a shortcut: `texture.as_image_copy()` returns exactly the
  `TexelCopyTextureInfo` shown above.

* `bytes_per_row`: This is how many bytes to advance to get to the next *block row* of data.

   This is required if you are copying more than 1 *block row* — which is why
   it's an `Option<u32>`. It is almost always true that you're copying more
   than 1 *block row* so it is therefore almost always `Some`.

* `rows_per_image`: This is the number of *block rows* to advance to get from the
   the start of one image to the next image.

   This is required if you are copying more than 1 layer. In other words,
   if `depth_or_array_layers` in the size argument is > 1 then you need to supply
   this value.

You can think of the copy as working like this

```rust
// pseudo code
let (x, y, z) = origin;
let (block_width, block_height, bytes_per_block) =
    get_block_info_for_texture_format(texture.format());

let blocks_across = width / block_width;
let blocks_down = height / block_height;
let bytes_per_block_row = blocks_across * bytes_per_block;

for layer in 0..depth_or_array_layers {
  for row in 0..blocks_down {
    let start = offset + (layer * rows_per_image + row) * bytes_per_row;
    copy_row_to_texture(
        &texture,              // texture to copy to
        x, y + row, z + layer, // where in texture to copy to
        &src_data_as_bytes[start..],
        bytes_per_block_row);
  }
}
```

### <a id="a-block-rows"></a>**block row**

Textures are organized into blocks. For most *regular* textures the block width
and block height are both 1. For compressed textures that changes. For example
the format, `Bc1RgbaUnorm` has a block width of 4 and a block height of 4.
That means if you set the width to 8, and the height to 12, only 6 blocks will be copied.
2 blocks for the first row, 2 for the 2nd row, 2 for the 3rd.

For compressed textures, size and origin must be aligned to blocks sizes.

> Important: Anywhere WebGPU takes a size (defined in the spec as a
> `GPUExtent3D`), wgpu takes an `Extent3d` struct. `height` and
> `depth_or_array_layers` default to 1, and `Extent3d` implements `Default`,
> so with struct update syntax
>
> * `wgpu::Extent3d { width: 2, ..Default::default() }` a size where width = 2, height = 1, depth_or_array_layers = 1
> * `wgpu::Extent3d { width: 2, height: 3, ..Default::default() }` a size where width = 2, height = 3, depth_or_array_layers = 1
> * `wgpu::Extent3d { width: 2, height: 3, depth_or_array_layers: 4 }` a size where width = 2, height = 3, depth_or_array_layers = 4

> In the same way, anywhere an origin appears (a `GPUOrigin3D` in the spec),
> wgpu takes an `Origin3d` struct with `x`, `y`, `z` fields. All of them
> default to 0 so
>
> * `wgpu::Origin3d { x: 5, ..Default::default() }` an origin where x = 5, y = 0, z = 0
> * `wgpu::Origin3d { x: 5, y: 6, ..Default::default() }` an origin where x = 5, y = 6, z = 0
> * `wgpu::Origin3d { x: 5, y: 6, z: 7 }` an origin where x = 5, y = 6, z = 7
>
> There's also `wgpu::Origin3d::ZERO` for the very common all-zeros case.

* `aspect` really only comes into play when copying data to a depth-stencil format.
  You can only copy to one aspect at a time, either the `TextureAspect::DepthOnly`
  or the `TextureAspect::StencilOnly`.

> Trivia: A texture has a `size()` method that returns its size as an
> `Extent3d`, and an `as_image_copy()` method that returns a whole-texture
> `TexelCopyTextureInfo`. In other words, given this texture
>
> ```rust
> let texture = device.create_texture(&wgpu::TextureDescriptor {
>   label: None,
>   format: wgpu::TextureFormat::R8Unorm,
>   size: wgpu::Extent3d { width: 2, height: 4, ..Default::default() },
>   usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
>   mip_level_count: 1,
>   sample_count: 1,
>   dimension: wgpu::TextureDimension::D2,
>   view_formats: &[],
> });
> ```
>
> all of these work
>
> ```rust
> // copy 2x4 pixels of data to texture
> let layout = wgpu::TexelCopyBufferLayout {
>   offset: 0,
>   bytes_per_row: Some(2),
>   rows_per_image: None,
> };
> queue.write_texture(texture.as_image_copy(), data, layout,
>     wgpu::Extent3d { width: 2, height: 4, ..Default::default() });
> queue.write_texture(texture.as_image_copy(), data, layout,
>     wgpu::Extent3d { width: texture.width(), height: texture.height(), ..Default::default() });
> queue.write_texture(texture.as_image_copy(), data, layout, texture.size()); // !!!
> ```
>
> That last one works because `texture.size()` is an `Extent3d` holding the
> texture's own `width`, `height`, and `depth_or_array_layers`.

## `copy_buffer_to_buffer`

`copy_buffer_to_buffer`, like the name suggests, copies data from one buffer to another.

signature:

```rust
encoder.copy_buffer_to_buffer(
  &source,       // buffer to copy from
  source_offset, // where to start copying from
  &dest,         // buffer to copy to
  dest_offset,   // where to start copying to
  size,          // how many bytes to copy
);
```

* `source` must have a usage of `BufferUsages::COPY_SRC`
* `dest` must have a usage of `BufferUsages::COPY_DST`
* `size` must be a multiple of 4

One wgpu nicety: `size` is actually an `Option<u64>` (the parameter accepts
either). If you pass `None` it means "from `source_offset` to the end of the
source buffer".

## `copy_buffer_to_texture`

`copy_buffer_to_texture`, like the name suggests, copies data from a buffer to a texture.

signature:

```rust
encoder.copy_buffer_to_texture(
  // details of the source buffer
  wgpu::TexelCopyBufferInfo {
    buffer: &buffer,
    layout: wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(bytes_per_row),
      rows_per_image: Some(rows_per_image),
    },
  },

  // details of the destination texture
  wgpu::TexelCopyTextureInfo {
    texture: &texture,
    mip_level: 0,
    origin: wgpu::Origin3d::ZERO,
    aspect: wgpu::TextureAspect::All,
  },

  // size:
  wgpu::Extent3d { width, height, depth_or_array_layers },
);
```

This has almost exactly the same parameters as `write_texture`.
The biggest difference is that `bytes_per_row` **must be
a multiple of 256!!**

* `texture` must have a usage of `TextureUsages::COPY_DST`
* `buffer` must have a usage of `BufferUsages::COPY_SRC`

## `copy_texture_to_buffer`

`copy_texture_to_buffer` like the name suggests, copies data from a texture to a buffer.

signature:

```rust
encoder.copy_texture_to_buffer(
  // details of the source texture
  wgpu::TexelCopyTextureInfo {
    texture: &texture,
    mip_level: 0,
    origin: wgpu::Origin3d::ZERO,
    aspect: wgpu::TextureAspect::All,
  },

  // details of the destination buffer
  wgpu::TexelCopyBufferInfo {
    buffer: &buffer,
    layout: wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(bytes_per_row),
      rows_per_image: Some(rows_per_image),
    },
  },

  // size:
  wgpu::Extent3d { width, height, depth_or_array_layers },
);
```

This has similar parameters to `copy_buffer_to_texture`
just the texture (now the source) and the buffer (now the destination)
are swapped. Like `copy_buffer_to_texture`, `bytes_per_row` **must be
a multiple of 256!!**

* `texture` must have a usage of `TextureUsages::COPY_SRC`
* `buffer` must have a usage of `BufferUsages::COPY_DST`

## `copy_texture_to_texture`

`copy_texture_to_texture` copies a portion of one texture to another.

The two textures must be must either be the same format, or they
must only differ in their `-srgb`-ness (for example `Rgba8Unorm` and
`Rgba8UnormSrgb`).

signature:

```rust
encoder.copy_texture_to_texture(
  // details of the source texture
  wgpu::TexelCopyTextureInfo {
    texture: &src_texture,
    mip_level: 0,
    origin: wgpu::Origin3d::ZERO,
    aspect: wgpu::TextureAspect::All,
  },

  // details of the destination texture
  wgpu::TexelCopyTextureInfo {
    texture: &dst_texture,
    mip_level: 0,
    origin: wgpu::Origin3d::ZERO,
    aspect: wgpu::TextureAspect::All,
  },

  // size:
  wgpu::Extent3d { width, height, depth_or_array_layers },
);
```

* src.`texture` must have a usage of `TextureUsages::COPY_SRC`
* dst.`texture` must have a usage of `TextureUsages::COPY_DST`
* `width` must be a multiple of block width
* `height` must be a multiple of block height
* src.`origin.x` must be a multiple of block width
* src.`origin.y` must be a multiple of block height
* dst.`origin.x` must be a multiple of block width
* dst.`origin.y` must be a multiple of block height

## Shaders

Shaders can read and write to storage buffers, storage textures,
and indirectly they can render to textures. Those are all ways
of getting data into buffers and textures. In other words
you can write shaders to generate and/or copy and transfer data.

## Mapping Buffers

You can map a buffer. Mapping a buffer means making it
available to read or write from your own code, on the CPU.
At least in version 1 of WebGPU,
mappable buffers have severe restrictions, namely, a
mappable buffer can only be used as a temporary place
to copy from or to. A mappable buffer can not be used as any
other type of buffer (like a uniform buffer, vertex buffer,
index buffer, storage buffer, etc...) [^mappedAtCreation]

[^mappedAtCreation]: The exception is if you set `mapped_at_creation: true`
See [mapped_at_creation](#a-mapped-at-creation).

You can create a mappable buffer with 2 combinations
of usage flags.

* `BufferUsages::MAP_READ | BufferUsages::COPY_DST`

  This is a buffer you can use the copy commands above to copy
  data from another buffer or a texture, then map it to
  read the values from your program

* `BufferUsages::MAP_WRITE | BufferUsages::COPY_SRC`

  This is a buffer you can map, you can then put
  data in it from your program, and finally unmap it and use
  the copy commands above to copy its contents to another
  buffer or texture.

The process of mapping a buffer is asynchronous. You call
`buffer.map_async(mode, range, callback)` where `range`
is a byte range like `0..size`, or `..` for
the entire buffer. `mode` must be either
`MapMode::Read` or `MapMode::Write` and must of course
match the `MAP_` usage flag you passed in when you created
the buffer.

In the JavaScript API, `mapAsync` returns a `Promise`. wgpu's raw
`map_async` instead takes a callback (and on native we also have to keep the
device polling), so, like we did in
[the first article](webgpu-fundamentals.html#a-run-computations-on-the-gpu),
we use the small `wgpu_fun::map_async` helper that wraps the callback in a
future we can simply `await`. When the future resolves the buffer is mapped.
You can then view some or all of the buffer by calling
`buffer.slice(range).get_mapped_range()`, where `range` selects a portion of
the buffer you mapped. `get_mapped_range` returns a `BufferView`, which acts
as a `&[u8]`, so generally, to be of any use, you'd use
`bytemuck::cast_slice` to view those bytes as your actual data type.

Here's one example of mapping a buffer

```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
  label: None,
  size: 1024,
  usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
  mapped_at_creation: false,
});

// map the entire buffer
wgpu_fun::map_async(&device, &buffer, wgpu::MapMode::Read).await;

{
  // get the entire buffer as a slice of 32bit floats.
  let view = buffer.slice(..).get_mapped_range().unwrap();
  let f32s: &[f32] = bytemuck::cast_slice(&view);

  ...
}

buffer.unmap();
```

Note: Once mapped, the buffer is not usable by WebGPU until you call `unmap`,
and every view of the mapped data must be dropped before you call `unmap` —
wgpu will panic otherwise. That's why the view above lives in its own `{ }`
block. In JavaScript, `unmap` silently makes the mapped data vanish out from
under you; in Rust, the borrow checker makes sure nothing can still be
looking at it. In other words, take the example above

```rust
let view = buffer.slice(..).get_mapped_range().unwrap();
let f32s: &[f32] = bytemuck::cast_slice(&view);

println!("{}", f32s[0]); // prints the first value

drop(view);              // compile error! we can't get rid of `view`
buffer.unmap();          // while `f32s` still borrows from it

println!("{}", f32s[0]);
```

(In JavaScript, that last `console.log` would print `undefined`; in Rust the
program simply doesn't compile.)

We've already seen examples of mapping a buffer for reading
in [the first article](webgpu-fundamentals.html#a-run-computations-on-the-gpu) where we doubled some numbers
in a storage buffer and the copied the results to a mappable buffer and mapped it to read out the results

Another example is the article on [compute shader basics](webgpu-compute-shaders.md)
where we output the various `@builtin` compute shader values to a storage buffer.
We then copied those results to a mappable buffer and mapped it read out the results.

## <a id="a-mapped-at-creation"></a>mapped_at_creation

`mapped_at_creation: true` is a flag you can add when you
create a buffer. In this case, the buffer does not need
the usage flags `BufferUsages::COPY_DST` nor `BufferUsages::MAP_WRITE`.

This is a special flag solely to let you put data in the
buffer on creation. You add the flag `mapped_at_creation: true` when you create the
buffer. The buffer is created, already mapped for writing. Example:

```rust
let buffer = device.create_buffer(&wgpu::BufferDescriptor {
  label: None,
  size: 16,
  usage: wgpu::BufferUsages::UNIFORM,
  mapped_at_creation: true,
});
{
  let mut view = buffer.slice(..).get_mapped_range_mut().unwrap();
  view.copy_from_slice(bytemuck::cast_slice(&[1.0f32, 2.0, 3.0, 4.0]));
}
buffer.unmap();
```

`get_mapped_range_mut` returns a `BufferViewMut`. Unlike the read view, it's
*write-only* — mapped memory can be
[write-combining](https://en.wikipedia.org/wiki/Write_combining) memory,
which is extremely slow to read, so wgpu doesn't let you. `copy_from_slice`
copies your bytes into the buffer, which is all we need here.

Or, more tersely, wgpu ships a helper that does exactly the
create-mapped/copy/unmap dance above, `create_buffer_init`:

```rust
use wgpu::util::DeviceExt;

let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
  label: None,
  contents: bytemuck::cast_slice(&[1.0f32, 2.0, 3.0, 4.0]),
  usage: wgpu::BufferUsages::UNIFORM,
});
```

Note that a buffer created with `mapped_at_creation: true` does not have
any flags set automatically. It is just a convenience for putting data
in the buffer when you first create it. It's mapped at creation, and
after you unmap it once, it behaves like any other buffer and will only
work for the usages you specified. In other words, if you to want to copy
to it later you need `BufferUsages::COPY_DST` or if you want to map it
later you need `BufferUsages::MAP_READ` or `BufferUsages::MAP_WRITE`.

## <a id="a-efficient"></a>Efficiently using mappable buffers

Above we saw that mapping a buffer is asynchronous. This means there's
an indeterminate amount of time from the point we ask for the buffer
to be mapped by calling `map_async`, until the time it's mapped and we can call `get_mapped_range`.

A common way to workaround this is to keep a set of buffers always mapped.
Since they are already mapped they are ready to use immediately. As soon
as you use one and unmap it, and as soon as you've submitted whatever
commands use the buffer, you ask for it to be mapped again. When its future
resolves, you put it back in a pool of already mapped buffers. If you ever
need a mapped buffer and none are available you create a new one and add
it to the pool.
