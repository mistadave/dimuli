#!/usr/bin/env bash
# Smoke test: install, launch, check start-up log, back closes the app.
# Usage: android/scripts/smoke.sh [-s SERIAL] [APK]
# Never sends scroll/mouse-wheel input (Slint's Android backend panics on it).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PKG="ch.staeheli.dimuli"
SERIAL=""

while getopts "s:" opt; do
    case "$opt" in
        s) SERIAL="$OPTARG" ;;
        *) echo "usage: $0 [-s SERIAL] [APK]" >&2; exit 2 ;;
    esac
done
shift $((OPTIND - 1))
APK="${1:-$REPO_ROOT/android/app/build/outputs/apk/debug/app-debug.apk}"

if [ -z "$SERIAL" ]; then
    SERIAL="$(adb devices | awk 'NR>1 && $2=="device" {print $1; exit}')"
fi
[ -n "$SERIAL" ] || { echo "SMOKE FAIL: no device"; exit 1; }
ADB=(adb -s "$SERIAL")

fail() { echo "SMOKE FAIL: $1"; exit 1; }

# wait for boot
booted=""
for _ in $(seq 1 90); do
    booted="$("${ADB[@]}" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"
    [ "$booted" = "1" ] && break
    sleep 2
done
[ "$booted" = "1" ] || fail "boot"

[ -f "$APK" ] || fail "apk missing: $APK"
"${ADB[@]}" install -r "$APK" 2>&1 | grep -q "Success" || fail "install"

"${ADB[@]}" logcat -c
"${ADB[@]}" shell am force-stop "$PKG"
"${ADB[@]}" shell am start -W -n "$PKG/.MainActivity" 2>&1 | grep -q "Status: ok" || fail "launch"

started=""
for _ in $(seq 1 15); do
    if "${ADB[@]}" logcat -d -s dimuli 2>/dev/null | grep -q "dimuli started" \
        && [ -n "$("${ADB[@]}" shell pidof "$PKG" | tr -d '\r')" ]; then
        started=1
        break
    fi
    sleep 1
done
[ -n "$started" ] || fail "start log / process"

mkdir -p "$REPO_ROOT/android/build/smoke"
"${ADB[@]}" exec-out screencap -p > "$REPO_ROOT/android/build/smoke/$SERIAL-start.png" || fail "screenshot"

"${ADB[@]}" shell input keyevent KEYCODE_BACK
closed=""
for _ in $(seq 1 10); do
    top="$("${ADB[@]}" shell dumpsys activity activities | grep topResumedActivity || true)"
    if ! echo "$top" | grep -q "$PKG"; then
        closed=1
        break
    fi
    sleep 1
done
[ -n "$closed" ] || fail "back did not close the app"

errors="$("${ADB[@]}" logcat -d -s 'RustPanic:*' 'AndroidRuntime:E' | grep -v '^--------- beginning of' || true)"
[ -z "$errors" ] || { echo "$errors"; fail "RustPanic/AndroidRuntime entries in logcat"; }

echo "SMOKE OK $SERIAL"
