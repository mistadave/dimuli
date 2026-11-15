---
phase: 06-polish-and-v1-acceptance
plan: 04
status: complete
requirements: [PERF-01, ERRM-01]
completed: 2026-11-14
actuals:
  tasks: 3
  commits: 0
---

# Phase 6 Plan 04: Android image import

Android now converts picked formats other than JPEG and PNG using ImageDecoder, downscales images exceeding the Rust decoder limits, and reports failed copies to Rust through JNI.

## Changes

- `MainActivity.kt`: software ImageDecoder with sRGB conversion, PNG for transparency and JPEG quality 95 otherwise; oversized images are scaled to at most 64 million pixels and 16,384 pixels per edge. Copy and conversion failures, including out-of-memory errors, are counted per image.
- `platform/android.rs`, `platform/native_events.rs`, `platform/mod.rs`, and `platform/desktop.rs`: propagate the failed-copy count; the JNI signature is `([Ljava/lang/String;I)V`.
- `controller.rs`, `state.rs`, and `view.rs`: include failed copies in the unreadable-image notice and adapt callers and tests.
- No Cargo or Gradle dependencies changed. No commits were created, as instructed.

## Verification

- HEIC tracer on API 36: opened in the editor; log recorded `transcoded image 0` and `picked images: 1`; cache copy begins `ff d8 ff`. The user confirmed the screenshot.
- An 80 MP JPEG was scaled to 8944 × 7155 pixels and loaded on the emulator.
- A readable JPEG and damaged HEIC produced `picked images: 1, failed copies: 1` and the expected notice. The original HEIC was only 2344 bytes, so the planned 4096-byte truncation did not damage it and zeroing from byte 2048 remained readable; truncating to 1024 bytes produced the intended failure.
- Targeted tests passed: 11 native-events, 1 state and 5 desktop tests. Workspace tests passed: 90 core and 131 app tests. Formatting, host Clippy, Android Clippy and the x86_64 APK build passed.
- Evidence is under `android/build/smoke/p6-04-*` and remains untracked via the existing ignore rules.

## Follow-up

PERF-01 and ERRM-01 remain open for their other phase plans and final acceptance. Plan 06-05 follows. The working tree remains uncommitted by instruction.
