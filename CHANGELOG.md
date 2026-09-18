# Changelog

All notable changes to Rustille are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Every package in the repository — the Rust crate, the Python wheel, the npm
packages and the C ABI — shares one version number; the release workflow
refuses to publish anything if they disagree.

## [Unreleased]

## [0.1.0] - 2026-09-18

First release.

### Added

- **Braille encoder** covering the whole `U+2800`–`U+28FF` block, with the
  standard dot-1..8 bit layout and exhaustive tests over all 256 masks.
- **`Canvas`**: a fixed-size drawing surface using exactly one bit per dot, with
  `set`/`unset`/`toggle`/`get`/`clear`/`fill`, Bresenham `line`, `rectangle`,
  `circle` and their filled variants, and clipping instead of wrapping.
- **Image renderer** with the pipeline decode → alpha compositing → fit/crop →
  resize → luminance → invert → dither → threshold → 2×4 packing → Unicode.
  PNG, JPEG, WebP, BMP, GIF and TIFF are supported, sniffed from the bytes.
- **`RenderOptions`**: width and height in characters, threshold, invert,
  dithering, colour mode, background, fit mode and cell aspect ratio, with a
  chainable builder and validation.
- **Aspect-ratio handling** via `cell_aspect_ratio` (default `2.0`), so square
  images come out square in a normal terminal.
- **Colour**: plain text by default, plus ANSI 256 and 24-bit truecolour driven
  by the average colour of a cell's lit dots, emitted only when it changes.
- **Dithering**: none and Floyd–Steinberg, applied before cells are packed, with
  a data-driven kernel type so more algorithms need no API change.
- **CLI** `rustille`, with stdin support, terminal-aware sizing, `--color auto`
  honouring `NO_COLOR` and TTY detection, and documented exit codes.
- **Python bindings** (PyO3 + maturin, abi3 wheels for CPython 3.9+) exposing
  `render_file`, `render_bytes`, `render_rgba`, `render_rgb`, `render_luma` and
  `Canvas`, with errors mapped onto Python's own exception hierarchy.
- **Node.js bindings** (napi-rs, Node-API) with generated TypeScript
  declarations and prebuilt binaries for eight targets.
- **WebAssembly bindings** (wasm-bindgen) with a browser-friendly `renderRgba`,
  an optional in-WASM decoder, `Canvas`, and generated TypeScript typings.
- **C ABI** (`rustille.h`, cdylib + staticlib) with explicit ownership, stable
  status codes, per-thread error messages, panic containment at the boundary,
  a pkg-config file and a CMake package config.
- **Benchmarks** (Criterion) for Braille packing, canvas rendering and
  1920×1080 grayscale/RGBA/truecolour/Floyd–Steinberg renders.
- **CI** across Ubuntu, macOS and Windows, covering formatting, clippy, tests,
  docs, MSRV, feature combinations, packaging, `cargo-deny`, and a real build
  and smoke test of every binding.
- **Automated releases**: one semver tag builds every artifact and publishes to
  crates.io, PyPI and npm through OIDC Trusted Publishing.

[Unreleased]: https://github.com/a269ch/rustille/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/a269ch/rustille/releases/tag/v0.1.0
