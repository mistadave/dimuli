# Phase 5: Android Shell - Research

**Researched:** 2026-11-13
**Domain:** Rust/Slint `NativeActivity` app shell on Android (Gradle + Kotlin + JNI), Photo Picker, MediaStore, sharing, insets, back, 16 KB alignment
**Confidence:** HIGH (core path built and run end-to-end on the `dimuli_api36` emulator during this research; API 33/34 paths and real devices still unverified)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### App Identity
- **D-01:** `applicationId` (and Kotlin package / `namespace`) is `ch.staeheli.dimuli`. — **Reversibility:** one-way — once the app is published to Google Play, the applicationId is the store identity and cannot change; changing it before that means renaming the Kotlin package, the manifest, the JNI symbol names (`Java_ch_staeheli_dimuli_...`) and the `FileProvider` authority.
- **D-02:** Launcher label is `dimuli` (lower case, like the project name).
- **D-03:** Launcher icon: the owner will supply the final icon later (format not decided yet: SVG layers or PNG). Phase 5 ships an **adaptive icon with a simple vector placeholder** (`mipmap-anydpi` adaptive icon with XML vector foreground/background, e.g. a stylised collage grid). Phase 5 does not wait for the real icon; swapping it in later must only replace the drawable resources.
- **D-04:** Phase 5 builds **debug-signed APKs only** (default debug keystore), installed with `adb install` on emulators and a real device. No release keystore, `signingConfig`, R8/minify or Play Store setup in this phase.
- **D-05:** No orientation lock: the app runs in portrait and landscape. (Android 16+ ignores orientation locks on large screens anyway.) The Slint UI must work in both orientations, and rotating must not lose the collage (see Claude's Discretion on `configChanges`).

### Platform Level (revised 2026-11-13, during plan-phase)
- **D-06:** `minSdk 33` (Android 13), `targetSdk`/`compileSdk 37` (Android 17); Rust build with `cargo ndk --platform 33`. Replaces the earlier minSdk 36. Rely only on APIs available from API 33: system Photo Picker via `MediaStore.ACTION_PICK_IMAGES` (+ `MediaStore.EXTRA_PICK_IMAGES_MAX`), `MediaStore.Images` insert without permission, `OnBackInvokedCallback` with `android:enableOnBackInvokedCallback="true"`. No fallbacks below API 33, no storage permissions. — **Reversibility:** costly — raising or lowering minSdk again touches the manifest/Gradle config, the `cargo ndk --platform` value in docs and agent files, the emulator test matrix, and any API used above the new floor.
- **D-07:** Edge-to-edge on **every** supported version: the app enables it itself on Android 13/14 (where it is not enforced) so the UI looks the same on 13-17, and insets reach Slint the same way on all versions.
- **D-08:** Phase 5 test matrix: emulators API 33, API 36 and API 37. A new AVD `dimuli_api33` (x86_64, Android 13 system image) must be created as part of this phase (SDK image download + `avdmanager`; may need the owner to run it). Real devices: one with **Android 14** (API 34; checks the self-enabled edge-to-edge of D-07 and the API-33-era code paths on hardware) and one with **Android 16**. The v1 acceptance stays on the real Android 16 device (ACPT-01, unchanged).

### Claude's Discretion
The owner chose not to discuss these areas; research and planning decide them within the constraints in PROJECT.md and `docs/CODING_GUIDELINES.md`:
- **Dependencies:** `NativeActivity` is not an AndroidX `ComponentActivity`, so the AndroidX `PickMultipleVisualMedia` contract named in `docs/PLAN.md` does not apply directly. Prefer platform APIs where they suffice (`MediaStore.ACTION_PICK_IMAGES` + `onActivityResult`, `OnBackInvokedCallback`, a minimal own `ContentProvider` vs AndroidX `FileProvider`). Any Rust crate (`jni`, `android_logger`, ...) or Gradle/AndroidX/Kotlin library that is still needed **must be presented to the owner for approval before use** (guideline rule, listed as a blocker in STATE.md) — the planner should make that an explicit checkpoint, not assume approval.
- **Save & share UX:** gallery file name (e.g. `dimuli-<timestamp>.jpg`, matching the desktop pattern), success message shown in the existing Slint message UI (not an Android Toast unless research shows a reason), when picked-image cache copies and the share file are deleted.
- **Rotation / configuration changes:** Since orientation is not locked (D-05), avoid Activity recreation losing state, e.g. via `android:configChanges` on the activity so `NativeActivity` keeps the native app alive; full lifecycle robustness is LIFE-01 in Phase 6.
- **Insets:** Check whether Slint 1.18's Android backend already provides safe-area insets itself before building a Kotlin -> JNI path; the existing `PlatformEvent::InsetsChanged` and `Backend.inset-*` properties are the target either way.
- **Workspace lint exception:** `unsafe_code = "deny"` needs a scoped exception limited to `platform/android.rs`, with `// SAFETY:` comments; JNI entry points catch panics.
- **Test scope:** Phase 5 is verified on emulators `dimuli_api33`, `dimuli_api36` and `dimuli_api37` (see D-08) plus the real Android 14 and Android 16 devices (ROADMAP success criterion 1 names them; the v1 acceptance on a real device is ACPT-01 in Phase 6).

### Deferred Ideas (OUT OF SCOPE)
- Release signing, R8/minify and Google Play setup — later, not part of v1 Phase 5.
- Final launcher icon from the owner — drop-in replacement once supplied (format still open).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ANDR-01 | Gradle project under `android/` (Kotlin `MainActivity : NativeActivity`, manifest, minSdk 33 / targetSdk 37) packages `libdimuli.so` built by `cargo ndk` for `arm64-v8a` and `x86_64`; installs and launches on emulators API 33/36/37 and real Android 14/16 devices | Verified toolchain: AGP 9.5.1 + Gradle 9.9.1 + JDK 25 + built-in Kotlin + compileSdk 37 builds an APK; launch to start screen verified on `dimuli_api36` (§Standard Stack, §Pattern 1, §Code Examples). `ANDROID_JAR` fix (§Pitfall 1), `[lib] name = "dimuli"` (§Pattern 2), AVD command (§Environment) |
| ANDR-02 | Pick up to 9 images with the system Photo Picker; URIs copied into the cache and delivered to Rust via JNI; cancel leaves start screen unchanged | `ACTION_PICK_IMAGES` + `EXTRA_PICK_IMAGES_MAX` + `onActivityResult` + copy on an executor + `RegisterNatives` callback verified end-to-end on API 36 (editor opened with 3 images) (§Pattern 4) |
| ANDR-03 | Save to gallery via `MediaStore.Images` (`Pictures/dimuli`) without permission; success/error message | `RELATIVE_PATH` + `IS_PENDING` insert verified: row `relative_path=Pictures/dimuli/, is_pending=0, owner_package_name=ch.staeheli.dimuli` (§Pattern 5) |
| ANDR-04 | Share via system share sheet (cache file, provider, `ACTION_SEND`); error message on failure | Own ~50-line `ContentProvider` (no AndroidX) + `ACTION_SEND` + `ClipData` grant verified: chooser opened with image preview (§Pattern 6). AndroidX `FileProvider` trade-off in §Alternatives |
| ANDR-05 | Edge-to-edge on every version; insets reach Slint `safe-area` | Slint 1.18 Android backend already reads insets in Java and pushes them via JNI into `Window.safe-area-insets` (verified in source and on device: top 50.29, bottom 24 logical px). Kotlin only enables edge-to-edge on API 33/34 (§Pattern 7). Requirement wording needs owner confirmation (§Open Questions 1) |
| ANDR-06 | Back gesture via Predictive Back callback forwarded to Rust | Own `OnBackInvokedCallback` registered **after** Slint's own callback (Rust triggers registration after `slint::android::init`) — verified: "back via dimuli callback" logged, app closed on start screen (§Pattern 8, §Pitfall 3) |
| ANDR-07 | 16 KB page alignment verified | NDK r30 output `LOAD ... Align 0x4000`, RELRO compliant; `zipalign -c -P 16 -v 4` → "Verification successful" on the built APK (§Pattern 9) |
| ANDR-08 | All editor interactions work with fingers | Slint backend forwards multi-touch as `InternalEvent::Touch` and synthesizes `PinchGesture` (verified in source); second finger synthesizes a `Released` for the primary finger (§Pitfall 6); mouse wheel in emulator crashes (`todo!()`, §Pitfall 5); landscape layout and missing glyph found (§Pitfall 9/10). Pinch needs a real device (manual) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Language Rust (stable, edition 2024), UI Slint with `backend-android-activity`; code and comments in English, communication in German.
- No `unwrap()` / `expect()` in production code (a panic crashes the app); workspace clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`.
- Logic stays free of Android dependencies (host-testable); Android-specific code is encapsulated (in `platform/`).
- UI changes are checked first on the desktop (`cargo run -p dimuli-app -- …`); Android build only for platform-specific changes.
- **New crates only after asking the owner and checking Android compatibility** (also Gradle dependencies and features, `docs/CODING_GUIDELINES.md` §8).
- All code is written by the `rust-coder` subagent; complex tasks are split into small subtasks. After a milestone / before committing new functionality, run the `code-reviewer` subagent and fix critical findings.
- Before finishing a change: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`.
- **No commits or pushes without explicit request.** (This research file was therefore not committed, although `commit_docs` is `true`.)
- `CLAUDE.md`'s `cargo apk` commands are stale (Phase 7 fixes them) — do not use `cargo apk`.
- From `docs/CODING_GUIDELINES.md` (binding): `unsafe` only in `platform/android.rs` with a `// SAFETY:` comment per block; JNI entry points catch panics; Kotlin callbacks never touch UI objects directly (always via `slint::invoke_from_event_loop`); JNI references not held beyond the call (use `Global` for long-lived); every Kotlin function called from/calling Rust lives in `MainActivity.kt` in `// region JNI bridge` with signatures documented on both sides; no extra manifest permissions; no AndroidX/third-party libraries without asking; `#[cfg(target_os = "android")]` only in `platform/`; the only allowed `#[allow]` so far is the generated `ui` module (this phase must add a documented second exception, see §Pattern 3).

## Summary

The whole Android path was prototyped in a scratch copy of the repository during this research and works on the `dimuli_api36` emulator: Gradle (AGP 9.5.1, Gradle 9.9.1, built-in Kotlin, JDK 25) packages a `libdimuli.so` built by `cargo ndk --platform 33`. The Slint start screen renders edge-to-edge. The system Photo Picker returns 3 images, which Kotlin copies into `cacheDir` and passes to Rust over JNI, and the editor opens with them. Saving writes `Pictures/dimuli/dimuli-<seconds>.jpg` through MediaStore without any permission. Sharing opens the system chooser through a ~50-line self-written `ContentProvider`, so AndroidX is not needed. Back on the start screen is routed through a Kotlin `OnBackInvokedCallback` to the Rust controller, which closes the app. Rotation keeps the collage because `configChanges` prevents Activity recreation. Relaunching after `finish()` in the same process works, because Slint keeps its platform in a thread-local and `android_main` runs on a new thread each time. The APK passes `zipalign -c -P 16`.

Three findings change the plan compared to `docs/PLAN.md`:
1. **Slint 1.18 already handles insets.** Its Android backend ships a Java helper that reads system-bar and cutout insets and pushes them into the built-in `Window.safe-area-insets` property, so no Kotlin → JNI inset path is needed. `app.slint` only has to use `root.safe-area-insets`.
2. **Slint 1.18 also registers its own `OnBackInvokedCallback` (API 34+).** dimuli's own callback must be registered *after* Slint's, or Slint's wins and closes the app from the editor.
3. **Build breaks unless `ANDROID_JAR` is set.** `cargo ndk --platform 33` exports `ANDROID_PLATFORM=33`, so Slint's build script looks for `platforms/android-33/android.jar`. That file does not exist here, and installing it would not help because Slint's Java needs API 34 constants. Point `ANDROID_JAR` at `platforms/android-37.0/android.jar`.

New dependencies needing owner approval are `jni 0.22.4` (already in the dependency tree through Slint and android-activity, so no new code is downloaded), `android_logger 0.15.1`, the Slint feature `backend-android-activity-06`, the Gradle plugin `com.android.application 9.5.1`, and the implicit `kotlin-stdlib 2.2.10` that built-in Kotlin adds to the APK. No AndroidX library is needed.

**Primary recommendation:** Build exactly the verified shape: Kotlin `MainActivity : NativeActivity` with methods the Rust side calls on the activity object, Rust registers the Kotlin `external fun`s with `RegisterNatives` from `android_main` (`jni 0.22` `native_method!`), insets come from Slint's `safe-area-insets`, a Gradle `Exec` task runs `cargo ndk` with `ANDROID_JAR` before `preBuild`, and every Rust↔Kotlin call returns immediately (work on `runOnUiThread` / a single-thread executor, results via `post_event`).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Activity lifecycle, intents, `onActivityResult` | Kotlin `MainActivity` (Java main thread) | — | Only the Activity can start activities and receive results; NativeActivity does not forward them to native code |
| Photo Picker launch + URI → cache copy | Kotlin (UI thread launch, executor thread copy) | Rust receives paths | ContentResolver is Java-only; copying off the UI thread avoids ANR |
| Decoding, collage state, export JPEG | Rust (`collage-core`, `dimuli-app`) | — | Unchanged; host-testable |
| Save to MediaStore | Kotlin executor thread | Rust supplies JPEG bytes, gets `Saved` | ContentResolver insert/IS_PENDING is a Java API |
| Share | Kotlin (`ShareProvider` + `ACTION_SEND`) | Rust supplies bytes, gets `Shared` | Chooser and URI grants need an Activity and a ContentProvider |
| Safe-area insets | Slint Android backend (its Java helper + JNI) | Kotlin enables edge-to-edge on API 33/34 | Slint already measures insets; the app only lays out with them |
| Back gesture | Kotlin `OnBackInvokedCallback` → JNI → Rust controller | Kotlin `finish()` on `close_app` | Decision (confirm / go back / close) is Rust state logic |
| Touch / pinch | Slint backend (android-activity input queue) | Rust gesture state machine | Unchanged; Slint synthesizes pinch from touches |
| Logging | Rust `log` → `android_logger` (tag `dimuli`) | Kotlin `Log.e("dimuli", …)` | `adb logcat -s dimuli` |
| Native build + packaging | Gradle `Exec` task → `cargo ndk` → `jniLibs` → AGP | — | One command builds everything; AGP strips and 16 KB-aligns |

## Standard Stack

### Core (Rust, Android target only)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `slint` feature `backend-android-activity-06` | slint 1.18.1 (locked) | Android backend (android-activity 0.6.1 `native-activity`, Skia renderer) | The feature name in 1.18 is `backend-android-activity-06`, not `backend-android-activity` [VERIFIED: slint-1.18.1/Cargo.toml `backend-android-activity-06 = ["i-slint-backend-android-activity/native-activity", "i-slint-backend-android-activity/aa-06", "i-slint-backend-selector/backend-android-activity"]`] |
| `jni` | 0.22.4 | Calling Kotlin methods, `RegisterNatives`, `native_method!` with built-in `catch_unwind` | Same major as Slint's backend (`jni = "0.22"`) and android-activity 0.6.1 (`jni 0.22.4`) → one copy, shared `JavaVM::singleton()` [VERIFIED: crates.io registry + local crate manifests] |
| `android_logger` | 0.15.1 (`default-features = false`) | `log` → logcat with tag `dimuli` | De-facto standard (rust-mobile org), MIT/Apache-2.0; `default-features = false` drops the `regex` filter feature [VERIFIED: crates.io registry; compiled for x86_64-linux-android] |

### Build toolchain (Gradle side)
| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| Android Gradle Plugin `com.android.application` | 9.5.1 | APK build, strip, 16 KB zip alignment | Max API 37, min Gradle 9.6.0, JDK 17+ [CITED: developer.android.com/build/releases/agp-9-5-0-release-notes]; latest on Google Maven [VERIFIED: dl.google.com maven-metadata] |
| Gradle (wrapper) | 9.9.1 | Build runner | Current release; runs on Java 25 since 9.1.0 [CITED: docs.gradle.org compatibility]; sha256 `d8a18a7a841dd5df825d35a685fad6f2d7f12ac0c280e57b67c27b4a520ba228` [VERIFIED: services.gradle.org] |
| Kotlin | built-in Kotlin of AGP 9 (KGP 2.2.10) | Compiles `MainActivity.kt` | AGP 9 compiles Kotlin without applying `org.jetbrains.kotlin.android`; adds `kotlin-stdlib` automatically [CITED: developer.android.com/build/migrate-to-built-in-kotlin; VERIFIED: AGP 9.5.1 POM depends on `kotlin-gradle-plugin` 2.2.10] |
| JDK | OpenJDK 25.0.5 (installed) | Gradle + javac for Slint's Java helper | Worked for AGP 9.5.1 and Slint's `javac -source 8 -target 8` (obsolete-option warnings only) [VERIFIED: local build] |
| NDK | r30 `30.0.16248370` (installed) | Linker; 16 KB alignment by default | Set `ndkVersion` in Gradle so AGP strips with this NDK [VERIFIED: APK lib shrank 210 MB → 30 MB debug] |
| cargo-ndk | 4.1.2 (installed) | Builds per ABI, copies into `jniLibs` | Exports `ANDROID_PLATFORM` (see §Pitfall 1) |

**Verified working combination (built + run in this session):** AGP 9.5.1 · Gradle 9.9.1 · JDK 25 · compileSdk 37 (`platforms/android-37.0`) · minSdk 33 · NDK 30.0.16248370 · Rust 1.99.1 · slint 1.18.1 · jni 0.22.4 · android_logger 0.15.1.

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Own `ShareProvider` (~50 lines Kotlin) | AndroidX `androidx.core:core` `FileProvider` | FileProvider is the documented standard and handles more edge cases, but pulls AndroidX (core + annotation + lifecycle, etc.), needs owner approval, and is overkill for serving one file. Own provider verified working with the system chooser. **Recommend own provider.** |
| `RegisterNatives` from `android_main` | `external fun` resolved by exported `Java_ch_staeheli_dimuli_MainActivity_*` symbols (`native_method!{ extern ... }`) + `System.loadLibrary("dimuli")` in Kotlin | Exported symbols survive the "native called before registration" race (process death while the picker is open) but need `System.loadLibrary` because NativeActivity loads the `.so` itself [ASSUMED — not tested]. RegisterNatives is verified; guard the race in Kotlin instead (§Pitfall 4). **Recommend RegisterNatives.** |
| `android_logger` crate | ~25-line own `log::Log` impl over `__android_log_write` (extern "C", liblog) | Saves one dependency but adds more `unsafe` FFI in `android.rs`. **Recommend `android_logger`** (if not approved, use the own logger). |
| Kotlin `MainActivity` | Java `MainActivity` | Java avoids the implicit `kotlin-stdlib` in the APK (~2.4 MB dex in debug), but `docs/PLAN.md` and the guidelines name Kotlin. Keep Kotlin; flag `kotlin-stdlib` in the approval checkpoint. |
| Gradle `Exec` task for `cargo ndk` | Manual two-step (`cargo ndk …` then `./gradlew assembleDebug`) or a shell script | Exec task gives one command and cannot forget a stale `.so`; verified working. Both are fine for Phase 7 docs. **Recommend Exec task** with `-PrustAbis=x86_64` for fast emulator builds. |

**Installation (after owner approval):**
```toml
# crates/dimuli-app/Cargo.toml
[lib]
name = "dimuli"                     # -> libdimuli.so, matches android.app.lib_name
crate-type = ["cdylib", "rlib"]

[target.'cfg(target_os = "android")'.dependencies]
slint = { workspace = true, features = ["backend-android-activity-06"] }
jni.workspace = true
android_logger.workspace = true

# Cargo.toml [workspace.dependencies]
jni = "0.22.4"
android_logger = { version = "0.15.1", default-features = false }
```
Renaming the lib changes `dimuli_app::` to `dimuli::` in `src/main.rs` (only user of the lib path). A lib and a bin both named `dimuli` build without collision warnings, including `cargo doc` [VERIFIED: host build of the probe].

**Version verification:** `jni` 0.22.4 (max stable, 2026-03-16), `android_logger` 0.15.1 (2025-06-29), `slint` 1.18.1 (2026-09-21), `android-activity` 0.6.1, `cargo-ndk` 4.1.2 [VERIFIED: crates.io API this session].

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| jni | crates.io | ~10 yrs (2016) | ~6.6 M/wk | github.com/jni-rs/jni-rs | [OK] | Approved by audit — **owner approval still required** (guideline §8) |
| android_logger | crates.io | ~10 yrs (2016) | ~475 k/wk | github.com/rust-mobile/android_logger-rs | [OK] | Approved by audit — **owner approval still required** |
| com.android.application (AGP) | Google Maven | — | — | Google | not run (Maven not supported by the audit tool) | Official Google build plugin — **owner approval as Gradle dependency** |
| org.jetbrains.kotlin:kotlin-stdlib 2.2.10 (implicit) | Maven Central | — | — | JetBrains | not run | Added automatically by built-in Kotlin — **owner approval as runtime library** |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
Both Rust crates are already in the dependency graph for Android through Slint/android-activity (`jni 0.22.4` via `i-slint-backend-android-activity` and `android-activity`; `jni` is in the current `Cargo.lock` via `webbrowser`). Making them direct dependencies adds no new code. **The planner must still add a `checkpoint:human-verify` (owner approval) before touching `Cargo.toml` / Gradle dependencies** (CONTEXT Claude's Discretion → Dependencies).

## Architecture Patterns

### System Architecture Diagram

```
 User tap "Bilder wählen"
   │
   ▼
 Slint UI (android_main thread) ──callback──▶ Controller ──▶ AndroidPlatform::pick_images(9)
                                                                 │ JNI call_method on Activity object
                                                                 ▼
                                   Kotlin MainActivity.pickImages(max) ──runOnUiThread──▶ startActivityForResult(ACTION_PICK_IMAGES)
                                                                                                │
                                                       System Photo Picker (other process) ◀────┘
                                                                 │ result (clipData / data)
                                                                 ▼
                         Kotlin onActivityResult (Java main thread) ──▶ executor thread: copy URIs → cacheDir/picked/<n>/img-i
                                                                 │ nativeImagesPicked(String[])   (registered by RegisterNatives)
                                                                 ▼
                         Rust native fn (catch_unwind via native_method!) ──▶ post_event(ImagesPicked) ──invoke_from_event_loop──▶ Controller
                                                                                                                          │
                     Export thread renders JPEG ◀── Controller (Save/Share) ◀──────────────────────────────────────────────┘
                                │ bytes
                                ▼
          AndroidPlatform::save_to_gallery / share ──JNI──▶ Kotlin executor: MediaStore insert (IS_PENDING) / write cache/share + ACTION_SEND chooser
                                                                 │ nativeSaved(ok) / nativeShared(ok)
                                                                 ▼
                                                        post_event(Saved/Shared) → message banner

 Insets:  Slint Java helper (WindowMetrics insets) ──JNI──▶ Slint backend ──▶ Window.safe-area-insets ──▶ app.slint padding
 Back:    System back ──▶ Kotlin OnBackInvokedCallback (registered after Slint's) ──nativeBackPressed──▶ post_event(BackPressed)
          ──▶ Controller: editor → confirm dialog; start screen → close_app() ──JNI──▶ Activity.finish()
```

### Recommended Project Structure
```
android/
├── settings.gradle.kts            # pluginManagement + google()/mavenCentral(), include(":app")
├── build.gradle.kts               # plugins { id("com.android.application") version "9.5.1" apply false }
├── gradle.properties              # org.gradle.jvmargs
├── gradlew, gradlew.bat, gradle/wrapper/{gradle-wrapper.jar,gradle-wrapper.properties}   # committed
└── app/
    ├── build.gradle.kts           # android {...}, buildRust Exec task
    └── src/main/
        ├── AndroidManifest.xml
        ├── kotlin/ch/staeheli/dimuli/{MainActivity.kt, ShareProvider.kt}
        ├── res/values/themes.xml  # edge-to-edge theme
        ├── res/mipmap-anydpi/ic_launcher.xml + res/drawable/ic_launcher_{foreground,background}.xml  # D-03 placeholder
        └── jniLibs/               # generated by cargo ndk (already gitignored)
crates/dimuli-app/src/platform/
├── mod.rs                         # + #[cfg(target_os = "android")] mod android;
├── desktop.rs
└── android.rs                     # android_main, AndroidPlatform, native methods (only file with unsafe)
```
`.gitignore` already ignores the generated libraries [VERIFIED: .gitignore:9 `/android/app/src/main/jniLibs/`] and `/android/.gradle/`, `/android/build/`, `/android/app/build/`, `/android/local.properties` (lines 5-8).

### Pattern 1: `android_main` lives in `platform/android.rs`
**What:** android-activity calls the Rust symbol `android_main(app: AndroidApp)` on its own new thread (named `android_main`, already attached to the JVM, thread context class loader = app class loader). It wraps it in `catch_unwind` and calls `ANativeActivity_finish` when it returns or panics [VERIFIED: android-activity-0.6.1/src/native_activity/glue.rs:925-995].
**How:** `#[unsafe(no_mangle)] fn android_main(app: slint::android::AndroidApp)` in a private module is exported from the cdylib (`nm -D` shows `T android_main`) [VERIFIED: probe build]. Inside: init `android_logger`, `slint::android::init(app.clone())`, register natives, ask Kotlin to install the back callback, then `crate::run(Rc::new(AndroidPlatform { app }))`. Existing entry: [VERIFIED: crates/dimuli-app/src/lib.rs:31 `pub fn run(platform: Rc<dyn Platform>) -> Result<(), slint::PlatformError> {`].
`slint::android` re-exports `AndroidApp` and the whole `android_activity` crate (`slint::android::android_activity`) — no direct `android-activity` dependency is needed [VERIFIED: slint-1.18.1/android.rs:82-103].
**Re-entry:** `android_main` can run several times per process (back → `finish()` → launcher again). Slint keeps its context in a `thread_local` and replaces the Android event-loop proxy, so `slint::android::init` succeeds again on the new thread [VERIFIED: i-slint-core-1.18.1/platform.rs:259-282 and a warm relaunch on the emulator]. Therefore **never store the Activity or `AndroidApp` in a `static`/`OnceLock`**; keep them in `AndroidPlatform` (android-activity docs warn the same about `activity_as_ptr`).

### Pattern 2: Library name and Cargo features
`crates/dimuli-app/Cargo.toml` currently has [VERIFIED: crates/dimuli-app/Cargo.toml:10-15]:
```
[lib]
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "dimuli"
path = "src/main.rs"
```
and [VERIFIED: :23-24] `[target.'cfg(not(target_os = "android"))'.dependencies]` / `env_logger.workspace = true`. Add `name = "dimuli"` to `[lib]` and the Android-only dependencies shown in §Installation. Feature unification is per target with resolver 3, so the desktop build does not get the Android backend (winit/femtovg were not compiled for Android either) [VERIFIED: probe build logs].

### Pattern 3: Scoped lint exceptions in `android.rs`
Workspace lints [VERIFIED: Cargo.toml:19-26]:
```
[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
indexing_slicing = "deny"
```
`deny` (not `forbid`) can be overridden locally. Put `#![allow(unsafe_code)]` as the first inner attribute of `platform/android.rs` with a reason comment. `unsafe_code` also fires for `#[unsafe(no_mangle)]`, so `android_main` must be in that file. Needed `unsafe` sites: `#[unsafe(no_mangle)]`, `env.as_cast_raw::<Global<JObject>>(&raw)` (Activity pointer), `env.register_native_methods(...)`. Each gets a `// SAFETY:` comment.
A second scoped allow is needed: `jni 0.22.4`'s `native_method!` expansion calls `AtomicBool::fetch_update`, which Rust 1.99 deprecates → `deprecated` warning in our crate (fails `-D warnings`). Put `#[allow(deprecated)]` on the `NATIVES` const only, with a comment [VERIFIED: probe build output "use of deprecated method `std::sync::atomic::Atomic::<bool>::fetch_update`: renamed to `try_update`"]. With both allows, `cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings` reports nothing in `android.rs` [VERIFIED].
`docs/CODING_GUIDELINES.md` §3 says the `ui` module is the *only* `#[allow]` exception, so the plan must update that section (doc change) to list `android.rs` (`unsafe_code`, and `deprecated` on `NATIVES`). Optional hardening: add workspace lint `clippy::undocumented_unsafe_blocks = "deny"` to enforce the SAFETY comments.

### Pattern 4: Photo Picker (platform API, no AndroidX)
- `Intent(MediaStore.ACTION_PICK_IMAGES).setType("image/*")`; add `EXTRA_PICK_IMAGES_MAX` only if the value is > 1. It must be ≤ `MediaStore.getPickImagesMaxLimit()`, otherwise the picker returns `RESULT_CANCELED` [CITED: developer.android.com/reference/android/provider/MediaStore#EXTRA_PICK_IMAGES_MAX; VERIFIED: android-37.0 stubs MediaStore.java:1251-1260].
- `startActivityForResult` / `onActivityResult(int,int,Intent)` on the platform `Activity` are **not deprecated** in API 37 (only AndroidX deprecates them) [VERIFIED: android-37.0 android-stubs-src.jar Activity.java:133, 343].
- Collect URIs from `data.clipData` (multi-select) and fall back to `data.data` [ASSUMED for the single-item case; verified for 3 items].
- Copy each URI with `contentResolver.openInputStream(uri)` on a single-thread executor into `cacheDir/picked/...`, then call `nativeImagesPicked(paths)`. Extension-less names (`img-0`) are fine: decoding sniffs the content [VERIFIED: crates/collage-core/src/decode.rs:38-40 `let mut reader = ImageReader::open(path)` … `.with_guessed_format()`; probe decoded a PNG named `img-2`].
- Cancel (`resultCode != RESULT_OK` or no URIs) → `nativeImagesPicked(emptyArray())` → controller already ignores an empty list ("cancelled by the user").
- Picker URI read access lasts until the app stops or the device restarts, so copy right away (no `takePersistableUriPermission` needed) [CITED: developer.android.com/training/data-storage/shared/photo-picker].
- Cache cleanup (discretion): delete `cacheDir/picked` at the start of each new pick and in `onCreate` (left over after process death). Picks only start from the start screen, where no collage holds the old files.

### Pattern 5: MediaStore save (no permission)
`ContentValues(DISPLAY_NAME = "dimuli-<unix seconds>.jpg", MIME_TYPE = "image/jpeg", RELATIVE_PATH = Environment.DIRECTORY_PICTURES + "/dimuli", IS_PENDING = 1)` → `insert(MediaStore.Images.Media.getContentUri(VOLUME_EXTERNAL_PRIMARY))` → `openOutputStream(uri).write(jpeg)` → `update(IS_PENDING = 0)`. On failure, `delete(uri)` the half-written row, then `nativeSaved(false)`. Verified result: `_display_name=dimuli-1794567059.jpg, relative_path=Pictures/dimuli/, is_pending=0, owner_package_name=ch.staeheli.dimuli` [VERIFIED: emulator API 36]. MediaStore picks a unique name itself if the name exists. The success text exists already [VERIFIED: crates/dimuli-app/src/texts.rs:15 `pub(crate) const SAVED: &str = "Collage gespeichert.";`].

### Pattern 6: Share without AndroidX
- Kotlin writes the bytes to `cacheDir/share/dimuli-collage.jpg` and builds `Intent(ACTION_SEND)` with `type = "image/jpeg"`, `EXTRA_STREAM = content://ch.staeheli.dimuli.share/dimuli-collage.jpg`, `clipData = ClipData.newRawUri(null, uri)` (makes the grant reach the chooser and enables the preview), and `FLAG_GRANT_READ_URI_PERMISSION`. It then calls `startActivity(Intent.createChooser(send, null))` on the UI thread.
- `ShareProvider : ContentProvider` (manifest: `exported="false"`, `grantUriPermissions="true"`, authority `ch.staeheli.dimuli.share`) serves exactly that one URI read-only: `openFile` → `ParcelFileDescriptor.open(file, MODE_READ_ONLY)`, `query` → `OpenableColumns.DISPLAY_NAME/SIZE` via `MatrixCursor`, `getType` → `image/jpeg`, and every other URI → `FileNotFoundException` [VERIFIED: chooser "Sharing image" with preview on API 36].
- Android cannot report whether the user actually shared. Post `Shared(Ok)` when the chooser started and `Shared(Err(SHARE_FAILED))` on an exception (e.g. `ActivityNotFoundException`). The existing text `SHARED = "Collage geteilt."` [VERIFIED: texts.rs:16] then claims more than is known (see Open Questions 3).
- Overwrite the single share file on each share; it lives in the cache (the system may clear it). Do not delete it right after `startActivity`, because the receiver reads it asynchronously.

### Pattern 7: Edge-to-edge and insets
- **Slint already provides the insets.** Its Java helper reads `getWindowManager().getCurrentWindowMetrics().getWindowInsets()` (`systemBars` max `displayCutout`) and calls native `setInsets`. The backend then calls `set_window_item_safe_area(...)`, which feeds the built-in `out property <Edges> safe-area-insets` of `Window` [VERIFIED: SlintAndroidJavaHelper.java:521-548; androidwindowadapter.rs:536-545; i-slint-compiler-1.18.1/builtin_elements.rs:2144]. `Edges` has `left, top, right, bottom` [VERIFIED: i-slint-common-1.18.1/builtin_structs.rs:204-213]. On device: portrait `top 50.29 bottom 24 left 0 right 0`; landscape `top 28.19 bottom 24 left 50.29` (cutout) [VERIFIED: emulator log].
- Current layout uses only Backend insets [VERIFIED: crates/dimuli-app/ui/app.slint:20-25]:
  ```
      // Content is inset by the safe area (system bars, display cutout).
      Rectangle {
          x: Backend.inset-left;
          y: Backend.inset-top;
          width: parent.width - Backend.inset-left - Backend.inset-right;
          height: parent.height - Backend.inset-top - Backend.inset-bottom;
  ```
  Change it to `max(Backend.inset-*, root.safe-area-insets.*)` (see Code Examples). The desktop stays unchanged (both are 0), and `PlatformEvent::InsetsChanged` / `Backend.inset-*` stay as the desktop/test path. No Kotlin → JNI inset code is needed.
- **Kotlin side (D-07):** On API 33/34, call `window.setDecorFitsSystemWindows(false)` (deprecated in API 37 stubs → guard with `Build.VERSION.SDK_INT < 35` and `@Suppress("DEPRECATION")`) [VERIFIED: android-37.0 stubs Window.java:120-121]. The theme sets transparent bars, `enforce*BarContrast=false`, `windowLayoutInDisplayCutoutMode=always`, light-icon off (dark UI: `Theme.background: #101114` [VERIFIED: crates/dimuli-app/ui/theme.slint:4 `out property <color> background: #101114;`]) and `windowBackground` = that colour (avoids a white flash before Slint renders). From API 35 with targetSdk ≥ 35, edge-to-edge is enforced anyway.
- **Why edge-to-edge matters on 33/34:** Slint reads raw WindowMetrics insets whether or not the window really draws behind the bars. If edge-to-edge were off there, the content would be inset twice. Verify on `dimuli_api33` and the Android 14 device (status-bar area must show the app background).

### Pattern 8: Back gesture
- Slint's helper registers its **own** `OnBackInvokedCallback` on API ≥ 34 (`UPSIDE_DOWN_CAKE`) and turns back into a `Key::Back` key event. If the Slint UI does not accept it, Slint calls `finish_activity()`, which **closes the app even in the editor** [VERIFIED: SlintAndroidJavaHelper.java:456-462, javahelper.rs:788-811].
- The dispatcher calls the **last registered** callback of the highest priority [CITED: developer.android.com/guide/navigation/custom-back/predictive-back-gesture "the last added enabled callback handles the next back gesture"].
- Therefore: Kotlin `installBackCallback()` (`PRIORITY_DEFAULT`) is **called from Rust right after `slint::android::init()`**. Both registrations are `runOnUiThread` posts from the same `android_main` thread, so Slint's runs first and ours ends up on top. Verified: `CoreBackPreview` shows two priority-0 callbacks, `KEYCODE_BACK` → "probe: back via dimuli callback" → controller `NotHandled` → `close_app()` → `finish()` [VERIFIED: emulator API 36]. Do **not** register in `onCreate` (Slint's would be on top on API ≥ 34).
- On API 33 Slint registers nothing; with `enableOnBackInvokedCallback="true"` the system delivers back only to our callback [ASSUMED — verify on `dimuli_api33`].
- Controller path exists [VERIFIED: crates/dimuli-app/src/controller.rs:155-161]:
  ```
              PlatformEvent::BackPressed => {
                  let outcome = self.state.borrow_mut().request_back();
                  self.refresh();
                  if outcome == BackOutcome::NotHandled {
                      self.platform.close_app();
                  }
              }
  ```
  `close_app` on Android = JNI `activity.finish()` (verified). Unregister the callback in `onDestroy`.
- Side effect: while our callback is registered, the system "back-to-home" predictive animation does not play. Optional later improvement: Rust tells Kotlin when the start screen is shown, so Kotlin unregisters there.

### Pattern 9: Build integration and 16 KB
- Gradle `Exec` task `buildRust` runs `cargo ndk -t arm64-v8a -t x86_64 --platform 33 -o app/src/main/jniLibs build -p dimuli-app --lib --release`, with env `ANDROID_JAR=<sdk>/platforms/android-37.0/android.jar` (from `androidComponents.sdkComponents.sdkDirectory`), and `preBuild` depends on it. `-PrustAbis=x86_64` limits the build to the emulator ABI [VERIFIED: task ran inside `./gradlew assembleDebug`].
- Build Rust with `--release` for every APK. A debug Rust build took ~18 s to export a 3-image collage on the emulator, release ~3 s, and a release build of all dependencies takes about 1 min [VERIFIED].
- `android { ndkVersion = "30.0.16248370"; buildToolsVersion = "37.0.0" }`. Without `buildToolsVersion`, AGP 9.5.1 silently **downloaded build-tools 36.0.0** into the SDK during this research [VERIFIED: Gradle log "Installing Android SDK Build-Tools 36"].
- 16 KB: NDK r30 links `LOAD … Align 0x4000`. The last RW `LOAD` equals `GNU_RELRO` (case "RELRO in its own LOAD segment", compliant). AGP stores the `.so` uncompressed (`Stored`), and `zipalign -c -P 16 -v 4 app-debug.apk` ends with "Verification successful" [VERIFIED: x86_64 probe APK; arm64-v8a release `.so` also shows `Align 0x4000` on every LOAD and a compliant RELRO; CITED: developer.android.com/guide/practices/page-sizes]. `zipalign` is not on PATH: use `$ANDROID_HOME/build-tools/37.0.0/zipalign`.
- Manifest: `<meta-data android:name="android.app.lib_name" android:value="dimuli"/>`. Keep `hasCode` at its default `true` (Kotlin code; cargo-apk's `hasCode="false"` must not be copied). Set `android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|layoutDirection|locale"` (rotation verified without recreation).

### Anti-Patterns to Avoid
- **`find_class("ch/staeheli/dimuli/MainActivity")` from a native thread:** call methods on the Activity *object* (`call_method(activity, ...)`) and get the class with `get_object_class(activity)`. android-activity also sets the thread context class loader, so `env.load_class` works on `android_main` [VERIFIED: android-activity init.rs:249-263].
- **Blocking the `android_main` thread waiting for the Java main thread:** can deadlock lifecycle callbacks (android-activity docs). Everything returns immediately; results come back via `post_event`.
- **Touching Slint objects from JNI natives:** natives run on the Java main/executor thread → only `post_event` (wraps `slint::invoke_from_event_loop`) [VERIFIED: crates/dimuli-app/src/platform/mod.rs:53-55 `pub(crate) fn post_event(event: PlatformEvent) {` / `crate::controller::post(move |controller| controller.handle_platform_event(event));`].
- **Wrapping `app.activity_as_ptr()` in `Global`/`Auto`:** it is an unowned global ref; dropping a wrapper would delete it. Use `env.as_cast_raw::<Global<JObject>>(&raw)` (borrowing cast).
- **Registering the back callback in `onCreate`:** loses to Slint's callback on API ≥ 34 (Pattern 8).
- **Using `MediaStore` without `IS_PENDING`:** gallery apps may show a half-written file.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Panic safety + error mapping at JNI entry points | Manual `extern "system"` + `catch_unwind` + fn-pointer casts | `jni::native_method!` with `error_policy = jni::errors::LogErrorAndDefault` | Type-checks the Java signature at compile time, wraps `catch_unwind`, logs errors via `log` [VERIFIED: jni-0.22.4 docs/macros/native_method.md] |
| Inset measurement | Kotlin `OnApplyWindowInsetsListener` + new JNI method | Slint `Window.safe-area-insets` | Already implemented and pushed via JNI by Slint (Pattern 7) |
| JVM attach / thread names / class loader | Manual `AttachCurrentThread` | `JavaVM::singleton()?.attach_current_thread(|env| …)` | android-activity initializes the singleton before `android_main` [VERIFIED: glue.rs:931-935] |
| Logcat | Own FFI logger | `android_logger` | Mature, handles tag/level/line splitting |
| Gallery insert | Writing files to `/sdcard/Pictures` | `MediaStore` + `RELATIVE_PATH` + `IS_PENDING` | No permission needed, gallery indexing included |
| 16 KB alignment | Linker flags | NDK r28+ default + AGP ≥ 8.5.1 packaging | Defaults are correct (verified) |

**Key insight:** Slint's Android backend already does much of the platform work (insets, back key, IME, clipboard, input). The shell must only add what Slint cannot do (activity results, MediaStore, share) and must not fight Slint where both act (the back callback order).

## Common Pitfalls

### Pitfall 1: `No Android platforms found` in Slint's build script
**What goes wrong:** `cargo ndk --platform 33 build` fails in `i-slint-backend-android-activity` build.rs ("No Android platforms found").
**Why:** cargo-ndk exports `ANDROID_PLATFORM=33` [VERIFIED: cargo-ndk-4.1.2/src/cli/mod.rs:635-644]. Slint's `android_build::android_jar(None)` then looks only for `platforms/android-33/android.jar` [VERIFIED: android-build-0.1.4/src/env_paths/mod.rs:70-91], and only `platforms/android-37.0` is installed. Installing `platforms;android-33` would not fix it: Slint's Java uses `Build.VERSION_CODES.UPSIDE_DOWN_CAKE` (API 34) [VERIFIED: SlintAndroidJavaHelper.java:458].
**How to avoid:** Always set `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar` (highest priority in `android_jar()`), in the Gradle task and in the documented manual command.
**Warning signs:** The build fails before any Rust code compiles; `cargo:rerun-if-env-changed=ANDROID_PLATFORM` appears in the output.

### Pitfall 2: Slint's back callback closes the app from the editor
See Pattern 8. **Warning sign:** back in the editor closes the app instead of showing the confirm dialog (API ≥ 34 only).

### Pitfall 3: Back delivered before natives are registered / race at startup
Between Activity start and `register_natives`, a back press or a late `onActivityResult` (process death while the picker was open → new process → result delivered to the new Activity before `android_main` registered the natives) calls an unregistered `external fun` → `UnsatisfiedLinkError` → crash.
**How to avoid:** Kotlin wraps every native call: `try { nativeX(...) } catch (e: UnsatisfiedLinkError) { Log.w("dimuli", "native not ready", e) }`. In the process-death case, dropping the pick result is acceptable (fresh start screen; LIFE-01 is Phase 6). `post_event` before the controller exists only logs "event dropped: no controller" [VERIFIED: controller.rs:29-40].

### Pitfall 4: `clippy -D warnings` for the Android target
Two issues appear only when linting `--target *-android`:
- the `native_method!` deprecation (Pattern 3, fix with a scoped allow);
- a **false positive** `clippy::missing_const_for_thread_local` on the existing `thread_local!` in `controller.rs:22-25`, which already uses `const { … }` (host clippy is clean) [VERIFIED: probe].
**How to avoid:** Keep the CLAUDE.md gate as host clippy. For an Android lint run, use `cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` (does not touch code). Do not add another `#[allow]` in `controller.rs`.

### Pitfall 5: Mouse wheel in the emulator crashes the app
Slint's Android input handling has `MotionAction::Scroll => todo!()` [VERIFIED: i-slint-backend-android-activity-1.18.1/androidwindowadapter.rs:443]. A wheel/trackpad scroll (emulator window mouse wheel, Bluetooth mouse) panics. android-activity catches the panic (log tag `RustPanic`) and finishes the Activity. NativeActivity takes the input queue, so Kotlin cannot filter it.
**How to avoid:** Testers must not use the mouse wheel in the emulator. Record it as a known Slint limitation (upstream issue). The desktop "Mausrad: zoomen" hint is wrong on Android but harmless.

### Pitfall 6: Second finger synthesizes a `Released` of the first finger
On the second touch, Slint's core sends `Released` for the primary finger before pinching [VERIFIED: i-slint-core-1.18.1/input.rs ~2251-2276 "Synthesize a Release for the primary finger"]. The canvas sends `canvas-released` → the Rust gesture machine gets `release` before `pinch-started`. If a long-press swap was active, the release can **drop the swap at the first finger's position**.
**How to avoid:** ANDR-08 device test case: long-press, then add a second finger. If the drop is wrong, cancel the swap on `pinch_started` (state logic, host-testable).

### Pitfall 7: Photo Picker shows "No photos yet" in the emulator after `adb push`
Pushed files are in MediaStore at once (`content query` lists them), but the picker keeps its own cache.
**How to avoid:** `adb shell am force-stop com.google.android.photopicker` (and wait a few seconds) after pushing test images [VERIFIED: emulator API 36].

### Pitfall 8: Debug-symbol `.so` is huge; release is needed for usable speed
Debug `libdimuli.so` is 210 MB unstripped. AGP strips it (needs `ndkVersion`), but debug Rust is slow (export ~6× slower). Build Rust with `--release` for APKs (Pattern 9).

### Pitfall 9: Missing glyph on Android
`editor-screen.slint:28` [VERIFIED: `text: Texts.settings + (root.settings-open ? " ▾" : " ▴");`] renders "▴" as a tofu box on Android (the system font has no U+25B4). "‹" in the top bar renders fine [VERIFIED: emulator screenshot]. Replace it with a drawn chevron (`Path`) or a glyph that is checked on device (ANDR-08 UI pass, desktop-checkable).

### Pitfall 10: Landscape layout leaves a tiny canvas
In landscape the top bar, hint text, template bar and settings panel take most of the height; the collage canvas is ~360 px tall on a 1080 px-high screen [VERIFIED: emulator screenshot]. D-05 requires the UI to work in both orientations, so the plan needs a layout task (e.g. side panel in landscape, hint hidden). It can be checked on the desktop by resizing the window to a landscape aspect ratio.

### Pitfall 11: AboutSlint text barely readable
"Version 1.18.1 / https://slint.dev/" in `AboutSlint` renders dark-on-dark on the start screen (LICN-01 attribution still visible as a logo badge) [VERIFIED: screenshot]. It is cosmetic; it can be fixed in Phase 6 or with the UI pass.

### Pitfall 12: Non-JPEG/PNG picks (HEIC, WebP, AVIF)
The picker with `image/*` can return HEIC/WebP (common on Samsung). `image` is built with only `jpeg`, `png` [VERIFIED: Cargo.toml:15 `image = { version = "0.25", default-features = false, features = ["jpeg", "png"] }`]. Such files fail to decode and are reported by the existing "skipped" notice. Converting them in Kotlin (`ImageDecoder` → JPEG) is possible but costs memory/time. See Open Questions 4.

## Code Examples

### Rust: `crates/dimuli-app/src/platform/android.rs` (skeleton, compiled + run in the probe)
```rust
//! Android platform: JNI bridge to `ch.staeheli.dimuli.MainActivity`.
// The only module allowed to use `unsafe` (docs/CODING_GUIDELINES.md §3): JNI needs it.
#![allow(unsafe_code)]

use std::path::PathBuf;
use std::rc::Rc;

use jni::errors::LogErrorAndDefault;
use jni::objects::{JObject, JObjectArray, JString};
use jni::refs::Global;
use jni::sys::jboolean;
use jni::{Env, JValue, JavaVM, jni_sig, jni_str, native_method};
use slint::android::AndroidApp;

use super::{Platform, PlatformEvent, post_event};

/// Called by android-activity on its own `android_main` thread (may run more than once per process).
#[unsafe(no_mangle)] // SAFETY: unique symbol looked up by android-activity's NativeActivity glue.
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_tag("dimuli").with_max_level(log::LevelFilter::Info),
    );
    if let Err(error) = slint::android::init(app.clone()) {
        log::error!("cannot init slint: {error}");
        return;
    }
    let platform = AndroidPlatform { app };
    if let Err(error) = platform.register_natives() {
        log::error!("cannot register native methods: {error}");
    }
    // After slint::android::init: our back callback must be registered after Slint's own.
    platform.call_void(jni_str!("installBackCallback"));
    if let Err(error) = crate::run(Rc::new(platform)) {
        log::error!("UI failed: {error}");
    }
}

pub(crate) struct AndroidPlatform {
    app: AndroidApp,
}

impl AndroidPlatform {
    fn with_activity<R>(
        &self,
        f: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<R>,
    ) -> jni::errors::Result<R> {
        let vm = JavaVM::singleton()?;
        let raw = self.app.activity_as_ptr() as jni::sys::jobject;
        vm.attach_current_thread(|env| {
            // SAFETY: `raw` is android-activity's unowned global ref to the Activity; it stays valid
            // while `self.app` lives, and the borrowing cast never deletes it.
            let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw)? };
            f(env, activity.as_ref())
        })
    }

    fn register_natives(&self) -> jni::errors::Result<()> {
        self.with_activity(|env, activity| {
            let class = env.get_object_class(activity)?;
            // SAFETY: every NativeMethod comes from `native_method!`, which checks the Rust signature
            // against the Java signature at compile time.
            unsafe { env.register_native_methods(&class, &NATIVES) }
        })
    }
}

impl Platform for AndroidPlatform {
    fn pick_images(&self, max: usize) {
        let max = i32::try_from(max).unwrap_or(i32::MAX);
        let result = self.with_activity(|env, activity| {
            env.call_method(activity, jni_str!("pickImages"), jni_sig!((max: jint) -> void), &[JValue::Int(max)])?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("cannot open the photo picker: {error}");
            post_event(PlatformEvent::PickFailed(crate::texts::PICK_FAILED.to_owned())); // new text
        }
    }

    fn save_to_gallery(&self, jpeg: Vec<u8>) {
        let result = self.with_activity(|env, activity| {
            let bytes = env.byte_array_from_slice(&jpeg)?;
            env.call_method(activity, jni_str!("saveToGallery"), jni_sig!((jpeg: jbyte[]) -> void), &[JValue::Object(&bytes)])?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("cannot hand the collage to MediaStore: {error}");
            post_event(PlatformEvent::Saved(Err(crate::texts::SAVE_FAILED.to_owned())));
        }
    }
    // share(jpeg): same as save_to_gallery, method "share", error -> Shared(Err(SHARE_FAILED)).
    // close_app(): self.call_void(jni_str!("finish")).
}

impl AndroidPlatform {
    /// Calls a `() -> void` method on the Activity; failures are only logged.
    fn call_void(&self, name: &jni::strings::JNIStr) {
        let result = self.with_activity(|env, activity| {
            env.call_method(activity, name, jni_sig!(() -> void), &[])?;
            Ok(())
        });
        if let Err(error) = result {
            log::error!("JNI call failed: {error}");
        }
    }
}

// jni 0.22.4's macro expansion calls AtomicBool::fetch_update (deprecated since Rust 1.99).
#[allow(deprecated)]
const NATIVES: [jni::NativeMethod<'static>; 2] = [
    native_method! { fn native_images_picked(paths: java.lang.String[]), fn = native_images_picked, error_policy = LogErrorAndDefault },
    native_method! { fn native_saved(ok: jboolean), fn = native_saved, error_policy = LogErrorAndDefault },
    // + native_shared(ok: jboolean), native_back_pressed()
];

fn native_images_picked<'local>(
    env: &mut Env<'local>,
    _this: JObject<'local>,
    paths: JObjectArray<'local, JString<'local>>,
) -> jni::errors::Result<()> {
    let count = paths.len(env)?;
    let mut picked = Vec::with_capacity(count);
    for index in 0..count {
        picked.push(PathBuf::from(paths.get_element(env, index)?.try_to_string(env)?));
    }
    post_event(PlatformEvent::ImagesPicked(picked));
    Ok(())
}

fn native_saved<'local>(_env: &mut Env<'local>, _this: JObject<'local>, ok: jboolean) -> jni::errors::Result<()> {
    post_event(PlatformEvent::Saved(if ok { Ok(()) } else { Err(crate::texts::SAVE_FAILED.to_owned()) }));
    Ok(())
}
```
Java method names are derived from the snake_case names: `native_images_picked` → `nativeImagesPicked` [VERIFIED: native_method.md "Converted from snake_case to lowerCamelCase"]. `jboolean` is a Rust `bool` in jni-sys 0.4 [VERIFIED: compiled with `if ok`]. Keep the mapping helpers pure where possible (e.g. `fn saved_event(ok: bool) -> PlatformEvent`) in a non-cfg module so host tests cover them.

### Kotlin: `MainActivity.kt` (verified shape; add the native-call guard and SDK check)
```kotlin
package ch.staeheli.dimuli

class MainActivity : NativeActivity() {
    private val io = Executors.newSingleThreadExecutor()
    private var backCallback: OnBackInvokedCallback? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        if (Build.VERSION.SDK_INT < 35) enableEdgeToEdgeLegacy()   // D-07, API 33/34
        File(cacheDir, "picked").deleteRecursively()
    }

    @Suppress("DEPRECATION")
    private fun enableEdgeToEdgeLegacy() = window.setDecorFitsSystemWindows(false)

    override fun onDestroy() {
        backCallback?.let { onBackInvokedDispatcher.unregisterOnBackInvokedCallback(it) }
        io.shutdown()
        super.onDestroy()
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != REQUEST_PICK) return
        val uris = if (resultCode != RESULT_OK) emptyList() else buildList {
            data?.clipData?.let { clip -> for (i in 0 until clip.itemCount) add(clip.getItemAt(i).uri) }
            if (isEmpty()) data?.data?.let { add(it) }
        }
        io.execute { native { nativeImagesPicked(copyToCache(uris).toTypedArray()) } }
    }

    // region JNI bridge
    // Rust -> Kotlin (called on the android_main thread; must return immediately)
    /** Rust: `pickImages(max: jint) -> void` */
    fun pickImages(max: Int) = runOnUiThread {
        val intent = Intent(MediaStore.ACTION_PICK_IMAGES).setType("image/*")
        val limit = max.coerceAtMost(MediaStore.getPickImagesMaxLimit())
        if (limit > 1) intent.putExtra(MediaStore.EXTRA_PICK_IMAGES_MAX, limit)
        startActivityForResult(intent, REQUEST_PICK)
    }
    /** Rust: `saveToGallery(jpeg: jbyte[]) -> void`; answers nativeSaved(ok) */
    fun saveToGallery(jpeg: ByteArray) = io.execute { /* Pattern 5 */ }
    /** Rust: `share(jpeg: jbyte[]) -> void`; answers nativeShared(ok) */
    fun share(jpeg: ByteArray) = io.execute { /* Pattern 6 */ }
    /** Rust: `installBackCallback() -> void`, called after slint::android::init */
    fun installBackCallback() = runOnUiThread {
        if (backCallback != null) return@runOnUiThread
        val callback = OnBackInvokedCallback { native { nativeBackPressed() } }
        onBackInvokedDispatcher.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_DEFAULT, callback)
        backCallback = callback
    }
    // Kotlin -> Rust (registered by Rust via RegisterNatives)
    private external fun nativeImagesPicked(paths: Array<String>)
    private external fun nativeSaved(ok: Boolean)
    private external fun nativeShared(ok: Boolean)
    private external fun nativeBackPressed()

    /** Natives exist only after android_main registered them (Pitfall 3). */
    private inline fun native(call: () -> Unit) =
        try { call() } catch (e: UnsatisfiedLinkError) { Log.w(TAG, "native side not ready", e) }
    // endregion

    private companion object { const val TAG = "dimuli"; const val REQUEST_PICK = 1 }
}
```
`finish()` is called by Rust directly (`close_app`); it is a platform method and needs no Kotlin wrapper.

### Manifest + theme (verified)
```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application android:label="dimuli" android:icon="@mipmap/ic_launcher"
        android:theme="@style/Theme.Dimuli" android:enableOnBackInvokedCallback="true">
        <activity android:name=".MainActivity" android:exported="true"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|layoutDirection|locale">
            <meta-data android:name="android.app.lib_name" android:value="dimuli" />
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
        <provider android:name=".ShareProvider" android:authorities="ch.staeheli.dimuli.share"
            android:exported="false" android:grantUriPermissions="true" />
    </application>
</manifest>
```
```xml
<style name="Theme.Dimuli" parent="@android:style/Theme.Material.NoActionBar">
    <item name="android:windowBackground">@color/background</item>  <!-- #101114 -->
    <item name="android:statusBarColor">@android:color/transparent</item>
    <item name="android:navigationBarColor">@android:color/transparent</item>
    <item name="android:enforceStatusBarContrast">false</item>
    <item name="android:enforceNavigationBarContrast">false</item>
    <item name="android:windowLayoutInDisplayCutoutMode">always</item>
    <item name="android:windowLightStatusBar">false</item>
    <item name="android:windowLightNavigationBar">false</item>
</style>
```
The probe used `launchMode="singleTask"` without problems. The default `standard` launch mode is recommended, because the launcher brings the existing task to front anyway [ASSUMED].

### `android/app/build.gradle.kts` (verified)
```kotlin
plugins { id("com.android.application") }

android {
    namespace = "ch.staeheli.dimuli"
    compileSdk = 37
    buildToolsVersion = "37.0.0"
    ndkVersion = "30.0.16248370"
    defaultConfig {
        applicationId = "ch.staeheli.dimuli"
        minSdk = 33
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

// cargo ndk -> src/main/jniLibs before every build; -PrustAbis=x86_64 for emulator-only builds.
val rustAbis = (findProperty("rustAbis") as String? ?: "arm64-v8a,x86_64").split(",")
val sdkDir = androidComponents.sdkComponents.sdkDirectory
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs").asFile
val buildRust by tasks.registering(Exec::class) {
    group = "build"
    description = "Builds libdimuli.so with cargo ndk"
    workingDir = rootDir.parentFile                       // repository root
    doFirst { environment("ANDROID_JAR", sdkDir.get().file("platforms/android-37.0/android.jar").asFile.path) }
    commandLine(listOf("cargo", "ndk") + rustAbis.flatMap { listOf("-t", it) } +
        listOf("--platform", "33", "-o", jniLibsDir.path, "build", "-p", "dimuli-app", "--lib", "--release"))
}
tasks.named("preBuild") { dependsOn(buildRust) }
```
`settings.gradle.kts`: `pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }`, `dependencyResolutionManagement { repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS); repositories { google(); mavenCentral() } }`, `include(":app")`. No `local.properties` is needed when `ANDROID_HOME` is set [VERIFIED].

### Gradle wrapper without Android Studio (verified)
```sh
cd /tmp && curl -LO https://services.gradle.org/distributions/gradle-9.9.1-bin.zip
echo "d8a18a7a841dd5df825d35a685fad6f2d7f12ac0c280e57b67c27b4a520ba228  gradle-9.9.1-bin.zip" | sha256sum -c
unzip -q gradle-9.9.1-bin.zip -d /tmp/gradle-dist
cd <repo>/android && /tmp/gradle-dist/gradle-9.9.1/bin/gradle wrapper --gradle-version 9.9.1 \
    --distribution-type bin --gradle-distribution-sha256-sum d8a18a7a841dd5df825d35a685fad6f2d7f12ac0c280e57b67c27b4a520ba228
```
The cached Gradle 8.9 in `~/.gradle` cannot run on JDK 25. Commit `gradlew`, `gradlew.bat` and `gradle/wrapper/*`.

### `app.slint` safe-area binding (verified on device)
```slint
    Rectangle {
        x: max(Backend.inset-left, root.safe-area-insets.left);
        y: max(Backend.inset-top, root.safe-area-insets.top);
        width: parent.width - self.x - max(Backend.inset-right, root.safe-area-insets.right);
        height: parent.height - self.y - max(Backend.inset-bottom, root.safe-area-insets.bottom);
```

### AVD for API 33
```sh
sdkmanager "system-images;android-33;google_apis;x86_64"
avdmanager create avd -n dimuli_api33 -k "system-images;android-33;google_apis;x86_64" -d pixel_8
emulator -avd dimuli_api33
```
The package name exists in the SDK list (rev 17) [VERIFIED: `sdkmanager --list`]. `pixel_8` matches the existing AVDs [VERIFIED: `avdmanager list avd`]. `sdkmanager` prints a deprecation notice pointing to the new `android sdk` CLI but still works.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `org.jetbrains.kotlin.android` plugin | AGP built-in Kotlin | AGP 9.0 | Do not apply the Kotlin plugin; AGP 10 removes the opt-out |
| `cargo-apk` | `cargo ndk` + Gradle | (project decision) | `hasCode=true`, Kotlin Activity possible |
| Kotlin inset listener → JNI | Slint `Window.safe-area-insets` | Slint 1.x Android backend (present in 1.18.1) | No custom inset code |
| `KEYCODE_BACK` interception | `OnBackInvokedCallback` | Android 13; default-on predictive back for targetSdk 36 on Android 16 | Back must go through the dispatcher |
| jni 0.21 `JNIEnv`, `GlobalRef` | jni 0.22 `EnvUnowned`/`Env`, `Global<T>`, `native_method!`, `JavaVM::singleton()` | jni 0.22 (2026) | Older blog examples do not compile against 0.22 |
| 4 KB pages | 16 KB required for Play on 64-bit (updates blocked from 2027-02-01) | Android 15+ | Defaults already comply |

**Deprecated/outdated:** `Window.setDecorFitsSystemWindows`, `setStatusBarColor`, `setNavigationBarColor` (deprecated in API 37 stubs; only needed below API 35). `sdkmanager` (deprecated in favor of `android sdk`, still functional).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | On API 33 with `enableOnBackInvokedCallback="true"`, back reaches only our `OnBackInvokedCallback` (Slint registers none below 34) | Pattern 8 | Back does nothing or closes the app on Android 13 → verify on `dimuli_api33` |
| A2 | Edge-to-edge on API 33/34 via `setDecorFitsSystemWindows(false)` + transparent-bar theme makes the NativeActivity surface draw behind the bars, so Slint's WindowMetrics insets are not applied twice | Pattern 7 | Double top/bottom padding on Android 13/14 → verify on `dimuli_api33` + Android 14 device |
| A3 | Single selection returns via `data.data` instead of `clipData` | Pattern 4 | One-image pick ignored → handled by the fallback code anyway |
| A4 | Exported-symbol alternative needs `System.loadLibrary` because NativeActivity's own loading does not register the library for JNI lookup | Alternatives | Only relevant if the planner chooses the alternative |
| A5 | Default `launchMode` is sufficient | Manifest | Duplicate activity instances in rare deep-link cases |
| A6 | The arm64 `.so` *runs* like the x86_64 one on devices (it was built, not run: `cargo ndk -t arm64-v8a --platform 33 build --release` succeeded in 42 s, `LOAD … Align 0x4000`, RELRO equals its LOAD) | Stack | Device-only crash → found in the first device install |

## Open Questions

1. **ANDR-05 wording vs. Slint's built-in insets**
   - Known: the requirement says "insets are read in Kotlin, passed to Rust via JNI". Slint's own Java helper already reads them and passes them via JNI into `safe-area-insets` (verified).
   - Unclear: whether the owner accepts that no dimuli-specific Kotlin inset code exists.
   - Recommendation: use Slint's path. Let the planner record the requirement interpretation in the plan, and have the verifier check the behaviour (no content under the bars on API 33/34/36/37) rather than the mechanism.
2. **Dependency approvals** (blocker): `jni 0.22.4`, `android_logger 0.15.1`, Slint feature `backend-android-activity-06`, AGP 9.5.1, Gradle 9.9.1 wrapper, implicit `kotlin-stdlib 2.2.10`. Recommendation: one `checkpoint:human-verify` task at the start of the phase listing all of them together with the alternatives (own logger, Java activity).
3. **Share success message:** Android cannot know whether sharing was completed. Recommendation: post `Shared(Ok)` when the chooser opened, and either keep "Collage geteilt." or show no notice on success (owner/UX choice). Add a new error text `SHARE_FAILED`.
4. **HEIC/WebP picks** (Pitfall 12): skip with the existing notice in Phase 5 (no code), or convert in Kotlin? Recommendation: skip in Phase 5, revisit with PERF-01/ERRM-01 in Phase 6. Check what the owner's real Android 14/16 devices store (HEIC is common on Samsung).
5. **Real-device connection from WSL2:** USB via `usbipd-win` (installed on the Windows side per PATH) or Wi-Fi `adb pair`/`adb connect`. The owner's preference decides; the plan needs a manual step.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| JDK (`java`, `javac`) | Gradle, Slint build.rs | ✓ | OpenJDK 25.0.5 | — |
| Android SDK platform 37 | compileSdk, `ANDROID_JAR` | ✓ | `platforms/android-37.0` | — |
| Build-tools 37.0.0 (`zipalign`, `d8`) | AGP, 16 KB check | ✓ | 37.0.0 (not on PATH; use full path) | — |
| Build-tools 36.0.0 | AGP 9.5.1 default | ✓ (auto-installed by AGP during this research) | 36.0.0 | set `buildToolsVersion = "37.0.0"` |
| NDK r30 | cargo-ndk, AGP strip | ✓ | 30.0.16248370 | — |
| cargo-ndk | Rust build | ✓ | 4.1.2 | — |
| Rust targets `aarch64/x86_64-linux-android` | Rust build | ✓ | rustc 1.99.1 | — |
| Gradle CLI | wrapper generation | ✗ (no `gradle` on PATH; cached 8.9 cannot run on JDK 25) | — | download Gradle 9.9.1 zip (verified) |
| AGP/Kotlin artifacts | Gradle build | ✓ cached in `~/.gradle` by this research | AGP 9.5.1 | network |
| Network (first build) | Skia prebuilt download (skia-bindings 0.153.3), Maven | ✓ | — | none — first build needs network |
| Emulator + KVM | testing | ✓ | emulator 37.2.13, `/dev/kvm` group kvm | — |
| AVD `dimuli_api36`, `dimuli_api37` | testing | ✓ | pixel_8, google_apis x86_64 | — |
| AVD `dimuli_api33` | testing (D-08) | ✗ | — | create (commands above; image download ~GB, may need owner) |
| Real Android 14 / 16 devices | ANDR-01, ANDR-08 | owner-provided | — | none (manual) |
| ImageMagick `convert` | creating test images for the emulator | ✓ | — | ffmpeg |

**Missing dependencies with no fallback:** AVD `dimuli_api33` (must be created in this phase); real-device access (owner).
**Missing dependencies with fallback:** Gradle CLI (download the distribution once to generate the wrapper).

**Side effects of this research on the machine:** `build-tools;36.0.0` was installed into `~/Android/Sdk` by AGP; Gradle 9.9.1 and AGP 9.5.1 artifacts were cached in `~/.gradle`. The probe app was installed on `dimuli_api36` and then uninstalled, test images and the saved collage were deleted, and the emulator was stopped. Probe files live only in the session scratchpad.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), host only; Android checks via `adb` commands on emulators; real-device checks manual |
| Config file | none (workspace `Cargo.toml`, `clippy.toml` allows unwrap/expect/panic/indexing in tests) |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace && (cd android && ./gradlew assembleDebug) && $ANDROID_HOME/build-tools/37.0.0/zipalign -c -P 16 -v 4 android/app/build/outputs/apk/debug/app-debug.apk` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ANDR-01 | APK builds for both ABIs | build | `cd android && ./gradlew assembleDebug && unzip -l app/build/outputs/apk/debug/app-debug.apk \| grep -E 'lib/(arm64-v8a\|x86_64)/libdimuli.so'` | ❌ Wave 0 (android/ project) |
| ANDR-01 | Launch to start screen | emulator smoke | `adb install -r <apk> && adb shell am start -W -n ch.staeheli.dimuli/.MainActivity` → `Status: ok`; `adb logcat -d -s RustPanic AndroidRuntime:E` empty; screenshot `adb exec-out screencap -p` | ❌ Wave 0 (optional `android/scripts/smoke.sh`) |
| ANDR-01 | Real Android 14/16 devices | manual | same commands with `-s <serial>` | manual-only (hardware) |
| ANDR-02 | Paths → `ImagesPicked`, empty → cancel | unit | `cargo test -p dimuli-app picked` (pure mapping helper + existing controller logic) | ❌ Wave 0 |
| ANDR-02 | Picker → cache → editor | emulator | push images, `am force-stop com.google.android.photopicker`, tap via `uiautomator dump` bounds; `adb shell run-as ch.staeheli.dimuli ls cache/picked` | manual-assisted |
| ANDR-03 | Saved in `Pictures/dimuli`, not pending | emulator | `adb shell content query --uri content://media/external/images/media --projection _display_name:relative_path:is_pending:owner_package_name \| grep dimuli-` → `relative_path=Pictures/dimuli/, is_pending=0` | manual-assisted |
| ANDR-03/04 | Result → message mapping (ok/err texts) | unit | `cargo test -p dimuli-app saved` / `shared` | ❌ Wave 0 |
| ANDR-04 | Chooser opens | emulator | after tapping "Teilen": `adb shell dumpsys activity activities \| grep -m1 topResumedActivity` contains `intentresolver` | manual-assisted |
| ANDR-05 | No content under bars (13/14/16/17) | visual | screenshots on `dimuli_api33/36/37` + devices, portrait + landscape | manual-only (visual) |
| ANDR-06 | Back: start → app closes; editor → confirm dialog | emulator + unit | `adb shell input keyevent KEYCODE_BACK` then `topResumedActivity`; existing `state.rs` back tests: `cargo test -p dimuli-app back` | ✅ (state tests) / manual |
| ANDR-07 | 16 KB alignment | build | `zipalign -c -P 16 -v 4 app-debug.apk` → "Verification successful"; `llvm-readelf -Wl lib*.so \| grep LOAD` → `0x4000` | ❌ Wave 0 (script line) |
| ANDR-08 | Divider drag, pinch, pan, long-press swap, template bar, settings, busy overlay | manual on device | pinch needs two fingers (real device; emulator only via Ctrl+drag in its window — **never the mouse wheel**, Pitfall 5) | manual-only |
| ANDR-08 | Swap is not dropped when a second finger arrives (Pitfall 6) | unit | `cargo test -p dimuli-app pinch_cancels_swap` (if the fix is needed) | ❌ conditional |
| D-05 | Rotation keeps the collage | emulator | `adb shell settings put system accelerometer_rotation 0; adb shell settings put system user_rotation 1`; logcat shows no second `android_main` start line | manual-assisted |

### Sampling Rate
- **Per task commit:** `cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace` (host; `android.rs` is not compiled on the host). For tasks touching `android.rs` add `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`.
- **Per wave merge:** `cd android && ./gradlew assembleDebug` + `zipalign -c -P 16` + install/launch smoke on `dimuli_api36`.
- **Phase gate:** full emulator matrix (`dimuli_api33`, `dimuli_api36`, `dimuli_api37`) + Android 14 and Android 16 devices, all ANDR checks green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `android/` Gradle project + wrapper — prerequisite for every Android check.
- [ ] Pure, host-testable mapping helpers for native results (paths → `ImagesPicked`, ok/err → `Saved`/`Shared` with texts) plus tests in `dimuli-app`.
- [ ] Optional `android/scripts/smoke.sh` (install, launch, back-closes, zipalign, readelf) to make the emulator checks repeatable.
- [ ] AVD `dimuli_api33` created (owner may need to run the download).

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — (no accounts) |
| V3 Session Management | no | — |
| V4 Access Control | yes | `ShareProvider` `exported="false"` + `grantUriPermissions="true"`, temporary read grant via intent flag/ClipData only; serves exactly one fixed URI read-only |
| V5 Input Validation | yes | Paths from Kotlin are app-cache paths created by the app; decoder limits already in `collage-core` (`DECODE_MAX_PIXELS = 100_000_000` [VERIFIED: decode.rs:13], `DECODE_MAX_EDGE`, `DECODE_MAX_ALLOC`); reject non-`r` modes and unknown URIs in the provider |
| V6 Cryptography | no | — |
| V8 Data Protection | yes | Picked images copied to private `cacheDir` (not world-readable); no storage permissions; logs must not contain image content (URIs in error logs are acceptable for debugging, no file contents) |
| V12 Files and Resources | yes | MediaStore insert with `IS_PENDING`; no direct external-storage writes; provider has no path parameter → no path traversal |

### Known Threat Patterns for Android shell

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Exported provider leaking app files | Information disclosure | `exported=false`, single hard-coded file, URI equality check, read-only mode |
| Path traversal via provider URI (`../`) | Tampering / disclosure | Do not derive file paths from the URI; compare the whole URI to the one allowed value |
| Malicious/huge image from picker (decompression bomb) | DoS | Existing decode limits; copying is bounded by user selection (≤ 9 via `EXTRA_PICK_IMAGES_MAX`) |
| Panic unwinding across JNI → undefined behaviour / crash | DoS | `native_method!` `catch_unwind` + `LogErrorAndDefault`; android-activity catches panics in `android_main` |
| Implicit intent hijack on share | Disclosure | Chooser (`createChooser`) with only a temporary read grant on one URI |
| Calling into stale Activity after relaunch | Tampering / crash | Activity reference taken fresh from `AndroidApp` per call, never cached in statics |

## Sources

### Primary (HIGH confidence)
- Local crate sources (read this session): `i-slint-backend-android-activity-1.18.1` (`lib.rs`, `androidwindowadapter.rs`, `javahelper.rs`, `java/SlintAndroidJavaHelper.java`, `build.rs`), `slint-1.18.1` (`android.rs`, `Cargo.toml`), `i-slint-core-1.18.1` (`platform.rs`, `input.rs`, `context.rs`), `i-slint-compiler-1.18.1/builtin_elements.rs`, `android-activity-0.6.1` (`lib.rs`, `init.rs`, `native_activity/glue.rs`), `jni-0.22.4` (`docs/0.22-MIGRATION.md`, `docs/macros/native_method.md`, `src/env.rs`, `src/vm/java_vm.rs`), `android_logger-0.15.1`, `android-build-0.1.4`, `cargo-ndk-4.1.2`
- Android API 37 stubs (`platforms/android-37.0/android-stubs-src.jar`): `Activity`, `MediaStore`, `Window`, `OnBackInvokedDispatcher`
- End-to-end probe in the session scratchpad: Android build + clippy, Gradle build, emulator `dimuli_api36` run (launch, back, relaunch, picker, save, share, rotation), `zipalign`, `llvm-readelf`
- Repository files: `Cargo.toml`, `crates/dimuli-app/Cargo.toml`, `src/lib.rs`, `src/main.rs`, `src/controller.rs`, `src/platform/{mod,desktop}.rs`, `src/texts.rs`, `ui/{app,backend,collage-canvas,theme,editor-screen}.slint`, `collage-core/src/decode.rs`, `.gitignore`, `docs/*.md`, `CLAUDE.md`, `.planning/*`

### Secondary (MEDIUM confidence)
- https://developer.android.com/build/releases/agp-9-5-0-release-notes — AGP 9.5 compatibility (API 37, Gradle 9.6.0, JDK 17)
- https://developer.android.com/build/migrate-to-built-in-kotlin — built-in Kotlin
- https://docs.gradle.org/current/userguide/compatibility.html — Java 25 support since Gradle 9.1.0
- https://developer.android.com/reference/android/provider/MediaStore — `ACTION_PICK_IMAGES`, `EXTRA_PICK_IMAGES_MAX`
- https://developer.android.com/training/data-storage/shared/photo-picker — access duration, availability
- https://developer.android.com/guide/practices/page-sizes — 16 KB checks
- https://developer.android.com/guide/navigation/custom-back/predictive-back-gesture — callback stack order
- Registries: crates.io API (versions), dl.google.com Maven metadata (AGP), services.gradle.org (Gradle), repo1.maven.org (Kotlin)

### Tertiary (LOW confidence)
- none used for recommendations

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — exact combination built and run.
- Architecture: HIGH for API 36 (verified end-to-end); MEDIUM for API 33/34 specifics (A1, A2 untested, no API 33 image installed).
- Pitfalls: HIGH — each one was reproduced or read in source.

**Research date:** 2026-11-13
**Valid until:** 2026-12-13 (AGP/Gradle move monthly; Slint pinned to 1.18)
