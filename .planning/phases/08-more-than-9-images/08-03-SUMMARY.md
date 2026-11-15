---
phase: 08-more-than-9-images
plan: 03
status: complete
requirements: [MANY-02, MANY-03]
completed: 2026-11-15
actuals:
  tasks: 3
  commits: 0
---

# Phase 8 Plan 03: App side for many images

Code by rust-coder (Task 1, then Tasks 2+3); orchestrator ran the gates. No commit.

## Built
- images.rs: `FULL_PREVIEW_IMAGES`, `preview_max_edge(count)` (2048 for 1-9; 1942/1773/1536/1228/1024 for 10/12/16/25/36, total preview pixels at the 9-image level); `load_image(path, max_edge)`; loader computes the edge once per pick.
- view.rs: preview cells use `collage.effective_style().corner_radius`; template thumbnails scale spacing with `style_scale(layout.max_cells_per_line())` (1-9 thumbnails bit-identical).
- state.rs: `log::info!("collage ready: {} images")` (count only); `Busy::Picking` with `begin_picking`, `pick_cancelled`, `pick_failed` (D-08); `begin_loading` accepts Picking on the start screen.
- controller.rs: `pick_images` calls `begin_picking()` (short borrow) + `refresh()` before `platform.pick_images(MAX_IMAGES)`; PickFailed → `pick_failed`; empty path list → `pick_cancelled`.
- No change to Slint files, Kotlin or `platform/`; `touch-target: 48px` unchanged (D-05/D-07).

## Tests (new)
`preview_max_edge_keeps_total_preview_pixels_at_the_nine_image_level`, `preview_respects_the_given_max_edge`, `many_images_load_into_the_editor_without_notice` (10/12/25/36 real PNGs), `too_many_images_notice_names_the_cap` ("Es werden nur die ersten 36 Bilder verwendet."), five view tests (`template_previews_of_one_to_nine_are_unchanged`, `template_previews_of_dense_grids_use_scaled_spacing`, `cell_views_use_the_effective_corner_radius`, `divider_handles_stay_grabbable_with_many_images`, `cells_keep_free_area_beside_the_handles_at_the_cap`), six `state::tests::pick_*`.

Smallest handle-free cell share at 36 images: 1:1 39.6 %, 4:5 44.8 %, 9:16 45.7 %, 16:9 30.8 % (matches D-06 basis).

## Gates
- preview/many_images/too_many_images: 5 passed; template_previews/effective_corner/divider_handles_stay/free_area: 6 passed; pick_: 6 passed
- Desktop start with 36 generated 1600x1200 JPEGs: `DISPLAY=:0`, `WAYLAND_DISPLAY=wayland-0`, `exit 124`, no panic (`android/build/smoke/p8-03-desktop-start.txt`, stderr only Wayland clipboard/D-Bus warnings)
- `android/app/src` and `platform/` unchanged; manifests unchanged
- fmt, clippy clean; workspace 108 + 148 passed; Android clippy (x86_64) clean

## Open
Docs (ARCHITECTURE limits/MAX_IMAGES 36, preview formula, effective style, Busy::Picking) are part of 08-05.
