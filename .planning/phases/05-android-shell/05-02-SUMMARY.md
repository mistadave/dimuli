---
phase: 05-android-shell
plan: 02
subsystem: android-shell
tags: [android, gradle, agp, kotlin, nativeactivity, jni, edge-to-edge, safe-area-insets, smoke-test]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 01
    provides: "libdimuli.so via cargo ndk, android_main, native_back_pressed, AndroidPlatform (close_app -> finish), Slint Window.safe-area-insets"
provides:
  - "android/ Gradle project (AGP 9.5.1 built-in Kotlin, wrapper 9.9.1 pinned by sha256, no dependencies block)"
  - "Gradle task buildRust (cargo ndk --platform 33 --release, ANDROID_JAR = platforms/android-37.0) wired before preBuild; property rustAbis"
  - "debug APK with lib/arm64-v8a/libdimuli.so and lib/x86_64/libdimuli.so"
  - "MainActivity : NativeActivity with installBackCallback(), nativeBackPressed(), native { } guard, legacy edge-to-edge on API < 35"
  - "Manifest: label dimuli, enableOnBackInvokedCallback, configChanges (rotation keeps native app), no permissions, no orientation lock"
  - "Theme.Dimuli: background #101114, transparent bars, cutout mode always"
  - "android/scripts/smoke.sh [-s SERIAL] [APK]: install / launch / back-closes / no-panic smoke test"
  - "Tracer proven end to end on dimuli_api36: launch, back on start screen closes app, insets portrait + landscape, rotation without restart"
affects: [05-03, 05-04, 05-05, 05-06, 05-07, 05-08]

# Actuals (#2632) - chars/4 over the hand-written files (generated gradlew/gradlew.bat/wrapper jar excluded)
actuals:
  tokens: 2200
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: ["Android Gradle Plugin 9.5.1 (built-in Kotlin, implicit kotlin-stdlib)", "Gradle wrapper 9.9.1 (distributionSha256Sum pinned)"]
  patterns:
    - "Rust build driven from Gradle: Exec task buildRust deletes src/main/jniLibs first, then cargo ndk per ABI into it"
    - "Kotlin -> Rust calls always wrapped in native { } (catches UnsatisfiedLinkError, Pitfall 3)"
    - "Insets only through Slint Window.safe-area-insets; Kotlin side only enables edge-to-edge (API 33/34) and sets the theme"
    - "Device smoke test as a shell script per adb serial, ending with back-closes-app and a RustPanic/AndroidRuntime check"

key-files:
  created:
    - android/settings.gradle.kts
    - android/build.gradle.kts
    - android/gradle.properties
    - android/gradlew
    - android/gradlew.bat
    - android/gradle/wrapper/gradle-wrapper.jar
    - android/gradle/wrapper/gradle-wrapper.properties
    - android/app/build.gradle.kts
    - android/app/src/main/AndroidManifest.xml
    - android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt
    - android/app/src/main/res/values/themes.xml
    - android/app/src/main/res/values/colors.xml
    - android/scripts/smoke.sh
  modified:
    - .gitignore

key-decisions:
  - "No android:icon in the manifest: no icon resource exists and the plan does not name one; the launcher shows the default icon until a later plan adds one"
  - "tasks.register<Exec>(\"buildRust\") instead of `by tasks.registering(Exec::class)`: same behaviour, avoids a Gradle 9.6+ deprecation"
  - "Rotation settings restored to the emulator's original values (accelerometer_rotation 1, user_rotation 0), not only user_rotation 0 as the plan names"

patterns-established:
  - "Device evidence lives under android/build/smoke/ (gitignored): <serial>-start.png from smoke.sh, api36-start-{portrait,landscape}.png, api36-rotation-logcat.txt"

requirements-completed: []  # Plan declares [ANDR-01, ANDR-05, ANDR-06]; all three are shared with later Phase-5 plans (05-03..05-08: picker, save/share, api33/api37 runs, back in editor), so none is marked complete here.

coverage:
  - id: D1
    description: "Debug APK with both ABIs builds through one Gradle command"
    requirement: ANDR-01
    verification:
      - kind: other
        ref: "android/gradlew -p android assembleDebug -> BUILD SUCCESSFUL; unzip -l app-debug.apk | grep -cE 'lib/(arm64-v8a|x86_64)/libdimuli.so' == 2"
        status: pass
    human_judgment: false
  - id: D2
    description: "APK installs and launches to the start screen on dimuli_api36; back on the start screen closes the app via Kotlin -> Rust -> controller -> finish()"
    requirement: ANDR-06
    verification:
      - kind: e2e
        ref: "android/scripts/smoke.sh -s emulator-5554 -> SMOKE OK emulator-5554 (run 3x in this pass)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Start screen inset by Slint safe area in portrait and landscape, background behind bars"
    requirement: ANDR-05
    verification:
      - kind: other
        ref: "android/build/smoke/api36-start-portrait.png, android/build/smoke/api36-start-landscape.png (inspected by executor)"
        status: pass
    human_judgment: true
    rationale: "Visual layout against system bars; executor inspected both screenshots, owner confirmation still listed under Open Human Checks"
  - id: D4
    description: "Rotation does not restart the native app (configChanges)"
    requirement: ANDR-05
    verification:
      - kind: other
        ref: "grep -c 'dimuli started' android/build/smoke/api36-rotation-logcat.txt == 1; same pid 4428 before and after portrait -> landscape -> portrait"
        status: pass
    human_judgment: false

# Metrics
duration: 2min (verification pass only; implementation and first Gradle build by rust-coder not timed)
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 02: Android Shell Tracer (Gradle + Kotlin) Summary

**dimuli now runs as a real Android app. A single `android/gradlew -p android assembleDebug` builds a debug APK with arm64-v8a and x86_64 `libdimuli.so` through `cargo ndk`. The app launches on dimuli_api36 to a start screen that respects the system bars and the camera cutout. Rotating keeps the native app alive. System back on the start screen closes the app over Kotlin `OnBackInvokedCallback` -> Rust `nativeBackPressed` -> controller -> `finish()`.**

## Performance

- **Duration:** about 2 min for this verification pass (2026-11-13T16:24:12Z to 16:25:54Z); the files and the first full Gradle build were done by rust-coder beforehand and not timed here
- **First full Gradle build:** not measured in this pass (rust-coder ran it, including the fresh arm64 release build). Re-run here: `BUILD SUCCESSFUL in 1s`, 1.3 s wall clock (Rust artefacts cached; `buildRust` still executes because it deletes and refills `jniLibs`)
- **Tasks:** 2 of 2
- **Files:** 13 created + 1 modified (`.gitignore`)

## Accomplishments

- Gradle project under `android/`: AGP 9.5.1 with built-in Kotlin, wrapper 9.9.1 pinned by `distributionSha256Sum`, no `dependencies` block, no AndroidX
- `buildRust` Exec task builds `libdimuli.so` per ABI with `cargo ndk --platform 33 ... --release` (`ANDROID_JAR` = `platforms/android-37.0/android.jar`), after first deleting `src/main/jniLibs`; `preBuild` depends on it
- `MainActivity : NativeActivity`: legacy edge-to-edge on API < 35, back callback bridge (`installBackCallback` / `nativeBackPressed` / `native { }` guard), callback unregistered in `onDestroy`
- Manifest with `enableOnBackInvokedCallback`, full `configChanges`, `android.app.lib_name = dimuli`, no permissions, no orientation lock; dark theme `Theme.Dimuli` with transparent bars and cutout mode `always`
- `android/scripts/smoke.sh`: repeatable install / launch / back-closes / no-panic test per adb serial

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request, so all changes stay in the working tree (HEAD unchanged at `262306a`).

Suggested commit message when the owner asks to commit (together with or after 05-01):

```
Add Android Gradle shell and prove the tracer on the emulator (phase 5, plan 02)

- android/ Gradle project: AGP 9.5.1 with built-in Kotlin, wrapper
  9.9.1 pinned by sha256, no dependencies (D-09)
- buildRust task builds libdimuli.so with cargo ndk --platform 33
  --release for arm64-v8a and x86_64 before preBuild
- MainActivity (NativeActivity): edge-to-edge on API 33/34 and the
  back callback bridge to Rust (installBackCallback / nativeBackPressed)
- Manifest: label dimuli, predictive back opt-in, configChanges for
  rotation, no permissions; dark theme with transparent system bars
- android/scripts/smoke.sh: install, launch, back closes the app
```

## Files Created/Modified

- `android/settings.gradle.kts` (new): plugin/dependency repositories, `FAIL_ON_PROJECT_REPOS`, `include(":app")`
- `android/build.gradle.kts` (new): AGP 9.5.1 `apply false`
- `android/gradle.properties` (new): JVM args
- `android/gradlew`, `android/gradlew.bat`, `android/gradle/wrapper/gradle-wrapper.jar`, `android/gradle/wrapper/gradle-wrapper.properties` (new, generated): wrapper 9.9.1 with `distributionSha256Sum`
- `android/app/build.gradle.kts` (new): namespace/applicationId `ch.staeheli.dimuli`, SDK 33/37, NDK 30.0.16248370, ABI filters, Java 17, `buildRust`
- `android/app/src/main/AndroidManifest.xml` (new)
- `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt` (new)
- `android/app/src/main/res/values/themes.xml`, `android/app/src/main/res/values/colors.xml` (new)
- `android/scripts/smoke.sh` (new, executable)
- `.gitignore`: `/android/.kotlin/` added
- `crates/dimuli-app/src/platform/android.rs`: not touched in this plan (no Rust-side defect)
- Evidence (gitignored, `/android/build/`): `android/build/smoke/api36-start-portrait.png`, `android/build/smoke/api36-start-landscape.png`, `android/build/smoke/api36-rotation-logcat.txt`, `android/build/smoke/emulator-5554-start.png`

Gitignore coverage checked with `git check-ignore`: `android/.gradle`, `android/.kotlin`, `android/build`, `android/app/build`, `android/app/src/main/jniLibs`, `android/local.properties` are all ignored.

## Gate Results (re-run in this pass)

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS (86 + 92 passed, 0 failed) |
| Android clippy x86_64 (`cargo ndk ... clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local`) | PASS (exit 0) |
| `android/gradlew -p android assembleDebug` | PASS (`BUILD SUCCESSFUL`). The only Gradle deprecation (`--warning-mode all`) is `Configuration.setVisible(boolean)`, which comes from AGP, not from the project scripts |
| ABIs in APK | PASS: 2 (`lib/arm64-v8a/libdimuli.so` 15.2 MB, `lib/x86_64/libdimuli.so` 16.2 MB) |
| `android/scripts/smoke.sh -s emulator-5554` | PASS: `SMOKE OK emulator-5554` (3 runs: Task 1 gate, Task 2 step 1, Task 2 verify) |
| Task 2 evidence (`test -s` both PNGs, exactly 1 `dimuli started` in rotation log) | PASS (exit 0) |
| `settings get system user_rotation` after task | PASS: `0` |

Task 1 acceptance greps, each run separately: `/android/.kotlin/` 1; `applicationId = "ch.staeheli.dimuli"` 1; SDK/NDK settings 5; `android-37.0/android.jar` 1; dependency lines 0; AGP `9.5.1` 1; wrapper sha256 1; `gradlew` executable + wrapper jar present (exit 0); manifest attributes 4; `uses-permission` 0; `screenOrientation` 0; Kotlin literals 7; `#101114` 1; smoke.sh executable (exit 0) + `SMOKE OK` 1; ABIs 2. All PASS.
Extra checks: no `<permission`/`uses-permission` element at all (0); no `dependencies` word in app/build.gradle.kts (0); `dependsOn(buildRust)` 1; `android:value="dimuli"` 1; `installBackCallback` in android.rs 1. The only "scroll" in smoke.sh is the comment saying it never sends scroll input (Pitfall 5).

## Task 2 Results (dimuli_api36, emulator-5554, 1080x2400, gesture navigation)

| Check | Evidence | Result |
|-------|----------|--------|
| Relaunch after smoke | `am start -W`: `Status: ok`, LaunchState WARM, TotalTime 162 ms | PASS |
| Portrait insets | `android/build/smoke/api36-start-portrait.png`: title "dimuli", hint, "Bilder wählen" button and Slint badge sit in the middle of the screen, well below the status bar (about 0-130 px) and above the gesture handle; side margins equal (about 63 px) | PASS |
| Portrait background | Background `#101114` fills the whole screen including behind the transparent status bar and gesture bar, with no white or black bands | PASS |
| Landscape insets / cutout | `android/build/smoke/api36-start-landscape.png` (ROTATION_90, 2400x1080): content is shifted right. Left margin is about 195 px (the camera cutout is on the left side in this rotation), right margin about 63 px. Title sits below the status bar and the badge ends above the gesture handle, so nothing is under the cutout or the bars | PASS |
| Landscape background | App background extends behind the status bar, the cutout strip on the left and the gesture bar | PASS |
| Rotation keeps native app | `android/build/smoke/api36-rotation-logcat.txt`: exactly 1 `dimuli started`; pid 4428 unchanged through portrait -> landscape -> portrait; still 1 `dimuli started` after rotating back | PASS |
| No crash | `logcat -d -s RustPanic:* AndroidRuntime:E`: 0 entries after rotation and rotate-back | PASS |
| Back after rotation | `KEYCODE_BACK` in portrait after the round trip: `topResumedActivity` = NexusLauncherActivity | PASS |
| Rotation restored | `user_rotation` 0, `accelerometer_rotation` 1 (the emulator's original values) | PASS |

Step 4 (fix through rust-coder) was not needed. No change was made to themes.xml or MainActivity.kt.

## Decisions Made

- I restored both rotation settings to the values they had before the task (`accelerometer_rotation` was 1). The plan only names `user_rotation 0`. Leaving auto-rotate off would change the emulator for later plans.

## Deviations from Plan

These were reported by rust-coder for Task 1. I checked both; neither changes behaviour.

**1. [Rule 3 - Blocking] No `android:icon` in the manifest**
- **Issue:** No icon resource exists and the plan names none. Referencing one would fail the resource link.
- **Effect:** The launcher and the splash window use the system default icon. A later plan (store/branding) adds a real icon.
- **Files:** android/app/src/main/AndroidManifest.xml

**2. [Rule 3 - Blocking] `tasks.register<Exec>("buildRust")` instead of `registering(Exec::class)`**
- **Issue:** Gradle 9.6+ deprecates the delegated `registering` form.
- **Fix:** Lazy `tasks.register<Exec>` with identical configuration. `dependsOn(buildRust)` still resolves (acceptance key-link grep 1).
- **Files:** android/app/build.gradle.kts

**Total deviations:** 2 (both blocking-avoidance, no behaviour change). No scope creep.

## Issues Encountered

- **The smoke screenshot shows the splash window, not the start screen.** smoke.sh takes `android/build/smoke/<serial>-start.png` right after `dimuli started` appears in the log. That is before Slint's first frame, so the image shows only the system splash (dark radial gradient, no icon). The script asserts nothing about the image, so the gate is unaffected. The Task 2 screenshots (taken 2-3 s after launch) show the real start screen. Suggested follow-up for rust-coder (05-06 or later): wait 1-2 s, or wait for the first Slint frame, before `screencap` in smoke.sh.
- **The Slint badge's "Version 1.18.1 / https://slint.dev/" text is dark on dark** and barely readable on `#101114`. This is Slint's built-in `AboutSlint` styling, not an inset problem. Note for UI polish.
- **No ANR dialog this time.** The "System UI isn't responding" ANR that rust-coder saw under build load did not appear in this pass.
- **zsh gotcha:** an unquoted `$S` holding `adb -s emulator-5554` is not word-split in zsh. Use literal commands in docs and scripts (smoke.sh is bash and unaffected).

## Open Human Checks

- **Owner visual confirmation (plan `<human-check>`):** open `android/build/smoke/api36-start-portrait.png` and `android/build/smoke/api36-start-landscape.png`. Expected: no text, button or badge under the status bar, navigation bar or camera cutout; app background behind the bars. The executor inspected both and found them correct (see Task 2 results).
- **Code review** (`code-reviewer` subagent, CLAUDE.md): not run yet for 05-01 + 05-02. It is due before committing the new functionality.
- **Real-device check of the predictive back animation:** not covered here. The emulator `KEYCODE_BACK` triggers the callback but not the swipe preview.

## User Setup Required

None.

## Next Phase Readiness

- Ready for 05-03 (photo picker). The JNI bridge pattern (Rust `call_void` -> Kotlin method, Kotlin `native { external }` -> Rust `post_event`) is proven end to end, and smoke.sh can be extended per slice.
- Fast x86_64-only rebuild for emulator iteration: `android/gradlew -p android assembleDebug -PrustAbis=x86_64`.
- Emulator dimuli_api36 is left running as emulator-5554 with the app closed.

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED

All 13 created files and 3 evidence files exist; 0 commits since 262306a (as required, no commits).
