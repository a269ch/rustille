#!/usr/bin/env bash
# Sets the version of every package in the repository.
#
#   scripts/set-version.sh 0.2.0
#
# Afterwards, update CHANGELOG.md, commit, and tag with a leading `v`:
#
#   git tag v0.2.0 && git push origin main --tags
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ $# -ne 1 ]; then
    echo "usage: $(basename "$0") X.Y.Z[-prerelease]" >&2
    exit 2
fi

version="${1#v}"
if ! printf '%s' "$version" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+([-+].+)?$'; then
    echo "error: '$version' is not a semantic version" >&2
    exit 2
fi

# Rewrites the first `version = "..."` line of a TOML file.
set_toml_version() {
    local file="$1"
    local tmp
    tmp="$(mktemp)"
    awk -v version="$version" '
        !done && /^version[[:space:]]*=/ { print "version = \"" version "\""; done = 1; next }
        { print }
    ' "$file" >"$tmp"
    mv "$tmp" "$file"
    echo "  $file"
}

# Rewrites the first `"version": "..."` line of a JSON file.
set_json_version() {
    local file="$1"
    local tmp
    tmp="$(mktemp)"
    awk -v version="$version" '
        !done && /"version"[[:space:]]*:/ {
            print "  \"version\": \"" version "\","
            done = 1
            next
        }
        { print }
    ' "$file" >"$tmp"
    mv "$tmp" "$file"
    echo "  $file"
}

echo "setting every package to $version"
set_toml_version "$root/Cargo.toml"
set_toml_version "$root/bindings/python/Cargo.toml"
set_toml_version "$root/bindings/node/Cargo.toml"
set_toml_version "$root/bindings/wasm/Cargo.toml"
set_json_version "$root/bindings/node/package.json"

# The version macros in the generated C header.
tmp="$(mktemp)"
major="${version%%.*}"
rest="${version#*.}"
minor="${rest%%.*}"
patch="${rest#*.}"
patch="${patch%%[-+]*}"
awk -v version="$version" -v major="$major" -v minor="$minor" -v patch="$patch" '
    /^#define RUSTILLE_VERSION / { print "#define RUSTILLE_VERSION \"" version "\""; next }
    /^#define RUSTILLE_VERSION_MAJOR / { print "#define RUSTILLE_VERSION_MAJOR " major; next }
    /^#define RUSTILLE_VERSION_MINOR / { print "#define RUSTILLE_VERSION_MINOR " minor; next }
    /^#define RUSTILLE_VERSION_PATCH / { print "#define RUSTILLE_VERSION_PATCH " patch; next }
    { print }
' "$root/bindings/c/cbindgen.toml" >"$tmp"
mv "$tmp" "$root/bindings/c/cbindgen.toml"
echo "  $root/bindings/c/cbindgen.toml"

# Refresh the lock files so the workspace version lands in Cargo.lock too.
cargo metadata --format-version 1 --manifest-path "$root/Cargo.toml" >/dev/null
for manifest in python node wasm; do
    cargo metadata --format-version 1 \
        --manifest-path "$root/bindings/$manifest/Cargo.toml" >/dev/null
done

echo
echo "now update CHANGELOG.md, then:"
echo "  scripts/check-version.sh v$version"
