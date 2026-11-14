---
phase: 05-android-shell
plan: 01
subsystem: platform
tags: [rust, android, jni, slint, cargo-ndk, android_logger, safe-area-insets]

# Dependency graph
requires:
  - phase: 04-export
    provides: Platform trait with PlatformEvent, controller back path (BackOutcome::NotHandled -> close_app), desktop export/save
provides:
  - "libdimuli.so for x86_64-linux-android via cargo ndk --platform 33 ([lib] name = \"dimuli\", Slint backend-android-activity-06 for Android only)"
  - "crates/dimuli-app/src/platform/android.rs: android_main, AndroidPlatform (with_activity / register_natives / call_void), NATIVES with native_back_pressed"
  - "Kotlin contract expected by Rust: installBackCallback(), nativeBackPressed(), finish() on the activity class"
  - "texts::PICK_FAILED, texts::SHARE_FAILED (Android only)"
  - "Window.safe-area-insets as the single inset source; own inset path (InsetsChanged, Insets, set_insets, Backend.inset-*) removed"
  - "Second documented lint exception (platform/android.rs) in docs/CODING_GUIDELINES.md; clippy::undocumented_unsafe_blocks = deny"
affects: [05-02, 05-03, 05-04, 05-08]

# Actuals (#2632) - chars/4 over the realized diff (Cargo.lock and .planning excluded)
actuals:
  tokens: 5300
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: ["jni 0.22.4", "android_logger 0.15.1 (default-features = false)", "slint feature backend-android-activity-06 (Android target only)"]
  patterns:
    - "Activity reference taken per call from AndroidApp::activity_as_ptr() with a borrowing cast; AndroidApp never stored in a static"
    - "JNI natives declared only via jni::native_method! with error_policy = LogErrorAndDefault; natives only post PlatformEvents"
    - "Android-only items gated with #[cfg(target_os = \"android\")] instead of #[allow(dead_code)]"

key-files:
  created:
    - crates/dimuli-app/src/platform/android.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/dimuli-app/Cargo.toml
    - crates/dimuli-app/src/main.rs
    - crates/dimuli-app/src/platform/mod.rs
    - crates/dimuli-app/src/texts.rs
    - crates/dimuli-app/src/state.rs
    - crates/dimuli-app/src/view.rs
    - crates/dimuli-app/src/controller.rs
    - crates/dimuli-app/ui/app.slint
    - crates/dimuli-app/ui/backend.slint
    - docs/CODING_GUIDELINES.md
    - docs/ARCHITECTURE.md

key-decisions:
  - "texts::PICK_FAILED and texts::SHARE_FAILED are #[cfg(target_os = \"android\")] so the host build has no dead code without needing an #[allow] (guidelines allow only two #[allow] exceptions)"
  - "Android clippy runs with -A clippy::missing_const_for_thread_local as prescribed by the plan: false positive on the already-const thread_local! in controller.rs:22 (Pitfall 4), not in android.rs"

patterns-established:
  - "android_main order: logger -> 'dimuli started' -> slint::android::init -> register_natives -> installBackCallback (after Slint, last callback wins) -> crate::run"

requirements-completed: []  # Plan declares [ANDR-01, ANDR-05, ANDR-06]; all three are shared with later Phase-5 plans and are proven end-to-end only in 05-02 ff.

coverage:
  - id: D1
    description: "Workspace builds libdimuli.so for x86_64-linux-android with only the D-09 crates; android_main exported"
    requirement: ANDR-01
    verification:
      - kind: other
        ref: "ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 build -p dimuli-app --lib --release && test -s target/x86_64-linux-android/release/libdimuli.so"
        status: pass
      - kind: other
        ref: "nm -D target/x86_64-linux-android/release/libdimuli.so | grep android_main"
        status: pass
    human_judgment: false
  - id: D2
    description: "android.rs lints clean on the Android target (unsafe only there, every unsafe block with SAFETY comment)"
    requirement: ANDR-06
    verification:
      - kind: other
        ref: "ANDROID_JAR=... cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local"
        status: pass
    human_judgment: false
  - id: D3
    description: "Own inset path removed; Window.safe-area-insets is the single inset source; canvas test kept as invalid_canvas_sizes_are_sanitized"
    requirement: ANDR-05
    verification:
      - kind: unit
        ref: "crates/dimuli-app/src/state.rs#invalid_canvas_sizes_are_sanitized"
        status: pass
      - kind: other
        ref: "grep -rE 'InsetsChanged|set_insets|struct Insets' crates/dimuli-app/src | wc -l == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "Desktop layout unchanged after the safe-area switch (desktop insets are 0)"
    verification:
      - kind: other
        ref: "timeout 15 target/debug/dimuli android/build/smoke/pick-{a,b,c}.jpg -> exit 124"
        status: pass
    human_judgment: true
    rationale: "The smoke run only proves the binary does not crash; layout identity (no offset, no margin) is visual"
  - id: D5
    description: "Back round trip Kotlin -> native_back_pressed -> controller -> finish() and Android safe-area insets on a device"
    requirement: ANDR-06
    verification: []
    human_judgment: true
    rationale: "Kotlin side does not exist yet; proven on dimuli_api36 in 05-02"

# Metrics
duration: 5min (verification pass only; implementation by rust-coder not timed)
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 01: Rust Preparation for the Android Shell Summary

**libdimuli.so builds for x86_64-linux-android with jni 0.22.4 / android_logger 0.15.1 / Slint backend-android-activity-06; platform/android.rs provides android_main, the JNI back native and AndroidPlatform (close_app -> Activity.finish()), and Slint's Window.safe-area-insets replaces the producer-less own inset path.**

## Performance

- **Duration:** about 5 min for this verification pass (16:12-16:17 UTC); implementation by rust-coder happened before and was not timed here
- **Started:** 2026-11-13T16:12:32Z
- **Completed:** 2026-11-13T16:17Z
- **Tasks:** 2 of 2
- **Files modified:** 13 modified + 1 created (plus `.planning/config.json` with `use_worktrees: false`, which is GSD config, not part of this plan)
- **First Android release build:** not measured here; rust-coder ran the first (uncached) build. The re-run in this pass was cached and took 6 s (`Finished release in 4.86s`).

## Accomplishments

- `[lib] name = "dimuli"`; desktop binary switched to `dimuli::`; `cargo ndk` emits `target/x86_64-linux-android/release/libdimuli.so` (20 MB, exports `android_main` and `ANativeActivity_onCreate`)
- `crates/dimuli-app/src/platform/android.rs`: `android_main` (logger with tag `dimuli`, logs `dimuli started`, `slint::android::init`, `register_natives`, `installBackCallback` strictly after Slint init, `crate::run`), `AndroidPlatform { app }` with `with_activity` / `register_natives` / `call_void`, `NATIVES` with `native_back_pressed` (only posts `PlatformEvent::BackPressed`), `close_app` -> `finish`; pick/save/share post German failure events until 05-03/05-04
- Scoped JNI lint exception (`#![allow(unsafe_code)]` in android.rs, `#[allow(deprecated)]` on `NATIVES` only) documented in docs/CODING_GUIDELINES.md §3; `clippy::undocumented_unsafe_blocks = "deny"` added to the workspace
- D-13: `PlatformEvent::InsetsChanged`, `state::Insets`, `AppState.insets`, `set_insets`, `Snapshot.insets`, `Backend.inset-*` removed; `ui/app.slint` uses `root.safe-area-insets` (with `max(0px, …)` clamps); docs/ARCHITECTURE.md names `Window.safe-area-insets` as the single source and shows "Edge-to-Edge" in the overview box

## Task Commits

No commits: CLAUDE.md and the plan forbid commits without an explicit owner request. All changes remain in the working tree (HEAD unchanged at `262306a`).

Suggested commit message when the owner asks to commit:

```
Prepare Rust side of the Android shell (phase 5, plan 01)

- Build libdimuli.so via cargo ndk: [lib] name = "dimuli", Slint
  backend-android-activity-06 for Android only, jni 0.22.4 and
  android_logger 0.15.1 (owner-approved, D-09)
- Add platform/android.rs: android_main, AndroidPlatform over JNI,
  native_back_pressed -> PlatformEvent::BackPressed, close_app -> finish()
- Scope the JNI lint exception to platform/android.rs and deny
  undocumented unsafe blocks; document it in CODING_GUIDELINES.md
- Use Slint's Window.safe-area-insets as the only inset source and
  remove the unused InsetsChanged / Insets / Backend.inset-* path (D-13)
- Add PICK_FAILED and SHARE_FAILED texts (Android only)
```

## Files Created/Modified

- `crates/dimuli-app/src/platform/android.rs` (new) - Android entry point, JNI bridge, `AndroidPlatform`
- `Cargo.toml` - workspace deps `jni`, `android_logger`; clippy lint `undocumented_unsafe_blocks = "deny"`
- `Cargo.lock` - jni, android_logger, i-slint-backend-android-activity and their transitive deps (android-activity, ndk, ndk-sys, android-build, android_log-sys, cesu8, env_filter, windows-* target shims); `cargo tree --target x86_64-linux-android --depth 1` shows only the approved direct deps
- `crates/dimuli-app/Cargo.toml` - `[lib] name = "dimuli"`; Android-only dependency section
- `crates/dimuli-app/src/main.rs` - `dimuli_app::` -> `dimuli::` (`select_desktop_backend` untouched)
- `crates/dimuli-app/src/platform/mod.rs` - `#[cfg(target_os = "android")] mod android;`, `InsetsChanged` removed, module doc without insets
- `crates/dimuli-app/src/texts.rs` - `PICK_FAILED`, `SHARE_FAILED` (Android only)
- `crates/dimuli-app/src/state.rs` - `Insets`, `insets`, `set_insets` removed; test renamed to `invalid_canvas_sizes_are_sanitized`
- `crates/dimuli-app/src/view.rs` - `Snapshot.insets` and the four `set_inset_*` calls removed
- `crates/dimuli-app/src/controller.rs` - `InsetsChanged` arm and `Insets` import removed
- `crates/dimuli-app/ui/app.slint` - content Rectangle inset by `root.safe-area-insets`
- `crates/dimuli-app/ui/backend.slint` - `inset-*` properties removed
- `docs/CODING_GUIDELINES.md` - §3 second `#[allow]` exception
- `docs/ARCHITECTURE.md` - overview "Edge-to-Edge", no `insets` field / variant, safe-area sentence
- `android/build/smoke/pick-{a,b,c}.jpg` - 640x480 test images (gitignored via `/android/build/`)

## Gate Results (re-run in this pass)

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0, no warnings) |
| `cargo test --workspace` | PASS (86 + 92 passed, 0 failed) |
| Android clippy x86_64 (`-D warnings -A clippy::missing_const_for_thread_local`) | PASS (exit 0). Without `-A`: one error, `controller.rs:22` (pre-existing `const` thread_local, false positive); nothing in android.rs |
| Android release build + `test -s libdimuli.so` | PASS; `nm -D` shows `android_main` |
| `cargo test -p dimuli-app --lib invalid_canvas_sizes_are_sanitized` | PASS (1 passed) |
| ARCHITECTURE.md D-13 gate | PASS (exit 0) |
| Desktop smoke (3 images asserted with `test -s`, `timeout 15`) | PASS (exit 124) |

Acceptance grep counts, Task 1: `name = "dimuli"` 2 (>=2); `backend-android-activity-06` 1; `jni = "0.22.4"` 1; `android_logger = {…}` 1; `undocumented_unsafe_blocks` 1; `dimuli_app::` in main.rs 0; `mod android;` 1; `PICK_FAILED: &str` 1, `SHARE_FAILED: &str` 1; `#![allow(unsafe_code)]` 1; `// SAFETY:` 3 (>=2); `dimuli started` 1; entry/back/finish tokens 4; files with `unsafe` outside android.rs 0; `allow(unsafe_code)` in guidelines 1. All PASS.

Acceptance grep counts, Task 2: `safe-area-insets` in app.slint 4 (>=4); `Backend.inset-` 0; `inset-` in backend.slint 0; `InsetsChanged|set_insets|struct Insets` in src 0; ARCHITECTURE `InsetsChanged` 0, `canvas, insets` 0, `safe-area-insets` 1, `Edge-to-Edge` 1. All PASS.

Prohibitions: direct deps on Android are exactly `android_logger 0.15.1`, `jni 0.22.4`, `slint 1.18.1` (+ existing `collage-core`, `image`, `log`); no `unwrap`/`expect`/`panic!` in changed production code (only in pre-existing `#[cfg(test)]` code of desktop.rs); `unsafe` only in android.rs, each block with `// SAFETY:`.

## Decisions Made

- The `-A clippy::missing_const_for_thread_local` flag is part of the plan's own verify command (05-RESEARCH Pitfall 4, 05-VALIDATION), so it is not a deviation. Confirmed in this pass that it suppresses only the false positive at `controller.rs:22`, whose `thread_local!` initializer is already `const`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `PICK_FAILED` / `SHARE_FAILED` gated with `#[cfg(target_os = "android")]`**
- **Found during:** Task 1 (A5)
- **Issue:** The plan adds both constants unconditionally, but on the host nothing uses them, so `-D warnings` fails with dead-code errors. docs/CODING_GUIDELINES.md §3 allows only two `#[allow]` exceptions, so `#[allow(dead_code)]` would break the rules.
- **Fix:** Both constants carry `#[cfg(target_os = "android")]` plus a comment. Text and type match the plan exactly; the acceptance greps (`PICK_FAILED: &str`, `SHARE_FAILED: &str` = 1) pass. If a later plan needs either text on the desktop, remove the `cfg` then.
- **Files modified:** crates/dimuli-app/src/texts.rs
- **Verification:** host clippy and Android clippy clean
- **Committed in:** not committed (owner rule)

---

**Total deviations:** 1 auto-fixed (1 blocking). **Impact on plan:** None on behaviour; required by the coding guidelines. No scope creep.

## Issues Encountered

- Minor German grammar in docs/CODING_GUIDELINES.md §3, point 2: "Alles andere handgeschriebene Code" should read "Der übrige handgeschriebene Code". Not changed, because no criterion requires it (role constraint). It can be fixed together with 05-08's doc work or before the commit.
- The guidelines line "Native JNI-Einstiegspunkte (`extern "system" fn Java_...`) fangen Panics mit `std::panic::catch_unwind` ab" describes the old pattern. android.rs uses `native_method!` with `LogErrorAndDefault`, which wraps `catch_unwind` itself (threat T-05-01). Consistent in substance; 05-08 may want to update the wording.

## Open Human Checks

- Desktop layout identity (plan Task 2 `<human-check>`): run `cargo run -p dimuli-app -- android/build/smoke/pick-a.jpg android/build/smoke/pick-b.jpg` once. Expected: start screen and editor fill the window exactly as before (no offset, no margin). The automated run only proves that the app does not crash (exit 124 after 15 s).
- Android launch, back round trip, safe-area insets and rotation: deferred to 05-02 Task 2 on dimuli_api36 (requires the Kotlin side).
- Code review (`code-reviewer` subagent, CLAUDE.md): not run in this plan; due before committing the new functionality.

## User Setup Required

None. No external service configuration required.

## Next Phase Readiness

- Ready for 05-02 (Gradle + Kotlin tracer). The Kotlin activity must provide `installBackCallback()`, `native fun nativeBackPressed()` and the inherited `finish()`; natives are registered on the activity's runtime class, so no package name is hard-coded in Rust.
- The x86_64 release artefacts are cached for the Gradle `buildRust` task.

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED
