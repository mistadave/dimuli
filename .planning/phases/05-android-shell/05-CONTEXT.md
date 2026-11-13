# Phase 5: Android Shell - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning

<domain>
## Phase Boundary

dimuli runs as a real Android app: a Gradle project under `android/` with a Kotlin `MainActivity : NativeActivity` packages `libdimuli.so` (built by `cargo ndk` for `arm64-v8a` and `x86_64`). A JNI bridge implements the existing `Platform` trait / `PlatformEvent` contract on Android: system Photo Picker (up to 9 images, copied into the cache), save to the gallery via `MediaStore.Images` (`Pictures/dimuli`, no permission), share via the system share sheet, edge-to-edge insets, back gesture forwarded to Rust, 16 KB page alignment, and full touch editing on device. Requirements ANDR-01..08.

Not in this phase: lifecycle/state preservation polish, large-image performance, complete error-message pass, real-device v1 acceptance (Phase 6); `CLAUDE.md` build-docs rewrite (Phase 7); more than 9 images (Phase 8).

</domain>

<decisions>
## Implementation Decisions

### App Identity
- **D-01:** `applicationId` (and Kotlin package / `namespace`) is `ch.staeheli.dimuli`. — **Reversibility:** one-way — once the app is published to Google Play, the applicationId is the store identity and cannot change; changing it before that means renaming the Kotlin package, the manifest, the JNI symbol names (`Java_ch_staeheli_dimuli_...`) and the `FileProvider` authority.
- **D-02:** Launcher label is `dimuli` (lower case, like the project name).
- **D-03:** Launcher icon: the owner will supply the final icon later (format not decided yet: SVG layers or PNG). Phase 5 ships an **adaptive icon with a simple vector placeholder** (`mipmap-anydpi` adaptive icon with XML vector foreground/background, e.g. a stylised collage grid). Phase 5 does not wait for the real icon; swapping it in later must only replace the drawable resources.
- **D-04:** Phase 5 builds **debug-signed APKs only** (default debug keystore), installed with `adb install` on emulators and a real device. No release keystore, `signingConfig`, R8/minify or Play Store setup in this phase.
- **D-05:** No orientation lock: the app runs in portrait and landscape. (Android 16+ ignores orientation locks on large screens anyway.) The Slint UI must work in both orientations, and rotating must not lose the collage (see Claude's Discretion on `configChanges`).

### Platform Level (revised 2026-11-13, during plan-phase)
- **D-06:** `minSdk 33` (Android 13), `targetSdk`/`compileSdk 37` (Android 17); Rust build with `cargo ndk --platform 33`. Replaces the earlier minSdk 36. Rely only on APIs available from API 33: system Photo Picker via `MediaStore.ACTION_PICK_IMAGES` (+ `MediaStore.EXTRA_PICK_IMAGES_MAX`), `MediaStore.Images` insert without permission, `OnBackInvokedCallback` with `android:enableOnBackInvokedCallback="true"`. No fallbacks below API 33, no storage permissions. — **Reversibility:** costly — raising or lowering minSdk again touches the manifest/Gradle config, the `cargo ndk --platform` value in docs and agent files, the emulator test matrix, and any API used above the new floor.
- **D-07:** Edge-to-edge on **every** supported version: the app enables it itself on Android 13/14 (where it is not enforced) so the UI looks the same on 13-17, and insets reach Slint the same way on all versions.
- **D-08:** Phase 5 test matrix: emulators API 33, API 36 and API 37. A new AVD `dimuli_api33` (x86_64, Android 13 system image) must be created as part of this phase (SDK image download + `avdmanager`; may need the owner to run it). Real devices: one with **Android 14** (API 34; checks the self-enabled edge-to-edge of D-07 and the API-33-era code paths on hardware) and one with **Android 16**. The v1 acceptance stays on the real Android 16 device (ACPT-01, unchanged).

### Owner Decisions after Research (2026-11-13)
- **D-09:** Dependencies **approved** by the owner (exact versions from `05-RESEARCH.md`): Rust `jni 0.22.4`, `android_logger 0.15.1`, Slint feature `backend-android-activity-06` (Android target only); Android build: AGP 9.5.1 with built-in Kotlin (implicit `kotlin-stdlib 2.2.10`), Gradle wrapper 9.9.1. **No AndroidX**: sharing uses an own minimal `ContentProvider`. No further dependency without asking again. Plans need no separate approval checkpoint for these.
- **D-10:** Insets use **Slint 1.18's built-in mechanism** (its Java helper feeds `Window.safe-area-insets`); no dimuli Kotlin inset code. Kotlin only enables edge-to-edge on Android 13/14 (D-07). ANDR-05 is reworded accordingly; verification checks the behaviour (no content under system bars on API 33/34/36/37, portrait + landscape), not the mechanism.
- **D-11:** Sharing shows **no message on success** (the chooser is the feedback; Android cannot report completion). Only failures show a German error message (new text, e.g. `SHARE_FAILED`).
- **D-12:** HEIC/WebP picks stay unsupported in Phase 5 (skipped with the existing "unreadable" notice); solving it is deferred to Phase 6 (PERF-01/ERRM-01).

- **D-13:** (owner, after plan check 2026-11-13) Remove the unused own inset path (`PlatformEvent::InsetsChanged`, `AppState.insets`/`Insets`/`set_insets`, `view::Snapshot.insets`, `Backend.inset-*`); Slint's `Window.safe-area-insets` is the single source. Keep the canvas-sanitizing test (renamed). Update `docs/ARCHITECTURE.md` accordingly.

### Claude's Discretion
The owner chose not to discuss these areas; research and planning decide them within the constraints in PROJECT.md and `docs/CODING_GUIDELINES.md`:
- **Dependencies (superseded by D-09):** `NativeActivity` is not an AndroidX `ComponentActivity`, so the AndroidX `PickMultipleVisualMedia` contract named in `docs/PLAN.md` does not apply directly. Prefer platform APIs where they suffice (`MediaStore.ACTION_PICK_IMAGES` + `onActivityResult`, `OnBackInvokedCallback`, a minimal own `ContentProvider` vs AndroidX `FileProvider`). Any Rust crate (`jni`, `android_logger`, ...) or Gradle/AndroidX/Kotlin library that is still needed **must be presented to the owner for approval before use** (guideline rule, listed as a blocker in STATE.md) — the planner should make that an explicit checkpoint, not assume approval.
- **Save & share UX:** gallery file name (e.g. `dimuli-<timestamp>.jpg`, matching the desktop pattern), success message shown in the existing Slint message UI (not an Android Toast unless research shows a reason), when picked-image cache copies and the share file are deleted.
- **Rotation / configuration changes:** Since orientation is not locked (D-05), avoid Activity recreation losing state, e.g. via `android:configChanges` on the activity so `NativeActivity` keeps the native app alive; full lifecycle robustness is LIFE-01 in Phase 6.
- **Insets (superseded by D-10):** Check whether Slint 1.18's Android backend already provides safe-area insets itself before building a Kotlin -> JNI path; the existing `PlatformEvent::InsetsChanged` and `Backend.inset-*` properties are the target either way.
- **Workspace lint exception:** `unsafe_code = "deny"` needs a scoped exception limited to `platform/android.rs`, with `// SAFETY:` comments; JNI entry points catch panics.
- **Test scope:** Phase 5 is verified on emulators `dimuli_api33`, `dimuli_api36` and `dimuli_api37` (see D-08) plus the real Android 14 and Android 16 devices (ROADMAP success criterion 1 names them; the v1 acceptance on a real device is ACPT-01 in Phase 6).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project scope and requirements
- `.planning/ROADMAP.md` §Phase 5 — goal and success criteria
- `.planning/REQUIREMENTS.md` §Android Platform — ANDR-01..08
- `.planning/PROJECT.md` §Constraints — build flow, SDK levels, ABIs, edge-to-edge, 16 KB, dependency rules, unsafe policy
- `.planning/STATE.md` §Blockers/Concerns — unsafe lint conflict, dependency approvals, real-device availability

### Design docs (German)
- `docs/PLAN.md` §Android-Integration, §Android-Versionen — Kotlin activity, picker -> cache copy -> `onImagesPicked(paths)`, MediaStore, FileProvider + `ACTION_SEND`, `cargo ndk` -> `jniLibs`, insets, 16 KB, predictive back
- `docs/ARCHITECTURE.md` — module layout (`platform/android.rs`, `android_main` calls `run(AndroidPlatform)`), data flow, `Platform` trait, limits
- `docs/CODING_GUIDELINES.md` — binding rules incl. no unwrap/expect/panic, unsafe only in `platform/android.rs`, dependency approval, completion checklist
- `CLAUDE.md` — agent workflow (code by `rust-coder`, review by `code-reviewer`, no commits without request); its `cargo apk` commands are stale (Phase 7)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/dimuli-app/src/platform/mod.rs`: `Platform` trait (`pick_images(max)`, `save_to_gallery(jpeg)`, `share(jpeg)`, `close_app()`) and `PlatformEvent` (`ImagesPicked`, `PickFailed`, `Saved`, `Shared`, `InsetsChanged`, `BackPressed`) — the Android implementation plugs in here; `post_event()` delivers events to the controller on the UI thread from any thread (use it from JNI callbacks).
- `crates/dimuli-app/src/platform/desktop.rs`: reference implementation of the trait.
- `crates/dimuli-app/ui/backend.slint` (`inset-top/bottom/left/right`) and `ui/app.slint` (content inset by the safe area) — insets UI already wired.
- `crates/dimuli-app/src/texts.rs`: German user messages (e.g. `SHARE_UNAVAILABLE`) — add Android save/share/pick messages here.

### Established Patterns
- `lib.rs` exposes `run(platform: Rc<dyn Platform>)`; the crate is already `cdylib` + `rlib`, so `android_main` lives in the lib and calls `run(AndroidPlatform)`.
- Workspace lints deny `unsafe_code`, `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`; the generated Slint module is the only `#[allow]` exception so far.
- `env_logger` only on non-Android targets; Android logging needs a logger (tag `dimuli`, per `adb logcat -s dimuli`).
- Background work in short-lived threads, results posted back to the UI thread.

### Integration Points
- `crates/dimuli-app/src/lib.rs`: add `#[cfg(target_os = "android")] android_main` and the `platform::android` module.
- `crates/dimuli-app/Cargo.toml`: enable Slint `backend-android-activity` (feature `native-activity`) for the Android target; Android-only deps under `[target.'cfg(target_os = "android")'.dependencies]` (after approval).
- New `android/` Gradle project: `MainActivity : NativeActivity` in package `ch.staeheli.dimuli`, manifest (`android.app.lib_name` = `dimuli`), adaptive placeholder icon, provider config for sharing.
- Uncommitted desktop-only change in `main.rs` (winit + software renderer for WSLg) is `cfg(not(android))` and does not affect the Android path.

</code_context>

<specifics>
## Specific Ideas

- Placeholder icon idea: a stylised collage grid (e.g. 2x2 or one large + two small cells) as an adaptive vector icon, so the later real icon only replaces the drawables.

</specifics>

<deferred>
## Deferred Ideas

- Release signing, R8/minify and Google Play setup — later, not part of v1 Phase 5.
- Final launcher icon from the owner — drop-in replacement once supplied (format still open).

</deferred>

---

*Phase: 05-android-shell*
*Context gathered: 2026-11-13*
