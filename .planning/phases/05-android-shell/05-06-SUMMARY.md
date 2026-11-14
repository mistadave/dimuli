---
phase: 05-android-shell
plan: 06
subsystem: android
tags: [android, emulator, avd, 16kb-pages, zipalign, aapt2, edge-to-edge, back-handling, api33, api37]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 05
    provides: "UI pass (landscape editor, drawn icons, launcher icon) verified on dimuli_api36"
provides:
  - "AVD dimuli_api33 (system-images;android-33;google_apis;x86_64, pixel_8) in ~/.android/avd"
  - "android/scripts/check-apk.sh [APK]: zipalign -c -P 16, LOAD alignment 0x4000 for arm64-v8a and x86_64, no uses-permission, package/minSdk 33/targetSdk 37/label; prints APK CHECK OK"
  - "Full flow matrix (launch, back, pick, save, share, rotation, insets) passed on API 33 and API 37 emulators"
  - "Research assumptions A1 (back on API 33 reaches OnBackInvokedCallback) and A2 (self-enabled edge-to-edge on API 33) confirmed; no MainActivity.kt change needed"
affects: [05-07, 05-08]

# Actuals (#2632) - chars/4 over the realized diff: check-apk.sh (3.1 KB) is the only repo file this plan added
actuals:
  tokens: 780
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "APK gate script: tool paths from ANDROID_HOME (build-tools 37.0.0) and ANDROID_NDK_HOME (llvm-readelf); each step prints 'APK CHECK FAIL: <step>' and exits 1"
    - "Emulator matrix run one emulator at a time on fixed ports (5556 = api33, 5558 = api37), evidence saved to android/build/smoke/<serial>-*.txt before 'emu kill' so the verify block can be re-run offline"

key-files:
  created:
    - android/scripts/check-apk.sh
  modified: []

key-decisions:
  - "05-06: A1 and A2 confirmed on dimuli_api33 without a code change; MainActivity.kt (listed in files_modified for a possible fix) stays untouched"
  - "05-06: sdkmanager ran with stdin from /dev/null so no license prompt could be answered on the owner's behalf; the android-sdk-license was already accepted, the install finished without a prompt"
  - "05-06: VALIDATION.md status column not changed: the plan does not ask for it and no earlier plan of this phase updated it"

patterns-established:
  - "Picker on API 33 and on this API 37 image both run as com.google.android.providers.media.module/...PhotoPickerActivity; tiles by content-desc, confirm button text 'Add (3)'"

requirements-completed: []  # Plan lists ANDR-01..07; all are listed again by 05-07 (real devices) and 05-08 (milestone close), so none is marked complete here (same handling as 05-03..05-05).

coverage:
  - id: D1
    description: "AVD dimuli_api33 exists next to dimuli_api36 and dimuli_api37"
    requirement: ANDR-07
    verification:
      - kind: other
        ref: "avdmanager list avd | grep -c 'Name: dimuli_api33' -> 1 (pixel_8, Android 13.0 google_apis/x86_64)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Full debug APK: zipalign 16 KB ok, every LOAD segment 0x4000 for both ABIs, no uses-permission, minSdk 33, targetSdk 37, package ch.staeheli.dimuli, label dimuli"
    requirement: ANDR-07
    verification:
      - kind: build
        ref: "android/gradlew -p android assembleDebug && android/scripts/check-apk.sh -> BUILD SUCCESSFUL, APK CHECK OK, exit 0 (APK 20:10:39 local, both libdimuli.so inside)"
        status: pass
    human_judgment: false
  - id: D3
    description: "All Android flows on API 33 (emulator-5556)"
    requirement: ANDR-01
    verification:
      - kind: e2e
        ref: "android/build/smoke/emulator-5556-{smoke,logcat,mediastore,chooser}.txt; verify block exit 0; screenshots emulator-5556-*.png inspected"
        status: pass
    human_judgment: true
    rationale: "Insets and dialog presence are visual"
  - id: D4
    description: "All Android flows on API 37 (emulator-5558)"
    requirement: ANDR-01
    verification:
      - kind: e2e
        ref: "android/build/smoke/emulator-5558-{smoke,logcat,mediastore,chooser}.txt; verify block exit 0; screenshots emulator-5558-*.png inspected"
        status: pass
    human_judgment: true
    rationale: "Insets and dialog presence are visual"
  - id: D5
    description: "A1: KEYCODE_BACK in the editor on API 33 reaches dimuli (confirm dialog, dimuli stays topResumedActivity)"
    requirement: ANDR-06
    verification:
      - kind: e2e
        ref: "emulator-5556-confirm.png ('Collage verwerfen?'), topResumedActivity = ch.staeheli.dimuli/.MainActivity after back"
        status: pass
    human_judgment: true
  - id: D6
    description: "A2: edge-to-edge on API 33 (setDecorFitsSystemWindows(false)) draws the app background behind status and gesture bar with single insets"
    requirement: ANDR-05
    verification:
      - kind: e2e
        ref: "emulator-5556-start-portrait.png, -start-landscape.png, -editor.png, -editor-landscape.png: #101114 behind both bars, top bar starts directly below the 63 px status bar, settings toggle ends directly above the gesture bar"
        status: pass
    human_judgment: true
    rationale: "Inset correctness is visual"

# Metrics
duration: 12min
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 06: Emulator Gate API 33 / API 37 Summary

**New AVD `dimuli_api33` (Android 13), a reusable `android/scripts/check-apk.sh` that proves the full both-ABI debug APK is 16 KB aligned (zipalign and every ELF LOAD segment at 0x4000), requests no permission and targets minSdk 33 / targetSdk 37, and a complete flow run on the API 33 and API 37 emulators: launch, back closes on the start screen, back in the editor asks for confirmation, 3-image pick, save into `Pictures/dimuli`, share opens the system chooser, rotation keeps the collage in the same process, nothing under the bars. Research assumptions A1 and A2 hold on API 33 without any code change.**

## Performance

- **Duration:** about 12 min (2026-11-13T19:11:31Z to 19:23:37Z), including the 1.5 GB system image download, two emulator boots and the gate re-runs. The check script was written beforehand by rust-coder and is not timed here.
- **Tasks:** 2 of 2
- **Files:** 1 created (android/scripts/check-apk.sh by rust-coder), 0 modified

## Accomplishments

- **AVD:** `sdkmanager --install "system-images;android-33;google_apis;x86_64"` (stdin from /dev/null, no prompt appeared, exit 0; the SDK license had already been accepted by the owner earlier), then `echo no | avdmanager create avd -n dimuli_api33 -k "system-images;android-33;google_apis;x86_64" -d pixel_8`. Config matches dimuli_api36 (1080x2400, 420 dpi, 2 GB RAM).
- **check-apk.sh** (rust-coder): `set -euo pipefail`; pre-checks the APK, zipalign, aapt2, llvm-readelf and unzip; (a) last line of `zipalign -c -P 16 -v 4` must be `Verification successful`; (b) fresh `android/build/apk-libs`, both `arm64-v8a` and `x86_64` libdimuli.so required, at least one LOAD line each and every LOAD line ends in `0x4000`; (c) no `uses-permission` in `aapt2 dump permissions`; (d) badging contains package, `minSdkVersion:'33'`, `targetSdkVersion:'37'`, `application-label:'dimuli'`. rust-coder proved every failure path once in a scratch directory. I read the script and it matches the plan spec.
- **Emulator matrix:** run by me on emulator-5556 (dimuli_api33) and then emulator-5558 (dimuli_api37), one at a time, with the full APK from Task 1. Results below.

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; HEAD unchanged at `262306a`, 0 commits since. `android/scripts/check-apk.sh` is untracked (all of `android/` is still untracked from 05-02).

Suggested commit message when the owner asks to commit (together with or after 05-01..05-05):

```
Add APK gate script and API 33 emulator check (phase 5, plan 06)

- android/scripts/check-apk.sh: 16 KB zip and ELF LOAD alignment for
  both ABIs, no uses-permission, package, minSdk 33, targetSdk 37 and
  label; prints APK CHECK OK
- Full flow verified on Android 13 and Android 17 emulators (launch,
  back, pick, save, share, rotation, insets); back and edge-to-edge
  on Android 13 work without changes
```

## Files Created/Modified

- `android/scripts/check-apk.sh` (new, executable, rust-coder)
- Outside the repository: AVD `~/.android/avd/dimuli_api33.avd`, system image `$ANDROID_HOME/system-images/android-33/google_apis/x86_64`
- Evidence (gitignored, `android/build/smoke/`): `emulator-5556-*` and `emulator-5558-*` (`smoke.txt`, `logcat.txt`, `mediastore.txt`, `chooser.txt`, `start.png` from smoke.sh, `start-portrait.png`, `start-landscape.png`, `editor.png`, `confirm.png`, `saved.png`, `share.png`, `editor-landscape.png`, `editor-after-rotation.png`, `launcher.png`); `android/build/apk-libs/` (extracted libs from check-apk.sh)

## Gate Results (all re-run individually after the emulators were stopped)

| Gate | Result |
|------|--------|
| `avdmanager list avd \| grep -c 'Name: dimuli_api33'` | PASS (1) |
| `android/gradlew -p android assembleDebug && android/scripts/check-apk.sh` | PASS (`BUILD SUCCESSFUL`, `APK CHECK OK`, exit 0; APK unchanged at 20:10:39 local, i.e. the installed one) |
| `grep -c 'SMOKE OK emulator-5556' …-5556-smoke.txt && grep -c 'SMOKE OK emulator-5558' …-5558-smoke.txt` | PASS (1, 1) |
| API 33 evidence gate (start count, pick/save/share lines, no RustPanic/FATAL, MediaStore row, chooser) | PASS (exit 0) |
| API 37 evidence gate (same) | PASS (exit 0) |
| `cargo fmt --all --check` | PASS (no Rust change in this plan) |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (86 + 104 passed, 0 failed) |
| shellcheck on check-apk.sh | Not run: shellcheck is not installed |

Acceptance criteria, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `avdmanager list avd \| grep -c 'Name: dimuli_api33'` | >= 1 | 1 |
| `test -x android/scripts/check-apk.sh` | exit 0 | 0 |
| `grep -c 'APK CHECK OK' check-apk.sh` | >= 1 | 1 |
| `grep -c 'zipalign -c -P 16' check-apk.sh` | >= 1 | 2 |
| check-apk.sh on the full APK | `APK CHECK OK` | `APK CHECK OK` |
| `SMOKE OK <serial>` in both smoke transcripts | 1 each | 1 / 1 |
| `dimuli started` per logcat | exactly 1 | 1 / 1 |
| `picked images: 3`, `saved to gallery: true`, `share sheet opened: true` | >= 1 each | 1/1/1 on both |
| `RustPanic` / `FATAL EXCEPTION` | 0 | 0 / 0 |
| MediaStore row `relative_path=Pictures/dimuli/, is_pending=0, owner_package_name=ch.staeheli.dimuli` | >= 1 | 1 / 1 |
| chooser file names the system chooser | match | API 33 `android/com.android.internal.app.ChooserActivity`; API 37 `com.android.intentresolver/.ChooserActivityLauncher` |
| Back in editor leaves dimuli top resumed | yes | yes on both |
| Results table and A1/A2 verdicts in SUMMARY | present | below |

## Emulator Results

All screenshots opened and judged by me. Paths relative to the repo root, under `android/build/smoke/`.

| Check | API 33 (emulator-5556, Android 13, no cutout) | API 37 (emulator-5558, Android 17, top cutout) |
|-------|------------------------------------------------|-----------------------------------------------|
| launch | PASS: `SMOKE OK`, `am start -W` Status ok, exactly 1 `dimuli started` (`emulator-5556-smoke.txt`, `-logcat.txt`) | PASS: same (`emulator-5558-smoke.txt`, `-logcat.txt`) |
| start-screen back | PASS: smoke.sh back -> topResumedActivity is no longer dimuli | PASS: same |
| editor back confirm | PASS (A1): KEYCODE_BACK -> dimuli stays top resumed, "Collage verwerfen?" with Abbrechen / Verwerfen (`emulator-5556-confirm.png`); Abbrechen returns to the editor | PASS: same (`emulator-5558-confirm.png`) |
| pick | PASS: system picker (`com.google.android.providers.media.module/…PhotoPickerActivity`), 3 tiles, "Add (3)" -> `picked images: 3`, editor with 3 images in the 3-image template (`emulator-5556-editor.png`) | PASS: same picker package on this image, `picked images: 3` (`emulator-5558-editor.png`; order blue/red/green because of picker order, fine) |
| save | PASS: `saved to gallery: true`, banner "Collage gespeichert." below the top bar with drawn cross (`emulator-5556-saved.png`), MediaStore row 3 `Pictures/dimuli/, is_pending=0, ch.staeheli.dimuli` (`-mediastore.txt`) | PASS: same (`emulator-5558-saved.png`, `-mediastore.txt`) |
| share | PASS: `share sheet opened: true`, top = `android/com.android.internal.app.ChooserActivity` (`-chooser.txt`), sheet with Nearby/Edit and Print/Drive/Maps/Messages (`emulator-5556-share.png`); back closes it, dimuli top again | PASS: top = `com.android.intentresolver/.ChooserActivityLauncher`, "Sharing image" sheet (`emulator-5558-share.png`); back closes it |
| rotation | PASS: start screen and editor, pid 5520 before/landscape/after; landscape editor = canvas left, side panel right with template bar and "Einstellungen" + up chevron (`emulator-5556-editor-landscape.png`); after rotating back the same collage (`-editor-after-rotation.png`) | PASS: pid 2913 at all points; same layouts (`emulator-5558-editor-landscape.png`, `-editor-after-rotation.png`) |
| insets portrait | PASS (A2): app background #101114 behind status bar and gesture bar; title/button not under any bar; editor top bar starts directly below the 63 px status bar, settings toggle ends above the gesture bar, so insets are applied once (`emulator-5556-start-portrait.png`, `-editor.png`) | PASS: top bar below the taller status bar (cutout, about 132 px), bottom clear (`emulator-5558-start-portrait.png`, `-editor.png`) |
| insets landscape | PASS: no cutout on this image; content within the status bar / gesture bar insets, canvas body ends above the gesture bar (`emulator-5556-start-landscape.png`, `-editor-landscape.png`) | PASS: left cutout inset honoured (start button at x 195 vs 63 in portrait, editor top bar from x 132), nothing under the cutout or bars (`emulator-5558-start-landscape.png`, `-editor-landscape.png`) |
| no crash | PASS: `logcat -d -s 'dimuli:*' 'RustPanic:*' 'AndroidRuntime:E'` has only the 4 dimuli info lines + state line; 0 RustPanic / FATAL | PASS: same |

Extra (05-05 follow-up "launcher icon on API 33 and 37"): API 33 app drawer shows "dimuli" with the adaptive collage-grid icon (`emulator-5556-launcher.png`); API 37 home dock shows the same icon (`emulator-5558-launcher.png`, the drawer swipe did not open the drawer there, label covered by badging `application-label:'dimuli'`).

### Assumption verdicts

- **A1 (back on API 33 reaches dimuli's `OnBackInvokedCallback`): CONFIRMED.** Back in the editor shows the confirm dialog and dimuli stays top resumed; back on the start screen closes the app (smoke.sh). No fix needed.
- **A2 (self-enabled edge-to-edge on API 33): CONFIRMED.** `setDecorFitsSystemWindows(false)` for SDK < 35 draws the app background behind both bars, and Slint's safe-area insets are applied once (no double top or bottom padding). No fix needed; MainActivity.kt unchanged.

## Decisions Made

- No MainActivity.kt change: A1 and A2 pass, so the conditional fix path of Task 2 was not needed.
- dimuli_api36 (emulator-5554) was stopped before booting API 33 to keep memory free (15 GB host, about 6 GB available); its flows were covered by 05-02..05-05 and nothing in this plan changed the APK contents, so no re-run on API 36 was required.
- VALIDATION.md status column left as is (see key-decisions).

## Deviations from Plan

None that change the result. Two run-time notes:

**1. [Environment] First start-screen capture on the fresh API 33 image was disturbed by the launcher**
- **What happened:** right after the first boot, my first launch's portrait screenshot showed the Pixel launcher Overview (Recents) with a first-run tooltip, and ActivityManager logged `Killing 2696:ch.staeheli.dimuli … remove task` about 7 s after launch, before the rotation took effect; the landscape screenshot then showed the home screen.
- **Evidence it is not an app fault:** no RustPanic, AndroidRuntime or dimuli error line; no crash; the kill reason is task removal by the system UI, not a crash or ANR.
- **Handling:** cleared logcat and repeated step 3 once the launcher had settled; the repeat passed (pid 5520 unchanged through rotation) and every later step used that process. Both start screenshots were overwritten by the repeat. The saved logcat has exactly one `dimuli started` because it was cleared before the repeat.

**2. [Environment] `accelerometer_rotation` on API 37 read back 0 right after the restore**
- Setting it to 1 again made it stick (`1`); `user_rotation` is 0. Both emulators ended with user_rotation 0 / accelerometer_rotation 1.

## Issues Encountered

- API 33 share sheet showed no image preview at screenshot time (empty area above Nearby/Edit), API 37 shows "Sharing image" with an empty preview area too. Probably the preview was still loading 5 s after opening; the share intent itself works (chooser opened with image targets). Worth a look on a real device in 05-07.
- shellcheck is not installed on the host, so the script was not linted.

## Follow-ups / Open Items

From this plan:
- Share sheet preview empty on both emulators at the time of the screenshot; check on a real device whether the `ClipData` thumbnail shows (05-07).
- smoke.sh: its start screenshot is taken before Slint's first frame (rust-coder report; still open, smoke.sh outside this plan's files).
- Optional: install shellcheck and lint `android/scripts/*.sh` (new tool, owner decision).
- 05-07: run `android/scripts/check-apk.sh` before installing on the real devices (validation row 05-07-T2).

Still open from 05-02..05-05:
- Code review (`code-reviewer`) for 05-01..05-06 not run yet; due before committing (05-08).
- 05-08 review notes: stale doc comment in controller.rs (~line 176); Kotlin failure branches of `saveToGallery`/`share`; `io.execute` after `io.shutdown()`.
- Possibly superfluous `@Deprecated`/`@Suppress("DEPRECATION")` on `onActivityResult`.
- Quote logcat filter specs in plans and docs (`'RustPanic:*'`) because of zsh.
- Portrait format buttons in the settings panel very tall; landscape last TemplateBar thumbnail cut; launcher icon grid exceeds the safe zone; `Theme.top-bar-height` token; desktop hint lost "Mausrad"; optional second chevron Path (05-05).
- Landscape settings scroll by finger, predictive back animation, HEIC/WebP, gallery view of saved collages: real devices (05-07).
- Owner visual confirmation of the inset screenshots (05-02 and this plan).

## Open Human Checks

- Owner look at `android/build/smoke/emulator-5556-*.png` and `emulator-5558-*.png` (plan's human-check): no content under bars or cutout in portrait and landscape, app background behind the bars on API 33, confirm dialog after back, side-panel layout in landscape. I inspected all of them and they match.

## User Setup Required

None. (The API 33 system image installed without a license prompt.)

## Next Phase Readiness

- Ready for 05-07 (real devices Android 14 and 16; needs the owner to connect them).
- Emulators: none left running. dimuli_api36 (emulator-5554) was stopped at the start of this plan, emulator-5556 and emulator-5558 were stopped after their evidence was saved. Restart with `emulator -avd dimuli_api36` if needed. dimuli_api33 and dimuli_api37 have the full debug APK from 20:10:39 local installed; dimuli_api36 still has the x86_64-only build from 05-05.

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED

check-apk.sh and all 26 evidence files (8 txt, 18 png) exist and are non-empty; AVD directory ~/.android/avd/dimuli_api33.avd exists; 0 commits since 262306a (as required, no commits).
