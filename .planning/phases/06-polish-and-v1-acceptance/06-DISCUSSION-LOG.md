# Phase 6 Discussion Log (2026-11-14)

Human reference only; not consumed by downstream agents.

Owner chose not to discuss areas one by one: "mach für alle punkte einen vorschlag wie umzusetzen, sehe mir diese an und diskutiere nur bei bedarf".
Claude proposed all four gray areas; owner: "alle punkte so übernehmen und weiter machen".

| Area | Proposal accepted |
|------|-------------------|
| HEIC/WebP | Kotlin transcodes non-JPEG/PNG picks via ImageDecoder to JPEG q95 in cache; no new dependency; failures = unreadable |
| Process death | No persistence; normal pause/resume proven; restart notice after system close; research activity recreation within a live process |
| Large images | DECODE_MAX_PIXELS 64 MP everywhere; Kotlin downscales picks above 64 MP; Rust limit as safety net |
| Messages | Banner kept; success auto-hides ~3 s, errors stay; own German text per failure path + log::error! |

Claude's discretion: M3/M4 notes, 05-REVIEW N1-N6 where cheap, wording, auto-hide details, test split per device-first rule.
