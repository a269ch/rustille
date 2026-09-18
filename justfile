# Rustille developer tasks.
#
#   just            list the recipes
#   just check      run everything CI runs for Rust
#   just check-all  ... plus every binding
#
# Recipes that need an external toolchain say so and fail with a clear message
# rather than a stack trace.

set shell := ["bash", "-euo", "pipefail", "-c"]

python_venv := justfile_directory() / ".venv"
wasm_out := justfile_directory() / "bindings/wasm/pkg"

# List the available recipes.
default:
    @just --list

# Format every Rust crate, including the out-of-workspace bindings.
fmt:
    cargo fmt --all
    cargo fmt --manifest-path bindings/python/Cargo.toml --all
    cargo fmt --manifest-path bindings/node/Cargo.toml --all
    cargo fmt --manifest-path bindings/wasm/Cargo.toml --all

# Check formatting without changing anything.
fmt-check:
    cargo fmt --all -- --check
    cargo fmt --manifest-path bindings/python/Cargo.toml --all -- --check
    cargo fmt --manifest-path bindings/node/Cargo.toml --all -- --check
    cargo fmt --manifest-path bindings/wasm/Cargo.toml --all -- --check

# Clippy, warnings denied, over the workspace.
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Clippy over the bindings that live in their own workspaces.
lint-bindings:
    cargo clippy --manifest-path bindings/python/Cargo.toml --all-targets -- -D warnings
    cargo clippy --manifest-path bindings/node/Cargo.toml --all-targets -- -D warnings
    cargo clippy --manifest-path bindings/wasm/Cargo.toml --all-targets -- -D warnings

# The Rust test suite.
test:
    cargo test --workspace --all-features

# Rebuild the golden fixtures after an intentional rendering change.
bless:
    RUSTILLE_BLESS=1 cargo test -p rustille --all-features --test golden

# Build the API documentation.
doc *args:
    cargo doc --workspace --no-deps --all-features {{args}}

# Validate the crates.io package without publishing.
package:
    cargo publish --dry-run -p rustille --all-features --locked

# Criterion benchmarks.
bench *args:
    cargo bench -p rustille {{args}}

# Run the CLI against an image.
run *args:
    cargo run -p rustille --features cli --release -- {{args}}

# --- bindings -------------------------------------------------------------

# Build the Python wheel and run pytest against it.
python:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v python3 >/dev/null || { echo "python3 is required"; exit 1; }
    [ -d "{{python_venv}}" ] || python3 -m venv "{{python_venv}}"
    "{{python_venv}}/bin/pip" install -q --upgrade pip
    "{{python_venv}}/bin/pip" install -q "maturin>=1.7,<2.0" pytest
    cd bindings/python
    "{{python_venv}}/bin/maturin" develop --release
    "{{python_venv}}/bin/python" -m pytest tests -q

# Build the native Node package and run its tests.
node:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v npm >/dev/null || { echo "npm is required"; exit 1; }
    cd bindings/node
    npm install
    npm run build
    npm test

# Build the WebAssembly package and run the Node smoke test.
wasm:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v wasm-pack >/dev/null || { echo "wasm-pack is required: cargo install wasm-pack"; exit 1; }
    cd bindings/wasm
    wasm-pack build --target web --out-dir pkg
    wasm-pack build --target nodejs --out-dir pkg-node
    command -v node >/dev/null && node examples/node.mjs ./pkg-node

# Generate the C header.
c-header:
    ./scripts/gen-header.sh

# Build the C library and compile + run the C example against it.
c:
    ./scripts/build-c-example.sh release

# Stage the C release bundle for the host target.
c-package target=`rustc -vV | sed -n 's|host: ||p'`:
    ./scripts/package-c.sh {{target}}

# --- release --------------------------------------------------------------

# Verify that every package carries the same version.
check-version tag="":
    ./scripts/check-version.sh {{tag}}

# Set the version of every package.
set-version version:
    ./scripts/set-version.sh {{version}}

# Dependency licence and advisory audit.
deny:
    #!/usr/bin/env bash
    set -euo pipefail
    command -v cargo-deny >/dev/null || { echo "cargo-deny is required: cargo install cargo-deny"; exit 1; }
    cargo deny --workspace check

# --- aggregates -----------------------------------------------------------

# Everything CI runs for the Rust workspace.
check: fmt-check lint test doc package check-version

# Everything, including the bindings. Needs python3, npm and wasm-pack.
check-all: check c python node wasm

# What the release workflow validates before it publishes anything.
release-check tag: (check-version tag) fmt-check lint test package c
    @echo "release checks passed for {{tag}}"
