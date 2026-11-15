---
phase: 06-polish-and-v1-acceptance
plan: 05
status: complete
requirements: [LIFE-01, ERRM-01]
completed: 2026-11-14
actuals:
  tasks: 2
  commits: 0
---

# Phase 6 Plan 05: Lifecycle and restart notice

The app detects a lost open collage after an Android process restart, preserves the running activity through additional configuration changes, and addresses Phase 5 review notes N1–N5.

## Changes

- `MainActivity.kt`: stores an open-collage boolean in saved instance state, reads it before starting NativeActivity, exposes JNI methods `collageLostOnRestart()Z` and `setCollageOpen(Z)V`, cleans the share cache on startup, and updates the deprecation suppression.
- `platform/android.rs`, `platform/mod.rs`, `platform/desktop.rs`, `controller.rs`: propagate the restart flag and push changed collage-open state to Android.
- `state.rs`, `texts.rs`: persistent restart error and tests `restart_notice_is_a_persistent_error_on_the_start_screen` and `restart_notice_is_not_hidden_by_the_success_timer`.
- `AndroidManifest.xml`: handles font weight, color mode, grammatical gender, touchscreen, MCC and MNC changes; `ShareProvider.kt` reports missing files; `smoke.sh` avoids pipefail/SIGPIPE problems and waits for the first frame.
- No commits were created, as instructed.

## Verification

- Formatting, host and Android Clippy, 90 core and 133 app tests, and x86_64 APK build passed.
- Emulator restart with an open collage displayed the persistent notice; restart from the start screen did not. User confirmed the screenshot.
- Smoke test passed. Home, recents, share chooser, rotation and font-weight changes preserved the modified collage; PID stayed 5076, log recorded one `dimuli started` and no RustPanic.
- Evidence is under `android/build/smoke/p6-05-*`.

## Observation

The additional picker-process-kill experiment returned a selected image, but `am kill` did not terminate the process: its PID remained 5076. Thus the behavior after an actual process loss during the picker remains unverified; this observation was not an acceptance gate. The emulator restarted after disappearing during the smoke run; the repeated run passed.

## Follow-up

LIFE-01 and ERRM-01 remain open for remaining phase verification and real-device acceptance. Plan 06-06 follows. Changes remain uncommitted.
