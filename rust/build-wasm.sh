#!/usr/bin/env bash
# Build the Rust examples for the browser.
#
# Usage: ./build-wasm.sh [example-name ...]
# With no arguments, builds every example in examples/src/bin.
#
# Requires the wasm32-unknown-unknown target (Arch: pacman -S rust-wasm)
# and wasm-bindgen (Arch: pacman -S wasm-bindgen).
set -euo pipefail
cd "$(dirname "$0")"

if [[ $# -gt 0 ]]; then
    bins=("$@")
else
    bins=()
    for f in examples/src/bin/*.rs; do
        bins+=("$(basename "${f%.rs}")")
    done
fi

bin_flags=()
for b in "${bins[@]}"; do
    bin_flags+=(--bin "$b")
done

cargo build --release --target wasm32-unknown-unknown "${bin_flags[@]}"

for b in "${bins[@]}"; do
    wasm-bindgen --target web --no-typescript \
        --out-dir ../webgpu/wasm/"$b" \
        target/wasm32-unknown-unknown/release/"$b".wasm
done
echo "built ${#bins[@]} example(s) into webgpu/wasm/"
