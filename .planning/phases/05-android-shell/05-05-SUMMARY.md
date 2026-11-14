---
phase: 05-android-shell
plan: 05
subsystem: ui
tags: [slint, android, landscape, flickable, path, adaptive-icon, launcher, emulator]

# Dependency graph
requires:
  - phase: 05-android-shell
    plan: 04
    provides: "Save/share slice on dimuli_api36; banner and chevron tofu findings handed to this UI pass"
provides:
  - "EditorScreen.landscape: canvas beside a 300 px side panel (Flickable column with TemplateBar, SettingsToggle, SettingsPanel); portrait layout unchanged; settings-open survives rotation"
  - "Non-exported SettingsToggle component with a drawn Path chevron (up closed, down open) used by both orientations"
  - "MessageBanner close button as drawn Path cross; banner placed below the top bar"
  - "AppWindow init sets Palette.color-scheme = ColorScheme.dark (readable AboutSlint attribution)"
  - "Adaptive launcher icon (background, foreground, monochrome) and android:icon on <application>"
  - "Theme tokens side-panel-width (300px) and chevron-size (12px)"
affects: [05-06, 05-07, 05-08]

# Actuals (#2632) - chars/4 over the lines this plan added (Slint diff lines + 3 icon XML files + manifest attribute)
actuals:
  tokens: 1450
  tasks: 2
  commits: 0
plan_head_before: 262306a50e492d830a8bbfa9307fd0ece188cb50
plan_head_after: 262306a50e492d830a8bbfa9307fd0ece188cb50

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Orientation switch in Slint: `property <bool> landscape: root.width > root.height` with two `if` branches; presentation state kept as a root property so it survives rotation"
    - "Glyph-free icons: small Path elements (viewbox 12x10 / 12x12, stroke Theme.on-surface, Theme.border-medium) instead of Unicode arrows/crosses the Android font lacks"
    - "Scrollable side column: Flickable with content-width: self.width and content-height: column.preferred-height (viewport-height is deprecated in Slint 1.18)"

key-files:
  created:
    - android/app/src/main/res/mipmap-anydpi/ic_launcher.xml
    - android/app/src/main/res/drawable/ic_launcher_foreground.xml
    - android/app/src/main/res/drawable/ic_launcher_background.xml
  modified:
    - crates/dimuli-app/ui/editor-screen.slint
    - crates/dimuli-app/ui/theme.slint
    - crates/dimuli-app/ui/app.slint
    - crates/dimuli-app/ui/message-banner.slint
    - crates/dimuli-app/ui/texts.slint
    - android/app/src/main/AndroidManifest.xml

key-decisions:
  - "05-05: Settings chevron is one Path whose commands switch on `open` instead of two Paths with visibility; the `Path >= 2` acceptance grep gives 1, accepted as deviation (same visual result, no duplicated element)"
  - "05-05: Landscape body positioned with explicit y/height instead of a VerticalLayout (the layout gave the body only its minimum height); the editor hint is dropped from the Android text ('Mausrad' removed)"
  - "05-05: Message banner sits below the top bar (y = touch-target + 3 * spacing-s) so Zurück/Speichern/Teilen stay reachable while a message is shown"

patterns-established:
  - "Landscape check on the emulator: user_rotation 1 with accelerometer_rotation 0, then restore 0/1; pid before/after plus a `dimuli started` count prove no native restart"

requirements-completed: []  # Plan declares [ANDR-08, ANDR-01]; both are listed again by 05-06/05-07/05-08 (API matrix, real devices, milestone close), so neither is marked complete here (same handling as 05-03/05-04).

coverage:
  - id: D1
    description: "Landscape editor: canvas left at nearly full body height, side panel right with template bar and settings toggle, expanded settings scroll inside the panel, no hint line; nothing under bars or cutout"
    requirement: ANDR-08
    verification:
      - kind: e2e
        ref: "android/build/smoke/api36-editor-landscape.png, api36-settings-landscape.png (rust-coder 20:03 local, final build 20:02:52); inspected by executor"
        status: pass
    human_judgment: true
    rationale: "Layout quality is visual; scrolling inside the panel was not driven (no scroll input allowed), only seen as cut-off content"
  - id: D2
    description: "Portrait editor unchanged (top bar, canvas, hint, template bar, toggle, settings panel)"
    requirement: ANDR-08
    verification:
      - kind: e2e
        ref: "android/build/smoke/api36-editor-portrait.png, api36-chevron.png"
        status: pass
    human_judgment: true
    rationale: "Visual comparison"
  - id: D3
    description: "Rotation keeps the native app alive"
    requirement: ANDR-08
    verification:
      - kind: e2e
        ref: "api36-editor-rotation-logcat.txt: 1 x 'dimuli started' (editor with 3 images, rust-coder); executor re-run on start screen 20:07 local: pid 8214 before/landscape/after, 1 x 'dimuli started' after logcat -c, crash filter empty"
        status: pass
    human_judgment: false
  - id: D4
    description: "Drawn settings chevron (up closed, down open) and drawn banner cross, no tofu"
    verification:
      - kind: other
        ref: "grep '▾\\|▴' editor-screen.slint = 0, no ▾/▴/✕ left under ui/; api36-chevron.png, api36-editor-landscape.png (up), api36-settings-landscape.png (down), api36-banner.png (cross)"
        status: pass
    human_judgment: true
    rationale: "Glyph rendering is visual"
  - id: D5
    description: "Slint attribution readable on the dark start screen"
    verification:
      - kind: e2e
        ref: "api36-start-attribution.png (portrait), executor screenshot scratchpad exec-start-landscape.png (landscape, final build): white text, white #MadeWithSlint pill"
        status: pass
    human_judgment: true
    rationale: "Contrast is visual"
  - id: D6
    description: "Launcher shows label 'dimuli' with the adaptive collage-grid icon"
    requirement: ANDR-01
    verification:
      - kind: other
        ref: "aapt2 dump badging: application-label:'dimuli', icon='res/mipmap-anydpi-v21/ic_launcher.xml' (gate exit 0)"
        status: pass
      - kind: e2e
        ref: "android/build/smoke/api36-launcher.png"
        status: pass
    human_judgment: true
    rationale: "Icon appearance is visual; placeholder until the owner's final icon"

# Metrics
duration: 8min (verification pass; implementation by rust-coder not timed)
completed: 2026-11-13
status: complete
---

# Phase 5 Plan 05: Android UI Pass Summary

**The editor now has a landscape layout (canvas on the left at full body height, 300 px side panel on the right with template bar, settings toggle and the settings panel in a scrollable Flickable, no hint line), the settings toggle and the banner close button use drawn Path icons instead of glyphs that render as tofu on Android, the Slint attribution is readable through the dark std-widget colour scheme, and the launcher shows "dimuli" with an adaptive collage-grid vector icon. Verified on dimuli_api36; rotation keeps the native app alive.**

## Performance

- **Duration:** about 8 min for this verification pass (2026-11-13T19:05Z to 19:13Z): all gates and acceptance greps re-run, screenshots inspected, one rotation re-run. Implementation and rust-coder's emulator runs were done before and are not timed.
- **Tasks:** 2 of 2
- **Files:** 3 created, 6 modified

## Accomplishments

- `editor-screen.slint`: `property <bool> landscape: root.width > root.height`; portrait branch = previous VerticalLayout (toggle replaced by `SettingsToggle`); landscape branch = `TopBar` at y 0, below it a HorizontalLayout with `CollageCanvas { horizontal-stretch: 1 }` and a `Theme.side-panel-width` column (`Theme.surface`, `clip: true`) holding a `Flickable` (`content-width: self.width`, `content-height: column.preferred-height`) with `TemplateBar`, `SettingsToggle`, `if root.settings-open: SettingsPanel`. `settings-open` stays a root property.
- `SettingsToggle` (non-exported, same file): `in-out property <bool> open`, AppButton with `Texts.settings` and one `Path` (`Theme.chevron-size`, inset `Theme.spacing-m`, vertically centred, stroke `Theme.on-surface` / `Theme.border-medium`, round caps/joins); commands `M 0 8 L 6 2 L 12 8` (up, closed) / `M 0 2 L 6 8 L 12 2` (down, open).
- `theme.slint`: `side-panel-width: 300px`, `chevron-size: 12px`.
- `app.slint`: imports `Palette`, `init => { Palette.color-scheme = ColorScheme.dark; }`; MessageBanner `y: Theme.touch-target + 3 * Theme.spacing-s` (below the top bar).
- `message-banner.slint`: "✕" text replaced by a drawn Path cross (12x12, same stroke tokens).
- `texts.slint`: editor hint without "Mausrad" ("Ziehen: Bild verschieben · Zwei Finger: zoomen · Lange drücken: tauschen").
- Launcher: `mipmap-anydpi/ic_launcher.xml` (`<adaptive-icon>` with background, foreground, monochrome), `drawable/ic_launcher_background.xml` (108x108 `#101114`), `drawable/ic_launcher_foreground.xml` (tall `#7aa7ff` cell x 24..52, two `#f1f2f5` cells x 56..84, corner radius 4); manifest `<application android:icon="@mipmap/ic_launcher">`, label `dimuli` unchanged.

## Task Commits

No commits. CLAUDE.md and the plan forbid commits without an explicit owner request; all changes stay in the working tree (HEAD unchanged at `262306a`, 0 commits since).

Suggested commit message when the owner asks to commit (together with or after 05-01..05-04):

```
Android UI pass: landscape editor, drawn icons, launcher icon (phase 5, plan 05)

- Landscape editor: canvas beside a scrollable side panel with the
  template bar and settings; portrait layout unchanged
- Settings chevron and banner close button are drawn paths (the
  Android system font has no glyphs for the arrow and cross characters)
- Message banner sits below the top bar
- Dark std-widget colour scheme so the Slint attribution is readable
- Adaptive collage-grid launcher icon (background, foreground,
  monochrome) as a placeholder for the final icon
- Editor hint no longer mentions the mouse wheel
```

## Files Created/Modified

- `crates/dimuli-app/ui/editor-screen.slint`: landscape branch, `SettingsToggle` with Path chevron
- `crates/dimuli-app/ui/theme.slint`: `side-panel-width`, `chevron-size`
- `crates/dimuli-app/ui/app.slint`: `Palette` import, `init` dark scheme, banner y
- `crates/dimuli-app/ui/message-banner.slint`: Path cross (not in the plan's file list, see Deviations)
- `crates/dimuli-app/ui/texts.slint`: hint text (not in the plan's file list, see Deviations)
- `android/app/src/main/AndroidManifest.xml`: `android:icon="@mipmap/ic_launcher"`
- `android/app/src/main/res/mipmap-anydpi/ic_launcher.xml` (new)
- `android/app/src/main/res/drawable/ic_launcher_background.xml` (new)
- `android/app/src/main/res/drawable/ic_launcher_foreground.xml` (new)
- Evidence (gitignored, `android/build/smoke/`): `api36-start-attribution.png`, `api36-editor-portrait.png`, `api36-chevron.png`, `api36-banner.png`, `api36-editor-landscape.png`, `api36-settings-landscape.png`, `api36-launcher.png`, `api36-editor-rotation-logcat.txt` (rust-coder). Executor screenshot: `/tmp/claude-1000/-home-dstaehel-Documents-dev-dave-dimuli/89528c34-c79f-44fe-95a9-c2bb398036b2/scratchpad/exec-start-landscape.png` (session scratchpad).

Note: `git diff HEAD -- crates/dimuli-app/ui/app.slint` also shows the safe-area inset change from 05-01 (uncommitted); only the `Palette` import, the `init` block and the banner `y` belong to this plan.

## Gate Results (all re-run in this pass)

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | PASS (exit 0) |
| `cargo clippy --all-targets -- -D warnings` | PASS (exit 0) |
| `cargo test --workspace` | PASS (86 + 104 passed, 0 failed; unchanged, plan has no Rust changes) |
| `test -s` pick-a/b/c.jpg | PASS |
| `cargo build -p dimuli-app` + `timeout 15 target/debug/dimuli pick-a pick-b pick-c` (Task 1) | PASS, exit 124 |
| `timeout 15 target/debug/dimuli pick-a pick-b` (Task 2) | PASS, exit 124 |
| `android/gradlew -p android assembleDebug -PrustAbis=x86_64` | PASS (`BUILD SUCCESSFUL`; native lib 20:02:52 newer than the last Slint edit 20:02:32, APK 20:06:00) |
| `android/scripts/smoke.sh -s emulator-5554` | PASS (`SMOKE OK emulator-5554`) |
| T1 artefact gate (both landscape PNGs non-empty, exactly 1 `dimuli started`) | PASS (exit 0) |
| T2 badging gate (`application-label:'dimuli'` + `icon='res/mipmap-anydpi-v21/ic_launcher.xml'`, count >= 2) | PASS (exit 0); AAPT2 qualifies the folder as `mipmap-anydpi-v21`, the plan's regex allows this |
| Android clippy (cargo ndk) | Not re-run: no Rust source changed in this plan (rust-coder reported PASS) |
| Full both-ABI build | Not re-run (rust-coder reported PASS; `target/aarch64-linux-android/release/libdimuli.so` 20:04:19 is newer than the last Slint edit) |

Acceptance greps, each run separately:

| Criterion | Expected | Got |
|-----------|----------|-----|
| `landscape` in editor-screen.slint | >= 2 | 3 |
| `Theme.side-panel-width` in editor-screen.slint | >= 1 | 1 |
| `side-panel-width` in theme.slint | 1 | 1 |
| `Flickable` in editor-screen.slint | >= 1 | 1 |
| pick images non-empty + desktop 15 s run | exit 0 / 124 | 0 / 124 |
| `dimuli started` in api36-editor-rotation-logcat.txt | 1 | 1 |
| `▾\|▴` in editor-screen.slint | 0 | 0 |
| `chevron-size` in theme.slint | 1 | 1 |
| `Path` in editor-screen.slint | >= 2 | **1** (see Deviation 1) |
| `ColorScheme.dark` in app.slint | 1 | 1 |
| adaptive-icon tags (sort -u) | 4 | 4 |
| `android:icon="@mipmap/ic_launcher"` in manifest | 1 | 1 |
| `#101114` in ic_launcher_background.xml | 1 | 1 |
| aapt2 badging label + mipmap-anydpi icon | both | both |

Extra: key link `Theme.side-panel-width` present; `Theme.chevron-size` used 2x in editor-screen.slint; no `▾`, `▴` or `✕` left anywhere under `crates/dimuli-app/ui/`.

## Emulator Results (dimuli_api36, emulator-5554, 1080x2400, x86_64 build)

| Check | Evidence | Source | Result |
|-------|----------|--------|--------|
| Landscape editor layout | `android/build/smoke/api36-editor-landscape.png`: content starts at x 132 (cutout inset), top bar y 74..242, canvas body 242..1016 = 774 of 1080 px screen height (the full body between top bar and gesture-bar inset); the 1:1 collage fills that height; side panel x 1612..2400 with template bar and "Einstellungen" + up chevron; no hint line | rust-coder 20:03, inspected | PASS |
| Landscape settings open | `android/build/smoke/api36-settings-landscape.png`: down chevron, Format row, Abstand slider, "Rand" cut at the panel bottom (content continues, scrollable); canvas unchanged | rust-coder 20:03, inspected | PASS (scroll not driven, see human checks) |
| Portrait unchanged | `android/build/smoke/api36-editor-portrait.png` (after rotating back): top bar, canvas, hint, template bar, toggle with up chevron | rust-coder 20:03, inspected | PASS |
| Nothing under bars/cutout | All three: status bar, cutout side and gesture bar are free of app content | inspected | PASS |
| Rotation without native restart (editor) | `api36-editor-rotation-logcat.txt`: `dimuli started` 20:02:56, `picked images: 3` 20:03:07, nothing else (rotation at 20:03) | rust-coder | PASS |
| Rotation re-run (start screen) | `logcat -c`, launch, user_rotation 1, screenshot, user_rotation 0: pid 8214 unchanged at all three points, exactly 1 `dimuli started`, `logcat -d -s 'RustPanic:*' 'AndroidRuntime:E'` empty; rotation restored to user_rotation 0 / accelerometer_rotation 1 | me, 20:07 local | PASS |
| Chevron | `api36-chevron.png` (portrait, open: down chevron), landscape screenshots (closed: up, open: down); clean strokes, no box | rust-coder, inspected | PASS |
| Banner | `api36-banner.png`: "Collage gespeichert." below the top bar (Zurück/Speichern/Teilen visible), drawn cross in the close button | rust-coder, inspected | PASS |
| Attribution | `api36-start-attribution.png` and my landscape start screenshot: slint logo on a white pill, "Version 1.18.1 / https://slint.dev/" in white on #101114 | rust-coder + me | PASS |
| Launcher | `api36-launcher.png`: app drawer entry "dimuli" with a dark circular icon, blue tall cell left, two light cells right | rust-coder, inspected | PASS (see follow-up on mask clipping) |

## Decisions Made

- Spot-check instead of a full re-run: rust-coder's landscape and portrait screenshots were taken with the final native build (20:02:52 lib, 20:03 screenshots); the chevron in both states also appears there. I re-ran the rotation on the start screen with pid tracking and the crash filter, which also confirms the attribution with the final build.
- ANDR-01 and ANDR-08 are not marked complete: 05-06, 05-07 and 05-08 list them again.

## Deviations from Plan

**1. [Acceptance grep] One Path with switching `commands` instead of two Paths**
- **Plan:** "add two `Path` children ... an upward chevron visible while the panel is closed and a downward chevron visible while it is open"; acceptance `grep -c 'Path' editor-screen.slint >= 2`.
- **Built:** one `Path` with `commands: root.open ? "M 0 2 L 6 8 L 12 2" : "M 0 8 L 6 2 L 12 8"` (exactly the planned command strings). The grep returns **1**.
- **Judgment:** accepted. The truth behind the grep (drawn chevron, up closed / down open, no glyph) is met and visible in both orientations; one element instead of two duplicated ones with `visible` bindings fits the guidelines' no-duplication rule. The coordinator or owner can overrule; the fix would be adding a second Path and toggling `visible`, with no visual change. rust-coder did not run this grep individually.

**2. [Rule 1 - layout] Landscape body positioned explicitly instead of a VerticalLayout**
- **Issue:** with `VerticalLayout { TopBar; HorizontalLayout {...} }` the body only got its minimum height (canvas-min-height 120 px).
- **Fix:** `TopBar` at `y: 0`, body Rectangle at `y: top-bar.height`, `height: max(0px, parent.height - top-bar.height)`. All values from Theme or measured element sizes; result is the planned layout (screenshot shows full body height).

**3. [API] `Flickable.content-height` instead of `viewport-height`**
- `viewport-height` is deprecated in Slint 1.18; `content-height: column.preferred-height` implements the plan's "viewport height follows the inner layout's preferred height". `content-width: self.width` keeps the horizontally scrolling TemplateBar from widening the column.

**4. [Rule 2 - scope widened from 05-04 follow-ups] message-banner.slint and texts.slint**
- Banner close button "✕" was tofu on Android (05-04 finding); replaced by a drawn Path cross using the same tokens (`Theme.chevron-size`, `Theme.on-surface`, `Theme.border-medium`).
- Editor hint no longer says "oder Mausrad" (05-03 follow-up "Mausrad in the Android hint"). This also changes the desktop hint, which still supports the wheel; acceptable because the hint is a short gesture guide and touch is the target platform. Owner may want a platform-specific hint later.
- Both files are outside the plan's `files_modified`, but both items were explicitly handed to 05-05 by earlier summaries.

**5. [Rule 2 - UX] Banner below the top bar**
- `y: Theme.touch-target + 3 * Theme.spacing-s` (TopBar height is `touch-target + 2 * spacing-s`, so the banner sits one `spacing-s` below it). Fixes the 05-04 finding that the banner covered Zurück/Speichern/Teilen. The formula duplicates the TopBar height; follow-up: a `Theme.top-bar-height` token used by both.

**Total deviations:** 5 (1 acceptance-grep miss accepted, 2 implementation adjustments, 2 scope additions from earlier follow-ups). No new crates, no Rust changes.

## Issues Encountered

- No desktop screenshot possible under WSLg/Wayland; the desktop check is the 15 s run only. Desktop landscape view (window resized) is an owner check.
- My first rotation command failed because zsh does not word-split `$A` (`adb -s ...` in a variable); re-run with `ANDROID_SERIAL`. No effect on the emulator.

## Follow-ups / Open Items

From this plan:
- Portrait format buttons in the settings panel are very tall (~330 px in `api36-chevron.png`); pre-existing, consider capping their height.
- Landscape: the last TemplateBar thumbnail is cut at the panel edge (horizontal scroll affordance, acceptable) and the canvas does not grow/shrink when settings open (by design, the panel scrolls).
- Landscape settings scroll inside the side panel is not driven on the emulator (scroll input not allowed); check by finger on a real device (05-07).
- Launcher icon: the 60x60 grid (24..84) exceeds the 66 dp safe-zone circle at its corners, so the circular mask cuts the cell corners (visible in `api36-launcher.png`). Placeholder anyway; if kept, shrink to about 30..78.
- `Theme.top-bar-height` token instead of duplicating the TopBar height formula in app.slint (Deviation 5).
- Desktop hint lost the "Mausrad" mention (Deviation 4); optional platform-specific hint.
- Possibly add the second Path if the coordinator insists on the literal acceptance grep (Deviation 1).

Still open from 05-02..05-04:
- Code review (`code-reviewer`) for 05-01..05-05 not run yet; due before committing (05-08).
- 05-08 review: stale doc comment in controller.rs (~line 176); Kotlin failure branches of `saveToGallery`/`share`; `io.execute` after `io.shutdown()`.
- Remove the likely superfluous `@Deprecated`/`@Suppress("DEPRECATION")` on `onActivityResult` if the compiler stays quiet.
- Quote logcat filter specs in plans and docs (`'RustPanic:*'`) because of zsh.
- smoke.sh screenshot shows the splash, not the first Slint frame.
- Predictive back animation on a real device (05-07); owner visual confirmation of the 05-02 inset screenshots; gallery app view of saved collages (05-04).
- HEIC/WebP (D-12) not tested on the device.
- 05-06: run the landscape layout, chevron and launcher icon also on API 33 and 37.

## Open Human Checks

- Desktop: `cargo run -p dimuli-app -- android/build/smoke/pick-a.jpg android/build/smoke/pick-b.jpg android/build/smoke/pick-c.jpg`, resize the window wider than high. Expected: side panel on the right, chevron and banner cross drawn, attribution readable.
- Owner visual confirmation of `android/build/smoke/api36-editor-landscape.png`, `api36-settings-landscape.png`, `api36-editor-portrait.png`, `api36-chevron.png`, `api36-banner.png`, `api36-start-attribution.png`, `api36-launcher.png`. I inspected all of them and they match the plan's expectations (with the follow-ups above).
- Scroll the expanded settings inside the landscape side panel with a finger (real device, 05-07).

## User Setup Required

None.

## Next Phase Readiness

- Ready for 05-06 (emulator gate API 33/37).
- Emulator dimuli_api36 is still running as emulator-5554 with the x86_64-only debug build installed; dimuli is on the start screen in portrait; rotation settings restored (user_rotation 0, accelerometer_rotation 1).

---
*Phase: 05-android-shell*
*Completed: 2026-11-13*

## Self-Check: PASSED

All 3 created icon files and all 9 referenced evidence files exist and are non-empty; 0 commits since 262306a (as required, no commits); ROADMAP shows 05-05 checked and 5/8; STATE.md has one consistent position (Current Plan 6, Plan 6 of 8, 5 of 8 complete, next 05-06).
