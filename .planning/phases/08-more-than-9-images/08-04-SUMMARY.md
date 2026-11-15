---
phase: 08-more-than-9-images
plan: 04
subsystem: android-verification
status: complete
tags: [emulator, photo-picker, memory, many-images, D-05, D-07, D-08]
requirements: [MANY-02, MANY-03]
completed: 2026-11-15
dependency_graph:
  requires: [08-03]
  provides: ["emulator evidence for 12 and 25 images under android/build/smoke/p8-04-*"]
  affects: [08-05, 08-06]
tech-stack:
  added: []
  patterns: ["picker automation via uiautomator content-desc filter ('Photo taken on Nov 15, 2026 9:59 PM')"]
key-files:
  created:
    - .planning/phases/08-more-than-9-images/08-04-SUMMARY.md
  modified: []
decisions:
  - "Did not delete the older p6-* test files on the emulator (orchestrator rule: no deletion of existing media); only p8-* were replaced"
  - "Added a second, fresh-process 25-image run so the extra taps land during the Kotlin copy (Busy::Picking) and VmHWM is measured for 25 images alone"
metrics:
  duration: "~20 min"
  started: 2026-11-15T20:58Z
actuals:
  tokens: 0
  tasks: 2
  commits: 0
plan_head_before: 28c999a36847b50b546dfa49adbc39470f9c1664
plan_head_after: 28c999a36847b50b546dfa49adbc39470f9c1664
---

# Phase 8 Plan 04: Emulator verification of 12 and 25 images Summary

On emulator-5554 (dimuli_api36), 12 and 25 picked 12 MP JPEGs (4000x3000) load into the editor, can be edited and save to the gallery. Peak memory is 482-486 MB (VmHWM), slightly below the 9-image baseline. Dividers can still be grabbed with 25 images. The D-08 overlay is released when the picker is cancelled and swallows taps while the copy runs. No code was changed and nothing was committed.

## Gates

| Gate | Result |
|------|--------|
| `SMOKE OK emulator-5554` in `p8-04-smoke-5554.txt` (debug APK `-PrustAbis=x86_64`, build OK) | PASS |
| After a cancelled pick, `p8-04-cancel-focus.txt` names `com.google.android.photopicker` (overlay released, new pick started) | PASS |
| 12: `picked images: 12, failed copies: 0`, `collage ready: 12 images`, `saved to gallery: true`, no panic, crash buffer empty | PASS |
| 12: VmHWM < 1 GiB | PASS (485 932 kB) |
| 25: `picked images: 25, failed copies: 0` exactly once, `collage ready: 25 images`, `saved to gallery: true` | PASS |
| 25: focus is dimuli and not the picker, no panic, crash buffer empty | PASS |
| 25: each gesture screenshot differs from the previous one; `p8-04-25-16x9.png` exists | PASS |
| 25: VmHWM < 1 GiB | PASS (485 932 kB in run 1, 482 264 kB in the fresh-process run 2) |

Tracer gate (Task 1): all `<verify>` checks are automated, `human_verify_mode` is end-of-phase, so the checks were re-run, all passed, and Task 2 followed.

## Memory and timing

| Scenario | VmHWM | VmRSS (after save) | Load s (Done tap -> editor) | of which Rust load (log) | Export s (Speichern -> saved) |
|----------|-------|--------------------|-----------------------------|--------------------------|-------------------------------|
| 9 images, baseline (06-06/06-08) | 500 624 kB | - | - | - | - |
| 12 images (fresh process) | 485 932 kB | 393 888 kB | 18.3 | 10.2 | 13.9 |
| 25 images, run 1 (same process as the 12 run) | 485 932 kB (*) | 396 500 kB | 28.1 | 15.8 | 25.6 (template 6-6-6-7, 16:9) |
| 25 images, run 2 (fresh process, cold start) | 482 264 kB | 396 260 kB | 28.3 | 17.2 | 25.3 (rows-5-5-5-5-5, 1:1) |

(*) VmHWM is the peak over the life of the process. In run 1 it therefore covers both the 12 and the 25 run; it did not rise above the 12-image peak. Run 2 measures 25 images on their own.

- The Kotlin copy takes about 8 s for 12 and 11.6-12.4 s for 25, about 0.47 s per 12 MP JPEG. Times were measured on the host, polling every 0.5 s; the device log stamps are in `p8-04-times.txt`. The host clock runs about 30.5 s ahead of the device clock, and this is already corrected for.
- Cache (Pitfall 8): `cache/picked` holds 49 332 kB with 25 images (25 files) and 23 700 kB with 12. `/data` is at 17 % (1.5 G of 10 G used).

## D-08, D-05, D-07 observations

- **D-08 cancel:** cancelling with Back leaves the start screen without an overlay (`p8-04-cancel.png`), and "Bilder wählen" opens the picker again. Dismissing the picker by swiping the sheet down, which happened by accident (see deviation 2), also logged `picked images: 0` and left the start screen without an overlay.
- **D-08 repeated taps:** in run 2 both taps on "Bilder wählen", 0.3 s apart, finished about 3.8 s before the copy ended (`picked images` 22:15:13.241), so they hit the overlay in the Busy::Picking state. `picked images:` appears exactly once (`p8-04-25b-logcat.txt`), focus stayed on dimuli, and `p8-04-25-copying.png` shows the scrim with "Bilder werden geladen ...". In run 1 the taps came in just after the copy (Busy::Loading); there too only one pick happened.
- **D-05:** with 25 images (rows-5-5-5-5-5, 1:1), the row divider between rows 1 and 2 moved about 78 px (input 80 px), and the column divider between cells 1 and 2 of row 1 moved right. Pan: the difference is limited to the bottom-left cell (37 922 of 37 925 pixels). Swap: cells 1 and 4 of row 3 exchanged. Template 3 (6-6-6-7) was applied.
- **D-07 (16:9, template 6-6-6-7):** the canvas is 1080x~600 px (physical) and the rows are about 143 px tall. Estimate from the 48 dp (126 px) square handles: no cell is fully covered. Inner cells in rows 1-3 keep a free strip about 47 px wide; the 7 cells in row 4 keep only about 21-28 px. All 25 cells are visible (`p8-04-25-16x9.png`). The plan applies 16:9 after the template switch, so the default 5x5 template in 16:9 (research: 3 of 25 fully covered) was not shown on the emulator.

## Research assumptions

- **A4:** partly confirmed. Copying a 12 MP JPEG takes about 0.47 s; HEIC transcoding was not tested because the assets are JPEG. Export at 25 x 12 MP takes about 25 s (about 1 s per image) and the Rust load about 0.65-0.7 s per image, so load and export grow linearly, as Pitfall 6 expects.
- **A5:** confirmed. VmHWM was 485 932 kB with 12 and 482 264 kB with 25 images, both slightly below the 9-image level of 500 624 kB. The preview budget holds.
- **A6:** not measured. All drags, the pan, the swap and the template switch with 25 tiles and 24 handles were processed correctly and without errors. Frame smoothness cannot be measured with adb, so it is left to the owner check in 08-06.

## Deviations from Plan

1. **[Owner rule] p6-* files kept.** Plan step 2 would have deleted `/sdcard/Pictures/p6-*`. Following the orchestrator's rule against deleting existing media, only `p8-*` was replaced. This had no effect, because selection filters by the picker's content-desc for the p8 photos.
2. **[Rule 3 - blocking] Repeated the first 25-image attempt.** The collage saved in Task 1 is the newest item in the picker, and a filter on "Nov 15" alone had selected it. While correcting this, a downward swipe on the sheet closed the picker (cancel, `picked images: 0`; overlay released correctly). The logcat was cleared and the 25-image run repeated with the filter `Photo taken on Nov 15, 2026 9:59 PM`, so exactly the 25 p8 photos were selected. The gate files contain only the valid run.
3. **[Rule 3 - blocking] Repeated the pan.** The first pan started at x=50 and triggered Android's back-edge gesture, which opened the "Collage verwerfen?" dialog. "Abbrechen" closed it (the collage was unchanged; `p8-04-25-pre-pan.png`) and the pan was repeated from x=100.
4. **[Rule 2 - evidence] Second fresh-process 25-image run.** In run 1, `screencap` blocked for about 11.6 s while the copy I/O ran: it captured the picker's closing frame, and the taps came after the copy. Run 2 provides the overlay screenshot during Busy::Picking and a VmHWM for 25 images alone. The run-1 screenshot is kept as `p8-04-25-copying-run1.png`; `p8-04-25-copying.png` (= `p8-04-25b-copying.png`) is from run 2. Run 2's files: `p8-04-25b-logcat.txt`, `p8-04-25b-crash.txt` (empty), `p8-04-25b-focus.txt`, `p8-04-mem-25b.txt`.

## Defects

None caused by phase 8. Two findings from before phase 8, for the owner or the backlog (no fix in this plan):
- `ui/busy-overlay.slint` (unchanged since phase 6): the pulse circle sits at the left edge (x 0-126 px) instead of the centre. In the `VerticalLayout`, the Rectangle with a fixed `width` is placed left-aligned. Visible in `p8-04-25-copying.png`.
- With gesture navigation, the left and right screen edges belong to the system back gesture. In the 1:1 format, whose canvas spans the full width, a pan in the outer cells starting within about 60 px of the edge opens the discard dialog. This is Android's behaviour, but worth a note in 08-06.

## Deferred to 08-06 (human-check, Task 2 step 9)

Screenshots for owner review: `p8-04-cancel.png`, `p8-04-12-editor.png` (4-4-4, no notice banner; confirmed by the executor at a glance), `p8-04-25-copying.png`, `p8-04-25-editor.png`, `p8-04-25-row-divider.png`, `p8-04-25-col-divider.png`, `p8-04-25-pan.png`, `p8-04-25-swap.png`, `p8-04-25-template.png`, `p8-04-25-16x9.png`, `p8-04-25-saved.png`.

## Emulator state for 08-05/08-06

emulator-5554 is still running and holds p8-01..p8-25 in `/sdcard/Pictures/`. Three new collages from this plan are in `/sdcard/Pictures/dimuli/` and are the newest items in the picker, so later automation must filter for p8 photos (`Photo taken on Nov 15, 2026 9:59 PM`). dimuli is open in the editor with 25 images. The phone 8200fcc9 was not touched.

## Self-Check: PASSED

- Evidence files present: `p8-04-smoke-5554.txt`, `p8-04-cancel.png`, `p8-04-cancel-focus.txt`, `p8-04-12-{editor,saved}.png`, `p8-04-12-{logcat,crash}.txt`, `p8-04-mem-12.txt`, `p8-04-25-{copying,editor,row-divider,col-divider,pan,swap,template,16x9,saved}.png`, `p8-04-25-{logcat,crash,focus}.txt`, `p8-04-mem-25.txt`, `p8-04-cache-25.txt`, `p8-04-times.txt`, `p8-assets/p8-01..25.jpg`
- No commits expected (commits: 0); source tree unchanged by this plan (`git status` shows only the pre-existing phase-8 changes)
