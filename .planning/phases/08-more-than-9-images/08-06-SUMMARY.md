---
phase: 08-more-than-9-images
plan: 06
status: complete
requirements: [MANY-01, MANY-02, MANY-03]
completed: 2026-11-15
actuals:
  tasks: 3
  commits: 0
---

# Phase 8 Plan 06: Owner check

## Owner kit (Task 1)
- Desktop: 36 generated JPEGs in `android/build/smoke/p8-desktop/`; start with 36 arguments: `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `exit 124`, no panic (`p8-06-desktop-start.txt`).
- Emulator emulator-5554: reviewed both-ABI APK, `SMOKE OK emulator-5554` (`p8-06-smoke-5554.txt`), 25 `p8-` photos in /sdcard/Pictures, dimuli relaunched on the start screen.
- Phone 8200fcc9 (Android 14): reviewed arm64 build from 08-05, `SMOKE OK 8200fcc9`; same sources, so the 08-05 transcript was copied to `p8-06-device-8200fcc9-smoke.txt` instead of a second install over the unstable usbipd link.
- `p8-06-owner-kit.txt` with commands, screenshots list and memory/time table.

## Owner verdict (Task 2)
**approved** — cap of 36 stays (D-06). The 08-04 human-checks (a-h) are covered by this approval.

## Close-out (Task 3)
- docs/PLAN.md: milestone 8 ticked.
- 08-VALIDATION.md: all rows green, sign-off items ticked, `status: validated`, approval line dated.
- REQUIREMENTS.md MANY-01..03 complete; ROADMAP Phase 8 6/6 complete; PROJECT.md and STATE.md updated.

## Observations for later (not caused by Phase 8)
- Busy overlay: the pulse circle sits at the left edge instead of the centre (`ui/busy-overlay.slint`, unchanged since Phase 6).
- Gesture navigation: a pan starting within ~60 px of the screen edge triggers Android's back gesture (discard dialog).
- Review notes 1-3 in 08-REVIEW.md (free-area test threshold, missing reason comment on one exact compare, back during the copy closes the app).
