# Plan 06-06: Test findings

Initial checks found two API 37 failures. A subsequent authorized decoder retry change and repeat checks are described below; no commits were made.

## API 37: 80 MP image import failed

On `dimuli_api37` (emulator-5558), build the full debug APK, install it, and select `p6-small.heic`, `p6-big80.jpg` (10000 × 8000), and `p6-alpha.webp` together. The log records `java.io.IOException: getPixels failed with error out of memory` in `MainActivity.transcode` and `picked images: 2, failed copies: 1`; the editor shows the other two images and an unreadable-image notice. Their export succeeded. Evidence: `android/build/smoke/p6-06-emulator-5558-logcat.txt` and `p6-06-emulator-5558-editor.png`.

Repeat measurement: the emulator booted with 4,006,172 kB total RAM (despite the AVD configuration specifying 2 GB); `dalvik.vm.heapgrowthlimit` was 192m and `dalvik.vm.heapsize` 576m. The process address-space limit was unlimited. The same import failed again in `ImageDecoder.nDecodeBitmap`, with 2 images accepted and 1 failed. Sampling `/proc` at approximately 0.2 s intervals recorded a process VmHWM of 242,716 kB and a minimum system MemAvailable of 2,162,812 kB. After the error, `dumpsys meminfo` reported native heap allocated 68,620 kB and Dalvik heap allocated 2,033 kB. RSS does not measure failed allocation requests or all virtual reservations; these observations do not establish the decoder's allocation limit or prove which allocation failed. The Java heap values are not a total native-process limit.

Retry experiment: on API 37 the first decode again reported out of memory, then a retry at half the previous target width and height succeeded at 4472 × 3577. All three images loaded and the collage saved successfully; VmHWM was 334,844 kB. Evidence: `android/build/smoke/p6-retry-api37-logcat.txt` and `p6-retry-api37.png`. The retry catches OutOfMemoryError and IOExceptions whose message contains “out of memory”; other decoder error wording has not been validated. The separate API 37 back-navigation failure remains open.

## API 37: Smoke back check failed twice

`android/scripts/smoke.sh -s emulator-5558` returned `SMOKE FAIL: back did not close the app` twice. The process remained present after the first run. Determine whether this is an app lifecycle defect or a smoke-check assumption before fixing. Evidence: `p6-06-emulator-5558-smoke.txt` and `p6-06-emulator-5558-smoke-retry.txt` in `android/build/smoke/`.

## Future improvement: tiled image processing

Investigate decoding and rendering large images in regions or tiles to reduce peak memory while retaining detail for zoom and export. Evaluate Android format support, image orientation, cache and export behavior, and complexity against the simpler reduced-resolution retry. This is deferred; it is not a prerequisite for the current retry test.

## Successful checks

- API 36: 50 MP image loaded, panned and saved; VmHWM 644560 kB and VmRSS 165412 kB.
- API 36: seven mixed images loaded and saved with PID 5646 unchanged, two downscales, three transcodes, no failed copies; VmHWM 733380 kB and VmRSS 316708 kB. Cache formats and sizes: JPEG 8944×7155, JPEG 1600×1200, JPEG 8160×6144, JPEG 1600×1200, JPEG 7302×8763, JPEG 4000×3000, PNG 800×800. The rotated image is upright with the red band on the right.
- Full two-ABI APK check passed; API 33 smoke, three-image import and save passed.
- On the updated build API 36 repeated the 50 MP case (VmHWM 675048 kB) and seven-image case (VmHWM 690896 kB, VmRSS 273700 kB); all seven loaded and saved, PID unchanged. Cache sizes: JPEG 1600×1200, PNG 800×800, JPEG 1600×1200, JPEG 7302×8763, JPEG 4000×3000, JPEG 8160×6144, JPEG 8944×7155. AVIF opened and transcoded successfully.
- On the updated build the full two-ABI APK check, API 36 smoke and API 33 smoke / three-image import / save passed.
- The owner device was not attached.

The original API 37 back-check failures have not been root-caused. The parent reports five subsequent successful smoke runs after emulator restart; their supporting output is being reconciled. PERF-01 also remains subject to real-device acceptance.
