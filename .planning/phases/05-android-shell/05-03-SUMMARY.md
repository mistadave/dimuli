---
phase: 05-android-shell
plan: 03
subsystem: android-shell
tags: [android, jni, photo-picker, media-store, kotlin, cache, back-gesture, emulator]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 02
    provides: "Gradle shell, MainActivity JNI bridge (native { } guard, back callback), smoke.sh, dimuli_api36 tracer"
provides:
  - "platform/native_events.rs: host-tested images_picked_event / pick_failed_event (4 tests)"
  - "AndroidPlatform::pick_images over JNI pickImages(I)V; natives native_images_picked(String[]) and native_pick_failed() (NATIVES = 3)"
  - "MainActivity: pickImages (ACTION_PICK_IMAGES, limit min(max, getPickImagesMaxLimit)), onActivityResult (cancel / clipData / data fallback / all-copies-failed), copy to cacheDir/picked/img-<i> on single-thread executor io, stale cache cleared in onCreate"
  - "ANDR-02 end to end and the editor half of ANDR-06 proven on dimuli_api36"
affects: [05-04, 05-05, 05-06, 05-07, 05-08]

# Actuals (#2632) - chars/4 over the lines this plan added (native_events.rs whole, pick/native parts of android.rs and MainActivity.kt)
actuals:
  tokens: 2100
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Native results mapped to PlatformEvent by pure functions in platform/native_events.rs (cfg(any(android, test))), JNI glue stays thin"
    - "Kotlin I/O on one single-thread executor (io); onCreate cleanup and copy jobs are serialized, so a result delivered after process death cannot race the cleanup"
    - "Picker cache files named only by index (img-<i>), never from URI or display name"

key-files:
  created:
    - crates/dimuli-app/src/platform/native_events.rs
  modified:
    - crates/dimuli-app/src/platform/mod.rs
    - crates/dimuli-app/src/platform/android.rs
    - crates/dimuli-app/src/texts.rs
    - android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt

key-decisions:
  - "texts::PICK_FAILED is cfg(any(target_os = android, test)), because native_events (compiled for host tests) uses it; keeps the 05-01 rule of no #[allow(dead_code)]"
  - "Cancel leaves cacheDir/picked untouched; the directory is only replaced when a new pick returns URIs"

patterns-established:
  - "Emulator picker driving: uiautomator dump of the system picker (Slint surface stays invisible), tiles by content-desc, 'Done' button; dimuli's own buttons from screencap coordinates"
  - "zsh: quote logcat filter specs ('RustPanic:*') or the glob aborts the command and an empty-output check passes falsely"

requirements-completed: []  # Plan declares [ANDR-02, ANDR-06]; both are shared with 05-06/05-07/05-08 (API 33/37 matrix, real devices, milestone close), so neither is marked complete here.

coverage:
  - id: D1
    description: "Mapping of picked paths and pick failure to PlatformEvent is host-tested"
    requirement: ANDR-02
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib native_events -> 4 passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Picking 3 images copies them to cache/picked/img-0..2, Rust logs 'picked images: 3' and the editor opens with them"
    requirement: ANDR-02
    verification:
      - kind: e2e
        ref: "emulator-5554: logcat 'picked images: 3' (1), run-as ls cache/picked -> img-0 img-1 img-2 (3888/3887/3888 bytes = sources); android/build/smoke/api36-editor.png"
        status: pass
    human_judgment: true
    rationale: "Editor rendering is visual; executor inspected the screenshot"
  - id: D3
    description: "Cancelling the picker logs 'picked images: 0' and leaves the start screen unchanged"
    requirement: ANDR-02
    verification:
      - kind: e2e
        ref: "emulator-5554: picker top-resumed, KEYCODE_BACK, dimuli top-resumed again, logcat 'picked images: 0'; android/build/smoke/api36-cancel.png"
        status: pass
    human_judgment: false
  - id: D4
    description: "Single pick reaches Rust and opens the editor"
    requirement: ANDR-02
    verification:
      - kind: e2e
        ref: "emulator-5554: logcat 'picked images: 1', cache/picked contains only img-0; android/build/smoke/api36-single.png"
        status: pass
    human_judgment: false
  - id: D5
    description: "Back in the editor asks 'Collage verwerfen?' and keeps the app; after Verwerfen, back on the start screen closes it"
    requirement: ANDR-06
    verification:
      - kind: e2e
        ref: "emulator-5554 (run twice, after 3-pick and after single pick): topResumedActivity dimuli after back in editor; android/build/smoke/api36-confirm.png; after Verwerfen + back topResumedActivity = NexusLauncherActivity"
        status: pass
    human_judgment: false

# Metrics
duration: 32min (verification pass incl. full emulator re-run; implementation by rust-coder not timed)
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 03: Photo Picker Slice Summary

**On Android, "Bilder wählen" now opens the system Photo Picker (`ACTION_PICK_IMAGES`, limit `min(9, getPickImagesMaxLimit())`). The chosen images are copied to `cacheDir/picked/img-<i>` on a background executor, handed to Rust over JNI (`nativeImagesPicked`) and the editor opens with them. Cancel keeps the start screen, a single pick works, a picker that cannot start or copies that all fail show the German `PICK_FAILED` message, and back in the editor asks "Collage verwerfen?" before discarding. Verified independently on dimuli_api36.**

## Performance

- **Duration:** about 32 min for this verification pass (2026-11-13T16:34:39Z to 17:06Z), including a full independent emulator re-run of every plan check. Implementation and rust-coder's own emulator runs were done beforehand and not timed here.
- **Tasks:** 2 of 2
- **Files:** 1 created, 4 modified

## Accomplishments

- `platform/native_events.rs` (new, `cfg(any(target_os = "android", test))`): `images_picked_event` (drops empty strings, keeps order) and `pick_failed_event`, with 4 host tests
- `platform/android.rs`: `pick_images` calls Kotlin `pickImages(I)V` (`i32::try_from(max).unwrap_or(i32::MAX)`, JNI error -> log + `PickFailed`). `NATIVES` has 3 entries now: `native_images_picked` reads the `String[]` with `len`/`get_element`/`try_to_string` (errors propagate, no indexing), logs `picked images: N` and posts the event; `native_pick_failed` logs `photo picker failed` and posts `PickFailed`
- `MainActivity.kt`: `io` single-thread executor (shut down in `onDestroy`), `REQUEST_PICK`, `pickImages` with `ActivityNotFoundException` -> `nativePickFailed`, `onActivityResult` (cancel/no URI -> empty array without touching the cache; `clipData` with `data` fallback; copy on `io`; all copies failed -> `nativePickFailed`), `copyToCache` writes only `img-<index>` and logs exceptions without content, `onCreate` deletes stale `cache/picked`
- No permission added (`uses-permission` count 0), no HEIC/WebP conversion code (D-12), no AndroidX (D-09)

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `262306a`).

Suggested commit message when the owner asks to commit (together with or after 05-01/05-02):

```
Open the system Photo Picker on Android (phase 5, plan 03)

- Kotlin pickImages starts ACTION_PICK_IMAGES with a limit of
  min(9, getPickImagesMaxLimit()); results are copied to
  cacheDir/picked/img-<i> on a single-thread executor
- Rust receives the paths over JNI (nativeImagesPicked) and opens
  the editor; cancel sends an empty list, a picker that cannot
  start or failed copies send nativePickFailed
- platform/native_events.rs maps the native results to
  PlatformEvents (host-tested)
- stale cache copies are removed when the activity starts
```

## Files Created/Modified

- `crates/dimuli-app/src/platform/native_events.rs` (new): result mapping + 4 tests
- `crates/dimuli-app/src/platform/mod.rs`: `#[cfg(any(target_os = "android", test))] mod native_events;`
- `crates/dimuli-app/src/platform/android.rs`: JNI `pick_images`, natives `native_images_picked`, `native_pick_failed`
- `crates/dimuli-app/src/texts.rs`: `PICK_FAILED` cfg widened to `any(target_os = "android", test)` (not in the plan's file list, see Deviations)
- `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt`: picker, result handling, cache copy, cleanup
- Evidence (gitignored, `android/build/smoke/`): `api36-editor.png`, `api36-confirm.png`, `api36-cancel.png`, `api36-single.png` (overwritten by my re-run), plus `api36-start-verify.png`, `api36-after-discard.png`, `api36-confirm-single.png`, `api36-after-discard-2.png`. rust-coder's scratch files `dbg.png`, `start.png`, `ui.xml` also lie there.

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS (86 + 96 passed, 0 failed; was 86 + 92 after 05-02, +4 native_events tests) |
| `cargo test -p dimuli-app --lib native_events` | PASS: 4 passed (`picked_paths_become_images_picked_event`, `picked_empty_list_means_cancelled`, `picked_empty_strings_are_dropped`, `picked_failure_carries_pick_failed_text`) |
| `cargo test -p dimuli-app --lib back` | PASS: 6 passed, including `back_asks_for_confirmation_and_discards_on_confirm` and `back_on_start_screen_is_not_handled_and_back_again_closes_dialog` |
| Android clippy x86_64 (`cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`) | PASS (exit 0, no warning/error lines) |
| `android/gradlew -p android assembleDebug -PrustAbis=x86_64 --rerun-tasks` | PASS (`BUILD SUCCESSFUL`, no Kotlin `w:`/`e:` lines; `:app:compileDebugKotlin --rerun-tasks` again clean) |
| `android/scripts/smoke.sh -s emulator-5554` | PASS: `SMOKE OK emulator-5554` |
| T1 device gate (`picked images: 3` >= 1 and >= 3 `img-` files) | PASS (1 line; img-0, img-1, img-2) |
| T2 device gate (`picked images: 0` >= 1 and no RustPanic / AndroidRuntime:E entries) | PASS (see note: first attempt was invalid under zsh, re-run quoted) |
| T2 close gate (topResumedActivity no longer dimuli after Verwerfen + back) | PASS (NexusLauncherActivity) |

Acceptance greps, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `mod native_events;` in mod.rs | 1 | 1 |
| `fn images_picked_event` in native_events.rs | 1 | 1 |
| `native_images_picked` in android.rs | >= 2 | 3 |
| `picked images: ` in android.rs | 1 | 1 |
| Kotlin literals (ACTION_PICK_IMAGES, EXTRA_PICK_IMAGES_MAX, getPickImagesMaxLimit, external nativeImagesPicked, override onActivityResult, clipData) | 6 | 6 |
| `"img-` in MainActivity.kt | >= 1 | 1 |
| `fn pick_failed_event` in native_events.rs | 1 | 1 |
| `native_pick_failed` in android.rs | >= 2 | 3 |
| Kotlin literals (external nativePickFailed, ActivityNotFoundException, deleteRecursively, emptyArray) | 4 | 4 |
| native_events tests | >= 4 passed | 4 |

Extra checks: key links present (`pickImages` in android.rs 1, `images_picked_event` in android.rs 2, `ImagesPicked` in controller.rs 1); no `.unwrap()`/`.expect(` in android.rs (0); `uses-permission` in the manifest 0.

## Emulator Results (dimuli_api36, emulator-5554, 1080x2400, x86_64 build)

All checks below were re-run by me in one session (17:35-17:37 local) after rebuilding and installing the APK; rust-coder's earlier screenshots with the same names were replaced by mine.

| Check | Evidence | Result |
|-------|----------|--------|
| Stale cache removed on start | After launch: `run-as ls cache/picked` -> "No such file or directory" (rust-coder's earlier copies gone) | PASS |
| Picker opens | Tap "Bilder wählen" (540,1185); uiautomator shows the system picker half sheet ("dimuli will only have access to the photos you select") | PASS |
| 3-image pick | Three 5:06 PM tiles selected ("3 photos or videos selected"), Done -> `picked images: 3`; `cache/picked` = img-0/1/2 with the source sizes 3888/3887/3888 bytes | PASS |
| Editor with 3 images | `android/build/smoke/api36-editor.png`: red on top, green and blue below in the 3-image template, top bar "‹ Zurück / Speichern / Teilen" below the status bar | PASS |
| Back in editor | KEYCODE_BACK -> topResumedActivity still `ch.staeheli.dimuli`; `android/build/smoke/api36-confirm.png` shows "Collage verwerfen?" with "Wenn du zurückgehst, geht die aktuelle Collage verloren." and Abbrechen / Verwerfen over the dimmed editor | PASS |
| Verwerfen | Tap (812,1360) -> start screen (`api36-after-discard.png`), app still top | PASS |
| Cancel | Picker opened (top = photopicker), KEYCODE_BACK -> `picked images: 0` (logged on the UI thread), dimuli top again; `android/build/smoke/api36-cancel.png` = unchanged start screen, no message; cache untouched (img-0..2 still there) | PASS |
| Single pick | One tile, Done -> `picked images: 1`; `cache/picked` now holds only img-0 (directory recreated); `android/build/smoke/api36-single.png` = editor with one green image and only the 1-image template | PASS |
| Back -> Verwerfen -> back closes | KEYCODE_BACK in editor (dimuli stays), Verwerfen, KEYCODE_BACK on start -> topResumedActivity = NexusLauncherActivity | PASS |
| No crash | `logcat -d -s 'RustPanic:*' 'AndroidRuntime:E'` empty (exit 0); 0 `dimuli` error lines | PASS |

Reused from rust-coder (not re-run): the failure path was seen live once. A stale picker index (Pitfall 7) gave a URI whose copy failed, Kotlin called `nativePickFailed` and the `PICK_FAILED` banner appeared. The `ActivityNotFoundException` branch was not triggered (the picker exists on every API 33+ image) and is reviewed in code only.

## Decisions Made

- I re-ran every emulator check myself instead of only reusing rust-coder's screenshots. The images overwrite rust-coder's files with the same names, so the referenced evidence is from this pass.
- ANDR-02 and ANDR-06 are not marked complete: 05-06, 05-07 and 05-08 list them again (API 33/37 matrix, real devices, milestone close).

## Deviations from Plan

**1. [Rule 3 - Blocking] `texts::PICK_FAILED` under `cfg(any(target_os = "android", test))`**
- **Issue:** `native_events.rs` is compiled for host tests and uses `PICK_FAILED`, which 05-01 had limited to `cfg(target_os = "android")`. Without the change the host test build fails.
- **Fix:** cfg widened by one condition. This follows the 05-01 decision (cfg instead of `#[allow(dead_code)]`, guideline §3 allows only two `#[allow]` exceptions).
- **Files:** crates/dimuli-app/src/texts.rs (not in the plan's `files_modified`)
- **Judgment:** acceptable, needed and minimal.

**2. [Rule 1 - quality, no behaviour change] `@Deprecated(...)` on the `onActivityResult` override plus `@Suppress("DEPRECATION")` on the `super` call**
- **Reported reason:** silence a Kotlin warning.
- **My check:** In `platforms/android-37.0/android.jar`, `Activity.onActivityResult(int, int, Intent)` and `startActivityForResult(Intent, int)` have no `Deprecated` attribute. 42 other Activity members do, so the markers are present in the jar. The plan says the same ("not deprecated in API 37"). So the annotation is most likely unnecessary and marks our own override as deprecated for no reason. It does not change behaviour, and all gates are clean.
- **Action:** not a FIX_REQUEST. Listed as a follow-up for the 05-08 code review: remove `@Deprecated` and the `@Suppress`, rebuild, and keep them only if `compileDebugKotlin` then really warns.
- **Files:** android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt

**3. [Coverage gap] `Intent.data` single-pick fallback not exercised live**
- The picker in multi-select mode (`EXTRA_PICK_IMAGES_MAX` > 1) returns a single pick in `clipData` as well. The `data` fallback only runs when the extra is left out (`limit <= 1`), which `MAX_IMAGES = 9` never does today. The code path (`fromClip.ifEmpty { listOfNotNull(data.data) }`) is correct on review. The plan truth "a single image reaches Rust and opens the editor" holds; only the `data` branch is unproven on a device.

**Total deviations:** 3 (1 blocking-avoidance, 1 superfluous annotation, 1 untested defensive branch). No scope creep, no behaviour change.

## Issues Encountered

- **zsh glob in the crash gate:** the plan's `adb logcat -d -s RustPanic:* AndroidRuntime:E` fails under zsh with "no matches found: RustPanic:*". The empty output then made `test -z` pass even though logcat had not run. Re-run with quoted filters (`'RustPanic:*'`, exit 0): still empty, so the gate really passes. smoke.sh is bash and not affected. Plans and docs should quote the filter.
- **Missing glyph after "Einstellungen":** the chevron next to "Einstellungen" renders as tofu (box) on Android (`api36-editor.png`, `api36-single.png`). It is already in scope for 05-05 ("settings chevron").
- **Editor hint mentions "Mausrad"** ("Zwei Finger oder Mausrad: zoomen") on Android too. This is a UI text item for 05-05 or Phase 6.
- **Large empty settings area:** the collapsed settings panel takes about the lower third of the portrait editor. UI pass item (05-05).

## Threat Model Check

- T-05-08 / T-05-09: copies only in `cacheDir/picked`, names `img-<index>`; on the device the files are `-rw-------` owned by the app uid. Directory recreated per pick, deleted in `onCreate` (both seen on the emulator).
- T-05-10: logs carry only counts and exception messages (`picked images: N`, `copy failed`, `photo picker failed`).
- T-05-11: copy runs on `io` (the native call for 3 and 1 picks came from thread 5636, not the UI thread 5539).
- T-05-07: decode limits unchanged; picker limit set through `EXTRA_PICK_IMAGES_MAX`.
- No new security surface beyond the threat model.

## Follow-ups / Open Items

From this plan:
- 05-08 code review: remove the superfluous `@Deprecated`/`@Suppress("DEPRECATION")` on `onActivityResult` if the compiler stays quiet (Deviation 2). Also check that `io.execute` can never run after `io.shutdown()` (it would throw `RejectedExecutionException`; today `onActivityResult` cannot arrive after `onDestroy` of the same instance because `configChanges` prevents recreation).
- Quote logcat filter specs in plans and docs (`'RustPanic:*'`) because of zsh.
- 05-05: settings chevron tofu, "Mausrad" in the Android hint, height of the collapsed settings panel.
- HEIC/WebP (D-12) not tested on the device. Behaviour comes from the existing decoder and the "übersprungen" notice, not from code in this plan.

Still open from 05-02:
- Code review (`code-reviewer`) for 05-01 + 05-02 + 05-03 not run yet; due before committing (scheduled in 05-08).
- smoke.sh screenshot shows the splash, not the first Slint frame (wait before `screencap`).
- Slint badge "Version / slint.dev" text is dark on dark (05-05 "readable Slint attribution").
- Predictive back animation on a real device (05-07).
- Owner visual confirmation of the 05-02 inset screenshots.

## Open Human Checks

- Owner visual confirmation (plan `<human-check>`): open `android/build/smoke/api36-editor.png`, `android/build/smoke/api36-cancel.png`, `android/build/smoke/api36-single.png` and `android/build/smoke/api36-confirm.png`. Expected: 3-image editor with red/green/blue; unchanged start screen with no message; 1-image editor; "Collage verwerfen?" dialog with Verwerfen / Abbrechen. I inspected all four and they match.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 05-04 (save and share). The Rust -> Kotlin call with arguments (`pickImages(I)V`) and Kotlin -> Rust with an array (`nativeImagesPicked([Ljava/lang/String;)V`) are proven, and so is the `io` executor pattern.
- Emulator dimuli_api36 is still running as emulator-5554 with the x86_64-only debug build installed and dimuli closed (launcher on top). Test images `/sdcard/Pictures/pick-{a,b,c}.jpg` are on the device.

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED

All 3 code files and the 4 plan-named evidence screenshots exist; 0 commits since 262306a (as required, no commits).
