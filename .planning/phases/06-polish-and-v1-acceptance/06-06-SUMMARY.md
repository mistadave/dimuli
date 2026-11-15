---
phase: 06-polish-and-v1-acceptance
plan: 06
status: complete
requirements: [PERF-01]
completed: 2026-11-14
actuals:
  tasks: 3
  commits: 0
---

# Phase 6 Plan 06: Android image and memory matrix

The updated Android build loaded and saved large synthetic images on API 33, 36 and 37. API 37 required the separately authorized lower-resolution decoder retry after its first 80 MP decode exhausted memory. The full two-ABI APK passed check-apk. No commits were created.

## Memory on API 36

| Scenario | VmHWM | VmRSS | Evidence |
| --- | ---: | ---: | --- |
| 50 MP load, pan, save | 675048 kB (~659 MiB) | 673112 kB (~657 MiB) | `p6-06-final-big50-mem.txt` |
| Seven-image load and save | 690896 kB (~675 MiB) | 273700 kB (~267 MiB) | `p6-06-final-mem-matrix.txt` |

The research estimate was 350–450 MB for nine large photos; these peak RSS measurements exceed it and should not be compared directly without matching image sizes and measurement methods. API 37 retry test reported VmHWM 334844 kB. The API 37 emulator reported about 4 GB total RAM despite its configured 2 GB; these numbers are emulator observations, not device guarantees.

## Matrix and formats

API 36 imported seven images, logged two downscales and three transcodes, saved successfully and retained PID 5367. Selection order was HEIC, transparent WebP, opaque WebP, rotated JPEG, 12 MP JPEG, 50 MP JPEG, 80 MP JPEG. Cache copies in that order: JPEG 1600×1200; PNG 800×800; JPEG 1600×1200; JPEG 7302×8763; JPEG 4000×3000; JPEG 8160×6144; JPEG 8944×7155. The rotated photo was upright with its red band on the right; the editor showed all seven images. AVIF was transcoded successfully on API 36.

API 33 smoke and three-image import/save passed on the updated build. API 37 initially failed to import the 80 MP image, then the authorized retry at half target dimensions succeeded at 4472×3577; three images imported with zero failures. The subsequent successful save was observed live by the executing worker but its log was not captured in the evidence file. The owner device was not attached.

## Research assumptions

- A1 and A2: the rotated large JPEG was upright and preserved its aspect after scaling; this does not establish orientation behavior for every HEIF variant.
- A3: no double rotation was observed; the test did not separately establish JPEG metadata removal or alpha-to-black behavior. Transparent WebP became PNG.
- A4: not verified; no real 10-bit HEIF was tested.
- A5: not established; the small synthetic HEIC succeeded, but a larger synthetic HEIC was not tested.
- A10: `run-as` reading process status worked for VmHWM and VmRSS.

## Deviations and remaining observations

API 37 smoke initially failed twice with `back did not close the app`; after emulator restart the investigating worker observed five successful smoke runs in tool output. No separate successful smoke transcript was saved, and the original failure was not root-caused. Full details and original evidence are in `06-06-FINDINGS.md`.

Automatic approval review rejected deleting all emulator media entries matching `p6-%`; the matrix was repeated using separately named synthetic files without deleting the entries. It also rejected restarting the API 36 emulator after the tests, so it was not restarted. No source was changed by this plan's test execution; the decoder retry was separately authorized and implemented between runs.

PERF-01 remains open for the real-device acceptance in 06-08. Evidence files are in ignored `android/build/smoke/`.
