# Phase 6: Polish and v1 Acceptance - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

The app stays correct and responsive in real use on Android: pause/resume keeps the collage (LIFE-01), large real-camera photos and HEIC/WebP picks load, edit and export without crash/OOM/stutter (PERF-01), every failure path shows a short German message (ERRM-01), the open M3/M4 review notes are closed (POLI-01, POLI-02), and the v1 success metric is met on a real Android 16 device (ACPT-01). No new user-facing features; no persistence.

</domain>

<decisions>
## Implementation Decisions

All four areas were proposed by Claude and accepted by the owner unchanged (2026-11-14).

### HEIC/WebP and other formats
- **D-01:** On Android, Kotlin transcodes every picked image that is not JPEG or PNG (by MIME type from the ContentResolver) while copying it into `cacheDir/picked/`: decode with the platform `android.graphics.ImageDecoder` (covers HEIC/HEIF, WebP, and AVIF where the device supports it), write a JPEG (quality 95) into the cache. JPEG/PNG keep the current byte copy. No new dependency.
- **D-02:** `ImageDecoder` applies EXIF orientation, so transcoded copies are written already oriented and without EXIF orientation; Rust handles them like any JPEG.
- **D-03:** A failed transcode counts as an unreadable image: skipped, existing "n Bilder konnten nicht gelesen werden" notice, `Log.e` with the cause.
- **D-04:** Rejected: `libheif` (C library, LGPL, hard cross-build) and the `image` crate `webp` feature (WebP only). Desktop stays JPEG/PNG only.

### Process / activity death (LIFE-01)
- **D-05:** No persistence in v1 (unchanged project decision). LIFE-01 is proven for the normal cases where the process survives: home, app switcher, returning from Photo Picker and share sheet, rotation (already via `configChanges`, Phase 5 D-05).
- **D-06:** When Android destroyed the activity/process and the user comes back, the app starts on the start screen and shows a German notice, e.g. *"Die App wurde vom System geschlossen. Die Collage konnte nicht erhalten bleiben."* Detection in Kotlin via `savedInstanceState != null` in `onCreate`, forwarded to Rust (exact mechanism: planner/research).
- **D-07:** Research must check whether the activity can be recreated while the process keeps running (e.g. developer option "Don't keep activities", memory pressure) and how `android_main` / `run()` behave then (state currently lives inside `run()`); if keeping `AppState` across such a recreation is cheap and safe, propose it, otherwise D-06 applies.

### Large images (PERF-01)
- **D-08:** `DECODE_MAX_PIXELS` drops from 100 MP to **64 MP** on all platforms (a 100 MP RGBA decode is about 400 MB; export long edge is only 3000 px). Record the decision in `docs/ARCHITECTURE.md` (limits table). — **Reversibility:** reversible — one constant plus docs/tests.
- **D-09:** On Android, Kotlin downscales any picked image above 64 MP to at most 64 MP while copying (same `ImageDecoder` path as D-01, e.g. `setTargetSize`), so very large captures (e.g. 200 MP modes) still work. The Rust limit stays as a safety net; images above it are skipped with a notice (always the case on desktop).
- **D-10:** Peak memory during load and export must be checked with real large photos (e.g. 50 MP JPEG, HEIC) on the emulator and the connected device.

### Messages (ERRM-01)
- **D-11:** Keep the existing message banner. Success messages (e.g. "Collage gespeichert.") hide automatically after about 3 s; error messages stay until the user closes them.
- **D-12:** Partial load failure: continue with the readable images and show the existing count notice (current behaviour, keep).
- **D-13:** Every failure path has its own German text: image unreadable, image too large, transcode failed (= unreadable), pick failed, export failed, out of memory (if detectable), save failed, share failed, restart after system close (D-06). Each text says in du-form what happened and what to do, no technical details; the code logs the cause with `log::error!` (Kotlin: `Log.e`).

### Owner Decisions after Research (2026-11-14)
- **D-15:** Restart notice (D-06) uses the precise variant: Rust tells Kotlin on every screen change whether a collage is open (`setCollageOpen(Z)`), Kotlin stores it in `onSaveInstanceState`; the notice appears only if a collage was open when the system closed the app.
- **D-16:** Transcoding (D-01) writes **PNG** when the decoded image has an alpha channel, otherwise JPEG q95.
- **D-17:** `DECODE_MAX_ALLOC` drops from 512 MiB to **256 MiB** together with D-08 (record in docs/ARCHITECTURE.md). — **Reversibility:** reversible — one constant plus docs/tests.
- **D-18:** Images above the edge limit (16384 px) get the "too large" message instead of "unreadable" (adjust the existing test).
- Research recommendation adopted: add the missing `configChanges` values (`fontWeightAdjustment|colorMode|grammaticalGender|touchscreen|mcc|mnc`) so these config changes no longer recreate the activity; keeping `AppState` across a real recreation is not pursued (D-07 answered: not cheap).

### Claude's Discretion
- M3 review notes: update only affected cells during a gesture (no full snapshot allocation per pointer event), drop the unnecessary pixel-buffer copy on load, remove the 1-frame flicker after an aspect change.
- M4 review notes: render tests with outputs of 64 px or less; desktop "Teilen" reports "nicht verfügbar" immediately without rendering first.
- Phase 5 review notes N1-N6 (`05-REVIEW.md`): fold in where cheap (e.g. smoke.sh SIGPIPE and first-frame screenshot, `@Deprecated` on `onActivityResult`, `ShareProvider.query` for a missing file, cleaning `cacheDir/share` at start).
- Exact auto-hide duration and banner animation; exact German wording within D-13.
- Test split per the owner rule (memory "device-first smoke tests"): smoke/insets on the connected device (`-PrustAbis=arm64-v8a`), automated picker/gallery/share flows and the API 33/37 matrix only on emulators.

### Acceptance (ACPT-01)
- **D-14:** ACPT-01 is part of the owner's final device test on Android 16 (and Android 14), together with the deferred 05-07 checklist (touch editing, Pitfall 6, picker, save to Google Photos, share, back). Milestone 5 and 6 are ticked in `docs/PLAN.md` only after that test.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project rules and design
- `CLAUDE.md` — agent workflow (rust-coder writes code, code-reviewer before commit, no commits without request)
- `docs/PLAN.md` — milestone 6 text incl. M3/M4 review notes; Android integration as built
- `docs/ARCHITECTURE.md` — modules, limits table (`DECODE_MAX_PIXELS`), data flow, platform interface
- `docs/CODING_GUIDELINES.md` — binding rules (no unwrap/expect, JNI rules §3, Kotlin scope §5, texts §6)

### Requirements
- `.planning/REQUIREMENTS.md` — LIFE-01, PERF-01, ERRM-01, POLI-01, POLI-02, ACPT-01
- `.planning/ROADMAP.md` — Phase 6 success criteria 1-5

### Prior phase
- `.planning/phases/05-android-shell/05-CONTEXT.md` — D-05 (configChanges/rotation), D-09 (approved deps, no AndroidX), D-11 (silent share), D-12 (HEIC deferred here)
- `.planning/phases/05-android-shell/05-REVIEW.md` — open notes N1-N6
- `.planning/phases/05-android-shell/05-07-SUMMARY.md` — deferred device checklist (becomes part of ACPT-01 final test)
- `README.md` — WSL2 device setup, build/check/smoke commands

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt` `copyToCache` — the place for transcoding/downscaling (D-01, D-09); runs on the `io` executor.
- `crates/dimuli-app/src/texts.rs` — all Rust-side German texts; `images_skipped(count)` for partial failures.
- `crates/dimuli-app/src/state.rs` `load_notice`, `dismiss_message`, `Message` — banner state; auto-hide needs a timer (Slint `Timer` or controller-side).
- `crates/collage-core/src/decode.rs` — `DECODE_MAX_PIXELS` / `DECODE_MAX_EDGE` / `DECODE_MAX_ALLOC`.
- `android/scripts/smoke.sh`, `check-apk.sh` — device/emulator gates.

### Established Patterns
- Platform results only as `PlatformEvent` via `post_event`; natives via `native_method!` + RegisterNatives; pure mapping in `platform/native_events.rs` (host-tested).
- Background work in short-lived threads, results via `invoke_from_event_loop`.
- `image` crate built with `jpeg` + `png` only.

### Integration Points
- New "restored after system close" signal: Kotlin `onCreate` → JNI → `PlatformEvent` (new variant) or a flag read at start.
- `view.rs` `Models::snapshot/apply` — gesture path for the M3 allocation fix.

</code_context>

<specifics>
## Specific Ideas

- Restart notice wording example: "Die App wurde vom System geschlossen. Die Collage konnte nicht erhalten bleiben."
- Transcoded JPEG quality 95; size cap 64 MP.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope. (Persistence of collages remains out of scope for v1.)

</deferred>

---

*Phase: 06-polish-and-v1-acceptance*
*Context gathered: 2026-11-14*
