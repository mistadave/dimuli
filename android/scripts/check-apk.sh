#!/usr/bin/env bash
# APK gate: 16 KB zip/ELF alignment, no permissions, SDK levels, label.
# Usage: android/scripts/check-apk.sh [APK]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
APK="${1:-$ANDROID_DIR/app/build/outputs/apk/debug/app-debug.apk}"

PKG="ch.staeheli.dimuli"
BUILD_TOOLS="${ANDROID_HOME:-}/build-tools/37.0.0"
ZIPALIGN="$BUILD_TOOLS/zipalign"
AAPT2="$BUILD_TOOLS/aapt2"
READELF="${ANDROID_NDK_HOME:-}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf"
LIBS_DIR="$ANDROID_DIR/build/apk-libs"

fail() {
    echo "APK CHECK FAIL: $1"
    exit 1
}

[ -f "$APK" ] || fail "apk missing: $APK"
for tool in "$ZIPALIGN" "$AAPT2" "$READELF"; do
    [ -x "$tool" ] || fail "tool missing: $tool"
done
command -v unzip >/dev/null 2>&1 || fail "tool missing: unzip"

# (a) zip entries are 16 KB aligned.
zip_out="$("$ZIPALIGN" -c -P 16 -v 4 "$APK" 2>&1)" || {
    echo "$zip_out" | tail -n 5
    fail "zipalign -c -P 16"
}
[ "$(printf '%s\n' "$zip_out" | tail -n 1)" = "Verification successful" ] || {
    echo "$zip_out" | tail -n 5
    fail "zipalign -c -P 16 (no 'Verification successful')"
}

# (b) every LOAD segment of libdimuli.so is aligned to 0x4000, for both ABIs.
rm -rf "$LIBS_DIR"
mkdir -p "$LIBS_DIR"
unzip -q -o "$APK" 'lib/*/libdimuli.so' -d "$LIBS_DIR" || fail "unzip libdimuli.so"
for abi in arm64-v8a x86_64; do
    so="$LIBS_DIR/lib/$abi/libdimuli.so"
    [ -f "$so" ] || fail "libdimuli.so missing for $abi"
    load_lines="$("$READELF" -Wl "$so" | grep -E '^[[:space:]]*LOAD' || true)"
    [ -n "$load_lines" ] || fail "no LOAD segments in $abi libdimuli.so"
    while IFS= read -r line; do
        # The segment alignment is the last column of the program header line.
        [[ "$line" =~ 0x4000[[:space:]]*$ ]] || {
            echo "$line"
            fail "LOAD segment in $abi libdimuli.so not aligned to 0x4000"
        }
    done <<< "$load_lines"
done

# (c) the merged manifest requests no permissions.
perms="$("$AAPT2" dump permissions "$APK" 2>&1)" || fail "aapt2 dump permissions"
if printf '%s\n' "$perms" | grep -q 'uses-permission'; then
    printf '%s\n' "$perms" | grep 'uses-permission'
    fail "APK declares uses-permission"
fi

# (d) package, SDK levels and label.
badging="$("$AAPT2" dump badging "$APK" 2>&1)" || fail "aapt2 dump badging"
for needle in \
    "package: name='$PKG'" \
    "minSdkVersion:'33'" \
    "targetSdkVersion:'37'" \
    "application-label:'dimuli'"; do
    printf '%s\n' "$badging" | grep -qF -- "$needle" || fail "badging lacks $needle"
done

echo "APK CHECK OK"
