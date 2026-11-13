# API Coverage — Android platform services used by the dimuli shell (Phase 5)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.
> Scope: the Android framework APIs the shell integrates through Kotlin `MainActivity` / `ShareProvider` and the Slint Android backend (minSdk 33, targetSdk 37). No third-party web API is involved.

| capability | decision | reason |
|---|---|---|
| photo-picker-multi-select (ACTION_PICK_IMAGES, EXTRA_PICK_IMAGES_MAX) | INTEGRATE | |
| photo-picker-single-select (result in Intent.data) | INTEGRATE | |
| photo-picker-cancel (RESULT_CANCELED) | INTEGRATE | |
| photo-picker-max-limit (MediaStore.getPickImagesMaxLimit) | INTEGRATE | |
| photo-picker-video-and-mixed-media | OPT-OUT | collages are built from still images only (type image/*) |
| photo-picker-persistable-uri-permission | OPT-OUT | images are copied to the cache right after picking; no later access to the URIs is needed |
| photo-picker-embedded-in-app (Android 15+ embedded picker) | OPT-OUT | not available at minSdk 33; the system picker covers the need on every supported version |
| photo-picker-heic-webp-conversion | OPT-OUT | deferred to Phase 6 by owner decision D-12 (PERF-01/ERRM-01); skipped with the existing notice |
| mediastore-images-insert (Pictures/dimuli, IS_PENDING) | INTEGRATE | |
| mediastore-delete-own-failed-row | INTEGRATE | |
| mediastore-update-or-edit-saved-collage | OPT-OUT | no persistence or project editing in v1 (PERS-01 is v2) |
| share-action-send-single-image (chooser, ClipData grant) | INTEGRATE | |
| share-action-send-multiple | OPT-OUT | one collage per share |
| share-chooser-result-callback (IntentSender) | OPT-OUT | owner decision D-11: no success message; the chooser itself is the feedback |
| content-provider-read-only-single-file (own ShareProvider) | INTEGRATE | |
| androidx-fileprovider | OPT-OUT | owner decision D-09: no AndroidX; own ContentProvider serves the one file |
| on-back-invoked-callback (predictive back dispatch) | INTEGRATE | |
| on-back-animation-callback (API 34) | OPT-OUT | not needed yet: no custom back animation in v1 and the API is above the minSdk 33 floor |
| window-safe-area-insets (via Slint's Java helper) | INTEGRATE | |
| edge-to-edge-enable-api-33-34 (setDecorFitsSystemWindows) | INTEGRATE | |
| runtime-permissions | OPT-OUT | none required at minSdk 33; guidelines forbid extra manifest permissions |
| activity-lifecycle-state-restore | OPT-OUT | explicitly out of scope for Phase 5: lifecycle robustness is LIFE-01 in Phase 6; configChanges keeps the app alive on rotation |
