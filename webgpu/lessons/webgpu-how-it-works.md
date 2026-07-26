Title: WebGPU How It Works
Description: How WebGPU works
TOC: How It Works

Let's try to explain WebGPU by implementing something similar to what the GPU does
with vertex shaders and fragment shaders but in plain Rust. Hopefully this will give
you an intuitive feeling about what's really going on.

If you're familiar with
[`Iterator::map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.map),
if you squint real hard you can get some idea of how these 2 different kinds of
shader functions work. With `map` you provide a function to transform a value.

Example:

```rust
let shader = |v: f32| v * 2.0;  // double the input
let input = [1.0, 2.0, 3.0, 4.0];
let output: Vec<f32> = input.into_iter().map(shader).collect();  // result [2.0, 4.0, 6.0, 8.0]
```

Above our "shader" for `map` is just a function that given a number, returns
its double. That's probably the closest analogy in Rust to what "shader"
means. It's a function that returns or generates values. You don't call it
directly. Instead, you specify it and then the system calls it for you.

For a GPU vertex shader you don't map over an input array. Instead, you just
specify a count of how many times you want the function to be called.

```rust
fn draw(count: usize, vertex_shader_fn: impl Fn(usize) -> f32) {
  let mut internal_buffer = Vec::new();
  for i in 0..count {
    internal_buffer.push(vertex_shader_fn(i));
  }
  println!("{internal_buffer:?}");
}
```

One consequence is that unlike `map`, we no longer need a source array to do
something. The only input our shader gets is the iteration number.

```rust
let shader = |i: usize| (i * 2) as f32;
let count = 4;
draw(count, shader);
// outputs [0.0, 2.0, 4.0, 6.0]
```

The thing that makes GPU work complicated is that these functions run on a separate
system in your computer, the GPU. This means all the data you create and reference
has to somehow be sent to the GPU and you then need to communicate to the shader
where you put that data and how to access it.

Vertex And Fragment shaders can take data in 6 ways. Uniforms, Attributes, Buffers, Textures, Inter-Stage Variables, Constants.

1. Uniforms

   Uniforms are values that are the same for each iteration of the shader. Think
   of them as constant global variables. You can set them before a shader is run
   but, while the shader is being used, they remain constant, or to put it
   another way, they remain *uniform*.

   Let's change `draw` to pass uniforms to a shader. To do this we'll
   make a slice called `bindings` and use it to pass in the uniforms.

   ```rust
   *fn draw(
   *  count: usize,
   *  vertex_shader_fn: impl Fn(usize, &[Uniforms]) -> f32,
   *  bindings: &[Uniforms],
   *) {
     let mut internal_buffer = Vec::new();
     for i in 0..count {
   *    internal_buffer.push(vertex_shader_fn(i, bindings));
     }
     println!("{internal_buffer:?}");
   }
   ```

   And then let's change our shader to use the uniforms

   ```rust
   struct Uniforms {
     multiplier: f32,
   }

   let vertex_shader = |i: usize, bindings: &[Uniforms]| {
     let uniforms = &bindings[0];
     i as f32 * uniforms.multiplier
   };
   let count = 4;
   let uniforms1 = Uniforms { multiplier: 3.0 };
   let uniforms2 = Uniforms { multiplier: 5.0 };
   let bindings1 = [uniforms1];
   let bindings2 = [uniforms2];
   draw(count, vertex_shader, &bindings1);
   // outputs [0.0, 3.0, 6.0, 9.0]
   draw(count, vertex_shader, &bindings2);
   // outputs [0.0, 5.0, 10.0, 15.0]
   ```

   So, the concept of uniforms hopefully seems pretty straight forward. The
   indirection through `bindings` is there because this is "similar" to how things
   are done in WebGPU. Like was mentioned above, we access the things, in this case
   the uniforms, by location/index. Here they are found in `bindings[0]`.

2. Attributes (vertex shaders only)

   Attributes provide per shader iteration data. In `map` above,
   the value `v` was pulled from `input` and automatically provided
   to the function. This is very similar to an attribute in a shader.

   The difference is, we are not mapping over the input, instead,
   because we are just counting, we need to tell WebGPU
   about these inputs and how to get data out of them.

   Imagine we updated `draw` like this.

   ```rust
   +struct Attrib<'a> {
   +  source: &'a [f32],
   +  offset: usize,
   +  stride: usize,
   +}

   *fn draw(
   *  count: usize,
   *  vertex_shader_fn: impl Fn(usize, &[&[f32]], &[f32]) -> f32,
   *  bindings: &[&[f32]],
   *  attribs_spec: &[Attrib],
   *) {
     let mut internal_buffer = Vec::new();
     for i in 0..count {
   *    let attribs = get_attribs(attribs_spec, i);
   *    internal_buffer.push(vertex_shader_fn(i, bindings, &attribs));
     }
     println!("{internal_buffer:?}");
   }

   +fn get_attribs(attribs: &[Attrib], ndx: usize) -> Vec<f32> {
   +  attribs.iter().map(|a| a.source[ndx * a.stride + a.offset]).collect()
   +}
   ```

   Then we could call it like this.

   ```rust
   let buffer1 = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
   let buffer2 = [11.0, 22.0, 33.0, 44.0];
   let attribs_spec = [
     Attrib { source: &buffer1, offset: 0, stride: 2 },
     Attrib { source: &buffer1, offset: 1, stride: 2 },
     Attrib { source: &buffer2, offset: 0, stride: 1 },
   ];
   let vertex_shader = |_i: usize, _bindings: &[&[f32]], attribs: &[f32]|
       (attribs[0] + attribs[1]) * attribs[2];
   let bindings: &[&[f32]] = &[];
   let count = 4;
   draw(count, vertex_shader, bindings, &attribs_spec);
   // outputs [11.0, 110.0, 297.0, 572.0]
   ```

   As you can see above, `get_attribs` uses `offset`, and `stride` to
   compute indices into the corresponding `source` buffer and pulls out values.
   The pulled out values are then sent to the shader. On each iteration
   `attribs` will be different.

   ```
    iteration |  attribs
    ----------+------------------
        0     | [0.0, 1.0, 11.0]
        1     | [2.0, 3.0, 22.0]
        2     | [4.0, 5.0, 33.0]
        3     | [6.0, 7.0, 44.0]
   ```

3. Raw Buffers

   Buffers are effectively arrays, again for our analogy let's make a version
   of `draw` that uses buffers. We'll pass these buffers via `bindings`
   like we did with uniforms — which means, this time, `bindings` is a slice
   of buffers, a `&[&[f32]]`.

   ```rust
   let buffer1 = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
   let buffer2 = [11.0, 22.0, 33.0, 44.0];
   let attribs_spec: &[Attrib] = &[];
   let bindings: &[&[f32]] = &[
     &buffer1,
     &buffer2,
   ];
   let vertex_shader = |ndx: usize, bindings: &[&[f32]], _attribs: &[f32]|
       (bindings[0][ndx * 2] + bindings[0][ndx * 2 + 1]) * bindings[1][ndx];
   let count = 4;
   draw(count, vertex_shader, bindings, attribs_spec);
   // outputs [11.0, 110.0, 297.0, 572.0]
   ```

   Here we got the same result as we did with attributes except this time,
   instead of the system pulling the values out of the buffers for us, we
   calculated our own indices into the bound buffers. This is more flexible than
   attributes since we basically have random access to the arrays. But, it's
   potentially slower for that same reason. Given the way attributes worked the
   GPU knows the values will be accessed in order which it can use to optimize.
   For example, in order access is usually cache friendly. When we calculate our
   own indices the GPU has no idea which part of a buffer we're going to access
   until we actually try to access it.

4. Textures

   Textures are 1d, 2d, or 3d arrays of data. Of course, we could implement
   our own 2d or 3d arrays using buffers. What's special about textures
   is they can be sampled. Sampling means that we can ask the GPU to compute
   a value between the values we supply. We'll cover that this means in
   [the article on textures](webgpu-textures.html). For now, let's make
   a Rust analogy again.

   First we'll create a function `texture_sample` that *samples* an array
   between values.

   ```rust
   fn texture_sample(texture: &[f32], ndx: f32) -> f32 {
     let start_ndx = ndx as usize;      // round down to an int
     let fraction = ndx % 1.0;          // get the fractional part between indices
     let start = texture[start_ndx];
     let end = texture[start_ndx + 1];
     start + (end - start) * fraction   // compute value between start and end
   }
   ```

   A function something like that already exists on the GPU.

   Now let's use that in a shader.

   ```rust
   let texture = [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
   let attribs_spec: &[Attrib] = &[];
   let bindings: &[&[f32]] = &[
     &texture,
   ];
   let vertex_shader = |ndx: usize, bindings: &[&[f32]], _attribs: &[f32]|
       texture_sample(bindings[0], ndx as f32 * 1.75);
   let count = 4;
   draw(count, vertex_shader, bindings, attribs_spec);
   // outputs [10.0, 27.5, 45.0, 62.5]
   ```

   When `ndx` is `3` we'll pass in `3 * 1.75` or `5.25` into `texture_sample`.
   That will compute a `start_ndx` of `5`. So we'll pull out indices `5` and `6`
   which are `60` and `70`. `fraction` becomes `0.25`, so we'll get
   `60 + (70 - 60) * 0.25` which is `62.5`.

   Looking at the code above we could write `texture_sample` ourselves in our shader
   function. We could manually pull out the 2 values and interpolate between them.
   The reason the GPU has this special functionality is it can do it much faster
   and, depending on the settings, it may read as many as sixteen 4-float values
   to produce one 4-float value for us. That would be a lot of work to do manually.

5. Inter-Stage Variables (fragment shaders only)

   Inter-Stage Variables are outputs from a vertex shader to a fragment shader. As was mentioned
   above, a vertex shader outputs positions that are used to draw/rasterize points,
   lines, and triangles. 
   
   Let's imagine we're drawing a line. Let's say our vertex shader was run
   twice, the first time it output the equivalent of `5,0` and the second time
   the equivalent of `25,4`. Given those 2 points the GPU will draw a line from
   `5,0` to `25,4` exclusive. To do this it will call our fragment shader 20
   times, once for each of the pixels on that line. Each time it calls our
   fragment shader it's up to us to decide what color to return.

   Let's assume we have pair of functions that help us draw a line between
   2 points. The first function computes how many pixel's we need to draw and some
   values to help draw them. The second takes that info plus a pixel number
   and gives us a pixel position. Example:

   ```rust
   let line = calc_line(&[10.0, 10.0], &[13.0, 13.0]);
   for i in 0..line.num_pixels {
     let p = calc_line_point(&line, i);
     println!("{},{}", p[0], p[1]);
   }
   // prints
   // 10,10
   // 11,11
   // 12,12
   ```
   
   Note: How `calc_line` and `calc_line_point` work are unimportant, what's
   important is that they do work and let the loop above provide
   the pixel positions for a line. **Though if you're curious, see the live
   code example near the bottom of the article.**

   So, let's change our vertex shader so it outputs 2 values per iteration. We
   could do that in many ways. Here's one. (In Rust that also means updating
   the vertex shader function type `draw` accepts so it returns an `[f32; 2]`
   instead of an `f32`.)

   ```rust
   let buffer1 = [5.0, 0.0, 25.0, 4.0];
   let attribs_spec = [
     Attrib { source: &buffer1, offset: 0, stride: 2 },
     Attrib { source: &buffer1, offset: 1, stride: 2 },
   ];
   let bindings: &[&[f32]] = &[];
   let vertex_shader = |_ndx: usize, _bindings: &[&[f32]], attribs: &[f32]|
       [attribs[0], attribs[1]];
   let count = 2;
   draw(count, vertex_shader, bindings, &attribs_spec);
   // outputs [[5.0, 0.0], [25.0, 4.0]]
   ```

   Now let's write some code that loops over points 2 at a time and 
   calls `rasterize_lines` to rasterize a line.

   ```rust
   fn rasterize_lines(
     dest: &mut [i32],
     dest_width: usize,
     inputs: &[[f32; 2]],
     frag_shader_fn: impl Fn(&[&[f32]]) -> i32,
     bindings: &[&[f32]],
   ) {
     for ndx in (0..inputs.len() - 1).step_by(2) {
       let p0 = &inputs[ndx    ];
       let p1 = &inputs[ndx + 1];
       let line = calc_line(p0, p1);
       for i in 0..line.num_pixels {
         let p = calc_line_point(&line, i);
         let offset = p[1] as usize * dest_width + p[0] as usize;  // y * width + x
         dest[offset] = frag_shader_fn(bindings);
       }
     }
   }
   ```

   We can update `draw` to use that code like this

   ```rust
   -fn draw(
   -  count: usize,
   -  vertex_shader_fn: impl Fn(usize, &[&[f32]], &[f32]) -> [f32; 2],
   -  bindings: &[&[f32]],
   -  attribs_spec: &[Attrib],
   -) {
   +fn draw(
   +  dest: &mut [i32], dest_width: usize,
   +  count: usize,
   +  vertex_shader_fn: impl Fn(usize, &[&[f32]], &[f32]) -> [f32; 2],
   +  fragment_shader_fn: impl Fn(&[&[f32]]) -> i32,
   +  bindings: &[&[f32]],
   +  attribs_spec: &[Attrib],
   +) {
     let mut internal_buffer = Vec::new();
     for i in 0..count {
       let attribs = get_attribs(attribs_spec, i);
       internal_buffer.push(vertex_shader_fn(i, bindings, &attribs));
     }
   -  println!("{internal_buffer:?}");
   +  rasterize_lines(dest, dest_width, &internal_buffer,
   +                  fragment_shader_fn, bindings);
   }
   ```

   Now we're actually using `internal_buffer` 😃!
   
   Let's update the code that calls `draw`.

   ```rust
   let buffer1 = [5.0, 0.0, 25.0, 4.0];
   let attribs_spec = [
     Attrib { source: &buffer1, offset: 0, stride: 2 },
     Attrib { source: &buffer1, offset: 1, stride: 2 },
   ];
   let bindings: &[&[f32]] = &[];
   let vertex_shader = |_ndx: usize, _bindings: &[&[f32]], attribs: &[f32]|
       [attribs[0], attribs[1]];
   let count = 2;
   -draw(count, vertex_shader, bindings, &attribs_spec);

   +let width = 30;
   +let height = 5;
   +let mut pixels = vec![0; width * height];
   +let frag_shader = |_bindings: &[&[f32]]| 6;

   *draw(
   *   &mut pixels, width,
   *   count, vertex_shader, frag_shader,
   *   bindings, &attribs_spec);
   ```

   If we print `pixels` as a rectangle where `0` becomes `.` we'd get this

   ```
   .....666......................
   ........66666.................
   .............66666............
   ..................66666.......
   .......................66.....
   ```

   Unfortunately, our fragment shader gets no input that changes each iteration so
   there is no way to output anything different for each pixel. This is where
   inter-stage variables come in. Let's change our first shader to output an extra value.

   ```rust
   let buffer1 = [5.0, 0.0, 25.0, 4.0];
   +let buffer2 = [9.0, 3.0];
   let attribs_spec = [
     Attrib { source: &buffer1, offset: 0, stride: 2 },
     Attrib { source: &buffer1, offset: 1, stride: 2 },
   +  Attrib { source: &buffer2, offset: 0, stride: 1 },
   ];
   let bindings: &[&[f32]] = &[];
   let vertex_shader = |_ndx: usize, _bindings: &[&[f32]], attribs: &[f32]|
   -    [attribs[0], attribs[1]];
   +    vec![vec![attribs[0], attribs[1]], vec![attribs[2]]];

   ...
   ```

   Our vertex shader now returns a *list of arrays*: the first array is the
   position, anything after it is an extra value. If we changed nothing else,
   after the loop inside `draw`, `internal_buffer` would have these values

   ```rust
    [
      [[ 5.0, 0.0], [9.0]],
      [[25.0, 4.0], [3.0]],
    ]
   ```

   We can easily compute a value from 0.0 to 1.0 that represents how far along
   the line we are. We can use this to interpolate the extra value we just
   added.

   ```rust
   fn rasterize_lines(
     dest: &mut [i32],
     dest_width: usize,
   -  inputs: &[[f32; 2]],
   -  frag_shader_fn: impl Fn(&[&[f32]]) -> i32,
   +  inputs: &[Vec<Vec<f32>>],
   +  frag_shader_fn: impl Fn(&[&[f32]], &[Vec<f32>]) -> i32,
     bindings: &[&[f32]],
   ) {
     for ndx in (0..inputs.len() - 1).step_by(2) {
   -    let p0 = &inputs[ndx    ];
   -    let p1 = &inputs[ndx + 1];
   +    let p0 = &inputs[ndx    ][0];
   +    let p1 = &inputs[ndx + 1][0];
   +    let v0 = &inputs[ndx    ][1..];  // everything but the first value
   +    let v1 = &inputs[ndx + 1][1..];
       let line = calc_line(p0, p1);
       for i in 0..line.num_pixels {
         let p = calc_line_point(&line, i);
   +      let t = i as f32 / line.num_pixels as f32;
   +      let inter_stage_variables = interpolate_arrays(v0, v1, t);
         let offset = p[1] as usize * dest_width + p[0] as usize;  // y * width + x
   -      dest[offset] = frag_shader_fn(bindings);
   +      dest[offset] = frag_shader_fn(bindings, &inter_stage_variables);
       }
     }
   }

   +// interpolate_arrays(&[vec![1.0, 2.0]], &[vec![3.0, 4.0]], 0.25) => [[1.5, 2.5]]
   +fn interpolate_arrays(v0: &[Vec<f32>], v1: &[Vec<f32>], t: f32) -> Vec<Vec<f32>> {
   +  v0.iter().enumerate().map(|(ndx, array0)| {
   +    let array1 = &v1[ndx];
   +    interpolate_values(array0, array1, t)
   +  }).collect()
   +}

   +// interpolate_values(&[1.0, 2.0], &[3.0, 4.0], 0.25) => [1.5, 2.5]
   +fn interpolate_values(array0: &[f32], array1: &[f32], t: f32) -> Vec<f32> {
   +  array0.iter().enumerate().map(|(ndx, a)| {
   +    let b = array1[ndx];
   +    a + (b - a) * t
   +  }).collect()
   +}
   ```

   Now we can use those inter-stage variables in our fragment shader

   ```rust
   -let frag_shader = |_bindings: &[&[f32]]| 6;
   +let frag_shader = |_bindings: &[&[f32]], inter_stage_variables: &[Vec<f32>]|
   +    inter_stage_variables[0][0] as i32;  // convert to int
   ```

   If we ran it now we'd see results like this

   ```
   .....988......................
   ........87776.................
   .............66655............
   ..................54443.......
   .......................33.....
   ```

   The first iteration of the vertex shader output `[[5.0, 0.0], [9.0]]` and
   the 2nd iteration output `[[25.0, 4.0], [3.0]]` and you can see, 
   as the fragment shader was called, the 2nd value of each of those
   was interpolated between the two values.

   We could make another function `map_triangle` that given 3 points
   rasterized a triangle calling the fragment shader function for each
   point inside the triangle. It would interpolate the inter-stage variables
   from 3 points instead of 2.

Here are all the examples above running live in case you find it
useful to play around with them to understand them.

{{{example url="../webgpu-javascript-analogies.html"}}}

What happens in the Rust above is an analogy. The details
of how inter-stage variables are actually interpolated, how lines are drawn, how
buffers are accessed, how textures are sampled, uniforms, attributes specified,
etc... are different in WebGPU, but the concepts are very similar so
I hope this Rust analogy provided some help in getting a mental
model of what's happening.

Why is it this way? Well, if you look at `draw` and `rasterize_lines`
you might notice that each iteration is entirely independent of
the other iterations. Another way to say this, you could process
each iteration in any order. Instead of 0, 1, 2, 3, 4 you could
process them 3, 1, 4, 0, 2 and you'd get the exact same result.
The fact that they are independent means each iteration can be
run in parallel by a different processor. Modern 2021 top end
GPUs have 10000 or more processors. That means up to 10000 things can be
run in parallel. That is where the power of using the GPU comes from.
By following these patterns the system can massively parallelize
the work.

The biggest limitations are:

1. A shader function can only reference
   its inputs (attributes, buffers, textures, uniforms, inter-stage variables).

2. A shader can not allocate memory.

3. A shader has to be careful if it references things it writes to, the thing it's
   generating values for.

   When you think about it this makes sense. Imagine `frag_shader`
   above tried to reference `dest` directly. That would mean when
   trying to parallelize things it would be impossible to coordinate.
   Which iteration would go first? If the 3rd iteration referenced `dest[0]`
   then the 0th iteration would need to run first but if the 0th iteration
   referenced `dest[3]` then the 3rd iteration would need to run first.

   Designing around this limitation also happens with CPUs and multiple
   thread or processes but in GPU land, with up to 10000 processors running
   at once, it takes special coordination. We'll try to cover some of the
   techniques in other articles.
