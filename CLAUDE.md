# CLAUDE.md

## Projekt

**dimuli** ist eine Collage-Maker-App für Android: Mehrere Bilder (1–n) werden in Felder eines Layouts angeordnet
und als ein Bild exportiert.

- **Sprache:** Rust (stable)
- **UI:** [Slint](https://slint.dev) mit dem Android-Backend (`backend-android-activity`); UI-Dateien unter `ui/*.slint`
- **Build:** `cargo-apk`, Android SDK + NDK (`ANDROID_HOME`, `ANDROID_NDK_HOME` müssen gesetzt sein)
- **Android:** minSdk 36 (Android 16), targetSdk 37 (Android 17), NDK r30; ABIs `arm64-v8a` und `x86_64` (Emulator)
- **Entwicklung:** komplett in WSL2; Emulator-AVDs `dimuli_api36` und `dimuli_api37` (Start: `emulator -avd dimuli_api36`)

**Plan und Fortschritt:** [`docs/PLAN.md`](docs/PLAN.md). Dort stehen Architektur und Meilensteine;
erledigte Meilensteine werden dort abgehakt.

## Befehle

```sh
cargo check                       # schnelle Prüfung (Host)
cargo run                         # Desktop-Vorschau der UI (Host)
cargo test                        # Tests der Collage-Logik (Host)
cargo clippy --all-targets -- -D warnings
cargo fmt

cargo apk build --target aarch64-linux-android --lib   # APK bauen
cargo apk run   --target aarch64-linux-android --lib   # auf Gerät installieren und starten
adb logcat -s dimuli                                   # App-Logs
```

## Arbeitsweise für Claude

- Vor dem Abschluss einer Änderung `cargo fmt`, `cargo clippy` und `cargo test` ausführen und Fehler beheben.
- Logik (Layouts, Feldberechnung, Bildskalierung/-zuschnitt, Export) ohne Android-Abhängigkeiten halten,
  damit sie auf dem Host testbar ist; Android-spezifischen Code (Bildauswahl, Speichern, Berechtigungen) kapseln.
- UI-Änderungen zuerst per `cargo run` auf dem Desktop prüfen; Android-Build nur bei plattformspezifischen Änderungen.
- Kein `unwrap()`/`expect()` in Produktionscode: ein Panic lässt die App abstürzen.
- Neue Crates nur nach Rückfrage hinzufügen und auf Android-Kompatibilität prüfen.
- Nicht ohne ausdrückliche Aufforderung committen oder pushen.
- Code und Kommentare auf Englisch, Kommunikation auf Deutsch.
