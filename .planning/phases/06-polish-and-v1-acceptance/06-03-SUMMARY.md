---
phase: 06-polish-and-v1-acceptance
plan: 03
subsystem: app canvas sizing (Slint canvas / backend / view / state / controller), gesture fast path, platform share capability
tags: [canvas, flicker, gesture-fast-path, row-sync, share, platform-trait]

# Dependency graph
requires:
  - phase: 06-polish-and-v1-acceptance
    plan: 02
    provides: "AppState.message_seq, used by Controller::update_gesture to fall back to a full refresh when a gesture raised a message"
provides:
  - "view.rs: fit_canvas(area, collage) -> Size (0x0 for invalid input); Snapshot.canvas_width/canvas_height set in the same apply as the cell/divider models"
  - "view.rs: cell_views_into / divider_views_into (reused buffers), sync_rows (slice row sync, sync_model delegates), Models::{cell_rows, divider_rows, dividers_dirty}, fill_gesture_rows / apply_gesture_rows"
  - "state.rs: AppState.canvas_area, set_canvas_area (sanitizing), private refit_canvas called from set_canvas_area, select_aspect, finish_loading, confirm_back"
  - "state.rs: AppState.share_supported (default true) and the Share early-out in begin_export with texts::SHARE_UNAVAILABLE"
  - "controller.rs: on_canvas_area_resized handler; update_gesture for moved/pinched/scrolled with refresh fallback on message_seq/screen change; install copies Platform::share_supported()"
  - "platform: Platform::share_supported (no default body); desktop false, Android true"
  - "Slint: Backend.canvas-width / canvas-height / canvas-area-resized(length, length); CollageCanvas reports its own size and draws the size Rust computed"
affects: [06-05, 06-07, 06-08]

# Actuals (#2632) - chars/4 over the realized code diff of this plan. The 8 files share hunks with 06-02;
# current diff of 06-02's 8 files is 48,335 chars vs 24,574 at 06-02 close, plus ~1,100 chars in platform/*.rs
# and the collage-canvas.slint hunk => ~24,900 chars for 06-03.
actuals:
  tokens: 6200
  tasks: 3
  commits: 0
plan_head_before: ff11541208a5b6e669342453d41347f14e5b5d7b
plan_head_after: ff11541208a5b6e669342453d41347f14e5b5d7b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Rust owns the canvas size: the UI only reports the available area, Rust computes the fitted canvas and the cells in the same transition and writes both in one apply"
    - "Gesture fast path: reusable row buffers in Models, row-wise model sync, full refresh only when the transition changed message_seq or screen"
    - "Platform capabilities as explicit trait methods without default bodies, copied into AppState at install"

key-files:
  created: []
  modified:
    - crates/dimuli-app/ui/backend.slint
    - crates/dimuli-app/ui/collage-canvas.slint
    - crates/dimuli-app/src/view.rs
    - crates/dimuli-app/src/state.rs
    - crates/dimuli-app/src/controller.rs
    - crates/dimuli-app/src/platform/mod.rs
    - crates/dimuli-app/src/platform/desktop.rs
    - crates/dimuli-app/src/platform/android.rs

key-decisions:
  - "POLI-01: the canvas size is computed in Rust (view::fit_canvas) and written together with the cells, so an aspect change cannot show a frame with stale cells; the Slint canvas has no sizing logic left"
  - "PERF-01 (gesture half): pan/pinch/wheel use update_gesture (row-only sync from reused buffers); press/release/long-press/cancel/pinch-start/area changes and buttons keep the full refresh"
  - "POLI-02 (share half): desktop Teilen answers with SHARE_UNAVAILABLE before any render or busy overlay; desktop share() keeps posting SHARE_UNAVAILABLE as a defensive fallback"
  - "POLI-02 marked complete (render-test half done in 06-01, share half here); POLI-01 stays open until the owner's device check (06-08 Task 2 step 5); PERF-01 continues in 06-04/06-06/06-08"

patterns-established:
  - "Slint components report sizes, Rust computes layout-relevant sizes and pushes them back as properties in the same apply as dependent models"

requirements-completed: [POLI-02]  # POLI-01 waits for 06-08 device check; PERF-01 continues in 06-04/06-06/06-08.

coverage:
  - id: D1
    description: "fit_canvas fits the collage ratio into the area and returns 0x0 for invalid input; an aspect change refits canvas and cells in one transition"
    requirement: POLI-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib fit_canvas -> 2 passed; state::tests::aspect -> 1 passed (aspect_change_refits_canvas_and_cells_in_one_transition); invalid_canvas_sizes_are_sanitized passed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Emulator layouts for 16:9, 9:16, landscape and portrait: centered, correct ratio, images fill their cells, no strip outside the canvas"
    requirement: POLI-01
    verification:
      - kind: e2e
        ref: "emulator-5554 screenshots p6-03-aspect-16x9.png (916x515 px, 1.78), p6-03-aspect-9x16.png (290x515, 0.563), p6-03-landscape.png (436x775, 0.563, centered at x=872 of area 132..1612), p6-03-portrait.png; logcat 1x 'dimuli started', no RustPanic; installed libdimuli.so sha256 == local APK and contains the 06-03 log string"
        status: pass
    human_judgment: true
  - id: D3
    description: "Gesture rows: only the panned cell changes, the buffer is reused, divider rows only while dragging a divider"
    requirement: PERF-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib view::tests::gesture -> 3 passed"
        status: pass
    human_judgment: false
  - id: D4
    description: "Share unsupported -> immediate SHARE_UNAVAILABLE error, no job, no busy; saving unaffected; default share still exports"
    requirement: POLI-02
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib state::tests::share_unsupported -> 2 passed; export_starts_in_the_editor_and_blocks_input passed"
        status: pass
    human_judgment: false

# Metrics
duration: 8min (verification pass only; implementation and emulator run by rust-coder not timed)
completed: 2026-11-14
status: complete
---

# Phase 6 Plan 03: Rust-Owned Canvas Size, Gesture Fast Path, Desktop Share Early-Out Summary

**Rust now computes the canvas size (`view::fit_canvas`) from the area the Slint canvas reports and writes it together with the cells in one apply, so an aspect change can no longer show a frame with stale cells (POLI-01). Pan, pinch and wheel events go through `Controller::update_gesture`, which refills reused row buffers and syncs only changed rows, falling back to a full refresh when a message or screen change happened (PERF-01). `Platform::share_supported` (desktop false, Android true) lets `begin_export(Share)` answer "Teilen ist auf diesem Gerät nicht verfügbar. …" before any render or busy overlay (POLI-02). Verified independently: every plan gate and acceptance criterion passes, including the `share_supported` greps.**

## Performance

- **Duration:** about 8 min for this verification pass (2026-11-14T20:28:31Z to about 20:36Z, warm cargo cache). Implementation and the emulator run by rust-coder were done beforehand and not timed here.
- **Tasks:** 3 of 3
- **Files:** 0 created, 8 modified

## Accomplishments

- `collage-canvas.slint`: the root `CollageCanvas` calls `Backend.canvas-area-resized(self.width, self.height)` in `changed width`, `changed height` and `init`; the inner `canvas` uses `Backend.canvas-width` / `canvas-height` and stays centered. Touch, pinch and timer code unchanged; no sizing logic in Slint.
- `backend.slint`: `canvas-aspect` float and `canvas-resized` callback removed; `in property <length> canvas-width`, `canvas-height`, `callback canvas-area-resized(length, length)` with comments.
- `view.rs`: `fit_canvas` (min(area.w, area.h * ratio), height = width / ratio, `Size::default()` for non-finite/non-positive input or result); `Snapshot.canvas_width/canvas_height` from `state.canvas`, set in `apply` directly before `sync_model(cells)` / `sync_model(dividers)`; `cell_views_into` / `divider_views_into` with thin `cell_views` / `divider_views` wrappers; `sync_rows(&VecModel<T>, &[T])` (`set_vec` only on row-count change, otherwise `set_row_data` per differing row), `sync_model` delegates; `Models` gets `cell_rows`, `divider_rows`, `dividers_dirty`, `fill_gesture_rows(&AppState)` and `apply_gesture_rows()`.
- `state.rs`: `canvas_area`; `set_canvas_area` (finite, non-negative) stores the area and calls `refit_canvas`; `refit_canvas` = `fit_canvas(canvas_area, collage size or 1x1)`, also called in `select_aspect` (after `set_aspect`), `finish_loading` (after the collage is created) and `confirm_back`; `share_supported: bool` (default `true`); `begin_export` early-out after the busy/screen guard and before the collage clone or `busy` (logs `sharing is not available on this platform`, `show_error(SHARE_UNAVAILABLE)`, `None`).
- `controller.rs`: `on!(on_canvas_area_resized, … set_canvas_area)`; `update_gesture` reads `message_seq`/`screen` in one borrow, applies the transition in a second, falls back to `refresh()` if either changed, otherwise `fill_gesture_rows(&state.borrow())` then `apply_gesture_rows()` with the borrow released; used for `on_canvas_moved`, `on_canvas_pinched`, `on_canvas_scrolled` only; `install` sets `state.share_supported = platform.share_supported()` in the same borrow block as `touch_target`.
- `platform/mod.rs`: `fn share_supported(&self) -> bool;` with doc comment, no default body; `desktop.rs` returns `false` (share() still posts `SHARE_UNAVAILABLE`), `android.rs` returns `true`.

### Tests (names as planned)

- view.rs: `fit_canvas_keeps_the_collage_ratio_inside_the_area`, `fit_canvas_is_empty_for_invalid_input`, `gesture_rows_change_only_the_panned_cell`, `gesture_rows_reuse_their_buffer` (capacity and pointer unchanged), `gesture_rows_include_dividers_only_while_dragging_a_divider`.
- state.rs: `aspect_change_refits_canvas_and_cells_in_one_transition` (16:9 -> 300x168.75, every cell inside; 9:16 -> 168.75x300), `share_unsupported_reports_unavailable_without_export_job`, `share_unsupported_still_allows_saving`; adapted: `invalid_canvas_sizes_are_sanitized` (area 200x100, canvas 100x100), `editor()` helper uses `set_canvas_area(300, 300)`; kept: `export_starts_in_the_editor_and_blocks_input`.
- `#[test]` count: view.rs 20 -> 25, state.rs 62 -> 65; dimuli-app lib 119 -> 127 (8 new).

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `ff11541`, `git rev-list --count ff11541..HEAD` = 0).

Suggested commit message when the owner asks to commit (alone or together with other Phase 6 plans):

```
Let Rust size the canvas, speed up gestures, answer desktop share at once (phase 6, plan 03)

- the canvas reports its available area; Rust computes the fitted
  canvas size and the cells in the same transition, so an aspect
  change never shows a frame with stale cells
- pan, pinch and wheel events refill reused row buffers and update
  only the changed cell rows; a message raised mid-gesture still
  triggers a full refresh
- Platform::share_supported: desktop "Teilen" reports "nicht
  verfügbar" without rendering or showing the busy overlay
```

## Files Created/Modified

- `crates/dimuli-app/ui/backend.slint`: canvas-width/height, canvas-area-resized (the `message-style` hunk in the same diff is 06-02)
- `crates/dimuli-app/ui/collage-canvas.slint`: area reporting, size from Rust
- `crates/dimuli-app/src/view.rs`: fit_canvas, *_into builders, sync_rows, gesture row buffers, 5 tests
- `crates/dimuli-app/src/state.rs`: canvas_area, set_canvas_area, refit_canvas, share_supported early-out, 3 new + 2 adapted tests
- `crates/dimuli-app/src/controller.rs`: on_canvas_area_resized, update_gesture, share_supported at install
- `crates/dimuli-app/src/platform/{mod,desktop,android}.rs`: share_supported
- Other files in `git diff --stat -- crates/` belong to 06-01 (`decode.rs`, `render.rs`, `images.rs`) and 06-02 (`texts.rs`, `types.slint`, `message-banner.slint`, `theme.slint`, message hunks of state/view/controller/backend). `Cargo.toml`, `Cargo.lock`, crate manifests and Gradle files unchanged (T-06-SC).

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| T1 `cargo test -p dimuli-app --lib fit_canvas` | PASS: 2 passed |
| T1 `cargo test -p dimuli-app --lib state::tests::aspect` | PASS: 1 passed (`aspect_change_refits_canvas_and_cells_in_one_transition`) |
| T1/T2/T3 `cargo fmt --all --check` | PASS (exit 0, no "Diff in") |
| T1/T2/T3 `cargo clippy --all-targets -- -D warnings` | PASS (exit 0, 0 `warning`/`error` lines) |
| T1/T2/T3 `cargo test --workspace` | PASS: 90 (collage-core) + 127 (dimuli-app) passed, 0 failed |
| T1 desktop: assets present, `cargo build -p dimuli-app`, `timeout 15 target/debug/dimuli p6-a.jpg p6-b.jpg` | PASS: exit status 124 (only arboard/xdg-portal warnings, no panic) |
| T1 emulator evidence: `test -s p6-03-aspect-16x9.png && test -s p6-03-landscape.png && dimuli started <= 1 && no RustPanic` | PASS (exit 0; 1x `dimuli started`, `picked images: 2`) |
| T2 `cargo test -p dimuli-app --lib view::tests::gesture` | PASS: 3 passed |
| T3 `cargo test -p dimuli-app --lib state::tests::share_unsupported` | PASS: 2 passed |
| T3 Android clippy `ANDROID_JAR=…/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` | PASS (exit 0) |

Acceptance greps, each run separately (HEAD values re-checked at `ff11541`):

| Criterion | Expected | Got |
|-----------|----------|-----|
| `fn fit_canvas` in view.rs | >= 1 | 3 (fn + 2 test names) |
| `fn set_canvas_area` in state.rs | >= 1 | 1 |
| `canvas-aspect` in ui/*.slint | 0 (HEAD 3) | 0 (HEAD 3) |
| `canvas-resized` in ui/*.slint | 0 (HEAD 4) | 0 (HEAD 4) |
| `canvas-area-resized` in ui/*.slint | >= 1 | 4 (backend decl + 3 in collage-canvas) |
| `on_canvas_resized` in controller.rs | 0 (HEAD 1) | 0 (HEAD 1) |
| `fn update_gesture` in controller.rs | >= 1 | 1 |
| `fn fill_gesture_rows` in view.rs | >= 1 | 1 |
| `update_gesture` in controller.rs | >= 2 | 4 (def + 3 callers) |
| `fn share_supported` in platform/mod.rs | >= 1 | 1 |
| `fn share_supported` in platform/desktop.rs | >= 1 | 1 |
| `fn share_supported` in platform/android.rs | >= 1 | 1 |
| `share_supported` in state.rs | >= 1 | 5 |
| key link `canvas_area_resized` in controller.rs | >= 1 | 1 |
| key link `set_canvas_width` in view.rs | >= 1 | 1 |
| key link `share_supported` in controller.rs | >= 1 | 1 |

## Screenshot Observations (human-check, done by this executor)

- `p6-03-aspect-16x9.png` (portrait, settings open, 16:9 selected): canvas about 916x515 px (ratio 1.78), horizontally centered, white border, red and green cells fill their cells to the canvas edge, divider handle in the middle; nothing drawn outside the white canvas.
- `p6-03-aspect-9x16.png` (9:16 selected): canvas about 290x515 px (0.563), centered at x=540, cells fill the canvas, no strip outside.
- `p6-03-landscape.png` (user_rotation 1): canvas about 436x775 px (0.563) in the left area, centered (x center 872 = center of the area 132..1612), settings panel on the right with 9:16 still selected; layout correct.
- `p6-03-portrait.png` (back to user_rotation 0): identical layout to the 9:16 portrait shot, no leftover from landscape.
- Evidence integrity: the emulator had `ch.staeheli.dimuli` installed at 21:26:15 (local); the pulled `base.apk` has a `libdimuli.so` with the same sha256 as `android/app/build/outputs/apk/debug/app-debug.apk` and contains the 06-03 string `sharing is not available on this platform`, so the screenshots show the 06-03 build. (APK file mtime 21:26:55 is a re-package after install, same pattern as in 06-02.)
- Rotation settings on emulator-5554 restored and re-checked: `accelerometer_rotation` = 1, `user_rotation` = 0.
- Only `emulator-5554` attached (`adb devices`); real device 8200fcc9 not connected, so no device smoke in this plan (not required).

## Must-Have Truths (checked against the diff)

| Truth | Evidence | Result |
|-------|----------|--------|
| Aspect, template or window/screen size change updates canvas size and every cell in the same transition | `refit_canvas` in `select_aspect`/`finish_loading`/`confirm_back`/`set_canvas_area`; `apply` sets canvas width/height right before syncing cells/dividers; template changes keep the collage aspect and go through the same full refresh; `aspect_change_refits_canvas_and_cells_in_one_transition` | PASS |
| Pan/pinch/wheel only rewrite changed cell rows (dividers only while dragging), no snapshot, reused buffer | `update_gesture` for the 3 handlers; `fill_gesture_rows` / `apply_gesture_rows` / `sync_rows`; 3 gesture tests | PASS |
| A message raised during a gesture still reaches the banner | `update_gesture` compares `message_seq` and `screen` before/after and calls `refresh()` on change | PASS (code review; no dedicated test, see Deviation 2) |
| Desktop Teilen shows SHARE_UNAVAILABLE without export or busy overlay; Android unchanged | early-out before clone/busy; desktop `share_supported` false, Android true; 2 share tests + default-true export test | PASS (unit; desktop click not exercised, see Deviation 3) |

Production-code rules: no `unwrap`/`expect`/`panic!` outside `mod tests` in view.rs, state.rs, controller.rs or the platform diffs; `AppState` borrow released before `apply_gesture_rows` (Slint models) and before `refresh`; the Slint canvas holds no sizing logic; `Platform::share_supported` has no default body; no new crate or feature.

## Decisions Made

- POLI-02 marked complete: render tests <= 64 px (06-01) and the desktop share early-out (here) together satisfy the requirement text.
- POLI-01 stays pending: structurally fixed and host-tested; the visual one-frame check is the owner's device step (06-08 Task 2 step 5).
- PERF-01 stays pending: Android import/downscale (06-04), memory matrix (06-06) and device stutter check (06-08) remain.

## Deviations from Plan

**1. [Evidence, no defect] Emulator evidence produced by rust-coder, re-validated here**
- Screenshots and logcat were captured by rust-coder before this pass. This executor re-ran the evidence gate, opened all four screenshots, and proved the installed library equals the local APK build and contains 06-03 code. Not re-captured to avoid an extra picker run.

**2. [Observation] No unit test for the gesture message fallback**
- The truth "a message raised during a gesture still reaches the banner" is implemented in `Controller::update_gesture` (needs a Slint window, so not host-testable in the current test setup). The plan did not ask for a test; candidate for the 06-07 review.

**3. [Observation] Desktop share click not exercised in the desktop run**
- The 15 s desktop run only proves start-up; the share early-out is covered by `share_unsupported_*` tests and the `install` copy of `share_supported()`. A manual click check fits 06-08 or the 06-07 review.

**4. [Observation] `sync_rows` allocates on row-count change**
- `set_vec(items.to_vec())` copies when the number of rows differs (template/image-count change, never during a pan/pinch). Accepted; matches the plan ("set_vec only when the row count differs").

**Total deviations:** 1 evidence deviation, 3 observations. No defect, no FIX_REQUEST, no scope creep.

## Issues Encountered

None. All gates green on the first run.

## Threat Model Check

- T-06-06 (Tampering, area sizes): mitigated. `set_canvas_area` ignores non-finite/negative values; `fit_canvas` returns 0x0 for invalid area or collage size and for a non-finite result; tests `fit_canvas_is_empty_for_invalid_input`, `invalid_canvas_sizes_are_sanitized`.
- T-06-07 (DoS, gesture path): mitigated. Per event: at most 9 placements, `CellView.image` is a refcounted `slint::Image` clone (no pixel copy), buffers reused (`gesture_rows_reuse_their_buffer`).
- T-06-SC: no manifest, lock or Gradle file changed.
- No new security surface beyond the threat model.

## Follow-ups / Open Items

- 06-07 docs: `docs/ARCHITECTURE.md` should describe that Rust owns the canvas size (Slint reports `canvas-area-resized`, Rust pushes `canvas-width/height`), the gesture fast path (`update_gesture`, `fill_gesture_rows`/`apply_gesture_rows`) and `Platform::share_supported` (rust-coder flagged this; scheduled for 06-07 docs task).
- 06-07 review: Deviations 2 and 3.
- 06-08 Task 2 step 4/5: gesture smoothness with large photos and no flicker on format change on real devices.
- Still open from Phase 5: final device test (Android 14 + 16), owner commit of Phase 5 and Phase 6 work.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 06-04 (Kotlin ImageDecoder transcode, downscale, failed-copy count). Working tree green; HEAD unchanged at `ff11541`.

---
*Phase: 06-polish-and-v1-acceptance*
*Completed: 2026-11-14*

## Self-Check: PASSED

All 8 modified code files exist and contain the planned artifacts; 0 commits since ff11541 (as required, no commits); STATE.md, ROADMAP.md (3/8), REQUIREMENTS.md (POLI-02) and 06-VALIDATION.md rows 6-03-01..03 updated.
