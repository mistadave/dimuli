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
   (Photo Picker, MediaStore, Teilen,               Layout, Geometrie, Zuschnitt,
    Insets, Zurück-Geste)                           Export-Renderer
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
| `layout` | `LayoutNode`, `Split`, Zellrechtecke berechnen, Trennlinien finden und ziehen (abstandsbewusst: gleiche `ratio`s ergeben gleich große Felder) |
| `templates` | Vorlagen je Bildanzahl: ausgewogene Raster (Zeilen/Spalten) plus „großes Bild links/oben“ für 3–5 |
| `collage` | `Collage` (Gesamtmodell) und alle Bearbeitungsoperationen |
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
    pub fn cell_at(&self, point: Vec2) -> Option<CellId>; // Hit-Test für Tauschen
}

pub fn source_crop(image: Size, cell: Size, t: CellTransform) -> Rect; // crop-Modul
pub fn render(collage: &Collage, sources: &[ImageSource], opts: ExportOptions) -> Result<Vec<u8>, CoreError>;
```

## dimuli-app

| Datei | Aufgabe |
|---|---|
| `lib.rs` | `run(platform)`: UI erstellen, Controller verdrahten; `android_main` ruft `run` mit `AndroidPlatform` |
| `main.rs` | Desktop-Einstieg: `run(DesktopPlatform)`; wählt standardmäßig winit mit Software-Renderer (WSLg-OpenGL-Problem), `SLINT_BACKEND` überschreibt das |
| `state.rs` | `AppState { screen, collage, images, busy: Option<Busy>, message, templates, canvas, insets, confirm_back, gesture }` samt allen Zustandsübergängen (Gesten-Zustandsmaschine `Gesture`, Laden, Export, Zurück, Stil); reines Rust, ohne Fenster testbar |
| `images.rs` | Laden im Hintergrund-Thread (Dekodieren über `collage_core::decode_unoriented`, danach Vorschau ≤ `PREVIEW_MAX_EDGE` verkleinern und orientieren; Fehler sind `CoreError`); `ImageStore`: `ImageId → { path, preview: slint::Image, preview_size }`, `sources(&Collage)` liefert die `ImageSource`s für den Export |
| `controller.rs` | Dünn: verbindet UI-Callbacks und `PlatformEvent`s mit `AppState`-Übergängen und stößt danach `refresh` an; `post()` bringt Aktionen aus anderen Threads in den UI-Thread (Controller per `thread_local` auffindbar) |
| `view.rs` | Reine Umrechnung Collage-Einheiten ↔ Pixel (1.0 = Canvas-Breite in px), `source-clip-*`, Trennlinien-Touchflächen (≥ 48 px), Wheel-Zoom; `Models::snapshot/apply` schreibt `CellView`/`DividerView`/`TemplateView` in die Slint-Models (nur geänderte Zeilen) |
| `texts.rs` | Vom Rust-Code erzeugte deutsche Meldungen (Texte der `.slint`-Dateien: `ui/texts.slint`) |
| `ui/` | `app.slint` (Einstieg), `backend.slint` (`global Backend`: alle Properties/Callbacks zwischen UI und Rust), `theme.slint`, `texts.slint`, `types.slint` (Structs), je eine Komponente pro Datei |
| `platform/mod.rs` | `trait Platform`, `enum PlatformEvent`, `post_event()` |
| `platform/desktop.rs`, `platform/android.rs` | Implementierungen |

### Plattform-Schnittstelle

```rust
pub trait Platform {
    fn pick_images(&self, max: usize);                  // → PlatformEvent::ImagesPicked
    fn save_to_gallery(&self, jpeg: Vec<u8>);          // → PlatformEvent::Saved
    fn share(&self, jpeg: Vec<u8>);                    // → PlatformEvent::Shared
    fn close_app(&self);                               // Zurück auf dem Startbildschirm beendet die App
}

pub enum PlatformEvent {
    ImagesPicked(Vec<PathBuf>),                        // Kopien im App-Cache; leer = abgebrochen
    PickFailed(String),                                // Meldung für den Benutzer (z. B. Desktop ohne Argumente)
    Saved(Result<(), String>),
    Shared(Result<(), String>),
    InsetsChanged { top: f32, bottom: f32, left: f32, right: f32 },
    BackPressed,
}
```

Desktop (`DesktopPlatform`): `pick_images` liefert die beim Start übergebenen Pfade (`cargo run -p dimuli-app -- a.jpg b.jpg`,
alle Pfade; der Controller kürzt auf `MAX_IMAGES` und zeigt den Hinweis; ohne Argumente `PickFailed`). `max` ist nur ein Hinweis.
`close_app` beendet auf dem Desktop die Slint-Event-Loop. `save_to_gallery` schreibt `dimuli-<Unix-Sekunden>.jpg` ins Arbeitsverzeichnis
(nie überschreibend: bei Kollision `-1`, `-2`, …; der volle Pfad wird geloggt); `share` meldet „nicht verfügbar“.

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

### Bildschirme

1. **Start:** Button „Bilder wählen“.
2. **Editor:** Canvas, darunter Vorlagenleiste; Einstellungen (Format, Abstand, Radius, Farbe) als ausklappbares Panel;
   Aktionen „Speichern“ und „Teilen“ in der oberen Leiste.

Export ist kein eigener Bildschirm, sondern ein Ladezustand (`busy`) über dem Editor.
Zurück im Editor → Start (mit Rückfrage, da die Collage verloren geht).
Zurück auf dem Startbildschirm (auch während des Ladens) ist nicht behandelt (`BackOutcome::NotHandled`);
der Controller ruft dann `platform.close_app()` auf.

## Abläufe

**Bilder wählen:** Start → `platform.pick_images(MAX_IMAGES)` → `ImagesPicked(paths)` → Hintergrund-Thread liest
Bildgröße + erzeugt Vorschaubild → Controller erstellt `Collage` mit erster passender Vorlage → Editor.

**Bearbeiten (synchron, UI-Thread):** Geste in Slint → Callback → `collage.drag_divider/pan/zoom/swap` →
`view::update(...)` → Slint zeichnet neu. Keine Threads, keine Kopien der Bilder.

**Exportieren:** `AppState::begin_export` (nur im Editor und wenn nicht `busy`) setzt `busy = Exporting`, beendet laufende Gesten und liefert einen `ExportJob` (geklonte `Collage` + `ImageSource`s aus dem `ImageStore`) →
Thread `exporter`: `render()` (Panic wird per `catch_unwind` zum Fehler) → `post` → `AppState::finish_export` beendet `busy` (Ende des Ladezustands ist das Render-Ende, nicht die Antwort der Plattform) →
bei Erfolg `platform.save_to_gallery(bytes)` bzw. `share(bytes)` → `Saved/Shared` → Meldung; bei Fehler wird der `CoreError` geloggt und eine deutsche Meldung gezeigt
(Quellbild fehlt/unlesbar vs. allgemein). Während des Exports schluckt das Overlay alle Eingaben und die Zurück-Geste ist `Handled` (ohne Wirkung).

## Grenzen und Konstanten (v1)

| Konstante | Wert | Grund |
|---|---|---|
| `MAX_IMAGES` | 9 | Vorlagen gibt es für 1–9; mehr Bilder per Raster-Generator folgen in Meilenstein 8 |
| `PREVIEW_MAX_EDGE` | 2048 px | Speicher in der UI begrenzen |
| `EXPORT_LONG_EDGE` | 3000 px | Gute Qualität bei vertretbarer Dateigröße |
| `DECODE_MAX_PIXELS` | 100 MP | Schutz vor manipulierten/riesigen Bildern |
| `MIN_CELL_SIZE` | 0.05 | Feld wird beim Ziehen nicht schmaler als 5 % der Collage-Breite |
| `MAX_SPACING` / `MAX_PADDING` / `MAX_CORNER_RADIUS` | 0.05 / 0.1 / 0.1 | Grenzen für den Stil, damit auch 9 Felder sichtbar bleiben |
| `MAX_ZOOM` | 5.0 | Sinnvolle Obergrenze für Pinch-Zoom |
| Export-Format | JPEG, Qualität 90 | Galerie- und Teilen-kompatibel |

## Bewusst nicht in v1

Persistenz/Projekte speichern, Undo/Redo, Text/Sticker, Filter, freie Anordnung, mehr als 9 Bilder (Meilenstein 8),
Übersetzungen (nur vorbereitet). Die Architektur verbaut nichts davon: Undo z. B. wäre ein Stapel von
`Collage`-Klonen im Controller.
