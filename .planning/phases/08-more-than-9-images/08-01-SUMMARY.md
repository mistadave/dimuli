---
phase: 08-more-than-9-images
plan: 01
status: complete
requirements: [MANY-01]
completed: 2026-11-15
actuals:
  tasks: 2
  commits: 0
---

# Phase 8 Plan 01: Effective style rule, 1-9 frozen

Code by rust-coder in three subtasks (freeze, rule, edge cases); orchestrator ran the gates. No commit.

## Built
- `LayoutNode::max_cells_per_line()` (layout.rs)
- `FULL_STYLE_CELLS_PER_LINE = 4`, `style_scale()`, `Collage::effective_style()`; `content_bounds` (now takes `&Style`), `placements`, `dividers`, `drag_divider` use one effective style per call; `style()`/`set_style()` unchanged (D-04) (collage.rs)
- Export corner radius from `effective_style()` (render.rs)
- Tests: frozen `legacy_*` 1-9 generator + `templates_for_one_to_nine_are_unchanged`, `one_to_nine_geometry_equals_raw_style` (+ exact `effective_style() == style()`), tracer `effective_style_scales_from_five_cells_per_line`, `export_uses_the_effective_corner_radius` (64 px, `setup_with`), eight edge-case tests; `#[cfg(test)] test_grid` helper

## Gate results
- `one_to_nine` before production change: `test result: ok. 2 passed` — after: `test result: ok. 2 passed`
- `effective`: 5 passed; `max_cells_per_line style_scale effective dense`: 10 passed
- `self.style.(spacing|padding)` in collage.rs: 2 (only in `effective_style`)
- fmt, clippy clean; `cargo test --workspace`: 102 (collage-core) + 133 (dimuli-app) passed
- Cargo/Gradle manifests unchanged since 5c2f109

## Deviations
- `MAX_IMAGES` is still 9 until 08-02, so collages with more than 9 cells cannot be built yet. Tests that the plan specified on 10-cell layouts use 9-cell equivalents with the same property, each marked "switch to … once MAX_IMAGES is raised (08-02)":
  - tracer and dense drag: TopBottom [5, 4] instead of [5, 5] (DividerId(4) = last divider of row 0)
  - stored-style test: apply_template to [3, 3, 3] instead of [3, 3, 4]
  - radius for a [10] row checked arithmetically (`MAX_CORNER_RADIUS * style_scale(10)`), [9] row on a real collage
- **For 08-02:** switch these marked tests back to the 10+ layouts.

## Pitfall 3 audit (collage-core)
Production reads of the raw stored style only in `effective_style` (collage.rs 271-273). The other production hits (`content_bounds`, `placements`, `dividers`, `drag_divider`, render.rs `compose`) read the already effective style. All other hits are tests. dimuli-app consumers (view.rs preview radius, template thumbnails) follow in 08-03.
