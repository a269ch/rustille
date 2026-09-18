#!/usr/bin/env bash
# Generates bindings/c/include/rustille.h with cbindgen.
#
# The header is derived from bindings/c/src/lib.rs and is therefore not
# committed; run this (or `just c-header`) before building a C consumer.
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
