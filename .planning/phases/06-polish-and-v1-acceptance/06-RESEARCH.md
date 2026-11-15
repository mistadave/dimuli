# Phase 6: Polish and v1 Acceptance - Research

**Researched:** 2026-11-14
**Domain:** Android lifecycle (NativeActivity + android-activity 0.6 + Slint 1.18), platform image transcoding (Kotlin `ImageDecoder`), Rust memory/UI-update hygiene, user-facing error messages
**Confidence:** HIGH for in-repo behaviour and library internals (read from source this session, two emulator probes run); MEDIUM for `ImageDecoder` details (official docs plus AOSP commit mirrors; on-device confirmation still needed)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

All four areas were proposed by Claude and accepted by the owner unchanged (2026-11-14).

#### HEIC/WebP and other formats
- **D-01:** On Android, Kotlin transcodes every picked image that is not JPEG or PNG (by MIME type from the ContentResolver) while copying it into `cacheDir/picked/`: decode with the platform `android.graphics.ImageDecoder` (covers HEIC/HEIF, WebP, and AVIF where the device supports it), write a JPEG (quality 95) into the cache. JPEG/PNG keep the current byte copy. No new dependency.
- **D-02:** `ImageDecoder` applies EXIF orientation, so transcoded copies are written already oriented and without EXIF orientation; Rust handles them like any JPEG.
- **D-03:** A failed transcode counts as an unreadable image: skipped, existing "n Bilder konnten nicht gelesen werden" notice, `Log.e` with the cause.
- **D-04:** Rejected: `libheif` (C library, LGPL, hard cross-build) and the `image` crate `webp` feature (WebP only). Desktop stays JPEG/PNG only.

#### Process / activity death (LIFE-01)
- **D-05:** No persistence in v1 (unchanged project decision). LIFE-01 is proven for the normal cases where the process survives: home, app switcher, returning from Photo Picker and share sheet, rotation (already via `configChanges`, Phase 5 D-05).
- **D-06:** When Android destroyed the activity/process and the user comes back, the app starts on the start screen and shows a German notice, e.g. *"Die App wurde vom System geschlossen. Die Collage konnte nicht erhalten bleiben."* Detection in Kotlin via `savedInstanceState != null` in `onCreate`, forwarded to Rust (exact mechanism: planner/research).
- **D-07:** Research must check whether the activity can be recreated while the process keeps running (e.g. developer option "Don't keep activities", memory pressure) and how `android_main` / `run()` behave then (state currently lives inside `run()`); if keeping `AppState` across such a recreation is cheap and safe, propose it, otherwise D-06 applies.

#### Large images (PERF-01)
- **D-08:** `DECODE_MAX_PIXELS` drops from 100 MP to **64 MP** on all platforms (a 100 MP RGBA decode is about 400 MB; export long edge is only 3000 px). Record the decision in `docs/ARCHITECTURE.md` (limits table). — **Reversibility:** reversible — one constant plus docs/tests.
- **D-09:** On Android, Kotlin downscales any picked image above 64 MP to at most 64 MP while copying (same `ImageDecoder` path as D-01, e.g. `setTargetSize`), so very large captures (e.g. 200 MP modes) still work. The Rust limit stays as a safety net; images above it are skipped with a notice (always the case on desktop).
- **D-10:** Peak memory during load and export must be checked with real large photos (e.g. 50 MP JPEG, HEIC) on the emulator and the connected device.

#### Messages (ERRM-01)
- **D-11:** Keep the existing message banner. Success messages (e.g. "Collage gespeichert.") hide automatically after about 3 s; error messages stay until the user closes them.
- **D-12:** Partial load failure: continue with the readable images and show the existing count notice (current behaviour, keep).
- **D-13:** Every failure path has its own German text: image unreadable, image too large, transcode failed (= unreadable), pick failed, export failed, out of memory (if detectable), save failed, share failed, restart after system close (D-06). Each text says in du-form what happened and what to do, no technical details; the code logs the cause with `log::error!` (Kotlin: `Log.e`).

#### Acceptance (ACPT-01)
- **D-14:** ACPT-01 is part of the owner's final device test on Android 16 (and Android 14), together with the deferred 05-07 checklist (touch editing, Pitfall 6, picker, save to Google Photos, share, back). Milestone 5 and 6 are ticked in `docs/PLAN.md` only after that test.

### Claude's Discretion
- M3 review notes: update only affected cells during a gesture (no full snapshot allocation per pointer event), drop the unnecessary pixel-buffer copy on load, remove the 1-frame flicker after an aspect change.
- M4 review notes: render tests with outputs of 64 px or less; desktop "Teilen" reports "nicht verfügbar" immediately without rendering first.
- Phase 5 review notes N1-N6 (`05-REVIEW.md`): fold in where cheap (e.g. smoke.sh SIGPIPE and first-frame screenshot, `@Deprecated` on `onActivityResult`, `ShareProvider.query` for a missing file, cleaning `cacheDir/share` at start).
- Exact auto-hide duration and banner animation; exact German wording within D-13.
- Test split per the owner rule (memory "device-first smoke tests"): smoke/insets on the connected device (`-PrustAbis=arm64-v8a`), automated picker/gallery/share flows and the API 33/37 matrix only on emulators.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope. (Persistence of collages remains out of scope for v1.)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| LIFE-01 | Pause/resume (home, app switcher, returning from Photo Picker or share sheet) keeps the collage and all edits | §Lifecycle facts (process survives -> `android_main` keeps running; verified: unhandled config change recreates the activity in-process and loses state); Pattern 4 (complete `configChanges`), Pattern 5 (restart notice via Kotlin flag), Pitfall 1-3 |
| PERF-01 | Large photos load/edit/export without crash/OOM/stutter; gesture updates only affected cells; no extra pixel-buffer copy on load; `DECODE_MAX_PIXELS` decided and recorded | Pattern 1 (Kotlin transcode/downscale), Pattern 6 (preview without copy), Pattern 7 (gesture fast path), memory budget table, host benchmark (50 MP decode 109 ms / 158 MB RSS) |
| ERRM-01 | Every failure path shows a short German message, logs with `log::error!` | Failure-path inventory table (9 paths, current vs. needed), Pattern 8 (auto-hide with `MessageKind::Success` + `slint::Timer`), OOM detectability analysis |
| POLI-01 | No one-frame flicker after aspect change | Slint 1.18.1 runs `changed` handlers before rendering (source-verified), so the flicker is timing-dependent; Pattern 9 makes Rust own the canvas size so cells and canvas change in one transition |
| POLI-02 | Render tests <= 64 px; desktop "Teilen" -> "nicht verfügbar" immediately | Exact list of offending tests (`opts(200)`, `opts(101)`, `opts(100)`, `opts(90)`); Pattern 10 (`share_supported` flag checked in `begin_export`) |
| ACPT-01 | Real Android 16 device: 1-9 images, save, visible in gallery | Owner checkpoint (D-14) merging the 05-07 Task 3 checklist; plan must end with `checkpoint:human-verify` |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Rust stable (edition 2024), Slint UI with the Android backend; UI files under `ui/*.slint`. Code and comments in English, communication with the owner in German.
- **Every code change is written by the `rust-coder` subagent** (`.claude/agents/rust-coder.md`), even simple ones. Very complex tasks are split into small subtasks first. The main agent only reads, plans, coordinates and checks.
- **No `unwrap()` / `expect()` in production code** (workspace clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `undocumented_unsafe_blocks`).
- **No new crates (or crate features, or Gradle dependencies, or AndroidX) without asking the owner**, and check Android compatibility.
- **No commits or pushes without explicit request.**
- Before finishing a change: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.
- Logic (layouts, cell maths, scaling/cropping, export) stays free of Android dependencies and host-testable; Android code is encapsulated behind the `Platform` trait; `cfg(target_os = "android")` only under `platform/`.
- UI changes are checked first on the desktop (`cargo run -p dimuli-app -- a.jpg b.png`); Android builds only for platform changes.
- After a milestone / before committing new functionality: run the `code-reviewer` subagent and fix critical findings (no re-review for pure fix-ups or doc/config-only changes).
- `docs/CODING_GUIDELINES.md` is binding: `unsafe` only in `platform/android.rs` with `// SAFETY:`; only two `#[allow]` exceptions; JNI natives only via `native_method!` + `RegisterNatives`; natives only call `post_event`; no `AndroidApp`/Activity in a `static`; Kotlin only for what needs the Activity, every JNI-facing Kotlin function in `// region JNI bridge` with signatures documented on both sides; no logic in `.slint`; colors/sizes in `Theme`; test images <= 64 px, generated in tests; texts collected centrally (`texts.rs`, `ui/texts.slint`).
- Owner memory (device-first smoke tests): if a real device is connected, use it for smoke/insets (`-PrustAbis=arm64-v8a`); picker/gallery/share automation and the API 33/37 matrix only on emulators; the final manual device test (Android 14 + 16) is the owner's.

## Summary

Phase 6 is mostly hardening of existing code paths, plus one Kotlin feature (transcoding). Nothing needs a new dependency: `ImageDecoder` (API 28+) and `Bitmap.compress` are platform APIs, `slint::Timer` is already in Slint 1.18.1, and every Rust change stays inside `collage-core`, `state.rs`, `view.rs`, `controller.rs`, `images.rs`, `texts.rs` and `platform/`.

The central lifecycle finding (D-07): each `ANativeActivity_onCreate` spawns a **new** `android_main` thread; on `onDestroy` android-activity sends `MainEvent::Destroy`, Slint 1.18.1's event loop breaks on it, `run()` returns and the whole `AppState` is dropped [VERIFIED: android-activity-0.6.1/src/native_activity/glue.rs:400-420, 876-1003; i-slint-backend-android-activity-1.18.1/androidwindowadapter.rs:271-273]. Re-initialising Slint on the new thread works because Slint's global context is thread-local and the Android event-loop proxy is a replaceable `Mutex` [VERIFIED: i-slint-core-1.18.1/context.rs:23-26, platform.rs:259-282]. An emulator probe confirmed that an activity **is** recreated in the same process (same PID 4050, second `dimuli started` log line) when the user toggles "bold text" (`font_weight_adjustment`), because `fontWeightAdjustment` is missing from `configChanges`. Android itself never destroys a single activity for memory; it kills the whole process [CITED: developer.android.com/guide/components/activities/activity-lifecycle]. So in-process recreation only comes from unhandled configuration changes and the "Don't keep activities" developer option. **Recommendation:** do not carry `AppState` across recreation (it would need a `Send` session in a static, a Kotlin guard against deleting `cacheDir/picked`, and guards for in-flight background results); instead close the real gap by completing `configChanges`, and let D-06 cover everything else.

For messages, add a third `MessageKind` (success) that auto-hides through one `slint::Timer` owned by the controller, with a message sequence number in `AppState` so the timer can only dismiss the message it was armed for (host-testable). For POLI-01, Slint 1.18.1 runs `changed` handlers inside `draw_contents` before painting [VERIFIED: i-slint-core-1.18.1/window.rs:794-811, 1839-1846], so the flicker depends on timing. The robust fix is to let Rust compute the canvas size from the available area and the collage aspect, so canvas size and cell geometry change in the same transition.

**Primary recommendation:** Four sequential host-side plans (core limits + render-test sizes; preview copy + gesture fast path; canvas sizing + share early-out; messages/texts/auto-hide + late-result guards), then one Android plan (Kotlin transcode/downscale with a failed-count JNI parameter, complete `configChanges`, restart notice, N1-N5 cleanups), then emulator/device verification with memory measurement, code review and docs, and finally the owner's device checkpoint (ACPT-01 + 05-07 checklist).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HEIC/WebP/AVIF -> JPEG transcode, > 64 MP downscale | Kotlin `MainActivity.copyToCache` (Android platform) | — | Needs `ContentResolver` + `ImageDecoder`; runs on the existing `io` executor (D-01, D-09) |
| Decode limits (`DECODE_MAX_*`), "too large" classification | `collage-core::decode` | `dimuli-app::images` | Safety net shared by preview and export; pure Rust, host-tested |
| Preview creation (one copy into `SharedPixelBuffer`) | `dimuli-app::images` (loader thread) | — | Background thread, `Send` buffers only |
| Restart detection (collage lost) | Kotlin `onCreate`/`onSaveInstanceState` | `platform/android.rs` (pull at start), `state.rs` (message) | Only the Activity sees `savedInstanceState`; Rust pulls one boolean via JNI |
| Message kinds, sequence, auto-hide decision | `state.rs` (pure) | `controller.rs` (`slint::Timer`) | Logic must not live in `.slint` (guidelines §2/§4); timer is the only UI-thread side effect |
| Failure texts | `texts.rs` | `ui/texts.slint` (none needed) | Central German texts |
| Canvas size from area + aspect | `view.rs` (pure fn) + `state.rs` | `collage-canvas.slint` (reports area, draws given size) | Removes Slint-handler timing from the geometry path (POLI-01) |
| Gesture fast path (changed rows only) | `view.rs::Models` | `controller.rs` | Model update strategy, no Slint logic |
| Share availability | `Platform` trait (`share_supported`) -> `AppState` flag | `state.rs::begin_export` | Host-testable early-out (POLI-02) |
| Late background results after a restart | `state.rs` guards (`finish_loading`, `finish_export`) | — | Results posted to a new controller must not apply to a fresh state |

## Standard Stack

### Core (all already in the project; no additions)
| Library / API | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `android.graphics.ImageDecoder` | platform, API 28+ (minSdk 33) | Decode HEIC/HEIF, WebP, AVIF; scaled decode via `setTargetSize` | Platform decoder, security-patched by OS updates, no dependency [CITED: developer.android.com/reference/android/graphics/ImageDecoder] |
| `android.graphics.Bitmap.compress(JPEG, 95, out)` | platform | Write the transcoded JPEG (drops EXIF, so orientation is baked in) | Platform encoder [ASSUMED: EXIF-free output; verify with a rotated source] |
| `android.graphics.BitmapFactory` (`inJustDecodeBounds`) | platform | Read width/height of byte-copied JPEG/PNG to decide on downscale | Header-only, no pixel allocation [ASSUMED] |
| `slint::Timer` (`TimerMode::SingleShot`) | slint 1.18.1 (locked) | Auto-hide of success messages | `start()` restarts a running timer [VERIFIED: i-slint-core-1.18.1/timers.rs:80-110] |
| `image` | 0.25.10 (locked; features `jpeg`, `png` only) | Decode with `Limits`; `ImageError::Limits(LimitErrorKind::InsufficientMemory)` is returned before allocating | [VERIFIED: image-0.25.10/src/io/limits.rs:99-107] |
| `jni` | 0.22.4 (locked) | New `()Z` pull call and `()V`/`(Z)V` calls; `JValueOwned::z()` returns `bool` | [VERIFIED: jni-0.22.4/src/env.rs:2899-2905, jvalue.rs:128-130] |
| android-activity | 0.6.1 (via `backend-android-activity-06`) | NativeActivity glue: one `android_main` thread per activity instance | [VERIFIED: android-activity-0.6.1/src/native_activity/glue.rs:876-1003] |

### Supporting (host test tooling, already installed)
| Tool | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| ImageMagick `convert` | 6 with libheif 1.12, libwebp 1.2.2 | Create HEIC/AVIF/WebP test files for the emulator | Verified this session: `convert -size 400x300 gradient:red-blue t.heic` (also `.avif`, `.webp`) works |
| `ffmpeg` | system | Create 50 MP / 80 MP JPEGs fast (`-f lavfi -i testsrc2=s=8160x6144 -frames:v 1`) | Verified: 50 MP in 0.7 s, 80 MP in 1.0 s |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Restart notice from Kotlin `savedInstanceState` (D-06) | android-activity native saved state (`MainEvent::SaveState` + `app.saved_state()` via `slint::android::init_with_event_listener`) | Pure Rust, but D-06 locks Kotlin detection; keep Kotlin |
| Rust-owned canvas size (POLI-01) | Cell geometry in collage units, multiplied by canvas width in `cell-tile.slint` | Also flicker-free, but moves arithmetic into `.slint` and changes `CellView`/`DividerView` types; Rust-owned size keeps logic in Rust |
| Keep `AppState` across in-process recreation (D-07) | (rejected) | See Pattern 5 note: static `Send` session + Kotlin cache guard + result guards; covers only dev-option and missed config changes |

**Installation:** none (no new crates, Gradle dependencies or features).

## Package Legitimacy Audit

No external packages are installed in this phase. The only new APIs are Android platform classes (`ImageDecoder`, `Bitmap`, `BitmapFactory`, `ColorSpace`) and existing locked crates (`slint` 1.18.1, `image` 0.25.10, `jni` 0.22.4).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                       Photo Picker (system)
                              │ content:// URIs
                              ▼
 MainActivity.onActivityResult ──► io executor: copyToCache(uris)
                                      │ per URI:
                                      ├─ MIME (ContentResolver.getType) ∈ {jpeg, png}?
                                      │     yes ─► byte copy ─► bounds check (BitmapFactory)
                                      │                 ├─ ≤ 64 MP and edges ≤ 16384 ─► keep
                                      │                 └─ larger ─► ImageDecoder(file) ─► scaled
                                      │     no  ─► ImageDecoder(uri) ─► (scaled if > 64 MP) ─► JPEG q95
                                      │ failures counted (Log.e)
                                      ▼
                     nativeImagesPicked(paths[], failed)  ──► post_event(ImagesPicked{paths, failed})
                                                                     │ (UI thread)
                                                                     ▼
 Controller ─► AppState.begin_loading ─► loader thread: decode (Limits: 64 MP, 16384 edge, alloc)
                                              │  thumbnail ≤ 2048 ─► orient ─► ONE copy into SharedPixelBuffer
                                              ▼
                     post ─► AppState.finish_loading (only if busy == Loading)
                                 │ counts: unreadable / too large / Kotlin-failed / truncated
                                 ▼
                         Message{kind, text, seq} ──► view snapshot ──► Slint banner
                                 │ kind == Success
                                 ▼
                         Controller timer (3 s, single shot) ─► AppState.expire_message(seq)

 Gesture events ─► AppState transition ─► Models::apply_cells (changed rows only, scratch Vec reused)
 Area resize / aspect change ─► AppState(area, aspect) ─► view::fit_canvas ─► canvas size + cells (same update)

 Lifecycle:  onCreate(savedState) ─ Kotlin reads "collage was open" flag ─┐
             android_main (new thread per activity) ─► JNI pull ()Z ──────┴─► Controller::install ─► restart notice
             onSaveInstanceState ◄─ Kotlin flag ◄─ Rust setCollageOpen(Z) on Start/Editor transitions
             onDestroy ─► MainEvent::Destroy ─► Slint loop breaks ─► run() returns ─► AppState dropped
```

### Recommended Project Structure (files touched)
```
crates/collage-core/src/decode.rs      # DECODE_MAX_PIXELS 64 MP; Limits error -> ImageTooLarge (optional)
crates/collage-core/src/render.rs      # tests: outputs <= 64 px
crates/dimuli-app/src/images.rs        # preview: one copy into SharedPixelBuffer
crates/dimuli-app/src/state.rs         # MessageKind::Success + seq; expire_message; failure counts;
                                       # late-result guards; canvas area; share_supported
crates/dimuli-app/src/view.rs          # fit_canvas; Models::apply_cells (gesture fast path)
crates/dimuli-app/src/controller.rs    # timer; update_gesture; restart notice; collage_open push
crates/dimuli-app/src/texts.rs         # new texts (too large, restarted)
crates/dimuli-app/src/platform/{mod,desktop,android,native_events}.rs
crates/dimuli-app/ui/{backend,collage-canvas}.slint
android/app/src/main/kotlin/ch/staeheli/dimuli/{MainActivity,ShareProvider}.kt
android/app/src/main/AndroidManifest.xml   # configChanges completed
android/scripts/smoke.sh                   # N2, N3
docs/ARCHITECTURE.md, docs/PLAN.md         # limits table, decisions, ticks after ACPT-01
```

### Pattern 1: Kotlin transcode / downscale in `copyToCache` (D-01, D-02, D-03, D-09)
**What:** Copy every URI into `cacheDir/picked/img-<i>`. If the MIME type is not JPEG/PNG, or the copied JPEG/PNG is larger than 64 MP or has an edge above 16384 px, decode with `ImageDecoder` (software allocator, sRGB, scaled target) and write a JPEG q95 over the target. Count failures.
**When to use:** Always on Android, inside the existing `io` executor task (never on the UI thread).
**Example:**
```kotlin
// Source: pattern from developer.android.com/reference/android/graphics/ImageDecoder
// (OnHeaderDecodedListener setters, setTargetSize, ALLOCATOR_SOFTWARE). Values 64_000_000 and 16384
// mirror DECODE_MAX_PIXELS (new) and DECODE_MAX_EDGE (crates/collage-core/src/decode.rs:15).
private fun transcodeToJpeg(source: ImageDecoder.Source, target: File) {
    val bitmap = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
        decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE   // HARDWARE is the default for large images
        decoder.setTargetColorSpace(ColorSpace.get(ColorSpace.Named.SRGB)) // Rust assumes sRGB
        val (w, h) = info.size.width to info.size.height
        val scale = fitScale(w, h)                            // <= 1.0, see below
        if (scale < 1.0) decoder.setTargetSize(
            maxOf(1, floor(w * scale).toInt()), maxOf(1, floor(h * scale).toInt()))
    }
    try {
        target.outputStream().use { out ->
            if (!bitmap.compress(Bitmap.CompressFormat.JPEG, 95, out)) throw IOException("compress failed")
        }
    } finally {
        bitmap.recycle()   // free ~256 MB native memory now, not at the next GC
    }
}

/** Largest factor <= 1 that keeps w*h <= MAX_PIXELS and both edges <= MAX_EDGE (floor, never round up). */
private fun fitScale(w: Int, h: Int): Double {
    val byPixels = sqrt(MAX_PIXELS.toDouble() / (w.toLong() * h).toDouble())
    val byEdge = MAX_EDGE.toDouble() / maxOf(w, h)
    return minOf(1.0, byPixels, byEdge)
}
// catch (e: Exception) AND catch (e: OutOfMemoryError) around each image: OOM is an Error, not an Exception.
```

### Pattern 2: Failed-count over JNI (D-03, ERRM-01)
Today `copyToCache` silently drops failed copies; Rust receives fewer paths and shows **no** notice. A 3-image pick with one failed copy opens the editor with 2 images and says nothing [VERIFIED: MainActivity.kt:238-249 drops failures; native_events.rs:15 `.filter(|path| !path.is_empty())`]. Also when *all* copies fail Kotlin calls `nativePickFailed()`, whose text is "Die Bildauswahl konnte nicht geöffnet werden…" (wrong cause) [VERIFIED: MainActivity.kt:209-216; texts.rs:16-17].
**Recommendation:** change the native to `nativeImagesPicked(paths: Array<String>, failed: Int)` (JNI `([Ljava/lang/String;I)V`, Rust `native_method! { fn native_images_picked(paths: java.lang.String[], failed: jint) … }`) and `PlatformEvent::ImagesPicked { paths, failed }` (or a `PickedImages` struct). `failed > 0 && paths.is_empty()` means "nothing readable" -> `NO_IMAGES_LOADED`; `paths.is_empty() && failed == 0` stays "cancelled". The mapping stays pure in `native_events.rs` (host-tested). `jint` already appears in this project's JNI DSL: `jni_sig!((max: jint) -> void)` [VERIFIED: platform/android.rs:108]; the `native_method!` form with `jint` is [ASSUMED] to work the same way (compile-time checked by the macro).

### Pattern 3: Failure-path inventory (ERRM-01, D-13)

| # | Failure path | Today | Needed |
|---|---|---|---|
| 1 | Image unreadable (Rust decode error) | `images_skipped(n)` notice, `log::error!` per image in `images.rs:112` | keep (D-12) |
| 2 | Image too large (`ImageTooLarge`, decode `Limits`) | counted as "skipped" (same text) | separate count + text, e.g. `images_too_large(n)`: "Ein Bild ist zu groß und wurde übersprungen. Wähle ein kleineres Bild aus." |
| 3 | Transcode/copy failed (Kotlin) | silently dropped (see Pattern 2) | counted as unreadable (D-03) |
| 4 | Pick failed (picker not startable) | `PICK_FAILED` | keep; all-copies-failed -> `NO_IMAGES_LOADED` instead |
| 5 | Export failed | `export_error()` -> `EXPORT_SOURCE_UNREADABLE` / `GENERIC_ERROR` | keep; `ImageTooLarge` in export already maps to source-unreadable |
| 6 | Out of memory | not detectable as such (see below) | only via `image` `Limits` (-> path 2) and Kotlin `OutOfMemoryError` (-> path 3) |
| 7 | Save failed | `SAVE_FAILED` | keep |
| 8 | Share failed | `SHARE_FAILED` (Android), `SHARE_UNAVAILABLE` (desktop, after a full render) | desktop early-out (Pattern 10) |
| 9 | Restart after system close | none | new text, persistent (Error kind), shown at start (Pattern 5) |

**OOM detectability (D-13 "if detectable"):** Rust's default allocation-failure handler aborts the process; it cannot be caught [ASSUMED: std `handle_alloc_error` default behaviour]. On Android, Linux overcommit plus the low-memory killer usually kill the process before `malloc` fails [ASSUMED]. What *is* detectable: (a) `image` refuses an allocation above `Limits::max_alloc` with `ImageError::Limits(InsufficientMemory)` before allocating [VERIFIED: image-0.25.10/src/io/limits.rs:99-107], map it to the "too large" class; (b) Kotlin `OutOfMemoryError` from `ImageDecoder`/`Bitmap` is a `java.lang.Error`, so `catch (e: Exception)` misses it; catch it explicitly and count as unreadable. OOM is otherwise *prevented* by limits, not reported. After a real OOM kill, the restart notice (path 9) is what the user sees.

### Pattern 4: Complete `configChanges` (LIFE-01, D-07)
Current value [VERIFIED: android/app/src/main/AndroidManifest.xml:11]:
`android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|layoutDirection|locale"`

Missing per the official value list: `colorMode`, `fontWeightAdjustment`, `grammaticalGender` (API 34), `touchscreen`, `mcc`, `mnc` [CITED: developer.android.com/guide/topics/manifest/activity-element, configChanges table]. **Probe (emulator API 36, this session):** with the app on the start screen, `adb shell settings put secure font_weight_adjustment 300` recreated `MainActivity` in the same process (PID 4050 before and after; logcat shows a second `dimuli::platform::android: dimuli started` and `finishDrawing of relaunch`); no `RustPanic`. The setting was deleted afterwards. With `fontWeightAdjustment` added, the same toggle must leave exactly one `dimuli started` line. Slint only re-reads density on `ConfigChanged` [VERIFIED: androidwindowadapter.rs:253-270], so absorbing these changes is safe; a bold-text change simply takes effect on the next start.

### Pattern 5: Restart notice — Kotlin flag, Rust pulls (D-06)
- Kotlin keeps `@Volatile private var collageOpen = false` (set by Rust through a new JNI-bridge method `setCollageOpen(open: Boolean)`, `(Z)V`, called by the controller whenever `screen` changes between Start and Editor), and `@Volatile private var collageLost = false`.
- `onSaveInstanceState(outState)`: `super.onSaveInstanceState(outState); outState.putBoolean(KEY_COLLAGE_OPEN, collageOpen)`.
- `onCreate(savedInstanceState)`: **before** `super.onCreate(...)` set `collageLost = savedInstanceState?.getBoolean(KEY_COLLAGE_OPEN, false) == true`. Reason: `super.onCreate` of `NativeActivity` loads the library and `ANativeActivity_onCreate` spawns the `android_main` thread [VERIFIED: glue.rs:876-922], which may pull the flag immediately.
- Rust pulls once in `android_main`, after `register_natives`: `env.call_method(activity, jni_str!("collageLostOnRestart"), jni_sig!("()Z"), &[])?.z()?` and hands it to the controller (e.g. a `Platform` method `restored_after_close(&self) -> bool`, desktop `false`). The controller shows the persistent restart text once.
- This is a precise variant of D-06's `savedInstanceState != null`: it shows the notice only if a collage was really open (a process death while the user sat on the start screen or in the picker gets no misleading "Collage lost" text). The minimal variant (`savedInstanceState != null`) is one line less; planner picks (Open Question 1).
- `setCollageOpen` is a field write on the android_main thread: no `runOnUiThread`, no deadlock risk with `onSaveInstanceState`, which itself blocks the UI thread until android_main handles `SaveState` [VERIFIED: glue.rs:528-560].

**Why not keep `AppState` across in-process recreation (D-07 answer):** it is possible (Collage and `SharedPixelBuffer` are `Send` — `SharedVector<T: Send+Sync>` is `Send` [VERIFIED: i-slint-core-1.18.1/sharedvector.rs:97-99] and both already cross threads in `ExportJob`/`LoadedImage`), but it needs: a `static Mutex<Option<Session>>` filled when `run()` returns; `StoredImage` keeping a clone of the preview buffer; Kotlin skipping the `cacheDir/picked` wipe in `onCreate` [VERIFIED: MainActivity.kt:30-32] when the process already ran; resetting `busy`; and guards for loader/export results that arrive at the new controller. All of that serves only the developer option and configuration changes we can absorb (Pattern 4), because Android kills whole processes for memory [CITED: activity-lifecycle "The system never kills an activity directly to free up memory. Instead, it kills the process the activity runs in"]. Not cheap enough; D-06 applies.

### Pattern 6: Preview without the extra copy (PERF-01, M3)
Today: `preview.into_rgba8()` (allocation + conversion) then `SharedPixelBuffer::clone_from_slice(rgba.as_raw(), …)` (second full copy) [VERIFIED: images.rs:51, 68]. Fix: allocate `SharedPixelBuffer::<Rgba8Pixel>::new(w, h)` and fill `make_mut_slice()` in one pass from the oriented preview: fast paths for `DynamicImage::ImageRgb8` (alpha 255) and `ImageRgba8` (`copy_from_slice` on `make_mut_bytes()`), generic fallback `to_rgba8()` for rare formats [VERIFIED API: i-slint-core-1.18.1/graphics/image.rs:90, 106, 122]. Saves one preview-sized buffer (2048x1536x4 = 12.6 MB) of peak per image and one memcpy. One copy from the `image` crate's `Vec` into Slint's `SharedVector` is unavoidable (there is no adopt-a-Vec constructor).

### Pattern 7: Gesture fast path (PERF-01, M3)
Today every pointer event runs `Controller::update -> refresh -> Models::snapshot` which builds all `CellView`s, all `DividerView`s, compares template names, clones the message string and sets ~15 scalar properties [VERIFIED: controller.rs:113-124; view.rs:333-401]. Recommendation:
- `Controller::update_gesture(transition)` for `canvas-moved`, `canvas-pinched`, `canvas-scrolled`: apply the transition, then `Models::apply_cells(window, &state)` (and `apply_dividers` only while `Gesture::DraggingDivider`).
- `Models` keeps a `RefCell<Vec<CellView>>` scratch buffer: cleared and refilled while the state is borrowed (no allocation after warm-up), borrow released, then rows are compared and only changed rows get `set_row_data` (as `sync_model` does). This keeps the controller rule "never hold the state borrow across a call into Slint" [VERIFIED: controller.rs:1-5].
- Press/release/long-press/cancel keep the full `refresh` (they change highlight or end gestures; rare events).
- Note the gesture cost is independent of the source photo size: previews are capped at `PREVIEW_MAX_EDGE` [VERIFIED: images.rs:18 `pub(crate) const PREVIEW_MAX_EDGE: u32 = 2048;`].

### Pattern 8: Auto-hide of success messages (D-11)
- `state.rs`: extend `MessageKind` [VERIFIED: state.rs:27-30 `pub(crate) enum MessageKind { Error, Notice, }`] with `Success`; `Message` gets a `seq: u64` (incremented by every `show_*`). `save_finished(Ok)` uses `show_success(texts::SAVED)`. Partial-load notices stay `Notice` (persistent: they report a failure); the restart text is `Error`.
- `AppState::expire_message(seq)`: clears the message only if it is still `Success` with that `seq` (pure, host-tested).
- `controller.rs`: one `slint::Timer` field; after each refresh, if the current message is `Success` and its `seq` differs from the armed one, `timer.start(TimerMode::SingleShot, MESSAGE_HIDE_AFTER, …)` with a `Weak<Controller>` in the closure (a strong `Rc` would form a cycle via the timer stored in the controller). `start` restarts a running timer [VERIFIED: timers.rs:80-82].
- The banner's visual style for `Success` can reuse `Theme.notice` (or add a theme color); `message-is-error` stays a bool or becomes an enum — keep the Slint side presentation-only.

### Pattern 9: Rust owns the canvas size (POLI-01)
Today `collage-canvas.slint` computes `width: max(0px, min(parent.width, parent.height * Backend.canvas-aspect))` and reports its size back with `changed width/height => Backend.canvas-resized(...)` [VERIFIED: collage-canvas.slint:10-24]. After `select_aspect`, Rust sets `canvas-aspect` and the cells in the same refresh, but the cells are computed with the **old** `state.canvas`, so correctness depends on the change handler running before the next paint. In Slint 1.18.1 `draw_contents` runs `ensure_tree_instantiated`, which loops `ChangeTracker::run_change_handlers_once()` before rendering [VERIFIED: i-slint-core-1.18.1/window.rs:794-811, 1846], so the bad frame may never show today — but that is an internal ordering, not a contract.
**Fix:** the canvas reports its *available area* (`canvas-area-resized(w, h)`); Rust stores the area and computes the canvas size with a pure `view::fit_canvas(area, aspect) -> Size` (same formula as the Slint binding); the snapshot sets `Backend.canvas-width/height` together with the cells. Aspect change, template change and rotation then update canvas and cells in one transition. Host test: after `select_aspect`, `state.canvas` equals `fit_canvas(area, new aspect)` and the cell rects already use it.

### Pattern 10: Desktop share early-out (POLI-02)
Add `fn share_supported(&self) -> bool` to `Platform` (desktop `false`, Android `true`), copy it into `AppState` at install, and in `begin_export(ExportAction::Share)` with `share_supported == false` show `SHARE_UNAVAILABLE` (`"Teilen ist auf diesem Gerät nicht verfügbar."` [VERIFIED: texts.rs:21]) and return `None` without setting `busy`. Host test: no `ExportJob`, `busy` stays `None`, message is the unavailable text. Keep `DesktopPlatform::share` answering with the same text as a defensive fallback.

### Anti-Patterns to Avoid
- **Auto-hide timer in `.slint`:** a `Timer { running: … }` in the banner would put a decision in UI code and cannot tell two consecutive success messages apart; keep it in the controller/state.
- **`catch (e: Exception)` only around bitmap work:** misses `OutOfMemoryError`.
- **Rounding up the target size:** `round()` can produce `w*h` slightly above 64 MP, which Rust then rejects; use `floor`.
- **Holding the `AppState` borrow while writing Slint models** (controller rule).
- **Hardware bitmaps for transcoding:** `ALLOCATOR_DEFAULT` "will typically result in a HARDWARE allocation" [CITED: ImageDecoder#ALLOCATOR_DEFAULT]; use `ALLOCATOR_SOFTWARE` for a CPU-side compress.
- **Re-running the picker/gallery automation on the real device** (owner rule): emulator only.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HEIC/WebP/AVIF decoding | `libheif` / `webp` crate (rejected D-04) | Kotlin `ImageDecoder` | Platform codecs, OS security updates, EXIF orientation handled |
| Downscaling huge sources on Android | Rust-side scaled JPEG decode | `ImageDecoder.setTargetSize` | Decoder can subsample during decode instead of after [CITED: ImageDecoder#setTargetSampleSize "allows ImageDecoder to round in the direction that it can do most efficiently"] |
| One-shot/restartable timer | thread + sleep + `invoke_from_event_loop` | `slint::Timer` | Runs on the UI thread, restartable, cancelled on drop |
| Alloc limits for decoding | own size arithmetic before allocation | `image::Limits` (`max_alloc`, edge limits) | Already wired; returns an error before allocating |
| Activity lifecycle state machine | own Pause/Resume tracking in Rust | android-activity + Slint backend | Window/term handling already correct (`do_render` skips when no window [VERIFIED: androidwindowadapter.rs:516-527]) |

**Key insight:** every hard part of this phase (codecs, scaled decode, timers, lifecycle glue) already exists in the platform or the locked crates; the work is wiring and guarding.

## Lifecycle Facts (D-07 investigation)

| Question | Finding | Source |
|---|---|---|
| What runs per activity instance? | `ANativeActivity_onCreate` spawns a new thread running `android_main`; `onCreate` waits only until the thread left `Init` | [VERIFIED: glue.rs:876-922] |
| What happens on `onDestroy`? | `notify_destroyed` writes `AppCmd::Destroy` and **blocks** until the android_main thread is `Stopped`; after `android_main` returns, glue calls `ANativeActivity_finish` | [VERIFIED: glue.rs:400-420, 985-996] |
| Does Slint stop on Destroy? | Yes: `PollEvent::Main(MainEvent::Destroy) => return Ok(ControlFlow::Break(()))` | [VERIFIED: androidwindowadapter.rs:271-273] |
| Can Slint re-init on the new thread? | Yes: `GLOBAL_CONTEXT` is thread-local; on Android `EVENTLOOP_PROXY` is a `Mutex<Option<…>>` overwritten by `set_platform` | [VERIFIED: context.rs:23-26; platform.rs:224, 259-282] |
| Home / app switcher / picker / share sheet | Activity stops but is not destroyed while the process lives; Slint skips rendering while no native window exists | [VERIFIED: androidwindowadapter.rs:516-527]; [CITED: activity-lifecycle onStop "the Activity object is kept resident in memory"] |
| In-process recreation observed? | Yes, by toggling bold text (missing `fontWeightAdjustment`) on API 36 emulator | emulator probe (this session) |
| "Don't keep activities" via `settings put global always_finish_activities 1` | Did **not** take effect in the probe (launch was HOT, one `dimuli started`); the developer-options UI toggle is needed for that test | emulator probe (this session) |
| Results after recreation | Background results use `post` -> `invoke_from_event_loop` -> the **new** thread's `CURRENT` controller (thread-local) | [VERIFIED: controller.rs:21-39] |

## Memory Budget (PERF-01, D-10)

Host benchmark this session (release build, `image` 0.25.10, zune-jpeg; scratch crate, not in the repo):

| Source | Decode | Thumbnail to 2048 | Peak RSS |
|---|---|---|---|
| 8160x6144 JPEG (50 MP) | 109 ms | 187 ms | 158 MB |
| 10000x8000 JPEG (80 MP) | 188 ms | 257 ms | 247 MB |

Estimated peak per stage on a device (decoded JPEG is RGB8, 3 bytes/pixel):
- **Kotlin transcode of a 64 MP source:** `ARGB_8888` bitmap 256 MB in the native heap ("In Android 8.0 (API level 26), and higher, the bitmap pixel data is stored in the native heap" [CITED: developer.android.com/topic/performance/graphics/manage-memory]) plus JPEG output stream; one image at a time on the single `io` thread. 10-bit HEIF may decode to a wider config (up to 8 bytes/pixel) [ASSUMED] — the 64 MP cap then means ~512 MB; set the sRGB target colour space and measure.
- **Rust load (per image, sequential):** decoded <= 64 MP RGB8 = 192 MB + preview <= 12.6 MB; previews retained: 9 x <= 12.6 MB = 113 MB.
- **Rust export:** one decoded source (<= 192 MB) + cell + 3000 px canvas (27 MB RGB for 1:1) + JPEG bytes.
- Expected app peak around 350-450 MB with 9 large photos. Emulators have `hw.ramSize=2G` (MemAvailable ~1.1 GB measured), so the 2 GB AVDs are the tighter test.
- Device timing estimate: 3-5x host for a 50 MP decode, i.e. roughly 1-1.5 s per image with the busy overlay [ASSUMED].

**`DECODE_MAX_ALLOC`:** currently `pub const DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;` [VERIFIED: decode.rs:17]. With 64 MP, an 8-bit RGBA decode is 256,000,000 bytes, below 256 MiB; 16-bit sources would still be allowed up to 512 MiB. Lowering to 256 MiB bounds the worst case (Open Question 3).

## Common Pitfalls

### Pitfall 1: Activity recreated in-process by an unhandled configuration change
**What goes wrong:** bold text, a SIM change (mcc/mnc), display colour mode or grammatical gender recreate `MainActivity`; `android_main` restarts; the collage is gone.
**Why:** `configChanges` lacks `fontWeightAdjustment|colorMode|grammaticalGender|touchscreen|mcc|mnc` (verified by probe for `fontWeightAdjustment`).
**How to avoid:** add them (Pattern 4); the restart notice covers anything left.
**Warning signs:** two `dimuli started` lines with the same PID in `adb logcat -s dimuli`.

### Pitfall 2: Late background results land in a fresh state
**What goes wrong:** an export or load started before a recreation finishes afterwards; `finish_export` sets `busy = None` and returns the JPEG (it would save a collage the user no longer sees), `finish_loading` opens an editor nobody asked for.
**Why:** `post` resolves the controller on the current UI thread [VERIFIED: controller.rs:28-39]; `finish_*` have no busy guard [VERIFIED: state.rs:225-230, 293-294].
**How to avoid:** `finish_loading` only when `busy == Some(Busy::Loading)`, `finish_export` only when `busy == Some(Busy::Exporting)`; otherwise log and drop. Host tests.

### Pitfall 3: Picker result arrives before natives / event loop exist (process death while the picker is open)
**What goes wrong:** after a process death the result is delivered to the new process; if `copyToCache` finishes before `android_main` registered natives (`UnsatisfiedLinkError`, caught and logged by `native {}`) or before Slint set its proxy (`invoke_from_event_loop` error, logged), the pick is lost.
**Why:** `onActivityResult` runs before `onResume` while android_main is still starting.
**How to avoid:** accept (window is milliseconds; the copy itself takes longer) and verify once on the emulator (`am kill` while the picker is open). Do not add a queue unless the test fails.

### Pitfall 4: Rounding/edge mismatch between Kotlin and Rust limits
**What goes wrong:** Kotlin scales to ~64 MP with rounding up, or keeps a 20000x3000 panorama (60 MP) whose edge exceeds `DECODE_MAX_EDGE` (16384) [VERIFIED: decode.rs:15 `pub const DECODE_MAX_EDGE: u32 = 16_384;`]; Rust rejects it as unreadable.
**How to avoid:** Kotlin's condition is "pixels > 64 MP **or** max edge > 16384", target computed with `floor`; define the pixel limit identically on both sides (64_000_000).

### Pitfall 5: Transparent WebP/AVIF/HEIF become black
**What goes wrong:** JPEG has no alpha; a translucent source transcoded to JPEG gets black (premultiplied) areas instead of the collage background [ASSUMED behaviour of `Bitmap.compress(JPEG)` on alpha bitmaps].
**How to avoid:** D-01 locks JPEG; if the owner agrees, write PNG when `bitmap.hasAlpha()` (Rust already composites PNG alpha over the background). Open Question 2.

### Pitfall 6: Wide-gamut / HDR sources
**What goes wrong:** HEIF from recent phones may be Display P3 or 10-bit; without a target colour space the JPEG carries an ICC profile that the `image` crate ignores -> shifted colours; 10-bit configs use more memory.
**How to avoid:** `decoder.setTargetColorSpace(ColorSpace.get(ColorSpace.Named.SRGB))` in the header listener; verify colours and memory with a real HEIC from the owner's phone during the final test [ASSUMED effect on bitmap config].

### Pitfall 7: Emulator HEVC decoder limits for synthetic HEIC files
**What goes wrong:** ImageMagick/libheif 1.12 writes a single (non-grid) HEVC image; phone HEICs are 512 px grids. A large single-image HEIC may exceed the emulator's software HEVC decoder limits and fail [ASSUMED].
**How to avoid:** use synthetic HEIC files of at most 1920x1440 for the automated emulator test; test real phone HEICs (12-50 MP grids) in the owner's device test.

### Pitfall 8: Slint Android backend panics on mouse-wheel input
Already known from Phase 5 (`MotionAction::Scroll => todo!()`); never send scroll events in emulator automation [VERIFIED in 05-RESEARCH Pitfall 5].

### Pitfall 9: Photo Picker does not show freshly pushed files
`adb shell am force-stop com.google.android.photopicker` after `adb push` and wait a few seconds [VERIFIED in 05-RESEARCH Pitfall 7].

### Pitfall 10: Smoke script false failures (N2, N3)
`adb install … | grep -q Success` under `set -o pipefail` can fail by SIGPIPE; capture output first (`out="$(adb install -r "$APK" 2>&1)"; grep -q Success <<<"$out"`). The start screenshot is taken before the first Slint frame; wait ~2 s before `screencap` [VERIFIED: smoke.sh:39, 56-57].

## Code Examples

### Pulling a boolean from Kotlin (jni 0.22.4)
```rust
// Source: jni-0.22.4/src/env.rs:2896-2905 (raw signatures are accepted by jni_sig!) and
// jvalue.rs:128-130 (`z()` -> bool). Called once in android_main after register_natives.
fn collage_lost_on_restart(&self) -> bool {
    let result = self.with_activity(|env, activity| {
        env.call_method(activity, jni_str!("collageLostOnRestart"), jni_sig!("()Z"), &[])?.z()
    });
    result.unwrap_or_else(|error| {
        log::error!("cannot read the restart flag: {error}");
        false
    })
}
```

### Kotlin side of the restart flag
```kotlin
// region JNI bridge
/** Rust: `collageLostOnRestart() -> Boolean` (`()Z`), read once by android_main at start. */
fun collageLostOnRestart(): Boolean = collageLost
/** Rust: `setCollageOpen(open: Boolean)` (`(Z)V`) on every Start/Editor change; only stores the value. */
fun setCollageOpen(open: Boolean) { collageOpen = open }
// endregion

override fun onCreate(savedInstanceState: Bundle?) {
    // Before super.onCreate: NativeActivity starts android_main from there, which may read the flag at once.
    collageLost = savedInstanceState?.getBoolean(KEY_COLLAGE_OPEN, false) == true
    super.onCreate(savedInstanceState)
    …
}

override fun onSaveInstanceState(outState: Bundle) {
    super.onSaveInstanceState(outState)
    outState.putBoolean(KEY_COLLAGE_OPEN, collageOpen)
}
```

### Auto-hide arming in the controller
```rust
// Source: slint 1.18.1 Timer::start restarts a running timer (i-slint-core-1.18.1/timers.rs:80-82).
fn arm_message_timer(self: &Rc<Self>) {
    let seq = match &self.state.borrow().message {
        Some(m) if m.kind == MessageKind::Success => m.seq,
        _ => return self.message_timer.stop(),
    };
    if self.armed_seq.replace(Some(seq)) == Some(seq) {
        return; // already armed for this message
    }
    let weak = Rc::downgrade(self);
    self.message_timer.start(slint::TimerMode::SingleShot, MESSAGE_HIDE_AFTER, move || {
        if let Some(c) = weak.upgrade() {
            c.update(|s| s.expire_message(seq));
        }
    });
}
```

### Render tests at <= 64 px (POLI-02)
Offending calls today [VERIFIED: render.rs test module, lines 428, 453, 569, 588]: `compose(&collage, &sources, opts(200))`, `opts(101)`, `render(&collage, &sources, opts(100))` (asserts `(80, 100)`), `opts(90)` (asserts width 90). Change to <= 64 (e.g. `opts(64)`, `opts(63)` for the odd-size gap test, `opts(60)` with expected `(48, 60)` for 4:5, `opts(63)` for nine images) and update assertions. `output_size(&collage, 1000)` and `output_size(&collage, u32::MAX)` only compute sizes, no rendering; they may stay.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `BitmapFactory` decode + manual EXIF rotation | `ImageDecoder` with orientation handling inside hwui (CTS `ImageDecoderTest#testRespectOrientation`) | Android 9 (API 28) / hwui change 139145be | Transcoded copies come out upright (D-02) [CITED: AOSP commit 139145be mirror on gitlab.e.foundation] — verify on emulator with a rotated source |
| Bitmap pixels in the Java heap | Bitmap pixels in the native heap | Android 8.0 (API 26) | Java heap growth limit (192 MB on the AVDs) does not cap transcoding; system RAM does |
| No AVIF | AVIF decode in the platform | Android 12 (API 31) | Covered by minSdk 33 "where the device supports it" [CITED: secondhand, libavif.web.app and press; not in official reference] |

**Deprecated/outdated:** `onActivityResult` is deprecated but required (no AndroidX, Phase 5 D-09); N1 suggests `@Suppress("OVERRIDE_DEPRECATION")` instead of `@Deprecated` [VERIFIED: MainActivity.kt:196].

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `ImageDecoder` applies EXIF/HEIF orientation for full decodes of JPEG and HEIF | Pattern 1, State of the Art | Transcoded images sideways; fix by applying `ExifInterface` rotation (platform API) — verify with a rotated large JPEG on the emulator |
| A2 | `ImageInfo.getSize()` and `setTargetSize` both use the oriented size | Pattern 1 | Distorted aspect for rotated > 64 MP sources; verify with the same test |
| A3 | `Bitmap.compress(JPEG)` writes no EXIF and drops alpha to black | Pattern 1, Pitfall 5 | Double rotation in Rust (if EXIF kept) or black areas; verify by pulling a transcoded copy (`run-as`) and inspecting it |
| A4 | `setTargetColorSpace(SRGB)` keeps 10-bit HEIF at 8-bit config | Pitfall 6, Memory | Up to ~512 MB per 64 MP HEIF; measure with a real phone HEIC |
| A5 | Synthetic single-image HEIC > ~2 MP may fail in the emulator HEVC decoder | Pitfall 7 | Only test-design risk |
| A6 | Rust OOM aborts; Android usually kills by LMK before `malloc` fails | Pattern 3 | ERRM text for OOM not reachable anyway |
| A7 | `native_method!` accepts `failed: jint` like `jni_sig!` does | Pattern 2 | Compile error only; use `i32` per macro docs |
| A8 | `BitmapFactory.Options.inJustDecodeBounds` reads JPEG/PNG headers without decoding pixels | Pattern 1 | Slow copy step for large JPEGs only |
| A9 | Device decode time 3-5x host | Memory | Longer busy overlay; no correctness impact |
| A10 | `run-as ch.staeheli.dimuli cat /proc/<pid>/status` (VmHWM) works on the debug APK for peak-RSS measurement | Validation | Fall back to polling `dumpsys meminfo` |
| A11 | Relaunching from launcher/recents after `am kill` restores the task with `savedInstanceState` | Validation | Use recents (`KEYCODE_APP_SWITCH`) instead of `am start` for the test |

## Open Questions (RESOLVED)

1. **Restart notice precision.** — RESOLVED: owner chose the `collageOpen` variant (CONTEXT D-15); implemented in 06-05.
   - What we know: D-06 says `savedInstanceState != null`; that also fires when no collage was open.
   - Recommendation: use the `collageOpen` flag variant (Pattern 5); it still detects in Kotlin. Planner may choose the minimal variant if the owner prefers.
2. **Alpha sources transcoded to JPEG (Pitfall 5).** — RESOLVED: owner chose PNG when `bitmap.hasAlpha()` (CONTEXT D-16); implemented in 06-04 Task 1 (`hasAlpha` gate).
   - What we know: D-01 locks JPEG q95; transparent WebP/AVIF/HEIF would get black areas.
   - Recommendation: ask the owner whether PNG for `bitmap.hasAlpha()` is acceptable; otherwise accept and document.
3. **`DECODE_MAX_ALLOC` 512 MiB -> 256 MiB.** — RESOLVED: owner approved 256 MiB (CONTEXT D-17); implemented in 06-01.
   - What we know: with 64 MP, 8-bit sources fit in 256 MiB; 16-bit sources above ~44 MP would then be "too large".
   - Recommendation: lower it with D-08 and map `Limits` errors to the "too large" text; needs an owner nod because it is a second constant.
4. **Edge limit error classification.** — RESOLVED: owner chose "too large" (CONTEXT D-18); implemented in 06-01 (test updated).
   - What we know: `edge_beyond_limit_is_rejected_as_decode_error` asserts `ImageDecode` for an edge above 16384 [VERIFIED: decode.rs:193-200].
   - Recommendation: optional; add an explicit edge check after `dimensions()` returning `ImageTooLarge` (and update that test) so desktop users get the "too large" text. On Android Kotlin prevents it.
5. **Banner colour for success.** — RESOLVED: planner/executor discretion (UI-only). Reuse `Theme.notice` or add a `Theme.success`; UI-only, no logic.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | all | ✓ | rustc 1.99.1 | — |
| cargo-ndk | Android build | ✓ | 4.1.2 | — |
| Android SDK / build-tools / NDK | Android build, check-apk | ✓ | build-tools 36.0.0, 37.0.0; NDK 30.0.16248370 | — |
| JDK | Gradle | ✓ | OpenJDK 25.0.5 | — |
| Emulator AVDs | picker/gallery/share/lifecycle automation, API matrix | ✓ | dimuli_api33/36/37, 2 GB RAM each; `emulator-5554` (API 36) running | — |
| Real device 8200fcc9 (Android 14) | smoke/insets, memory on real hardware | ✗ (not attached right now) | — | re-attach via `usbipd.exe attach --wsl --busid 1-1` (README) |
| Real Android 16 device | ACPT-01 | owner-held | — | none (owner checkpoint) |
| ImageMagick with HEIC/AVIF/WebP write | test files | ✓ | IM6, libheif 1.12.0, libwebp 1.2.2 | — |
| ffmpeg | 50/80 MP test JPEGs | ✓ | system | ImageMagick (slower) |

**Missing dependencies with no fallback:** Android 16 device (owner provides at the final checkpoint).
**Missing dependencies with fallback:** Android 14 device (re-attach; emulator meanwhile).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), 86 core + 104 app tests green today (~1 s warm) |
| Config file | none (workspace `Cargo.toml` lints) |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace && ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` |
| Android gates | `android/gradlew -p android assembleDebug && android/scripts/check-apk.sh`; device: `android/gradlew -p android assembleDebug -PrustAbis=arm64-v8a && android/scripts/smoke.sh -s 8200fcc9` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PERF-01 | 64 MP limit: 8000x8000 header accepted, 8001x8000 `ImageTooLarge` | unit | `cargo test -p collage-core decode::tests::pixel_budget` | ❌ Wave 0 (add boundary test next to `pixel_budget_exceeded_is_image_too_large`) |
| PERF-01 | Preview: RGB8 -> RGBA buffer, alpha 255, values preserved, one buffer | unit | `cargo test -p dimuli-app images::tests::preview` | ❌ Wave 0 |
| PERF-01 | Gesture path updates only the panned cell's row; templates untouched | unit | `cargo test -p dimuli-app view::tests::gesture` | ❌ Wave 0 |
| PERF-01 | 50 MP JPEG, 80 MP JPEG (Kotlin downscale), HEIC/WebP/AVIF load, edit, export on emulator; peak memory recorded | e2e (emulator) | push files, pick via uiautomator, then `adb logcat -d -s dimuli \| grep -c "skipping"` = 0 and `picked images: 5`; VmHWM via `run-as` | ❌ Wave 0 (scripted steps in plan) |
| PERF-01 | Gesture smoothness with large photos | manual (owner device) | — | owner checkpoint |
| ERRM-01 | Each failure path maps to its own German text (too large, Kotlin-failed count, all failed -> NO_IMAGES_LOADED, restart) | unit | `cargo test -p dimuli-app state::tests texts::tests native_events::tests` | ❌ Wave 0 |
| ERRM-01 | Success auto-hides only its own `seq`; errors stay | unit | `cargo test -p dimuli-app state::tests::expire` | ❌ Wave 0 |
| ERRM-01 | Kotlin transcode failure counted (garbage `.heic` pushed) | e2e (emulator) | logcat `Log.e` line + banner screenshot | ❌ Wave 0 |
| LIFE-01 | Late load/export results ignored when not busy | unit | `cargo test -p dimuli-app state::tests::late` | ❌ Wave 0 |
| LIFE-01 | Restart flag -> persistent restart text at start | unit | `cargo test -p dimuli-app state::tests::restart` | ❌ Wave 0 |
| LIFE-01 | HOME + relaunch, app switcher, picker return, share-sheet return keep the editor; exactly one `dimuli started` | e2e (emulator) | `test "$(adb -s emulator-5554 logcat -d -s dimuli \| grep -c 'dimuli started')" -eq 1` + editor screenshot | ❌ Wave 0 |
| LIFE-01 | `font_weight_adjustment` toggle no longer recreates the activity | e2e (emulator) | same count check after `settings put secure font_weight_adjustment 300` / `settings delete …` | ❌ Wave 0 |
| LIFE-01 | Process death (`am kill` after HOME with collage open) -> restart notice | e2e (emulator) | relaunch from recents, screenshot shows banner | ❌ Wave 0 |
| LIFE-01 | `configChanges` complete | static | `aapt2 dump xmltree --file AndroidManifest.xml app-debug.apk \| grep configChanges` or grep on manifest for `fontWeightAdjustment` | ❌ Wave 0 (could be added to `check-apk.sh`) |
| POLI-01 | `select_aspect` updates canvas size and cells in one transition | unit | `cargo test -p dimuli-app state::tests::aspect` and `view::tests::fit_canvas` | ❌ Wave 0 |
| POLI-01 | No visible flicker | manual (desktop `cargo run`, device) | — | manual |
| POLI-02 | No render test above 64 px | static | `! grep -nE "opts\((6[5-9]\|[7-9][0-9]\|[1-9][0-9]{2,})\)" crates/collage-core/src/render.rs` | ✅ (grep) |
| POLI-02 | Desktop share: no export job, unavailable text | unit | `cargo test -p dimuli-app state::tests::share_unsupported` | ❌ Wave 0 |
| ACPT-01 | Android 16: 1-9 images, save, visible in Google Photos; plus 05-07 Task 3 steps on 14 and 16 | manual | owner `checkpoint:human-verify` | — |

### Sampling Rate
- **Per task commit (no commits without request; per task):** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`; tasks touching `platform/android.rs` add the `cargo ndk … clippy` command; Kotlin tasks add `android/gradlew -p android assembleDebug -PrustAbis=x86_64`.
- **Per wave merge:** full suite + `assembleDebug` + `check-apk.sh` + `smoke.sh` on the device (if attached) or `emulator-5554`.
- **Phase gate:** emulator flows (large images, formats, lifecycle, restart notice) on dimuli_api36, smoke on dimuli_api33 and dimuli_api37, code-reviewer run, then the owner's device checkpoint.

### Wave 0 Gaps
- [ ] Boundary tests for the 64 MP limit (`decode.rs`), keep the existing 101 MP test
- [ ] `images.rs` preview conversion tests (RGB, RGBA, gray fallback)
- [ ] `state.rs` tests: message seq/expiry, failure counts (too large vs unreadable vs Kotlin-failed), late-result guards, restart notice, share unsupported, aspect/canvas
- [ ] `native_events.rs` tests for the `failed` count mapping
- [ ] `view.rs` tests: `fit_canvas`, gesture row update
- [ ] Emulator test assets: `big50.jpg` (8160x6144), `big80.jpg` (10000x8000), `small.heic` (<= 1920x1440), `t.webp`, `t.avif`, a rotated JPEG > 64 MP (`convert … -orient RightTop` on a downscaled copy, or EXIF via the existing test helper), a garbage `.heic`
- [ ] Emulator lifecycle script steps (HOME/app switcher/share back/am kill) — can live in the plan or a new `android/scripts/lifecycle.sh`

## Security Domain

### Applicable ASVS Categories (Level 1)

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (share provider) | `ShareProvider` not exported, whole-URI comparison, read-only; N5 changes only the missing-file case |
| V5 Input Validation | yes | Untrusted image files: platform `ImageDecoder` (OS-patched) + Rust `image::Limits` (64 MP, 16384 edge, alloc limit); cache names never derived from URIs |
| V6 Cryptography | no | — |
| V8 Data Protection | yes | Transcoded copies drop EXIF (GPS) [ASSUMED A3]; byte-copied JPEGs keep EXIF only in app-private `cacheDir/picked`; the exported collage is encoded fresh by `JpegEncoder` without EXIF |
| V7 Logging | yes | Log causes and counts, no image contents; avoid logging content URIs where possible (existing `Log.e(TAG, "copy failed", e)` may include the URI in the exception message — acceptable, T-05-10) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Decompression bomb (huge dimensions) | Denial of service | Kotlin downscale before Rust; Rust limits checked before allocation |
| Malformed HEIC/WebP exploiting a codec | Elevation / tampering | Platform codecs updated via OS/Mainline; no bundled C decoder (D-04) |
| OOM crash via many large images | Denial of service | Sequential decode, previews capped at 2048 px, alloc limit, restart notice |
| Stale share file read by another app later | Information disclosure | File only reachable via temporary grant; N4 cleans `cacheDir/share` at process start |

## Sources

### Primary (HIGH confidence)
- Local sources read this session: `android-activity-0.6.1/src/native_activity/glue.rs`, `i-slint-backend-android-activity-1.18.1/{lib.rs,androidwindowadapter.rs}`, `i-slint-core-1.18.1/{context.rs,platform.rs,window.rs,timers.rs,sharedvector.rs,graphics/image.rs}`, `slint-1.18.1/android.rs`, `image-0.25.10/src/io/limits.rs`, `jni-0.22.4/src/{env.rs,jvalue.rs}`
- Project files: `MainActivity.kt`, `ShareProvider.kt`, `AndroidManifest.xml`, `platform/{mod,android,desktop,native_events}.rs`, `controller.rs`, `state.rs`, `view.rs`, `images.rs`, `texts.rs`, `decode.rs`, `render.rs`, `ui/*.slint`, `smoke.sh`, `docs/*`, Phase 5 review/summary/research
- Emulator probes (API 36, `emulator-5554`): in-process recreation via `font_weight_adjustment`; `always_finish_activities` via `settings put` ineffective
- Host benchmark: 50 MP / 80 MP JPEG decode + thumbnail (scratch crate, image 0.25.10)

### Secondary (MEDIUM confidence)
- https://developer.android.com/reference/android/graphics/ImageDecoder — allocators, `setTargetSize`, `setTargetSampleSize`, header-listener setters
- https://developer.android.com/guide/components/activities/activity-lifecycle — process vs activity kill, `savedInstanceState`
- https://developer.android.com/guide/topics/manifest/activity-element — `configChanges` values
- https://developer.android.com/topic/performance/graphics/manage-memory — bitmap pixels in native heap since API 26
- https://gitlab.e.foundation/e/os/android_frameworks_base/-/commit/139145be8082f0606045b89feebc9126a9a376d8 — "Handle EXIF orientation in hwui/ImageDecoder" (AOSP mirror; `testRespectOrientation`)

### Tertiary (LOW confidence)
- https://libavif.web.app/article/how-android-integrates-libavif-for-native-avif and press coverage — AVIF in Android 12+
- https://developer.android.com/ndk/guides/image-decoder — format list (no orientation statement)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; APIs verified in local sources or official docs
- Architecture: HIGH for lifecycle and Slint internals (source + probe); MEDIUM for the Kotlin transcode details (docs + AOSP mirror, needs emulator confirmation A1-A4)
- Pitfalls: HIGH for 1, 2, 4, 8-10; MEDIUM for 3, 5-7

**Research date:** 2026-11-14
**Valid until:** 2026-12-14 (locked crate versions; Android platform behaviour stable)
