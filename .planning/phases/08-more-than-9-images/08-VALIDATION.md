---
phase: "8"
slug: "more-than-9-images"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-11-15"
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]` (`cargo test`), no extra crates; emulator checks via `adb` scripts (`android/scripts/smoke.sh`, logcat, `/proc/<pid>/status`) |
| **Config file** | none (workspace `Cargo.toml`; `clippy.toml` allows unwrap/expect/panic/indexing in tests) |
| **Quick run command** | `cargo test -p collage-core --lib <filter>` or `cargo test -p dimuli-app --lib <filter>` (the filter named in each task) |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |
| **Android lint** | `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` |
| **Estimated runtime** | ~5 s per filtered crate run after an incremental build; full gate ~1-2 min |

---

## Sampling Rate

- **After every task commit:** the task's filtered `cargo test … --lib <filter>` (checked for a non-zero passed count) plus the full suite command
- **After every plan wave:** full suite command; Android lint when `dimuli-app` changed (08-03, 08-05)
- **Before `/gsd-verify-work`:** full suite green, Android lint green, emulator evidence for 12 and 25 images (08-04), review gates (08-05), owner approval (08-06)
- **Max feedback latency:** ~60 s for a crate test run, ~2 min for the full gate

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 1 | MANY-01 | T-08-01 | Style factor finite and exactly 1.0 up to 4 cells per line; 1-9 geometry bit-identical | unit (Wave 0 freeze, then tracer) | `cargo test -p collage-core --lib one_to_nine` and `cargo test -p collage-core --lib effective` | ❌ W0 (task creates) | ⬜ pending |
| 8-01-02 | 01 | 1 | MANY-01 | T-08-02 | Dense drags use the same spacing as placements, stop at MIN_CELL_SIZE | unit (tdd) | `cargo test -p collage-core --lib -- max_cells_per_line style_scale effective dense` | ❌ (task creates) | ⬜ pending |
| 8-02-01 | 02 | 2 | MANY-01, MANY-02 | T-08-03 | Cap 36 enforced in the core (TooManyImages, no templates above) | unit (tracer) | `cargo test -p collage-core --lib many_images` | ❌ (task creates) | ⬜ pending |
| 8-02-02 | 02 | 2 | MANY-01 | T-08-04 | Drags on cells below MIN_CELL_SIZE are no-ops, never panic or overlap | unit (tdd) | `cargo test -p collage-core --lib many_images` (at least 7 passed) | ❌ (task creates) | ⬜ pending |
| 8-03-01 | 03 | 3 | MANY-02, MANY-03 | T-08-05, T-08-08 | Preview pixels capped at the 9-image budget; log line without paths | unit (tracer) + desktop start | `cargo test -p dimuli-app --lib -- preview_max_edge preview_respects many_images too_many_images`; display-aware start gate: `grep -qx 'exit 124' android/build/smoke/p8-03-desktop-start.txt` and no panic in `p8-03-desktop-stderr.txt` when a `DISPLAY`/`WAYLAND_DISPLAY` is recorded in it (none recorded: reported as not evaluated, not a failure) | ❌ (task creates) | ⬜ pending |
| 8-03-02 | 03 | 3 | MANY-03 | — | N/A (view geometry: D-05 handles, D-06 free area, radius, thumbnails) | unit (tdd) | `cargo test -p dimuli-app --lib -- template_previews effective_corner divider_handles_stay free_area` | ❌ (task creates) | ⬜ pending |
| 8-03-03 | 03 | 3 | MANY-03 | T-08-06, T-08-07 | No second pick while copying; overlay never sticks after cancel or failure (D-08) | unit (tdd) + Android lint | `cargo test -p dimuli-app --lib state::tests::pick_` | ❌ (task creates) | ⬜ pending |
| 8-04-01 | 04 | 4 | MANY-02, MANY-03 | T-08-09, T-08-10 | 12 photos within VmHWM below 1 GiB; picker cancel releases the overlay | emulator (scripted adb) | `grep -q 'picked images: 12, failed copies: 0' android/build/smoke/p8-04-12-logcat.txt` (+ smoke, focus, crash and VmHWM gates in the plan) | ❌ (task creates evidence) | ⬜ pending |
| 8-04-02 | 04 | 4 | MANY-03 | T-08-10 | 25 photos within VmHWM below 1 GiB; exactly one pick despite repeated taps | emulator (scripted adb) | `grep -q 'collage ready: 25 images' android/build/smoke/p8-04-25-logcat.txt` (+ focus, screenshot-diff and VmHWM gates in the plan) | ❌ (task creates evidence) | ⬜ pending |
| 8-05-01 | 05 | 5 | MANY-01, MANY-02, MANY-03 | T-08-11, T-08-SC | Review findings fixed; no manifest or Android source change | review + full gates | full suite + Android lint; `grep -q 'APK CHECK OK' android/build/smoke/p8-05-check-apk.txt` | ❌ (task creates) | ⬜ pending |
| 8-05-02 | 05 | 5 | MANY-01, MANY-02, MANY-03 | T-08-12 | Docs record the limits and behaviour as built | doc greps | `grep -cE '^\| .MAX_IMAGES. \| 36 \|' docs/ARCHITECTURE.md` (+ the other doc gates in the plan) | ✅ docs exist | ⬜ pending |
| 8-06-01 | 06 | 6 | MANY-01, MANY-02, MANY-03 | T-08-13 | Only smoke on the owner's phone | desktop start + smoke | display-aware start gate: `grep -qx 'exit 124' android/build/smoke/p8-06-desktop-start.txt` and no panic in `p8-06-desktop-stderr.txt` when a `DISPLAY`/`WAYLAND_DISPLAY` is recorded in it (none recorded: reported as not evaluated, not a failure) | ❌ (task creates evidence) | ⬜ pending |
| 8-06-02 | 06 | 6 | MANY-01, MANY-02, MANY-03 | T-08-14 | Owner's photos never touched by automation | manual (owner checkpoint) | — (see Manual-Only Verifications) | n/a | ⬜ pending |
| 8-06-03 | 06 | 6 | MANY-01, MANY-02, MANY-03 | — | N/A | doc grep | `grep -c '^- \[x\] 8\.' docs/PLAN.md` | ✅ docs exist | ⬜ pending |

*Status: ⬜ not run yet · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/collage-core/src/templates.rs` tests: verbatim `legacy_templates_for` copy and `templates_for_one_to_nine_are_unchanged`, green on the unchanged code (08-01 Task 1, subtask A)
- [ ] `crates/collage-core/src/collage.rs` tests: `one_to_nine_geometry_equals_raw_style` over all 1-9 templates, 4 formats and 4 styles, exact equality, green on the unchanged code (08-01 Task 1, subtask A)
- [ ] `CLASSIC_MAX_IMAGES` (9) scopes the classic tests that only hold for 1-9 when `MAX_IMAGES` becomes 36 (08-02 Task 1)

Everything else uses the existing `#[test]` infrastructure, the existing render-test helpers (images of at most 64 px generated in the test) and the proven emulator scripts.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Pinch zoom with many images | MANY-03 | `adb shell input` injects a single pointer only; zoom math is count-independent and host-tested | 08-06 step 8: two fingers on the phone or Ctrl + mouse drag on the emulator, never the mouse wheel on Android |
| Desktop editor with 36 images (templates, settings, save) | MANY-01, MANY-02 | No input automation under WSLg; the executor proves only that the binary starts with 36 arguments (and only when a display is recorded) | 08-06 steps 1-7 |
| Visual judgement of dense grids and the accepted 16:9 occlusion at 25 images | MANY-03 (D-07) | Which element moved and whether occlusion is acceptable is visual; scripted gates only prove that the screen changed | 08-04 human-checks, deferred to and judged by the owner in 08-06 Task 2 step 9 a-h; 08-06 step 9 |
| Cap of 36 confirmed | MANY-02 (D-06) | Owner decision | 08-06 step 10 |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the owner checkpoint 8-06-02 is manual-only and listed above)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [ ] Feedback latency < 60 s (to be confirmed by the 08-01/08-02 run times)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
