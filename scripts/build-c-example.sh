#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="${1:-release}"
build_dir="$root/target/c-example"
mkdir -p "$build_dir"

case "$profile" in
    release) cargo_flags=(--release); target_dir="$root/target/release" ;;
    debug)   cargo_flags=();         target_dir="$root/target/debug" ;;
    *) echo "unknown profile: $profile (expected debug or release)" >&2; exit 2 ;;
esac

echo "==> building rustille-capi ($profile)"
cargo build -p rustille-capi "${cargo_flags[@]}"

echo "==> generating header"
"$root/scripts/gen-header.sh" >/dev/null

case "$(uname -s)" in
    Darwin) shared_ext="dylib"; system_libs=(-lm) ;;
    MINGW*|MSYS*|CYGWIN*) shared_ext="dll"; system_libs=(-lm) ;;
    *) shared_ext="so"; system_libs=(-lm -lpthread -ldl) ;;
esac

cc_bin="${CC:-cc}"
include_dir="$root/bindings/c/include"
source_file="$root/bindings/c/examples/demo.c"

echo "==> compiling the C example against librustille.a"
"$cc_bin" -std=c11 -Wall -Wextra -Werror -O2 \
    -I"$include_dir" \
    "$source_file" \
    "$target_dir/librustille.a" \
    "${system_libs[@]}" \
    -o "$build_dir/demo-static"

echo "==> running the statically linked example"
"$build_dir/demo-static"

if [ -f "$target_dir/librustille.$shared_ext" ]; then
    echo "==> compiling the C example against librustille.$shared_ext"
    "$cc_bin" -std=c11 -Wall -Wextra -Werror -O2 \
        -I"$include_dir" \
        "$source_file" \
        -L"$target_dir" -lrustille \
        "${system_libs[@]}" \
        -o "$build_dir/demo-shared"

    echo "==> running the dynamically linked example"
    LD_LIBRARY_PATH="$target_dir:${LD_LIBRARY_PATH:-}" \
    DYLD_LIBRARY_PATH="$target_dir:${DYLD_LIBRARY_PATH:-}" \
        "$build_dir/demo-shared" >/dev/null
    echo "shared linkage ok"
fi

echo "c example ok"
