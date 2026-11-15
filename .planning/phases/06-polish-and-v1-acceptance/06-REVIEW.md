## Review: Phase 6 – Polish und v1-Abnahme (`git diff ff11541 -- crates android`, keine untracked Dateien unter crates/ oder android/)

**Ergebnis:** ⚠️ freigegeben mit Auflagen (keine kritischen Befunde; zwei Warnungen sind vor dem Commit sinnvoll zu beheben oder bewusst zu akzeptieren)

### Automatische Prüfungen
| Prüfung | Ergebnis |
|---|---|
| fmt (`cargo fmt --all -- --check`) | ✅ keine Abweichungen |
| clippy (`--workspace --all-targets -D warnings`) | ✅ keine Warnungen |
| test (`cargo test --workspace`) | ✅ 90 (collage-core) + 133 (dimuli-app) bestanden, 0 fehlgeschlagen |
| `cargo ndk -t x86_64 --platform 33 check -p dimuli-app` | ⚠️ nicht auswertbar: Build-Skript von `i-slint-backend-android-activity` bricht mit „No Android platforms found" ab (`ANDROID_HOME` bzw. SDK-Plattformen in dieser Umgebung nicht auffindbar). Die Android-Rust-Teile (`platform/android.rs`) wurden daher nur per Lektüre geprüft. |
| cargo audit | nicht installiert, übersprungen |

### Bestätigungen zu den Schwerpunkten
- **Keine neue Dependency, Permission oder AndroidX:** Seit ff11541 ist keine `Cargo.toml`, `Cargo.lock` und keine Gradle-Datei geändert (`git diff ff11541 --stat` listet sie nicht). Im Manifest gibt es keine `uses-permission`/`queries`. Die einzige Manifest-Änderung ist die Erweiterung von `configChanges` (`fontWeightAdjustment|colorMode|grammaticalGender|touchscreen|mcc|mnc`), die keine Rechte betrifft.
- **JNI-Sicherheit** (`platform/android.rs`):
  - Native `native_images_picked(paths: String[], failed: jint)` entspricht `([Ljava/lang/String;I)V`. `native_method!` prüft die Signatur zur Compilezeit, und der Kotlin-Aufruf `nativeImagesPicked(Array<String>, Int)` passt dazu.
  - Alle Natives posten nur Events (`post_event`) und berühren kein Slint.
  - `error_policy = LogErrorAndDefault` verhindert Panics bzw. Exceptions an der Grenze.
  - Die zwei neuen ausgehenden Aufrufe (`collageLostOnRestart` `()Z` und `setCollageOpen` `(Z)V`) laufen über `with_activity`, das den Aufruf in `with_local_frame(16, …)` kapselt. Lokale Referenzen häufen sich also nicht an.
  - Der einzige neue `unsafe`-Bezug ist unverändert und hat ein `// SAFETY:`.
  - Fehler werden geloggt und führen zu `false`/No-op statt zum Abbruch.
  - Kein JNI-Namenskonflikt in Kotlin: `private var collageOpen/collageLost` erzeugen keine Accessoren, die mit `setCollageOpen`/`collageLostOnRestart` kollidieren.
- **Restart-Flag vor `super.onCreate`:** `MainActivity.kt` liest `collageLost = savedInstanceState?.getBoolean(KEY_COLLAGE_OPEN)` als erste Zeile in `onCreate`, also vor `super.onCreate`. `android_main` (in `NativeActivity` über `super.onCreate` gestartet) liest den Wert erst danach. `@Volatile` ist korrekt, weil Rust von einem anderen Thread schreibt bzw. liest. `onSaveInstanceState` schreibt den Wert.
- **Timer:** Die Closure in `arm_message_timer` (`controller.rs`) hält nur `seq` (Copy) und löst den Controller über `CURRENT` (`Weak`) auf, also keine starke Referenz und kein Zyklus. `armed_seq` verhindert Neustarts und wird bei Nicht-Erfolgsnachricht zurückgesetzt. `expire_message(seq)` löscht nur die Success-Nachricht mit passender `seq`; Fehler, Hinweise und neuere Nachrichten bleiben, durch Tests belegt.
- **State-Borrow:** Das Muster „kurz borgen, dann Slint/Plattform/Timer" wird in `refresh`, `update`, `update_gesture`, `finish_export` und `handle_platform_event` eingehalten (`set_collage_open` läuft nach Freigabe des Borrows). Ausnahme siehe Warnung 2.
- **Late-Result-Guards:** `finish_loading` verwirft Ergebnisse, wenn `busy != Loading`, und `finish_export` verwirft sie, wenn `busy != Exporting`. Beide sind getestet (`late_loading_result_is_ignored_when_not_loading`, `late_export_result_is_ignored_when_not_exporting`).
- **`fit_canvas` / Sanitizing:** `view::fit_canvas` liefert `0×0` bei NaN, ∞, ≤ 0 und bei einem Ergebnis ohne gültige Breite oder Höhe. Es gibt keine Division durch 0, weil Fläche und Collage-Größe vorher geprüft werden. `set_canvas_area` ignoriert ungültige Werte, und `refit_canvas` wird bei Laden, Formatwechsel und Vorlagenwechsel aufgerufen. Die Tests prüfen das auch (`aspect_change_refits_canvas_and_cells_in_one_transition`). Canvas-Größe und Zellen werden im selben `apply` gesetzt.
- **Gesten-Fast-Path:** `update_gesture` vergleicht `message_seq` und `screen` vor und nach dem Übergang und fällt dann auf `refresh()` zurück. Alle Meldungen laufen über `show()`, das `seq` hochzählt, auch `core_failed`. Andere Snapshot-Felder (Template-Index, Hintergrund, `confirm_back`, Busy) ändern `move_to`, `scroll` und `pinch` nicht, was ich im Code geprüft habe. Dividers werden nur während `DraggingDivider` aktualisiert. Die Puffer werden wiederverwendet (Test `gesture_rows_reuse_their_buffer`).
- **Kein `unwrap`/`expect`/`panic!`/ungeprüfter Index** im Produktionscode der geänderten Rust-Dateien (per Skript für alle Dateien vor `#[cfg(test)]` geprüft). `as_chunks` und `get` werden korrekt verwendet. Keine TODO-/`dbg!`-/`println!`-Reste im Diff.
- **Decode-Limits** (`collage-core/src/decode.rs`):
  - 64 MP, 16 384 Kante und 256 MiB sind umgesetzt.
  - Kante und Pixelzahl werden vor der Dekodierung geprüft; `u64`-Arithmetik verhindert Überläufe.
  - Die Pufferprüfung `decoder.total_bytes() > DECODE_MAX_ALLOC` kommt hinzu, weil `into_decoder` `max_alloc` nicht für den Ausgabepuffer reserviert; `ImageError::Limits` wird auf `ImageTooLarge` abgebildet.
  - Die Tests decken Kante +1 in Breite und Höhe, die Kante genau am Limit, 8000×8000 (= 64 MP, erlaubt), 8001×8000 (abgelehnt) und 6000×6000 mit 16-Bit-RGBA (288 MB, abgelehnt) ab.
- **Nutzertexte** (`texts.rs`): durchgehend Deutsch in der Du-Form. Alle Fehlertexte nennen, was passiert ist und was der Nutzer tun kann, und enthalten keine technischen Begriffe. Der Test `failure_texts_say_what_happened_and_what_to_do` erzwingt zwei Sätze und das Fehlen technischer Marker. `restart_notice` und `share_unsupported` sind getestet.
- **Kotlin-Transcoding** (`MainActivity.kt`):
  - `OutOfMemoryError` wird in `copyToCache` gefangen und in `transcode` auch beim Retry behandelt (Rethrow im letzten Versuch). `failed++` und das Löschen von `target` und `.tmp` passieren in beiden Catch-Zweigen.
  - Das Bitmap wird in `transcode` im `finally` recycelt, auch bei fehlgeschlagenem `compress`.
  - Die Ausgabe ist EXIF-frei: `compress` schreibt kein EXIF, und `ImageDecoder` wendet die Orientierung vorher an.
  - Der Retry mit Faktor 0,5 und höchstens 3 Wiederholungen bei OOM (auch als IOException „out of memory") ist begrenzt.
  - Eigene Lognachrichten enthalten keine URIs, nur den Index.

### Befunde
#### 🔴 Kritisch
keine

#### 🟠 Warnung
1. **Exception-Stacktraces können die URI ins Log schreiben** – `android/app/src/main/kotlin/ch/staeheli/dimuli/MainActivity.kt` (Catch-Blöcke in `copyToCache`, `Log.e(TAG, "transcode failed or copy failed", e)` und `Log.e(TAG, "image $index needs too much memory", e)`)
   Problem: Die eigenen Nachrichten sind sauber, aber das Übergeben des Throwables druckt `e.message` samt Stacktrace. `FileNotFoundException`, `SecurityException` und ImageDecoder-/ContentResolver-Fehler enthalten häufig die vollständige `content://…`-URI (Dateiname/Pfad des Nutzers). Das widerspricht dem Ziel „keine URI im Log". Der Altcode hatte dasselbe Muster, aber Phase 6 erweitert die Pfade.
   Vorschlag: Nur die Klasse loggen, z. B. `Log.e(TAG, "image $index failed: ${e.javaClass.simpleName}")`, und den Throwable nicht übergeben (für OOM ebenso).

2. **Plattformaufrufe während eines `borrow_mut()`** – `crates/dimuli-app/src/controller.rs` (in `Controller::new`, Block um `state.touch_target = …`, `controller.platform.share_supported()`, `controller.platform.restored_after_close()`)
   Problem: Die Regel im Modulkopf („state is borrowed only … never across a call into Slint, another callback or …") und die Guidelines zu Borrow-Dauer werden hier verletzt: Beide Plattformaufrufe und `window.global::<Theme>().get_touch_target()` laufen, während `state.borrow_mut()` aktiv ist. Aktuell sind alle Implementierungen trivial (Bool-Rückgabe), es kann also kein Re-Entry-Panic auftreten. Das wird aber fragil, sobald eine Plattform echte Arbeit leistet.
   Vorschlag: Werte vorher in lokale Variablen lesen und dann kurz borgen:
   ```rust
   let touch_target = window.global::<Theme>().get_touch_target();
   let share_supported = controller.platform.share_supported();
   let restored = controller.platform.restored_after_close();
   {
       let mut state = controller.state.borrow_mut();
       state.touch_target = touch_target;
       state.share_supported = share_supported;
       if restored { state.show_restart_notice(); }
   }
   ```

#### 🟡 Hinweis
1. **OOM-Erkennung per Nachrichtentext ist brüchig** – `MainActivity.kt` (`transcode`, `e.message?.contains("out of memory", ignoreCase = true)`). Das Verhalten ist harmlos (sonst wird der Fehler nach oben gereicht und das Bild gezählt/übersprungen), aber lokalisierte oder geänderte Meldungen führen ohne Retry zu „Bild übersprungen". Optional: Kommentar mit konkretem Gerät/API ergänzen oder bei jeder `IOException` einmalig mit halber Skalierung wiederholen.
2. **`set_collage_open` wird bei JNI-Fehler nicht wiederholt** – `controller.rs` (`refresh`): `collage_open_pushed` wird vor dem Plattformaufruf gesetzt, und `AndroidPlatform::set_collage_open` loggt Fehler nur. Folge: Ein einmal fehlgeschlagener Aufruf lässt den Restart-Hinweis nach Prozess-Tod aus. Das ist sehr unwahrscheinlich; optional `bool` aus `set_collage_open` zurückgeben und den Cache nur bei Erfolg setzen.
3. **`share/`-Cache wird beim Activity-Start gelöscht** – `MainActivity.kt` (`onCreate`, `File(cacheDir, "share").deleteRecursively()`). Wird die Activity neu erzeugt, während eine Ziel-App noch die geteilte Datei liest, schlägt deren Lesen fehl (`ShareProvider` antwortet jetzt mit `FileNotFoundException("nothing shared")`). Das ist ein seltener Randfall, und das Verhalten ist dokumentiert, daher nur ein Hinweis.
4. **16-Bit-Quellen unter 64 MP können als „zu groß" abgelehnt werden** – `decode.rs` (`total_bytes() > DECODE_MAX_ALLOC`). Beabsichtigt und getestet (`decoded_buffer_beyond_alloc_limit_is_image_too_large`), aber z. B. ein 16-Bit-RGBA-PNG ab ca. 5,9 MP·4 B·2 > 256 MiB wird verworfen. Der Nutzertext „Wähle ein kleineres Bild aus" ist dann passend. Optional in `PLAN.md`/Doku erwähnen.
5. **`edge_at_limit_is_accepted` und `pixel_budget_allows_exactly_64_megapixels` prüfen nur „nicht `ImageTooLarge`"** – `decode.rs` (Tests). Das ist für Header-only-PNGs ausreichend; die Aussagekraft ließe sich erhöhen, indem man den erwarteten anderen Fehler (`ImageDecode`) explizit prüft.
6. **Dauer des Smoke-Skripts** – `android/scripts/smoke.sh`: das neue `sleep 2` vor dem Screenshot ist funktional in Ordnung, aber ein fester Wert. Optional: auf ein Log-/UI-Signal warten.

### Positiv
- Saubere Trennung: Alle Logik (Fit-Berechnung, Nachrichten-Lebenszyklus, Late-Result-Guards, Limits) liegt in testbarem Rust-Code. Kotlin und JNI posten nur Events, und `cfg(target_os)` bleibt in `platform/`.
- Die Nachrichten-Sequenznummer ist eine robuste Lösung für den Auto-Hide-Timer, und der Timer hält keine starke Referenz.
- Sorgfältige Fehlerpfade in Kotlin: temporäre Datei plus atomares Ersetzen, Aufräumen bei jedem Fehler, korrekt gezählte `failed`-Werte bis in die UI-Texte.
- Gesten-Fast-Path mit Wiederverwendung der Puffer, differenziertem Row-Sync und sicherem Fallback bei Nachrichten- oder Screenwechsel.
- Gute Testabdeckung der neuen Logik (≈ 40 neue Tests), inkl. Grenzfällen (NaN, ∞, 0, negative Fehlerzähler, 64 MP ± 1).
- Keine neuen Abhängigkeiten, Berechtigungen oder AndroidX.

### Fazit
Die Änderung ist in gutem Zustand: fmt, clippy und Tests sind grün, JNI-Grenze, Restart-Erkennung, Timer, Late-Result-Guards und Decode-Limits sind korrekt umgesetzt. Vor dem Commit sollten die URI-sichere Log-Ausgabe (Warnung 1) und das Herausziehen der Plattformaufrufe aus dem `borrow_mut`-Block (Warnung 2) erledigt werden. Der Android-Rust-Check per `cargo ndk` konnte hier nicht laufen (SDK-Plattform nicht gefunden) und sollte lokal mit gesetztem `ANDROID_HOME` wiederholt werden.
