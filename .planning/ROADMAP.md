# Roadmap: dimuli

## Overview

dimuli turns 1 to n photos into a single collage image on Android. The journey so far built everything that can be proven on the host: the WSL2 Android toolchain, the pure-Rust layout engine, the Slint editor with desktop preview, and the export renderer (Phases 1-4, complete). What remains is bringing it to a real phone: the Android shell with Photo Picker, MediaStore saving and sharing (Phase 5), polish for lifecycle, large images and error messages ending in the v1 acceptance test on a real Android 16 device (Phase 6), refreshing the developer docs for the new build flow (Phase 7), and lifting the 9-image limit (Phase 8, a committed follow-up beyond the v1 success metric).

Phase N corresponds to `docs/PLAN.md` milestone N (milestone 0, "record the plan", is the planning baseline and not a phase). Phases 1-4 were delivered before GSD adoption and are recorded here as complete work; they have no GSD plan files.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Tooling Setup** - WSL2 toolchain for Android: SDK, NDK r30, emulators with KVM, Rust targets, cargo-ndk
- [x] **Phase 2: Workspace and Layout Engine** - Cargo workspace and `collage-core`: templates, geometry, divider dragging, cover transform, swapping (completed 2026-11-13)
- [x] **Phase 3: Desktop Editor** - Slint UI with desktop preview: loading, editor, dividers, pan/zoom, swap, settings (completed 2026-11-13)
- [x] **Phase 4: Export Renderer** - JPEG export in `collage-core` with pixel test; desktop save to file (completed 2026-11-13)
- [x] **Phase 5: Android Shell** - Gradle project, Kotlin activity, JNI bridge, Photo Picker, MediaStore, sharing; running in the emulator and on a device
- [x] **Phase 6: Polish and v1 Acceptance** - Lifecycle, performance with large images, error messages, open review notes, real-device acceptance
- [x] **Phase 7: Developer Docs for the Android Build** - Replace `cargo-apk` commands in `CLAUDE.md` with the `cargo ndk` + Gradle flow
- [x] **Phase 8: More Than 9 Images** - Grid generator for counts above 9; raise `MAX_IMAGES` to 36 (D-06)

## Phase Details

### Phase 1: Tooling Setup
**Goal**: A developer can build for and run Android from WSL2 (delivered as `docs/PLAN.md` milestone 1)
**Depends on**: Nothing (first phase)
**Requirements**: ENV-01
**Success Criteria** (what must be TRUE):
  1. `emulator -accel-check` reports KVM acceleration and `emulator -avd dimuli_api36` / `dimuli_api37` start in WSLg
  2. Android SDK (platform 37, build-tools 37.0.0, platform-tools) and NDK r30 `30.0.16248370` are installed under `~/Android/Sdk` with `ANDROID_HOME` / `ANDROID_NDK_HOME` set
  3. Rust targets `aarch64-linux-android` and `x86_64-linux-android` and `cargo-ndk` are installed
**Plans**: Pre-GSD (delivered as `docs/PLAN.md` milestone 1; no plan files)

### Phase 2: Workspace and Layout Engine
**Goal**: A host-testable `collage-core` crate computes collage layouts and image placement with no UI or Android dependency (delivered as `docs/PLAN.md` milestone 2)
**Depends on**: Phase 1
**Requirements**: LAYT-01, LAYT-02, LAYT-03, LAYT-04, LAYT-05
**Success Criteria** (what must be TRUE):
  1. For every image count 1-9 a template yields cell rectangles that do not overlap and respect spacing (unit-tested)
  2. Dragging a divider moves only its split and never makes a cell narrower than 5 % of the collage width (unit-tested)
  3. Pan and zoom keep the visible section inside the image (cover, zoom 1.0-5.0), and swapping exchanges the images of two cells (unit-tested)
**Plans**: Pre-GSD (delivered as `docs/PLAN.md` milestone 2; no plan files)

### Phase 3: Desktop Editor
**Goal**: Users can load images and edit a collage in a Slint UI that runs on the desktop (delivered as `docs/PLAN.md` milestone 3)
**Depends on**: Phase 2
**Requirements**: ENV-02, EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05, EDIT-06, EDIT-07, EDIT-08, LICN-01
**Success Criteria** (what must be TRUE):
  1. `cargo run -p dimuli-app -- a.jpg b.png ...` shows a loading state and then the editor with the images in the first fitting template
  2. Dividers can be dragged, images panned and zoomed (pinch / wheel), and two images swapped by long-press and drag
  3. Template bar, settings panel (spacing, padding, corner radius, background) and the four aspect ratios update the canvas live
  4. Back in the editor asks for confirmation before returning to the start screen, and the start screen shows the Slint attribution
**Plans**: Pre-GSD (delivered as `docs/PLAN.md` milestone 3; no plan files)
**UI hint**: yes

### Phase 4: Export Renderer
**Goal**: A collage can be rendered to a JPEG that looks like the preview, and saved as a file on the desktop (delivered as `docs/PLAN.md` milestone 4)
**Depends on**: Phase 3
**Requirements**: EXPT-01, EXPT-02, EXPT-03, EXPT-04
**Success Criteria** (what must be TRUE):
  1. Export on the desktop writes `dimuli-<unix-seconds>.jpg` (3000 px long edge, quality 90) that matches the on-screen collage, never overwriting an existing file
  2. During export a busy overlay is shown and all input is swallowed
  3. A pixel test compares the rendered output at defined locations on a generated test image
  4. If a source image is missing or unreadable, the user sees a German message that says so, distinct from the general failure message
**Plans**: Pre-GSD (delivered as `docs/PLAN.md` milestone 4; no plan files)

### Phase 5: Android Shell
**Goal**: dimuli runs as a real Android app: users can pick images with the system Photo Picker, edit by touch, and save to the gallery or share (`docs/PLAN.md` milestone 5)
**Depends on**: Phase 4 (and Phase 1 tooling)
**Requirements**: ANDR-01, ANDR-02, ANDR-03, ANDR-04, ANDR-05, ANDR-06, ANDR-07, ANDR-08
**Success Criteria** (what must be TRUE):
  1. The APK built with `cargo ndk` + Gradle (`arm64-v8a`, `x86_64`) installs with `adb install` and launches to the start screen on emulators API 33, API 36 and API 37, and on real devices with Android 14 and Android 16
  2. Tapping "Bilder wählen" opens the system Photo Picker; selecting 3-5 images (up to 9) opens the editor with them, and cancelling the picker leaves the start screen unchanged
  3. With fingers on the touchscreen, user can drag dividers, pinch-zoom and pan an image, swap images by long-press, switch templates and change settings
  4. "Speichern" puts the collage into `Pictures/dimuli` without any permission prompt and it shows up in the gallery app (Google Photos); "Teilen" opens the system share sheet with the JPEG
  5. System bars never cover content (edge-to-edge also on Android 13/14), the back gesture asks for confirmation in the editor and returns to the start screen (and closes the app from there), and the native library passes the 16 KB alignment check (`zipalign -c -P 16`)
**Plans**: 8/8 plans executed

Plans:
- [x] 05-01-PLAN.md — Rust preparation: approved crates, lib name `dimuli`, Slint Android feature, scoped JNI lint exception, android_main + back-native skeleton; own inset path removed (D-13), Slint safe-area insets; host gates + cargo ndk build/clippy
- [x] 05-02-PLAN.md — Tracer: Gradle project + wrapper, Kotlin MainActivity, manifest, back JNI round trip, smoke.sh; APK launches on dimuli_api36, back closes; insets + rotation checked
- [x] 05-03-PLAN.md — Photo Picker slice: ACTION_PICK_IMAGES, cache copy, nativeImagesPicked; cancel/single/failure; editor back confirm
- [x] 05-04-PLAN.md — Save (MediaStore Pictures/dimuli) and share (own ShareProvider + chooser, no success message)
- [x] 05-05-PLAN.md — UI pass: landscape editor layout, settings chevron, readable Slint attribution, adaptive launcher icon
- [x] 05-06-PLAN.md — Emulator gate: AVD dimuli_api33, 16 KB / permission check script, full flow matrix on API 33 and 37
- [x] 05-07-PLAN.md — Real devices Android 14 + 16: connect (owner), install + smoke, finger tests (owner)
- [x] 05-08-PLAN.md — Milestone close: code review + critical fixes, design docs updated, milestone 5 ticked
**UI hint**: yes

### Phase 6: Polish and v1 Acceptance
**Goal**: The app stays correct and responsive in real use, and the v1 success metric is met on a real Android 16 device (`docs/PLAN.md` milestone 6, including the open M3/M4 review notes)
**Depends on**: Phase 5
**Requirements**: LIFE-01, PERF-01, ERRM-01, POLI-01, POLI-02, ACPT-01
**Success Criteria** (what must be TRUE):
  1. After pausing and resuming the app (home, app switcher, returning from the Photo Picker or share sheet) the collage and all edits are still there
  2. With large photos from a real camera, loading, dragging/panning/zooming and export complete without crash or out-of-memory and without visible stutter; the `DECODE_MAX_PIXELS` value for devices is decided and recorded
  3. Every failure (unreadable image, pick, export, save, share) shows a short German message saying what happened and what to do, with no technical details
  4. Changing the aspect ratio shows no flicker frame, desktop "Teilen" immediately reports "nicht verfügbar" without rendering first, and render tests use outputs of 64 px or less with `cargo test --workspace`, clippy and fmt green
  5. On a real Android 16 device, a user creates a collage from 1-9 images, saves it to the gallery, and sees it there (v1 success metric)
**Plans**: 8 plans

Plans:
- [x] 06-01-PLAN.md — Core: decode limits 64 MP / 256 MiB / edge as "too large" (D-08, D-17, D-18), preview with one copy, render tests <= 64 px
- [x] 06-02-PLAN.md — Messages: success auto-hide after 3 s (D-11), own du-form text per failure incl. "zu groß" (D-13), late-result guards
- [x] 06-03-PLAN.md — Rust-owned canvas size (no aspect flicker), gesture fast path, desktop share early-out
- [x] 06-04-PLAN.md — Android import: Kotlin ImageDecoder transcode HEIC/WebP/AVIF (PNG for alpha), downscale > 64 MP, failed-copy count over JNI
- [x] 06-05-PLAN.md — Lifecycle: restart notice (D-06, D-15), complete configChanges, review notes N1-N5, emulator pause/resume flows
- [x] 06-06-PLAN.md — Emulator verification: 50/80 MP, rotated, HEIC/WebP/alpha matrix with peak memory (D-10), API 33/37 matrix, device smoke
- [x] 06-07-PLAN.md — Code review + critical fixes, docs record limits and Phase 6 as built
- [x] 06-08-PLAN.md — Owner's final device test on Android 14 + 16 (ACPT-01 + deferred 05-07 checklist), device memory, milestones 5 and 6 ticked
**UI hint**: yes

### Phase 7: Developer Docs for the Android Build
**Goal**: A developer (or Claude) can build, install and run dimuli on the emulator by following `CLAUDE.md` alone (`docs/PLAN.md` milestone 7)
**Depends on**: Phase 5 (the documented commands must exist and be verified; kept in PLAN.md order after Phase 6)
**Requirements**: ENV-03
**Success Criteria** (what must be TRUE):
  1. `CLAUDE.md` lists the `cargo ndk` + Gradle commands for building, installing and viewing logs, and no `cargo-apk` commands remain
  2. Every documented command runs successfully as written in the WSL2 setup (emulator `dimuli_api36`)
  3. The project structure in `CLAUDE.md` includes the `android/` Gradle project next to the two crates
**Plans**: 1 plan

Plans:
- [x] 07-01-PLAN.md — CLAUDE.md: `cargo ndk` + Gradle build/check/install/start/log commands, Android lint, structure with `android/`; every command run as written on dimuli_api36; agent commands listed for the owner; milestone 7 ticked

### Phase 8: More Than 9 Images
**Goal**: Users can build a collage from more than 9 images; the image limit is raised from 9 to 36 (D-06, a 6x6 grid) (`docs/PLAN.md` milestone 8; committed follow-up beyond the v1 success metric)
**Depends on**: Phase 6 (hard dependency is Phase 5 for the emulator checks; Phase 6 performance work comes first)
**Requirements**: MANY-01, MANY-02, MANY-03
**Success Criteria** (what must be TRUE):
  1. Picking more than 9 images (e.g. 12 and 25; test sizes to be confirmed in discuss-phase) opens the editor with a valid grid layout in which all images are visible and no cells overlap
  2. The "too many images" hint no longer appears for selections up to the new limit of 36 (D-06), and the 1-9 image templates behave exactly as before
  3. With many images, dividers, pan/zoom, swap and export stay operable and stable on the emulator without out-of-memory
**Plans**: 6 plans

Plans:
- [x] 08-01-PLAN.md — Core: freeze 1-9 (legacy copy, raw-style geometry, green first), effective style `4/n` from 5 cells per line via `Collage::effective_style()` for geometry, drags and export (D-01..D-04)
- [x] 08-02-PLAN.md — Core: grid generator for 10-36 (two most balanced line counts, max 6 templates), `MAX_IMAGES` = 36 (D-06), validity and drag matrix for every format and style
- [x] 08-03-PLAN.md — App: preview edge by count, effective radius in the preview, scaled thumbnails, D-05/D-06 handle tests, loading overlay from "Bilder wählen" until the picker answers (D-08)
- [x] 08-04-PLAN.md — Emulator: 12 and 25 photos via the Photo Picker (load, divider drags, pan, swap, template, 16:9, save), peak memory, times, D-08 cancel and repeated taps
- [x] 08-05-PLAN.md — Code review + critical fixes, full gates, docs record the cap, preview budget, effective style and pick flow as built
- [x] 08-06-PLAN.md — Owner check on desktop (36 images), emulator or phone; cap 36 confirmed; milestone 8 ticked
**UI hint**: yes

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Tooling Setup | n/a (pre-GSD) | Complete | - |
| 2. Workspace and Layout Engine | n/a (pre-GSD) | Complete | 2026-11-13 |
| 3. Desktop Editor | n/a (pre-GSD) | Complete | 2026-11-13 |
| 4. Export Renderer | n/a (pre-GSD) | Complete | 2026-11-13 |
| 5. Android Shell | 8/8 | Complete | 2026-11-15 |
| 6. Polish and v1 Acceptance | 8/8 | Complete | 2026-11-15 |
| 7. Developer Docs for the Android Build | 1/1 | Complete | 2026-11-15 |
| 8. More Than 9 Images | 6/6 | Complete | 2026-11-15 |
