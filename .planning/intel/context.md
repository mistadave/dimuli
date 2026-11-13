# Context (DOC-derived, plus DOC-like prose from the SPEC)

Entries are keyed by topic, in English, with source attribution. Sources: PLAN = /home/dstaehel/Documents/dev/dave/dimuli/docs/PLAN.md; GUIDE = /home/dstaehel/Documents/dev/dave/dimuli/docs/CODING_GUIDELINES.md; ARCH = /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md.

## v1 scope (PLAN)
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/PLAN.md (section "Kontext")
- notes: dimuli is an Android app that arranges 1 to n images into the cells of a collage and saves the result to the gallery or shares it. For the first version:
  - Templates per image count whose dividers can be moved
  - Pan and zoom an image within its cell; swap images between cells
  - Spacing, padding/corner radius, background color and aspect ratio (1:1, 4:5, 9:16, 16:9)
  - Export to the gallery and via the share dialog
  - Development and test entirely in WSL2: the Android emulator runs there via KVM, its window appears via WSLg
- Acceptance criteria: absent in the source (no PRD).

## Tooling state at plan time (PLAN)
- source: PLAN (section "Kontext")
- notes: Stated "as-is" at plan time: Rust 1.99 installed; Android SDK/NDK, `adb`, `cargo-ndk` and the Android targets missing; "the repo so far contains only CLAUDE.md". Milestone 1 (tooling setup) is now checked done, so this statement is historical.

## Workspace layout (PLAN)
- source: PLAN (section "Architektur")
- notes: Cargo workspace with two crates and a Gradle project: `Cargo.toml` (workspace); `crates/collage-core/` (pure logic, no UI/Android dependency, fully testable); `crates/dimuli-app/` (Slint UI + platform layer; lib as cdylib for Android + bin for desktop; `ui/*.slint`; `src/platform/{mod.rs, desktop.rs, android.rs}`); `android/` (Gradle project: manifest, Kotlin activity, FileProvider). Details on modules/data model/flow/limits are in ARCHITECTURE.md.

## Android integration approach (PLAN)
- source: PLAN (section "Android-Integration")
- notes: `cargo-apk` is not sufficient: image picking and sharing need `onActivityResult`/intents, which `NativeActivity` does not forward to native code. Therefore:
  - Gradle project with `MainActivity : NativeActivity` in Kotlin. Slint runs via `backend-android-activity` (feature `native-activity`).
  - Image picking: Photo Picker (`PickMultipleVisualMedia`). Kotlin copies the selected URIs into the cache directory and calls native function `onImagesPicked(paths)`. Rust handles the rest.
  - Saving: `MediaStore.Images` (`Pictures/dimuli`), no storage permission from API 29.
  - Sharing: file in cache, `FileProvider` and `ACTION_SEND`.
  - Build: `cargo ndk` places `libdimuli.so` under `android/app/src/main/jniLibs/`; Gradle builds the APK from it.
  - Target ABIs: `x86_64` (emulator) and `arm64-v8a` (devices).

## Android version targets (PLAN)
- source: PLAN (section "Android-Versionen")
- notes: `minSdk` 36 (Android 16); `targetSdk`/`compileSdk` 37 (Android 17, newest stable API level); NDK r30 (`30.0.16248370`); Rust build `cargo ndk --platform 36 ...`. Consequences stated:
  - Photo Picker, MediaStore without storage permission and `FileProvider` sharing exist on all target devices; no fallbacks for older versions needed.
  - Edge-to-edge is mandatory from Android 15: app draws under status and navigation bars. Kotlin reads system insets (`WindowInsets`) and passes them to Rust via JNI; the Slint UI indents content accordingly (`safe-area` properties).
  - 16 KB memory pages: native libraries must be 16 KB aligned (mandatory on Google Play, possible on devices from Android 15). NDK r28+ does this by default; verify with `zipalign -c -P 16` or the APK Analyzer.
  - Predictive Back: the Kotlin activity forwards the back callback (`OnBackPressedCallback`/`OnBackInvokedCallback`) to Rust: in the editor back goes to the start view; on the start view the app is closed.
  - Emulator image: Android 16 (API 36) x86_64; additionally an image of the newest version for the "up to latest" test.

## Milestone status (PLAN)
- source: PLAN (section "Umsetzung in Meilensteinen"); completion state as checked in the source checkboxes (consistent with git history: workspace/collage-core, Slint desktop app, export renderer + desktop save)
- notes:
  - [x] 0. Record the plan as `docs/PLAN.md`, reference it from `CLAUDE.md`. Completed milestones are checked off there.
  - [x] 1. Set up tooling (all in WSL): Android cmdline-tools in `~/Android/Sdk` plus platform-tools, emulator, `platforms;android-37.0`, `build-tools;37.0.0`, NDK r30 (`ndk;30.0.16248370`); emulator images Android 16 (API 36) and Android 17 (API 37) x86_64 with AVDs `dimuli_api36` and `dimuli_api37`; `rustup target add x86_64-linux-android aarch64-linux-android`, `cargo install cargo-ndk`; `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `ANDROID_NDK_HOME`, `PATH` in `~/.zshrc` (manual); user added to group `kvm` then `wsl --shutdown` (manual); emulator start verified (`emulator -accel-check`, `emulator -avd dimuli_api36`). All sub-items checked.
  - [x] 2. Workspace and collage-core with unit tests: tree, templates, geometry, divider dragging, cover transform, swapping.
  - [x] 3. Desktop UI with Slint: load images, editor, dividers, pan/zoom, swap, settings.
  - [x] 4. Export renderer in collage-core with test (pixel comparison on a test image); save as file on desktop.
  - [ ] 5. Android shell: Gradle project, Kotlin activity, JNI bridge, Photo Picker, MediaStore, sharing; run in emulator. (OPEN)
  - [ ] 6. Polish: lifecycle (pause/resume, keep state), performance with large images, error messages in the UI. (OPEN) Open review notes from M3: reduce snapshot allocations per pointer event (only update cells during a gesture), possible 1-frame flicker after aspect change, unnecessary pixel buffer copy when loading, possibly lower `DECODE_MAX_PIXELS` to 64 MP for devices. Open review notes from M4: some render tests use output sizes above 64 px; desktop "share" first renders fully and then reports "not available".
  - [ ] 7. Update `CLAUDE.md`: replace `cargo-apk` commands with the `cargo ndk` + Gradle flow and add the structure. (OPEN)
  - [ ] 8. More than 9 images: grid generator in `templates` that produces a layout for any image count; raise `MAX_IMAGES` (goal: no fixed limit, cells get smaller). Check memory and usability with many images in the emulator. (OPEN)
- Summary: milestones 0-4 done; 5-8 open.

## Dependencies (PLAN)
- source: PLAN (section "Wichtige Abhängigkeiten")
- notes: `slint` (+ `slint-build`), `image`, `tiny-skia`, `jni`, `android_logger`, `log`, `thiserror`; desktop only: `env_logger`. Desktop image selection without a file dialog: images are passed as arguments (`cargo run -p dimuli-app -- a.jpg b.jpg`); reason: `rfd` would need GTK3 or an xdg portal under WSL.

## Slint license (PLAN)
- source: PLAN (section "Lizenz Slint")
- notes: dimuli uses Slint under the Slint Royalty-free License 2.0. Condition: the app shows a Slint attribution (widget `AboutSlint` or "Made with Slint" badge), in dimuli on the start screen or in "About".

## Verification plan (PLAN)
- source: PLAN (section "Verifikation")
- notes:
  - `cargo test -p collage-core`: templates, geometry (no overlap, spacing correct), drag limits, cover clamping, export pixel test.
  - `cargo run -p dimuli-app`: full flow on desktop (pick images, arrange, export, open file).
  - `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
  - Emulator (API 36 and newest API): install APK with `adb install` and play through. Also check: content not covered by system bars, back gesture works, 16 KB alignment passes. Flow: Photo Picker with 3-5 images, drag dividers, zoom, swap, save to gallery (check in Google Photos), open share dialog, pause and resume app. Logs via `adb logcat -s dimuli`.

## Coding conventions (GUIDE) - normative, not locked
- source: GUIDE (classified DOC; the classifier notes the rules are normative conventions). The file states: "Binding rules for everyone who writes code for dimuli, especially the coding agent. On conflicts: this file > general conventions. Deviations only with a justification in a code comment."
- notes:
  - 1 Principles: simple over clever; small steps (one change = one purpose); reuse existing `collage-core` functions; identifiers, comments, commit messages and log messages in English, user-facing UI text in German.
  - 2 Layers: `crates/collage-core` (layout tree, templates, geometry, transforms, export renderer; only pure Rust crates, no Slint, no JNI); `crates/dimuli-app/src` (app state, core <-> UI; depends on `collage-core`, `slint`); `crates/dimuli-app/src/platform` (image picking, saving, sharing, insets, back; platform APIs); `crates/dimuli-app/ui` (`.slint`: presentation and gesture events only); `android/` (Kotlin activity, manifest, Gradle). Dependencies point only downward: `ui` -> `app` -> `core`. Platform code only behind the `Platform` trait; `#[cfg(target_os = "android")]` only in `platform/`. No logic in `.slint` or Kotlin; Kotlin contains only what cannot work without the Activity.
  - 3 Rust style: `cargo fmt` default config; `cargo clippy --all-targets -- -D warnings` clean; edition 2024, stable toolchain, no nightly; naming snake_case/PascalCase/SCREAMING_SNAKE_CASE with descriptive names; functions short (guideline < 50 lines); minimal visibility (`pub(crate)` before `pub`).
  - 3 Rust types/data: newtypes for IDs (`CellId(u32)`, `ImageId(u32)`), no bare `usize` across module boundaries; geometry in collage units (1.0 = collage width), normalized 0.0-1.0 coordinates only for image sections (`crop`); `f32` for geometry, compare with tolerance in tests; make invalid states impossible (enums over bool flags, validating constructors, e.g. `ratio` always in `[MIN_RATIO, 1 - MIN_RATIO]`).
  - 3 Error handling: no `unwrap()`, `expect()`, `panic!`, `unreachable!` or possibly out-of-bounds `v[i]` in production code (allowed in tests and `build.rs`); `collage-core` uses own error enums with `thiserror` (`CoreError`); `dimuli-app` translates errors at the UI boundary into user messages and logs every error with `log::error!`; no `Box<dyn Error>`/`anyhow` in `collage-core`; `anyhow` in `dimuli-app` only after asking.
  - 3 unsafe/FFI: `unsafe` only in `platform/android.rs` (JNI), each block with a `// SAFETY:` comment; the only `#[allow]` exception is the `ui` module in `dimuli-app/src/lib.rs` that includes `slint::include_modules!()` output, limited to lints the generated code really violates; native JNI entry points (`extern "system" fn Java_...`) catch panics with `std::panic::catch_unwind`; Kotlin callbacks never touch UI objects directly, always switch via `slint::invoke_from_event_loop`; do not keep JNI local references beyond the call, use `GlobalRef` for long-lived objects.
  - 3 Performance/memory: never hold images several times in full resolution; UI works with previews (max 2048 px edge); expensive work (decode, export) not on the UI thread; avoid per-frame allocations in gesture handlers; update models selectively.
  - 4 Slint: one component per file under `ui/`, entry `ui/app.slint`, file names `kebab-case.slint`; no business logic (data as properties/models, actions as callbacks e.g. `callback divider-moved(int, length)`); colors, spacing, font sizes centrally in `ui/theme.slint` (`global Theme`), no magic numbers; touch targets at least 48 px; always respect safe-area insets (edge-to-edge) via `safe-area` properties.
  - 5 Kotlin/Android: Kotlin only for activity lifecycle, Photo Picker, MediaStore, share intent, insets, back callback; every Kotlin function called from/calling Rust lives in `MainActivity.kt` in the section `// region JNI bridge`, signatures documented identically on both sides; no extra manifest permissions without asking (minSdk 36 needs no storage permission); no AndroidX/third-party libraries without asking.
  - 6 UI texts: short, German, collected in one place (prepared for translation via `@tr("...")`); every longer action (export, loading) shows progress or a loading state; error messages say what happened and what the user can do, without technical details.
  - 7 Tests: every public function in `collage-core` has unit tests in the same module (`#[cfg(test)] mod tests`); test names describe behavior (`dragging_divider_respects_min_cell_size`); always test edge cases (1 image, max image count, extreme aspect ratios, spacing 0); reproduce bugs as a failing test first; export tests compare pixels at defined locations (no whole reference images); test images small (<= 64 px) and generated in the test.
  - 8 Dependencies: new crates, Gradle dependencies or features only after asking; check actively maintained, license MIT/Apache-2.0 (or compatible), builds for `aarch64-linux-android`; `default-features = false` and only needed features (e.g. `image` only `jpeg`, `png`); versions centrally in workspace `Cargo.toml` under `[workspace.dependencies]`.
  - 9 Documentation: `///` doc comments for all `pub` items in `collage-core` (what it does, units, error cases); comments explain why, not what; track architecture/plan changes in `docs/PLAN.md` and check off finished milestones; update `CLAUDE.md` when commands or setup change.
  - 10 Git: commits only on explicit request; English imperative messages, subject <= 72 characters (e.g. `Add divider drag to layout tree`), optional blank line + rationale; one commit = one logical change, fmt/clippy/test green before commit; no generated files, build artifacts, keys or `local.properties`.
  - 11 Completion checklist: `cargo fmt --all`; `cargo clippy --all-targets -- -D warnings`; `cargo test --workspace`; new logic has tests incl. edge cases; no `unwrap`/`expect`/`panic` in production code; UI change verified with `cargo run` on desktop, platform change in emulator `dimuli_api36`; `docs/PLAN.md`/`CLAUDE.md` updated if affected; short summary to the user (what changed, what was verified, what is open).
- cross_refs declared by classifier: docs/PLAN.md, CLAUDE.md

## Screens (ARCH)
- source: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md (section "Bildschirme")
- notes: 1. Start: button "Bilder wählen". 2. Editor: canvas, template bar below; settings (aspect ratio, spacing, radius, color) as an expandable panel; actions "Speichern" and "Teilen" in the top bar. Export is not a separate screen but a loading state (`busy`) over the editor.

## User flows (ARCH)
- source: ARCH (section "Abläufe")
- notes:
  - Pick images: Start -> `platform.pick_images(MAX_IMAGES)` -> `ImagesPicked(paths)` -> background thread reads image size + creates preview image -> controller creates `Collage` with the first fitting template -> editor.
  - Edit (synchronous, UI thread): gesture in Slint -> callback -> `collage.drag_divider/pan/zoom/swap` -> `view::update(...)` -> Slint redraws. No threads, no copies of images.
  - Export: see "Export flow" in `constraints.md`.

## Explicitly out of scope for v1 (ARCH)
- source: ARCH (section "Bewusst nicht in v1")
- notes: Persistence/saving projects, undo/redo, text/stickers, filters, free-form arrangement, more than 9 images (milestone 8), translations (only prepared). The architecture does not block any of these: undo, for example, would be a stack of `Collage` clones in the controller.
