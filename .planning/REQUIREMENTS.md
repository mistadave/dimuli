# Requirements: dimuli

**Defined:** 2026-11-13
**Core Value:** A user can turn a handful of photos into one well-arranged collage image and get it into their gallery in a few taps.

Source: derived from `docs/PLAN.md` (v1 scope, milestones, verification plan), `docs/ARCHITECTURE.md` (behavior, limits) and `docs/CODING_GUIDELINES.md`. No PRD with acceptance criteria exists; requirement wording and the verification criteria in the roadmap are derived, not quoted. Requirements delivered by docs/PLAN.md milestones 1-4 are marked complete (verified by unit tests and desktop runs, not yet on Android).

## v1 Requirements

Requirements for the first release. v1 success metric: on a real Android 16 device, a user can create a collage from 1-9 images and save it to the gallery.

### Developer Environment

- [x] **ENV-01**: The WSL2 toolchain for Android work is in place: Android SDK + NDK r30, emulator AVDs `dimuli_api36` and `dimuli_api37` running with KVM, Rust targets `aarch64-linux-android` and `x86_64-linux-android`, `cargo-ndk`, and `ANDROID_HOME` / `ANDROID_NDK_HOME` set
- [x] **ENV-02**: A developer can preview the full app on the desktop with `cargo run -p dimuli-app -- a.jpg b.png` (images as CLI arguments, no file dialog)
- [ ] **ENV-03**: `CLAUDE.md` documents the `cargo ndk` + Gradle build, install and log flow and the project structure (including `android/`), and no longer lists `cargo-apk` commands

### Layout Engine (collage-core)

- [x] **LAYT-01**: Templates exist for every image count from 1 to 9 (balanced grids, plus "large image left/top" variants for 3-5), and a different template can be applied to an existing collage while keeping all images
- [x] **LAYT-02**: Cell rectangles are computed in collage units from a binary split tree; they never overlap, spacing is exact, and equal ratios yield equally sized cells
- [x] **LAYT-03**: Dragging a divider changes only its split ratio and clamps so that no cell becomes narrower than 5 % of the collage width
- [x] **LAYT-04**: Every image fills its cell in cover mode: zoom is limited to 1.0-5.0, panning keeps the visible section inside the image, and one crop function serves both preview and export
- [x] **LAYT-05**: Two cells' images can be swapped, with hit-testing to find the target cell

### Editor (desktop UI)

- [x] **EDIT-01**: Given 1-9 images, the app loads them in the background with previews of at most 2048 px (loading state shown) and opens the editor with the first fitting template; extra images beyond 9 are dropped with a visible hint
- [x] **EDIT-02**: User can drag dividers with mouse or touch using handles with a touch target of at least 48 px
- [x] **EDIT-03**: User can pan an image inside its cell by dragging and zoom it by pinch or mouse wheel (around the gesture center / pointer)
- [x] **EDIT-04**: User can swap two images by long-pressing one (500 ms) and dragging it onto another cell
- [x] **EDIT-05**: User can switch the template of the current collage from a template bar below the canvas
- [x] **EDIT-06**: User can set spacing, padding, corner radius and background color in an expandable settings panel; values are clamped to valid limits so 9 cells stay visible
- [x] **EDIT-07**: User can choose the aspect ratio 1:1, 4:5, 9:16 or 16:9
- [x] **EDIT-08**: Back in the editor asks for confirmation before returning to the start screen (the collage is lost); back on the start screen is reported as not handled so the platform can close the app; during export back has no effect

### Licensing

- [x] **LICN-01**: The start screen shows the Slint attribution (`AboutSlint`) as required by the Slint Royalty-free License 2.0

### Export

- [x] **EXPT-01**: User can export the collage as a JPEG (3000 px long edge, quality 90) rendered off the UI thread behind a busy overlay that swallows input; the result matches the preview crop
- [x] **EXPT-02**: The renderer handles real-world sources safely: EXIF orientation, alpha (premultiplied scaling), 16-bit/grayscale input, corner radius with antialiasing, background alpha blended over white, images decoded one at a time with a 100 MP decode limit
- [x] **EXPT-03**: On the desktop, saving writes `dimuli-<unix-seconds>.jpg` into the working directory without ever overwriting an existing file
- [x] **EXPT-04**: When export fails, the user sees a German message that distinguishes a missing/unreadable source image from a general failure, and the error is logged

### Android Platform

- [ ] **ANDR-01**: A Gradle project under `android/` (Kotlin `MainActivity : NativeActivity`, manifest, minSdk 33 / targetSdk 37) packages `libdimuli.so` built by `cargo ndk` for `arm64-v8a` and `x86_64`; the APK installs and launches to the start screen on emulators API 33, API 36 and API 37 (new AVD `dimuli_api33`) and on real devices with Android 14 and Android 16
- [ ] **ANDR-02**: User can pick up to 9 images with the system Photo Picker; the selected URIs are copied into the app cache and delivered to Rust via the JNI bridge; cancelling the picker leaves the start screen unchanged
- [ ] **ANDR-03**: User can save the collage to the gallery through `MediaStore.Images` (`Pictures/dimuli`) without any storage permission, and sees a success or error message
- [ ] **ANDR-04**: User can share the collage via the system share sheet (cache file, own minimal `ContentProvider` instead of AndroidX `FileProvider`, `ACTION_SEND`), and sees an error message if sharing fails
- [ ] **ANDR-05**: The UI is edge-to-edge on every supported version (enabled explicitly on Android 13/14, where it is not enforced): system-bar insets reach the UI through Slint's built-in Android `safe-area-insets` (Kotlin only enables edge-to-edge on Android 13/14), so no content is covered by status or navigation bars
- [ ] **ANDR-06**: The back gesture works (Predictive Back callback forwarded to Rust): in the editor it asks for confirmation and returns to the start screen, on the start screen it closes the app
- [ ] **ANDR-07**: The native library is 16 KB page aligned, verified with `zipalign -c -P 16` or the APK Analyzer
- [ ] **ANDR-08**: All editor interactions work with fingers on Android (divider drag, pinch zoom, pan, long-press swap, template bar, settings panel), including the swallowed-input busy overlay during export

### Robustness and Polish

- [ ] **LIFE-01**: Pausing and resuming the app (home button, app switcher, returning from Photo Picker or share sheet) keeps the current collage and all edits
- [ ] **PERF-01**: Large photos (e.g. 12 MP and above from a real device camera) load, edit and export without crash, out-of-memory or visible gesture stutter; pointer events during a gesture update only the affected cells (no full snapshot allocation); loading avoids the unnecessary pixel-buffer copy; the `DECODE_MAX_PIXELS` value for devices (100 MP vs 64 MP) is decided and recorded
- [ ] **ERRM-01**: Every failure path (unreadable image, pick failure, export, save, share) shows a short German message that says what happened and what the user can do, without technical details, and logs the error with `log::error!`
- [ ] **POLI-01**: Changing the aspect ratio shows no intermediate one-frame flicker (M3 review note)
- [x] **POLI-02**: Render tests use output sizes of at most 64 px as required by the coding guidelines, and desktop "Teilen" reports "nicht verfügbar" immediately instead of rendering the collage first (M4 review notes)
- [ ] **ACPT-01**: On a real Android 16 device, a user can create a collage from 1-9 images and save it to the gallery, and the saved image appears in the gallery app (v1 success metric)

## Post-v1 Requirements (committed, Phase 8)

Committed follow-up work from `docs/PLAN.md` milestone 8. Out of scope for the v1 success metric (1-9 images) but kept as a roadmap phase.

### Many Images

- [ ] **MANY-01**: A grid generator in `templates` produces a valid layout (no overlaps, correct spacing) for any image count above 9, in addition to the existing templates for 1-9
- [ ] **MANY-02**: The fixed `MAX_IMAGES` limit of 9 is raised or removed (goal: no fixed limit, cells simply get smaller): picking and loading accept more than 9 images and the "too many images" hint no longer appears
- [ ] **MANY-03**: Memory use and usability with many images are verified in the emulator (loading, dividers, pan/zoom, swap and export remain operable and stable)

## v2 Requirements

Deferred to a future release. Tracked but not in the current roadmap. The architecture does not block any of these.

### Project Management

- **PERS-01**: User can save a collage as a project and reopen it later
- **UNDO-01**: User can undo and redo edits (stack of `Collage` clones in the controller)

### Content

- **TEXT-01**: User can add text and stickers to a collage
- **FILT-01**: User can apply filters to images
- **FREE-01**: User can arrange images free-form instead of in template cells

### Localization

- **I18N-01**: UI is available in languages besides German (texts already prepared via `@tr("...")`)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Android versions below 13 | minSdk 33; the system Photo Picker (`ACTION_PICK_IMAGES`), MediaStore without permission, `FileProvider` and `OnBackInvokedCallback` are available everywhere, no fallbacks needed |
| Storage permissions / extra manifest permissions | Not required at minSdk 33; guidelines forbid extra permissions without asking |
| Native desktop file dialog (`rfd`) | Needs GTK3 or an xdg portal under WSL; CLI arguments are used instead |
| Persistence in v1 | Collage lives only during the session; saves a storage format and migration (see PERS-01 in v2) |

## Traceability

Which phases cover which requirements. Phase N corresponds to `docs/PLAN.md` milestone N.

| Requirement | Phase | Status |
|-------------|-------|--------|
| ENV-01 | Phase 1 | Complete |
| LAYT-01 | Phase 2 | Complete |
| LAYT-02 | Phase 2 | Complete |
| LAYT-03 | Phase 2 | Complete |
| LAYT-04 | Phase 2 | Complete |
| LAYT-05 | Phase 2 | Complete |
| ENV-02 | Phase 3 | Complete |
| EDIT-01 | Phase 3 | Complete |
| EDIT-02 | Phase 3 | Complete |
| EDIT-03 | Phase 3 | Complete |
| EDIT-04 | Phase 3 | Complete |
| EDIT-05 | Phase 3 | Complete |
| EDIT-06 | Phase 3 | Complete |
| EDIT-07 | Phase 3 | Complete |
| EDIT-08 | Phase 3 | Complete |
| LICN-01 | Phase 3 | Complete |
| EXPT-01 | Phase 4 | Complete |
| EXPT-02 | Phase 4 | Complete |
| EXPT-03 | Phase 4 | Complete |
| EXPT-04 | Phase 4 | Complete |
| ANDR-01 | Phase 5 | Pending |
| ANDR-02 | Phase 5 | Pending |
| ANDR-03 | Phase 5 | Pending |
| ANDR-04 | Phase 5 | Pending |
| ANDR-05 | Phase 5 | Pending |
| ANDR-06 | Phase 5 | Pending |
| ANDR-07 | Phase 5 | Pending |
| ANDR-08 | Phase 5 | Pending |
| LIFE-01 | Phase 6 | Pending |
| PERF-01 | Phase 6 | Pending |
| ERRM-01 | Phase 6 | Pending |
| POLI-01 | Phase 6 | Pending |
| POLI-02 | Phase 6 | Complete |
| ACPT-01 | Phase 6 | Pending |
| ENV-03 | Phase 7 | Pending |
| MANY-01 | Phase 8 | Pending |
| MANY-02 | Phase 8 | Pending |
| MANY-03 | Phase 8 | Pending |

**Coverage:**
- v1 requirements: 35 total (20 complete, 15 pending)
- Post-v1 committed requirements: 3 total (3 pending)
- Mapped to phases: 38 of 38
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-13*
*Last updated: 2026-11-13 after initial roadmap creation (complete = delivered by PLAN.md milestones 1-4)*
