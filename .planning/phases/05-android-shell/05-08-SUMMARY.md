---
phase: 05-android-shell
plan: 08
subsystem: android
tags: [review, docs, milestone-close, kotlin-hardening]

requires:
  - phase: 05-android-shell
    plan: 07
    provides: "Android 14 device smoke + insets (partial; Android 16 and touch checklist deferred)"
provides:
  - "05-REVIEW.md: code-reviewer report over all Phase 5 changes (0 critical, 4 warnings, 7 notes)"
  - "Review warnings 1-4 fixed (MainActivity.kt error paths, controller.rs doc comment)"
  - "docs/PLAN.md, docs/ARCHITECTURE.md, docs/CODING_GUIDELINES.md describe the Android shell as built"
affects: [phase-6, phase-7]

actuals:
  tokens: 4000
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

key-files:
  created:
    - .planning/phases/05-android-shell/05-REVIEW.md
  modified:
    - android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt
    - crates/dimuli-app/src/controller.rs
    - docs/PLAN.md
    - docs/ARCHITECTURE.md
    - docs/CODING_GUIDELINES.md
    - .planning/phases/05-android-shell/05-VALIDATION.md

key-decisions:
  - "05-08: Milestone 5 NOT ticked in docs/PLAN.md: precondition (owner approval of 05-07 Task 3) is open because the owner deferred the device checks to a final device test"
  - "05-08: Review warnings 1-4 fixed although not critical (reviewer: fix before commit; small, contained to MainActivity.kt + one doc comment)"

requirements-completed: []  # ANDR-08 and the Android 16 device remain open (final device test)

coverage:
  - id: D1
    description: "Code review saved, no critical finding open"
    verification:
      - kind: other
        ref: ".planning/phases/05-android-shell/05-REVIEW.md (0 critical)"
        status: pass
  - id: D2
    description: "Gates after fixes"
    verification:
      - kind: build
        ref: "fmt, clippy, cargo test (86 + 104 passed), cargo ndk clippy x86_64, assembleDebug, check-apk.sh APK CHECK OK, smoke.sh emulator-5554 SMOKE OK, smoke.sh 8200fcc9 SMOKE OK"
        status: pass
  - id: D3
    description: "Doc gates of Task 2 (except milestone tick)"
    verification:
      - kind: other
        ref: "PLAN.md: PickMultipleVisualMedia/WindowInsets 0, ShareProvider >=1, ANDROID_JAR >=1, FileProvider 0, 'Kotlin liest' 0; ARCHITECTURE.md: safe-area-insets >=1, InsetsChanged 0, native_events >=1, ShareProvider >=1; CODING_GUIDELINES.md: GlobalRef 0, native_method! >=1, ShareProvider >=1"
        status: pass
  - id: D4
    description: "Milestone 5 ticked"
    verification:
      - kind: other
        ref: "grep -c '^- \\[x\\] 5\\.' docs/PLAN.md -> 0 (precondition open)"
        status: deferred

duration: 30min
completed: 2026-11-14
status: partial
---

# Phase 5 Plan 08: Milestone Close Summary

**code-reviewer found no critical issue in Phase 5; the four warnings (Kotlin error paths and a stale doc comment) are fixed, all gates are green including smoke on the API 36 emulator and the Android 14 device, and the three design docs now describe the Android shell as built. Milestone 5 stays unticked until the owner's final device test (05-07 Task 3, Android 16).**

## Review (Task 1)

Report: `05-REVIEW.md`. Critical: none.

| # | Warning | Disposition |
|---|---------|-------------|
| W1 | `pickImages`/`share` caught only `ActivityNotFoundException`; other `RuntimeException`s from `startActivity*` crash on the UI thread | Fixed: catch `RuntimeException`, report `nativePickFailed()` / `nativeShared(false)` |
| W2 | `nativeSaved(true)` inside the write `try`; a later exception deleted the finished row | Fixed: `val ok = try { …; true } catch { …; false }`, `nativeSaved(ok)` after the try |
| W3 | Stale doc comment `controller.rs` `start_export` (D-11) | Fixed |
| W4 | `io.execute` after `io.shutdown()` throws `RejectedExecutionException` | Fixed: helper `runOnIo(onRejected, task)` used by `saveToGallery`, `share`, `onActivityResult`, reports failure via the matching native |

| # | Note | Disposition |
|---|------|-------------|
| N1 | `@Deprecated` on `onActivityResult` override unnecessary | Open (cosmetic) |
| N2 | smoke.sh start screenshot before Slint's first frame | Open (Phase 6/7 tooling) |
| N3 | smoke.sh `adb install \| grep -q` under pipefail (SIGPIPE) | Open (Phase 6/7 tooling) |
| N4 | Share file stays in `cacheDir/share` | Open (documented decision; optional cleanup at start) |
| N5 | `ShareProvider.query` length 0 for missing file | Open (optional) |
| N6 | Kotlin logic covered only by emulator/device runs | Open (note for docs/Phase 6) |
| N7 | Docs: Slint safe-area model | Done in Task 2 |

Behaviour change vs. 05-07: only failure paths in Kotlin (success paths unchanged); the Android 14 device was re-smoked with the new APK (SMOKE OK). No further device re-check needed.

## Gates after fixes

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (86 + 104) |
| `cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib …` (with ANDROID_JAR) | PASS |
| `android/gradlew -p android assembleDebug` | PASS (no Kotlin warnings) |
| `android/scripts/check-apk.sh` | `APK CHECK OK` |
| `android/scripts/smoke.sh -s emulator-5554` | `SMOKE OK emulator-5554` |
| `android/scripts/smoke.sh -s 8200fcc9` (Android 14 device) | `SMOKE OK 8200fcc9` |

## Docs (Task 2)

- **docs/PLAN.md:** structure tree (`native_events.rs`, ShareProvider, scripts); Android-Integration (`backend-android-activity-06`, `ACTION_PICK_IMAGES` + `onActivityResult`, `cacheDir/picked/img-<n>`, RegisterNatives, `IS_PENDING`, ShareProvider + chooser, silent share, `buildRust` with `ANDROID_JAR`, `-PrustAbis`, check/smoke scripts); Android-Versionen (insets from Slint `Window.safe-area-insets`, Kotlin only enables edge-to-edge on 13/14, back callback after `slint::android::init`, HEIC/WebP until milestone 6). **Milestone 5 not ticked** (see key-decisions).
- **docs/ARCHITECTURE.md:** `lib.rs` row without `android_main`; rows for `platform/desktop.rs`, `platform/android.rs`, `platform/native_events.rs`; Android paragraph (picker, MediaStore, share, `close_app`). Inset lines from 05-01 (D-13) still match the code.
- **docs/CODING_GUIDELINES.md:** §2 platform row; §3 `native_method!` + RegisterNatives, natives only `post_event`, `jni::refs::Global`, no `AndroidApp`/Activity in statics; §4 `Window.safe-area-insets`; §5 Kotlin scope incl. edge-to-edge 13/14 and `ShareProvider.kt`.
- **05-VALIDATION.md:** status column 05-01..05-06 green, 05-07-T2 partial, 05-07-T3 deferred.

## Stale for Phase 7 (ENV-03)

- `CLAUDE.md` still documents `cargo apk` commands (and its build section predates the Gradle flow).
- `.claude/agents/rust-coder.md`: Android check lacks `ANDROID_JAR`.
- README.md has the current device/build commands; CLAUDE.md should point to it or adopt them.

## Open

- Final device test (owner): Android 16 smoke + screenshots, Task 3 checklist on both devices, then tick milestone 5.
- Emulator `dimuli_api36` (emulator-5554) left running.

## Task Commits

None (no commit without explicit request). HEAD unchanged at `262306a`.

---
*Phase: 05-android-shell*
*Completed (partial): 2026-11-14*
