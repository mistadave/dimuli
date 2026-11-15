---
phase: 06-polish-and-v1-acceptance
plan: 01
subsystem: collage-core decode / app image loading
tags: [decode-limits, memory, preview, slint, render-tests, perf]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 08
    provides: "Phase 5 code base (HEAD ff11541), shared decoder decode_unoriented used by preview and export"
provides:
  - "decode.rs: DECODE_MAX_PIXELS = 64_000_000 (D-08), DECODE_MAX_ALLOC = 256 MiB (D-17), explicit edge check and decoder.total_bytes() check before from_decoder, ImageError::Limits -> ImageTooLarge; every limit violation is CoreError::ImageTooLarge (D-18)"
  - "images.rs: preview_buffer(&DynamicImage) + fill_rgba write RGB/RGBA previews once into the SharedPixelBuffer (M3); load_image uses it"
  - "render.rs: all render tests at <= 64 px output (M4)"
affects: [06-02, 06-03, 06-04, 06-06, 06-07]

# Actuals (#2632) - chars/4 over the realized code diff (git diff -- crates/, 15,933 chars incl. context)
actuals:
  tokens: 4000
  tasks: 3
  commits: 0
plan_head_before: ff11541208a5b6e669342453d41347f14e5b5d7b
plan_head_after: ff11541208a5b6e669342453d41347f14e5b5d7b

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Decode limits checked in our own code right after decoder.dimensions() (edge, pixels, total_bytes), so every violation maps to one error variant before any pixel allocation; reader Limits keep only max_alloc as a backstop"
    - "Single-pass conversion into Slint buffers via as_chunks::<N>() zipped with make_mut_slice(); no copy_from_slice, no indexing"

key-files:
  created: []
  modified:
    - crates/collage-core/src/decode.rs
    - crates/dimuli-app/src/images.rs
    - crates/collage-core/src/render.rs

key-decisions:
  - "D-08/D-17/D-18 implemented as planned: 64 MP pixel budget, 256 MiB decoded-buffer limit, 16384 px edge reported as ImageTooLarge instead of ImageDecode"
  - "images.rs test oversized_image_is_rejected_before_decoding now expects ImageTooLarge for 20000x20000 (consequence of D-18, not a scope change)"

patterns-established:
  - "Header-only PNG test helper takes bit depth and colour type (png_header_only_with), so allocation limits can be tested without pixel data"

requirements-completed: []  # Plan declares [PERF-01, POLI-02]; both are shared with later plans (PERF-01: 06-03/06-04/06-06/06-08 gestures, HEIC/large picks, device memory; POLI-02: desktop share early-out in 06-03), so neither is marked complete here.

coverage:
  - id: D1
    description: "> 64 MP, edge > 16384 px or decoded buffer > 256 MiB are rejected from the header as ImageTooLarge; exactly 64 MP and an edge of exactly 16384 px pass the limits"
    requirement: PERF-01
    verification:
      - kind: unit
        ref: "cargo test -p collage-core --lib decode -> 12 passed (pixel_budget_allows_exactly_64_megapixels, pixel_budget_exceeded_by_one_column_is_image_too_large, pixel_budget_exceeded_is_image_too_large, edge_beyond_limit_is_image_too_large, edge_at_limit_is_accepted, decoded_buffer_beyond_alloc_limit_is_image_too_large, zero_sized_image_is_an_error, ...)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Preview and export inherit the limits through decode_unoriented (key links)"
    requirement: PERF-01
    verification:
      - kind: unit
        ref: "images::tests::oversized_image_is_rejected_before_decoding (20000x20000 and 10100x10000 -> ImageTooLarge); grep decode_unoriented in images.rs (2) and render.rs (2); render suite 21 passed"
        status: pass
    human_judgment: false
  - id: D3
    description: "RGB/RGBA previews are written once into the Slint pixel buffer; other formats via to_rgba8 fallback; empty images rejected"
    requirement: PERF-01
    verification:
      - kind: unit
        ref: "cargo test -p dimuli-app --lib images::tests::preview_buffer -> 4 passed; images::tests::loads_png_with_source_colours passed; clone_from_slice count 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "No collage-core render test renders above 64 px; same test count as before"
    requirement: POLI-02
    verification:
      - kind: static
        ref: "grep -nE 'opts\\((6[5-9]|[7-9][0-9]|[1-9][0-9]{2,})\\)' render.rs | wc -l -> 0"
        status: pass
      - kind: unit
        ref: "cargo test -p collage-core --lib render -> 21 passed (21 #[test] at HEAD and now)"
        status: pass
    human_judgment: false

# Metrics
duration: 1min (verification pass only; implementation by rust-coder not timed)
completed: 2026-11-14
status: complete
---

# Phase 6 Plan 01: Decode Limits, Single-Copy Previews, Small Render Tests Summary

**The shared decoder now refuses images above 64 MP, with an edge above 16,384 px or with a decoded buffer above 256 MiB, always as `CoreError::ImageTooLarge` and before any pixel allocation (D-08, D-17, D-18). App previews of RGB/RGBA sources are written once into the Slint pixel buffer (`preview_buffer`, M3), and every collage-core render test stays at or below 64 px (M4). Verified independently: all plan gates and acceptance criteria pass.**

## Performance

- **Duration:** about 1 min for this verification pass (2026-11-14T20:14:40Z to 20:15:16Z, warm cargo cache). Implementation by rust-coder was done beforehand and not timed here.
- **Tasks:** 3 of 3
- **Files:** 0 created, 3 modified

## Accomplishments

- `decode.rs`: `DECODE_MAX_PIXELS = 64_000_000` and `DECODE_MAX_ALLOC = 256 * 1024 * 1024` with reasons in the doc comments; `max_image_width`/`max_image_height` removed from the reader `Limits` (only `max_alloc` stays); after the zero-size check one combined check returns `ImageTooLarge { width, height }` for edge > 16384 or pixels > 64 MP; then `decoder.total_bytes() > DECODE_MAX_ALLOC` -> `ImageTooLarge`; `from_decoder` errors of kind `ImageError::Limits` -> `ImageTooLarge`, all others stay `ImageDecode`.
- `decode.rs` tests: helper generalized to `png_header_only_with(w, h, bit_depth, colour_type)` + `decode_bytes`; new `edge_at_limit_is_accepted`, `pixel_budget_allows_exactly_64_megapixels`, `pixel_budget_exceeded_by_one_column_is_image_too_large`, `decoded_buffer_beyond_alloc_limit_is_image_too_large` (16-bit RGBA 6000x6000 = 288,000,000 bytes); `edge_beyond_limit_is_rejected_as_decode_error` renamed to `edge_beyond_limit_is_image_too_large` with exact variant/size assertions for 16385x1 and 1x16385.
- `images.rs`: `preview_buffer` (zero-size check -> `InvalidImageSize`, one `SharedPixelBuffer::new`, RGB via `as_chunks::<3>()` + alpha 255, RGBA and fallback via `fill_rgba` with `as_chunks::<4>()`, length mismatch -> `InvalidImageSize`); `load_image` calls it after `apply_orientation`, the intermediate `into_rgba8()` image and `clone_from_slice` are gone. Tests `preview_buffer_adds_opaque_alpha_to_rgb`, `preview_buffer_keeps_rgba_alpha`, `preview_buffer_converts_other_formats`, `preview_buffer_rejects_empty_images`, `loads_png_with_source_colours`.
- `render.rs` (tests only): `two_solid_images_fill_their_cells` 200 -> 64, `zero_spacing_leaves_no_gap_between_cells` 101 -> 63, `corner_radius_rounds_cell_corners` 200 -> 64 (probes (0,0), (63,63), (32,32), (32,0), 8x8 antialias block), `render_outputs_decodable_jpeg_of_expected_size` 100 -> 60 with (48, 60), `max_images_collage_renders` 90 -> 63. No comments quoting old sizes.

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `ff11541`, `git rev-list --count ff11541..HEAD` = 0).

Suggested commit message when the owner asks to commit (alone or together with later Phase 6 plans):

```
Tighten decode limits and copy previews once (phase 6, plan 01)

- DECODE_MAX_PIXELS 64 MP and DECODE_MAX_ALLOC 256 MiB; edge,
  pixel and decoded-buffer violations are all reported as
  ImageTooLarge before any pixel allocation
- previews of RGB/RGBA images are written once into the Slint
  pixel buffer (preview_buffer) instead of via an RGBA copy
- render tests use output sizes of at most 64 px
```

## Files Created/Modified

- `crates/collage-core/src/decode.rs`: constants, explicit edge/pixel/`total_bytes` checks, `ImageError::Limits` mapping, 4 new tests + 1 renamed
- `crates/dimuli-app/src/images.rs`: `preview_buffer`, `fill_rgba`, `load_image` wiring, 5 new tests; existing test `oversized_image_is_rejected_before_decoding` now expects `ImageTooLarge` for 20000x20000 (D-18 consequence, noted by the caller)
- `crates/collage-core/src/render.rs`: test sizes only (5 tests)
- No other code file changed (`git diff --stat -- crates/` lists exactly these three); `Cargo.toml`, `Cargo.lock` and the crate manifests are unchanged (T-06-SC).

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| T1 `cargo test -p collage-core --lib decode` | PASS: 12 passed, 0 failed (incl. the 4 named new tests and the renamed edge test) |
| T2 `cargo test -p dimuli-app --lib images::tests::preview_buffer` | PASS: 4 passed |
| T3 render grep gate `test "$(grep -nE 'opts\((6[5-9]\|[7-9][0-9]\|[1-9][0-9]{2,})\)' render.rs \| wc -l)" -eq 0` | PASS (exit 0; 0 matches) |
| `cargo test -p collage-core --lib render` | PASS: 21 passed (21 `#[test]` at HEAD, 21 now; none removed) |
| `cargo test -p dimuli-app --lib images` | PASS: 30 passed (incl. `oversized_image_is_rejected_before_decoding`, `loads_png_with_source_colours`) |
| `cargo fmt --all --check` | PASS (exit 0, no "Diff in") |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0, 0 `warning`/`error` lines) |
| `cargo test --workspace` | PASS: 90 + 109 passed, 0 failed |

Acceptance greps, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `DECODE_MAX_PIXELS: u64 = 64_000_000` in decode.rs | >= 1 | 1 |
| `DECODE_MAX_ALLOC: u64 = 256 * 1024 * 1024` in decode.rs | >= 1 | 1 |
| `max_image_width` in decode.rs | 0 | 0 |
| `total_bytes` in decode.rs | >= 1 | 1 |
| `fn edge_beyond_limit_is_image_too_large` in decode.rs | >= 1 | 1 |
| `fn preview_buffer` in images.rs | >= 1 | 5 (fn + 4 test fns with that prefix) |
| `clone_from_slice` in images.rs | 0 | 0 |
| oversized `opts(...)` in render.rs | 0 | 0 |
| `opts(64)` in render.rs (artifact `contains`) | >= 1 | 9 |
| `decode_unoriented` in images.rs / render.rs (key links) | >= 1 each | 2 / 2 |

Test counts by file (`#[test]`): decode.rs 8 -> 12, images.rs 17 -> 22, render.rs 21 -> 21.

## Must-Have Truths (checked against the diff)

| Truth | Evidence | Result |
|-------|----------|--------|
| > 64 MP, edge > 16384 or buffer > 256 MiB rejected before pixel decode, always `ImageTooLarge` | Checks sit between `dimensions()` and `from_decoder`; tests 8001x8000, 10100x10000, 16385x1, 1x16385, 16-bit RGBA 6000x6000 assert `ImageTooLarge` (exact sizes where the plan asks for them); app loader test 20000x20000 -> `ImageTooLarge` | PASS |
| Exactly 8000x8000 and an edge of exactly 16384 still pass | `pixel_budget_allows_exactly_64_megapixels`, `edge_at_limit_is_accepted` (result is anything but `ImageTooLarge`; the stub IDAT ends in `ImageDecode`) | PASS |
| RGB/RGBA previews built with a single copy into the Slint buffer | `preview_buffer` matches `ImageRgb8`/`ImageRgba8` and writes straight into `make_mut_slice()`; `into_rgba8()` and `clone_from_slice` removed from `load_image` | PASS |
| No render test > 64 px; fmt/clippy/test green | grep gate 0, gates above | PASS |

Production-code rules: no `unwrap`/`expect`/`panic!` outside the `mod tests` blocks in decode.rs (tests start line 74) and images.rs (line 224); no indexing in the new production code; no new crate or feature.

## Decisions Made

- PERF-01 and POLI-02 are not marked complete: PERF-01 also needs gesture work (06-03), large/HEIC picks (06-04) and device memory checks (06-06/06-08); POLI-02 also needs the desktop "Teilen" early-out (06-03).

## Deviations from Plan

**1. [Expected consequence, caller-noted] `images.rs` test `oversized_image_is_rejected_before_decoding` changed**
- The 20000x20000 case used to expect `ImageDecode` (reader edge limit). With D-18 it now expects `ImageTooLarge`. The test lies in a planned file; the change is required by the plan's truth and is not a scope change.

**2. [Observation, no defect] Length checks use truncating division**
- `preview_buffer`/`fill_rgba` compare `raw.len() / 3` (or `/ 4`) with `target.len()`. A source buffer 1-2 bytes longer than `width * height * channels` would pass and the remainder would be ignored by `as_chunks`. It cannot panic and cannot happen with `image::ImageBuffer` (exact length by construction). The plan text ("if the source length does not match `width * height * channels` return `InvalidImageSize`") is met in spirit; an exact `raw.len() != target.len() * 3` comparison would be stricter. Candidate for the 06-07 code review, not a FIX_REQUEST.

**Total deviations:** 2 (1 expected test adjustment, 1 minor observation). No scope creep, no behaviour change beyond the plan.

## Issues Encountered

- `docs/ARCHITECTURE.md:200` still lists `DECODE_MAX_PIXELS | 100 MP`. Expected: the plan defers the docs entry to 06-07.
- `as_chunks` needs Rust 1.88+; the workspace has no `rust-version` pin and the toolchain here is 1.99.1, so this is fine, but worth knowing if an MSRV is ever set.

## Threat Model Check

- T-06-01 (DoS, decode_unoriented): mitigated. Pixel budget, edge limit and `total_bytes` check run before `from_decoder`; each limit has a test.
- T-06-02 (Tampering, preview_buffer): mitigated. Zero-size check before allocation, `zip` over `as_chunks` (no slice copies, no indexing), mismatch -> `InvalidImageSize`; the loader keeps its per-image `catch_unwind`.
- T-06-SC: no manifest or lock file changed (`git diff HEAD --stat` on Cargo.toml/Cargo.lock/crate manifests is empty).
- No new security surface beyond the threat model.

## Follow-ups / Open Items

- 06-02: show the German "too large" text for edge violations (now classified as `ImageTooLarge`; `texts.rs:44` already groups `ImageTooLarge`).
- 06-07: update `docs/ARCHITECTURE.md` (64 MP, 256 MiB); review the truncating length check (Deviation 2).
- Still open from Phase 5: final device test (Android 14 + 16), code review before commit, owner commit of Phase 5 and Phase 6 work.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 06-02 (messages and lifecycle). Working tree green; HEAD unchanged at `ff11541`.

---
*Phase: 06-polish-and-v1-acceptance*
*Completed: 2026-11-14*

## Self-Check: PASSED

All 3 modified code files exist and contain the planned artifacts; 0 commits since ff11541 (as required, no commits).
