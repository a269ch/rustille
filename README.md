# Rustille

**Turn pixels into Braille.**

[![CI](https://github.com/a269ch/rustille/actions/workflows/ci.yml/badge.svg)](https://github.com/a269ch/rustille/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/rustille.svg)](https://crates.io/crates/rustille)
[![docs.rs](https://img.shields.io/docsrs/rustille)](https://docs.rs/rustille)
[![PyPI](https://img.shields.io/pypi/v/rustille.svg)](https://pypi.org/project/rustille/)
[![npm](https://img.shields.io/npm/v/rustille.svg)](https://www.npmjs.com/package/rustille)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

Rustille renders images as Unicode Braille art and gives you a drawing canvas
built on the same encoder. It is a Rust library first; the CLI, the Python
package, the Node.js addon, the WebAssembly module and the C ABI are all thin
adapters over one core.

```console
$ rustille ring.png --width 44
```

```text
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣀⣀⣀⣀⣀⣀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣠⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣶⣤⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣤⣾⣿⣿⡿⠿⠛⠋⣉⣉⣉⣉⣉⣉⡙⠛⠻⢿⣿⣿⣷⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⣴⣿⣿⣿⠟⢉⣠⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣶⣤⡉⠻⢿⣿⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⢀⣾⣿⣿⠟⢁⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⣄⠙⣿⣿⣷⡄⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⢀⣾⣿⡿⠃⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⠈⢿⣿⣿⡄⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⣾⣿⣿⠃⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⠀⣿⣿⣿⡀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⢸⣿⣿⡏⢠⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇⠸⣿⣿⣇⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⢸⣿⣿⡇⢸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⠀⣿⣿⣿⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⢸⣿⣿⣇⠸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇⢰⣿⣿⡟⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⢿⣿⣿⡀⢻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠀⣾⣿⣿⠃⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠘⣿⣿⣷⡄⠻⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡟⢁⣾⣿⣿⠏⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠘⢿⣿⣿⣆⠙⢿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠋⣠⣾⣿⣿⠋⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠈⠻⣿⣿⣷⣤⡈⠛⠿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⢉⣠⣾⣿⣿⠟⠁⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠻⢿⣿⣿⣷⣤⣤⣉⡉⠛⠛⠛⠛⢉⣉⣠⣤⣾⣿⣿⣿⠟⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠛⠿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡿⠟⠋⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠙⠛⠛⠛⠛⠛⠉⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
```

## Why Braille?

Braille characters are the densest text pixels there are. One character from
the Unicode `Braille Patterns` block (`U+2800`–`U+28FF`) carries a **2×4 grid of
dots**, so a single character is eight logical pixels — four times the vertical
resolution of a half-block character and eight times that of plain ASCII art.

```text
        x=0    x=1
y=0     dot1   dot4        0x01   0x08
y=1     dot2   dot5        0x02   0x10
y=2     dot3   dot6        0x04   0x20
y=3     dot7   dot8        0x40   0x80

character = U+2800 + mask
```

The numbering is the historical six-dot Braille order extended with dots 7 and
8, which is why the bits are not a plain row-major scan. Setting dot 1 gives
`U+2801` (`⠁`); setting everything gives `U+28FF` (`⣿`).

An 80×24 terminal therefore holds 160×96 dots.

## Installation

| | |
|---|---|
| **CLI** | `cargo install rustille` — or grab a [release binary](https://github.com/a269ch/rustille/releases) |
| **Rust** | `cargo add rustille` |
| **Python** | `pip install rustille` |
| **Node.js** | `npm install rustille` |
| **Browser / WASM** | `npm install rustille-wasm` |
| **C / C++ / anything** | download `rustille-c-vX.Y.Z-<target>.tar.gz` from the [releases](https://github.com/a269ch/rustille/releases) |

Prebuilt binaries ship for Linux (glibc and musl), macOS and Windows on x86-64
and aarch64, so none of the packages need a Rust toolchain at install time.

## Command line

```console
$ rustille cat.png                        # fills the terminal
$ rustille cat.png --width 100            # 100 characters wide
$ rustille cat.png --height 40            # 40 characters tall
$ rustille cat.png --threshold 150        # only brighter pixels become dots
$ rustille cat.png --invert               # draw the dark parts instead
$ rustille cat.png --dither floyd-steinberg
$ rustille cat.png --color always
$ rustille cat.png --fit contain --width 80 --height 24
$ cat cat.png | rustille -                # read from standard input
$ rustille cat.png -o art.txt             # write to a file
```

| Flag | Values | Default | Meaning |
|---|---|---|---|
| `-w, --width` | characters | terminal width, else 80 | Output width |
| `-H, --height` | characters | terminal height − 1 | Output height |
| `-t, --threshold` | `0`–`255` | `128` | Luminance at or above which a dot is drawn |
| `-i, --invert` | | off | Draw dark pixels instead of bright ones |
| `-d, --dither` | `none`, `floyd-steinberg` | `none` | Dithering algorithm |
| `-c, --color` | `auto`, `always`, `never`, `ansi256`, `truecolor` | `auto` | ANSI colour |
| `--fit` | `contain`, `fill`, `stretch` | `contain` | How the image maps onto the box |
| `-b, --background` | `black`, `white`, `#rrggbb`, `r,g,b` | `black` | Composited under transparent pixels |
| `--cell-aspect` | ratio | `2.0` | Height/width ratio of a terminal cell |
| `-o, --output` | path | stdout | Write here instead of standard output |

Rendered output goes to standard output; diagnostics go to standard error. The
exit code is `0` on success, `1` on a runtime failure (unreadable file, corrupt
image) and `2` when the arguments themselves are wrong.

`rustille --version` and `rustille --help` do what you expect.

## Rust

```toml
[dependencies]
rustille = "0.1"
```

```rust
use rustille::{Dither, RenderOptions, Renderer};

let options = RenderOptions::default()
    .width(80)
    .threshold(128)
    .dither(Dither::FloydSteinberg);

let output = Renderer::new(options).render_file("cat.png")?;
println!("{output}");
# Ok::<(), rustille::Error>(())
```

Nothing in the rendering path touches the filesystem unless you ask it to:

```rust
use rustille::{RenderOptions, Renderer};

let renderer = Renderer::new(RenderOptions::default().width(80));

let from_memory = renderer.render_bytes(&encoded_png)?;         // any supported format
let from_pixels = renderer.render_rgba(width, height, &rgba)?;  // tightly packed RGBA8
let from_rgb    = renderer.render_rgb(width, height, &rgb)?;    // RGB8
let from_gray   = renderer.render_luma(width, height, &gray)?;  // 8-bit grayscale
```

`render_rgba` is the entry point every binding uses, which is what keeps the
algorithm in exactly one place.

### Canvas

```rust
use rustille::Canvas;

let mut canvas = Canvas::new(100, 50);   // 100 × 50 *dots* = 50 × 13 characters

canvas.set(10, 10);
canvas.set(11, 11);
canvas.unset(10, 10);

canvas.line(0, 0, 99, 49);
canvas.rectangle(0, 0, 99, 49);
canvas.circle(50, 25, 20);

println!("{}", canvas.render());
```

The canvas stores exactly one bit per dot: one `u8` per character cell, and
that byte *is* the Braille mask, so rendering is a straight walk over the
buffer. `render_into(&mut String)` reuses a buffer if you are animating.

```text
⡏⠛⠭⣉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⣉⠭⠛⢹
⡇⠀⠀⠀⠉⠒⠤⣀⠀⡠⠒⠉⠉⠑⠢⡀⣀⠤⠒⠉⠀⠀⠀⢸
⡇⠀⠀⠀⠀⠀⠀⠀⡝⠒⠤⣀⣀⠤⠒⠙⡄⠀⠀⠀⠀⠀⠀⢸
⡇⠀⠀⠀⠀⠀⠀⠀⣇⠤⠒⠉⠉⠒⠤⣀⠇⠀⠀⠀⠀⠀⠀⢸
⡇⠀⠀⠀⣀⠤⠒⠉⠈⠢⣀⠀⠀⢀⡠⠊⠉⠒⠤⣀⠀⠀⠀⢸
⣇⣤⣒⣉⣀⣀⣀⣀⣀⣀⣀⣉⣉⣁⣀⣀⣀⣀⣀⣀⣉⣒⣤⣸
```

Out-of-range coordinates are clipped, never wrapped, and `line`, `rectangle`
and `circle` clip too, so drawing off the edge is safe.

Full API documentation lives on [docs.rs](https://docs.rs/rustille).

## Python

```console
$ pip install rustille
```

```python
import rustille

print(rustille.render_file("cat.png", width=80))

with open("cat.png", "rb") as handle:
    print(rustille.render_bytes(handle.read(), width=80, dither="floyd-steinberg"))

output = rustille.render_rgba(
    rgba,
    width=image_width,
    height=image_height,
    output_width=80,
)

canvas = rustille.Canvas(100, 50)
canvas.set(1, 1)
canvas.circle(50, 25, 20)
print(canvas.render())
```

Errors map onto Python's own hierarchy: `OSError` for I/O, `ValueError` for bad
arguments, and `rustille.DecodeError` (a subclass of `rustille.RustilleError`)
for images that will not decode.

Wheels are `abi3` and built for CPython 3.9+, so a single wheel per platform
covers every supported Python version.

## Node.js

```console
$ npm install rustille
```

```js
import { renderFile, renderBytes, renderRgba, Canvas } from "rustille";

console.log(renderFile("cat.png", { width: 80 }));
console.log(renderBytes(buffer, { width: 80, dither: "floyd-steinberg" }));
console.log(renderRgba(rgba, imageWidth, imageHeight, { width: 80 }));

const canvas = new Canvas(100, 50);
canvas.circle(50, 25, 20);
console.log(canvas.render());
```

Built with [napi-rs](https://napi.rs) against Node-API, so the same binary works
across Node versions. TypeScript declarations are generated as part of the
build. Node 18 and newer are supported.

## WebAssembly

```console
$ npm install rustille-wasm
```

The browser already decodes images, so hand Rustille the pixels:

```js
import init, { renderRgba } from "rustille-wasm";

await init();

const bitmap = await createImageBitmap(file);
const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
const context = canvas.getContext("2d");
context.drawImage(bitmap, 0, 0);
const { data, width, height } = context.getImageData(0, 0, bitmap.width, bitmap.height);

console.log(renderRgba(data, width, height, { width: 80 }));
```

`renderBytes` is also available for decoding PNG/JPEG/WebP/BMP/GIF/TIFF inside
WebAssembly; build with `--no-default-features` to drop it and shrink the
module. There is a runnable [browser example](bindings/wasm/examples/browser.html)
and a [Node example](bindings/wasm/examples/node.mjs).

## C ABI

The C ABI is the universal integration layer: anything that can call C (C++,
Go via cgo, C#, Swift, Ruby FFI, PHP FFI, LuaJIT, Zig, …) can use Rustille
through it.

```c
#include <rustille.h>

RustilleOptions options;
rustille_options_init(&options);
options.width = 80;
options.dither = RUSTILLE_DITHER_FLOYD_STEINBERG;

char *art = NULL;
RustilleStatus status =
    rustille_render_rgba(rgba, length, width, height, &options, &art);

if (status == RUSTILLE_STATUS_OK) {
    puts(art);
    rustille_string_free(art);
} else {
    fprintf(stderr, "%s\n", rustille_status_message(status));
}
```

The contract is deliberately small:

* every function tolerates null pointers and returns `RUSTILLE_STATUS_NULL_POINTER`;
* strings returned through an out-parameter are owned by the caller and freed
  with `rustille_string_free`;
* `rustille_last_error_message()` returns a per-thread detailed message (also
  caller-owned);
* **no panic can cross the boundary** — every entry point runs inside
  `catch_unwind` and reports `RUSTILLE_STATUS_PANIC`;
* the core crate is `#![forbid(unsafe_code)]`; every `unsafe` block in Rustille
  lives in this one adapter and carries a `SAFETY:` comment.

Release bundles contain the header, a shared library, a static library, a
pkg-config `.pc` file and a CMake package config. Locally:

```console
$ just c          # build the library, generate the header, compile and run the C example
$ just c-header   # just regenerate bindings/c/include/rustille.h
```

## Sizing and aspect ratio

`width` and `height` are measured in **characters**, not pixels. A width of 80
means 80 columns, which is 160 dots.

A terminal character cell is normally about twice as tall as it is wide.
`cell_aspect_ratio` (default `2.0`) describes that, and because one cell holds
2×4 dots, the default makes a single *dot* square — so a square image comes out
square rather than stretched.

Give one dimension and the other follows from the image. Give both and `--fit`
decides:

| Fit | Behaviour |
|---|---|
| `contain` (default) | Scale to fit inside the box, preserving the aspect ratio. The output may be smaller than the box in one dimension; nothing is padded. |
| `fill` | Scale to cover the box, preserving the aspect ratio, then centre-crop. |
| `stretch` | Use the box exactly and ignore the aspect ratio. |

Give neither and the library uses 80 columns. The **library never looks at the
terminal** — only the CLI does, and only when no size was requested.

## Colour

The library returns plain text by default; you never get an escape sequence you
did not ask for.

```rust
use rustille::{ColorMode, RenderOptions};

let options = RenderOptions::default().color(ColorMode::TrueColor);
```

* `ColorMode::None` — plain Unicode (default).
* `ColorMode::Ansi256` — `ESC [ 38;5;N m`, matched against the xterm 6×6×6 cube
  and the 24-step grey ramp.
* `ColorMode::TrueColor` — `ESC [ 38;2;R;G;B m`.

A cell's colour is the average of its *lit* dots, so the colour follows the ink
rather than the background. A sequence is only emitted when the colour actually
changes, and each row ends with a reset.

The CLI's `--color auto` honours `NO_COLOR`, checks that stdout is a terminal,
and picks 24-bit when `COLORTERM` says `truecolor` or `24bit`.

## Dithering

Dithering runs on the luminance plane *before* pixels are packed into cells, so
it sees the full 2×4 resolution of every character:

```console
$ rustille photo.jpg --width 60 --dither floyd-steinberg
```

`none` and `floyd-steinberg` ship today. Error-diffusion kernels are data
(`rustille::dithering::ErrorDiffusionKernel`), so Atkinson, Sierra and Stucki
are a constant each — no renderer or public API change.

## Supported image formats

PNG, JPEG, WebP, BMP, GIF and TIFF, decoded with the
[`image`](https://crates.io/crates/image) crate. The format is sniffed from the
bytes, not from the file name.

Decoding is behind the default `decode` feature. Turn it off for a
dependency-light build with just the raw-pixel and canvas APIs, or pick single
formats with `decode-png`, `decode-jpeg`, `decode-webp`, `decode-bmp`,
`decode-gif` and `decode-tiff`.

```toml
rustille = { version = "0.1", default-features = false, features = ["decode-png"] }
```

SVG is out of scope for now, and nothing in the core ever makes a network
request.

## Performance

The pipeline is linear in the number of *output* dots, not input pixels: the
image is resampled once with a separable area/linear filter, then each cell is
packed with eight comparisons. The output `String` is sized up front from the
cell count and the colour mode, so the hot loop never reallocates, and no
`format!` runs inside it.

```console
$ cargo bench -p rustille
```

Criterion benchmarks cover Braille packing, canvas rendering, and 1920×1080
grayscale, RGBA, truecolour and Floyd–Steinberg renders.

## Development

```console
$ just            # list the recipes
$ just check      # everything CI runs for Rust: fmt, clippy, tests, docs, packaging
$ just test
$ just bless      # rebuild the golden fixtures after an intentional change
$ just c          # C library + header + C example
$ just python     # wheel + pytest
$ just node       # native addon + node --test
$ just wasm       # wasm-pack + node smoke test
$ just check-all  # all of the above
```

[`just`](https://github.com/casey/just) is optional; every recipe is a short
shell command you can also run by hand. See [CONTRIBUTING.md](CONTRIBUTING.md)
for the repository layout and the reasoning behind it.

## Release process

Releases are fully automated from a tag:

```console
$ ./scripts/set-version.sh 0.2.0
$ $EDITOR CHANGELOG.md
$ git commit -am "release 0.2.0"
$ git tag v0.2.0 && git push origin main --tags
```

GitHub Actions then validates that every package agrees on the version, runs
the whole test suite, builds every artifact, creates the GitHub Release with
checksums, and publishes to crates.io, PyPI and npm using OIDC Trusted
Publishing. No manual `cargo publish`, `maturin publish` or `npm publish` is
involved. [RELEASING.md](RELEASING.md) documents the one-time registry setup.

## Prior art

[drawille](https://github.com/asciimoo/drawille) popularised the Braille canvas
idea and [picharsso](https://github.com/kelvindecosta/picharsso) is a lovely
image-to-text-art tool. Rustille was written from the Unicode Braille
specification and standard image-processing algorithms; it is an independent
clean-room implementation and contains no code from either project. (Drawille is
AGPL-3.0; Rustille is not a derivative work of it.)

## License

Licensed under either of

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
