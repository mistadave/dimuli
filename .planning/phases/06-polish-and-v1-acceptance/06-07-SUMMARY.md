---
phase: 06-polish-and-v1-acceptance
plan: 07
status: complete
requirements: [PERF-01, ERRM-01, LIFE-01, POLI-01, POLI-02]
completed: 2026-11-15
actuals:
  tasks: 2
  commits: 0
---

# Phase 6 Plan 07: Code review and docs

The code-reviewer checked all Phase 6 changes (`git diff ff11541 -- crates android`; no untracked files there). Result: approved with conditions, **0 critical findings**, 2 warnings, 6 notes. Report: `06-REVIEW.md`. No commit made.

## Findings and dispositions

| Finding | Disposition |
|---|---|
| Critical | none |
| W1 Throwables in `Log.e` can leak the content:// URI (`MainActivity.kt`) | Fixed by rust-coder: all seven `Log.e(…, e)` calls in MainActivity.kt log only `e.javaClass.simpleName`; `Log.w` in `runOnIo`/`native` unchanged (no user data). ShareProvider.kt has no log calls. |
| W2 Platform/Slint calls while `state.borrow_mut()` in `Controller::new` (`controller.rs`) | Fixed by rust-coder: values read into locals first, then a short borrow. Behaviour unchanged. |
| N1 OOM detection by message text (`transcode`) | Open for owner; harmless fallback (image counted as failed) |
| N2 `set_collage_open` not retried after a JNI failure | Open for owner; very unlikely |
| N3 `share/` cache cleared on activity start | Accepted; rare edge case, documented |
| N4 16-bit sources under 64 MP can be "too large" | Intended; ARCHITECTURE.md notes that `DECODE_MAX_ALLOC` also limits 16-bit sources |
| N5 limit tests only assert "not ImageTooLarge" | Open for owner (test strength only) |
| N6 fixed `sleep 2` in smoke.sh | Open for owner |

Both fixes only change log text and borrow order; no emulator-verified behaviour from 06-02..06-06 changed, so only the standard smoke was re-run.

## Gates (after fixes)

- `cargo fmt --all --check`, `cargo clippy --all-targets -D warnings`: clean
- `cargo test --workspace`: 90 + 133 passed
- Android clippy (`cargo ndk -t x86_64 --platform 33`): clean
- `android/gradlew -p android assembleDebug` (both ABIs) + `check-apk.sh`: `APK CHECK OK`
- `smoke.sh -s emulator-5554` (dimuli_api36): `SMOKE OK emulator-5554` (`android/build/smoke/p6-07-smoke-5554.txt`)
- Device 8200fcc9 not attached: no device smoke

## Doc changes

- `docs/ARCHITECTURE.md`: overview box (HEIC/WebP conversion), module rows for state/images/controller/view, Platform trait with `share_supported`, `restored_after_close`, `set_collage_open`, `ImagesPicked { paths, failed }`, desktop share early-out, Android transcoding/downscale/retry, `configChanges`, restart notice, N6 note on Kotlin test coverage, limits table (`DECODE_MAX_PIXELS` 64 MP, `DECODE_MAX_EDGE` 16 384 px, `DECODE_MAX_ALLOC` 256 MiB, `MESSAGE_HIDE_AFTER` 3 s) and the "zu groß" note.
- `docs/PLAN.md`: picker bullet with transcoding and `nativeImagesPicked(paths, failed)`, HEIC/WebP/AVIF support, lifecycle bullet, milestone-6 decision (64 MP + 256 MiB), extended emulator verification. Milestones 5 and 6 stay unticked (06-08, D-14).
- `docs/CODING_GUIDELINES.md`: §5 Kotlin scope (ImageDecoder conversion, restart flag), §7 Kotlin covered only by emulator/device tests.

## Left for Phase 7

- ENV-03: `CLAUDE.md` still documents `cargo apk` instead of `cargo ndk` + Gradle (unchanged here by design).
