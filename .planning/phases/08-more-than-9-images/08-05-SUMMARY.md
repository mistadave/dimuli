---
phase: 08-more-than-9-images
plan: 05
status: complete
requirements: [MANY-01, MANY-02, MANY-03]
completed: 2026-11-15
actuals:
  tasks: 2
  commits: 0
---

# Phase 8 Plan 05: Code review and docs

## Review
code-reviewer on `git diff 5c2f109 -- crates android`: **approved, 0 critical, 0 warnings, 5 notes** (`08-REVIEW.md`). No fix was required; nothing delegated to rust-coder.

| Note | Disposition |
|---|---|
| 1 Free-area test only for the first 36-template, threshold 0.25 vs D-06's 30 % (measured min 0.308 in 16:9) | Open for owner (test strength) |
| 2 `templates_for_one_to_nine_are_unchanged` exact compare lacks reason comment | Open for owner (one-line comment) |
| 3 Back during the Kotlin copy (Busy::Picking) closes the app, as with Loading before Phase 8 | Open for owner (optional: treat Picking/Loading as Handled) |
| 4 No timeout if the picker never answers | Accepted for v1 (all Kotlin paths answer) |
| 5 Untracked `.codex/` | Not committed |

## Gates (no code change after the review)
- fmt, clippy, workspace tests 108 + 148, Android clippy (x86_64): clean (reviewer run)
- `assembleDebug` (both ABIs) + `check-apk.sh`: `APK CHECK OK` (`p8-05-check-apk.txt`)
- `SMOKE OK emulator-5554` (`p8-05-smoke-5554.txt`)
- Phone 8200fcc9 attached: arm64 build + `SMOKE OK 8200fcc9` (`p8-05-device-8200fcc9-smoke.txt`); both-ABI APK rebuilt afterwards, `APK CHECK OK`
- Cargo/Gradle manifests and `android/app/src` unchanged since 5c2f109

## Docs
- docs/ARCHITECTURE.md: limits table (`MAX_IMAGES` 36 with reason D-06, `PREVIEW_MAX_EDGE`/`preview_max_edge(n)` formula, new `FULL_STYLE_CELLS_PER_LINE` row, style-limits reason, `MIN_CELL_SIZE` 16:9 note), module rows (layout `max_cells_per_line`, templates 10+, collage `effective_style`), operations block (`effective_style`), state.rs `Busy::Picking`, images.rs `preview_max_edge`, pick flow with `begin_picking` (D-08), back on start screen while picking, gestures note on handle occlusion (D-07), Android cache size, desktop cut to 36; "Bewusst nicht in v1" no longer lists more than 9 images.
- docs/PLAN.md: milestone 8 text "auf 36 angehoben", verification adds 12/25 emulator and 36 desktop; milestone 8 still unticked.
- docs/CODING_GUIDELINES.md: previews smaller from 10 images (budget as for 9).
- All Task 2 doc gates pass; CLAUDE.md unchanged.
