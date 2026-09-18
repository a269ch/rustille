# Contributing to Rustille

Thanks for taking a look. Bug reports, small fixes and new dithering
algorithms are all welcome.

## Quick start

```console
$ git clone https://github.com/a269ch/rustille
$ cd rustille
$ cargo test --workspace --all-features
$ cargo run -p rustille --features cli -- some-image.png --width 80
```

If you have [`just`](https://github.com/casey/just):

```console
$ just check      # everything CI runs for the Rust workspace
$ just check-all  # the above plus every binding (needs python3, npm, wasm-pack)
```

## Repository layout

```text
crates/rustille/        the library and the CLI - all the logic lives here
bindings/c/             C ABI (rustille-capi), the only crate with `unsafe`
bindings/python/        PyO3 + maturin
bindings/node/          napi-rs
bindings/wasm/          wasm-bindgen
scripts/                header generation, C packaging, version management
.github/workflows/      CI and the automated release
```

### Why the bindings are not workspace members

`crates/rustille` and `bindings/c` are members of the root workspace; the
Python, Node and WebAssembly bindings are listed under `exclude` and are their
own one-package workspaces.

That is deliberate. Each of those three is driven by its own build tool
(maturin, `@napi-rs/cli`, `wasm-pack`) and links against a host runtime. Keeping
them out of the root workspace means:

* `cargo test --workspace --all-features` never tries to link `libpython` (PyO3's
  `extension-module` feature makes a test binary unlinkable on Linux);
* FFI-only dependencies never take part in feature unification for the core
  crate, so `cargo build -p rustille` stays honest;
* each binding pins its own `Cargo.lock`.

The cost is that `cargo fmt`/`cargo clippy` at the root do not see them, so the
`justfile` and CI run those tools in each binding directory too. `just fmt` and
`just lint-bindings` cover all four.

## Where code belongs

**All rendering logic lives in `crates/rustille`.** Every binding is an adapter
that converts arguments into `RenderOptions` and calls the same public API the
CLI uses — usually `Renderer::render_rgba`. If you find yourself writing a loop
over pixels in a binding, it belongs in the core instead.

The core is `#![forbid(unsafe_code)]`. The only `unsafe` in the repository is in
`bindings/c`, and every block there carries a `SAFETY:` comment explaining what
the caller must guarantee.

## Coding standards

* `cargo fmt` (default rustfmt settings) and `cargo clippy -- -D warnings` must
  both be clean.
* Public items need rustdoc; `missing_docs` is denied.
* No `.unwrap()` or `.expect()` on a library path. Where an invariant genuinely
  makes a failure impossible, prove it in a comment next to the code.
* Fallible public APIs return `rustille::Result`. Add a new `Error` variant only
  when bindings can map it to something meaningful — the enum is `#[non_exhaustive]`
  but it is also the source of the C ABI's status codes.
* Avoid allocating per pixel, and never call `format!` inside a rendering loop.
  Size output buffers up front.

## Tests

```console
$ cargo test --workspace --all-features
$ just bless   # after an intentional rendering change
```

* Unit tests live next to the code; integration tests in
  `crates/rustille/tests/`.
* Fixtures are **generated in-process** (`tests/common/mod.rs`) rather than
  committed as binaries, so a reviewer can see what an image contains.
* Golden files in `crates/rustille/tests/golden/` are small enough to read in a
  diff. `RUSTILLE_BLESS=1 cargo test --test golden` rewrites them; always look
  at the resulting diff before committing it.
* A change to the pipeline that moves dots should move goldens. If it does not,
  the test is probably not covering the change.

## Adding a dithering algorithm

Error-diffusion kernels are data. Add a constant next to `FLOYD_STEINBERG` in
`src/dithering.rs`, a variant to `Dither`, a line to `Dither::ALL` and the
`FromStr`/`as_str` mapping in `src/options.rs`, and a golden test. No renderer
change is needed, and every binding picks the new name up automatically because
they all parse `Dither` from a string.

## Working on the bindings

| | |
|---|---|
| Python | `just python` — builds the wheel with maturin and runs pytest against the installed package, not the sources. |
| Node | `just node` — `napi build` then `node --test`. |
| WASM | `just wasm` — `wasm-pack build` for both `web` and `nodejs`, then the Node smoke test. |
| C | `just c` — builds the library, regenerates `rustille.h` with cbindgen, then compiles and runs `bindings/c/examples/demo.c` against both the static and the shared library. |

Generated files (`bindings/c/include/rustille.h`, `bindings/node/index.js`,
`bindings/node/index.d.ts`, `bindings/wasm/pkg/`) are **not** committed; CI
regenerates them. `git status` should be clean after a full build, and CI checks
exactly that.

## Versions

Every package shares one version. Never edit versions by hand:

```console
$ ./scripts/set-version.sh 0.2.0
$ ./scripts/check-version.sh v0.2.0
```

`check-version.sh` also runs in CI and is the first gate of the release
workflow.

## Pull requests

* Branch off `main`.
* Add a `## [Unreleased]` entry to `CHANGELOG.md` for anything user-visible.
* Keep the diff focused; unrelated reformatting makes review harder.
* CI must be green: formatting, clippy, tests on Linux/macOS/Windows, MSRV,
  feature combinations, packaging, `cargo-deny`, and a real build of every
  binding.

## Licensing and provenance

Rustille is dual licensed MIT OR Apache-2.0, and contributions are accepted
under the same terms.

Rustille is an independent, clean-room implementation written from the Unicode
Braille specification and standard image-processing algorithms.
[drawille](https://github.com/asciimoo/drawille) is AGPL-3.0 licensed: do not
copy code, internal architecture, function names or non-trivial implementation
details from it into this project. Behaviour and UX are fair game as
inspiration; source is not.
