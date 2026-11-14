---
phase: 05-android-shell
plan: 07
subsystem: android
tags: [android, real-device, android14, usbipd, wsl2, edge-to-edge, smoke]

requires:
  - phase: 05-android-shell
    plan: 06
    provides: "Gated full debug APK (check-apk.sh), emulator matrix API 33 / API 37"
provides:
  - "Debug APK installs, launches and closes on back on a real Android 14 device (OnePlus 9 Pro)"
  - "Edge-to-edge (self-enabled for SDK < 35) verified on real Android 14 hardware in portrait and landscape"
  - "README.md: WSL2 real-device setup (usbipd bind/attach, udev rule, adb), build/install/check commands"
affects: [05-08]

actuals:
  tokens: 1200
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

key-files:
  created:
    - README.md
  modified: []

key-decisions:
  - "05-07: Owner decision 2026-11-14: report written with the Android 14 device only; the Android 16 device and the manual touch checklist (Task 3) on both devices are deferred until all work is finished (final device test before v1 acceptance)"
  - "05-07: OxygenOS denies 'settings put system' to adb until 'Disable permission monitoring' is enabled in the developer options; owner enabled it, rotation via settings put now works on the OnePlus"

requirements-completed: []  # ANDR-08 (real multi-touch) and the Android 16 device are still open; see Deferred

coverage:
  - id: D1
    description: "Android 14 device connected to adb in WSL2"
    requirement: ANDR-01
    verification:
      - kind: other
        ref: "adb devices -l -> 8200fcc9 device product:OnePlus9Pro_EEA model:LE2123; getprop release 14, sdk 34"
        status: pass
    human_judgment: false
  - id: D2
    description: "Gated APK installed: assembleDebug + check-apk.sh"
    requirement: ANDR-07
    verification:
      - kind: build
        ref: "android/gradlew -p android assembleDebug && android/scripts/check-apk.sh -> APK CHECK OK (APK 20:10:39 local 2026-11-13, unchanged since 05-06)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Install, launch, 'dimuli started', back on start screen closes, no RustPanic/AndroidRuntime error on Android 14"
    requirement: ANDR-01
    verification:
      - kind: e2e
        ref: "android/build/smoke/device-8200fcc9-smoke.txt -> SMOKE OK 8200fcc9"
        status: pass
    human_judgment: false
  - id: D4
    description: "Edge-to-edge on Android 14: background behind bars, content clear of bars and cutout"
    requirement: ANDR-05
    verification:
      - kind: e2e
        ref: "android/build/smoke/device-8200fcc9-portrait.png, -landscape.png (inspected)"
        status: pass
    human_judgment: true
    rationale: "Insets are visual"
  - id: D5
    description: "Android 16 device: smoke and screenshots"
    requirement: ANDR-01
    verification: []
    status: deferred
    human_judgment: false
  - id: D6
    description: "Owner touch checklist (Task 3, steps 1-11) on Android 14 and Android 16, incl. Pitfall 6"
    requirement: ANDR-08
    verification: []
    status: deferred
    human_judgment: true

duration: 25min
completed: 2026-11-14
status: partial
---

# Phase 5 Plan 07: Real-Device Check Summary (Android 14 only)

**The gated debug APK installs and runs on a real Android 14 device (OnePlus 9 Pro): smoke test passes (install, launch, back closes the app, no panic), and the self-enabled edge-to-edge draws behind status and gesture bar with all content clear of bars and camera cutout in portrait and landscape. The Android 16 device and the owner's touch checklist (Task 3) are deferred by owner decision to a final device test once all work is done.**

## Device

| Field | Value |
|-------|-------|
| Serial | 8200fcc9 |
| Model | OnePlus 9 Pro (LE2123, OnePlus9Pro_EEA) |
| Android | 14 (API 34) |
| ABI | arm64-v8a |
| Page size | 4096 |
| Screen | 1080 x 2412, punch-hole camera top left |

Android 16 device: not connected (deferred).

## Task Results

| Task | Result |
|------|--------|
| 1. Connect devices | Android 14 connected via usbipd-win (`usbipd bind` in admin PowerShell, `usbipd.exe attach --wsl --busid 1-1`) plus udev rule `/etc/udev/rules.d/51-android.rules` (VID 18d1, MODE 0666). Android 16: deferred |
| 2. Install / launch / back / screenshots | PASS on Android 14 (details below). Android 16: deferred |
| 3. Owner touch checklist | Deferred (owner decision, both devices) |

## Gate Results

| Gate | Result |
|------|--------|
| `android/gradlew -p android assembleDebug && android/scripts/check-apk.sh` | PASS (`APK CHECK OK`; gradle up to date, APK identical to 05-06) |
| `android/scripts/smoke.sh -s 8200fcc9` | PASS (`SMOKE OK 8200fcc9`, `device-8200fcc9-smoke.txt`) |
| two physical devices in `adb devices` | NOT MET (one device; deferred) |
| two `SMOKE OK` device transcripts | NOT MET (one; deferred) |

## Insets (Android 14, D-07)

- **Portrait** (`device-8200fcc9-portrait.png`): app background #101114 behind status bar and gesture bar; title, text, button and Slint badge well clear of both bars and the cutout.
- **Landscape** (`device-8200fcc9-landscape.png`): cutout on the left; the button starts at x 168 (left cutout inset honoured) and ends 72 px from the right edge (normal padding); nothing under status bar, gesture bar or cutout; background behind the bars.
- Verdict: edge-to-edge self-enabled on SDK < 35 works on real Android 14 hardware, insets applied once.

## Deviations from Plan

1. **[Owner decision] Android 16 device and Task 3 deferred.** The owner will test both devices together once all remaining work (05-08, phase 6+) is done. ANDR-08 (real multi-touch) therefore stays unproven on hardware.
2. **[Environment] Rotation on OxygenOS.** `settings put system accelerometer_rotation/user_rotation` first failed (`SecurityException: com.android.shell was not granted android.permission.WRITE_SETTINGS`), so the first landscape shot used `cmd window user-rotation lock 1` / `free`. The owner then enabled "Berechtigungsüberwachung deaktivieren" (Disable permission monitoring) in the developer options; after that `settings put` works and the landscape screenshot was retaken the plan's way (identical result). Owner settings restored to accelerometer_rotation 1, user_rotation 0.
3. **[Environment] No separate logcat transcript.** The device dropped off USB/IP before the logcat dump; smoke.sh already checks for RustPanic and AndroidRuntime errors and reported none.

## Issues Encountered (WSL2 device access, documented in README.md)

- Before USB debugging was on, the phone enumerated as `18d1:4ee8 MIDI function` only.
- Right after enabling it Windows showed `0000:0002 Unbekanntes USB-Gerät (Fehler beim Anfordern einer Gerätebeschreibung)`; fixed by a phone restart.
- `usbipd bind` needs admin rights; `attach` does not.
- `adb devices` showed `no permissions` until the udev rule existed and the device was detached and re-attached (first re-attach: `Device in error state`, second attempt worked).
- After USB/IP detaches (cable, sleep, Windows) the device must be attached again.

## Deferred (final device test)

- Android 16 device: Task 2 (smoke, portrait/landscape screenshots).
- Task 3 on both devices: picker with own photos, divider drag, pinch/pan, long-press swap, Pitfall 6 (long-press + second finger), templates/settings, rotation in the editor, save into Pictures/dimuli (Google Photos), share sheet with preview (empty preview seen on emulators in 05-06), back confirm dialog.

## Task Commits

None (CLAUDE.md: no commits without explicit request). HEAD unchanged at `262306a`.

## Next

- 05-08 (milestone close) can proceed; it must record ANDR-08 and the Android 16 check as deferred to the final device test.

---
*Phase: 05-android-shell*
*Completed (partial): 2026-11-14*
