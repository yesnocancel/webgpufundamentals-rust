# WebGPU Fundamentals in Rust

A Rust + [wgpu](https://wgpu.rs/) adaptation of
[webgpufundamentals.org](https://webgpufundamentals.org/): the same series of
lessons teaching WebGPU from the ground up, but with all application code in
Rust instead of JavaScript.

- The WGSL shaders are identical to the originals — WGSL is the same language
  no matter what drives it.
- Every example is a small Rust program in
  [`rust/examples/src/bin/`](rust/examples/src/bin/). Each one:
  - runs **natively** (`cargo run --bin webgpu-simple-triangle`) on
    Vulkan/Metal/DX12, and
  - runs **in the browser**, compiled to WebAssembly, embedded live in the
    lessons.
- A tiny helper crate, [`rust/wgpu_fun`](rust/wgpu_fun), does the
  window/canvas + device setup shared by all examples (explained in the first
  lesson).

This is a fork of the original
[webgpufundamentals](https://github.com/gfxfundamentals/webgpufundamentals);
the lesson prose, diagrams, site machinery, and translations come from there.
Translations are currently still the original JavaScript versions.

## Building the examples

```sh
cd rust
cargo build                       # native debug build of every example
cargo run --bin webgpu-simple-triangle

# browser builds (requires wasm32 target + wasm-bindgen CLI)
./build-wasm.sh                   # outputs into webgpu/wasm/<example>/
```

Every example also has a headless test mode used to verify the whole set:

```sh
WGPU_FUN_TEST=1 ./target/debug/webgpu-simple-triangle   # writes test_frames/*.png
```

## Building the site

The site is built into the `out` folder:

```sh
npm ci
npm run build
npm run serve
```

now open your browser to `http://localhost:8080`.

See [`rust/CONVERSION.md`](rust/CONVERSION.md) for the conventions used to
port lessons and examples.

## License

MIT (same as the original). Original lessons by
[Gregg Tavares](https://github.com/greggman).
