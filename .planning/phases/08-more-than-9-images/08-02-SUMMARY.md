---
phase: 08-more-than-9-images
plan: 02
status: complete
requirements: [MANY-01, MANY-02]
completed: 2026-11-15
actuals:
  tasks: 2
  commits: 0
---

# Phase 8 Plan 02: Grid generator for 10-36, MAX_IMAGES 36

Code by rust-coder (two subtasks); orchestrator ran the gates. No commit.

## Built
- templates.rs: branch for counts above 9 next to the untouched 1-9 generator; `CLASSIC_MAX_IMAGES` (pub(crate), 9), `GRID_LINE_COUNTS` (2 most balanced line counts), `MAX_GRID_TEMPLATES` (6). Even grids only (rows/cols), no big-* above 9. Truncation to 6 placed after the big-* block (borrow of the push closure); same result since big-* exists only up to 5 images.
- lib.rs: `MAX_IMAGES = 36` (D-06) with reason in the doc.
- Tests fixed for the higher cap: `all_templates_produce_valid_cells_in_every_format` and `grid_shapes_respect_line_limits` now loop over `1..=CLASSIC_MAX_IMAGES`; `max_images_collage_renders` colour `(i % 12) * 20` (no u8 overflow).
- New tests: `many_images_default_templates_are_the_most_balanced`, `many_images_export_shows_every_cell` (12 images, 64 px), `many_images_get_four_to_six_balanced_grids`, `many_images_templates_produce_valid_cells_in_every_format` (all templates 10-36, 4 formats, default + max style; MIN_CELL_SIZE except 16:9 at max style, which only requires positive size), `many_images_drag_never_panics_overlaps_or_shrinks` (25 and 36), `many_images_dense_vertical_dividers_stay_pinned` (36, cols-7-7-7-7-8, 16:9, max style).
- 08-01 carry-over: tests switched back to the intended 10+ layouts ([5,5], [3,3,4], real [10] row); "switch to" comments removed.

## Gate results
- `many_images`: 7 passed; `one_to_nine`: 2 passed (1-9 still frozen)
- `MAX_IMAGES: usize = 36` present; no `i * 20` in `max_images_collage_renders`; no `for count in 1..=MAX_IMAGES` in collage.rs
- fmt, clippy clean; workspace 108 + 133 passed; manifests unchanged since 5c2f109
- dimuli-app needed no change for the cap (its loops over MAX_IMAGES pass with 36)

## Notes
- Template names for 13/16/25 only checked indirectly (first template for 16 and 25; 10, 12, 36 fully). Simulated minimum cell sizes from the plan (0.1194/0.0647 default, 0.0906/0.0359 max) confirmed only indirectly through the assertions.
- docs/ARCHITECTURE.md still names the 9-image limit — updated in 08-03/08-05.
