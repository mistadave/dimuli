## Conflict Detection Report

### BLOCKERS (0)

(none)

### WARNINGS (0)

(none)

### INFO (4)

[INFO] No ADR or PRD documents in the ingest set
  Found: 3 docs ingested: /home/dstaehel/Documents/dev/dave/dimuli/docs/ARCHITECTURE.md (SPEC), /home/dstaehel/Documents/dev/dave/dimuli/docs/PLAN.md (DOC), /home/dstaehel/Documents/dev/dave/dimuli/docs/CODING_GUIDELINES.md (DOC)
  Note: decisions.md and requirements.md are intentionally empty. The "Leitentscheidungen" table in ARCHITECTURE.md is preserved as a non-locked entry in constraints.md. v1 scope statements from PLAN.md are in context.md (no acceptance criteria exist in any source). No locked decisions exist, so no LOCKED-vs-LOCKED check applied. No cycles in the cross-ref graph (CODING_GUIDELINES -> PLAN -> ARCHITECTURE); no UNKNOWN/low-confidence classifications.

[INFO] Build tooling differs between CLAUDE.md and PLAN.md (already tracked)
  Found: /home/dstaehel/Documents/dev/dave/dimuli/docs/PLAN.md (section "Android-Integration") states that `cargo-apk` is not sufficient and prescribes `cargo ndk` + Gradle with a Kotlin `MainActivity : NativeActivity`; /home/dstaehel/Documents/dev/dave/dimuli/CLAUDE.md (not an ingested doc, referenced by CODING_GUIDELINES.md) still lists `cargo apk build/run` commands.
  Note: PLAN.md milestone 7 (open) is the planned fix. Not a blocker; PLAN.md (the ingested source) is taken as the intended direction. Relevant to milestone 5 (Android shell).

[INFO] Stale "as-is" statements in PLAN.md
  Found: /home/dstaehel/Documents/dev/dave/dimuli/docs/PLAN.md section "Kontext" says the repo contains only CLAUDE.md and that Android SDK/NDK, `adb`, `cargo-ndk` and targets are missing; milestones 1-4 in the same file are checked done.
  Note: Recorded as historical in context.md. Milestone checkbox state (0-4 done, 5-8 open) is taken as current.

[INFO] Known pending deviations from guidelines and constants
  Found: CODING_GUIDELINES.md section 7 requires test images <= 64 px, while PLAN.md M6 notes some render tests use sizes above 64 px; ARCHITECTURE.md sets `DECODE_MAX_PIXELS` = 100 MP while PLAN.md M6 notes it may be lowered to 64 MP for devices.
  Note: Both are acknowledged open items in PLAN.md milestone 6, not contradictions. ARCHITECTURE.md (SPEC) values are kept in constraints.md as current. Other cross-checked values agree across sources: preview max edge 2048 px, `MAX_IMAGES` 9 with milestone 8 raising it, 48 px touch targets, minSdk 36, back behavior.
