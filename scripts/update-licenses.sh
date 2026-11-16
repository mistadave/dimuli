#!/usr/bin/env bash
# Regenerates crates/dimuli-app/licenses/rust-crates.txt (licenses of all Rust crates shipped in the
# Android APK) with cargo-about. The result is committed, so normal builds do not need cargo-about.
#
#   scripts/update-licenses.sh           regenerate the file
#   scripts/update-licenses.sh --check   exit non-zero if the committed file is outdated
#
# Needs: cargo install cargo-about
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="$root/crates/dimuli-app/licenses/rust-crates.txt"
manual="$root/crates/dimuli-app/licenses/other-components.txt"

mode="generate"
case "${1:-}" in
    "") ;;
    --check) mode="check" ;;
    *)
        echo "usage: $0 [--check]" >&2
        exit 2
        ;;
esac

if ! cargo about --version >/dev/null 2>&1; then
    echo "cargo-about is not installed (cargo install cargo-about)" >&2
    exit 2
fi

cd "$root"
tmp="$(mktemp)"
log="$(mktemp)"
trap 'rm -f "$tmp" "$log"' EXIT

# Slint has no license file in the crate metadata that cargo-about can read; it is listed by hand in
# other-components.txt. cargo-about only warns about it.
if ! cargo about generate --config about.toml --locked --offline --output-file "$tmp" about.hbs 2>"$log"; then
    cat "$log" >&2
    exit 1
fi

# Versions of crate-based components in the hand-maintained file must match Cargo.lock exactly.
manual_ok=1
for crate in slint skia-bindings; do
    versions="$(awk -v name="$crate" '$0 == "name = \"" name "\"" { getline; gsub(/[^0-9.]/, "", $0); print }' Cargo.lock)"
    if [ -z "$versions" ]; then
        echo "error: $crate not found in Cargo.lock" >&2
        manual_ok=0
    fi
    for version in $versions; do
        pattern="(^|[ (])$crate ${version//./\\.}([,) ]|\$)"
        if ! grep -Eq "$pattern" "$manual"; then
            echo "$manual does not list $crate $version; update it by hand" >&2
            manual_ok=0
        fi
    done
done
echo "reminder: check kotlin-stdlib and annotations in $manual by hand against the Gradle runtime classpath" >&2
if [ "$manual_ok" = 0 ] && [ "$mode" = "check" ]; then
    exit 1
fi

if [ "$mode" = "check" ]; then
    if cmp -s "$tmp" "$target"; then
        echo "licenses are up to date"
    else
        echo "$target is outdated; run scripts/update-licenses.sh" >&2
        exit 1
    fi
else
    cp "$tmp" "$target"
    chmod 0644 "$target"
    echo "wrote $target"
fi
