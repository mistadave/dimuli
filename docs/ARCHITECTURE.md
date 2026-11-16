# Architektur

Bewusst minimal: zwei Crates, ein Zustand, ein Datenfluss. Neue Schichten oder Muster nur, wenn ein konkretes
Problem sie verlangt.

## Überblick

```
┌──────────────────────────── dimuli-app ────────────────────────────┐
│                                                                    │
│   ui/*.slint  ──callbacks──▶  Controller  ──mutiert──▶  AppState   │
│       ▲                          │    ▲                    │       │
│       └───── View-Models ◀───────┘    │                    │       │
│                                       │ PlatformEvent      │       │
│   platform/ (Android | Desktop) ──────┘                    │       │
│        ▲  pick / save / share                              │       │
└────────┼───────────────────────────────────────────────────┼───────┘
         │                                                   ▼
   Kotlin-Activity                                  collage-core (reine Logik)
   (Photo Picker, Umwandlung HEIC/WebP,             Layout, Geometrie, Zuschnitt,
    MediaStore, Teilen, Edge-to-Edge, Zurück-Geste) Export-Renderer
```

## Leitentscheidungen

| # | Entscheidung | Grund |
|---|---|---|
| 1 | **Ein Zustand** (`AppState`) im UI-Thread, nur der Controller verändert ihn | Keine Synchronisation, ein Ort für die Wahrheit |
| 2 | **Einbahn-Datenfluss:** Event → Controller → Core → View-Models → UI | Nachvollziehbar, UI bleibt dumm |
| 3 | Core rechnet in **Collage-Einheiten** (1.0 = Collage-Breite) | Gleiche Logik für Bildschirm und Export, unabhängig von Pixelgrößen; Abstände sind in x und y gleich groß |
| 4 | **Eine Zuschnitt-Funktion** für Vorschau *und* Export | Export sieht garantiert aus wie die Vorschau |
| 5 | Layout als **Binärbaum aus Splits** | Jede Trennlinie = ein `ratio`; Vorlagen und Ziehen werden trivial |
| 6 | Hintergrundarbeit nur in **kurzlebigen Threads** mit eigenen Daten | Kein Shared State, kein Async-Runtime nötig |
| 7 | Plattform hinter **einem Trait** + Ergebnis als `PlatformEvent` | Desktop zum Entwickeln, Android nur als dünne Hülle |
| 8 | Keine Persistenz in v1 | Collage lebt nur während der Sitzung; spart Speicherformat und Migration |

## collage-core

Reine Rust-Logik ohne UI- oder Plattformabhängigkeit. Module:

| Modul | Inhalt |
|---|---|
| `geometry` | `Size`, `Vec2`, `Rect` (Collage-Einheiten, `f32`) |
| `layout` | `LayoutNode`, `Split`, Zellrechtecke berechnen, Trennlinien finden und ziehen (abstandsbewusst: gleiche `ratio`s ergeben gleich große Felder); `max_cells_per_line` (Felder in der vollsten Zeile/Spalte) |
| `templates` | Vorlagen je Bildanzahl: für 1–9 unverändert (ausgewogene Raster plus „großes Bild links/oben“ für 3–5), ab 10 Bildern nur gleichmäßige Raster aus den zwei ausgewogensten Zeilenzahlen, höchstens 6 Vorlagen |
| `collage` | `Collage` (Gesamtmodell) und alle Bearbeitungsoperationen; `effective_style` (Stil für Geometrie, Vorschau und Export) |
| `crop` | Zuschnitt: aus Bildgröße, Zellgröße und `CellTransform` das sichtbare Quellrechteck |
| `decode` | `decode_unoriented(path)` (Dekodieren mit Limits `DECODE_MAX_*`, liefert Bild + EXIF-Orientierung) und `decode_image(path)` (zusätzlich orientiert); geteilt von Export und App-Vorschau |
| `render` | Export: `compose` (Bilder einzeln über `decode_unoriented` dekodieren, den `placements()`-Crop in gespeicherte Pixel zurückrechnen, ohne Kopie als View direkt auf Zellgröße verkleinern und erst die kleine Zelle orientieren – Spitzenspeicher ≈ dekodiertes Bild + Zelle; 16-Bit/Float/Graustufen+Alpha brauchen zusätzlich eine 8-Bit-RGBA-Kopie des sichtbaren Ausschnitts; Quellen mit Alpha werden premultipliziert skaliert, damit keine Farbsäume entstehen; Eckenradius per antialiasierter Abdeckungsmaske, Kantenrundung der Pixelrechtecke ohne Lücken) und `render` (JPEG-Kodierung); `ExportOptions { long_edge, jpeg_quality }` wird geklemmt (`1..=MAX_EXPORT_LONG_EDGE` = 8192, Qualität 1–100); Hintergrund mit Alpha wird über Weiß gemischt |
| `error` | `CoreError` (`thiserror`) |

### Datenmodell

```rust
pub struct Collage {
    aspect: AspectRatio,              // Square, Portrait4x5, Story9x16, Landscape16x9
    style: Style,                     // spacing, padding, corner_radius, background: Rgba
    layout: LayoutNode,
    cells: Vec<Cell>,                 // index = CellId; Felder privat, Zugriff über Methoden
}

pub enum LayoutNode {
    Cell(CellId),
    Split(Box<Split>),                // Split { dir: SplitDir, ratio: f32, first, second }
}

pub struct Cell {
    pub image: ImageMeta,             // jedes Feld hat genau ein Bild
    pub transform: CellTransform,
}

pub struct CellTransform {
    zoom: f32,                        // 1.0..=MAX_ZOOM; 1.0 = Bild füllt Feld genau ("cover")
    center: Vec2,                     // Mitte des sichtbaren Ausschnitts in Bildkoordinaten (0..1);
                                      // bleibt bei Größenänderung des Felds sinnvoll
}
```

Der Core kennt Bilder nur als `ImageId` + Pixelgröße (`ImageMeta { id, width, height }`), nie als Pixeldaten –
außer in `decode` und `render`, die Dateipfade (`ImageSource { id, path }`) bekommen.

### Wichtigste Operationen

```rust
impl Collage {
    pub fn new(images: &[ImageMeta], template: &Template) -> Result<Self, CoreError>;
    pub fn apply_template(&mut self, template: &Template) -> Result<(), CoreError>; // Bilder bleiben erhalten
    pub fn placements(&self) -> Vec<CellPlacement>;   // je Feld: Rechteck + Bildausschnitt (für UI und Export)
    pub fn dividers(&self) -> Vec<Divider>;           // Ziehflächen für die UI
    pub fn drag_divider(&mut self, id: DividerId, delta: Vec2) -> Result<(), CoreError>; // klemmt auf Mindestgröße
    pub fn pan(&mut self, cell: CellId, delta: Vec2) -> Result<(), CoreError>;
    pub fn zoom(&mut self, cell: CellId, factor: f32, focus: Vec2) -> Result<(), CoreError>;
    pub fn swap(&mut self, a: CellId, b: CellId) -> Result<(), CoreError>;
    pub fn set_aspect(&mut self, aspect: AspectRatio);
    pub fn set_style(&mut self, style: Style);       // klemmt auf gültige Werte
    pub fn effective_style(&self) -> Style;          // Stil für Geometrie, Vorschau und Export; style() bleibt der gespeicherte Wert
    pub fn cell_at(&self, point: Vec2) -> Option<CellId>; // Hit-Test für Tauschen
}

pub fn source_crop(image: Size, cell: Size, t: CellTransform) -> Rect; // crop-Modul
pub fn render(collage: &Collage, sources: &[ImageSource], opts: ExportOptions) -> Result<Vec<u8>, CoreError>;
```

## dimuli-app

| Datei | Aufgabe |
|---|---|
| `lib.rs` | `run(platform)`: UI erstellen, Controller verdrahten |
| `main.rs` | Desktop-Einstieg: `run(DesktopPlatform)`; wählt standardmäßig winit mit Software-Renderer (WSLg-OpenGL-Problem), `SLINT_BACKEND` überschreibt das |
| `state.rs` | `AppState { screen, collage, images, busy: Option<Busy> (Laden, Export, `Picking`: Bildauswahl offen, unsichtbar; `Copying`: Kotlin kopiert, `Decoding`: Rust dekodiert; beide unsichtbar, Overlay erst nach 250 ms als `CopyingShown` bzw. `Loading`), message, templates, canvas_area, canvas, confirm_back, gesture, share_supported }` samt allen Zustandsübergängen (Gesten-Zustandsmaschine `Gesture`, Laden, Export, Zurück, Stil); Meldungen haben eine Art (Fehler/Hinweis/Erfolg) und eine Sequenznummer; `canvas` wird aus `canvas_area` und dem Collage-Format berechnet; reines Rust, ohne Fenster testbar |
| `images.rs` | Laden im Hintergrund-Thread (Dekodieren über `collage_core::decode_unoriented`, danach Vorschau ≤ `preview_max_edge(n)` verkleinern und orientieren, mit nur einer Kopie des Pixelpuffers; Fehler sind `CoreError`); `ImageStore`: `ImageId → { path, preview: slint::Image, preview_size }`, `sources(&Collage)` liefert die `ImageSource`s für den Export |
| `controller.rs` | Dünn: verbindet UI-Callbacks und `PlatformEvent`s mit `AppState`-Übergängen und stößt danach `refresh` an; `post()` bringt Aktionen aus anderen Threads in den UI-Thread (Controller per `thread_local` auffindbar); `update_gesture` aktualisiert bei Gesten nur die betroffenen Zeilen; ein `Timer` blendet Erfolgsmeldungen aus |
| `view.rs` | Reine Umrechnung Collage-Einheiten ↔ Pixel (1.0 = Canvas-Breite in px), `source-clip-*`, Trennlinien-Touchflächen (≥ 48 px), Wheel-Zoom; `fit_canvas` (größte Canvas mit Collage-Format in der verfügbaren Fläche, ungültige Werte ergeben 0); Gesten-Schnellpfad (bei geänderter Meldung oder anderem Bildschirm voller Refresh); `Models::snapshot/apply` schreibt `CellView`/`DividerView`/`TemplateView` in die Slint-Models (nur geänderte Zeilen) |
| `licenses.rs` | Lizenzhinweise der Info-Seite: bettet `licenses/rust-crates.txt` (generiert) und `licenses/other-components.txt` (von Hand) per `include_str!` ein und parst sie (`parse`, panikfrei) zu `LicenseEntry { name, packages, text }`; `Models::attach` übergibt sie als Modell an `Backend.licenses`, zusammen mit `CARGO_PKG_VERSION` |
| `settings.rs` | `AppSettings` (Standardwerte des Benutzers, `Default` = Werkseinstellung): Format, Abstand, Rand, Ecken, Hintergrund (Index in `BACKGROUND_PRESETS`), Export-Langkante (`EXPORT_SIZE_PRESETS` 2000/3000/4000/6000 px) und JPEG-Qualität (`JPEG_QUALITY_PRESETS` 85/90/95); Setter liefern „geändert“ (= Speichern nötig); `to_text`/`parse` (tolerant, nie Fehler); `load`/`save` (atomar: Temp-Datei + `rename`); Details siehe „Einstellungen“ unten |
| `texts.rs` | Vom Rust-Code erzeugte deutsche Meldungen (Texte der `.slint`-Dateien: `ui/texts.slint`) |
| `ui/` | `app.slint` (Einstieg), `backend.slint` (`global Backend`: alle Properties/Callbacks zwischen UI und Rust), `theme.slint`, `texts.slint`, `types.slint` (Structs), je eine Komponente pro Datei |
| `platform/mod.rs` | `trait Platform`, `enum PlatformEvent`, `post_event()` |
| `platform/desktop.rs` | `DesktopPlatform` |
| `platform/android.rs` | `android_main` (`#[unsafe(no_mangle)]`, ruft `run` mit `AndroidPlatform`), `AndroidPlatform` (JNI-Aufrufe in `MainActivity`), JNI-Natives per `native_method!` + `RegisterNatives`; einzige Datei mit `unsafe` |
| `platform/native_events.rs` | Reine Abbildung der Kotlin-Ergebnisse auf `PlatformEvent`s, auf dem Host getestet |

### Plattform-Schnittstelle

```rust
pub trait Platform {
    fn pick_images(&self, max: usize);                  // → PlatformEvent::ImagesPicked
    fn save_to_gallery(&self, jpeg: Vec<u8>);          // → PlatformEvent::Saved
    fn share_supported(&self) -> bool;                 // vor dem Rendern geprüft
    fn restored_after_close(&self) -> bool;            // System hat den Prozess mit offener Collage beendet
    fn set_collage_open(&self, open: bool);            // für den Neustart-Merker
    fn share(&self, jpeg: Vec<u8>);                    // → PlatformEvent::Shared
    fn data_dir(&self) -> Option<PathBuf>;             // beschreibbares Verzeichnis für die Einstellungen
    fn close_app(&self);                               // Zurück auf dem Startbildschirm beendet die App
}

pub enum PlatformEvent {
    ImagesPicked { paths: Vec<PathBuf>, failed: usize }, // Kopien im App-Cache + Zahl der Fehlschläge; leer und 0 = abgebrochen
    PickFailed(String),                                // Meldung für den Benutzer (z. B. Desktop ohne Argumente)
    Saved(Result<(), String>),
    Shared(Result<(), String>),
    BackPressed,
}
```

Safe-Area-Insets (Systemleisten, Display-Ausschnitt) liefert Slint selbst als `Window.safe-area-insets`; `ui/app.slint` rückt den Inhalt
damit ein (Desktop: 0), die Plattform meldet keine Insets.

Desktop (`DesktopPlatform`): `pick_images` liefert die beim Start übergebenen Pfade (`cargo run -p dimuli-app -- a.jpg b.jpg`,
alle Pfade; der Controller kürzt auf `MAX_IMAGES` (36) und zeigt den Hinweis; ohne Argumente `PickFailed`). `max` ist nur ein Hinweis.
`close_app` beendet auf dem Desktop die Slint-Event-Loop. `save_to_gallery` schreibt `dimuli-<Unix-Sekunden>.jpg` ins Arbeitsverzeichnis
(nie überschreibend: bei Kollision `-1`, `-2`, …; der volle Pfad wird geloggt). `share_supported` ist `false`: „Teilen“ meldet
sofort „nicht verfügbar“, ohne vorher zu rendern. `restored_after_close` ist immer `false`.

Android (`AndroidPlatform`): `pick_images` öffnet den System-Photo-Picker (`ACTION_PICK_IMAGES`, höchstens `max` Bilder);
Kotlin kopiert die Auswahl nach `cacheDir/picked/img-<n>` (wird beim nächsten Auswählen bzw. Start gelöscht; bis zu 36 Kopien, gemessen ca. 48 MiB bei 25 Fotos mit 12 MP).
JPEG und PNG werden unverändert kopiert; alles andere (HEIC/HEIF, WebP, AVIF, wo das Gerät es kann) wandelt Kotlin dabei mit
`ImageDecoder` um: sRGB, aufrecht ausgerichtet, ohne EXIF, als JPEG (Qualität 95) bzw. PNG bei Transparenz. Bilder über 64 MP
oder mit einer Kante über 16 384 px werden beim Kopieren verkleinert (reicht der Speicher nicht, bis zu drei weitere Versuche mit
halber Größe). Fehlschläge zählt Kotlin und meldet sie mit `nativeImagesPicked(paths, failed)`.
`configChanges` deckt alle Konfigurationsänderungen ab, die Activity wird also z. B. beim Drehen nicht neu erstellt.
Beendet Android den Prozess, während eine Collage offen ist (`set_collage_open` → `onSaveInstanceState`), meldet
`restored_after_close` beim nächsten Start `true` und der Startbildschirm zeigt einen Hinweis; die Collage selbst wird in v1 nicht gespeichert.
`save_to_gallery` schreibt per MediaStore nach `Pictures/dimuli` (`IS_PENDING`, keine Berechtigung).
`share` schreibt `cacheDir/share/dimuli-collage.jpg` und öffnet über den eigenen `ShareProvider` den Chooser;
`Shared(Ok)` zeigt keine Meldung. `close_app` ruft `Activity.finish()` auf.

Die Kotlin-Logik (Umwandlung, Prüfungen im `ShareProvider`, Rücknahme beim MediaStore, Neustart-Merker) ist nur durch
Emulator- und Gerätetests abgedeckt, nicht durch `cargo test`.

Die Plattform liefert Ergebnisse mit `platform::post_event(event)` (beliebiger Thread); das nutzt
`invoke_from_event_loop` und findet den Controller über ein `thread_local`.

Alle Methoden kehren sofort zurück. Ergebnisse kommen asynchron als `PlatformEvent` über
`slint::invoke_from_event_loop` zum Controller.

### Gesten

Die Canvas-`TouchArea` meldet nur rohe Zeigerdaten in Canvas-Pixeln (`canvas-pressed/moved/released/long-pressed/scrolled/pinched`);
Rust entscheidet per `Gesture` (`Idle → Pressed → Panning | Swapping`, `Pinching`) und `cell_at`, was sie bedeuten.
Long-Press (500 ms) ist ein `Timer` in Slint; Bewegung über 10 px vor dem Ablauf macht aus dem Druck ein Verschieben.
Mausrad → `zoom` unter dem Zeiger, Pinch (`ScaleRotateGestureHandler`) → `zoom` um den Gestenmittelpunkt.
Trennlinien haben keine eigene `TouchArea`: Jede hat genau einen Griff mit quadratischer Zielfläche (Kantenlänge `Theme.touch-target`, 48 px,
zentriert auf dem Gap). Bei `press` prüft Rust zuerst die Griffe (bei Überlappung gewinnt der nächste Mittelpunkt) und startet
`Gesture::DraggingDivider`, sonst läuft die Zell-Logik. Die Griffe in `.slint` sind reine Darstellung.
Bei vielen Bildern überlappen die Griffe; in 16:9 können einzelne Felder ganz unter Griffen liegen (bei 25 Bildern etwa drei,
bewusst hingenommen, D-07): dort funktionieren Pinch-Zoom und das Ablegen beim Tauschen, aber nicht Verschieben oder das Starten eines Tauschs.

### Bildschirme

1. **Start:** Button „Bilder wählen“ und oben rechts ein Zahnrad (aus Formen gezeichnet, `gear-button.slint`; 48 px Touch-Ziel).
2. **Einstellungen (`Screen::Settings`):** feste Leiste (`back-bar.slint`: „‹ Zurück“, Titel), darunter eine `ScrollView`:
   „Standard für neue Collagen“ (Format, Abstand/Rand/Ecken, Hintergrund), „Export“ (Größe, Qualität), Zurücksetzen
   (einmalige Rückfrage, reiner UI-Zustand) und „Info“. Geteilte Komponenten mit dem Editor-Panel: `SliderRow`,
   `ChoiceRow`, `SwatchRow`. Buttons speichern sofort; Schieberegler melden `settings-committed` erst beim Loslassen
   (`ValueSlider.released`), der Controller (`edit_settings`/`commit_settings`) speichert dann einmal. Ausstehende Änderungen (`AppState.settings_unsaved`) werden auch beim Verlassen des Einstellungsbildschirms gespeichert (`refresh`), falls ein Loslassen nie ankam.
   Erreichbar nur vom Start ohne laufende Arbeit (`AppState::open_settings`); Zurück → Start (`Handled`).
3. **Info (`Screen::About`):** Zurück-Leiste, App-Name und Version, Kurzbeschreibung, das `AboutSlint`-Widget
   (Pflicht der Slint-Lizenz: erreichbar über Start → Zahnrad → Einstellungen → Info), Überschrift „Open-Source-Lizenzen“ und die
   Lizenzkarten; alles unter der festen Zurück-Leiste scrollt als eine `ScrollView` (Tippen klappt den vollen Text auf; welcher Eintrag offen ist, ist reiner UI-Zustand in
   `about-screen.slint`). Erreichbar nur von den Einstellungen ohne laufende Arbeit (`AppState::open_about`);
   Zurück (Button und Geste) führt zu den Einstellungen und ist `Handled`, die App schließt sich dabei nicht.
4. **Editor:** Canvas, darunter Vorlagenleiste; Einstellungen (Format, Abstand, Radius, Farbe) als ausklappbares Panel;
   Aktionen „Speichern“ und „Teilen“ in der oberen Leiste.

Lizenzdateien (`crates/dimuli-app/licenses/`, Format: je Lizenztext ein Abschnitt aus den Zeilen `@@@@ LICENSE <Name>`,
`@@@@ PACKAGES <a 1.0, b 2.0>`, `@@@@ TEXT` und dem vollen Text):
- `rust-crates.txt` wird mit `scripts/update-licenses.sh` aus `about.toml` und `about.hbs` per `cargo-about` erzeugt
  (nur Android-Ziele, ohne Build-/Dev-Abhängigkeiten) und eingecheckt; `--check` schlägt fehl, wenn die Datei veraltet ist.
  `cargo-about` ist nur ein Build-Werkzeug, keine Abhängigkeit der App.
- `other-components.txt` ist von Hand gepflegt: Slint (`LicenseRef-Slint-Royalty-free-2.0`, von `cargo-about` nicht
  lesbar), Skia samt der mitgelieferten Bibliotheken (FreeType, libjpeg-turbo, libpng, zlib, expat, Wuffs) und die
  Kotlin-Bibliotheken (`kotlin-stdlib`, `annotations`). Bei Updates von Slint, `skia-bindings` oder Gradle-Abhängigkeiten anpassen
  (das Skript warnt, wenn die Slint-Version nicht mehr passt).

Export ist kein eigener Bildschirm, sondern ein Ladezustand (`busy`) über dem Editor.
Zurück im Editor → Start (mit Rückfrage, da die Collage verloren geht).
Zurück auf dem Startbildschirm (auch während die Bildauswahl offen ist oder Bilder geladen werden) ist nicht behandelt (`BackOutcome::NotHandled`);
der Controller ruft dann `platform.close_app()` auf.

## Abläufe

**Bilder wählen:** Start → `AppState::begin_picking` zeigt sofort den Ladezustand (eine zweite Auswahl ist gesperrt; Abbrechen
oder ein Fehler beenden ihn, D-08) → `platform.pick_images(MAX_IMAGES)` → `ImagesPicked { paths, failed }` → Hintergrund-Thread liest
Bildgröße + erzeugt Vorschaubild → Controller erstellt `Collage` mit erster passender Vorlage → Editor.

**Bearbeiten (synchron, UI-Thread):** Geste in Slint → Callback → `collage.drag_divider/pan/zoom/swap` →
`view::update(...)` → Slint zeichnet neu. Keine Threads, keine Kopien der Bilder.

**Exportieren:** `AppState::begin_export` (nur im Editor und wenn nicht `busy`) setzt `busy = Exporting`, beendet laufende Gesten und liefert einen `ExportJob` (geklonte `Collage` + `ImageSource`s aus dem `ImageStore`) →
Thread `exporter`: `render()` (Panic wird per `catch_unwind` zum Fehler) → `post` → `AppState::finish_export` beendet `busy` (Ende des Ladezustands ist das Render-Ende, nicht die Antwort der Plattform) →
bei Erfolg `platform.save_to_gallery(bytes)` bzw. `share(bytes)` → `Saved/Shared` → Meldung (Teilen zeigt bei Erfolg keine Meldung, D-11); bei Fehler wird der `CoreError` geloggt und eine deutsche Meldung gezeigt
(Quellbild fehlt/unlesbar vs. allgemein). Während des Exports schluckt das Overlay alle Eingaben und die Zurück-Geste ist `Handled` (ohne Wirkung).

### Einstellungen (App-Standardwerte)

`AppState.settings: AppSettings` gilt für **neue** Collagen (`finish_loading` setzt Format und Stil) und für **jeden
Export** (`begin_export` kopiert Langkante und Qualität als `ExportJob.options`). Eine geöffnete Collage ändert sich
nie durch geänderte Einstellungen; Änderungen sind während `busy` gesperrt.

- **Speicherort** (`Platform::data_dir`, Datei `settings.txt`): Android `AndroidApp::internal_data_path()`
  (app-privat, keine Berechtigung, bei Deinstallation gelöscht); Desktop `$XDG_CONFIG_HOME/dimuli`, sonst
  `$HOME/.config/dimuli` (nur absolute Pfade). Laden einmalig beim Start (fehlend/unlesbar/zu groß → Werkseinstellung,
  nur Log). Die Datei-IO läuft bewusst im UI-Thread (winzige Datei, nur bei Tippen oder Loslassen). Speichern atomar (`settings.txt.tmp` → `rename`); schlägt es fehl, Log + Meldung
  `texts::SETTINGS_SAVE_FAILED`, die Werte gelten bis zum App-Ende weiter.
- **Format** (Text, eine Zeile je Schlüssel, `version=1` muss die erste Zeile sein, sonst Werkseinstellung):

  ```text
  version=1
  aspect=1:1            # 1:1 | 4:5 | 9:16 | 16:9
  spacing=0             # Collage-Einheiten, 0..=MAX_SPACING
  padding=0.01          # 0..=MAX_PADDING
  corner_radius=0       # 0..=MAX_CORNER_RADIUS
  background=0          # Index in BACKGROUND_PRESETS
  export_long_edge=3000 # 2000 | 3000 | 4000 | 6000
  jpeg_quality=90       # 85 | 90 | 95
  ```

  Unbekannte Schlüssel werden ignoriert; ungültige oder außerhalb des Bereichs liegende Werte fallen je Schlüssel
  auf die Werkseinstellung zurück (kein Klemmen).
- **Schnittstelle für die Einstellungsseite:** `AppState::set_default_aspect/_spacing/_padding/_corner_radius/
  _background`, `set_export_size`, `set_export_quality` (Index bzw. Wert; Rückgabe `true` = gespeichert werden muss),
  `reset_settings_to_factory`. Der Controller (`change_settings`) speichert danach über die Plattform. Backend:
  Callbacks `default-aspect-selected`, `default-spacing-changed`, `default-padding-changed`,
  `default-corner-radius-changed`, `default-background-selected`, `export-size-selected`,
  `export-quality-selected`, `settings-reset` und Properties `default-*-index`/`default-spacing/-padding/-corner-radius`,
  `export-size-index`, `export-quality-index`.

## Grenzen und Konstanten (v1)

| Konstante | Wert | Grund |
|---|---|---|
| `MAX_IMAGES` | 36 | 6x6-Raster; darüber verdecken die 48-px-Griffe der Trennlinien auf dem Handy zu viel der Felder für Verschieben und Tauschen (D-06); der Photo Picker wird mit 36 geöffnet, der Desktop kürzt längere Argumentlisten mit Hinweis |
| `PREVIEW_MAX_EDGE` | 2048 px | Speicher in der UI begrenzen; bis 9 Bilder 2048 px, darüber `preview_max_edge(n)` = abgerundet 2048 · √(9/n) (36 Bilder: 1024 px), damit der Vorschauspeicher auf dem Stand von 9 Bildern bleibt |
| `FULL_STYLE_CELLS_PER_LINE` | 4 | Bis 4 Felder pro Zeile/Spalte gilt der Stil unverändert (1–9 Bilder pixelgleich); ab 5 wirken Abstand, Rand und Eckenradius mit dem Faktor 4/n (`Collage::effective_style`, Vorschau und Export gleich, D-01/D-02) |
| `EXPORT_LONG_EDGE` | 3000 px | Gute Qualität bei vertretbarer Dateigröße |
| `DECODE_MAX_PIXELS` | 64 MP | Schutz vor manipulierten/riesigen Bildern; 64 MP RGBA sind 256 MB, der Export braucht nur 3000 px, Android verkleinert größere Bilder schon beim Kopieren |
| `DECODE_MAX_EDGE` | 16 384 px | Längste erlaubte Kante |
| `DECODE_MAX_ALLOC` | 256 MiB | Obergrenze für den Dekodier-Puffer; begrenzt auch 16-Bit-Quellen |
| `MESSAGE_HIDE_AFTER` | 3 s | Erfolgsmeldungen verschwinden von selbst; Fehler und Hinweise bleiben |
| `MIN_CELL_SIZE` | 0.05 | Feld wird beim Ziehen nicht schmaler als 5 % der Collage-Breite; in 16:9 mit vielen Bildern und maximalem Stil können Felder schon darunter starten, ihre Trennlinien bleiben dann stehen |
| `MAX_SPACING` / `MAX_PADDING` / `MAX_CORNER_RADIUS` | 0.05 / 0.1 / 0.1 | Grenzen des gespeicherten Stils (Schieberegler); wirksam wird der Stil über `effective_style` |
| `MAX_ZOOM` | 5.0 | Sinnvolle Obergrenze für Pinch-Zoom |
| Export-Format | JPEG, Qualität 90 (einstellbar 85/90/95), Langkante 3000 px (einstellbar 2000–6000) | Galerie- und Teilen-kompatibel |

Überschreitet ein Bild eine der `DECODE_*`-Grenzen, sieht der Benutzer die Meldung „zu groß“ (nicht „unlesbar“).

## Bewusst nicht in v1

Persistenz/Projekte speichern, Undo/Redo, Text/Sticker, Filter, freie Anordnung,
Übersetzungen (nur vorbereitet). Die Architektur verbaut nichts davon: Undo z. B. wäre ein Stapel von
`Collage`-Klonen im Controller.
