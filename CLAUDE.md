# CLAUDE.md

## Projekt

**dimuli** ist eine Collage-Maker-App für Android: Mehrere Bilder (1–n) werden in Felder eines Layouts angeordnet
und als ein Bild exportiert.

- **Sprache:** Rust (stable)
- **UI:** [Slint](https://slint.dev) mit dem Android-Backend (`backend-android-activity-06`); UI-Dateien unter
  `crates/dimuli-app/ui/*.slint`
- **Build:** `cargo ndk` + Gradle; der Gradle-Task `buildRust` baut `libdimuli.so` mit `cargo ndk`, Gradle baut daraus
  die APK (`android/`). Voraussetzungen: Android SDK + NDK r30 (`ANDROID_HOME`, `ANDROID_NDK_HOME` müssen gesetzt sein),
  Rust-Targets `aarch64-linux-android`, `x86_64-linux-android`, `cargo-ndk`, JDK 17+ (Einrichtung: `docs/PLAN.md`, M1)
- **Android:** minSdk 33 (Android 13), targetSdk 37 (Android 17), NDK r30; ABIs `arm64-v8a` und `x86_64` (Emulator)
- **Entwicklung:** komplett in WSL2; Emulator-AVDs `dimuli_api33`, `dimuli_api36` und `dimuli_api37` (Start: siehe Befehle)

**Plan und Fortschritt:** [`docs/PLAN.md`](docs/PLAN.md). Dort stehen Meilensteine und Android-Setup;
erledigte Meilensteine werden dort abgehakt.

**Architektur:** [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md). Module, Datenmodell, Datenfluss, Grenzen.

**Coding Guidelines:** [`docs/CODING_GUIDELINES.md`](docs/CODING_GUIDELINES.md). Vor dem Schreiben von Code lesen
und einhalten; die Checkliste am Ende vor jedem Abschluss abarbeiten.

## Struktur

```text
crates/collage-core/   # Logik ohne UI-/Android-Abhängigkeit: Vorlagen, Geometrie, Decode, Export (host-testbar)
crates/dimuli-app/     # Slint-UI (`ui/*.slint`), Zustand, Controller, Plattform-Schicht (`src/platform/`);
                       # lib `dimuli` (cdylib für Android) + bin (Desktop)
android/               # Gradle-Projekt: Manifest, Kotlin `MainActivity`/`ShareProvider`,
                       # `scripts/` (check-apk.sh, smoke.sh)
docs/                  # PLAN.md, ARCHITECTURE.md, CODING_GUIDELINES.md
```

## Befehle

```sh
cargo check                       # schnelle Prüfung (Host)
cargo run -p dimuli-app -- a.jpg b.png   # Desktop-Vorschau; Bilder als Argumente (kein Dateidialog)
cargo test --workspace            # alle Tests (Host)
cargo clippy --all-targets -- -D warnings
cargo fmt

# Android-Lint bei Änderungen an Android-Code; `-A ...`: Fehlalarm bei `thread_local!` in controller.rs
ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local

emulator -avd dimuli_api36                  # Emulator im eigenen Terminal; Seriennummer emulator-5554
android/gradlew -p android assembleDebug    # Debug-APK für arm64-v8a und x86_64; buildRust ruft cargo ndk
android/scripts/check-apk.sh                # 16-KB-Alignment, keine Berechtigungen, SDK-Stufen; braucht beide ABIs
android/gradlew -p android assembleDebug -PrustAbis=x86_64   # schneller: nur Emulator-ABI (Gerät: arm64-v8a)
android/scripts/smoke.sh -s emulator-5554   # installieren, starten, Start-Log prüfen, Zurück schließt die App
adb -s emulator-5554 install -r android/app/build/outputs/apk/debug/app-debug.apk
adb -s emulator-5554 shell am start -n ch.staeheli.dimuli/.MainActivity
adb -s emulator-5554 logcat -s dimuli       # App-Logs
```

Alle Befehle im Repo-Wurzelverzeichnis. Echtes Gerät: per usbipd an WSL anhängen (siehe `README.md`), mit
`-PrustAbis=arm64-v8a` bauen und `emulator-5554` durch die Seriennummer aus `adb devices` ersetzen.

## Arbeitsweise für Claude

- Vor dem Abschluss einer Änderung `cargo fmt`, `cargo clippy` und `cargo test` ausführen und Fehler beheben.
- Logik (Layouts, Feldberechnung, Bildskalierung/-zuschnitt, Export) ohne Android-Abhängigkeiten halten,
  damit sie auf dem Host testbar ist; Android-spezifischen Code (Bildauswahl, Speichern, Berechtigungen) kapseln.
- UI-Änderungen zuerst per `cargo run -p dimuli-app -- …` auf dem Desktop prüfen; Android-Build nur bei plattformspezifischen Änderungen.
- Kein `unwrap()`/`expect()` in Produktionscode: ein Panic lässt die App abstürzen.
- Neue Crates nur nach Rückfrage hinzufügen und auf Android-Kompatibilität prüfen.
- Code wird immer vom Subagent `rust-coder` (`.claude/agents/rust-coder.md`) geschrieben, auch einfache Anpassungen.
  Der Hauptagent liest, plant, koordiniert und prüft die Ergebnisse. Sehr komplexe Aufgaben vorher in kleinere,
  klar abgegrenzte Subtasks aufteilen und einzeln an `rust-coder` geben.
- Nach Abschluss eines Meilensteins bzw. vor dem Commit neuer Funktionalität den Subagent `code-reviewer`
  (`.claude/agents/code-reviewer.md`) ausführen und kritische Befunde beheben. Kein erneutes Review, wenn die
  Änderungen nur Befunde eines vorherigen Reviews umsetzen, sowie bei reinen Doku-/Konfigurationsänderungen.
- Nicht ohne ausdrückliche Aufforderung committen oder pushen.
- Code und Kommentare auf Englisch, Kommunikation auf Deutsch.
