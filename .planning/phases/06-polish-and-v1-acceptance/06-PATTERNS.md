# Phase 6: Polish and v1 Acceptance - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 19 (all modifications of existing files; no new source files)
**Analogs found:** 19 / 19 (each file is its own analog: copy the pattern of the neighbouring function in the same file)

All paths are git-tracked sources. Line numbers refer to the state at commit `ff11541`.

## File Classification

| Modified File | Role | Data Flow | Closest Analog (in-file / sibling) | Match Quality |
|---|---|---|---|---|
| `crates/collage-core/src/decode.rs` | utility (limits) | transform | constants lines 12-17 | exact |
| `crates/collage-core/src/render.rs` (tests only) | test | batch | test `two_solid_images_fill_their_cells` ~line 422 | exact |
| `crates/dimuli-app/src/images.rs` | service (loader thread) | file-I/O / transform | `load_image` lines 43-71 | exact |
| `crates/dimuli-app/src/state.rs` | store (pure state machine) | event-driven | `show_error`/`show_notice` 155-167, `finish_loading` 225-266, `begin_export` 272-289, `load_notice` 671-683 | exact |
| `crates/dimuli-app/src/view.rs` | transform (state -> Slint models) | request-response | `Models::snapshot/apply` 333-434, `sync_model` 437-447 | exact |
| `crates/dimuli-app/src/controller.rs` | controller | event-driven | `install` 50-64, `connect` 66-110, `update/refresh` 113-124 | exact |
| `crates/dimuli-app/src/texts.rs` | config (texts) | — | `images_skipped` / consts | exact |
| `crates/dimuli-app/src/platform/mod.rs` | trait / event enum | event-driven | `Platform` trait 17-28, `PlatformEvent` 31-44 | exact |
| `crates/dimuli-app/src/platform/desktop.rs` | provider | request-response | `impl Platform for DesktopPlatform` 33-57 | exact |
| `crates/dimuli-app/src/platform/android.rs` | provider (JNI) | request-response | `pick_images` 102-117, `NATIVES` 139-166, `native_images_picked` 178-191, `android_main` 19-40 | exact |
| `crates/dimuli-app/src/platform/native_events.rs` | utility (pure mapping) | transform | `images_picked_event` 9-19 + tests | exact |
| `crates/dimuli-app/ui/backend.slint` | config (UI interface) | — | `canvas-resized` callback line 40, `message-is-error` line 10 | exact |
| `crates/dimuli-app/ui/collage-canvas.slint` | component | event-driven | lines 9-24 (canvas size + `changed` handlers) | exact |
| `crates/dimuli-app/ui/message-banner.slint` | component | — | lines 6-25 | exact |
| `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt` | provider (Activity) | file-I/O | `copyToCache` 230-251, `onActivityResult` 196-218, JNI-bridge region 45-194 | exact |
| `android/app/src/main/kotlin/ch/staeheli/dimuli/ShareProvider.kt` | provider | request-response | `query` 34-50 | exact |
| `android/app/src/main/AndroidManifest.xml` | config | — | line 11 `configChanges` | exact |
| `android/scripts/smoke.sh` | test script | batch | lines 39-57 | exact |
| `docs/ARCHITECTURE.md`, `docs/PLAN.md` | docs | — | limits table / milestone list | exact |

## Pattern Assignments

### `crates/collage-core/src/decode.rs` (utility, transform)
Lines 12-17 — only change values, keep doc comments:
```rust
/// Images with more pixels are rejected before decoding (protects against manipulated files).
pub const DECODE_MAX_PIXELS: u64 = 100_000_000;   // -> 64_000_000 (D-08)
pub const DECODE_MAX_EDGE: u32 = 16_384;
pub const DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;  // -> 256 * 1024 * 1024 (D-17)
```
D-18: map `image::ImageError::Limits(..)` from the decoder (and edge-limit rejection) to `CoreError::ImageTooLarge` inside `decode_unoriented` (uses the local `fail` closure pattern at line ~35 for other errors). Adjust the existing edge-limit test.

### `crates/collage-core/src/render.rs` (tests, POLI-02)
Replace `opts(200)` (~line 426), `opts(101)`, `opts(100)` (assert `(80,100)` -> `opts(60)` / `(48,60)`), `opts(90)` with values <= 64 and update assertions. Tests generate images with `solid(w, h, COLOR)` + `setup(&dir, ...)`; keep that.

### `crates/dimuli-app/src/images.rs` (loader, Pattern 6)
Current (lines 49-70) — the two copies to collapse:
```rust
let mut preview = downscale(decoded, PREVIEW_MAX_EDGE);
preview.apply_orientation(orientation);
let rgba = preview.into_rgba8();                       // copy 1
...
preview: SharedPixelBuffer::clone_from_slice(rgba.as_raw(), preview_width, preview_height), // copy 2
```
Replace with `SharedPixelBuffer::<Rgba8Pixel>::new(w, h)` + fill `make_mut_bytes()`/`make_mut_slice()` by matching `DynamicImage::ImageRgb8` / `ImageRgba8`, fallback `to_rgba8()`. Keep the zero-size `CoreError::InvalidImageSize` check. Use `chunks_exact`/iterators (clippy `indexing_slicing` denied).

### `crates/dimuli-app/src/state.rs` (store, event-driven)
**Message constructors** (lines 155-167) — copy for `show_success`, add `seq`:
```rust
pub(crate) fn show_notice(&mut self, text: impl Into<String>) {
    self.message = Some(Message { kind: MessageKind::Notice, text: text.into() });
}
```
`MessageKind` (27-30) gets `Success`; `Message` (34-37) gets `seq: u64`. `save_finished` (171-179) switches `show_notice(texts::SAVED)` -> `show_success`. New `expire_message(seq)` follows `dismiss_message` (192-194).

**Late-result guard** (Pitfall 2) — `finish_loading` (225-230) and `finish_export` (293-294) start with `self.busy = None;`. Prepend:
```rust
if self.busy != Some(Busy::Loading) { log::warn!("..."); return; }
```
**Early-out pattern** (POLI-02) — copy the guard style of `begin_export` (272-275):
```rust
if self.busy.is_some() || self.screen != Screen::Editor { return None; }
```
-> add `if action == ExportAction::Share && !self.share_supported { self.show_error(texts::SHARE_UNAVAILABLE); return None; }` before `busy` is set.

**Failure counts** — extend `load_notice(skipped, truncated)` (671-683) with `too_large` and Kotlin `failed` counts; it joins parts with `" "` and returns `Notice`. `finish_loading` needs to split `Err(CoreError::ImageTooLarge{..})` from other errors instead of `filter_map(Result::ok)`.

**Tests**: existing ones in the same file assert `message.kind == MessageKind::Notice/Error` (e.g. lines 766, 775); copy that form.

### `crates/dimuli-app/src/view.rs` (transform)
- `Snapshot.message` is `(String, bool)` via `m.kind == MessageKind::Error` (line ~362); extend if banner distinguishes Success.
- `canvas_aspect` computed in `snapshot` (~380-384) -> replace with `canvas_width/height` from new pure `fit_canvas(area, aspect) -> Size` (place next to `pixels_per_unit`, line 42, with host tests in the module's `tests`).
- Gesture fast path: copy `sync_model` (437-447) row comparison; scratch buffer as `RefCell<Vec<CellView>>` like `shown_templates: RefCell<Vec<String>>` (line 303). Cells are built by `cell_views(collage, &state.images, state.canvas, state.swap_highlight())` (line 336) — add an `_into(&mut Vec)` variant.

### `crates/dimuli-app/src/controller.rs` (controller)
**Callback wiring** (66-110), `on!` macro:
```rust
on!(on_canvas_moved, |c, x, y| c.update(|s| s.move_to(x, y)));
on!(on_canvas_resized, |c, w, h| c.update(|s| s.set_canvas(w, h)));
```
-> `canvas-moved`, `canvas-pinched`, `canvas-scrolled` switch to `c.update_gesture(...)`; `canvas-resized` becomes `canvas-area-resized`.
**Transition + refresh** (113-124) — new `update_gesture` copies `update` exactly, but calls `self.models.apply_cells(...)` instead of `refresh`; never hold `state.borrow()` across Slint calls (module doc lines 1-5).
**Install** (50-64): new fields `message_timer: slint::Timer`, `armed_seq: Cell<Option<u64>>`; read `platform.share_supported()` and `platform.restored_after_close()` here like `touch_target` (line 58). Timer closure must hold `Rc::downgrade(self)` (`Weak` already imported line 10).
**Collage-open push** (D-15): after each refresh, when `state.screen` differs from last pushed value, call `self.platform.set_collage_open(bool)`.

### `crates/dimuli-app/src/texts.rs`
Copy const/fn style:
```rust
pub(crate) fn images_skipped(count: usize) -> String {
    if count == 1 { "Ein Bild konnte nicht geladen werden und wurde übersprungen.".to_owned() }
    else { format!("{count} Bilder konnten nicht geladen werden und wurden übersprungen.") }
}
```
Add `images_too_large(count)` and `const RESTARTED: &str`. `SHARE_UNAVAILABLE` (line 21) is reused. Android-only texts use `#[cfg(any(target_os = "android", test))]` (lines 14-19).

### `crates/dimuli-app/src/platform/mod.rs` / `desktop.rs`
Trait (17-28): add documented methods `share_supported(&self) -> bool`, `restored_after_close(&self) -> bool`, `set_collage_open(&self, open: bool)` (consider default impls only if guidelines allow). `PlatformEvent::ImagesPicked(Vec<PathBuf>)` (line 34) -> `ImagesPicked { paths, failed }` (update `desktop.rs::pick_event` line 25 and controller `handle_platform_event` line 132). Desktop impl style (33-57): one-liners; `share` keeps posting `SHARE_UNAVAILABLE` as fallback.

### `crates/dimuli-app/src/platform/android.rs` (JNI)
**Outgoing call with arg** (102-117):
```rust
let result = self.with_activity(|env, activity| {
    env.call_method(activity, jni_str!("pickImages"), jni_sig!((max: jint) -> void), &[JValue::Int(max)])?;
    Ok(())
});
if let Err(error) = result { log::error!("cannot open the photo picker: {error}"); ... }
```
-> `setCollageOpen` with `jni_sig!((open: jboolean) -> void)`, `JValue::Bool(..)`; `collageLostOnRestart` with `jni_sig!("()Z")` + `.z()` (RESEARCH Code Examples), errors logged and default `false`.
**Natives table** (139-166): `[jni::NativeMethod<'static>; 5]` under `#[allow(deprecated)]`; change entry to `fn native_images_picked(paths: java.lang.String[], failed: jint)` and the handler signature (178-191) adds `failed: i32`; handler only calls `post_event(images_picked_event(list, failed))`.
**Start sequence** (19-40): pull the restart flag after `register_natives`, before `crate::run`; store it in `AndroidPlatform` (struct line 44-46).

### `crates/dimuli-app/src/platform/native_events.rs`
Copy `images_picked_event` (9-19) and add `failed: i32` -> `usize` via `usize::try_from(failed).unwrap_or(0)`; keep the empty-string filter. Rule: `paths.is_empty() && failed > 0` -> no-images event. Tests in the same file (`picked_paths_become_images_picked_event`) show the `assert_eq!(event, PlatformEvent::...)` form.

### `crates/dimuli-app/ui/collage-canvas.slint` / `backend.slint` / `message-banner.slint`
Current (collage-canvas 9-24):
```slint
width: max(0px, min(parent.width, parent.height * Backend.canvas-aspect));
height: Backend.canvas-aspect > 0 ? self.width / Backend.canvas-aspect : 0px;
changed width => { Backend.canvas-resized(self.width, self.height); }
init => { Backend.canvas-resized(self.width, self.height); }
```
-> outer `CollageCanvas` reports `self.width/height` via `canvas-area-resized` (changed + init on the outer element); inner `canvas` uses `Backend.canvas-width/height` (`in property <length>` in backend.slint next to `canvas-aspect`, line 18). Banner (message-banner 6-20) uses `Backend.message-is-error ? Theme.error : Theme.notice`; a Success style must come from `Theme`, no logic.

### `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt`
**Per-image try/catch in `copyToCache`** (230-251):
```kotlin
uris.forEachIndexed { index, uri ->
    val target = File(dir, "img-$index")
    try {
        val input = contentResolver.openInputStream(uri) ?: throw java.io.IOException("no stream")
        input.use { source -> target.outputStream().use { source.copyTo(it) } }
        paths.add(target.absolutePath)
    } catch (e: Exception) {
        Log.e(TAG, "copy failed", e)
        target.delete()
    }
}
```
-> return paths + failed count; add `catch (e: OutOfMemoryError)`; branch on `contentResolver.getType(uri)`; transcode via `ImageDecoder` (RESEARCH Pattern 1; PNG when `bitmap.hasAlpha()`, D-16).
**JNI bridge doc style** (62-66, 162-166): every bridge fun has KDoc `Rust: name (...) , JNI signature (...)`; place `collageLostOnRestart`/`setCollageOpen` inside `// region JNI bridge` (45 .. 194); update `nativeImagesPicked` KDoc to `([Ljava/lang/String;I)V`.
**Result dispatch** (196-218): replace `if (paths.isEmpty()) nativePickFailed()` by `nativeImagesPicked(paths, failed)`; `@Deprecated` at 196 -> `@Suppress("OVERRIDE_DEPRECATION")` (N1).
**onCreate** (26-33): set `collageLost` before `super.onCreate`; add `onSaveInstanceState`; add `File(cacheDir, "share")` cleanup next to the `picked` cleanup on `io` (line 32).

### `android/app/src/main/kotlin/ch/staeheli/dimuli/ShareProvider.kt`
`query` (34-50) — guard: `if (!isServed(uri)) throw FileNotFoundException(...)`; add same for `!servedFile().exists()` (N4).

### `android/app/src/main/AndroidManifest.xml`
Line 11: append `|fontWeightAdjustment|colorMode|grammaticalGender|touchscreen|mcc|mnc`.

### `android/scripts/smoke.sh`
Line 39 `"${ADB[@]}" install -r "$APK" 2>&1 | grep -q "Success"` -> `out="$("${ADB[@]}" install -r "$APK" 2>&1)"; grep -q Success <<<"$out" || fail install` (also line 43 `am start`). Before screencap (line 56) add `sleep 2`.

## Shared Patterns

### Error logging + user text
**Source:** `state.rs` `save_finished`/`share_finished` (171-190): `log::error!("... failed: {message}"); self.show_error(message);`. Kotlin: `Log.e(TAG, "...", e)`. Apply to all new failure paths (D-13).

### JNI rules
**Source:** `platform/android.rs` natives (169-225): natives only `post_event`, never touch Slint; all `unsafe` with `// SAFETY:`; Kotlin calls into Rust only through `native { ... }` (MainActivity 188-192) and off-UI work via `runOnIo` (178-185).

### Pure mapping + host tests
**Source:** `native_events.rs`, `state.rs` tests. Any new decision (failed counts, expire_message, fit_canvas, share early-out, busy guards) is a pure function with a `#[test]` in the same file.

### Background -> UI thread
**Source:** `controller.rs::post` (28-39) via `invoke_from_event_loop` + thread-local `CURRENT`.

## No Analog Found

| File / Feature | Role | Reason |
|---|---|---|
| `ImageDecoder` transcode in `MainActivity.kt` | file-I/O | No bitmap code exists yet; use RESEARCH.md Pattern 1 |
| `slint::Timer` auto-hide in `controller.rs` | event-driven | No timer in codebase; use RESEARCH.md "Auto-hide arming" example |
| `onSaveInstanceState` / restart flag | lifecycle | New; use RESEARCH.md "Kotlin side of the restart flag" |

## Metadata
**Analog search scope:** `crates/collage-core/src`, `crates/dimuli-app/src`, `crates/dimuli-app/ui`, `android/app/src/main`, `android/scripts`
**Files scanned:** 16
**Pattern extraction date:** 2026-11-14
