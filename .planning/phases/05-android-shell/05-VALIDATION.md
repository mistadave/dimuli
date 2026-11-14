---
phase: "5"
slug: "android-shell"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-11-13"
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `05-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test harness (`cargo test`, host only); Android checks via `adb` on emulators; real-device checks manual |
| **Config file** | none (workspace `Cargo.toml`) |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace && (cd android && ./gradlew assembleDebug) && $ANDROID_HOME/build-tools/37.0.0/zipalign -c -P 16 -v 4 android/app/build/outputs/apk/debug/app-debug.apk` |
| **Estimated runtime** | ~30 s host; ~3-5 min with Gradle/cargo-ndk release build |

---

## Sampling Rate

- **After every task (no commits without owner request, CLAUDE.md):** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`; for tasks touching `platform/android.rs` additionally `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` (the `-A` silences a false positive on the existing `thread_local!` in controller.rs, Pitfall 4)
- **After every plan wave:** wave 1 (05-01, no Gradle project yet): `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 build -p dimuli-app --lib --release`; from wave 2 on: `android/gradlew -p android assembleDebug` (`-PrustAbis=x86_64` for emulator-only waves), `android/scripts/smoke.sh -s emulator-5554` on `dimuli_api36`; from 05-06 on also `android/scripts/check-apk.sh`
- **Before `/gsd-verify-work`:** full emulator matrix (`dimuli_api33`, `dimuli_api36`, `dimuli_api37`) + real Android 14 and Android 16 devices
- **Max feedback latency:** ~300 seconds

---

## Per-Task Verification Map

Filled from the PLAN.md task IDs (planner, 2026-11-13; revised after the plan check: the former tracer plan is split per owner decision into 05-01 Rust/workspace preparation and 05-02 Gradle/Kotlin tracer, all later plans renumbered +1). Emulator serials: `emulator-5554` = dimuli_api36, `emulator-5556` = dimuli_api33, `emulator-5558` = dimuli_api37. Never use the mouse wheel in an emulator (Slint `todo!()`, Pitfall 5).

| Task ID | Plan | Wave | Requirement / Decision | Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|------------------------|----------|-----------|-------------------|-------------|--------|
| 05-01-T1 | 01 | 1 | ANDR-01, ANDR-06, D-06, D-09, D-11 | Workspace builds `libdimuli.so` for Android with the approved crates; `android_main` + back-native skeleton; scoped JNI lint exception | host gates + Android clippy + Android release build | `ANDROID_JAR=… cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`; `ANDROID_JAR=… cargo ndk -t x86_64 --platform 33 build -p dimuli-app --lib --release && test -s target/x86_64-linux-android/release/libdimuli.so` | ❌ W0 (android.rs created here) | ✅ green |
| 05-01-T2 | 01 | 1 | ANDR-05, D-10, D-13 | Content inset by `Window.safe-area-insets`; own inset path removed; ARCHITECTURE.md inset lines updated; desktop unchanged | unit + doc grep + desktop | `cargo test --workspace`; `grep -rE 'InsetsChanged\|set_insets\|struct Insets' crates/dimuli-app/src \| wc -l` (0); `grep -c 'InsetsChanged' docs/ARCHITECTURE.md` (0); test images asserted with `test -s`, then desktop `timeout 15 target/debug/dimuli …` (exit 124) | ✅ | ✅ green |
| 05-02-T1 | 02 | 2 | ANDR-01, ANDR-06, D-01, D-02, D-04, D-05, D-06, D-07, D-09 | APK with both ABIs; launch on api36; back on start closes | build + emulator smoke (tracer) | `android/gradlew -p android assembleDebug` + `unzip -l … \| grep -cE 'lib/(arm64-v8a\|x86_64)/libdimuli.so'` (>= 2) + `android/scripts/smoke.sh -s emulator-5554` | ❌ W0 (created by this task: android/, smoke.sh) | ✅ green |
| 05-02-T2 | 02 | 2 | ANDR-05, D-05, D-07, D-10 | Start screen clear of the bars on api36 in portrait and landscape; rotation keeps native app | emulator | smoke + `test -s android/build/smoke/api36-start-portrait.png && test -s android/build/smoke/api36-start-landscape.png`; `api36-rotation-logcat.txt` has one `dimuli started` | ✅ (after T1) | ✅ green |
| 05-03-T1 | 03 | 3 | ANDR-02 | Paths -> `ImagesPicked`; 3-image pick opens editor | unit + emulator | `cargo test -p dimuli-app --lib native_events`; logcat `picked images: 3`; `run-as … ls cache/picked` (3 files) | ❌ W0 (native_events.rs created here) | ✅ green |
| 05-03-T2 | 03 | 3 | ANDR-02, ANDR-06 | Cancel = empty list; single pick; pick failure text; editor back confirm | unit + emulator | `cargo test -p dimuli-app --lib native_events && cargo test -p dimuli-app --lib back`; logcat `picked images: 0`; topResumedActivity checks | ✅ | ✅ green |
| 05-04-T1 | 04 | 4 | ANDR-03, ANDR-04, D-11 | ok/err -> `Saved`/`Shared`; share success silent | unit | `cargo test -p dimuli-app --lib saved && cargo test -p dimuli-app --lib shared` | ✅ (native_events.rs from 05-03) | ✅ green |
| 05-04-T2 | 04 | 4 | ANDR-03, D-06 | Saved to `Pictures/dimuli`, `is_pending=0`, no permission | emulator | `adb -s emulator-5554 shell content query --uri content://media/external/images/media --projection _display_name:relative_path:is_pending:owner_package_name \| grep -c 'relative_path=Pictures/dimuli/, is_pending=0, owner_package_name=ch.staeheli.dimuli'` (>= 1) | — | ✅ green |
| 05-04-T3 | 04 | 4 | ANDR-04, D-09 | Chooser opens; provider refuses ungranted access | emulator | `grep -ciE 'intentresolver\|ChooserActivity' android/build/smoke/api36-chooser.txt`; shell `content query` on the provider -> Permission Denial | — | ✅ green |
| 05-05-T1 | 05 | 5 | ANDR-08, D-05 | Landscape editor layout; rotation keeps collage | desktop + emulator | test images asserted with `test -s`, then desktop `timeout 15 target/debug/dimuli …` (exit 124); smoke; `api36-editor-rotation-logcat.txt` one `dimuli started` | — | ✅ green |
| 05-05-T2 | 05 | 5 | ANDR-01, ANDR-08, D-02, D-03 | Chevron, readable attribution, launcher icon | build + emulator | `aapt2 dump badging … \| grep -cE "application-label:'dimuli'\|icon='res/mipmap-anydpi[^']*/ic_launcher.xml'"` (>= 2) | — | ✅ green |
| 05-06-T1 | 06 | 6 | ANDR-07, D-08 | AVD api33; 16 KB zip + ELF alignment; no permissions; SDK levels | build | `avdmanager list avd \| grep -c 'Name: dimuli_api33'`; `android/scripts/check-apk.sh` -> `APK CHECK OK` | ❌ W0 (check-apk.sh created here) | ✅ green |
| 05-06-T2 | 06 | 6 | ANDR-01..06, D-07 (A1, A2) | All flows on API 33 and API 37 | emulator | file evidence `android/build/smoke/emulator-555{6,8}-{smoke,logcat,mediastore,chooser}.txt` (see plan) | — | ✅ green |
| 05-07-T2 | 07 | 7 | ANDR-01, ANDR-05, D-04, D-07, D-08 | Install + launch + back on Android 14 and 16 devices | device smoke | `android/scripts/check-apk.sh`; `android/build/smoke/device-*-smoke.txt` (>= 2 `SMOKE OK`) | — | 🟡 partial (Android 14 green; Android 16 deferred) |
| 05-07-T3 | 07 | 7 | ANDR-02..06, ANDR-08 | Finger editing, picker, save, share, back on devices | manual (checkpoint:human-verify) | — (owner) | — | ⏸ deferred (final device test) |
| 05-08-T1 | 08 | 8 | all ANDR | Code review, critical fixes, regression | review + gates | host gates + Android clippy + `check-apk.sh` + `smoke.sh -s emulator-5554` | — | ⬜ pending |
| 05-08-T2 | 08 | 8 | docs (guidelines §9), D-13 (regression guard) | Docs match the build; milestone 5 ticked | doc grep | `grep -c '^- \[x\] 5\.' docs/PLAN.md` (1) and doc gates in the plan | — | ⬜ pending |

Checkpoint tasks without automated verify: 05-06-T1 may raise a dynamic `checkpoint:human-action` if the API 33 system image cannot be installed non-interactively; 05-07-T1 (`checkpoint:human-action`, connect devices; verified by `adb devices`) and 05-07-T3 (`checkpoint:human-verify`). No three consecutive implementation tasks lack an automated command.

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `platform/android.rs` + `[lib] name = "dimuli"` (05-01-T1) — prerequisite for the Android library build
- [ ] `android/` Gradle project + wrapper (05-02-T1) — prerequisite for every APK and emulator check
- [ ] Host-testable mapping helpers in `platform/native_events.rs` (paths → `ImagesPicked` in 05-03-T1, ok/err → `Saved`/`Shared` in 05-04-T1) plus tests
- [ ] `android/scripts/smoke.sh` (install, launch, back closes; 05-02-T1) and `android/scripts/check-apk.sh` (zipalign, readelf, permissions, SDK levels; 05-06-T1)
- [ ] AVD `dimuli_api33` created in 05-06-T1 (`system-images;android-33;google_apis;x86_64`; owner runs the download only if sdkmanager needs a license confirmation)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Launch on real Android 14 and 16 devices | ANDR-01 | hardware | install via adb (`-s <serial>`), launch, check start screen |
| Picker flow → editor | ANDR-02 | system UI interaction | pick 3-5 images, editor opens; cancel leaves start screen unchanged |
| No content under system bars | ANDR-05 | visual | screenshots on API 33/36/37 + devices, portrait and landscape |
| Touch editing incl. pinch, long-press swap, busy overlay | ANDR-08 | multi-touch | on a real device; on the emulator never use the mouse wheel (crashes, Slint `todo!()`) |
| Rotation keeps the collage | D-05 | interaction | rotate in editor; collage and edits remain |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (checkpoints excepted)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [ ] Feedback latency < 300s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
