#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="${1:-$root/bindings/c/include/rustille.h}"

if ! command -v cbindgen >/dev/null 2>&1; then
    echo "cbindgen is not installed; run: cargo install cbindgen --locked" >&2
    exit 1
fi

mkdir -p "$(dirname "$out")"
cbindgen --config "$root/bindings/c/cbindgen.toml" \
         --crate rustille-capi \
         --output "$out"
echo "wrote $out"
