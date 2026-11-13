# Constraints (SPEC-derived)

All entries originate from docs/ARCHITECTURE.md (classified SPEC, medium confidence, not locked). Rust code blocks are reproduced as in the source; German comments are translated to English.

## Guiding design decisions (Leitentscheidungen)
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content: Intent: "Deliberately minimal: two crates, one state, one data flow. New layers or patterns only when a concrete problem demands them." Non-locked design rationale (not a formal ADR set), eight decisions:
  1. One state (`AppState`) on the UI thread; only the controller mutates it. Reason: no synchronization, one source of truth.
  2. One-way data flow: Event -> Controller -> Core -> View-Models -> UI. Reason: traceable, UI stays dumb.
  3. Core computes in collage units (1.0 = collage width). Reason: same logic for screen and export, independent of pixel sizes; spacing is equal in x and y.
  4. One crop function for preview and export. Reason: export is guaranteed to look like the preview.
  5. Layout as a binary tree of splits. Reason: each divider = one `ratio`; templates and dragging become trivial.
  6. Background work only in short-lived threads with their own data. Reason: no shared state, no async runtime needed.
  7. Platform behind one trait + result as `PlatformEvent`. Reason: desktop for development, Android only as a thin shell.
  8. No persistence in v1. Reason: collage lives only during the session; saves a storage format and migration.

## Crate structure and architecture overview
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content: Two crates. `dimuli-app` contains `ui/*.slint` -callbacks-> `Controller` -mutates-> `AppState`; View-Models flow back from Controller to UI; `platform/` (Android | Desktop) sends `PlatformEvent` to the Controller (operations: pick / save / share). The Android platform wraps a Kotlin Activity (Photo Picker, MediaStore, share, insets, back gesture). `collage-core` is pure logic: layout, geometry, crop, export renderer.

## collage-core module responsibilities
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: api-contract
- content: Pure Rust logic with no UI or platform dependency. Modules:
  - `geometry`: `Size`, `Vec2`, `Rect` (collage units, `f32`).
  - `layout`: `LayoutNode`, `Split`, compute cell rectangles, find and drag dividers (spacing-aware: equal `ratio`s yield equally sized cells).
  - `templates`: templates per image count: balanced grids (rows/columns) plus "large image left/top" for 3-5.
  - `collage`: `Collage` (overall model) and all edit operations.
  - `crop`: from image size, cell size and `CellTransform`, the visible source rectangle.
  - `decode`: `decode_unoriented(path)` (decode with limits `DECODE_MAX_*`, returns image + EXIF orientation) and `decode_image(path)` (additionally oriented); shared by export and app preview.
  - `render`: export. `compose` decodes images one at a time via `decode_unoriented`, converts the `placements()` crop back into stored pixels, downsizes directly to cell size as a view without copying, and only then orients the small cell (peak memory approx. decoded image + cell; 16-bit/float/grayscale+alpha additionally need an 8-bit RGBA copy of the visible section; sources with alpha are scaled premultiplied to avoid color fringes; corner radius via antialiased coverage mask; edge rounding of pixel rectangles without gaps). `render` does JPEG encoding. `ExportOptions { long_edge, jpeg_quality }` is clamped (`1..=MAX_EXPORT_LONG_EDGE` = 8192, quality 1-100); a background with alpha is blended over white.
  - `error`: `CoreError` (`thiserror`).

## Collage data model
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: schema
- content:
  ```rust
  pub struct Collage {
      aspect: AspectRatio,              // Square, Portrait4x5, Story9x16, Landscape16x9
      style: Style,                     // spacing, padding, corner_radius, background: Rgba
      layout: LayoutNode,
      cells: Vec<Cell>,                 // index = CellId; fields private, access via methods
  }

  pub enum LayoutNode {
      Cell(CellId),
      Split(Box<Split>),                // Split { dir: SplitDir, ratio: f32, first, second }
  }

  pub struct Cell {
      pub image: ImageMeta,             // every cell has exactly one image
      pub transform: CellTransform,
  }

  pub struct CellTransform {
      zoom: f32,                        // 1.0..=MAX_ZOOM; 1.0 = image fills cell exactly ("cover")
      center: Vec2,                     // center of the visible section in image coordinates (0..1);
                                        // stays meaningful when the cell is resized
  }
  ```
  The core knows images only as `ImageId` + pixel size (`ImageMeta { id, width, height }`), never as pixel data, except in `decode` and `render`, which receive file paths (`ImageSource { id, path }`).

## collage-core public operations
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: api-contract
- content:
  ```rust
  impl Collage {
      pub fn new(images: &[ImageMeta], template: &Template) -> Result<Self, CoreError>;
      pub fn apply_template(&mut self, template: &Template) -> Result<(), CoreError>; // images are kept
      pub fn placements(&self) -> Vec<CellPlacement>;   // per cell: rectangle + image section (for UI and export)
      pub fn dividers(&self) -> Vec<Divider>;           // drag areas for the UI
      pub fn drag_divider(&mut self, id: DividerId, delta: Vec2) -> Result<(), CoreError>; // clamps to minimum size
      pub fn pan(&mut self, cell: CellId, delta: Vec2) -> Result<(), CoreError>;
      pub fn zoom(&mut self, cell: CellId, factor: f32, focus: Vec2) -> Result<(), CoreError>;
      pub fn swap(&mut self, a: CellId, b: CellId) -> Result<(), CoreError>;
      pub fn set_aspect(&mut self, aspect: AspectRatio);
      pub fn set_style(&mut self, style: Style);       // clamps to valid values
      pub fn cell_at(&self, point: Vec2) -> Option<CellId>; // hit test for swapping
  }

  pub fn source_crop(image: Size, cell: Size, t: CellTransform) -> Rect; // crop module
  pub fn render(collage: &Collage, sources: &[ImageSource], opts: ExportOptions) -> Result<Vec<u8>, CoreError>;
  ```

## dimuli-app file responsibilities
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: api-contract
- content:
  - `lib.rs`: `run(platform)` creates the UI and wires the controller; `android_main` calls `run` with `AndroidPlatform`.
  - `main.rs`: desktop entry `run(DesktopPlatform)`; defaults to winit with software renderer (WSLg OpenGL problem); `SLINT_BACKEND` overrides.
  - `state.rs`: `AppState { screen, collage, images, busy: Option<Busy>, message, templates, canvas, insets, confirm_back, gesture }` with all state transitions (gesture state machine `Gesture`, loading, export, back, style); pure Rust, testable without a window.
  - `images.rs`: loading in a background thread (decode via `collage_core::decode_unoriented`, then shrink preview to <= `PREVIEW_MAX_EDGE` and orient; errors are `CoreError`); `ImageStore`: `ImageId -> { path, preview: slint::Image, preview_size }`; `sources(&Collage)` yields the `ImageSource`s for export.
  - `controller.rs`: thin; connects UI callbacks and `PlatformEvent`s to `AppState` transitions and then triggers `refresh`; `post()` brings actions from other threads onto the UI thread (controller discoverable via `thread_local`).
  - `view.rs`: pure conversion collage units <-> pixels (1.0 = canvas width in px), `source-clip-*`, divider touch areas (>= 48 px), wheel zoom; `Models::snapshot/apply` writes `CellView`/`DividerView`/`TemplateView` into the Slint models (changed rows only).
  - `texts.rs`: German messages generated by Rust code (texts of the `.slint` files live in `ui/texts.slint`).
  - `ui/`: `app.slint` (entry), `backend.slint` (`global Backend`: all properties/callbacks between UI and Rust), `theme.slint`, `texts.slint`, `types.slint` (structs), one component per file.
  - `platform/mod.rs`: `trait Platform`, `enum PlatformEvent`, `post_event()`.
  - `platform/desktop.rs`, `platform/android.rs`: implementations.

## Platform trait
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: api-contract
- content:
  ```rust
  pub trait Platform {
      fn pick_images(&self, max: usize);                  // -> PlatformEvent::ImagesPicked
      fn save_to_gallery(&self, jpeg: Vec<u8>);          // -> PlatformEvent::Saved
      fn share(&self, jpeg: Vec<u8>);                    // -> PlatformEvent::Shared
      fn close_app(&self);                               // back on the start screen closes the app
  }
  ```
  All methods return immediately. Results arrive asynchronously as `PlatformEvent` via `slint::invoke_from_event_loop` at the controller.

## PlatformEvent and delivery
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content:
  ```rust
  pub enum PlatformEvent {
      ImagesPicked(Vec<PathBuf>),                        // copies in app cache; empty = cancelled
      PickFailed(String),                                // user message (e.g. desktop without arguments)
      Saved(Result<(), String>),
      Shared(Result<(), String>),
      InsetsChanged { top: f32, bottom: f32, left: f32, right: f32 },
      BackPressed,
  }
  ```
  The platform delivers results with `platform::post_event(event)` (any thread); this uses `invoke_from_event_loop` and finds the controller via a `thread_local`.

## Desktop platform behavior
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: api-contract
- content: `DesktopPlatform::pick_images` returns the paths passed at startup (`cargo run -p dimuli-app -- a.jpg b.jpg`, all paths; the controller truncates to `MAX_IMAGES` and shows a hint; without arguments `PickFailed`). `max` is only a hint. `close_app` ends the Slint event loop on desktop. `save_to_gallery` writes `dimuli-<unix-seconds>.jpg` into the working directory (never overwrites: on collision `-1`, `-2`, ...; full path is logged). `share` reports "not available".

## Gesture protocol
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content: The canvas `TouchArea` reports only raw pointer data in canvas pixels (`canvas-pressed/moved/released/long-pressed/scrolled/pinched`). Rust decides meaning via `Gesture` (`Idle -> Pressed -> Panning | Swapping`, `Pinching`) and `cell_at`. Long-press (500 ms) is a Slint `Timer`; movement over 10 px before expiry turns the press into a pan. Mouse wheel -> `zoom` under the pointer; pinch (`ScaleRotateGestureHandler`) -> `zoom` around the gesture center. Dividers have no own `TouchArea`: each has exactly one handle with a square target area (edge length `Theme.touch-target`, 48 px, centered on the gap). On `press` Rust checks the handles first (on overlap the nearest center wins) and starts `Gesture::DraggingDivider`; otherwise cell logic runs. Handles in `.slint` are presentation only.

## Back navigation behavior
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content: Back in the editor -> start screen (with confirmation prompt, since the collage is lost). Back on the start screen (also while loading) is not handled (`BackOutcome::NotHandled`); the controller then calls `platform.close_app()`. During export the overlay swallows all input and the back gesture is `Handled` (no effect).

## Export flow
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: protocol
- content: `AppState::begin_export` (only in the editor and when not `busy`) sets `busy = Exporting`, ends running gestures and returns an `ExportJob` (cloned `Collage` + `ImageSource`s from the `ImageStore`) -> thread `exporter`: `render()` (panic becomes an error via `catch_unwind`) -> `post` -> `AppState::finish_export` ends `busy` (end of the loading state is the end of rendering, not the platform's reply) -> on success `platform.save_to_gallery(bytes)` or `share(bytes)` -> `Saved/Shared` -> message. On error the `CoreError` is logged and a German message is shown (source image missing/unreadable vs. general).

## v1 limits and constants
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md
- type: nfr
- content:
  - `MAX_IMAGES` = 9 (templates exist for 1-9; more images via grid generator in milestone 8)
  - `PREVIEW_MAX_EDGE` = 2048 px (limit UI memory)
  - `EXPORT_LONG_EDGE` = 3000 px (good quality at reasonable file size)
  - `DECODE_MAX_PIXELS` = 100 MP (protection against manipulated/huge images)
  - `MIN_CELL_SIZE` = 0.05 (a cell cannot become narrower than 5 % of collage width when dragging)
  - `MAX_SPACING` / `MAX_PADDING` / `MAX_CORNER_RADIUS` = 0.05 / 0.1 / 0.1 (style limits so 9 cells stay visible)
  - `MAX_ZOOM` = 5.0 (sensible upper limit for pinch zoom)
  - Export format = JPEG, quality 90 (gallery- and share-compatible)
  - `MAX_EXPORT_LONG_EDGE` = 8192 (clamp for `ExportOptions.long_edge`, see `render` module)
