#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

fail() {
    echo "error: $*" >&2
    exit 1
}

toml_version() {
    local file="$1"
    grep -m1 -E '^version[[:space:]]*=' "$file" | cut -d'"' -f2
}

json_version() {
    local file="$1"
    grep -m1 -E '"version"[[:space:]]*:' "$file" | cut -d'"' -f4
}

declare -a names=()
declare -a versions=()

record() {
    names+=("$1")
    versions+=("$2")
}

workspace_version="$(toml_version "$root/Cargo.toml")"
[ -n "$workspace_version" ] || fail "could not read the workspace version from Cargo.toml"

record "Cargo.toml (workspace)"        "$workspace_version"
record "bindings/python/Cargo.toml"    "$(toml_version "$root/bindings/python/Cargo.toml")"
record "bindings/node/Cargo.toml"      "$(toml_version "$root/bindings/node/Cargo.toml")"
record "bindings/node/package.json"    "$(json_version "$root/bindings/node/package.json")"
record "bindings/wasm/Cargo.toml"      "$(toml_version "$root/bindings/wasm/Cargo.toml")"

header_version="$(grep -m1 -E '^#define RUSTILLE_VERSION ' "$root/bindings/c/cbindgen.toml" | cut -d'"' -f2)"
record "bindings/c/cbindgen.toml"      "$header_version"

expected="${1:-$workspace_version}"
expected="${expected#v}"

status=0
for index in "${!names[@]}"; do
    name="${names[$index]}"
    version="${versions[$index]}"
    if [ -z "$version" ]; then
        echo "  MISSING  $name" >&2
        status=1
    elif [ "$version" != "$expected" ]; then
        echo "  MISMATCH $name: $version (expected $expected)" >&2
        status=1
    else
        echo "  ok       $name: $version"
    fi
done

for manifest in "$root/crates/rustille/Cargo.toml" "$root/bindings/c/Cargo.toml"; do
    if ! grep -qE '^version\.workspace[[:space:]]*=[[:space:]]*true' "$manifest"; then
        echo "  MISMATCH ${manifest#"$root"/}: expected 'version.workspace = true'" >&2
        status=1
    else
        echo "  ok       ${manifest#"$root"/}: inherits $workspace_version"
    fi
done

if ! grep -qE "^## \[?$expected\]?" "$root/CHANGELOG.md"; then
    echo "  MISSING  CHANGELOG.md has no section for $expected" >&2
    status=1
else
    echo "  ok       CHANGELOG.md documents $expected"
fi

if [ "$status" -ne 0 ]; then
    fail "version mismatch; run scripts/set-version.sh $expected"
fi

echo "all packages are at $expected"
