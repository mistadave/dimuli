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
| 3 | Core rechnet in **normierten Koordinaten** (0–1) | Gleiche Logik für Bildschirm und Export, unabhängig von Pixelgrößen |
| 4 | **Eine Zuschnitt-Funktion** für Vorschau *und* Export | Export sieht garantiert aus wie die Vorschau |
| 5 | Layout als **Binärbaum aus Splits** | Jede Trennlinie = ein `ratio`; Vorlagen und Ziehen werden trivial |
| 6 | Hintergrundarbeit nur in **kurzlebigen Threads** mit eigenen Daten | Kein Shared State, kein Async-Runtime nötig |
| 7 | Plattform hinter **einem Trait** + Ergebnis als `PlatformEvent` | Desktop zum Entwickeln, Android nur als dünne Hülle |
| 8 | Keine Persistenz in v1 | Collage lebt nur während der Sitzung; spart Speicherformat und Migration |

## collage-core

Reine Rust-Logik ohne UI- oder Plattformabhängigkeit. Module:

| Modul | Inhalt |
|---|---|
| `geometry` | `Size`, `Vec2`, `Rect` (normiert, `f32`) |
| `layout` | `LayoutNode`, `Split`, Zellrechtecke berechnen, Trennlinien finden und ziehen |
| `templates` | Vorlagen je Bildanzahl |
| `collage` | `Collage` (Gesamtmodell) und alle Bearbeitungsoperationen |
| `crop` | Zuschnitt: aus Bildgröße, Zellgröße und `CellTransform` das sichtbare Quellrechteck |
| `render` | Export: Bilder dekodieren, zuschneiden, zusammensetzen, als JPEG/PNG kodieren |
| `error` | `CoreError` (`thiserror`) |

### Datenmodell

```rust
pub struct Collage {
    pub aspect: AspectRatio,          // Square, Portrait4x5, Story9x16, Landscape16x9
    pub style: Style,                 // spacing, padding, corner_radius, background: Rgba
    pub layout: LayoutNode,
    pub cells: Vec<Cell>,             // index = CellId
}

pub enum LayoutNode {
    Cell(CellId),
    Split { dir: SplitDir, ratio: f32, first: Box<LayoutNode>, second: Box<LayoutNode> },
}

pub struct Cell {
    pub image: Option<ImageId>,       // None = leeres Feld (Platzhalter)
    pub transform: CellTransform,
}

pub struct CellTransform {
    pub zoom: f32,                    // ≥ 1.0; 1.0 = Bild füllt Feld genau ("cover")
    pub offset: Vec2,                 // Verschiebung, so begrenzt, dass keine Lücke entsteht
}
```

Der Core kennt Bilder nur als `ImageId` + Pixelgröße (`ImageMeta { id, width, height }`), nie als Pixeldaten –
außer im `render`-Modul, das Dateipfade bekommt.

### Wichtigste Operationen

```rust
impl Collage {
    pub fn new(images: &[ImageMeta], template: &Template) -> Self;
    pub fn apply_template(&mut self, template: &Template);         // Bilder bleiben in Reihenfolge erhalten
    pub fn cell_rects(&self) -> Vec<(CellId, Rect)>;              // inkl. spacing/padding
    pub fn dividers(&self) -> Vec<Divider>;                       // Ziehflächen für die UI
    pub fn drag_divider(&mut self, id: DividerId, delta: f32);    // klemmt auf Mindestgröße
    pub fn pan(&mut self, cell: CellId, delta: Vec2);
    pub fn zoom(&mut self, cell: CellId, factor: f32, focus: Vec2);
    pub fn swap(&mut self, a: CellId, b: CellId);
    pub fn set_aspect(&mut self, aspect: AspectRatio);
    pub fn cell_at(&self, point: Vec2) -> Option<CellId>;         // Hit-Test für Tauschen
}

pub fn source_crop(image: ImageMeta, cell: Size, t: CellTransform) -> Rect; // crop-Modul
pub fn render(collage: &Collage, sources: &[ImageSource], opts: ExportOptions) -> Result<Vec<u8>, CoreError>;
```

## dimuli-app

| Datei | Aufgabe |
|---|---|
| `lib.rs` | `run(platform)`: UI erstellen, Controller verdrahten; `android_main` ruft `run` mit `AndroidPlatform` |
| `main.rs` | Desktop-Einstieg: `run(DesktopPlatform)` |
| `state.rs` | `AppState { screen, collage: Option<Collage>, images: ImageStore, busy: bool }` |
| `images.rs` | `ImageStore`: `ImageId → { path, meta, preview: slint::Image }`; lädt Vorschaubilder im Hintergrund |
| `controller.rs` | Einziger Ort, der `AppState` ändert: UI-Callbacks und `PlatformEvent`s behandeln, View-Models aktualisieren |
| `view.rs` | `AppState` → Slint-Structs (`CellView`, `DividerView`) umwandeln, inkl. Pixelumrechnung |
| `platform/mod.rs` | `trait Platform`, `enum PlatformEvent` |
| `platform/desktop.rs`, `platform/android.rs` | Implementierungen |

### Plattform-Schnittstelle

```rust
pub trait Platform {
    fn pick_images(&self, max: usize);                  // → PlatformEvent::ImagesPicked
    fn save_to_gallery(&self, jpeg: Vec<u8>);          // → PlatformEvent::Saved
    fn share(&self, jpeg: Vec<u8>);                    // → PlatformEvent::Shared
}

pub enum PlatformEvent {
    ImagesPicked(Vec<PathBuf>),                        // Kopien im App-Cache
    Saved(Result<(), String>),
    Shared(Result<(), String>),
    InsetsChanged { top: f32, bottom: f32, left: f32, right: f32 },
    BackPressed,
}
```

Alle Methoden kehren sofort zurück. Ergebnisse kommen asynchron als `PlatformEvent` über
`slint::invoke_from_event_loop` zum Controller.

### Bildschirme

1. **Start:** Button „Bilder wählen“.
2. **Editor:** Canvas, darunter Vorlagenleiste; Einstellungen (Format, Abstand, Radius, Farbe) als ausklappbares Panel;
   Aktionen „Speichern“ und „Teilen“ in der oberen Leiste.

Export ist kein eigener Bildschirm, sondern ein Ladezustand (`busy`) über dem Editor.
Zurück im Editor → Start (mit Rückfrage, da die Collage verloren geht).

## Abläufe

**Bilder wählen:** Start → `platform.pick_images(MAX_IMAGES)` → `ImagesPicked(paths)` → Hintergrund-Thread liest
Bildgröße + erzeugt Vorschaubild → Controller erstellt `Collage` mit erster passender Vorlage → Editor.

**Bearbeiten (synchron, UI-Thread):** Geste in Slint → Callback → `collage.drag_divider/pan/zoom/swap` →
`view::update(...)` → Slint zeichnet neu. Keine Threads, keine Kopien der Bilder.

**Exportieren:** Controller klont `Collage` + Liste der Original-Pfade → Thread: `render()` → JPEG-Bytes →
`invoke_from_event_loop` → `platform.save_to_gallery(bytes)` bzw. `share(bytes)` → `Saved/Shared` → Meldung.

## Grenzen und Konstanten (v1)

| Konstante | Wert | Grund |
|---|---|---|
| `MAX_IMAGES` | 9 | Vorlagen gibt es für 1–9; größer später per automatischem Raster |
| `PREVIEW_MAX_EDGE` | 2048 px | Speicher in der UI begrenzen |
| `EXPORT_LONG_EDGE` | 3000 px | Gute Qualität bei vertretbarer Dateigröße |
| `DECODE_MAX_PIXELS` | 100 MP | Schutz vor manipulierten/riesigen Bildern |
| `MIN_CELL_FRACTION` | 0.1 | Feld wird beim Ziehen nicht kleiner als 10 % der Split-Fläche |
| `MAX_ZOOM` | 5.0 | Sinnvolle Obergrenze für Pinch-Zoom |
| Export-Format | JPEG, Qualität 90 | Galerie- und Teilen-kompatibel |

## Bewusst nicht in v1

Persistenz/Projekte speichern, Undo/Redo, Text/Sticker, Filter, freie Anordnung, mehr als 9 Bilder,
Übersetzungen (nur vorbereitet). Die Architektur verbaut nichts davon: Undo z. B. wäre ein Stapel von
`Collage`-Klonen im Controller.
