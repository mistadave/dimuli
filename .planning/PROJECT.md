# dimuli

## What This Is

dimuli is an Android collage-maker app. The user picks 1 to n images, arranges them in the cells of a layout template, adjusts them by touch (move dividers, pan/zoom each image, swap images, change spacing/corners/background/aspect ratio), and saves the result to the gallery or shares it as one image. It is a solo-developer project (one product owner, Claude as implementer) written in Rust with the Slint UI toolkit (`backend-android-activity`), developed and tested entirely in WSL2 with a desktop preview and Android emulators.

## Core Value

A user can turn a handful of photos into one well-arranged collage image and get it into their gallery in a few taps.

**v1 success metric:** on a real Android 16 device, a user can create a collage from 1-9 images and save it to the gallery.

## Requirements

### Validated

<!-- Delivered by docs/PLAN.md milestones 1-4 (GSD Phases 1-4) before GSD adoption. Verified by unit tests and desktop runs, not yet on Android. -->

- ✓ WSL2 toolchain for Android work (SDK, NDK r30, emulator AVDs API 36/37 with KVM, Rust Android targets, cargo-ndk) — Phase 1 (ENV-01)
- ✓ Layout engine in `collage-core`: templates for 1-9 images, overlap-free exact geometry, divider dragging with minimum cell size, cover-mode pan/zoom with a shared crop function, image swapping — Phase 2 (LAYT-01..05)
- ✓ Desktop editor in Slint: image loading with previews, dividers, pan/zoom, long-press swap, template bar, style settings, aspect ratios 1:1 / 4:5 / 9:16 / 16:9, back confirmation, Slint attribution, desktop preview via CLI arguments — Phase 3 (ENV-02, EDIT-01..08, LICN-01)
- ✓ Export renderer: JPEG (3000 px long edge, quality 90) off the UI thread with busy overlay, pixel-consistent with the preview, robust source handling (EXIF orientation, alpha, 16-bit), desktop save to file, German export error messages — Phase 4 (EXPT-01..04)

### Active

<!-- Current scope. Full list with IDs in REQUIREMENTS.md. -->

- [ ] Android shell: Gradle project + Kotlin `MainActivity : NativeActivity`, JNI bridge, Photo Picker, MediaStore save, share sheet, edge-to-edge insets, back/predictive back, 16 KB alignment, touch editing on device — Phase 5 (ANDR-01..08)
- [ ] Polish: lifecycle (pause/resume keeps state), performance with large images, complete German error messages, open M3/M4 review notes, v1 acceptance on a real Android 16 device — Phase 6 (LIFE-01, PERF-01, ERRM-01, POLI-01..02, ACPT-01)
- [ ] CLAUDE.md documents the `cargo ndk` + Gradle flow instead of `cargo-apk` — Phase 7 (ENV-03)
- [ ] More than 9 images via a grid generator in `templates` (goal: no fixed limit) — Phase 8 (MANY-01..03); committed follow-up beyond the v1 success metric

### Out of Scope

<!-- Explicit boundaries. Reasons included to prevent re-adding. -->

- Persistence / saving and reopening projects — no storage format or migration in v1; the collage lives only during the session
- Undo/redo — not needed for v1; architecture allows it later as a stack of `Collage` clones in the controller
- Text, stickers, filters — outside the v1 collage scope
- Free-form arrangement — v1 uses template layouts with movable dividers
- Translations — UI is German only; texts are prepared for `@tr("...")`
- Android versions below 13 (minSdk 33) — system Photo Picker (`ACTION_PICK_IMAGES`), MediaStore without storage permission, `FileProvider` sharing and `OnBackInvokedCallback` are available everywhere; no fallbacks needed
- Storage permissions / extra manifest permissions — not required at minSdk 33
- Native desktop file dialog (`rfd`) — would need GTK3 or an xdg portal under WSL; desktop images are passed as CLI arguments instead

## Context

- **Repo state (2026-11-13):** Cargo workspace with `crates/collage-core` (pure logic) and `crates/dimuli-app` (Slint UI, controller, `platform/{mod,desktop}.rs`). No `android/` Gradle project and no `platform/android.rs` yet. Milestones 0-4 of `docs/PLAN.md` are done; 5-8 are open.
- **Human docs (source of this roadmap):** `docs/PLAN.md` (German plan and milestone checklist), `docs/ARCHITECTURE.md` (modules, data model, data flow, limits), `docs/CODING_GUIDELINES.md` (binding rules, completion checklist), `CLAUDE.md` (commands, agent workflow). GSD Phase N corresponds to PLAN.md milestone N; milestone 0 (record the plan) is the baseline, not a phase.
- **Known stale item:** `CLAUDE.md` still lists `cargo apk` commands; `docs/PLAN.md` prescribes `cargo ndk` + Gradle (fixed in Phase 7).
- **Open review notes:** M3 — reduce snapshot allocations per pointer event (only update cells during a gesture), possible 1-frame flicker after aspect change, unnecessary pixel-buffer copy on load, consider lowering `DECODE_MAX_PIXELS` to 64 MP for devices. M4 — some render tests use outputs above 64 px; desktop "Teilen" renders fully and then reports "nicht verfügbar". All scheduled in Phase 6.
- **Agent workflow:** code is written by the `rust-coder` subagent; `code-reviewer` runs before committing new functionality; no commits or pushes without an explicit request.
- **Verification approach:** `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt`; full flow on desktop via `cargo run -p dimuli-app -- a.jpg b.png`; Android checks in emulators `dimuli_api33` / `dimuli_api36` / `dimuli_api37` and on a real device; logs via `adb logcat -s dimuli`.

## Constraints

- **Tech stack**: Rust stable, edition 2024; Slint with `backend-android-activity` (feature `native-activity`); `collage-core` has no Slint, JNI or Android dependency — logic must stay host-testable
- **Platform**: Android minSdk 33 (Android 13), targetSdk/compileSdk 37 (Android 17), NDK r30 (`30.0.16248370`); ABIs `arm64-v8a` (devices) and `x86_64` (emulator)
- **Build**: `cargo ndk --platform 33 ...` places `libdimuli.so` under `android/app/src/main/jniLibs/`, Gradle builds the APK — `cargo-apk` is insufficient because `NativeActivity` does not forward `onActivityResult`/intents needed for picking and sharing
- **Android compatibility**: edge-to-edge on all versions — mandatory from Android 15, enabled explicitly on 13/14 (insets from Kotlin via JNI to Slint `safe-area`); native libs must be 16 KB aligned (check `zipalign -c -P 16`); Predictive Back forwarded to Rust (editor -> start screen, start screen -> close app)
- **Architecture**: one `AppState` on the UI thread, one-way data flow, dependencies point downward only (`ui` -> `app` -> `core`), `#[cfg(target_os = "android")]` only inside `platform/`, no logic in `.slint` or Kotlin, no persistence in v1
- **Code quality**: no `unwrap`/`expect`/`panic!`/unchecked indexing in production code; `clippy -D warnings` and `cargo fmt` clean; `unsafe` only in `platform/android.rs` with `// SAFETY:` comments (workspace currently sets `unsafe_code = "deny"`, so Phase 5 needs a scoped exception); JNI entry points catch panics
- **Dependencies**: new crates, Gradle/AndroidX/third-party libraries or features only after asking the owner; must be actively maintained, MIT/Apache-2.0 compatible, and build for `aarch64-linux-android`; versions centrally in workspace `Cargo.toml`
- **Performance/memory**: never hold full-resolution images several times; UI works with previews (max 2048 px edge); decode/export never on the UI thread; avoid per-frame allocations in gesture handlers; touch targets at least 48 px
- **Limits (v1)**: `MAX_IMAGES` 9, `PREVIEW_MAX_EDGE` 2048 px, `EXPORT_LONG_EDGE` 3000 px (max 8192), `DECODE_MAX_PIXELS` 100 MP, `MIN_CELL_SIZE` 0.05, `MAX_ZOOM` 5.0, export JPEG quality 90
- **License**: Slint Royalty-free License 2.0 — the app must show a Slint attribution (`AboutSlint` on the start screen)
- **Language**: code, comments, commit messages and logs in English; user-facing UI text in German; `docs/PLAN.md` and `CLAUDE.md` in German; `.planning/` files in English
- **Process**: solo developer + Claude; commits and pushes only on explicit request; each completed milestone is checked off in `docs/PLAN.md`

## Key Decisions

<!-- Decisions 1-8 are the "Leitentscheidungen" from docs/ARCHITECTURE.md. They are design rationale, not ADRs, and are NOT locked. -->

<decisions>
No ADR-locked decisions exist (no ADRs were ingested). The table below records non-locked guiding decisions; revisit any of them when a concrete problem demands it.
</decisions>

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| One `AppState` on the UI thread, mutated only by the controller | No synchronization, one source of truth | ✓ Good (held through Phase 4) |
| One-way data flow: Event -> Controller -> Core -> View-Models -> UI | Traceable; UI stays dumb | ✓ Good |
| Core computes in collage units (1.0 = collage width) | Same logic for screen and export; independent of pixel size; spacing equal in x and y | ✓ Good |
| One crop function for preview and export | Export is guaranteed to look like the preview | ✓ Good |
| Layout as a binary tree of splits (one `ratio` per divider) | Templates and divider dragging become trivial | ✓ Good |
| Background work only in short-lived threads with their own data | No shared state, no async runtime | ✓ Good |
| Platform behind one `Platform` trait, results as `PlatformEvent` | Desktop for development, Android only a thin shell | — Pending (Android implementation in Phase 5) |
| No persistence in v1 | Saves a storage format and migration | ✓ Good (so far) |
| Gradle + Kotlin `MainActivity : NativeActivity` + `cargo ndk` instead of `cargo-apk` | `cargo-apk` cannot receive picker/share results | — Pending (Phase 5) |
| minSdk 33 (Android 13) without fallbacks for older Android (changed from 36 on 2026-11-13) | Wider device reach; system Photo Picker, MediaStore, `FileProvider`, `OnBackInvokedCallback` exist from API 33 | — Pending (Phase 5) |
| Desktop image selection via CLI arguments, no file dialog | `rfd` needs GTK3 / xdg portal under WSL | ✓ Good |
| Export as JPEG, quality 90, 3000 px long edge | Gallery- and share-compatible at reasonable file size | ✓ Good |
| Slint under Royalty-free License 2.0 with `AboutSlint` on start screen | License condition | ✓ Good (attribution in place) |

---
*Last updated: 2026-11-13 after roadmap creation (ingest of docs/PLAN.md, ARCHITECTURE.md, CODING_GUIDELINES.md)*
