#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ $# -lt 1 ]; then
    echo "usage: $(basename "$0") <target-triple> [output-dir]" >&2
    exit 2
fi

target="$1"
version="$(grep -m1 -E '^version[[:space:]]*=' "$root/Cargo.toml" | cut -d'"' -f2)"
staging="${2:-$root/dist/rustille-c-v$version-$target}"

case "$target" in
    *-apple-*)   shared="librustille.dylib"; private_libs="-lm" ;;
    *-windows-*) shared="rustille.dll";      private_libs="-ladvapi32 -luserenv -lws2_32 -lbcrypt -lntdll" ;;
    *-musl)      shared="librustille.so";    private_libs="-lm" ;;
    *)           shared="librustille.so";    private_libs="-lm -lpthread -ldl" ;;
esac
static="librustille.a"
if [[ "$target" == *-windows-* ]]; then
    static="rustille.lib"
fi

echo "==> building rustille-capi for $target"
cargo build -p rustille-capi --release --target "$target"

echo "==> generating the header"
"$root/scripts/gen-header.sh" "$root/target/rustille.h" >/dev/null

echo "==> staging into $staging"
rm -rf "$staging"
mkdir -p "$staging/include" "$staging/lib/pkgconfig" "$staging/lib/cmake/rustille"

cp "$root/target/rustille.h" "$staging/include/rustille.h"
built="$root/target/$target/release"
artifacts=("$shared" "$static")
if [[ "$target" == *-windows-* ]]; then
    artifacts+=("rustille.dll.lib")
fi
for artifact in "${artifacts[@]}"; do
    if [ -f "$built/$artifact" ]; then
        cp "$built/$artifact" "$staging/lib/$artifact"
    else
        echo "warning: $built/$artifact was not produced" >&2
    fi
done

sed -e "s|@PREFIX@|/usr/local|g" \
    -e "s|@VERSION@|$version|g" \
    -e "s|@PRIVATE_LIBS@|$private_libs|g" \
    "$root/bindings/c/rustille.pc.in" >"$staging/lib/pkgconfig/rustille.pc"

sed -e "s|@VERSION@|$version|g" \
    -e "s|@SHARED_LIB@|$shared|g" \
    -e "s|@STATIC_LIB@|$static|g" \
    -e "s|@PRIVATE_LIBS@|$private_libs|g" \
    "$root/bindings/c/rustille-config.cmake.in" \
    >"$staging/lib/cmake/rustille/rustille-config.cmake"

cp "$root/LICENSE-MIT" "$root/LICENSE-APACHE" "$staging/"
cp "$root/bindings/c/examples/demo.c" "$staging/example.c"

cat >"$staging/README.md" <<EOF

Turn pixels into Braille, from any language that can call C.

    RustilleOptions options;
    rustille_options_init(&options);
    options.width = 80;

    char *art = NULL;
    if (rustille_render_rgba(rgba, len, w, h, &options, &art) == RUSTILLE_STATUS_OK) {
        puts(art);
        rustille_string_free(art);
    }

    include/rustille.h                            the header
    lib/$shared            shared library
    lib/$static            static library
    lib/pkgconfig/rustille.pc                     pkg-config metadata
    lib/cmake/rustille/rustille-config.cmake      CMake package config

Copy the tree over a prefix such as /usr/local, then:

    pkg-config --cflags --libs rustille
    find_package(rustille REQUIRED)
    target_link_libraries(my_app PRIVATE rustille::rustille)

The .pc file assumes a /usr/local prefix; edit its first line if you install
somewhere else.

Licensed under MIT OR Apache-2.0. See https://github.com/a269ch/rustille.
EOF

echo "==> $staging"
find "$staging" -type f | sed "s|$staging|.|"
