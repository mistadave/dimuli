# Phase 8: More Than 9 Images - Context

**Gathered:** 2026-11-15
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can build a collage from more than 9 images. A grid generator in `crates/collage-core/src/templates.rs` yields valid layouts (no overlaps, correct spacing) for any image count above 9; the fixed `MAX_IMAGES = 9` limit is lifted, the "too many images" hint no longer appears for normal selections, and the templates for 1-9 images behave exactly as before (MANY-01..03). Dividers, pan/zoom, swap and export stay operable and stable with many images on the emulator without out-of-memory.

Not in this phase: new layout styles beyond grids for >9, canvas zoom, persistence, any new editing capability.

</domain>

<decisions>
## Implementation Decisions

### Spacing, padding and corners with many cells (discussed)
- **D-01:** The style sliders keep their meaning ("equally airy / equally round") at every image count: the *effective* spacing, padding and corner radius shrink automatically when a layout has many cells per row/column, so cells never disappear. Preview and export use the same effective values (one shared function in collage-core, like the shared crop function). — **Reversibility:** costly — the effective-style rule feeds geometry, preview and export; changing it later changes how saved-looking collages render and their tests.
- **D-02:** Corner radius scales by the same rule as spacing (relative to cell size), so 25 small cells do not turn into circles. The existing clamp to half the cell edge stays.
- **D-03:** Scaling only starts at **more than 4 cells in one row or column** (5+). Every layout with at most 4 cells per line — i.e. all current 1-9 templates (they have at most 4 per line, `MAX_PER_LINE = 4`) — renders pixel-identical to before. Host tests must prove 1-9 geometry is unchanged.
- **D-04:** The stored `Style` values (slider positions, `MAX_SPACING` 0.05, `MAX_PADDING` 0.1, `MAX_CORNER_RADIUS` 0.1) stay as they are; only the effective values used for layout/render are derived.

### Dividers with small cells (discussed)
- **D-05:** Divider handles keep today's behaviour: one 48 px square handle per divider, on overlap the handle whose centre is nearest to the finger wins. No hiding of handles and no row/column-wide dragging. Verify on the emulator with 25 images that dividers are still grabbable; `MIN_CELL_SIZE` (5 %) stays.

### Claude's Discretion
- **Upper limit:** "no fixed limit" as the goal; the practical cap is the system Photo Picker's own maximum (`MediaStore.getPickImagesMaxLimit()`, typically 100). The app asks the picker for that many; the core accepts any count >= 1. Desktop uses the same cap for CLI arguments. If more paths still arrive than the cap, the existing cut-and-hint path stays (with the new number). Researcher/planner may lower the cap if memory or `MIN_CELL_SIZE` geometry (cells must fit at 5 %) proves it necessary; record the reason.
- **Templates for 10+ images:** even grids only (rows and columns, most balanced shapes first, as `grid_shapes`/`lines` do today), a handful of variants (about 4-6) in the template bar; an incomplete last line uses wider equal cells (no gaps), as the existing `lines()` does. No "big image + rest" variants above 9. The 4x4 caps (`MAX_LINES`, `MAX_PER_LINE`) are lifted only for counts above 9 so 1-9 outputs stay identical.
- **Memory:** preview size scales down with the image count so the total preview memory stays roughly at the 9-image level (9 x `PREVIEW_MAX_EDGE` 2048 today); exact formula is the planner's choice. Export keeps 3000 px and decodes sources one at a time as today.
- **Test sizes:** emulator checks with 12 and 25 images (load, dividers, pan/zoom, swap, export, peak memory via `run-as … /proc/<pid>/status`); host tests over a range including 10, 12, 16, 25, 50 and the cap.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements and plan
- `.planning/REQUIREMENTS.md` — MANY-01, MANY-02, MANY-03
- `.planning/ROADMAP.md` — Phase 8 goal and success criteria
- `docs/PLAN.md` — milestone 8 (Raster-Generator, `MAX_IMAGES` anheben)

### Design and rules
- `docs/ARCHITECTURE.md` — collage units, limits table (`MAX_IMAGES`, `PREVIEW_MAX_EDGE`, `MIN_CELL_SIZE`, `MAX_SPACING`), Platform trait (`pick_images(max)`)
- `docs/CODING_GUIDELINES.md` — tests (render tests <= 64 px), no unwrap/expect, Kotlin scope
- `CLAUDE.md` — commands, agent workflow (rust-coder writes code, code-reviewer at milestone end), no commits without request

### Prior phase results
- `.planning/phases/06-polish-and-v1-acceptance/06-08-SUMMARY.md` — peak memory with 9 images (489/579 MiB)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/collage-core/src/templates.rs`: `templates_for`, `grid_shapes`, `lines`, `even` — the grid generator to extend; caps `MAX_PER_LINE = 4`, `MAX_LINES = 4`.
- `crates/collage-core/src/layout.rs`: `LayoutNode` binary split tree, `MIN_CELL_SIZE = 0.05`, divider extraction.
- `crates/collage-core/src/collage.rs`: `Style` (spacing 0.01, padding 0.01 default; clamps), geometry used by both preview and export.
- `crates/dimuli-app/src/images.rs`: `PREVIEW_MAX_EDGE = 2048`, preview downscale.
- `crates/dimuli-app/ui/template-bar.slint`: scrolling template thumbnails.

### Established Patterns
- One shared function for anything preview and export must agree on (crop) — apply the same to effective style.
- Host-testable core; render tests at <= 64 px.

### Integration Points
- `MAX_IMAGES` used in `templates.rs`, `state.rs` (cut + `texts::too_many_images`), `controller.rs` (`platform.pick_images(MAX_IMAGES)`), tests in `render.rs`/`view.rs`/`state.rs`.
- Android: `MainActivity` picker `EXTRA_PICK_IMAGES_MAX` comes from the `max` argument.

</code_context>

<specifics>
## Specific Ideas

- 1-9 must be pixel-identical to the current build (owner priority).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 08-more-than-9-images*
