# Plan: dimuli – Collage-Maker (Rust + Slint, Android)

## Kontext

dimuli ist eine Android-App, die 1 bis n Bilder in Felder einer Collage anordnet und das Ergebnis in die
Galerie speichert oder teilt. Das Repo enthält bisher nur `CLAUDE.md`. Für die erste Version gilt:
- Vorlagen je Bildanzahl, deren Trennlinien sich verschieben lassen
- Bild im Feld verschieben und zoomen, Bilder zwischen Feldern tauschen
- Abstand, Rand/Eckenradius, Hintergrundfarbe und Seitenverhältnis (1:1, 4:5, 9:16, 16:9)
- Export in die Galerie und über den Teilen-Dialog
- Entwicklung und Test komplett in WSL2: Der Android-Emulator läuft dort über KVM, sein Fenster erscheint über WSLg

Ist-Zustand der Werkzeuge: Rust 1.99 ist installiert. Es fehlen Android SDK/NDK, `adb`, `cargo-ndk` und die Android-Targets.

## Architektur

Cargo-Workspace mit zwei Crates und einem Gradle-Projekt:

```
dimuli/
├── Cargo.toml                 # workspace
├── crates/
│   ├── collage-core/          # reine Logik, keine UI-/Android-Abhängigkeit, voll testbar
│   └── dimuli-app/            # Slint-UI + Plattform-Schicht; lib (cdylib für Android) + bin (Desktop)
│       ├── ui/*.slint
│       └── src/platform/{mod.rs, desktop.rs, android.rs, native_events.rs}
└── android/                   # Gradle-Projekt: Manifest, Kotlin-Activity, ShareProvider, scripts/
```

Details zu Modulen, Datenmodell, Datenfluss und Grenzen: [`ARCHITECTURE.md`](ARCHITECTURE.md).

### Android-Integration
`cargo-apk` reicht hier nicht: Bildauswahl und Teilen brauchen `onActivityResult` bzw. Intents, und
`NativeActivity` leitet diese nicht an nativen Code weiter. Deshalb:
- Gradle-Projekt mit `MainActivity : NativeActivity` in Kotlin. Slint läuft über das Feature
  `backend-android-activity-06` (`native-activity`).
- **Bildauswahl:** System-Photo-Picker über `MediaStore.ACTION_PICK_IMAGES` und `onActivityResult`
  (`NativeActivity` ist keine AndroidX-`ComponentActivity`, daher kein Activity-Result-Contract). Kotlin kopiert die
  ausgewählten URIs nach `cacheDir/picked/img-<n>` (alles außer JPEG/PNG wandelt es dabei mit `ImageDecoder` um,
  Bilder über 64 MP bzw. 16 384 px werden verkleinert) und ruft `nativeImagesPicked(paths, failed)` auf; die nativen Funktionen
  registriert Rust per `RegisterNatives`. Rust übernimmt den Rest.
- **Speichern:** `MediaStore.Images` (`Pictures/dimuli`) mit `IS_PENDING`, ab API 29 ohne Speicherberechtigung.
- **Teilen:** Datei in `cacheDir/share`, eigener `ShareProvider` (ContentProvider ohne AndroidX) und `ACTION_SEND`
  mit Chooser. Erfolgreiches Teilen zeigt keine Meldung, nur Fehler.
- **Build:** Der Gradle-Task `buildRust` ruft `cargo ndk --platform 33 … build -p dimuli-app --lib --release` auf und legt
  `libdimuli.so` unter `android/app/src/main/jniLibs/` ab; Gradle baut daraus die APK. Er setzt
  `ANDROID_JAR=$ANDROID_HOME/platforms/android-37.0/android.jar`, sonst sucht Slints Build-Skript `platforms/android-33`.
  `-PrustAbis=x86_64` baut nur für den Emulator. Prüfskripte: `android/scripts/check-apk.sh` (16-KB-Alignment,
  keine Berechtigungen, SDK-Stufen) und `android/scripts/smoke.sh` (installieren, starten, Zurück).
- Ziel-ABIs: `x86_64` (Emulator) und `arm64-v8a` (Geräte).

### Android-Versionen
| Einstellung | Wert |
|---|---|
| `minSdk` | **33** (Android 13) |
| `targetSdk` / `compileSdk` | 37 (Android 17, neueste stabile API-Stufe) |
| NDK | r30 (`30.0.16248370`) |
| Rust-Build | `cargo ndk --platform 33 …` |

Was daraus folgt:
- Photo Picker (`MediaStore.ACTION_PICK_IMAGES`, ab API 33 im System), MediaStore ohne Speicherberechtigung (ab API 29)
  und Teilen über den eigenen `ShareProvider` sind auf allen Zielgeräten vorhanden. Fallbacks für ältere Versionen sind nicht nötig.
- **Edge-to-Edge auf allen Versionen:** Ab Android 15 ist es Pflicht; auf Android 13/14 schaltet Kotlin es selbst ein
  (`setDecorFitsSystemWindows(false)`), damit die App überall gleich aussieht. Die App zeichnet unter Status- und
  Navigationsleiste. Slint 1.18 liest die Insets der Systemleisten und des Display-Ausschnitts selbst und stellt sie als
  `Window.safe-area-insets` bereit; `ui/app.slint` rückt den Inhalt damit ein.
- **16-KB-Speicherseiten:** Native Bibliotheken müssen 16-KB-aligned sein (Pflicht bei Google Play, ab Android 15
  auf Geräten möglich). NDK r28+ erledigt das standardmäßig; geprüft wird es mit `zipalign -c -P 16` bzw. im APK-Analyzer.
- **Predictive Back:** In der Kotlin-Activity wird der Zurück-Callback (`OnBackInvokedCallback`, ab API 33; im Manifest
  `android:enableOnBackInvokedCallback="true"`) an Rust weitergereicht: Im Editor geht es zurück zur Startansicht, auf der Startansicht wird die App geschlossen.
  Rust registriert den Callback erst nach `slint::android::init`, weil Slint ab API 34 einen eigenen registriert.
- HEIC/HEIF, WebP und (wo das Gerät es kann) AVIF wandelt Kotlin beim Kopieren mit `ImageDecoder` in JPEG bzw. PNG (bei
  Transparenz) um; Rust liest nur JPEG und PNG.
- **Lifecycle:** Konfigurationsänderungen (Drehen, Dunkelmodus, Schriftgröße …) erzeugen keine neue Activity (`configChanges`).
  Beendet das System die App mit offener Collage, zeigt der nächste Start einen Hinweis; eine Persistenz der Collage gibt es in v1 nicht.
- Emulator-Images (x86_64): Android 13 (API 33) als Minimum, Android 16 (API 36) und die neueste Version (API 37) für den Test „bis latest“.

## Umsetzung in Meilensteinen

- [x] 0. **Plan festhalten:** Diesen Plan als `docs/PLAN.md` im Projekt ablegen und in `CLAUDE.md` darauf verweisen.
  Erledigte Meilensteine werden dort abgehakt.
- [x] 1. **Werkzeuge einrichten** (alles in WSL)
    - [x] Android cmdline-tools in `~/Android/Sdk`, dazu platform-tools, emulator, `platforms;android-37.0`,
      `build-tools;37.0.0`, NDK r30 (`ndk;30.0.16248370`)
    - [x] Emulator-Images Android 16 (API 36) und Android 17 (API 37), x86_64; AVDs `dimuli_api36` und `dimuli_api37`
    - [x] `rustup target add x86_64-linux-android aarch64-linux-android`, `cargo install cargo-ndk`
    - [x] `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `ANDROID_NDK_HOME` und `PATH` in `~/.zshrc` eintragen (manuell)
    - [x] Benutzer zur Gruppe `kvm` hinzufügen (`sudo usermod -aG kvm $USER`), danach `wsl --shutdown` (manuell)
    - [x] Emulator-Start prüfen: `emulator -accel-check`, `emulator -avd dimuli_api36`
- [x] 2. **Workspace und collage-core** mit Unit-Tests: Baum, Vorlagen, Geometrie, Ziehen der Trennlinien, Cover-Transformation, Tauschen.
- [x] 3. **Desktop-UI mit Slint:** Bilder laden, Editor, Trennlinien, Verschieben/Zoomen, Tauschen, Einstellungen.
- [x] 4. **Export-Renderer** in collage-core mit Test (Pixelvergleich am Testbild); auf dem Desktop als Datei speichern.
- [x] 5. **Android-Schale:** Gradle-Projekt, Kotlin-Activity, JNI-Bridge, Photo Picker, MediaStore, Teilen; Lauf im Emulator
  (API 33, 36, 37; neues AVD `dimuli_api33`) und auf echten Geräten mit Android 14 und Android 16.
- [x] 6. **Feinschliff:** Lifecycle (Pause/Resume, Zustand halten), Performance mit großen Bildern, Fehlermeldungen in der UI.
    - [x] Review-Hinweise aus M3: Snapshot-Allokationen pro Zeigerereignis reduziert (Gesten-Schnellpfad aktualisiert nur
      geänderte Zeilen), kein 1-Frame-Flackern nach Formatwechsel (Leinwandgröße und Zeilen im selben Update),
      Vorschau wird einmal direkt in den Pixelbuffer kopiert, `DECODE_MAX_PIXELS` für Geräte auf 64 MP gesenkt,
      zusätzlich `DECODE_MAX_ALLOC` 256 MiB.
    - [x] Review-Hinweise aus M4: Render-Tests nutzen Ausgabegrößen bis 64 px; Desktop-„Teilen“ meldet
      „nicht verfügbar“, ohne vorher zu rendern.
- [x] 7. **CLAUDE.md aktualisieren:** `cargo-apk`-Befehle durch den `cargo ndk` + Gradle-Ablauf ersetzen und die Struktur ergänzen.
- [x] 8. **Mehr als 9 Bilder:** Raster-Generator in `templates`, der für jede Bildanzahl ein Layout erzeugt;
  `MAX_IMAGES` auf 36 angehoben (6x6; darüber verdecken die Trennlinien-Griffe zu viel der Felder). Speicher und Bedienbarkeit
  mit vielen Bildern im Emulator prüfen.

## Wichtige Abhängigkeiten
`slint` (+ `slint-build`), `image`, `tiny-skia`, `jni`, `android_logger`, `log`, `thiserror`; nur auf dem Desktop: `env_logger`.

Desktop-Bildauswahl ohne Dateidialog: Bilder werden als Argumente übergeben (`cargo run -p dimuli-app -- a.jpg b.jpg`).
Grund: `rfd` bräuchte unter WSL GTK3 bzw. ein xdg-Portal.

### Lizenz Slint
dimuli nutzt Slint unter der **Slint Royalty-free License 2.0**. Bedingung: Die App zeigt eine Slint-Attribution
(Widget `AboutSlint` oder „Made with Slint“-Badge), in dimuli auf dem Startbildschirm bzw. in „Über“.

## Verifikation
- `cargo test -p collage-core`: Vorlagen, Geometrie (keine Überlappung, Abstände stimmen), Ziehgrenzen,
  Cover-Begrenzung, Export-Pixeltest.
- `cargo run -p dimuli-app`: kompletter Ablauf auf dem Desktop (Bilder wählen, anordnen, exportieren, Datei öffnen).
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Emulator (API 33, API 36 und neueste API 37): APK mit `adb install` installieren und durchspielen. Zusätzlich prüfen: Inhalte
  werden nicht von System-Leisten verdeckt, Zurück-Geste funktioniert, 16-KB-Alignment besteht. Ablauf: Photo Picker mit 3–5 Bildern, Trennlinien ziehen,
  zoomen, tauschen, in die Galerie speichern (in Google Fotos prüfen), Teilen-Dialog öffnen,
  App pausieren und fortsetzen. Viele Bilder (12 und 25 im Emulator, 36 auf dem Desktop). Große Fotos (50/80 MP) sowie HEIC/WebP laden und speichern; Hinweis nach einem Neustart
  durch das System prüfen. Logs mit `adb logcat -s dimuli`.
