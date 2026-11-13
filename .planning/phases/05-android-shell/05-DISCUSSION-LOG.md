# Phase 5: Android Shell - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-13
**Phase:** 05-android-shell
**Areas discussed:** App identity

Areas offered but not selected: Dependencies (AndroidX vs platform APIs, `jni`/`android_logger` approval), Save & share UX, Test & acceptance.

---

## App Identity

### Package name (applicationId)

| Option | Description | Selected |
|--------|-------------|----------|
| ch.staeheli.dimuli | Reverse domain with the owner's surname | ✓ |
| io.github.<user>.dimuli | GitHub-based, no own domain | |
| dev.dimuli.app | Neutral, project-based | |

### Launcher name

| Option | Description | Selected |
|--------|-------------|----------|
| dimuli | Lower case, like the project | ✓ |
| Dimuli | Capitalised | |

### Launcher icon

| Option | Description | Selected |
|--------|-------------|----------|
| Simple placeholder icon | Adaptive vector icon with a stylised collage grid | |
| Android default icon | Green default icon for now | |
| Owner supplies icon | Owner provides artwork, integrated as adaptive icon | ✓ |

Follow-up: until the icon arrives → **placeholder until delivery** (Phase 5 does not wait). Expected format → **still open**.

### Signing

| Option | Description | Selected |
|--------|-------------|----------|
| Debug build only | Debug keystore, enough for emulator and device via adb | ✓ |
| Also release build | Own keystore, signingConfig, R8 | |

### Orientation

| Option | Description | Selected |
|--------|-------------|----------|
| Portrait and landscape | No lock; Android 16+ ignores locks on large screens | ✓ |
| Portrait only | screenOrientation=portrait | |

---

## Claude's Discretion

Dependencies (with mandatory owner approval for any crate/library), save & share UX details, rotation/configChanges handling, insets source, lint exception, test scope.

## Deferred Ideas

- Release signing / Play Store setup
- Final launcher icon (format open)
