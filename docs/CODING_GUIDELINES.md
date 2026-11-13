# Coding Guidelines

Verbindliche Regeln für alle, die an dimuli Code schreiben, insbesondere für den Coding Agent.
Bei Konflikten gilt: diese Datei > allgemeine Konventionen. Abweichungen nur mit Begründung im Code-Kommentar.

## 1. Grundsätze

- **Einfach vor clever.** Die einfachste Lösung, die die Anforderung erfüllt. Keine Abstraktionen „für später“.
- **Kleine Schritte.** Eine Änderung = ein Zweck. Kein Umbau von Code, der nichts mit der Aufgabe zu tun hat.
- **Bestehendes wiederverwenden.** Vor dem Schreiben neuer Funktionen prüfen, ob es sie in `collage-core` schon gibt.
- **Sprache:** Bezeichner, Kommentare, Commit-Messages und Log-Meldungen auf Englisch.
  UI-Texte für den Benutzer auf Deutsch (siehe Abschnitt 6).

## 2. Projektstruktur und Schichten

| Ort | Inhalt | Darf abhängen von |
|---|---|---|
| `crates/collage-core` | Layout-Baum, Vorlagen, Geometrie, Transformationen, Export-Renderer | nur reine Rust-Crates (kein Slint, kein JNI) |
| `crates/dimuli-app/src` | App-Zustand, Verbindung Core ↔ UI | `collage-core`, `slint` |
| `crates/dimuli-app/src/platform` | Bildauswahl, Speichern, Teilen, Insets, Back | Plattform-APIs (JNI; Desktop: Kommandozeilen-Argumente, Dateisystem) |
| `crates/dimuli-app/ui` | `.slint`-Dateien: nur Darstellung und Gesten-Events | – |
| `android/` | Kotlin-Activity, Manifest, Gradle | – |

- Abhängigkeiten zeigen nur nach unten: `ui` → `app` → `core`. `core` kennt weder UI noch Plattform.
- Plattformcode nur hinter dem `Platform`-Trait; `#[cfg(target_os = "android")]` ausschließlich in `platform/`.
- Logik gehört nicht in `.slint`-Dateien und nicht in Kotlin. Kotlin enthält nur, was ohne Activity nicht geht.

## 3. Rust

### Stil
- `cargo fmt` (Standardkonfiguration) und `cargo clippy --all-targets -- -D warnings` müssen sauber sein.
- Edition 2024, stable Toolchain. Keine Nightly-Features.
- Namen: `snake_case` für Funktionen/Module, `PascalCase` für Typen, `SCREAMING_SNAKE_CASE` für Konstanten.
  Sprechende Namen statt Abkürzungen (`divider_index`, nicht `di`).
- Funktionen kurz halten (Richtwert < 50 Zeilen). Lieber eine benannte Hilfsfunktion als ein Kommentarblock.
- Sichtbarkeit minimal: standardmäßig privat, `pub(crate)` vor `pub`.

### Typen und Daten
- Newtypes für IDs (`struct CellId(u32)`, `struct ImageId(u32)`), keine nackten `usize` über Modulgrenzen.
- Geometrie in einem einheitlichen Koordinatensystem: `collage-core` rechnet in **Collage-Einheiten (1.0 = Collage-Breite)**; normierte Koordinaten (0.0–1.0) nur für Bildausschnitte (`crop`)
  relativ zur Collage; erst die UI bzw. der Export rechnet in Pixel um.
- `f32` für Geometrie. Vergleiche in Tests mit Toleranz (`approx` bzw. `(a - b).abs() < EPS`), nie mit `==`.
- Ungültige Zustände unmöglich machen: `enum` statt Bool-Flags, Konstruktoren mit Validierung statt öffentlicher Felder,
  wo Invarianten gelten (z. B. `ratio` immer in `[MIN_RATIO, 1 - MIN_RATIO]`).

### Fehlerbehandlung
- Kein `unwrap()`, `expect()`, `panic!`, `unreachable!` oder Index-Zugriff `v[i]` mit möglichem Out-of-Bounds
  im Produktionscode. Erlaubt nur in Tests und in `build.rs`.
- `collage-core`: eigene Fehler-Enums mit `thiserror`, pro Modul oder Crate (`CoreError`).
- `dimuli-app`: Fehler an der Grenze zur UI in eine Benutzermeldung übersetzen, nicht verschlucken.
  Jeder Fehler wird mit `log::error!` geloggt.
- Kein `Box<dyn Error>` und kein `anyhow` in `collage-core`; in `dimuli-app` ist `anyhow` nur nach Rückfrage erlaubt.

### unsafe und FFI
- `unsafe` nur in `platform/android.rs` (JNI). Jeder `unsafe`-Block bekommt einen `// SAFETY:`-Kommentar.
- **Einzige Ausnahme für `#[allow]` auf Lints:** das Modul `ui` in `dimuli-app/src/lib.rs`, das den von
  `slint::include_modules!()` generierten Code einbindet. Generierter Code ist nicht änderbar; handgeschriebener Code
  bleibt voll gelintet. Das `allow` enthält nur die Lints, die der generierte Code wirklich verletzt.
- Native JNI-Einstiegspunkte (`extern "system" fn Java_...`) fangen Panics mit `std::panic::catch_unwind` ab,
  bevor sie die Grenze erreichen.
- Aufrufe aus Kotlin-Callbacks nie direkt auf UI-Objekte zugreifen lassen: immer über
  `slint::invoke_from_event_loop` in den UI-Thread wechseln.
- JNI-Referenzen nicht über den Aufruf hinaus halten (lokale Referenzen); für langlebige Objekte `GlobalRef`.

### Performance und Speicher
- Bilder nie mehrfach in voller Auflösung im Speicher halten. UI arbeitet mit Vorschaubildern (max. 2048 px Kante).
- Teure Arbeit (Dekodieren, Export) nicht im UI-Thread: eigener Thread, Ergebnis via `invoke_from_event_loop`.
- Keine Allokationen pro Frame in Gesten-Handlern, wenn vermeidbar; Models gezielt aktualisieren statt neu aufbauen.

## 4. Slint

- Eine Komponente pro Datei unter `ui/`, Einstieg `ui/app.slint`. Dateinamen `kebab-case.slint`.
- `.slint`-Dateien enthalten keine Geschäftslogik: Daten kommen als Properties/Models aus Rust,
  Aktionen gehen als `callback` zurück (`callback divider-moved(int, length)`).
- Farben, Abstände und Schriftgrößen zentral in `ui/theme.slint` (`global Theme`), keine Magic Numbers in Komponenten.
- Touch-Ziele mindestens 48 px (Android-Richtlinie).
- Safe-Area-Insets (Edge-to-Edge) immer berücksichtigen: Inhalte über die `safe-area`-Properties einrücken.

## 5. Kotlin / Android

- Kotlin nur für: Activity-Lifecycle, Photo Picker, MediaStore, Teilen-Intent, Insets, Back-Callback.
- Jede Kotlin-Funktion, die Rust aufruft oder von Rust aufgerufen wird, steht in `MainActivity.kt` im Abschnitt
  `// region JNI bridge`; Signaturen auf beiden Seiten identisch dokumentieren.
- Keine zusätzlichen Berechtigungen im Manifest ohne Rückfrage (minSdk 33 braucht keine Speicherberechtigung).
- Keine AndroidX-/Drittbibliotheken ohne Rückfrage.

## 6. UI-Texte und Bedienung

- Texte kurz und auf Deutsch, an einer Stelle gesammelt (vorbereitet für Übersetzung via `@tr("...")`).
- Jede länger dauernde Aktion (Export, Laden) zeigt einen Fortschritt oder Ladezustand.
- Fehlermeldungen sagen, was passiert ist und was der Benutzer tun kann – keine technischen Details.

## 7. Tests

- Jede öffentliche Funktion in `collage-core` hat Unit-Tests im selben Modul (`#[cfg(test)] mod tests`).
- Testnamen beschreiben das Verhalten: `dragging_divider_respects_min_cell_size`.
- Grenzfälle immer testen: 1 Bild, maximale Bildanzahl, extreme Seitenverhältnisse (Panorama, Hochformat), Abstand 0.
- Fehler zuerst als fehlschlagenden Test reproduzieren, dann beheben.
- Export-Tests vergleichen Pixel an definierten Stellen (kein Abgleich ganzer Referenzbilder).
- Testbilder klein halten (≤ 64 px) und im Test erzeugen statt als Datei einzuchecken.

## 8. Abhängigkeiten

- Neue Crates, Gradle-Abhängigkeiten oder Features nur nach Rückfrage.
- Vor dem Hinzufügen prüfen: aktiv gepflegt, Lizenz MIT/Apache-2.0 (oder kompatibel), baut für `aarch64-linux-android`.
- `default-features = false` und nur benötigte Features aktivieren (z. B. bei `image` nur `jpeg`, `png`).
- Versionen im Workspace-`Cargo.toml` unter `[workspace.dependencies]` zentral pflegen.

## 9. Dokumentation

- `///`-Doc-Kommentare für alle `pub`-Elemente in `collage-core`: was es tut, Einheiten (Collage-Einheiten/normiert/px), Fehlerfälle.
- Kommentare erklären das **Warum**, nicht das Was.
- Architektur- oder Planänderungen in `docs/PLAN.md` nachführen; erledigte Meilensteine dort abhaken.
- Ändern sich Befehle oder Setup, `CLAUDE.md` aktualisieren.

## 10. Git

- Commits nur auf ausdrückliche Aufforderung.
- Commit-Messages auf Englisch, Imperativ, Betreffzeile ≤ 72 Zeichen
  (`Add divider drag to layout tree`), bei Bedarf Leerzeile + Begründung.
- Ein Commit = eine logische Änderung; vor dem Commit müssen fmt, clippy und test grün sein.
- Keine generierten Dateien, Build-Artefakte, Schlüssel oder `local.properties` einchecken.

## 11. Checkliste vor Abschluss einer Aufgabe

- [ ] `cargo fmt --all`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Neue Logik hat Tests, Grenzfälle sind abgedeckt
- [ ] Kein `unwrap`/`expect`/`panic` im Produktionscode
- [ ] UI-Änderung per `cargo run` auf dem Desktop geprüft; Plattformänderung im Emulator (`dimuli_api36`) geprüft
- [ ] `docs/PLAN.md` / `CLAUDE.md` aktualisiert, falls betroffen
- [ ] Kurze Zusammenfassung an den Benutzer: was geändert, was geprüft, was offen ist
