---
phase: 05-android-shell
plan: 04
subsystem: android-shell
tags: [android, jni, media-store, share-sheet, content-provider, kotlin, emulator]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 03
    provides: "native_events.rs pattern, io single-thread executor, Rust -> Kotlin calls with arguments, Kotlin -> Rust natives"
provides:
  - "native_events::saved_event / shared_event (host-tested, 4 tests)"
  - "AppState::save_finished (SAVED notice / error) and AppState::share_finished (no message on success, D-11; error on failure), 4 tests"
  - "AndroidPlatform::save_to_gallery / share over JNI saveToGallery([B)V / share([B)V through a shared hand_over helper; with_activity runs inside env.with_local_frame(16)"
  - "Natives native_saved / native_shared (NATIVES = 5)"
  - "MainActivity.saveToGallery (MediaStore IS_PENDING flow into Pictures/dimuli) and MainActivity.share (cacheDir/share/dimuli-collage.jpg + createChooser)"
  - "ShareProvider.kt: own read-only, non-exported ContentProvider for exactly content://ch.staeheli.dimuli.share/dimuli-collage.jpg (D-09)"
  - "ANDR-03 and ANDR-04 end to end on dimuli_api36"
affects: [05-05, 05-06, 05-07, 05-08]

# Actuals (#2632) - chars/4 over the lines this plan added (ShareProvider.kt whole, save/share parts of MainActivity.kt and android.rs, saved/shared parts of native_events.rs and state.rs incl. tests)
actuals:
  tokens: 2900
  tasks: 3
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Rust -> Kotlin byte hand-over: one hand_over(method, jpeg) helper with jni_sig!((jpeg: jbyte[]) -> void); a JNI error becomes the matching failure event"
    - "Per-call JNI local frame (with_local_frame(16)) because the android_main thread stays attached for the app's life"
    - "MediaStore write with IS_PENDING 1 -> write -> IS_PENDING 0; on failure delete only the URI returned by this insert"
    - "Own ContentProvider for one file: whole-URI equality, read-only mode, exported=false + grantUriPermissions=true, grant carried by ClipData"

key-files:
  created:
    - android/app/src/main/kotlin/ch/staeheli/dimuli/ShareProvider.kt
  modified:
    - crates/dimuli-app/src/platform/native_events.rs
    - crates/dimuli-app/src/state.rs
    - crates/dimuli-app/src/controller.rs
    - crates/dimuli-app/src/texts.rs
    - crates/dimuli-app/src/platform/android.rs
    - android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt
    - android/app/src/main/AndroidManifest.xml
    - docs/ARCHITECTURE.md

key-decisions:
  - "texts::SHARE_FAILED is cfg(any(target_os = android, test)), same rule as PICK_FAILED in 05-03 (native_events is compiled for host tests)"
  - "Save and share failure paths are proven by host tests and code review only; on the emulator only the success paths were driven"

patterns-established:
  - "Share check on the emulator: topResumedActivity = com.android.intentresolver/.ChooserActivityLauncher while the chooser is open; provider denial checked with adb shell content query"

requirements-completed: []  # Plan declares [ANDR-03, ANDR-04]; both are listed again by 05-06, 05-07 and 05-08 (API 33/37 matrix, real devices, milestone close), so neither is marked complete here (same handling as 05-03).

coverage:
  - id: D1
    description: "Saved/Shared native results map to PlatformEvents with the German failure texts"
    requirement: ANDR-03
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib saved -> 4 passed; cargo test -p dimuli-app --lib shared -> 4 passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Share success shows no message (D-11); share failure and save failure show error messages; save success shows 'Collage gespeichert.'"
    requirement: ANDR-04
    verification:
      - kind: unit
        ref: "state::tests::shared_success_shows_no_message, shared_failure_shows_error_message, saved_success_shows_saved_notice, saved_failure_shows_error_message"
        status: pass
    human_judgment: false
  - id: D3
    description: "Speichern writes a finished MediaStore image into Pictures/dimuli without a permission prompt and shows the notice"
    requirement: ANDR-03
    verification:
      - kind: e2e
        ref: "emulator-5554 re-run 19:16 local: new row dimuli-1794593774.jpg relative_path=Pictures/dimuli/ is_pending=0 owner ch.staeheli.dimuli; logcat 'saved to gallery: true'; pulled file is a 3000x3000 JPEG; android/build/smoke/api36-saved.png"
        status: pass
    human_judgment: true
    rationale: "Gallery app presentation is visual and not checked (owner)"
  - id: D4
    description: "Teilen opens the system chooser with an image preview, no banner afterwards"
    requirement: ANDR-04
    verification:
      - kind: e2e
        ref: "rust-coder run 19:14 local: api36-chooser.txt = com.android.intentresolver/.ChooserActivityLauncher; api36-share-logcat.txt 'share sheet opened: true'; api36-share.png (preview of the collage), api36-after-share.png (no banner)"
        status: pass
    human_judgment: true
    rationale: "Executor inspected both screenshots; artefacts not re-generated"
  - id: D5
    description: "ShareProvider is not reachable without the grant"
    requirement: ANDR-04
    verification:
      - kind: e2e
        ref: "adb shell content query --uri content://ch.staeheli.dimuli.share/dimuli-collage.jpg -> SecurityException: Permission Denial ... not exported from UID 10216 (re-run by executor)"
        status: pass
    human_judgment: false

# Metrics
duration: 12min (verification pass incl. emulator spot-check; implementation by rust-coder not timed)
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 04: Save and Share Slice Summary

**On Android, "Speichern" now writes the exported JPEG through MediaStore into `Pictures/dimuli/dimuli-<unix seconds>.jpg` (IS_PENDING flow, no permission) and shows "Collage gespeichert."; "Teilen" writes `cacheDir/share/dimuli-collage.jpg` and opens the system share sheet through an own read-only, non-exported `ShareProvider` (no AndroidX), with no success message (D-11). Results come back as `Saved`/`Shared` events mapped by host-tested Rust code; failures show the German messages. Verified on dimuli_api36.**

## Performance

- **Duration:** about 12 min for this verification pass (2026-11-13T18:15Z to 18:27Z): all gates and greps re-run, one save re-run on the emulator, provider denial re-run, share artefacts inspected. Implementation and rust-coder's emulator runs were done before and are not timed here.
- **Tasks:** 3 of 3
- **Files:** 1 created, 8 modified (incl. one docs line)

## Accomplishments

- `platform/native_events.rs`: `saved_event(ok)` / `shared_event(ok)` (failure carries `SAVE_FAILED` / `SHARE_FAILED`), tests `saved_ok_maps_to_saved_ok`, `saved_failure_carries_save_failed_text`, `shared_ok_maps_to_shared_ok`, `shared_failure_carries_share_failed_text`
- `state.rs`: `save_finished` (Ok -> `SAVED` notice, Err -> log + error) and `share_finished` (Ok -> only `log::info!("share sheet opened")`, Err -> log + error), with 4 tests
- `controller.rs`: `Saved`/`Shared` go to `save_finished`/`share_finished`; `show_result` removed. The desktop share answer (`Err(SHARE_UNAVAILABLE)`) still shows its error through `share_finished`
- `texts.rs`: `SHARED` removed; `SHARE_FAILED` cfg widened to `any(target_os = "android", test)`
- `platform/android.rs`: `with_activity` wraps the closure in `env.with_local_frame(16, ...)`; new `hand_over(method, jpeg)` creates the byte array and calls `([B)V`; `save_to_gallery` / `share` post `Saved(Err(SAVE_FAILED))` / `Shared(Err(SHARE_FAILED))` on a JNI error; `NATIVES` has 5 entries; `native_saved` logs `saved to gallery: {ok}`, `native_shared` logs `share sheet opened: {ok}`
- `MainActivity.kt` (inside `// region JNI bridge`, KDoc both sides): `saveToGallery` on `io` (DISPLAY_NAME `dimuli-<s>.jpg`, MIME `image/jpeg`, RELATIVE_PATH `Pictures/dimuli`, IS_PENDING 1 -> write -> 0; failure logs, deletes only its own inserted URI, `nativeSaved(false)`); `share` on `io` writes the cache file (explicit `mkdirs` check), then on the UI thread `createChooser(ACTION_SEND)` with `EXTRA_STREAM`, `ClipData.newRawUri`, `FLAG_GRANT_READ_URI_PERMISSION`; `ActivityNotFoundException` -> `nativeShared(false)`
- `ShareProvider.kt` (new): whole-URI equality with `uri()`, `FileNotFoundException` for anything else and for a missing context, mode other than `"r"` -> `SecurityException`, `MODE_READ_ONLY`, `query` with `OpenableColumns` honouring the projection, `getType` `image/jpeg`, insert/update/delete no-ops
- `AndroidManifest.xml`: `<provider android:name=".ShareProvider" android:authorities="ch.staeheli.dimuli.share" android:exported="false" android:grantUriPermissions="true" />`; still no `uses-permission`
- `docs/ARCHITECTURE.md` (export data flow): notes that sharing shows no success message (D-11)

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `262306a`).

Suggested commit message when the owner asks to commit (together with or after 05-01..05-03):

```
Save to the gallery and share on Android (phase 5, plan 04)

- Speichern writes the JPEG through MediaStore into Pictures/dimuli
  (IS_PENDING while writing, no storage permission); a failed save
  removes only its own half-written row
- Teilen writes cacheDir/share/dimuli-collage.jpg and opens the system
  share sheet; an own read-only, non-exported ShareProvider serves the
  one file through a temporary read grant (no AndroidX FileProvider)
- Saved/Shared results are mapped by host-tested Rust code; sharing
  shows no success message (D-11), failures show German messages
- JNI calls from Rust run inside a local reference frame
```

## Files Created/Modified

- `android/app/src/main/kotlin/ch/staeheli/dimuli/ShareProvider.kt` (new)
- `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt`: `saveToGallery`, `share`, `nativeSaved`, `nativeShared`
- `android/app/src/main/AndroidManifest.xml`: `<provider>`
- `crates/dimuli-app/src/platform/native_events.rs`: `saved_event`, `shared_event` + 4 tests
- `crates/dimuli-app/src/platform/android.rs`: local frame, `hand_over`, JNI save/share, 2 natives
- `crates/dimuli-app/src/state.rs`: `save_finished`, `share_finished` + 4 tests
- `crates/dimuli-app/src/controller.rs`: event routing, `show_result` and the then unused `texts` import removed
- `crates/dimuli-app/src/texts.rs`: `SHARED` removed, `SHARE_FAILED` cfg widened
- `docs/ARCHITECTURE.md`: one sentence (share without success message), not in the plan's file list
- Evidence (gitignored, `android/build/smoke/`): `api36-saved.png`, `api36-chooser.txt`, `api36-share-logcat.txt`, `api36-share.png`, `api36-after-share.png` (all rust-coder, 19:13-19:14 local). rust-coder scratch files `s0.png`, `s1.png`, `emulator-5554-start.png` also lie there.

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS (86 + 104 passed, 0 failed; was 86 + 96 after 05-03, +8 tests) |
| `cargo test -p dimuli-app --lib saved` | PASS: 4 passed (2 native_events, 2 state) |
| `cargo test -p dimuli-app --lib shared` | PASS: 4 passed (2 native_events, 2 state) |
| `cargo test -p dimuli-app --lib shared_success_shows_no_message` | PASS: 1 passed |
| Android clippy x86_64 (`ANDROID_JAR=... cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`) | PASS (exit 0, no warning/error lines) |
| `android/gradlew -p android assembleDebug -PrustAbis=x86_64` | PASS (`BUILD SUCCESSFUL`) |
| `:app:compileDebugKotlin --rerun-tasks` | PASS, no Kotlin `w:`/`e:` lines (only Gradle's generic plugin deprecation notice, unchanged since 05-02) |
| T2 device gate (MediaStore row in Pictures/dimuli/ is_pending=0 owner ch.staeheli.dimuli AND `saved to gallery: true`) | PASS (exit 0, after my own save) |
| T3 artefact gate (`share sheet opened: true` in api36-share-logcat.txt AND intentresolver/ChooserActivity in api36-chooser.txt) | PASS (exit 0) |
| T3 provider gate (shell query refused) | PASS (exit 0): `java.lang.SecurityException: Permission Denial: opening provider ch.staeheli.dimuli.ShareProvider from (null) (pid=6531, uid=2000) that is not exported from UID 10216` |

Acceptance greps, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `fn saved_event` in native_events.rs | 1 | 1 |
| `fn shared_event` in native_events.rs | 1 | 1 |
| `fn share_finished` in state.rs | 1 | 1 |
| `fn save_finished` in state.rs | 1 | 1 |
| `fn show_result` in controller.rs | 0 | 0 |
| `texts::SHARED` in controller.rs | 0 | 0 |
| `SHARED: &str` in texts.rs | 0 | 0 |
| `native_saved` in android.rs | >= 2 | 3 |
| `with_local_frame` in android.rs | >= 1 | 1 |
| Kotlin save literals (IS_PENDING, RELATIVE_PATH, VOLUME_EXTERNAL_PRIMARY, DIRECTORY_PICTURES, external nativeSaved, fun saveToGallery) | 6 | 6 |
| `!!` in MainActivity.kt | 0 | 0 (also 0 in ShareProvider.kt) |
| `uses-permission` in the manifest | 0 | 0 |
| `native_shared` in android.rs | >= 2 | 3 |
| ShareProvider literals (class ShareProvider : ContentProvider(), FileNotFoundException, MODE_READ_ONLY, SecurityException, OpenableColumns.DISPLAY_NAME) | 5 | 5 |
| Manifest provider literals (authorities, grantUriPermissions="true", exported="false") | 3 | 3 |
| `androidx.` in the manifest | 0 | 0 |
| Kotlin share literals (Intent.createChooser, ClipData.newRawUri, FLAG_GRANT_READ_URI_PERMISSION, external nativeShared, fun share() | 5 | 5 |

Extra checks: key links present (`saveToGallery` in android.rs 1, `nativeShared` in MainActivity.kt 5, `ShareProvider.uri()` in MainActivity.kt 1, used for both `EXTRA_STREAM` and `ClipData` through a local `val uri`); `.unwrap()`/`.expect(` in android.rs 0; no `@Deprecated`/`@Suppress` added by this plan (the three in MainActivity.kt are from 05-02/05-03: `setDecorFitsSystemWindows`, `onActivityResult`).

## Emulator Results (dimuli_api36, emulator-5554, 1080x2400, x86_64 build)

| Check | Evidence | Source | Result |
|-------|----------|--------|--------|
| Save shows the notice, no permission dialog | `android/build/smoke/api36-saved.png`: banner "Collage gespeichert." over the 3-image editor (red/green/blue), no system dialog | rust-coder, inspected by me | PASS |
| Save re-run | Editor already open; tap "Speichern" (605,215) -> within 1 s `saved to gallery: true`; my screenshot shows the same banner; MediaStore now has row 4 `dimuli-1794593774.jpg, relative_path=Pictures/dimuli/, is_pending=0, owner_package_name=ch.staeheli.dimuli` next to rust-coder's row 3 (`dimuli-1794593628.jpg`) | me, 19:16 local | PASS |
| Saved file is a real collage JPEG | `/sdcard/Pictures/dimuli/dimuli-1794593774.jpg` pulled: JPEG, 3000x3000, 343629 bytes (same size as the share cache file of the same collage) | me | PASS |
| Chooser opens | `android/build/smoke/api36-chooser.txt`: `topResumedActivity=... com.android.intentresolver/.ChooserActivityLauncher` | rust-coder | PASS |
| Share log | `android/build/smoke/api36-share-logcat.txt`: `share sheet opened: true` (UI thread) and `dimuli::state: share sheet opened` (Rust main loop) | rust-coder | PASS |
| Share sheet preview | `android/build/smoke/api36-share.png`: "Sharing image" with a preview of the red/green/blue collage and targets (Quick Share, Print, Drive, Maps, Messages); the receiver could read the URI through the grant | rust-coder, inspected by me | PASS |
| No banner after share (D-11) | `android/build/smoke/api36-after-share.png`: editor without any banner | rust-coder, inspected by me | PASS |
| Share cache file | `run-as ls cache/share` -> `dimuli-collage.jpg`, 343629 bytes, `-rw-------` app uid | me | PASS |
| Provider denies the shell | SecurityException "Permission Denial ... not exported" (see gate table) | me | PASS |
| No crash | `logcat -d -s 'RustPanic:*' 'AndroidRuntime:E'` (quoted) after my save: empty, exit 0 | me | PASS |

I did not regenerate the share artefacts: the chooser activity, the log line, the preview image and the no-banner screenshot together prove the share truths, and the share code path ends where my save check started (same `hand_over`).

## Decisions Made

- Spot-check instead of a full re-run: rust-coder's artefacts are consistent with each other (timestamps 19:13-19:14, same collage, same file size as my save) and I re-ran the save, the MediaStore query, the provider denial and the crash check myself.
- ANDR-03 and ANDR-04 are not marked complete: 05-06, 05-07 and 05-08 list them again.

## Deviations from Plan

**1. [Rule 3 - Blocking] `texts::SHARE_FAILED` under `cfg(any(target_os = "android", test))`**
- **Issue:** `native_events.rs` is compiled for host tests and now uses `SHARE_FAILED`, which 05-01 limited to `cfg(target_os = "android")`. Without the change the host test build fails.
- **Fix:** cfg widened by one condition, same as `PICK_FAILED` in 05-03 (cfg instead of `#[allow(dead_code)]`).
- **Judgment:** acceptable, needed and minimal. texts.rs is in the plan's file list.

**2. [Rule 3 - Blocking] Unused `use crate::texts` removed from controller.rs**
- **Issue:** `show_result` was the last user of `texts::SAVED`/`texts::SHARED` in the controller; after deleting it the import is unused and clippy `-D warnings` fails.
- **Judgment:** direct consequence of plan step 3, no behaviour change.

**3. [Rule 1 - quality, no behaviour change] `hand_over(method, jpeg)` helper in android.rs**
- `save_to_gallery` and `share` share one helper instead of two copies of the JNI call. Signature, error mapping and local frame are as planned.

**4. [Docs] `docs/ARCHITECTURE.md` export data flow updated**
- One sentence: sharing shows no success message (D-11). Not in `files_modified`, but the architecture doc described the old behaviour. Acceptable.

**5. [Coverage gap] Failure paths only host-tested and reviewed**
- Not driven on the emulator: a failed MediaStore insert/write (row deletion), a failed share-file write, `ActivityNotFoundException` for the chooser, and a JNI error in `hand_over`. The Rust half (event mapping, messages) is covered by the 8 new host tests; the Kotlin half was read in code: `uri` is set only after a successful `insert`, so the `catch` deletes only this call's row; the share write failure returns before the chooser and reports `nativeShared(false)`.
- **Judgment:** acceptable for this plan; the plan truth "A failed save removes the half-written MediaStore row" rests on code review (planned for 05-08).

**Total deviations:** 5 (2 blocking-avoidance, 1 refactor, 1 doc sync, 1 coverage gap). No scope creep.

## Issues Encountered

- **Banner close button renders as tofu:** the "✕" on the message banner shows a missing-glyph box on Android (`api36-saved.png`), like the settings chevron seen in 05-03. Same root cause (glyph not in the Android font fallback); belongs to 05-05.
- **Stale doc comment:** `controller.rs` line ~176 still says the platform's answer (`Saved`/`Shared`) "arrives later as a notice"; for `Shared` success that is no longer true. Cosmetic, for the 05-08 review.
- The saved banner covers the top bar (Zurück/Speichern/Teilen) while it is shown. Existing message layout, not new in this plan; note for the 05-05 UI pass.

## Threat Model Check

- T-05-12: provider `exported="false"`, `grantUriPermissions="true"`; shell gets Permission Denial (re-run).
- T-05-13: `isServed(uri) = uri == uri()`; served path is built only from `cacheDir`, `"share"` and `FILE_NAME`.
- T-05-14: `mode != "r"` -> `SecurityException`; `MODE_READ_ONLY`; insert/update/delete no-ops.
- T-05-15: `Intent.createChooser` with `FLAG_GRANT_READ_URI_PERMISSION` on one URI, no persistable flag.
- T-05-16: IS_PENDING 1 -> 0 (row seen with `is_pending=0`); failure deletes only the URI from this insert.
- T-05-17: `uses-permission` count 0; no permission dialog in `api36-saved.png`.
- No new security surface beyond the threat model.

## Follow-ups / Open Items

From this plan:
- 05-05: tofu "✕" on the message banner (with the settings chevron); banner over the top bar.
- 05-08 code review: stale doc comment in controller.rs (~line 176); review the Kotlin failure branches of `saveToGallery`/`share` (Deviation 5); `io.execute` after `io.shutdown()` would throw `RejectedExecutionException` (also applies to `saveToGallery`/`share` now).
- 05-06: run save and share on API 33 and 37 (MediaStore query and chooser check as here).
- The gallery now holds two test collages (`Pictures/dimuli/dimuli-1794593628.jpg`, `dimuli-1794593774.jpg`) on dimuli_api36; harmless, can stay as test data.

Still open from 05-02/05-03:
- Code review (`code-reviewer`) for 05-01..05-04 not run yet; due before committing (scheduled in 05-08).
- Remove the likely superfluous `@Deprecated`/`@Suppress("DEPRECATION")` on `onActivityResult` if the compiler stays quiet.
- Quote logcat filter specs in plans and docs (`'RustPanic:*'`) because of zsh.
- 05-05: settings chevron tofu, "Mausrad" in the Android hint, height of the collapsed settings panel, readable Slint attribution.
- smoke.sh screenshot shows the splash, not the first Slint frame.
- Predictive back animation on a real device (05-07); owner visual confirmation of the 05-02 inset screenshots.
- HEIC/WebP (D-12) not tested on the device.

## Open Human Checks

- Gallery app view (plan Task 2 `<human-check>`): on the emulator open Google Photos (or Files) and look for the album/folder "dimuli". Expected: the saved collage (red top, green and blue below) is visible. Not done by me or rust-coder; the MediaStore row and the pulled 3000x3000 JPEG show the file is complete.
- Owner visual confirmation: `android/build/smoke/api36-saved.png` (banner "Collage gespeichert.", no permission dialog), `android/build/smoke/api36-share.png` (share sheet with collage preview), `android/build/smoke/api36-after-share.png` (no banner). I inspected all three and they match.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 05-05 (UI pass). Save and share work end to end on API 36.
- Emulator dimuli_api36 is still running as emulator-5554 with the x86_64-only debug build installed; dimuli is in the editor with the 3-image collage and the "Collage gespeichert." banner shown.

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED

ShareProvider.kt and all 5 referenced evidence files in android/build/smoke/ exist; 0 commits since 262306a (as required, no commits); ROADMAP shows 05-04 checked and 4/8; STATE.md has one consistent position (Current Plan 5, Plan 5 of 8).
