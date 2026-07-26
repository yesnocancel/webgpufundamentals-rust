Title: WebGPU Data Memory Layout
Description: How to layout and prepare data for WebGPU
TOC: Data Memory Layout

In WebGPU, nearly all of the data you provide to it needs to
be layed out in memory to match what you define in your shaders.
This is less alien to Rust programmers than it is to JavaScript
programmers — if you've ever used `#[repr(C)]` to hand a struct to a
C library you've seen this movie before — but be warned: WGSL has its
own layout rules and they are **not** Rust's rules.

In WGSL when you write your shaders, it's common to define `struct`s.
Structs look very much like Rust structs: you declare members of
a struct, each with a name and a type. But, while the Rust compiler
decides where each member of a Rust struct goes in memory for you,
when providing the data to a WGSL struct **it's up to you** to compute
where in a buffer that particular member of the struct will appear.

In [WGSL](webgpu-wgsl.html) v1, there are 4 base types

* `f32` (a 32bit floating point number)
* `i32` (a 32bit integer)
* `u32` (a 32bit unsigned integer)
* `f16` (a 16bit floating point number) [^f16-optional]

[^f16-optional]: `f16` support is an [optional feature](webgpu-limits-and-features.html)

A byte is 8 bits so a 32 bit value takes 4 bytes and a 16 bit value takes 2 bytes.

If we declare a struct like this

```wgsl
struct OurStruct {
  velocity: f32,
  acceleration: f32,
  frameCount: u32,
};
```

A visual representation of that structure might look something like this

<div class="webgpu_center" data-diagram="ourStructV1"></div>

Each square block is a byte. Above you can see our data takes 12 bytes.
`velocity` takes the first 4 bytes. `acceleration` takes the next 4,
and `frameCount` takes the last 4.

To pass data to the shader we need to prepare data to match the
memory layout of `OurStruct`. To do that we need to make 12 bytes
of memory and then fill out the values with the correct types at the
correct offsets.

```rust
const K_OUR_STRUCT_SIZE_BYTES: usize =
  4 + // velocity
  4 + // acceleration
  4 ; // frameCount
let mut our_struct_data = [0.0f32; K_OUR_STRUCT_SIZE_BYTES / 4];
```

Above, `our_struct_data` is 12 bytes of memory. Rust has no special
`ArrayBuffer` type like JavaScript — any array is just a chunk of memory
— so we declare the memory as an array of 3 `f32`s. (Why not `[u8; 12]`?
Because an array of bytes is only guaranteed to be *aligned* to 1 byte,
and the typed views we're about to make require their memory to be
aligned to the type they view. More on that later.)

To look at the contents of this memory as other types we can create
views of it. Rust already lets us view `our_struct_data` as 32bit
floating point values — it *is* an array of them. For a view of
**the same memory** as 32bit unsigned integer values we use the
[`bytemuck`](https://docs.rs/bytemuck) crate, the same crate we used in
[the fundamentals article](webgpu-fundamentals.html) to turn `f32`s
into bytes for `write_buffer`.

```rust
let our_struct_values_as_u32: &mut [u32] = bytemuck::cast_slice_mut(&mut our_struct_data);
```

`bytemuck::cast_slice_mut` reinterprets a slice of one type as a slice
of another type. Nothing is converted and nothing is copied —
`our_struct_values_as_u32` is a view of **the same memory** as
32bit unsigned integer values.

Now that we have a buffer and views we can set the data in the structure.

```rust
const K_VELOCITY_OFFSET: usize = 0;
const K_ACCELERATION_OFFSET: usize = 1;
const K_FRAME_COUNT_OFFSET: usize = 2;

our_struct_data[K_VELOCITY_OFFSET] = 1.2;
our_struct_data[K_ACCELERATION_OFFSET] = 3.4;

let our_struct_values_as_u32: &mut [u32] = bytemuck::cast_slice_mut(&mut our_struct_data);
our_struct_values_as_u32[K_FRAME_COUNT_OFFSET] = 56;    // an integer value
```

Note the offsets are in `f32`-sized units (4 bytes each), not in bytes,
because we use them to index the typed views.

One Rust-specific wrinkle: the `u32` view *mutably borrows*
`our_struct_data`, so while the view is in use we can't also touch the
`f32` array. That's why, above, we set the two `f32` members first and
made the `u32` view after. The borrow checker never changes what layout
is possible — it just sometimes dictates the order we do things in, or
asks us to make a view right where we use it and let it go out of scope.

## <a id="a-typed-arrays"></a> Arrays, slices, and views

Like many things in programming there are multiple ways we could
set the data for `OurStruct`. In JavaScript this is the job of
`TypedArray`s; in Rust the same jobs are covered by plain arrays,
`Vec`s, slices, and `bytemuck`'s cast functions. For example

* `let data = [0.0f32; 12];`

   This makes **new** memory, in this case of 12 * 4 bytes, zeroed.
   `vec![0.0f32; 12]` is the same thing on the heap. Staging data in an
   array or `Vec` of `f32`s like this is the workhorse of most examples
   on this site.

* `let data = [4.0f32, 5.0, 6.0];`

   This makes **new** memory, in this case of 3 * 4 bytes, and sets the
   initial values to 4, 5, 6.

   To build new memory *from existing values of a different type*, the
   values are converted one at a time — a plain iterator `map` with an
   `as` cast

   ```rust
   let dst: Vec<f32> = src.iter().map(|&v| v as f32).collect();
   ```

   The values are copied by number, not in binary.
   What does "copied by number" mean? Take this example

   ```rust
   let f32s = [0.8f32, 0.9, 1.0, 1.1, 1.2];
   let u32s: Vec<u32> = f32s.iter().map(|&v| v as u32).collect();
   println!("{u32s:?}");   // produces [0, 0, 1, 1, 1]
   ```

   The reason is you can't put values like 0.8 and 1.2 into a `u32`. They get converted to unsigned integers.

* `bytemuck::cast_slice(&data)` and `bytemuck::cast_slice_mut(&mut data)`

   These make a view of **existing memory** as a different type. No
   values are converted, no bytes are copied — the same bits are just
   looked at through different glasses. The element type of the view is
   picked by type inference, or you can spell it out with a turbofish:
   `bytemuck::cast_slice::<f32, u8>(&data)`.

   This is how we get the `&[u8]` that `queue.write_buffer` wants:
   `bytemuck::cast_slice(&data)` where `data` is our `[f32; N]`.

* `&data[begin..end]` (slicing)

   This makes a view of **part of existing memory**. `end` is not
   included, so `&data[5..10]` is a view of elements 5 to 9 of `data` —
   the equivalent of a JavaScript `TypedArray`'s `subarray(5, 10)`.
   Slicing and casting compose: `bytemuck::cast_slice(&data[1..3])`
   views elements 1 and 2 as another type.

Further, Rust arrays and slices have the equivalents of a
`TypedArray`'s properties

* `data.len()`: number of units (`length` in JavaScript)
* `std::mem::size_of_val(&data)`: size in bytes (`byteLength`)
* there are no equivalents of `byteOffset` and `buffer` — a Rust slice
  doesn't remember what it was sliced from. Instead, the borrow checker
  tracks the relationship between a view and the memory it views.

So we could change the code above to this. `split_at_mut` divides one
mutable slice into two non-overlapping mutable slices, which lets us
hold views of all three members at the same time.

```rust
const K_OUR_STRUCT_SIZE_F32_UNITS: usize =
  1 + // velocity
  1 + // acceleration
  1 ; // frameCount
let mut our_struct_data = [0.0f32; K_OUR_STRUCT_SIZE_F32_UNITS];
let (velocity_view, rest) = our_struct_data.split_at_mut(1);
let (acceleration_view, frame_count_rest) = rest.split_at_mut(1);
let frame_count_view: &mut [u32] = bytemuck::cast_slice_mut(frame_count_rest);

velocity_view[0] = 1.2;
acceleration_view[0] = 3.4;
frame_count_view[0] = 56;
```

And Rust has one more option with no JavaScript equivalent: declare a
Rust struct with the exact same memory layout and let the compiler
compute the offsets.

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct OurStruct {
    velocity: f32,
    acceleration: f32,
    frame_count: u32,
}

let our_struct_data = OurStruct {
    velocity: 1.2,
    acceleration: 3.4,
    frame_count: 56,   // an integer value
};
// bytemuck::bytes_of(&our_struct_data) is our 12 bytes,
// ready to hand to queue.write_buffer
```

`#[repr(C)]` tells the Rust compiler to keep the fields in the order we
wrote them and to lay them out by C's rules (without it, Rust is free to
reorder fields however it likes). The `bytemuck::Pod` and
`bytemuck::Zeroable` derives promise that the struct is "plain old
data" — any combination of bytes is a valid value — which is what makes
`bytemuck::bytes_of` (a byte view of a single value, like `cast_slice`
is for slices) safe to use.

A warning before you fall in love with this approach: the compiler
computes *Rust's* offsets, and, as we'll see below, WGSL's rules are
**not** the same as C's rules. They happen to agree for this struct.
For many structs they don't, and you'll have to add WGSL's padding as
explicit fields yourself.

## <a id="multiple-views-of-the-same-arraybuffer"></a>Multiple views of the same memory

Having a view of **the same memory** means exactly that. For example

```rust
let mut v1 = [0.0f32; 5];
let v2 = &mut v1[3..5];  // view the last 2 floats of v1
v2[0] = 123.0;
v2[1] = 456.0;
println!("{v1:?}");  // shows [0.0, 0.0, 0.0, 123.0, 456.0]
```

Similarly if we have different typed views

```rust
let f32s = [1.0f32, 1000.0, -1000.0];
let u32s: &[u32] = bytemuck::cast_slice(&f32s);

println!("{:?}", u32s.iter().map(|v| format!("{v:08x}")).collect::<Vec<_>>());
// shows ["3f800000", "447a0000", "c47a0000"]
```

The values above are the 32bit hex representations of the floating point values for 1, 1000, -1000

For example: Let's create 16 bytes of memory. Then we'll create different
typed views of the same memory.

```rust
let mut array_buffer = [0u64; 2];  // 16 bytes (u64 gives us 8 byte alignment)

// Set some values to start.
bytemuck::cast_slice_mut::<u64, f32>(&mut array_buffer)
    .copy_from_slice(&[123.0, -456.0, 7.8, -0.123]);

let as_i8:  &[i8]  = bytemuck::cast_slice(&array_buffer);
let as_u8:  &[u8]  = bytemuck::cast_slice(&array_buffer);
let as_i16: &[i16] = bytemuck::cast_slice(&array_buffer);
let as_u16: &[u16] = bytemuck::cast_slice(&array_buffer);
let as_i32: &[i32] = bytemuck::cast_slice(&array_buffer);
let as_u32: &[u32] = bytemuck::cast_slice(&array_buffer);
let as_f32: &[f32] = bytemuck::cast_slice(&array_buffer);
let as_f64: &[f64] = bytemuck::cast_slice(&array_buffer);
let as_i64: &[i64] = bytemuck::cast_slice(&array_buffer);
let as_u64: &[u64] = &array_buffer;
```

Notice we set the values through a *mutable* view first, and then made
all the views read-only. Any number of shared views of the same memory
can exist at the same time — it's only mutable views that must be
exclusive.

Here's a representation of all of those views, all viewing the same
memory. Below, edit any one number and the corresponding values that are
using the same memory will change.

<div data-diagram="typedArrays" data-caption="show integers as hex"></div>

## `cast_slice` issues

Be aware, `bytemuck`'s cast functions check their requirements at
runtime and **panic** if they aren't met!

```rust
let bytes: Vec<u8> = std::fs::read("data.bin").unwrap();
let f32s: &[f32] = bytemuck::cast_slice(&bytes);  // might panic!
```

Two things must be true to cast a slice: the total size in bytes must
divide evenly into whole elements of the target type (casting 12 bytes
to `&[f64]` panics), and the memory must be aligned for the target
type. A `Vec<u8>` is only guaranteed to be aligned to 1 byte, so
casting it to `&[f32]` (which needs 4-byte alignment) is a panic
waiting to happen — and worse, allocators usually hand out well-aligned
memory anyway, so this bug loves to hide and only panic once in a
while. That's also why we declared our 16 bytes above as `[0u64; 2]`.

Casting *toward* smaller alignment never has this problem —
`bytemuck::cast_slice::<f32, u8>` can't fail, which is why turning our
`f32` data into bytes for `write_buffer` is always safe.

If you need to reinterpret bytes whose alignment you don't control,
copy them instead

```rust
let f32s: Vec<f32> = bytemuck::pod_collect_to_vec(&bytes);  // Ok, copies
```

## vec and mat types

[WGSL](webgpu-wgsl.html) has types made from the 4 base types.
They are:

<div class="webgpu_center data-table">
  <div>
  <style>
    .wgsl-types tr:nth-child(5n) { height: 1em };
  </style>
  <table class="wgsl-types">
    <thead>
      <tr><th>type</th><th>description</th><th>short name</th><tr>
    </thead>
    <tbody>
      <tr><td><code>vec2&lt;f32&gt;</code></td><td>a type with 2  <code>f32</code>s</td><td><code>vec2f</code></td></tr>
      <tr><td><code>vec2&lt;u32&gt;</code></td><td>a type with 2  <code>u32</code>s</td><td><code>vec2u</code></td></tr>
      <tr><td><code>vec2&lt;i32&gt;</code></td><td>a type with 2  <code>i32</code>s</td><td><code>vec2i</code></td></tr>
      <tr><td><code>vec2&lt;f16&gt;</code></td><td>a type with 2  <code>f16</code>s</td><td><code>vec2h</code></td></tr>
      <tr></tr>
      <tr><td><code>vec3&lt;f32&gt;</code></td><td>a type with 3  <code>f32</code>s</td><td><code>vec3f</code></td></tr>
      <tr><td><code>vec3&lt;u32&gt;</code></td><td>a type with 3  <code>u32</code>s</td><td><code>vec3u</code></td></tr>
      <tr><td><code>vec3&lt;i32&gt;</code></td><td>a type with 3  <code>i32</code>s</td><td><code>vec3i</code></td></tr>
      <tr><td><code>vec3&lt;f16&gt;</code></td><td>a type with 3  <code>f16</code>s</td><td><code>vec3h</code></td></tr>
      <tr></tr>
      <tr><td><code>vec4&lt;f32&gt;</code></td><td>a type with 4  <code>f32</code>s</td><td><code>vec4f</code></td></tr>
      <tr><td><code>vec4&lt;u32&gt;</code></td><td>a type with 4  <code>u32</code>s</td><td><code>vec4u</code></td></tr>
      <tr><td><code>vec4&lt;i32&gt;</code></td><td>a type with 4  <code>i32</code>s</td><td><code>vec4i</code></td></tr>
      <tr><td><code>vec4&lt;f16&gt;</code></td><td>a type with 4  <code>f16</code>s</td><td><code>vec4h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat2x2&lt;f32&gt;</code></td><td>a matrix of 2 <code>vec2&lt;f32&gt;</code>s</td><td><code>mat2x2f</code></td></tr>
      <tr><td><code>mat2x2&lt;f16&gt;</code></td><td>a matrix of 2 <code>vec2&lt;f16&gt;</code>s</td><td><code>mat2x2h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat2x3&lt;f32&gt;</code></td><td>a matrix of 2 <code>vec3&lt;f32&gt;</code>s</td><td><code>mat2x3f</code></td></tr>
      <tr><td><code>mat2x3&lt;f16&gt;</code></td><td>a matrix of 2 <code>vec3&lt;f16&gt;</code>s</td><td><code>mat2x3h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat2x4&lt;f32&gt;</code></td><td>a matrix of 2 <code>vec4&lt;f32&gt;</code>s</td><td><code>mat2x4f</code></td></tr>
      <tr><td><code>mat2x4&lt;f16&gt;</code></td><td>a matrix of 2 <code>vec4&lt;f16&gt;</code>s</td><td><code>mat2x4h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat3x2&lt;f32&gt;</code></td><td>a matrix of 3 <code>vec2&lt;f32&gt;</code>s</td><td><code>mat3x2f</code></td></tr>
      <tr><td><code>mat3x2&lt;f16&gt;</code></td><td>a matrix of 3 <code>vec2&lt;f16&gt;</code>s</td><td><code>mat3x2h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat3x3&lt;f32&gt;</code></td><td>a matrix of 3 <code>vec3&lt;f32&gt;</code>s</td><td><code>mat3x3f</code></td></tr>
      <tr><td><code>mat3x3&lt;f16&gt;</code></td><td>a matrix of 3 <code>vec3&lt;f16&gt;</code>s</td><td><code>mat3x3h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat3x4&lt;f32&gt;</code></td><td>a matrix of 3 <code>vec4&lt;f32&gt;</code>s</td><td><code>mat3x4f</code></td></tr>
      <tr><td><code>mat3x4&lt;f16&gt;</code></td><td>a matrix of 3 <code>vec4&lt;f16&gt;</code>s</td><td><code>mat3x4h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat4x2&lt;f32&gt;</code></td><td>a matrix of 4 <code>vec2&lt;f32&gt;</code>s</td><td><code>mat4x2f</code></td></tr>
      <tr><td><code>mat4x2&lt;f16&gt;</code></td><td>a matrix of 4 <code>vec2&lt;f16&gt;</code>s</td><td><code>mat4x2h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat4x3&lt;f32&gt;</code></td><td>a matrix of 4 <code>vec3&lt;f32&gt;</code>s</td><td><code>mat4x3f</code></td></tr>
      <tr><td><code>mat4x3&lt;f16&gt;</code></td><td>a matrix of 4 <code>vec3&lt;f16&gt;</code>s</td><td><code>mat4x3h</code></td></tr>
      <tr></tr>
      <tr><td><code>mat4x4&lt;f32&gt;</code></td><td>a matrix of 4 <code>vec4&lt;f32&gt;</code>s</td><td><code>mat4x4f</code></td></tr>
      <tr><td><code>mat4x4&lt;f16&gt;</code></td><td>a matrix of 4 <code>vec4&lt;f16&gt;</code>s</td><td><code>mat4x4h</code></td></tr>
    </tbody>
  </table>
  </div>
</div>

Given that a `vec3f` is a type with 3 `f32`s and
`mat4x4f` is an 4x4 matrix of `f32`s, so it's 16 `f32`s,
what do think the following struct looks like in memory?

```wgsl
struct Ex2 {
  scale: f32,
  offset: vec3f,
  projection: mat4x4f,
};
```

Ready?

<div class="webgpu_center" data-diagram="ourStructEx2"></div>

What's up with that? It turns out every type has alignment requirements.
For a given type it must be aligned to a multiple of a certain number
of bytes.

Here are the sizes and alignments of the various types.

<div class="webgpu_center data-table" data-diagram="wgslTypeTable" style="width: 95%; columns: 14em;"></div>

Notice `vec3f` has an alignment of 16 but a size of 12. That's the rule
that pushed `offset` out to byte 16, and left 4 empty bytes between the
end of `offset` and the start of `projection`.

If we wanted to fill out `Ex2` with a `#[repr(C)]` Rust struct, this is
where the two rule sets part ways: Rust would happily put `offset` at
byte 4. We have to write WGSL's padding in ourselves, as explicit
fields. (On the Rust side a `vec3f` is just `[f32; 3]` and a `mat4x4f`
is `[f32; 16]`.)

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Ex2 {
    scale: f32,
    _padding0: [u32; 3],  // 12 bytes so `offset` starts on a 16 byte boundary
    offset: [f32; 3],
    _padding1: u32,       //  4 bytes so `projection` starts on a 16 byte boundary
    projection: [f32; 16],
}
```

The `Pod` derive requires that the struct contain no *implicit* padding
bytes, so everything is at least spelled out in the source — but note
that nothing checks these offsets against your WGSL. If your padding
fields are wrong, your data is silently wrong, exactly like a wrong
manual offset.

But wait, there's MORE!

What do you think the layout of this struct will be?

```wgsl
struct Ex3 {
  transform: mat3x3f,
  directions: array<vec3f, 4>,
};
```

The `array<type, count>` syntax defines an array of `type` with `count` elements.

Here's you go...

<div class="webgpu_center" data-diagram="ourStructEx3"></div>

If you look in the alignment table you'll see `vec3<f32>` has
an alignment of 16 bytes. That means each `vec3<f32>`, whether
it's in a matrix or an array ends up having an extra space.

Here's another one

```wgsl
struct Ex4a {
  velocity: vec3f,
};

struct Ex4 {
  orientation: vec3f,
  size: f32,
  direction: array<vec3f, 1>,
  scale: f32,
  info: Ex4a,
  friction: f32,
};
```

<div class="webgpu_center" data-diagram="ourStructEx4"></div>

Why did `size` end up at byte offset 12, just after orientation but `scale` and
`friction` got bumped offsets 32 and 64

That's because arrays and structs have their own own special alignment rules so
even though the array is a single `vec3f` and the `Ex4a` struct is also a single
`vec3f` they get aligned according to different rules.

<a id="a-struct-array-size-alignment"></a>
<div class="webgpu_center data-table">
  <div>
  <style>
    .wgsl-types tr:nth-child(5n) { height: 1em };
  </style>
  <table class="wgsl-types">
    <thead>
      <tr><th>type</th><th>align</th><th>size</th><tr>
    </thead>
    <tbody>
      <tr><td><code>struct</code> S with members M<sub>1</sub>...M<sub>N</sub></td><td>max(AlignOfMember(S,1), ... , AlignOfMember(S,N))</td><td>roundUp(AlignOf(S), justPastLastMember)

where justPastLastMember = OffsetOfMember(S,N) + SizeOfMember(S,N)</td></tr>
      <tr><td><code>array&lt;E, N&gt;</code></td><td>AlignOf(E)</td><td>N × roundUp(AlignOf(E), SizeOf(E))</td></tr>
    </tbody>
  </table>
  </div>
</div>

You can read the rules in more detail [here in the WGSL spec](https://www.w3.org/TR/WGSL/#alignment-and-size).

# Computing Offset and Sizes is a PITA!

Computing sizes and offsets of data in WGSL is probably the largest pain point
of WebGPU. You are required to compute these offsets yourself and keep them up
to date. If you add a member somewhere in the middle of a struct in your shaders
you need to go back to your Rust and update all the offsets and padding fields.
Get a single byte or length wrong and the data you pass to the shader will be
wrong. You won't get an error, but your shader will likely do the wrong thing
because it's looking at bad data. Your model won't draw or your computation will
produce bad results.

Fortunately there are crates to help with this.

Here are two: [encase](https://crates.io/crates/encase) and
[crevice](https://crates.io/crates/crevice)

Where the equivalent JavaScript library,
[webgpu-utils](https://github.com/greggman/webgpu-utils), parses your WGSL
code and computes the offsets from it, these crates work from your Rust
struct: you derive a trait on it and the crate lays your data out
according to WGSL's rules when it writes the bytes — no manual padding
fields, no manual offsets. This way you can change your structs and,
more often than not, things will just work.

For example, using that last example we can write it with `encase`
like this

```rust
use encase::{ShaderType, UniformBuffer};

#[derive(ShaderType)]
struct Ex4a {
    velocity: glam::Vec3,
}

#[derive(ShaderType)]
struct Ex4 {
    orientation: glam::Vec3,
    size: f32,
    direction: [glam::Vec3; 1],
    scale: f32,
    info: Ex4a,
    friction: f32,
}

// Set some values
let my_uniform_values = Ex4 {
    orientation: glam::vec3(1.0, 0.0, -1.0),
    size: 2.0,
    direction: [glam::vec3(0.0, 1.0, 0.0)],
    scale: 1.5,
    info: Ex4a {
        velocity: glam::vec3(2.0, 3.0, 4.0),
    },
    friction: 0.1,
};

let mut buffer = UniformBuffer::new(Vec::<u8>::new());
buffer.write(&my_uniform_values).unwrap();

// now pass buffer.into_inner() to WebGPU when needed.
```

`crevice` does the same job via its `AsStd140` / `AsStd430` derives
(std140 and std430 are the GLSL layout rule sets, which match WGSL's
uniform and storage rules for most types).

One caveat compared to `webgpu-utils`: these crates compute the layout
from your *Rust* struct, so it's still on you to keep the Rust struct
and the WGSL struct in sync — but adding a member in the middle no
longer silently shifts every offset after it.

Whether you use one of these crates or a different one or
none at all is up to you. For me, I would often spent 20-30-60 minutes
trying to figure out why something was not working only to find
that I manually computed an offset or size wrong so for my own work
I'd rather use a library and avoid that pain.

If you do want to do it manually though, 
[here's a page that will compute the offsets for you](resources/wgsl-offset-computer.html).

Otherwise, there are many libraries to help abstract webgpu
and make things like this, and others, easier. You can find
a list [here](webgpu-resources.html).

<!-- keep this at the bottom of the article -->
<link rel="stylesheet" href="webgpu-memory-layout.css">
<script type="module" src="webgpu-memory-layout.js"></script>
