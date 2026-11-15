---
phase: 07-developer-docs-for-the-android-build
plan: 01
subsystem: docs
tags: [docs, android, build, claude-md, cargo-ndk, gradle]
status: complete
requires: []
provides:
  - "CLAUDE.md: verified cargo ndk + Gradle build/check/install/start/log commands, Android lint, fast emulator build, device note"
  - "CLAUDE.md: Struktur section with crates/collage-core, crates/dimuli-app, android/, docs/"
  - "docs/PLAN.md: milestone 7 ticked"
affects: [".claude/agents/rust-coder.md", ".claude/agents/code-reviewer.md (findings only, not edited)"]
tech-stack:
  added: []
  patterns: ["every CLAUDE.md sh-block command proven by a PASS line in a gitignored transcript"]
key-files:
  created: []
  modified: [CLAUDE.md, docs/PLAN.md]
decisions:
  - "Every adb/smoke command in CLAUDE.md is pinned to -s emulator-5554; the device path is one sentence pointing to README.md (T-07-01)"
  - "Agent build commands (cargo ndk ... check without ANDROID_JAR) fail with EXIT 101; listed for the owner, not edited"
metrics:
  duration: "~20 min"
  completed: 2026-11-15
requirements: [ENV-03]
actuals:
  tokens: 1250      # chars/4 over the realized CLAUDE.md + docs/PLAN.md diff (5005 chars)
  tasks: 3
  commits: 0        # owner rule: no commits; all changes left uncommitted
plan_head_before: 8198fe4ffa296adce754756245ac02202c4d2fa2
plan_head_after: 8198fe4ffa296adce754756245ac02202c4d2fa2
---

# Phase 7 Plan 01: Developer Docs for the Android Build Summary

CLAUDE.md now documents the real `cargo ndk` + Gradle flow (build, APK check, fast x86_64 build, smoke, install, start, logcat, Android lint) plus a Struktur section with `android/`. All 14 sh-block commands were run exactly as written on dimuli_api36 (emulator-5554) and passed. Milestone 7 is ticked in docs/PLAN.md.

Changes are uncommitted (owner rule). `actuals.commits: 0` is intentional: this was a docs-only plan with no commit allowed, not uncommitted code.

## Doc changes

**CLAUDE.md** (51 -> 76 lines, limit 85):
- UI bullet: Slint feature `backend-android-activity-06`, UI files `crates/dimuli-app/ui/*.slint`.
- Build bullet: `cargo-apk` replaced by `cargo ndk` + Gradle (`buildRust` -> `libdimuli.so` -> APK). Prerequisites: SDK + NDK r30, `ANDROID_HOME`/`ANDROID_NDK_HOME`, Rust targets `aarch64-linux-android`/`x86_64-linux-android`, `cargo-ndk`, JDK 17+, setup pointer to `docs/PLAN.md` M1. Three lines.
- Entwicklung bullet: "(Start: siehe Befehle)".
- New `## Struktur` (fenced `text`): `crates/collage-core/`, `crates/dimuli-app/`, `android/`, `docs/`.
- Befehle: the five host lines are unchanged. Added the Android lint line (with `ANDROID_JAR` and `-A clippy::missing_const_for_thread_local`) and the seven Android lines plus `assembleDebug -PrustAbis=x86_64`. Every adb/smoke command has `-s emulator-5554`.
- Below the block: "Alle Befehle im Repo-Wurzelverzeichnis" plus a device sentence (usbipd, see `README.md`, `-PrustAbis=arm64-v8a`, use the serial from `adb devices`).
- "Arbeitsweise für Claude" is unchanged.

**docs/PLAN.md**: only the milestone 7 checkbox changed (`- [x] 7.`). Milestone 8 is still open, and the line-33 rationale ("`cargo-apk` reicht hier nicht") is unchanged.

## Transcript results (android/build/smoke/p7-commands.txt, full pass from a freshly started emulator)

| # | Command | Result |
|---|---------|--------|
| 1 | `cargo check` | PASS: exit 0, no warning/error lines |
| 2 | `cargo run -p dimuli-app -- a.jpg b.png` | PASS: timeout 20, cwd target/p7-docs with generated images, exit 124 |
| 3 | `cargo test --workspace` | PASS: exit 0, 5 `test result: ok`, none FAILED |
| 4 | `cargo clippy --all-targets -- -D warnings` | PASS: exit 0, no warnings |
| 5 | `cargo fmt` | PASS: exit 0, no diff in crates |
| 6 | `ANDROID_JAR=... cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` | PASS: exit 0, no warnings |
| 7 | `emulator -avd dimuli_api36` | PASS: run_in_background, booted sdk=36 |
| 8 | `android/gradlew -p android assembleDebug` | PASS: BUILD SUCCESSFUL |
| 9 | `android/scripts/check-apk.sh` | PASS: APK CHECK OK |
| 10 | `android/gradlew -p android assembleDebug -PrustAbis=x86_64` | PASS: BUILD SUCCESSFUL abis=x86_64 (only `lib/x86_64/libdimuli.so`) |
| 11 | `android/scripts/smoke.sh -s emulator-5554` | PASS: SMOKE OK emulator-5554 |
| 12 | `adb -s emulator-5554 install -r .../app-debug.apk` | PASS: Success |
| 13 | `adb -s emulator-5554 shell am start -n ch.staeheli.dimuli/.MainActivity` | PASS: started pid=5422 |
| 14 | `adb -s emulator-5554 logcat -s dimuli` | PASS: timeout 10, saw `dimuli started` |

The Task 1 tracer (`p7-tracer.txt`) ran the seven Android lines and got 7 PASS and 0 FAIL. Then, following the protocol, its emulator was stopped with `adb emu kill` and a fresh one was started for the full pass.

Harness notes:
- The command lines were extracted from CLAUDE.md with the gate pipeline and run with `eval`/`bash -c`. The harness lives in the session scratchpad, not in the repo.
- Before the `am start` line, the harness ran `adb -s emulator-5554 logcat -c`. This is a test step, not a documented command. Because of it, `dimuli started` in the logcat output comes from this start and not from the earlier smoke run.
- The desktop run printed harmless arboard clipboard WARN lines and an xdg-settings D-Bus notice. There was no `panicked` and no `UI failed`.

## Phone smoke (8200fcc9)

The phone was listed as `device`, so the device smoke ran. `android/gradlew -p android assembleDebug -PrustAbis=arm64-v8a` produced an APK with only `lib/arm64-v8a/libdimuli.so`. Then `android/scripts/smoke.sh -s 8200fcc9` printed **SMOKE OK 8200fcc9** (`android/build/smoke/p7-device-8200fcc9-smoke.txt`). usbipd did not drop the connection. Only the smoke ran on the phone, no picker or gallery automation.

Side effect: the debug APK on disk is now arm64-only (from the device build). The next `assembleDebug` rebuilds both ABIs.

## Agent command findings (owner decision; .claude/agents/* not edited)

`android/build/smoke/p7-agents.txt` starts with **EXIT 101**.

| File | Current command | Problem | Proposed replacement |
|------|-----------------|---------|----------------------|
| `.claude/agents/rust-coder.md` (Abschluss, line 65) | `cargo ndk -t x86_64 --platform 33 check -p dimuli-app` | FAILS (EXIT 101): the build script of `i-slint-backend-android-activity` 1.18.1 panics with "No Android platforms found" (build.rs:34) because `ANDROID_JAR` is not set | `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 check -p dimuli-app --lib` (verified: exit 0). Even better: use the CLAUDE.md lint line (clippy with `-D warnings -A clippy::missing_const_for_thread_local`). |
| `.claude/agents/code-reviewer.md` (Automatische Prüfungen, line 35) | same command | same failure | same replacement; the CLAUDE.md clippy line is the stronger check |
| both files | `cargo fmt --all` / `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` | Different from CLAUDE.md (`cargo fmt`, `cargo clippy --all-targets -- -D warnings`), but both forms pass (fmt check exit 0, workspace clippy exit 0). These differences do no harm. | optional: none needed |
| both files | no Gradle/APK commands | the agents do not document the Gradle build. That matches their role (CLAUDE.md is the entry point). | none |

## README comparison ("Bauen, installieren, prüfen"; README not edited)

- `android/gradlew -p android assembleDebug`, `android/scripts/check-apk.sh`: identical to CLAUDE.md.
- `android/scripts/smoke.sh -s <SERIAL>` and `adb -s <SERIAL> logcat -s dimuli`: same form. README uses the `<SERIAL>` placeholder, CLAUDE.md pins `emulator-5554`.
- README lacks the emulator start, `-PrustAbis` fast build, `adb install`, `am start` and the Android lint line. Those are additions in CLAUDE.md, not contradictions.
- Comments differ in wording only ("16-KB-Ausrichtung ... SDK-Versionen" vs. "16-KB-Alignment ... SDK-Stufen"). There are no conflicting commands.

## .planning/ follow-ups for the phase transition (not edited by this plan)

- `.planning/PROJECT.md` line 30 (Active): `- [ ] CLAUDE.md documents the cargo ndk + Gradle flow instead of cargo-apk — Phase 7 (ENV-03)`: mark as done.
- `.planning/PROJECT.md` line 50 (Context "Known stale item"): CLAUDE.md no longer lists `cargo apk`. Remove the item or move it to history.
- `.planning/STATE.md` line 108 (Blockers "[Docs]"): resolved, so remove it.
- Optional new item: the agent-file `cargo ndk ... check` command is stale (see above).

## Owner checks

- Read `git diff -- CLAUDE.md docs/PLAN.md`, especially "Struktur" and "Befehle". Check that it is short and German, that these are the commands you actually use, that the emulator as default (`-s emulator-5554`) and the device sentence fit your workflow (device first for short tests), and decide whether the default target should be the phone instead. Milestone 7 can be reopened if needed.
- Decide on the agent command replacement in `.claude/agents/rust-coder.md` and `.claude/agents/code-reviewer.md` (table above).

## Gate results

- Task 1: G1 (no cargo-apk in CLAUDE.md), G2 (7 Android lines, each PASS in tracer), G3 (sdk=36, APK CHECK OK, SMOKE OK, dimuli started, no FAIL), G4 (required commands present, no code/build/agent file changed): all exit 0.
- Task 2: 14 commands each with PASS in p7-commands.txt; sdk=36/APK CHECK OK/abis=x86_64/SMOKE OK/dimuli started/no FAIL; `## Struktur`; 3 structure paths; lint, -PrustAbis, README pointer, feature name; 76 <= 85 lines; phone smoke OK and no guarded file changed: all exit 0.
- Task 3: `[x] 7.` present, no `[ ] 7.`, `[ ] 8.` still open, `reicht hier nicht` kept, `git grep cargo[- ]apk` outside .planning and docs/PLAN.md -> exit 1 (no match), p7-agents.txt starts with `EXIT 101` and .claude/agents unchanged, full-pass re-check after the final CLAUDE.md: all exit 0.

## Deviations from Plan

None. The plan was executed as written. The only harness addition was the `logcat -c` before `am start` described above, a test step that changes no documented command.

## Environment state left behind

- Emulator dimuli_api36 (`emulator-5554`) is still running, started as a background task of this agent. Stop it with `adb -s emulator-5554 emu kill` when it is no longer needed.
- Evidence (gitignored): `android/build/smoke/p7-tracer.txt`, `p7-commands.txt`, `p7-agents.txt`, `p7-device-8200fcc9-smoke.txt`, `p7-logs/`, plus `target/p7-docs/{a.jpg,b.png}`.

## Self-Check: PASSED

- CLAUDE.md and docs/PLAN.md modified (git status `M`). The evidence files exist under android/build/smoke/.
- No commits expected (owner rule): HEAD is still 8198fe4.
