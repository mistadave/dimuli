# Phase 8: More Than 9 Images - Research

**Researched:** 2026-11-15
**Domain:** Rust layout engine (split tree, grid templates), Slint view models, Android Photo Picker limit, preview memory
**Confidence:** HIGH for code facts and geometry (read and simulated this session); MEDIUM for emulator memory and timing (extrapolated from phase 6 figures)

> **Superseded by the owner's decisions after this research (08-CONTEXT, 2026-11-15):** D-06 sets the cap to **36** (`MAX_IMAGES = 36`, 6x6), not 100; D-07 accepts the 16:9 handle occlusion at 25 images; D-08 puts the loading overlay right after the picker into 08-03. The plans test 10/12/16/25/36 (preview edge 1024 px at 36, cut at 37 paths with the hint naming 36). Figures below for `MAX_IMAGES = 100`, the 614 px preview edge, 49/50/64/81/100 images and the 101-path cut stay only as history.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Spacing, padding and corners with many cells (discussed)
- **D-01:** The style sliders keep their meaning ("equally airy / equally round") at every image count: the *effective* spacing, padding and corner radius shrink automatically when a layout has many cells per row/column, so cells never disappear. Preview and export use the same effective values (one shared function in collage-core, like the shared crop function). — **Reversibility:** costly — the effective-style rule feeds geometry, preview and export; changing it later changes how saved-looking collages render and their tests.
- **D-02:** Corner radius scales by the same rule as spacing (relative to cell size), so 25 small cells do not turn into circles. The existing clamp to half the cell edge stays.
- **D-03:** Scaling only starts at **more than 4 cells in one row or column** (5+). Every layout with at most 4 cells per line — i.e. all current 1-9 templates (they have at most 4 per line, `MAX_PER_LINE = 4`) — renders pixel-identical to before. Host tests must prove 1-9 geometry is unchanged.
- **D-04:** The stored `Style` values (slider positions, `MAX_SPACING` 0.05, `MAX_PADDING` 0.1, `MAX_CORNER_RADIUS` 0.1) stay as they are; only the effective values used for layout/render are derived.

### Dividers with small cells (discussed)
- **D-05:** Divider handles keep today's behaviour: one 48 px square handle per divider, on overlap the handle whose centre is nearest to the finger wins. No hiding of handles and no row/column-wide dragging. Verify on the emulator with 25 images that dividers are still grabbable; `MIN_CELL_SIZE` (5 %) stays.

### Claude's Discretion
- **Upper limit:** "no fixed limit" as the goal; the practical cap is the system Photo Picker's own maximum (`MediaStore.getPickImagesMaxLimit()`, typically 100). The app asks the picker for that many; the core accepts any count >= 1. Desktop uses the same cap for CLI arguments. If more paths still arrive than the cap, the existing cut-and-hint path stays (with the new number). Researcher/planner may lower the cap if memory or `MIN_CELL_SIZE` geometry (cells must fit at 5 %) proves it necessary; record the reason. *(Superseded by D-06: cap 36.)*
- **Templates for 10+ images:** even grids only (rows and columns, most balanced shapes first, as `grid_shapes`/`lines` do today), a handful of variants (about 4-6) in the template bar; an incomplete last line uses wider equal cells (no gaps), as the existing `lines()` does. No "big image + rest" variants above 9. The 4x4 caps (`MAX_LINES`, `MAX_PER_LINE`) are lifted only for counts above 9 so 1-9 outputs stay identical.
- **Memory:** preview size scales down with the image count so the total preview memory stays roughly at the 9-image level (9 x `PREVIEW_MAX_EDGE` 2048 today); exact formula is the planner's choice. Export keeps 3000 px and decodes sources one at a time as today.
- **Test sizes:** emulator checks with 12 and 25 images (load, dividers, pan/zoom, swap, export, peak memory via `run-as … /proc/<pid>/status`); host tests over a range including 10, 12, 16, 25, 50 and the cap. *(Per D-06: 10, 12, 16, 25 and the cap 36.)*

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.

### Specific Ideas
- 1-9 must be pixel-identical to the current build (owner priority).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MANY-01 | A grid generator in `templates` produces a valid layout (no overlaps, correct spacing) for any image count above 9, in addition to the existing templates for 1-9 | "Grid generator for 10+" (two most balanced line counts, max 6 templates, legacy branch untouched for 1-9); "Effective style rule" (gaps shrink by `4/n` from 5 cells per line); 1-9 identity proof via frozen legacy copy; validity test matrix (counts 10, 12, 16, 25, 50, 100 x 4 formats x 2 styles; per D-06 every count 10..=36) |
| MANY-02 | The fixed `MAX_IMAGES` limit of 9 is raised or removed (goal: no fixed limit, cells simply get smaller): picking and loading accept more than 9 images and the "too many images" hint no longer appears | "Cap": `MAX_IMAGES` stays one constant, value 100 = AOSP `PICK_IMAGES_MAX_LIMIT` (superseded by D-06: value 36); Kotlin already clamps to `getPickImagesMaxLimit()`; desktop cut-and-hint stays; preview edge scales with count |
| MANY-03 | Memory use and usability with many images are verified in the emulator (loading, dividers, pan/zoom, swap and export remain operable and stable) | "Memory" (constant preview budget, one decode at a time), "Touch geometry" (handle coverage table), emulator procedure for 12 and 25 images with VmHWM, divider-grabbable host test |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Code is always written by the `rust-coder` subagent (also small changes); the main agent plans, coordinates, checks. Very complex tasks are split into small, clearly bounded subtasks. (CLAUDE.md, memory "rust-coder für Code")
- Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` before finishing a change; fix all findings.
- Android lint on Android code changes: `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`.
- Logic (layouts, cell geometry, scaling, export) stays free of Android dependencies and host-testable; Android code stays encapsulated in `platform/` and Kotlin.
- UI changes are checked on the desktop first (`cargo run -p dimuli-app -- …`); Android build only for platform-specific changes.
- No `unwrap()`/`expect()`/`panic!`/out-of-bounds indexing in production code.
- No new crates (or Gradle deps) without asking the owner. This phase needs none.
- Run the `code-reviewer` subagent at the end of the milestone (before committing new functionality); fix critical findings.
- Never commit or push without an explicit request.
- Code and comments in English, communication with the owner in German; user-facing texts German and collected in `texts.rs` / `ui/texts.slint`.
- From `docs/CODING_GUIDELINES.md`: render tests use images <= 64 px generated in the test; `f32` comparisons in tests with tolerance (an exact `==` is fine where bit-identity is the claim being tested); newtypes for ids; doc comments with units on every `pub` item in `collage-core`; boundary cases tested (1 image, max image count, extreme aspect ratios, spacing 0); no allocations per frame in gesture handlers where avoidable; Kotlin changes are only covered by emulator/device tests, so they must be exercised in the emulator.
- Memory rule "device first": picker, gallery and share automation only on emulators (`dimuli_api33/36/37`); the attached phone (8200fcc9) only for smoke tests. Never send scroll or mouse-wheel input to dimuli on Android (Slint Android backend panics on it).

## Summary

The change is small in code but has three sharp edges. First, spacing, padding and radius reach the geometry in exactly four places (`Collage::content_bounds`, `placements`, `dividers`, `drag_divider`) plus two radius consumers (`render::compose` and `view::cell_views_into`). A single `Collage::effective_style()` that scales all three lengths by `style_scale(n) = 1` for `n <= 4` and `4/n` for `n >= 5` is enough. Here `n = LayoutNode::max_cells_per_line() = max(gaps(LeftRight), gaps(TopBottom)) + 1`. With this rule the gap-to-cell ratio stays almost constant and is continuous at the 4-to-5 boundary. It depends only on the tree structure, so dragging a divider never changes it. All 69 current templates for 1-9 have at most 4 cells per line (verified by running the current code), so the factor is exactly `1.0` and 1-9 output is bit-identical.

Second, the grid generator only needs a separate branch for counts above 9. That branch takes the two line counts closest to `sqrt(count)` and keeps at most 6 templates. The existing capped branch stays byte-for-byte as it is, and a frozen test copy of today's `templates_for` proves it. Counts 10-12 then still get grids with at most 4 cells per line (no scaling).

Third, the Photo Picker limit is 100 (`PICK_IMAGES_MAX_LIMIT = 100` in AOSP MediaStore). Kotlin already clamps the request to `getPickImagesMaxLimit()`, so `MAX_IMAGES` can stay one constant (value 100) and no JNI call is needed. Memory stays at the 9-image level if the preview edge scales as `2048 * sqrt(9 / count)` (614 px at 100). The real limit at high counts is touch geometry, not memory. With 48 px handles on a 411 px phone canvas, divider handles cover more than half of the average cell from about 42 images. From 81 images, many cells cannot start a pan or swap at all. With 25 images in 16:9, 3 of 25 cells are already fully covered. This conflicts with MANY-03 at the top of the range and needs an owner decision. The cap value is one constant either way.

**Primary recommendation:**
1. Add `max_cells_per_line()` and `style_scale()`, and route all geometry through `Collage::effective_style()`.
2. Add a separate `count > 9` grid branch and freeze 1-9 with a legacy-copy test.
3. Set `MAX_IMAGES = 100` and scale the preview edge by `sqrt(9 / count)`. *(Superseded by D-06: cap 36.)*
4. Before execution ends, put the touch-coverage table in front of the owner and let them confirm 100 or pick a lower cap (36 is the highest value where every cell keeps at least 30 % free area in every format). *(Resolved: D-06, the owner chose 36.)*

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Grid templates for 10+ | collage-core `templates` | — | Pure logic, host-testable (CLAUDE.md) |
| Effective style (spacing/padding/radius scaling) | collage-core `layout` + `collage` | — | D-01: one shared function for preview and export, like `source_crop` |
| Export with effective radius | collage-core `render` | — | Already consumes `placements()`; only the radius line changes |
| Image cap (`MAX_IMAGES`) | collage-core `lib.rs` | dimuli-app controller (cut + hint), Kotlin (clamp to picker limit) | One constant, the core keeps its count check |
| Preview edge by count | dimuli-app `images` | — | Memory policy of the UI, not of the core |
| Corner radius in preview, template thumbnails | dimuli-app `view` | Slint `template-bar.slint` (display only) | View model conversion lives in `view.rs` |
| Picker request | Kotlin `MainActivity.pickImages` | Rust `AndroidPlatform::pick_images` | Already clamps to `getPickImagesMaxLimit()`; no change needed |
| Busy feedback while Kotlin copies many picks (optional) | dimuli-app `state`/`controller` | — | Rust-only; Kotlin stays untouched |

## Standard Stack

No new dependencies. Everything uses what is in the workspace today.

| Library | Version (workspace) | Role in this phase |
|---------|---------------------|--------------------|
| `image` | `0.25` (`default-features = false, features = ["jpeg", "png"]`) [VERIFIED: Cargo.toml:15] | Preview downscale (`thumbnail`), export decode/resize (unchanged) |
| `slint` | `1.18` [VERIFIED: Cargo.toml:13] | `VecModel` rows for 100 cells, 99 handles, template thumbnails |
| Rust toolchain | rustc 1.99.1 / cargo 1.99.1 [VERIFIED: `rustc --version`] | — |

**Installation:** none.

## Package Legitimacy Audit

This phase installs no external packages (no crates, no Gradle dependencies).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| — | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Photo Picker (EXTRA_PICK_IMAGES_MAX = min(MAX_IMAGES, getPickImagesMaxLimit()))
      │ URIs (<= 100)
      ▼
Kotlin copyToCache (sequential, transcode HEIC/WebP, downscale > 64 MP)
      │ nativeImagesPicked(paths, failed)            Desktop: CLI paths
      ▼                                                    │
controller.start_loading ── paths.len() > MAX_IMAGES? ── cut + texts::too_many_images(MAX_IMAGES)
      │ paths, edge = preview_max_edge(paths.len())
      ▼
images::load_in_background (one decode at a time → preview <= edge)
      │ Vec<LoadedImage>
      ▼
state.finish_loading ── templates_for(n) ──► count <= 9: legacy capped generator (unchanged)
      │                                       count > 9 : two most balanced line counts, <= 6 templates
      ▼
Collage { style (stored, slider values), layout }
      │
      ├── effective_style() = style × style_scale(layout.max_cells_per_line())
      │        │ spacing / padding / corner_radius
      │        ▼
      ├── placements() / dividers() / drag_divider() / cell_at()   (geometry)
      │        │
      │        ├──► view::cell_views_into (px, effective radius) ──► Slint CellTile ×n
      │        ├──► view::divider_views / divider_handles (48 px) ──► DividerHandle ×(n-1), press hit-test
      │        └──► render::compose (3000 px, effective radius, one decode per cell) ──► JPEG
      │
      └── style() (stored) ──► settings sliders (D-04)
```

### Recommended Change Map (files)

```
crates/collage-core/src/
├── layout.rs     # + LayoutNode::max_cells_per_line()  (uses private gaps())
├── collage.rs    # + style_scale(), Collage::effective_style(); content_bounds/placements/dividers/drag_divider use it
├── render.rs     # radius from effective_style(); fix test overflow at line 585
├── templates.rs  # count > 9 branch; frozen legacy copy in tests
└── lib.rs        # MAX_IMAGES = 100 (doc: Photo Picker limit)  [superseded by D-06: 36]
crates/dimuli-app/src/
├── images.rs     # preview_max_edge(count); load_image(path, max_edge)
├── view.rs       # effective radius; thumbnail spacing × style_scale
├── controller.rs # unchanged logic (uses MAX_IMAGES); optional busy-on-pick
└── state.rs      # tests for 12/25/100; optional Busy while picking  [D-06: 10/12/25/36; D-08: Busy while picking is in scope]
android/…/MainActivity.kt   # no change needed (optional: log the picker limit)
docs/ARCHITECTURE.md, docs/PLAN.md   # limits table, templates row, milestone 8 tick
```

### Pattern 1: Effective style from tree structure (D-01..D-04)

**Where style enters geometry today** [VERIFIED: crates/collage-core/src/collage.rs:246-279, render.rs:84-90, view.rs:259]:

```rust
// collage.rs:246-249
fn content_bounds(&self) -> Rect {
    Rect::from_size(self.size()).inset(self.style.padding)
}
// collage.rs:252-254
self.layout
    .cell_rects(self.content_bounds(), self.style.spacing)
// collage.rs:270-271
self.layout
    .dividers(self.content_bounds(), self.style.spacing)
// collage.rs:276-278
let bounds = self.content_bounds();
self.layout
    .drag_divider(bounds, self.style.spacing, id, delta)
// render.rs:84 and :90
let style = collage.style();
let radius_px = style.corner_radius * scale_x;
// view.rs:259
let radius = collage.style().corner_radius;
```

`cell_at`, `pan`, `zoom`, `set_transform` all go through `placements()`, so they follow automatically. The renderer clamps the radius to half the cell (`let radius = radius.clamp(0.0, w.min(h) as f32 / 2.0);`, render.rs:320), and so does the preview (`corner_radius_px`, view.rs:231-235). D-02's clamp therefore stays without a code change.

**"Cells per line" for an arbitrary split tree.** The tree already has the right measure: `gaps(dir)` is the "Number of gaps along `dir` on the longest path through this subtree" [VERIFIED: crates/collage-core/src/layout.rs:192-201]:

```rust
fn gaps(&self, dir: SplitDir) -> u32 {
    match self {
        LayoutNode::Cell(_) => 0,
        LayoutNode::Split(split) if split.dir == dir => {
            1 + split.first.gaps(dir) + split.second.gaps(dir)
        }
        LayoutNode::Split(split) => split.first.gaps(dir).max(split.second.gaps(dir)),
    }
}
```

So `max_cells_per_line = max(gaps(LeftRight), gaps(TopBottom)) + 1`. It counts the fullest row or column, it is exactly what drives the total gap length along an axis, and it depends only on structure (ratios do not matter). A drag therefore never changes the effective style; only a template change can.

**Formula.** `style_scale(n) = 1.0` for `n <= 4`, else `4.0 / n`; it is applied to spacing, padding and corner radius (D-01, D-02). Why `4/n`:
- Continuity at the boundary: with content width `W`, the gap-to-cell ratio at `n = 4` is `4s / (W - 3s)`. For `n >= 5` with `s' = 4s/n` it is `4s / (W - 4s(n-1)/n)`. Both are equal at `n = 4`, and the ratio only creeps toward `4s / (W - 4s)` as `n` grows. So "equally airy" holds within a few percent at every count [computed this session].
- Exact identity for `n <= 4`: branch on the integer `n` and return the stored style unchanged. Do not multiply by `1.0` and compare floats; that is also correct but less obvious and invites `clippy::float_cmp` debates.
- Padding is included because D-01 lists it. The per-format feasibility below shows it is needed: without scaling padding, Landscape at 16+ images falls below `MIN_CELL_SIZE` even at default style.

**Proof that all 1-9 templates are unaffected** [VERIFIED: ran current `templates_for` this session via a scratch binary against `crates/collage-core`]. Each template is shown as `name[cells per LR line x cells per TB line]`:

```
1: single[1x1]
2: rows-2[2x1] cols-2[1x2]
3: rows-1-2[2x2] cols-1-2[2x2] rows-2-1[2x2] cols-2-1[2x2] rows-3[3x1] cols-3[1x3] big-left-3[2x2] big-top-3[2x2]
4: rows-2-2[2x2] cols-2-2[2x2] rows-4[4x1] cols-4[1x4] rows-1-1-2[2x3] cols-1-1-2[3x2] rows-2-1-1[2x3] cols-2-1-1[3x2] big-left-4[2x3] big-top-4[3x2]
5: rows-2-3[3x2] cols-2-3[2x3] rows-3-2[3x2] cols-3-2[2x3] rows-1-2-2[2x3] cols-1-2-2[3x2] rows-2-2-1[2x3] cols-2-2-1[3x2] rows-1-1-1-2[2x4] cols-1-1-1-2[4x2] rows-2-1-1-1[2x4] cols-2-1-1-1[4x2] big-left-5[2x4] big-top-5[4x2]
6: rows-3-3[3x2] cols-3-3[2x3] rows-2-2-2[2x3] cols-2-2-2[3x2] rows-1-1-2-2[2x4] cols-1-1-2-2[4x2] rows-2-2-1-1[2x4] cols-2-2-1-1[4x2]
7: rows-2-2-3[3x3] cols-2-2-3[3x3] rows-3-2-2[3x3] cols-3-2-2[3x3] rows-3-4[4x2] cols-3-4[2x4] rows-4-3[4x2] cols-4-3[2x4] rows-1-2-2-2[2x4] cols-1-2-2-2[4x2] rows-2-2-2-1[2x4] cols-2-2-2-1[4x2]
8: rows-2-3-3[3x3] cols-2-3-3[3x3] rows-3-3-2[3x3] cols-3-3-2[3x3] rows-4-4[4x2] cols-4-4[2x4] rows-2-2-2-2[2x4] cols-2-2-2-2[4x2]
9: rows-3-3-3[3x3] cols-3-3-3[3x3] rows-2-2-2-3[3x4] cols-2-2-2-3[4x3] rows-3-2-2-2[3x4] cols-3-2-2-2[4x3]
```

The maximum is 4, so the scale is exactly 1 for every existing template.

### Pattern 2: Grid generator branch for count > 9

Today's caps and generator [VERIFIED: crates/collage-core/src/templates.rs:6-9, 48-52, 97-101]:

```rust
/// Maximum number of cells in one row/column of a grid template.
const MAX_PER_LINE: usize = 4;
/// Maximum number of rows/columns of a grid template.
const MAX_LINES: usize = 4;
...
if count == 0 || count > MAX_IMAGES {
    return out;
}
...
let mut line_counts: Vec<usize> = (1..=MAX_LINES.min(count))
    .filter(|&lines| count.div_ceil(lines) <= MAX_PER_LINE)
    .collect();
```

Recommendation:
- Keep this filter verbatim for `count <= 9`.
- For `count > 9`, take all line counts `1..=count`, use the same sort (distance to `sqrt(count)`, ties to fewer lines), and keep the first 2.
- Shapes per line count stay as today: ascending, plus descending if different, each as `rows-…` and `cols-…`. Then `out.truncate(6)` for `count > 9` only.
- `big-left/big-top` stays limited to `(3..=MAX_PER_LINE + 1)`, so it never appears above 9.

Resulting template sets (simulated with the exact `grid_shapes` ordering; `n` = cells per line):

| Count | Templates (n) | Scale |
|------:|---------------|------:|
| 10 | rows/cols-3-3-4, rows/cols-4-3-3, rows/cols-2-2-3-3 (all n=4) | 1.0 |
| 12 | rows/cols-4-4-4, rows/cols-3-3-3-3 (n=4) — 4 templates | 1.0 |
| 13 | rows/cols-3-3-3-4, rows/cols-4-3-3-3 (n=4), rows/cols-4-4-5 (n=5) | 1.0 / 0.8 |
| 16 | rows/cols-4-4-4-4 (n=4), rows/cols-5-5-6, rows/cols-6-5-5 (n=6) | 1.0 / 0.67 |
| 25 | rows/cols-5×5 (n=5), rows/cols-6-6-6-7, rows/cols-7-6-6-6 (n=7) | 0.8 / 0.57 |
| 50 | rows/cols-7-7-7-7-7-7-8, rows/cols-8-7-7-7-7-7-7, rows/cols-6-6-6-6-6-6-7-7 (all n=8) | 0.5 |
| 100 | rows/cols-10 (×10 lines, n=10), rows/cols-11 (×8 lines)-12 (n=12), rows/cols-12-11 (×8 lines) (n=12) | 0.4 / 0.33 |

The count is always 4 to 6 templates. The default (first) template is the most balanced grid; for 10 that is `rows-3-3-4`.

The default template for 10-12 images has at most 4 cells per line, so it uses the full style just like 1-9. Switching to a 5+ template visibly tightens the gaps; that is the D-03 rule working as intended.

### Pattern 3: Cap — keep one constant, value 100

> Superseded by D-06: the constant is `MAX_IMAGES = 36`. The AOSP picker limit (100) below stays relevant only as the upper bound Kotlin clamps to; 36 is below it, so every picker accepts the request.

- AOSP: `private static final int PICK_IMAGES_MAX_LIMIT = 100;` and `getPickImagesMaxLimit()` returns `PICK_IMAGES_MAX_LIMIT` [VERIFIED: android.googlesource.com platform/packages/providers/MediaProvider, main, apex/framework/java/android/provider/MediaStore.java:450 and :1010-1012]. The API was added in level 33, also in R Extensions 2 [CITED: developer.android.com/reference/android/provider/MediaStore#getPickImagesMaxLimit()].
- `EXTRA_PICK_IMAGES_MAX` "should be a positive integer greater than 1 and less than or equal to `getPickImagesMaxLimit()`, otherwise `RESULT_CANCELED` is returned." [CITED: developer.android.com MediaStore reference]
- Kotlin already handles this [VERIFIED: android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt:91-97]:
  ```kotlin
  fun pickImages(max: Int) = runOnUiThread {
      try {
          val intent = Intent(MediaStore.ACTION_PICK_IMAGES).setType("image/*")
          val limit = max.coerceAtMost(MediaStore.getPickImagesMaxLimit())
          // A value above the platform limit makes the picker return RESULT_CANCELED, and the
          // extra must be greater than 1 anyway.
          if (limit > 1) intent.putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, limit)
  ```
  The Rust side converts with `let max = i32::try_from(max).unwrap_or(i32::MAX);` (platform/android.rs:122).
- So: `pub const MAX_IMAGES: usize = 100;` in `collage-core/src/lib.rs`. Today's value is `pub const MAX_IMAGES: usize = 9;` [VERIFIED: crates/collage-core/src/lib.rs:26-27]. Nothing else changes in the pick path: `self.platform.pick_images(MAX_IMAGES);` (controller.rs:208) and `let truncated = paths.len() > MAX_IMAGES; paths.truncate(MAX_IMAGES);` (controller.rs:236-237). The device's effective limit is `min(100, getPickImagesMaxLimit())`. Desktop: more than 100 CLI paths are cut, with the hint "Es werden nur die ersten 100 Bilder verwendet." (`texts::too_many_images`, texts.rs:48-49).
- A JNI call to `getPickImagesMaxLimit()` from Rust is not worth it: Kotlin already clamps, the desktop has no such API, and the value is constant 100 in AOSP.
- Keep the core bound (`check_image_count` → `CoreError::TooManyImages`, and `templates_for` empty above `MAX_IMAGES`). It keeps tree depth and test sizes bounded and stays the single source of the cap. This deviates slightly from the discretion text "the core accepts any count >= 1"; it is recorded as A3.

### Pattern 4: Preview edge by count (constant memory budget)

Today: `pub(crate) const PREVIEW_MAX_EDGE: u32 = 2048;` (images.rs:18) and `let mut preview = downscale(decoded, PREVIEW_MAX_EDGE);` (images.rs:49) [VERIFIED].

Recommended: `preview_max_edge(count) = PREVIEW_MAX_EDGE` for `count <= 9`, else `floor(2048 * sqrt(9 / count))`. This keeps the total preview pixels at or below `9 × 2048²` for every count:

| Count | Edge (px) | Per image RGBA, 4:3 | Total |
|------:|----------:|--------------------:|------:|
| 9 | 2048 | 12.0 MiB | 108 MiB |
| 12 | 1773 | 9.0 MiB | 108 MiB |
| 25 | 1228 | 4.3 MiB | 108 MiB |
| 49 | 877 | 2.2 MiB | 108 MiB |
| 100 | 614 | 1.1 MiB | 108 MiB |

> Per D-06 the range ends at 36: edge 1024 px (`floor(2048 * sqrt(9 / 36))`); the 49 and 100 rows are history.

Preview quality stays sufficient. A cell's on-screen size also shrinks by about `1/sqrt(n)`, so the zoom headroom is unchanged. At 100 images a cell is about 39 logical px, or about 102 physical px at 2.625x. At `MAX_ZOOM` 5 the visible crop is `614/5 ≈ 123` source px, which is still at least 1:1.

The count to use is `paths.len()` at `start_loading`, an upper bound after Kotlin failures. Pass `max_edge` into `load_image`; `load_in_background` computes it from `paths.len()` before spawning the thread.

### Anti-Patterns to Avoid

- **Making `Collage::style()` return the effective style.** `AppState::update_style` reads `collage.style()`, changes one field and writes it back (state.rs:711-716). It also feeds the sliders (view.rs `Snapshot`). If `style()` returned effective values, every slider move would write back a shrunken style that keeps shrinking (D-04 violated). Keep `style()` stored and add `effective_style()`.
- **Per-axis factors** (scaling x gaps by the row count and y gaps by the column count). The layout uses one `spacing` for both axes (architecture decision 3: "Abstände sind in x und y gleich groß"). Keep one factor.
- **Scaling by image count instead of cells per line.** That would change 1-9 (D-03) and give a 12-image `4-4-4` grid smaller gaps than a 9-image `3-3-3` grid for no reason.
- **Touching the `count <= 9` branch** "to unify" the generator. Owner priority is pixel identity, so put the new logic beside it.
- **Storing the effective style in the `Collage`.** It must be derived on every call; `apply_template` changes it.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Counting cells per line | A new tree walk or template metadata | Existing private `LayoutNode::gaps(dir)` behind a new `pub fn max_cells_per_line()` | Already defines "longest path" exactly as the gap math uses it |
| Equal cells with gaps | Custom grid math | Existing `lines()` / `even()` (ratios `1/k`, gap-aware `Split::rects`) | Proven equal cells for any spacing |
| Limit of the picker | JNI call from Rust | Kotlin's existing `coerceAtMost(MediaStore.getPickImagesMaxLimit())` | Already there, no new bridge function |
| Downscaling previews | Own resampler | `DynamicImage::thumbnail` via existing `downscale()` | Already used, fast |
| Freezing 1-9 output | Hash constants of float bits | A test-only verbatim copy of today's `templates_for`/`grid_shapes` plus a "placements equal raw-style `cell_rects`" test | Readable, independent of hasher or `image` crate versions |

**Key insight:** every piece this phase needs already exists. The work is wiring one derived value through four call sites and adding a second branch next to an untouched one.

## Geometry Findings (MIN_CELL_SIZE and divider limits)

`pub const MIN_CELL_SIZE: f32 = 0.05;` [VERIFIED: layout.rs:14]. Defaults: `spacing: 0.01, padding: 0.01, corner_radius: 0.0` [VERIFIED: collage.rs:129-138]. Maxima: `MAX_SPACING: f32 = 0.05`, `MAX_PADDING: f32 = 0.1`, `MAX_CORNER_RADIUS: f32 = 0.1` [VERIFIED: collage.rs:11-15].

The table shows the smallest cell edge (collage units) over all templates for each count, with the `4/n` rule applied to spacing and padding (simulated with the same gap-aware formula). A `!` marks values below `MIN_CELL_SIZE`.

| Count | default Square/4:5/9:16 | default 16:9 | max style Square/4:5/9:16 | max style 16:9 |
|------:|------:|------:|------:|------:|
| 10, 12 | 0.2375 | 0.1281 | 0.1625 | 0.0531 |
| 16 | 0.1589 | 0.0860 | 0.1167 | 0.0438 ! |
| 25 | 0.1363 | 0.0738 | 0.1020 | 0.0395 ! |
| 50 | 0.1194 | 0.0647 | 0.0906 | 0.0359 ! |
| 64 | 0.0956 | 0.0518 | 0.0740 | 0.0302 ! |
| 81 | 0.0869 | 0.0472 ! | 0.0678 | 0.0280 ! |
| 100 | 0.0797 | 0.0433 ! | 0.0625 | 0.0260 ! |

Consequences:
- Ten rows in a 16:9 collage cannot reach 0.05 even with zero spacing and padding (`0.5625 / 10 = 0.056`). No style rule can make every format feasible at 100. The templates are aspect-agnostic, as today.
- The existing invariant test `all_templates_produce_valid_cells_in_every_format` (collage.rs, loop `for count in 1..=MAX_IMAGES`, assert `r.width >= MIN_CELL_SIZE && r.height >= MIN_CELL_SIZE` at line 709) only holds for 1-9. Keep it for `1..=9` and add a separate 10+ test:
  - No overlap, inside content, positive size, and gap equal to the effective spacing in all formats and styles.
  - `>= MIN_CELL_SIZE` in Square, 4:5 and 9:16 for both styles up to the cap.
  - `>= MIN_CELL_SIZE` in 16:9 at default style up to 64.
  - In 16:9 with large styles or above 64, only `> 0`.
- When cells already start below `MIN_CELL_SIZE` along an axis, `drag_divider` computes `lo > hi` and leaves the ratio unchanged (`if lo.is_nan() || hi.is_nan() || lo > hi { return Ok(()); }`, layout.rs:299-301). Dividers along that axis are then frozen, with no panic. Dividers along the other axis keep working. This is acceptable degraded behaviour, but tests must pin it: dragging never panics, never produces overlap, and never shrinks a cell below its starting size.

## Touch Geometry Findings (D-05 consequences)

Handles take precedence on press: `if let Some(id) = self.divider_at_pixels(x, y) { … Gesture::DraggingDivider … return; }` (state.rs:494-502), and `touch-target: 48px` (theme.slint:25) [VERIFIED]. Each divider's single handle sits at the centre of its gap. Row-divider handles of a `rows-…` grid all sit at the horizontal centre of the canvas, and column handles sit at each row's vertical centre.

The table shows the share of each cell **not** covered by any handle: a press there can start a pan, or a long-press a swap. Default style, first template, on the emulator's portrait canvas (1080x2400 at 420 dpi = 411 logical px wide [VERIFIED: `adb shell wm size/density`]; canvas area assumed 411x640).

| Count | Square: worst cell / average | 9:16: worst / avg | 16:9: worst / avg |
|------:|------:|------:|------:|
| 9 | 76 % / 89 % | 82 % / 92 % | 56 % / 80 % |
| 12 | 76 % / 85 % | 83 % / 89 % | 63 % / 75 % |
| 16 | 68 % / 80 % | 77 % / 85 % | 57 % / 69 % |
| 25 | 34 % / 67 % | 49 % / 75 % | **0 %** (3 of 25 cells fully covered) / 48 % |
| 36 | 40 % / 55 % | 45 % / 64 % | 31 % / 42 % |
| 49 | 5 % / 37 % | 4 % / 51 % | 0 % / 27 % |
| 64 | 7 % / 19 % | 18 % / 41 % | 6 % / 18 % |
| 81 | 0 % / 11 % (63 of 81 fully covered) | 0 % / 33 % | 0 % / 11 % |
| 100 | 0 % / 8 % (80 of 100 fully covered) | 7 % / 25 % | 0 % / 8 % |

What still works on a fully covered cell: pinch zoom (`pinch_started` overrides any gesture, state.rs:613-624), the swap *target* (release uses `cell_at`), and mouse-wheel zoom on desktop. What does not work: pan, and starting a swap. MANY-03 ("pan/zoom, swap remain operable") therefore fails at the top of the range. In 16:9 it fails for the centre column at 25 images already. This is a direct consequence of locked D-05, so it is an owner decision (see Open Questions 1 and 2).

## Common Pitfalls

### Pitfall 1: `u8` overflow in the existing max-images render test
**What goes wrong:** `render.rs:584-585` builds `(0..crate::MAX_IMAGES as u8)` and `solid(16, 12, [i * 20, 100, 50])` [VERIFIED]. With `MAX_IMAGES = 100`, `i * 20` overflows `u8` and the debug test panics. (With D-06's 36 it still overflows, from index 13 on.)
**How to avoid:** use a colour that cannot overflow (`[i.wrapping_mul(20), …]` or `(i % 12) * 20`) and keep the canvas at 63-64 px. Use 12 or 16 images for colour-per-cell checks (cells about 15 px). For the 100-image case, assert only success and size, because 100 cells at 63 px are about 5 px each.
**Warning signs:** `attempt to multiply with overflow` in `cargo test -p collage-core`.

### Pitfall 2: The 1-9 validity test becomes unsatisfiable when `MAX_IMAGES` rises
**What goes wrong:** `all_templates_produce_valid_cells_in_every_format` iterates `1..=MAX_IMAGES` with a `MIN_CELL_SIZE` assertion and max style. It fails for 16:9 from 13-16 images (see the table).
**How to avoid:** introduce a named test-visible constant for the classic range (`CLASSIC_MAX_IMAGES = 9`) and iterate it there; add the 10+ test with the per-format rules above. The same applies to `view.rs:731` (template previews; that test has no min-size assertion and works up to 100) and to the `state.rs` / `templates.rs` tests that loop to `MAX_IMAGES` (fine, but slower).

### Pitfall 3: Effective style forgotten in one consumer
**What goes wrong:** preview and export disagree. For example, the preview radius comes from `collage.style()` (view.rs:259) while export uses the effective radius, or `drag_divider` clamps with the raw spacing while `placements` uses the effective one, so cells jump at the clamp.
**How to avoid:** after the change, `grep -n "style()\.\(spacing\|padding\|corner_radius\)\|style\.spacing\|style\.padding\|style\.corner_radius"` over `crates/` may only hit `Style::sanitized`, the settings snapshot, `update_style` and tests. Add a test: for a 25-image collage, `cell_views` corner radius equals `corner_radius_px(effective.corner_radius, …)`, and `compose` at 64 px shows background at a corner pixel that the effective radius rounds.

### Pitfall 4: Template thumbnails turn into mush
**What goes wrong:** `const TEMPLATE_PREVIEW_SPACING: f32 = 0.06;` (view.rs:22) on a `56px` thumb with an inner square of 40 px (template-bar.slint, `Theme.spacing-s` 8 px inset). For 10 columns that gives 9 gaps × 2.4 px = 21.6 px of gaps and 1.8 px cells, drawn with `thumb-cell-radius: 2px`.
**How to avoid:** scale the thumbnail spacing with the same `style_scale(layout.max_cells_per_line())`. That is identical for 1-9; for 10 columns it gives 0.024, so cells are about 3.1 px. Optionally clamp the Slint radius `min(Theme.thumb-cell-radius, …)` so tiny cells stay squares. Thumbnails of `rows-10×10` and `cols-10×10` look identical (as `rows-3-3-3`/`cols-3-3-3` do today); that is accepted behaviour.

### Pitfall 5: No feedback while Kotlin copies many picks (and a re-pick race)
**What goes wrong:** after the picker returns, Kotlin copies and transcodes on its I/O thread (`runOnIo { copyToCache(uris) … nativeImagesPicked(…) }`, MainActivity.kt:233-236). The busy overlay only starts in Rust's `start_loading`, after the copy. With 25-100 camera photos (HEIC transcode, about 0.5-1 s each [ASSUMED]) the start screen sits idle for tens of seconds. A second tap on "Bilder wählen" opens the picker again, and its `copyToCache` runs `dir.deleteRecursively()` on `cacheDir/picked`, which can delete the files the first collage is about to use.
**How to avoid (recommended, Rust-only):** set a busy state when `pick_images` is called, and clear it on `ImagesPicked` (an empty list means cancelled) and on `PickFailed`. Kotlin answers both cases today (cancel → `nativeImagesPicked(emptyArray(), 0)`, start failure → `nativePickFailed()`), and the overlay already swallows input. Test the cancel path on the emulator. Decide in planning (Open Question 3).

### Pitfall 6: Load and export time scale linearly with the count
**What goes wrong:** both the loader and the export decode every source in full, one at a time. The export also resizes the visible region with `FilterType::CatmullRom` (render.rs:244). At 100 × 12 MP that is likely tens of seconds each on the emulator [ASSUMED]. The overlay shows only "Bilder werden geladen …" / "Collage wird erstellt …" without progress.
**How to avoid:** measure on the emulator at 12 and 25 (logcat timestamps from "picked images" to the editor, and from tapping "Speichern" to `saved to gallery: true`) and record them in the SUMMARY. A progress count is new UI and out of scope unless the owner asks.

### Pitfall 7: Emulator automation with 25 picks
**What goes wrong:** the picker grid on 1080x2400 does not show 25 thumbnails at once. The project rule forbids scroll and mouse-wheel input to dimuli.
**How to avoid:** use touch `input swipe` only while the Photo Picker is in the foreground (check with `uiautomator dump`), never `input roll`/`input scroll`. For swap use `input motionevent DOWN x y`, wait more than 500 ms, then `MOVE`/`UP` (`motionevent` and `draganddrop` exist on API 36 [VERIFIED: `adb shell input` help]). Delete earlier `p6-*` files from `/sdcard/Pictures/` and `am force-stop com.google.android.photopicker` after pushing.

### Pitfall 8: Cache disk usage
**What goes wrong:** 100 copies in `cacheDir/picked` (JPEG/PNG copied unchanged, others transcoded at quality 95) can take several hundred MB until the next pick or start.
**How to avoid:** nothing to build. Note it in ARCHITECTURE and watch emulator storage during the 25-image run.

## Code Examples

Skeletons use only names and values quoted above; new names are proposals.

### Cells per line (layout.rs)
```rust
/// Largest number of cells side by side along one axis (the fullest row or column) on any line through the
/// layout. Depends only on the tree structure, so dragging dividers never changes it.
pub fn max_cells_per_line(&self) -> usize {
    let gaps = self
        .gaps(SplitDir::LeftRight)
        .max(self.gaps(SplitDir::TopBottom));
    gaps as usize + 1
}
```

### Effective style (collage.rs)
```rust
/// Up to this many cells per line, spacing, padding and corner radius apply exactly as set (D-03).
pub const FULL_STYLE_CELLS_PER_LINE: usize = 4;

/// Factor for spacing, padding and corner radius of a layout whose fullest line has `cells_per_line` cells:
/// exactly `1.0` up to [`FULL_STYLE_CELLS_PER_LINE`], then `FULL_STYLE_CELLS_PER_LINE / cells_per_line`, so the
/// gap-to-cell ratio stays about the same at every image count.
pub fn style_scale(cells_per_line: usize) -> f32 {
    if cells_per_line <= FULL_STYLE_CELLS_PER_LINE {
        1.0
    } else {
        FULL_STYLE_CELLS_PER_LINE as f32 / cells_per_line as f32
    }
}

impl Collage {
    /// The style used for geometry, preview and export: the stored style, with spacing, padding and corner
    /// radius scaled down for layouts with more than [`FULL_STYLE_CELLS_PER_LINE`] cells per line.
    pub fn effective_style(&self) -> Style {
        let cells_per_line = self.layout.max_cells_per_line();
        if cells_per_line <= FULL_STYLE_CELLS_PER_LINE {
            return self.style; // bit-identical for every 1-9 template
        }
        let scale = style_scale(cells_per_line);
        Style {
            spacing: self.style.spacing * scale,
            padding: self.style.padding * scale,
            corner_radius: self.style.corner_radius * scale,
            background: self.style.background,
        }
    }

    fn content_bounds(&self) -> Rect {
        Rect::from_size(self.size()).inset(self.effective_style().padding)
    }
    // placements / dividers / drag_divider: replace `self.style.spacing` with `self.effective_style().spacing`
}
```

### Grid branch (templates.rs)
```rust
/// Up to this many images the original template set is used, unchanged (owner priority: pixel-identical 1-9).
const CLASSIC_MAX_IMAGES: usize = 9;
/// For more images: number of line counts tried (the most balanced ones) ...
const GRID_LINE_COUNTS: usize = 2;
/// ... and the most templates offered.
const MAX_GRID_TEMPLATES: usize = 6;

fn line_counts(count: usize) -> Vec<usize> {
    let ideal = (count as f32).sqrt();
    let mut counts: Vec<usize> = if count <= CLASSIC_MAX_IMAGES {
        (1..=MAX_LINES.min(count))
            .filter(|&lines| count.div_ceil(lines) <= MAX_PER_LINE)
            .collect()
    } else {
        (1..=count).collect()
    };
    counts.sort_by(|a, b| {
        let da = (*a as f32 - ideal).abs();
        let db = (*b as f32 - ideal).abs();
        da.total_cmp(&db).then(a.cmp(b))
    });
    if count > CLASSIC_MAX_IMAGES {
        counts.truncate(GRID_LINE_COUNTS);
    }
    counts
}
// templates_for: after the loops, `if count > CLASSIC_MAX_IMAGES { out.truncate(MAX_GRID_TEMPLATES); }`
```

### Frozen 1-9 proof (templates.rs tests)
```rust
/// Verbatim copy of `templates_for` / `grid_shapes` as of commit 5c2f109 (caps 4x4, MAX_IMAGES = 9).
/// Never edit: it is the reference that proves 1-9 templates are unchanged.
fn legacy_templates_for(count: usize) -> Vec<Template> { /* copied body */ }

#[test]
fn templates_for_one_to_nine_are_unchanged() {
    for count in 1..=CLASSIC_MAX_IMAGES {
        assert_eq!(templates_for(count), legacy_templates_for(count), "{count}");
    }
}
```
Plus, in collage.rs tests, for every template 1-9, all formats and the styles {default, max, zero, odd values}: `assert_eq!(c.effective_style(), c.style())` and `assert_eq!(c.placements().iter().map(|p| p.rect), layout.cell_rects(Rect::from_size(size).inset(style.padding), style.spacing))`, compared exactly (bit identity is the claim). The second test should be written and green **before** the effective-style change.

### Preview edge (images.rs)
```rust
/// Image counts up to this keep the full preview edge (unchanged 1-9 behaviour).
const FULL_PREVIEW_IMAGES: usize = 9;

/// Longest preview edge for a collage of `count` images: total preview pixels stay at the 9-image level.
pub(crate) fn preview_max_edge(count: usize) -> u32 {
    if count <= FULL_PREVIEW_IMAGES {
        return PREVIEW_MAX_EDGE;
    }
    let scale = (FULL_PREVIEW_IMAGES as f64 / count as f64).sqrt();
    (f64::from(PREVIEW_MAX_EDGE) * scale).floor() as u32
}
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `MAX_IMAGES = 9`, templates only for 1-9 | Grid branch for 10..=100, `MAX_IMAGES = 100` (D-06: 10..=36, `MAX_IMAGES = 36`) | "too many images" only for desktop CLI above 100 (D-06: above 36) |
| Fixed preview edge 2048 | `2048 * sqrt(9/count)` above 9 | Constant preview memory |
| Style used as stored | Stored style × `style_scale(cells per line)` | 1-9 unchanged; dense grids keep gaps and radii proportional |

## Recommended Plan Split

> History: the final plans 08-01..08-06 follow D-06 (cap 36, tests for 10/12/16/25/36), D-07 and D-08; the emulator run, the review/docs and the owner check are separate plans (08-04, 08-05, 08-06).

| Plan | Wave | Scope | Depends on |
|------|------|-------|------------|
| 08-01 Core: freeze 1-9 and add the effective style | 1 | (a) Write first, green on the current code: legacy-copy template test; "placements equal raw-style `cell_rects`" test for 1-9 in all formats and styles. (b) `max_cells_per_line`, `FULL_STYLE_CELLS_PER_LINE`, `style_scale`, `effective_style`; route `content_bounds`/`placements`/`dividers`/`drag_divider` and `render::compose` radius through it. (c) Unit tests: identity for n <= 4, `4/n` for 5, 8, 10, continuity, radius scaled and still clamped, `max_cells_per_line` for grids and big-left. | — |
| 08-02 Core: grid generator and cap | 2 | `count > 9` branch (2 line counts, at most 6 templates); `MAX_IMAGES = 100`; fix the render overflow test; split the min-size test 1..=9 vs 10..=cap; new tests for counts 10, 12, 16, 25, 50, 100 (ids 0..n pre-order, unique layouts, no overlap, gap equals effective spacing, min-size rules per format, drags never panic or overlap); render 12 or 16 images at 64 px with a colour check per cell; 100 images at 63 px renders. | 08-01 |
| 08-03 App: previews, view and cut path | 3 | `preview_max_edge` plus tests; `load_image(path, max_edge)`; effective radius in `cell_views_into`; thumbnail spacing × `style_scale`; state tests for 12, 25 and 100 (load, export sources, cut + hint at 101 with the text containing "100"); host tests for D-05 "every divider handle centre picks its own divider at 25 images on a 411 px canvas" and for the cell free-area numbers; optional busy-while-picking. Desktop check: `cargo run -p dimuli-app --` with 12, 25 and 100 generated JPEGs (desktop VmHWM via `/proc/<pid>/status`). Docs: ARCHITECTURE limits table (`MAX_IMAGES`, `PREVIEW_MAX_EDGE` row, "Grenzen für den Stil" note), templates row, desktop cut text, "Bewusst nicht in v1" line. | 08-02 |
| 08-04 Emulator, review, owner | 4 | Build `-PrustAbis=x86_64`, smoke on emulator-5554; 12 and 25 images: load, divider drag (row and column), pan, pinch, swap (motionevent), template switch, save; VmHWM/VmRSS and timings; optionally API 33/37 spot check of the picker limit. Run `code-reviewer`. Owner checkpoint: cap 100 vs lower (touch-coverage table, desktop screenshots at 49/100), 16:9 at 25. Tick `docs/PLAN.md` milestone 8 and REQUIREMENTS. | 08-03 |

08-01 and 08-02 both edit `collage.rs`/`templates.rs` tests, so keep them sequential. 08-03 depends on the new core API.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Slint on Android uses a scale factor of density/160, so the canvas is about 411 logical px on the 420 dpi emulator | Touch Geometry | Coverage percentages shift; the ranking by count stays |
| A2 | Canvas area height about 640 logical px in portrait (affects only the 9:16 column) | Touch Geometry | Minor |
| A3 | Keeping a core bound (`MAX_IMAGES = 100`, `TooManyImages`) is acceptable although the discretion text says "the core accepts any count >= 1" (RESOLVED: D-06, bounded at 36) | Pattern 3 | Owner may want the core unbounded; then move the cap to the app and drop the error variant |
| A4 | HEIC transcode about 0.5-1 s per image and full decode plus export of 100 × 12 MP takes tens of seconds on the emulator | Pitfalls 5-6 | Waiting time could be shorter or much longer; measure at 12 and 25 |
| A5 | Peak memory at 12 and 25 images stays near the 9-image figure (06-08: VmHWM 500624 kB on API 36) because the preview budget is constant and decodes are sequential | Memory | If GPU texture or Slint caches scale per image, VmHWM grows; the emulator run decides |
| A6 | 100 clipped `CellTile`s plus 99 handles render smoothly on the Android renderer (D-06: at most 36 tiles and 35 handles) | UI | Jank during drags at high counts; check at 25 on the emulator, 100 on desktop |

## Open Questions (RESOLVED)

1. **Cap value given the touch geometry (owner decision).** — RESOLVED: D-06 (36)
   - What we know: memory and layout validity allow 100. From about 42 images, handles cover more than half of the average cell on a phone; at 81-100, most cells cannot start a pan or swap (D-05 keeps handles as they are).
   - What's unclear: whether the owner accepts this at the top end (pinch zoom and swap targets still work) or prefers a lower cap.
   - Recommendation: implement with `MAX_IMAGES = 100` (owner's stated goal) as one constant. Show the coverage table and desktop screenshots at 49 and 100 in the 08-04 owner checkpoint. If the owner wants pan/swap everywhere, lower the cap to 36 (every cell keeps at least 30 % free area in all formats) and record the reason in ARCHITECTURE.

2. **16:9 with 25 images: 3 centre cells fully covered by handles.** — RESOLVED: D-07 (accepted)
   - What we know: the row-divider handles sit at the canvas centre and the rows are 42 px tall, below 48.
   - Recommendation: accept, since D-05 is locked and pinch zoom still works. Mention it in the emulator report. Moving row-divider handles off-centre would change "today's behaviour" and needs owner consent.

3. **Busy overlay while Kotlin copies picks (Pitfall 5).** — RESOLVED: D-08 (in 08-03). This is a small Rust-only change that also closes the re-pick race. Recommendation: include it in 08-03 if the planner treats it as within MANY-03 ("loading … remain operable"); otherwise record it as a follow-up.

4. **Core unbounded vs bounded (A3).** — RESOLVED: D-06 (bounded at 36). Recommendation was: keep it bounded at 100 for this phase.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | all | ✓ | rustc/cargo 1.99.1 | — |
| cargo-ndk | Android build | ✓ | `~/.cargo/bin/cargo-ndk` | — |
| Android SDK platform | Android lint/build | ✓ | android-37.0 | — |
| adb / emulator | 08-04 | ✓ | emulator-5554 running API 36, 1080x2400 at 420 dpi, MemTotal 2531836 kB | — |
| ImageMagick `convert` | Test image generation | ✓ | `/usr/bin/convert` | Generate PNGs in Rust tests |
| Real device | Smoke only (memory rule) | ✓ (8200fcc9 attached) | Android 14 | — |
| Baseline tests | — | ✓ | `cargo test --workspace`: 90 core + 133 app tests pass | — |

**Missing dependencies:** none.

## Validation Architecture

> **Note (D-06):** the owner set the cap to 36 after this research. The plans and 08-VALIDATION.md use the counts 10/12/16/25/36, the preview edge 1024 px at 36 and the cut at 37 paths with the hint naming 36. The values 50/100, 614 px, 101 paths and "100" in the map below are superseded and stay only as history.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` (`cargo test`), no extra crates |
| Config file | none (workspace `Cargo.toml`) |
| Quick run command | `cargo test -p collage-core` (core finished in 0.24 s plus build) / `cargo test -p dimuli-app` |
| Full suite command | `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --workspace` |
| Android lint | `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` |

### Phase Requirements → Test Map

> Superseded figures in this table (D-06): read 50/100 as 36, "614 at 100" as "1024 at 36", "101 paths ... '100'" as "37 paths ... '36'", and "Desktop 100 images" as "Desktop 36 images". The test names and commands in the plans are authoritative.
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| MANY-01 | 1-9 templates identical to the legacy generator | unit | `cargo test -p collage-core templates_for_one_to_nine_are_unchanged` | ❌ Wave 0 (08-01) |
| MANY-01 | 1-9 placements bit-identical to raw-style `cell_rects`; `effective_style() == style()` | unit | `cargo test -p collage-core one_to_nine` | ❌ Wave 0 (08-01) |
| MANY-01 | `style_scale`: 1 for n <= 4, `4/n` above; radius scaled and clamped | unit | `cargo test -p collage-core style_scale` | ❌ 08-01 |
| MANY-01 | Grid for 10, 12, 16, 25, 50, 100: ids 0..n, unique, 4-6 templates, no overlap, inside content, gap = effective spacing, min-size per format rule | unit | `cargo test -p collage-core many_images` | ❌ 08-02 |
| MANY-01 | Divider drags at 25 and 100 never panic, overlap or shrink below the start size | unit | `cargo test -p collage-core many_images_drag` | ❌ 08-02 |
| MANY-01 | Export of 12/16 images at 64 px: each cell centre has its colour; 100 at 63 px succeeds | unit | `cargo test -p collage-core render::tests::many` | ❌ 08-02 (fix `max_images_collage_renders`) |
| MANY-02 | `preview_max_edge`: 2048 up to 9, 1773 at 12, 1228 at 25, 614 at 100; total pixels <= 9 × 2048² | unit | `cargo test -p dimuli-app preview_max_edge` | ❌ 08-03 |
| MANY-02 | Loading 12, 25 and 100 images opens the editor without notice; 101 paths cut with the hint containing "100" | unit | `cargo test -p dimuli-app loading_` | partial (`loading_one_and_max_images_works`, `truncation_is_reported`) |
| MANY-02 | Export job collects all 100 sources | unit | `cargo test -p dimuli-app export_of_one_and_max_images_collects_all_sources` | ✅ (follows the constant) |
| MANY-03 | At 25 images on a 411 px canvas every handle centre picks its own divider (D-05) | unit | `cargo test -p dimuli-app divider_handles_stay_grabbable` | ❌ 08-03 |
| MANY-03 | Effective radius in `cell_views`; thumbnail spacing identical for 1-9 | unit | `cargo test -p dimuli-app view::tests` | ❌ 08-03 |
| MANY-03 | Emulator 12 and 25: load, divider drag, pan, pinch, swap, template switch, save; no `RustPanic`/`FATAL EXCEPTION`/`Out of memory`; VmHWM recorded | manual/scripted emulator | `adb -s emulator-5554 shell run-as ch.staeheli.dimuli cat /proc/$(adb -s emulator-5554 shell pidof ch.staeheli.dimuli)/status \| grep -E 'VmHWM\|VmRSS'` and `adb -s emulator-5554 logcat -d -s dimuli` | ❌ 08-04 |
| MANY-03 | Desktop 100 images: opens, export works, memory recorded | manual | `cargo run -p dimuli-app -- <100 jpgs>`; `grep -E 'VmHWM\|VmRSS' /proc/$(pidof dimuli-app)/status` | ❌ 08-03 |

### Sampling Rate
- **Per task commit:** `cargo test -p collage-core` or `cargo test -p dimuli-app` (whichever crate changed)
- **Per wave merge:** full suite command plus Android lint if `platform/` or Kotlin changed
- **Phase gate:** full suite green; emulator evidence for 12 and 25 (logcat, VmHWM, screenshots) under `android/build/smoke/p8-*`; owner checkpoint passed

### Wave 0 Gaps
- [ ] `templates.rs` tests: `legacy_templates_for` verbatim copy and `templates_for_one_to_nine_are_unchanged`
- [ ] `collage.rs` tests: placements of 1-9 equal raw-style `cell_rects` in all 4 formats and at least 4 styles (default, `MAX_*`, zero, odd values such as 0.037/0.071/0.063), exact equality
- [ ] Test-visible `CLASSIC_MAX_IMAGES` (9) so existing loops keep the 1-9 scope when `MAX_IMAGES` changes

Render tests stay at 64 px or below (63/64 px canvases, images 16x12).

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | No new permissions; the picker grants per-URI access |
| V5 Input Validation | yes | Image count bounded by `MAX_IMAGES` (core `TooManyImages`, controller cut); decode limits unchanged (`DECODE_MAX_PIXELS` 64 MP, `DECODE_MAX_ALLOC` 256 MiB); style values clamped by `Style::sanitized` |
| V6 Cryptography | no | — |
| V12 Files and Resources | yes | Cache copies stay app-private (`cacheDir/picked/img-<n>`); more files only increases disk use (Pitfall 8) |

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Resource exhaustion through many or huge images | Denial of service | Count cap 100 (D-06: 36), preview budget constant, one decode at a time, existing decode limits |
| Deep layout trees (stack) | Denial of service | Tree depth about 2·√100 = 20 at the cap (D-06: about 2·√36 = 12); the core bound keeps it there |

## Sources

### Primary (HIGH confidence)
- Repository source read this session: `crates/collage-core/src/{templates,layout,collage,render,lib,error}.rs`, `crates/dimuli-app/src/{images,view,state,controller,texts}.rs`, `platform/{android,desktop}.rs`, `ui/{template-bar,theme,collage-canvas,cell-tile,divider-handle}.slint`, `android/.../MainActivity.kt`
- Scratch binary against the current `collage-core`: template list and cells per line for 1-9
- AOSP MediaProvider `MediaStore.java` (main): `PICK_IMAGES_MAX_LIMIT = 100` — https://android.googlesource.com/platform/packages/providers/MediaProvider/+/refs/heads/main/apex/framework/java/android/provider/MediaStore.java
- `adb shell wm size/density`, `/proc/meminfo`, `adb shell input` help on emulator-5554 (API 36)
- `cargo test --workspace` baseline (90 + 133 pass)

### Secondary (MEDIUM confidence)
- [Android MediaStore reference](https://developer.android.com/reference/android/provider/MediaStore) — `getPickImagesMaxLimit()` (API 33, R Extensions 2), `EXTRA_PICK_IMAGES_MAX` rules
- [Photo picker guide](https://developer.android.com/about/versions/13/features/photopicker) — limit via `getPickImagesMaxLimit()`
- `.planning/phases/06-polish-and-v1-acceptance/06-08-SUMMARY.md`, `06-06-SUMMARY.md`, `06-06-FINDINGS.md` — memory baselines (VmHWM 500624 kB with 9 images; 690896 kB with seven large images)

### Tertiary (LOW confidence)
- Python simulation (session scratchpad) of the proposed rule: min cell sizes, handle coverage. It mirrors the gap-aware layout formula, but it is not the Rust code.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies.
- Architecture (effective style, generator, cap): HIGH — call sites read and quoted; 1-9 cells per line verified by running the code; picker limit verified in AOSP source.
- Geometry and touch numbers: MEDIUM — simulated, exact for even grids; canvas size assumed (A1, A2).
- Memory and timing on the emulator: MEDIUM/LOW — extrapolated; 08-04 measures them.

**Research date:** 2026-11-15
**Valid until:** 2026-12-15 (stable codebase; picker limit is a mainline constant)
