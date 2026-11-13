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
│       └── src/platform/{mod.rs, desktop.rs, android.rs}
└── android/                   # Gradle-Projekt: Manifest, Kotlin-Activity, FileProvider
```

### collage-core
- **Layout als Binärbaum:** `Node = Split { dir: H|V, ratio: f32, a, b } | Cell(CellId)`.
  Eine Trennlinie entspricht einem `Split`. Beim Ziehen ändert sich `ratio`; eine Mindestgröße pro Feld wird eingehalten.
- **Vorlagen:** `templates(count) -> Vec<Template>`, mehrere Varianten je Anzahl 1–9 (darüber ein automatisches Raster).
- **Geometrie:** `compute_cells(tree, canvas_size, spacing, padding) -> Vec<(CellId, Rect)>` und `compute_dividers(...)`
  für die Ziehflächen.
- **Bild im Feld:** `CellContent { image: ImageId, scale: f32, offset: Vec2 }`. Das Bild füllt das Feld immer
  vollständig („cover“). Zoom und Verschieben werden so begrenzt, dass keine Lücken entstehen.
- **Aktionen:** Bilder tauschen, Vorlage wechseln (Bilder bleiben erhalten), Seitenverhältnis ändern.
- **Export-Renderer:** Bilder mit `image` dekodieren (inkl. EXIF-Orientierung) und mit `tiny-skia` zusammensetzen
  (Clip-Pfad mit Eckenradius, Hintergrundfarbe). Ausgabe als JPEG/PNG-Bytes mit konfigurierbarer langer Kante
  (Standard 3000 px).
- **Speicher:** Für die UI dient eine verkleinerte Vorschau (max. 2048 px). Der Export dekodiert die Bilder
  nacheinander in voller Auflösung.

### dimuli-app
- **Slint-UI:** Startansicht (Bilder wählen) → Editor mit Canvas, Vorlagenleiste und Einstellungen
  (Abstand, Radius, Farbe, Format) → Export (Speichern/Teilen).
- Rust berechnet die Feld- und Trennlinien-Rechtecke und gibt sie als Slint-Models (`[CellData]`, `[DividerData]`) an die UI.
  Slint zeichnet nur und meldet Gesten zurück.
- **Gesten:** `TouchArea` zum Verschieben und Ziehen der Trennlinien. Getauscht wird per Long-Press und Ziehen auf ein
  anderes Feld (Hit-Test in Rust). Für Pinch-Zoom kommt die Pinch-Geste der aktuellen Slint-Version zum Einsatz,
  falls vorhanden, sonst ein Zoom-Slider.
- **Plattform-Trait:** `pick_images()`, `save_to_gallery(bytes)` und `share(bytes)`, jeweils mit einem
  asynchronen Ergebnis über `slint::invoke_from_event_loop`.
  - Desktop: `rfd` als Dateidialog, Speichern in eine Datei, Teilen als No-op.
  - Android: JNI-Aufrufe in die Kotlin-Activity (siehe unten).

### Android-Integration
`cargo-apk` reicht hier nicht: Bildauswahl und Teilen brauchen `onActivityResult` bzw. Intents, und
`NativeActivity` leitet diese nicht an nativen Code weiter. Deshalb:
- Gradle-Projekt mit `MainActivity : NativeActivity` in Kotlin. Slint läuft über `backend-android-activity`
  (Feature `native-activity`).
- **Bildauswahl:** Photo Picker (`PickMultipleVisualMedia`). Kotlin kopiert die ausgewählten URIs ins Cache-Verzeichnis
  und ruft die native Funktion `onImagesPicked(paths)` auf. Rust übernimmt den Rest.
- **Speichern:** `MediaStore.Images` (`Pictures/dimuli`), ab API 29 ohne Speicherberechtigung.
- **Teilen:** Datei im Cache, `FileProvider` und `ACTION_SEND`.
- **Build:** `cargo ndk` legt `libdimuli.so` unter `android/app/src/main/jniLibs/` ab, Gradle baut daraus die APK.
- Ziel-ABIs: `x86_64` (Emulator) und `arm64-v8a` (Geräte).

### Android-Versionen
| Einstellung | Wert |
|---|---|
| `minSdk` | **36** (Android 16) |
| `targetSdk` / `compileSdk` | 37 (Android 17, neueste stabile API-Stufe) |
| NDK | r30 (`30.0.16248370`) |
| Rust-Build | `cargo ndk --platform 36 …` |

Was daraus folgt:
- Photo Picker, MediaStore ohne Speicherberechtigung und `FileProvider`-Teilen sind auf allen Zielgeräten vorhanden.
  Fallbacks für ältere Versionen sind nicht nötig.
- **Edge-to-Edge ist ab Android 15 Pflicht:** Die App zeichnet unter Status- und Navigationsleiste.
  Kotlin liest die System-Insets aus (`WindowInsets`) und gibt sie per JNI an Rust weiter. Die Slint-UI
  rückt ihre Inhalte entsprechend ein (`safe-area`-Properties).
- **16-KB-Speicherseiten:** Native Bibliotheken müssen 16-KB-aligned sein (Pflicht bei Google Play, ab Android 15
  auf Geräten möglich). NDK r28+ erledigt das standardmäßig; geprüft wird es mit `zipalign -c -P 16` bzw. im APK-Analyzer.
- **Predictive Back:** In der Kotlin-Activity wird der Zurück-Callback (`OnBackPressedCallback`/`OnBackInvokedCallback`)
  an Rust weitergereicht: Im Editor geht es zurück zur Startansicht, auf der Startansicht wird die App geschlossen.
- Emulator-Image: Android 16 (API 36) x86_64; zusätzlich ein Image der neuesten Version für den Test „bis latest“.

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
- [ ] 2. **Workspace und collage-core** mit Unit-Tests: Baum, Vorlagen, Geometrie, Ziehen der Trennlinien, Cover-Transformation, Tauschen.
- [ ] 3. **Desktop-UI mit Slint:** Bilder laden, Editor, Trennlinien, Verschieben/Zoomen, Tauschen, Einstellungen.
- [ ] 4. **Export-Renderer** in collage-core mit Test (Pixelvergleich am Testbild); auf dem Desktop als Datei speichern.
- [ ] 5. **Android-Schale:** Gradle-Projekt, Kotlin-Activity, JNI-Bridge, Photo Picker, MediaStore, Teilen; Lauf im Emulator.
- [ ] 6. **Feinschliff:** Lifecycle (Pause/Resume, Zustand halten), Performance mit großen Bildern, Fehlermeldungen in der UI.
- [ ] 7. **CLAUDE.md aktualisieren:** `cargo-apk`-Befehle durch den `cargo ndk` + Gradle-Ablauf ersetzen und die Struktur ergänzen.

## Wichtige Abhängigkeiten
`slint` (+ `slint-build`), `image`, `tiny-skia`, `jni`, `android_logger`, `log`, `thiserror`; nur auf dem Desktop: `rfd`.

## Verifikation
- `cargo test -p collage-core`: Vorlagen, Geometrie (keine Überlappung, Abstände stimmen), Ziehgrenzen,
  Cover-Begrenzung, Export-Pixeltest.
- `cargo run -p dimuli-app`: kompletter Ablauf auf dem Desktop (Bilder wählen, anordnen, exportieren, Datei öffnen).
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Emulator (API 36 und neueste API): APK mit `adb install` installieren und durchspielen. Zusätzlich prüfen: Inhalte
  werden nicht von System-Leisten verdeckt, Zurück-Geste funktioniert, 16-KB-Alignment besteht. Ablauf: Photo Picker mit 3–5 Bildern, Trennlinien ziehen,
  zoomen, tauschen, in die Galerie speichern (in Google Fotos prüfen), Teilen-Dialog öffnen,
  App pausieren und fortsetzen. Logs mit `adb logcat -s dimuli`.
