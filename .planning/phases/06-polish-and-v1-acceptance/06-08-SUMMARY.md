---
phase: 06-polish-and-v1-acceptance
plan: 08
status: complete
requirements: [ACPT-01, LIFE-01, PERF-01, POLI-01, ANDR-01, ANDR-02, ANDR-03, ANDR-04, ANDR-05, ANDR-06, ANDR-07, ANDR-08]
completed: 2026-11-15
actuals:
  tasks: 3
  commits: 0
---

# Phase 6 Plan 08: Owner's final device test

The owner ran the final test on Android 16 (emulator dimuli_api36) and Android 14 (OnePlus 9 Pro) and approved both: all steps OK. Milestones 5 and 6 are ticked in docs/PLAN.md; ANDR-01..08, LIFE-01, PERF-01, ERRM-01, POLI-01 and ACPT-01 are marked complete; 06-VALIDATION.md is green.

## Deviation (owner decision)

ACPT-01 asks for a **real** Android 16 device. No real Android 16 phone was attached; the owner ran the Android 16 part on the emulator and decided that emulator Android 16 + real Android 14 are sufficient for v1. Likewise the 05-07 Android 16 inset screenshots and the multi-touch "second finger" check on Android 16 were not done on hardware (second finger checked on the OnePlus).

## Devices

| Device | Android | SDK | Page size | Smoke |
|---|---|---|---|---|
| emulator-5554 (dimuli_api36) | 16 | 36 | – | `SMOKE OK emulator-5554` |
| OnePlus 9 Pro LE2123 (8200fcc9) | 14 | 34 | 4096 | `SMOKE OK 8200fcc9` |

Both-ABI APK: `APK CHECK OK`. The OnePlus got an arm64-only build of the same sources because usbipd dropped the USB connection on the 35 MB APK.

## Peak memory

| Run | VmHWM | VmRSS |
|---|---:|---:|
| Emulator API 36, 9-image collage (no 80 MP photo), saved | 500624 kB (~489 MiB) | 179320 kB |
| OnePlus 9 Pro, owner's photos incl. camera HEIC, saved | 593172 kB (~579 MiB) | 364876 kB |
| (06-06) Emulator API 36, 50 MP load/pan/save | 675048 kB | 673112 kB |

## Run notes

### Android 16 — emulator dimuli_api36 (emulator-5554), 2026-11-15

Pre-run on the emulator at the owner's request; **does not replace ACPT-01 on a real Android 16 device** unless the owner decides so.

- Build: commit 7c9572c + log fix, both-ABI debug APK; `SMOKE OK emulator-5554`
- Owner verdict: **all steps OK** (1, 2, 3 with 9 images, 4–11). Step 4 second finger not reproducible with a mouse.
- Log evidence:
  - 16:43 1 HEIC picked and transcoded; 16:44 4 picked, 1 `DecodeException` (intentionally broken p6-broken*.heic)
  - 16:50:30 9 picked → 3 copied, 1 `DecodeException`, 5 `FileNotFoundException`. Same second, system_server logs
    "Unable to check Uri permission because caller is holding WM lock; assuming permission denied" → platform race when
    the picker returns, not a dimuli defect; app counted and reported the failures, no crash. **Watch on real devices.**
  - 16:50:45 picker cancelled (0/0, no message); 16:51:00 9 picked, 0 failed, 1 transcoded; 16:51:11 saved
  - No crash buffer entries
- Downscaling (> 64 MP): not part of this run; covered by 06-06 matrix (owner decision)
- Memory (`android/build/smoke/p6-08-mem-emulator-5554.txt`): VmHWM 500624 kB (~489 MiB), VmRSS 179320 kB, after the 9-image collage without the 80 MP photo

### Android 14 — OnePlus 9 Pro (8200fcc9)

- Installed (arm64), `SMOKE OK 8200fcc9`
- Owner verdict: **all steps OK** (1–10 incl. second finger in step 4); HEIC/HEVC photos from the phone camera load without problems
- Log evidence (OnePlus rotates logcat quickly, only the end is left): 16:57:41 saved; 17:00:03 4 picked, 1 transcoded
  (the camera HEIC), 0 failed; 17:00:15 saved. No dimuli entry in the crash buffer. No picker permission failures seen.
- No "downscaled" line: the owner's largest photo is below 64 MP / 16 384 px
- Memory (`android/build/smoke/p6-08-mem-8200fcc9.txt`): VmHWM 593172 kB (~579 MiB), VmRSS 364876 kB, after loading and
  saving the large-photo collage

## Observations for later

- Emulator API 36, 16:50:30: 5 of 9 picked images failed with `FileNotFoundException` while system_server logged "Unable to check Uri permission because caller is holding WM lock; assuming permission denied". Retried pick succeeded with 9/0. Treated as a platform race on the emulator; not seen on the OnePlus. If it shows up on real devices, consider retrying the copy once.
- The checklist artifact did not persist results to its store; verdicts were taken from chat.
- Wi-Fi adb was not used (USB via usbipd `--auto-attach`); nothing to disconnect.
