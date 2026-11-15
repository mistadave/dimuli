---
phase: "6"
slug: "polish-and-v1-acceptance"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-14"
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`); Android checks via `android/scripts/check-apk.sh`, `android/scripts/smoke.sh` and adb-driven emulator flows |
| **Config file** | none (workspace `Cargo.toml` lints, `clippy.toml`) |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace && ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` |
| **Android gates** | `android/gradlew -p android assembleDebug && android/scripts/check-apk.sh`; `android/scripts/smoke.sh -s emulator-5554`; device (owner rule, only when attached): `android/gradlew -p android assembleDebug -PrustAbis=arm64-v8a && android/scripts/smoke.sh -s 8200fcc9` |
| **Estimated runtime** | quick ~10 s warm; full suite ~60-90 s warm; emulator flows several minutes each |

---

## Sampling Rate

- **After every task:** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (no commits without the owner's request, so "per task" replaces "per commit"); tasks touching `platform/android.rs` add the `cargo ndk … clippy` command; Kotlin tasks add `android/gradlew -p android assembleDebug -PrustAbis=x86_64`
- **After every plan wave:** full suite + the plan's emulator check on `emulator-5554`
- **Before `/gsd-verify-work`:** full suite, `check-apk.sh`, smoke on emulator-5554 (and 8200fcc9 when attached), API 33/37 matrix (06-06), owner device test (06-08)
- **Max feedback latency:** ~90 s for host gates; emulator flows are wave-level checks

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 6-01-01 | 01 | 1 | PERF-01 | T-06-01 | Over-size images (64 MP, 16384 px edge, 256 MiB buffer) rejected from the header as ImageTooLarge before pixel allocation | unit (tracer) | `cargo test -p collage-core --lib decode` | ✅ decode.rs (new tests added in task) | ✅ green |
| 6-01-02 | 01 | 1 | PERF-01 | T-06-02 | Preview conversion cannot panic on malformed sizes (zip copies, size checks) | unit | `cargo test -p dimuli-app --lib images::tests::preview_buffer` | ✅ images.rs (new tests added in task) | ✅ green |
| 6-01-03 | 01 | 1 | POLI-02 | — | N/A | static + unit | `test "$(grep -nE 'opts\((6[5-9]\|[7-9][0-9]\|[1-9][0-9]{2,})\)' crates/collage-core/src/render.rs \| wc -l)" -eq 0 && cargo test -p collage-core --lib render` | ✅ render.rs | ✅ green |
| 6-02-01 | 02 | 2 | ERRM-01 | T-06-03 | Success text only; timer can only hide its own message | unit (tracer) + desktop run + emulator | `cargo test -p dimuli-app --lib state::tests::expire`; desktop `timeout 15 target/debug/dimuli …` (exit 124); emulator logcat `saved to gallery: true` + 2 screenshots | ✅ state.rs | ✅ green |
| 6-02-02 | 02 | 2 | ERRM-01, LIFE-01 | T-06-03, T-06-04, T-06-05 | No technical details in texts; late background results dropped | unit | `cargo test -p dimuli-app --lib state::tests::late && cargo test -p dimuli-app --lib state::tests::loading && cargo test -p dimuli-app --lib texts` | ✅ state.rs, texts.rs | ✅ green |
| 6-03-01 | 03 | 3 | POLI-01 | T-06-06 | Non-finite/negative areas ignored, canvas 0x0 for invalid input | unit (tracer) + desktop run + emulator | `cargo test -p dimuli-app --lib fit_canvas && cargo test -p dimuli-app --lib state::tests::aspect`; desktop `timeout 15 target/debug/dimuli …` (exit 124); emulator screenshots p6-03-* + logcat (1x `dimuli started`, no RustPanic) | ✅ view.rs, state.rs | ✅ green |
| 6-03-02 | 03 | 3 | PERF-01 | T-06-07 | Per-event work bounded by cell count, buffers reused | unit | `cargo test -p dimuli-app --lib view::tests::gesture` | ✅ view.rs | ✅ green |
| 6-03-03 | 03 | 3 | POLI-02 | — | N/A | unit + Android lint | `cargo test -p dimuli-app --lib state::tests::share_unsupported` + Android clippy | ✅ state.rs | ✅ green |
| 6-04-01 | 04 | 4 | PERF-01 | T-06-08, T-06-SC | Only the OS codec parses HEIC/WebP/AVIF; no manifest/dependency change | e2e emulator (tracer) | `grep -q 'transcoded image' android/build/smoke/p6-04-heic-logcat.txt` + JPEG magic of the cache copy; `git diff --quiet ff11541 -- <Cargo/Gradle manifests>` | ❌ evidence created in task | ✅ green |
| 6-04-02 | 04 | 4 | PERF-01 | T-06-09 | Bitmap sized to <= 64 MP / 16384 px before allocation; OOM caught per image | e2e emulator | `grep -q 'downscaled image' …p6-04-big80-logcat.txt` + `identify` of the cache copy (w*h <= 64,000,000) | ❌ evidence created in task | ✅ green |
| 6-04-03 | 04 | 4 | ERRM-01 | T-06-10, T-06-11, T-06-12 | Failed count clamps negatives; names never from URIs; JNI signature accepted | unit + e2e emulator | `cargo test -p dimuli-app --lib native_events && cargo test -p dimuli-app --lib state::tests::loading_counts_failed && cargo test -p dimuli-app --lib platform::desktop`; logcat `picked images: 1, failed copies: 1` | ✅ native_events.rs, state.rs, desktop.rs | ✅ green |
| 6-05-01 | 05 | 5 | LIFE-01, ERRM-01 | T-06-13, T-06-14 | Bundle holds one boolean; notice only when a collage was lost | unit (tracer) + e2e emulator | `cargo test -p dimuli-app --lib state::tests::restart && cargo test -p dimuli-app --lib texts`; logcat `collage lost` after kill with collage, none after kill on start | ✅ state.rs, texts.rs | ✅ green |
| 6-05-02 | 05 | 5 | LIFE-01 | T-06-15, T-06-16 | Stale share file removed; query fails for a missing file; config changes do not recreate | static + e2e emulator | configChanges grep (6 values); `grep -q 'SMOKE OK emulator-5554' …p6-05-smoke.txt`; exactly one `dimuli started`, one pid across home/recents/share/rotation/bold text | ❌ evidence created in task | ✅ green |
| 6-06-01 | 06 | 6 | PERF-01 | T-06-17, T-06-18 | Test images only on emulators | e2e emulator (tracer) | logcat `picked images: 1`, `saved to gallery: true`, no `skipping`/`RustPanic`/`Out of memory`; VmHWM file; MediaStore row | ❌ evidence created in task | ✅ green |
| 6-06-02 | 06 | 6 | PERF-01 | T-06-17, T-06-18 | Copies within limits; rotated photo upright; alpha as PNG | e2e emulator | logcat `picked images: 7, failed copies: 0`; `identify` loop over p6-06-copy-*; one pid; VmHWM file | ❌ evidence created in task | ✅ green |
| 6-06-03 | 06 | 6 | PERF-01 | T-06-17 | 16 KB alignment, no permissions | e2e emulator matrix | `APK CHECK OK`; `SMOKE OK emulator-5556/5558`; both logcats `picked images: 3, failed copies: 0` + `saved to gallery: true`; device smoke if attached | ❌ evidence created in task | ✅ green |
| 6-07-01 | 07 | 7 | PERF-01, ERRM-01, LIFE-01, POLI-01, POLI-02 | T-06-19 | Critical review findings fixed | review + full gates (tracer) | `test -s 06-REVIEW.md`; full suite; `assembleDebug && check-apk.sh`; smoke emulator-5554 | ❌ report created in task | ✅ green |
| 6-07-02 | 07 | 7 | PERF-01 | T-06-20 | Docs match the code | static | docs greps (64 MP, 256 MiB, ImageDecoder, restored_after_close; old limit/event/HEIC sentence gone; milestone 6 unticked; CLAUDE.md unchanged) | ✅ docs/*.md | ✅ green |
| 6-08-01 | 08 | 8 | ACPT-01 | T-06-21, T-06-22 | Only dimuli's start screen captured on personal phones | e2e real devices (tracer) | `APK CHECK OK`; at least two `SMOKE OK` device transcripts incl. 8200fcc9 | ❌ evidence created in task | ✅ green |
| 6-08-02 | 08 | 8 | ACPT-01, LIFE-01, PERF-01, POLI-01 | — | N/A | manual (owner checkpoint) | — (owner replies "approved") | — | ✅ green |
| 6-08-03 | 08 | 8 | ACPT-01, PERF-01 | T-06-21, T-06-23 | Wi-Fi adb disconnected after the test | static + device memory | `grep -c '^- \[x\] 5\.'` / `6\.` in docs/PLAN.md >= 1; VmHWM/TOTAL in p6-08-mem-*.txt for both phones | ✅ docs/PLAN.md | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0 plan: every new host test is written first inside its own task (`tdd="true"` with a `<behavior>` block), and every emulator asset is created by the task that needs it.

- [x] `crates/collage-core/src/decode.rs` — 64 MP / edge / alloc boundary tests (6-01-01)
- [x] `crates/dimuli-app/src/images.rs` — `preview_buffer_*` tests (6-01-02)
- [ ] `crates/dimuli-app/src/state.rs` — `expire_*`, `late_*`, `loading_*`, `aspect_*`, `share_unsupported_*`, `restart_*`, `loading_counts_failed_*` tests (6-02-01, 6-02-02, 6-03-01, 6-03-03, 6-04-03, 6-05-01)
- [x] `crates/dimuli-app/src/texts.rs` — `failure_texts_say_what_happened_and_what_to_do` (6-02-02)
- [ ] `crates/dimuli-app/src/view.rs` — `fit_canvas_*`, `gesture_rows_*` tests (6-03-01, 6-03-02)
- [ ] `crates/dimuli-app/src/platform/native_events.rs` — failed-count mapping tests (6-04-03)
- [ ] Emulator assets in `android/build/smoke/` (gitignored): `p6-a.jpg`, `p6-b.jpg`, `p6-small.heic`, `p6-broken.heic`, `p6-big50.jpg`, `p6-big80.jpg`, `p6-12mp.jpg`, `p6-rot.jpg`, `p6-t.webp`, `p6-alpha.webp`, `p6-t.avif` (created in 06-02..06-06)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| v1 success metric: 1 and 9 own photos (incl. HEIC and a large capture) -> save -> visible in Google Photos on Android 16 | ACPT-01 | Real device, own photos, Google Photos UI | 06-08 Task 2 steps 3, 9, 11 |
| Touch editing with fingers incl. Pitfall 6 on Android 14 and 16 | ACPT-01 (and Phase 5 ANDR-08) | Real multi-touch | 06-08 Task 2 step 4 |
| No visible gesture stutter with large photos | PERF-01 | Perceived smoothness | 06-08 Task 2 step 4 |
| No flicker frame on format change | POLI-01 | One-frame artefact is not capturable by screenshot; the cause is excluded by the same-transition host test | 06-08 Task 2 step 5; desktop run in 06-03 |
| Pause/resume on real hardware (Home, app switcher, share sheet) | LIFE-01 | Real launcher/recents behaviour | 06-08 Task 2 steps 7-8 (emulator proof in 06-05) |
| Success banner hides after ~3 s on device | ERRM-01 | Visual timing | 06-08 Task 2 step 9 (emulator screenshots in 06-02) |
| Screenshot inspections of emulator flows (banner, layouts, rotated copy) | ERRM-01, POLI-01, PERF-01, LIFE-01 | Visual | `<human-check>` blocks in 06-02..06-06, done by the executor |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (the owner checkpoint 6-08-02 is manual by design)
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 90 s for host gates
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** owner approved 2026-11-15 (Android 16 on emulator dimuli_api36, Android 14 on OnePlus 9 Pro; owner accepted the emulator in place of a real Android 16 device)
