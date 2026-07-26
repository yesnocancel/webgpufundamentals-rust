#!/usr/bin/env python3
"""Convert an example HTML page from inline JS to the Rust/wasm loader.

For each given example name, takes webgpu/<name>.html, replaces its inline
<script type="module">...</script> with a loader for the wasm-bindgen output
of the matching Rust example, and adds a link to the Rust source.

Pages with extra scripts or UI need manual attention; this handles the
common single-inline-script case and refuses otherwise unless --force.
"""
import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOADER = """  <script type="module">
import init from './wasm/{name}/{name}.js';
init();
  </script>
"""

def convert(name: str, force: bool) -> bool:
    path = ROOT / "webgpu" / f"{name}.html"
    if not path.exists():
        print(f"SKIP {name}: {path} does not exist", file=sys.stderr)
        return False
    rust_src = ROOT / "rust" / "examples" / "src" / "bin" / f"{name}.rs"
    if not rust_src.exists():
        print(f"SKIP {name}: no Rust example at {rust_src}", file=sys.stderr)
        return False
    html = path.read_text()
    scripts = re.findall(r'<script[^>]*type="module"[^>]*>.*?</script>\n?', html, re.S)
    ext = re.findall(r'<script[^>]*\bsrc=[^>]*>', html)
    if (len(scripts) != 1 or ext) and not force:
        print(f"SKIP {name}: {len(scripts)} module scripts, {len(ext)} external scripts;"
              " needs manual conversion", file=sys.stderr)
        return False
    new_html = html.replace(scripts[-1], LOADER.format(name=name))
    path.write_text(new_html)
    print(f"OK   {name}")
    return True

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("names", nargs="+")
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()
    ok = all([convert(n, args.force) for n in args.names])
    sys.exit(0 if ok else 1)

if __name__ == "__main__":
    main()
