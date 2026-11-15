## Review: Phase 8 – mehr als 9 Bilder (`git diff 5c2f109 -- crates android` plus untracked)

**Ergebnis:** ✅ freigegeben (0 kritisch, 0 Warnungen, 5 Hinweise)

### Automatische Prüfungen
| Prüfung | Ergebnis |
|---|---|
| `cargo fmt --all -- --check` | ✅ sauber |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ keine Warnungen |
| `cargo test --workspace` | ✅ 108 (collage-core) + 148 (dimuli-app) bestanden, 0 fehlgeschlagen |
| `cargo ndk -t x86_64 --platform 33 clippy -p dimuli-app --lib -- -D warnings -A clippy::missing_const_for_thread_local` | ✅ sauber |
| `cargo audit` | nicht ausgeführt (keine neue Abhängigkeit; `Cargo.toml` und `Cargo.lock` unverändert) |

### Prüfpunkte des Auftrags
**Effective Style (D-01..D-04): erfüllt.**
- Der grep trifft nur erlaubte Stellen:
  - `Style::sanitized`/`effective_style` (`collage.rs:271-273`)
  - `content_bounds` bekommt den Style als Parameter (`collage.rs:280`)
  - Tests
  - `render.rs:90`, das `effective_style()` aus `render.rs:84` nutzt
- `placements` (`collage.rs:285`), `dividers` (`:303`) und `drag_divider` (`:311`) holen den Style einmal und nutzen ihn für Bounds und Spacing. Der Drag-Clamp stimmt damit mit `placements` überein.
- Export-Radius (`render.rs:84`) und Preview-Radius (`view.rs:263`) nutzen `effective_style()`.
- Thumbnails nutzen `style_scale` (`view.rs:201`). Für ≤4 Zellen pro Linie ist der Faktor exakt 1.0.
- `style()` bleibt der gespeicherte Wert. Es wird nur in `state.rs:746` (Slider/`update_style`) und in Tests verwendet.
- `max_cells_per_line` hängt nur an der Baumstruktur. Das Ziehen ändert den Wert nicht (Test vorhanden).

**1-9-Identität (D-03): erfüllt.**
- Der `legacy_*`-Block in `templates.rs` entspricht `5c2f109`. Die einzigen Abweichungen sind die ersetzten Konstanten (`9`, `4`, `4 + 1`) und die Umbenennung auf `legacy_*`.
- Der 1-9-Zweig von `grid_shapes` ist unverändert. Der Zweig für >9 ist ein separates `if`.
- Zusätzliche Identitätstests:
  - `one_to_nine_geometry_equals_raw_style` (`collage.rs`): 4 Styles, alle Formate, alle Templates. Der Kommentar zur Bit-Identität ist vorhanden.
  - `template_previews_of_one_to_nine_are_unchanged` (`view.rs`)

**Generator und Cap (D-06): erfüllt.**
- `truncate(MAX_GRID_TEMPLATES)` greift nur bei `count > CLASSIC_MAX_IMAGES` (`templates.rs:101`).
- `MAX_IMAGES = 36` steht nur in `lib.rs`. `TooManyImages` und das leere `templates_for(37)` bleiben bestehen (Test vorhanden).
- Kein u8-Überlauf in den Tests:
  - `render.rs`: `(i % 12) * 20` ≤ 220
  - `state.rs`: `(i*7) as u8` und `(i*3) as u8` laufen mit `as` (wrappen, laufen nicht über). `255 - i as u8` ≤ 255 für i ≤ 35.

**`preview_max_edge`: erfüllt.**
- Die Berechnung läuft in f64. `count > 9` ergibt ein Verhältnis in (0,1), also einen Wert in 1..=2048. Kein Overflow, kein NaN, nie 0 (`.max(1)`).
- Das Budget ist im Doc-Kommentar dokumentiert (9 × 2048² Pixel). Der Test prüft `count·edge² ≤ Budget` für 10..=36 und die Monotonie.

**D-08: erfüllt.**
- Jedes Picker-Ergebnis beendet `Busy::Picking`:
  - Abbruch: leere Pfade → `pick_cancelled` (`controller.rs:239`)
  - `PickFailed` (auch JNI-Fehler, `android.rs:132-135` postet `pick_failed_event`) → `pick_failed`
  - Pfadliste: `begin_loading` akzeptiert `Picking` und setzt `Loading`
  - Kotlin-Fehler: `nativePickFailed`/`nativeImagesPicked(empty)` decken alle Pfade ab (`MainActivity.kt:99-103`, `:231-236`)
- `pick_images` hält den `borrow_mut` nur für das eine Statement. Der Plattformaufruf läuft ohne Borrow (`controller.rs:207-212`).
- Ein zweiter Tipp ist gesperrt: `begin_picking` gibt bei `busy.is_some()` oder `screen != Start` `false` zurück. Tests dazu sind vorhanden.
- Zurück-Geste: Während der System-Picker offen ist, geht sie an den Picker. Während Kotlin kopiert, ist sie `NotHandled` und beendet die App. Das war vorher genauso (siehe Hinweis 3).

**Sonstiges: erfüllt.**
- Kein `unwrap`/`expect`/`panic`/Indexzugriff im Produktionscode des Diffs.
- Neue pub-Elemente in collage-core (`style_scale`, `FULL_STYLE_CELLS_PER_LINE`, `effective_style`, `max_cells_per_line`, `MAX_IMAGES`) haben Doc-Kommentare. Die Einheiten stehen bei `effective_style` ("collage units") und `style_scale` (Faktor 0..=1) dabei.
- Render-Tests laufen mit ≤64 px und generierten Bildern.
- Deutsche Texte nur in `texts.rs`. Die Test-Assertion auf "Es werden nur die ersten 36 Bilder verwendet." ist ein Test-Literal.
- Keine neue Abhängigkeit. Kotlin und `platform/` sind seit `5c2f109` unverändert (`git diff --stat` leer).
- `docs/ARCHITECTURE.md` und `docs/PLAN.md` nennen bereits 36 und D-08.

### Befunde
#### 🔴 Kritisch
keine

#### 🟠 Warnung
keine

#### 🟡 Hinweis
1. **Test prüft die D-06-Zusage nur für das erste Template** – `crates/dimuli-app/src/view.rs` (`cells_keep_free_area_beside_the_handles_at_the_cap`, ca. Zeile 868-890)
   Der Test nimmt nur `templates_for(36)[0]` und fordert `worst >= 0.25`. D-06 begründet die 36 mit "mindestens 30 % frei in jedem Format". Gemessen sind 0.396, 0.448, 0.457 und 0.308 (16:9). Die Schwelle 0.25 lässt also Spielraum nach unten, obwohl die Begründung 0.30 nennt. Das ist keine Fehlfunktion.
   Vorschlag: Schwelle auf 0.30 anheben (16:9 liegt bei 0.308, das ist knapp) und über alle `templates_for(36)` schleifen. Oder den Kommentar anpassen ("regression guard, measured ≥ 0.308").

2. **Exakter Gleichheitsvergleich ohne Begründungskommentar** – `crates/collage-core/src/templates.rs` (`templates_for_one_to_nine_are_unchanged`)
   `assert_eq!(templates_for(count), legacy_templates_for(count))` vergleicht Layouts mit f32-Ratios exakt. Das ist hier gewollt (Bit-Identität). Die Guidelines verlangen aber einen Grund bei exakten Vergleichen, und in `collage.rs` und `view.rs` steht er.
   Vorschlag: Einzeiler ergänzen, z. B. `// Exact on purpose: the ratios are computed by the same formula, so equality proves 1-9 are unchanged.`

3. **Zurück-Geste während des Kopierens beendet die App** – `crates/dimuli-app/src/state.rs:423-435` (`request_back`)
   Bei `Busy::Picking` (Kotlin kopiert noch, Overlay sichtbar) liefert die Start-Seite `NotHandled`, und `close_app` schließt die App. Beim Laden (`Busy::Loading`) ist es dasselbe, also konsistent mit dem Zustand vor Phase 8, aber nun bei sichtbarem Overlay. Das ist kein Fehler.
   Vorschlag (optional): `Busy::Picking | Busy::Loading` in `request_back` wie `Exporting` als `Handled` behandeln. Dann beendet ein versehentliches Zurück den Kopiervorgang nicht.

4. **Kein Ausweg, falls nie eine Picker-Antwort kommt** – `crates/dimuli-app/src/state.rs:250-263`
   `Busy::Picking` sperrt die UI bis zu einer Antwort. Die Kotlin-Seite meldet in allen geprüften Pfaden zurück, und eine Wiederherstellung der Activity setzt den Zustand ohnehin zurück. Das Risiko hängt daher an einem Kotlin- oder Plattformfehler (Vermutung, kein belegter Fehler). Ein Timeout oder `resume`-Hook wäre eine Absicherung. Für v1 reicht die Beobachtung auf dem Gerät.

5. **Untracked `.codex/` im Arbeitsbaum** – Repo-Wurzel
   Gehört nicht zu Phase 8. Vor dem Commit nicht mit hinzufügen oder in `.gitignore` aufnehmen. Die `08-0x-SUMMARY.md` unter `.planning/` sind erwartungsgemäß neu.

### Positiv
- Eine einzige zentrale Funktion (`effective_style`) für Geometrie, Divider, Drag, Export und Preview. Die Konsistenz zwischen Preview und Export ist per Test abgesichert (`export_uses_the_effective_corner_radius`, `cell_views_use_the_effective_corner_radius`).
- Der Beweis der 1-9-Identität läuft über eine Referenzkopie von `5c2f109` und Bit-genaue Geometrie- und Thumbnail-Tests.
- Umfangreiche Grenztests für 10..=36 in allen Formaten bei Standard- und Maximal-Style (Überlappung, Lücken, Mindestgröße, Drag mit extremen Deltas). Der degenerierte 16:9-Fall bei 36 Bildern ist ausdrücklich dokumentiert und festgenagelt.
- Das Speicherbudget der Vorschau ist mathematisch begründet und per Test belegt.
- Der Picker-Zustandsautomat ist klein und hat Tests für alle Übergänge, auch dafür, dass fremde Busy-Zustände unberührt bleiben.

### Fazit
Die Änderung erfüllt alle Vorgaben aus D-01..D-08. fmt, clippy (Host und NDK) und alle Tests sind grün. Vor dem Commit ist nichts Zwingendes zu tun. Die Hinweise 1 bis 3 sind kleine Nachbesserungen, und `.codex/` sollte nicht mit committet werden.
