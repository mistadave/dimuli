---
phase: 06-polish-and-v1-acceptance
plan: 02
subsystem: app messages (state / view / controller / banner) and loader/export result handling
tags: [messages, success-banner, slint-timer, texts, late-results, lifecycle]

# Dependency graph
requires:
  - phase: 06-polish-and-v1-acceptance
    plan: 01
    provides: "Every decode limit violation is CoreError::ImageTooLarge (D-18), so the app can count 'too large' separately"
provides:
  - "state.rs: MessageKind::Success, Message.seq, AppState.message_seq, private show() helper, show_success, expire_message(seq), success_seq(); save_finished(Ok) -> Success"
  - "state.rs: finish_loading counts unreadable vs too large; load_notice(unreadable, too_large, truncated) -> Option<String>; only-too-large picks -> images_too_large(n) error"
  - "state.rs: busy guards in finish_loading / finish_export drop late results with log::warn! (LIFE-01, Pitfall 2)"
  - "controller.rs: MESSAGE_HIDE_AFTER = 3 s, message_timer: slint::Timer, armed_seq: Cell<Option<u64>>; timer closure finds the controller via CURRENT"
  - "view.rs: Snapshot.message: Option<(String, MessageKind)> -> Backend.message-style"
  - "Slint: enum MessageStyle { notice, error, success }, Backend.message-style, Theme.success/on-success, banner colours by style"
  - "texts.rs: images_too_large(count), reworded images_skipped, SHARE_UNAVAILABLE with next step, test failure_texts_say_what_happened_and_what_to_do"
affects: [06-04, 06-05, 06-07, 06-08]

# Actuals (#2632) - chars/4 over the realized code diff of the plan's 8 files (git diff, 24,574 chars incl. context)
actuals:
  tokens: 6100
  tasks: 2
  commits: 0
plan_head_before: ff11541208a5b6e669342453d41347f14e5b5d7b
plan_head_after: ff11541208a5b6e669342453d41347f14e5b5d7b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Messages carry a sequence number from one private builder (show); a single-shot slint::Timer is armed per success seq and can only expire that exact message"
    - "Timer closures capture plain data only and reach the controller through the thread-local CURRENT Weak (no Rc cycle)"
    - "Background results are accepted only in the matching busy state; anything else is dropped with log::warn!"

key-files:
  created: []
  modified:
    - crates/dimuli-app/src/state.rs
    - crates/dimuli-app/src/view.rs
    - crates/dimuli-app/src/controller.rs
    - crates/dimuli-app/src/texts.rs
    - crates/dimuli-app/ui/types.slint
    - crates/dimuli-app/ui/backend.slint
    - crates/dimuli-app/ui/message-banner.slint
    - crates/dimuli-app/ui/theme.slint

key-decisions:
  - "D-11: success messages auto-hide after 3 s (MESSAGE_HIDE_AFTER), errors and notices stay until closed; share success stays silent"
  - "D-12/D-13/D-18: unreadable and too-large images are counted and worded separately (du-form, with a next step); only-too-large picks explain the size, mixed/unreadable-only picks keep NO_IMAGES_LOADED"
  - "LIFE-01 (result half): finish_loading / finish_export ignore results unless busy is Loading / Exporting"
  - "ERRM-01 and LIFE-01 not marked complete: both are shared with 06-04/06-05 (failed-copy count, restart notice, lifecycle flows) and 06-08"

patterns-established:
  - "Banner look selected by an enum (MessageStyle) instead of a boolean; colours only in Theme"

requirements-completed: []  # Plan declares [ERRM-01, LIFE-01]; both continue in 06-04 / 06-05 / 06-08, so neither is marked complete here.

coverage:
  - id: D1
    description: "Saving shows 'Collage gespeichert.' as a success message; every message gets a new seq; expire only hides the success message with exactly that seq"
    requirement: ERRM-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib state::tests::expire -> 3 passed; state::tests::saved_success_shows_success_message, state::tests::every_message_gets_a_new_seq passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Success banner green on Android and gone 5 s later; editor unchanged"
    requirement: ERRM-01
    verification:
      - kind: e2e
        ref: "emulator-5554 logcat 'saved to gallery: true'; p6-02-saved.png banner pixel #B9E4C3 (Theme.success); p6-02-saved-later.png byte-identical to p6-02-editor.png (pre-save editor)"
        status: pass
    human_judgment: true
  - id: D3
    description: "Too large vs unreadable texts; partial loads open the editor; only-too-large picks explain the size"
    requirement: ERRM-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib state::tests::loading -> 9 passed (incl. loading_reports_too_large_images_separately, loading_reports_unreadable_and_too_large_together, loading_only_too_large_images_explains_the_size)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every failure text has two sentences (what happened, what to do) and no technical markers"
    requirement: ERRM-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib texts -> 2 passed (failure_texts_say_what_happened_and_what_to_do)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Late loading/export results change nothing"
    requirement: LIFE-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib state::tests::late -> 2 passed"
        status: pass
    human_judgment: false

# Metrics
duration: 3min (verification pass only; implementation by rust-coder not timed)
completed: 2026-11-14
status: complete
---

# Phase 6 Plan 02: Success Banner Auto-Hide, Failure Texts, Late-Result Guards Summary

**"Collage gespeichert." is now its own success message (green, `Theme.success`) that a 3 s single-shot `slint::Timer` hides again, armed per message sequence number so it can never hide a newer message; errors and notices stay until closed (D-11). Unreadable and too-large images get separate du-form texts with a next step (D-12, D-13, D-18), all failure texts are checked by a test, and `finish_loading` / `finish_export` drop results that arrive outside their busy state (LIFE-01, Pitfall 2). Verified independently: every plan gate and acceptance criterion passes.**

## Performance

- **Duration:** about 3 min for this verification pass (2026-11-14T20:20:08Z to about 20:23Z, warm cargo cache). Implementation and the emulator run by rust-coder were done beforehand and not timed here.
- **Tasks:** 2 of 2
- **Files:** 0 created, 8 modified

## Accomplishments

- `state.rs`: `MessageKind::Success`; `Message.seq`; `AppState.message_seq` (starts at 0); one private `show(kind, text)` uses `wrapping_add(1)`; `show_error`/`show_notice`/`show_success` delegate to it; `expire_message(seq)` clears only a Success message with that seq; `success_seq()`; `save_finished(Ok)` -> `show_success(texts::SAVED)`; `load_notice` returns `Option<String>` and `finish_loading` passes it to `show_notice` (no `Message` built outside `show`).
- `state.rs` `finish_loading`: guard `busy != Some(Busy::Loading)` -> `log::warn!("late loading result dropped")` and return; counts `ImageTooLarge` as too large, other errors and rejected inserts as unreadable; notice joins `images_skipped`, `images_too_large`, `too_many_images` in that order; when nothing is usable: `GENERIC_ERROR` if images loaded but the collage failed, `images_too_large(n)` if every failure was too large, else `NO_IMAGES_LOADED`.
- `state.rs` `finish_export`: guard `busy != Some(Busy::Exporting)` -> `log::warn!("late export result dropped")`, returns `None` without changes.
- `controller.rs`: `MESSAGE_HIDE_AFTER = Duration::from_secs(3)`; fields `message_timer`, `armed_seq`; `refresh` reads snapshot and `success_seq()` in one borrow scope, applies, then `arm_message_timer` (borrow released): new seq -> store + `start(SingleShot, 3 s, ...)`, same seq -> nothing, `None` -> `stop()` + clear. The closure captures only `seq` and upgrades `CURRENT`.
- `view.rs`: `Snapshot.message: Option<(String, MessageKind)>`; `apply` maps Error/Success/Notice(or none) to `MessageStyle` via `set_message_style`.
- Slint: `types.slint` `export enum MessageStyle { notice, error, success }`; `backend.slint` `in property <MessageStyle> message-style: MessageStyle.notice;` (boolean `message-is-error` removed); `theme.slint` `success: #b9e4c3`, `on-success: #0b2010`; `message-banner.slint` picks background and text colour by style (presentation only).

### New / changed texts (verbatim)

- `images_skipped(1)`: "Ein Bild konnte nicht gelesen werden und wurde übersprungen. Wähle bei Bedarf ein anderes Bild aus."
- `images_skipped(n)`: "{n} Bilder konnten nicht gelesen werden und wurden übersprungen. Wähle bei Bedarf andere Bilder aus."
- `images_too_large(1)` (new): "Ein Bild ist zu groß und wurde übersprungen. Wähle ein kleineres Bild aus."
- `images_too_large(n)` (new): "{n} Bilder sind zu groß und wurden übersprungen. Wähle kleinere Bilder aus."
- `SHARE_UNAVAILABLE`: "Teilen ist auf diesem Gerät nicht verfügbar. Speichere die Collage stattdessen."
- Unchanged: `NO_IMAGES_LOADED`, `GENERIC_ERROR`, `EXPORT_SOURCE_UNREADABLE`, `SAVE_FAILED`, `PICK_FAILED`, `SHARE_FAILED`, `SAVED`, `too_many_images`.

### Tests (names as planned)

- state.rs: `saved_success_shows_success_message` (renamed from `saved_success_shows_saved_notice`), `expire_hides_the_success_message_it_was_armed_for`, `expire_ignores_a_newer_message`, `expire_keeps_errors_and_notices`, `every_message_gets_a_new_seq`, `loading_reports_too_large_images_separately`, `loading_reports_unreadable_and_too_large_together`, `loading_only_too_large_images_explains_the_size`, `late_loading_result_is_ignored_when_not_loading`, `late_export_result_is_ignored_when_not_exporting`; kept: `loading_skips_broken_images_with_a_notice`, `loading_only_broken_images_stays_on_start_with_error`.
- texts.rs: `failure_texts_say_what_happened_and_what_to_do`.
- `#[test]` count: state.rs 53 -> 62, texts.rs 1 -> 2; dimuli-app lib 109 -> 119.

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `ff11541`, `git rev-list --count ff11541..HEAD` = 0).

Suggested commit message when the owner asks to commit (alone or together with other Phase 6 plans):

```
Auto-hide success messages and sharpen failure texts (phase 6, plan 02)

- "Collage gespeichert." is a success message (green banner) that
  hides itself after 3 s; a sequence number keeps the timer from
  hiding a newer message; errors and notices stay until closed
- too-large images get their own text, separate from unreadable
  ones; all failure texts say what to do next (tested)
- loading and export results that arrive outside their busy state
  are dropped with a log entry
```

## Files Created/Modified

- `crates/dimuli-app/src/state.rs`: message kind/seq/helpers, too-large counting, busy guards, 9 new tests + 1 renamed
- `crates/dimuli-app/src/view.rs`: message kind in the snapshot, `set_message_style`
- `crates/dimuli-app/src/controller.rs`: hide timer
- `crates/dimuli-app/src/texts.rs`: texts above, 1 test
- `crates/dimuli-app/ui/types.slint`, `backend.slint`, `message-banner.slint`, `theme.slint`: style enum, property, colours
- Other files in `git diff --stat -- crates/` (`decode.rs`, `render.rs`, `images.rs`) belong to 06-01. `Cargo.toml`, `Cargo.lock` and crate manifests unchanged (T-06-SC).

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| T1 `cargo test -p dimuli-app --lib state::tests::expire` | PASS: 3 passed, 0 failed |
| T1/T2 `cargo fmt --all --check` | PASS (exit 0, no "Diff in") |
| T1/T2 `cargo clippy --all-targets -- -D warnings` | PASS (exit 0, 0 `warning`/`error` lines) |
| T1/T2 `cargo test --workspace` | PASS: 90 (collage-core) + 119 (dimuli-app) passed, 0 failed |
| T1 desktop: assets present, `cargo build -p dimuli-app`, `timeout 15 target/debug/dimuli p6-a.jpg p6-b.jpg` | PASS: exit status 124 (only arboard/xdg-portal warnings in the log, no panic) |
| T1 emulator evidence: `grep -q 'saved to gallery: true' p6-02-saved-logcat.txt && test -s p6-02-saved.png && test -s p6-02-saved-later.png` | PASS (exit 0) |
| T2 `cargo test -p dimuli-app --lib state::tests::late` | PASS: 2 passed |
| T2 `cargo test -p dimuli-app --lib state::tests::loading` | PASS: 9 passed |
| T2 `cargo test -p dimuli-app --lib texts` | PASS: 2 passed |
| `state::tests::saved_success_shows_success_message`, `state::tests::every_message_gets_a_new_seq` | PASS (1 + 1) |

Acceptance greps, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `MessageKind::Success` in state.rs | >= 1 | 4 |
| `fn expire_message` in state.rs | >= 1 | 1 |
| `MESSAGE_HIDE_AFTER` in controller.rs | >= 1 | 2 |
| `Timer` in controller.rs | >= 1 | 3 |
| `enum MessageStyle` in types.slint | >= 1 | 1 |
| `message-is-error` in ui/*.slint | 0 (HEAD 3) | 0 (HEAD re-checked: 3) |
| `success` in theme.slint | >= 1 | 2 |
| `fn images_too_large` in texts.rs | >= 1 | 1 |
| `Speichere die Collage stattdessen` in texts.rs | >= 1 | 1 |
| `fn failure_texts_say_what_happened_and_what_to_do` in texts.rs | >= 1 | 1 |
| key link `expire_message` in controller.rs | >= 1 | 1 |
| key link `set_message_style` in view.rs | >= 1 | 1 |
| key link `images_too_large` in state.rs | >= 1 | 5 |

## Screenshot Observations (human-check, done by this executor)

- `android/build/smoke/p6-02-saved.png`: editor after "Speichern"; banner "Collage gespeichert." directly below the top bar in light green with dark text and the close button; sampled pixel (200,350) = `#B9E4C3` = `Theme.success`. Before this plan a saved notice was blue (`Theme.notice`), so the installed APK contains the 06-02 code.
- `android/build/smoke/p6-02-saved-later.png` (5 s later): no banner, canvas and controls unchanged; the file is byte-identical to `p6-02-editor.png` (the editor before saving), so the layout does not jump when the banner goes away.
- `p6-02-saved-logcat.txt`: `dimuli started` 21:18:05, `picked images: 1` 21:18:28, `saved to gallery: true` 21:18:45 (local time); no error lines.
- Only `emulator-5554` was attached (`adb devices`); the real device 8200fcc9 was not connected, so no device smoke in this plan (not required by the plan; device timing check is 06-08 Task 2 step 9).

## Must-Have Truths (checked against the diff)

| Truth | Evidence | Result |
|-------|----------|--------|
| Saved success in a success style, hides after ~3 s; errors and failure notices stay | `save_finished(Ok)` -> Success; timer only armed for `success_seq()`; `expire_keeps_errors_and_notices`; screenshots | PASS |
| A timer can only hide the message it was started for | `expire_message` compares kind and exact seq; `expire_ignores_a_newer_message`; `arm_message_timer` restarts the single-shot timer for a new seq | PASS |
| "zu groß" separate from "nicht gelesen"; partial failures still open the editor | `loading_reports_too_large_images_separately` (2 cells, too-large notice), `loading_reports_unreadable_and_too_large_together`, `loading_only_too_large_images_explains_the_size` | PASS |
| Every failure text says what happened and what to do, no technical details; failures logged with log::error! | text test over 11 texts; loader logs each skipped file (`images.rs:146`), save/share/export/collage failures log in state.rs; platform errors carry only `texts::*` constants (native_events.rs, android.rs, desktop.rs) | PASS |
| Late loading/export results are dropped with a log entry and change nothing | guards + `log::warn!`; `late_*` tests assert screen, collage, message, busy unchanged | PASS |

Production-code rules: no `unwrap`/`expect`/`panic!` added outside `mod tests` (all 13 new `unwrap` lines are in tests); no indexing in new production code; the `AppState` borrow is released before `apply` and before the timer start; no new crate or feature.

## Decisions Made

- ERRM-01 and LIFE-01 are not marked complete: ERRM-01 continues in 06-04 (failed-copy count) and 06-05 (restart notice); LIFE-01 in 06-05 (lifecycle flows) and 06-08 (device).

## Deviations from Plan

**1. [Evidence, no defect] The emulator run picked a different image than `p6-a.jpg`**
- The plan says to pick `p6-a.jpg`. `p6-02-picked.png` shows the newest gallery item selected, an earlier saved collage (red/green/blue), and the editor shows that single image. The check is about the save success banner and its disappearance, which does not depend on the image content, so the evidence still proves the truth. Noted for later emulator runs (06-04..06-06): select the pushed asset explicitly.

**2. [Observation, no defect] Hide timer after an activity recreation**
- The timer closure expires `seq` on whatever controller `CURRENT` holds. A new controller starts again at `message_seq = 0`, so a timer of an old controller could in theory hide a new success message with the same seq. This cannot happen as long as the old controller (and with it its `slint::Timer`) is dropped on recreation, which stops the timer. Candidate for the 06-05 lifecycle work / 06-07 review, not a FIX_REQUEST.

**3. [Observation] Sentence check in the text test is heuristic**
- `failure_texts_say_what_happened_and_what_to_do` counts `". "` plus a final `"."`. Texts with abbreviations ("z. B.") would be over-counted; none of the checked texts has one (`NO_IMAGE_ARGUMENTS` is excluded as planned).

**Total deviations:** 1 evidence deviation, 2 observations. No scope creep, no behaviour change beyond the plan.

## Issues Encountered

- APK mtime (21:18:33 local) is later than the logcat `dimuli started` line (21:18:05, emulator clock). The green banner pixel proves the installed build contains the 06-02 code, so this is a clock/re-package detail, not a stale-APK problem.

## Threat Model Check

- T-06-03 (Information disclosure, texts/banner): mitigated. Texts are constants without paths or error strings; text test rejects technical markers; causes go to `log::error!` only.
- T-06-04 (Tampering, late results): mitigated. Busy guards in `finish_loading`/`finish_export`; `late_*` tests.
- T-06-05 (Repudiation): mitigated for this plan's paths (loader per file, save/share/export/collage in state). Restart and Kotlin paths follow in 06-04/06-05 as planned.
- T-06-SC: no manifest or lock file changed; `slint::Timer` is part of slint 1.18.1.
- No new security surface beyond the threat model.

## Follow-ups / Open Items

- 06-04: failed-copy count from Kotlin should feed the same unreadable/too-large wording.
- 06-05: confirm the old controller and its timer are dropped on activity recreation (Deviation 2).
- 06-08 Task 2 step 9: success banner timing on the real device.
- Still open from Phase 5: final device test (Android 14 + 16), code review before commit, owner commit of Phase 5 and Phase 6 work.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 06-03 (Rust-owned canvas size, gesture fast path, desktop share early-out). Working tree green; HEAD unchanged at `ff11541`.

---
*Phase: 06-polish-and-v1-acceptance*
*Completed: 2026-11-14*

## Self-Check: PASSED

All 8 modified code files exist and contain the planned artifacts; 0 commits since ff11541 (as required, no commits).
