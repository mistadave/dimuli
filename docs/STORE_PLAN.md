# Plan: dimuli im Google Play Store veröffentlichen

Stand: 16.11.2026. Entwurf, noch nichts davon umgesetzt. Die Play-Regeln ändern sich laufend; vor jedem Schritt
die offizielle Play-Console-Hilfe prüfen.

## Ausgangslage

| Thema | Stand in dimuli | Für den Store |
|---|---|---|
| `applicationId` | `ch.staeheli.dimuli` | bleibt für immer die Store-Identität |
| `targetSdk` / `minSdk` | 37 / 33 | Play verlangt seit 31.08.2026 mindestens API 36 → erfüllt |
| Berechtigungen | keine (von `check-apk.sh` geprüft) | sehr einfache Datenschutzangaben |
| 16-KB-Seiten | erfüllt (`check-apk.sh`) | Pflicht für native Bibliotheken |
| Build | nur Debug-APK, Debug-Schlüssel | Release-**AAB**, eigener Upload-Schlüssel |
| Version | `versionCode 1`, `versionName 0.1.0` | jede Store-Version braucht einen höheren `versionCode` |
| Icon | Platzhalter (Phase 5, D-03) | endgültiges Icon + 512×512-PNG |
| Slint-Lizenz | Royalty-free 2.0, `AboutSlint` auf dem Info-Bildschirm (Zahnrad auf dem Start → Einstellungen → Info) | Attribution vorhanden; bleibt Pflicht |
| Lizenz eigener Code | Apache-2.0 (`LICENSE`, `Cargo.toml`), festgelegt am 16.11.2026 | erledigt |
| Remote-Repo | keines | nötig nur, falls Quellcode/Datenschutzseite öffentlich sein sollen |

## Entscheidungen vorab (Besitzer)

1. **Kontoart:** Privatkonto (einfacher, aber Pflicht-Test mit 12 Testern über 14 Tage) oder
   Organisationskonto (kein Pflicht-Test, braucht aber eine Firma mit D-U-N-S-Nummer).
2. **Name im Store:** „dimuli“ allein oder mit Zusatz („dimuli – Collage“); vorher Markenkonflikte prüfen.
3. **Sprachen:** Die App ist deutsch. Store-Eintrag nur Deutsch oder zusätzlich Englisch
   (dann auch App-Texte in `texts.rs` übersetzen?).
4. **Preis und Länder:** Vorschlag kostenlos, ohne Werbung, ohne In-App-Käufe, alle Länder.
5. **Quellcode öffentlich?** Bestimmt, wo Datenschutzerklärung und Lizenzhinweise liegen (z. B. GitHub Pages).
6. **Tester:** Wer sind die 12 Tester für den geschlossenen Test (nur bei Privatkonto)?

## Meilensteine

### S1. Entwicklerkonto
- Konto in der Play Console anlegen, einmalige Gebühr (25 USD), Identität per Ausweis verifizieren.
- Kontaktdaten: Die E-Mail-Adresse wird im Store öffentlich angezeigt, ggf. eine eigene Adresse anlegen.
- Bei Organisationskonto: D-U-N-S-Nummer beantragen (kann Wochen dauern).

### S2. Release-Build
- Upload-Schlüssel erzeugen (`keytool`), **außerhalb des Repos** sichern (Passwortmanager + Backup).
  Ohne ihn keine Updates (Zurücksetzen über den Google-Support ist möglich, aber mühsam).
- Gradle: `signingConfig` für `release`; Pfad und Passwörter aus `~/.gradle/gradle.properties` oder
  Umgebungsvariablen, nie im Repo.
- `bundleRelease` erzeugt ein **AAB** mit `arm64-v8a` (Gerät) und `x86_64`; Play liefert pro Gerät nur die passende ABI.
- Rust-Release-Profil prüfen (`lto`, `codegen-units = 1`, `strip`) für eine kleinere `libdimuli.so`;
  vorher/nachher Größe und Startzeit messen.
- Native Debug-Symbole für Play bereitstellen, damit Abstürze in Android vitals lesbar sind.
- `check-apk.sh` erweitern oder `bundletool` nutzen, damit Alignment, Berechtigungen und SDK-Stufen auch für das
  AAB geprüft werden.
- `versionCode`-Schema festlegen (z. B. fortlaufend), `versionName` auf `1.0.0`.
- **Play App Signing** aktivieren (Standard): Google verwaltet den App-Signaturschlüssel, wir nur den Upload-Schlüssel.

### S3. App fertigstellen
- Endgültiges Icon (adaptiv: Vorder- und Hintergrund) einsetzen, dazu 512×512-PNG für den Store.
- Anzeige „Über“ bzw. Lizenzen: **erledigt** (Stand: Info-Bildschirm über Zahnrad → Einstellungen mit `AboutSlint`, Version und allen
  Lizenztexten; Rust-Crates per `scripts/update-licenses.sh` (`cargo-about`), Slint/Skia/Kotlin von Hand in
  `crates/dimuli-app/licenses/`, siehe `docs/ARCHITECTURE.md`).
- Erledigt (16.11.2026): eigener Code unter Apache-2.0 (`LICENSE`, `license = "Apache-2.0"` in `Cargo.toml`).
- Abschließender Gerätetest auf API 33/34/36/37 (Emulator) und auf dem OnePlus; Speicher mit 36 Bildern.

### S4. Rechtliches und Formulare in der Play Console
- **Datenschutzerklärung** (öffentliche URL, Pflicht für alle Apps): dimuli sammelt und sendet nichts,
  hat keine Internet-Berechtigung, Bilder bleiben auf dem Gerät. Schweizer DSG und DSGVO kurz abdecken.
- **Datensicherheit** (Data safety): „Es werden keine Daten erhoben oder geteilt.“
- **Inhaltseinstufung** (IARC-Fragebogen): ohne problematische Inhalte → niedrigste Stufe.
- **Zielgruppe:** ab 13 bzw. 18 Jahren, nicht für Kinder (sonst gilt die strengere Familienrichtlinie).
- **Werbung:** nein. **App-Zugriff:** keine Anmeldung nötig.
- Weitere Erklärungen nach Aufforderung der Console (z. B. Regierungs-App: nein, Finanzfunktionen: nein).

### S5. Store-Eintrag
- Titel (max. 30 Zeichen), Kurzbeschreibung (max. 80), ausführliche Beschreibung (max. 4000).
- Grafiken: App-Icon 512×512, Feature-Grafik 1024×500, mindestens 2 Handy-Screenshots
  (z. B. Startbildschirm, Editor mit Collage, Einstellungen, Export); optional Tablet-Screenshots.
- Kategorie: Fotografie. Kontakt-E-Mail, optional Website.

### S6. Testen in der Play Console
1. **Interner Test** (bis 100 Tester, sofort verfügbar): AAB hochladen, Installation aus dem Store prüfen,
   Pre-Launch-Bericht (automatische Tests auf Google-Geräten) auswerten.
2. **Geschlossener Test** (nur Privatkonto Pflicht): mindestens 12 Tester, durchgehend 14 Tage eingeschrieben.
   Die Frist beginnt erst nach Googles Freigabe der Testversion. Tester um echtes Feedback bitten; Rückmeldungen und
   Fixes dokumentieren, weil der Antrag auf Produktionszugang danach fragt.
3. Fehler aus Tests und Android vitals beheben, neue Testversionen hochladen (`versionCode` erhöhen).

### S7. Veröffentlichung
- Produktionszugang beantragen (Fragebogen zum Test), Prüfung durch Google abwarten (einige Tage).
- Produktion mit **gestaffeltem Rollout** (z. B. 20 % → 100 %), Absturzrate in Android vitals beobachten.
- Danach: jährliche Anhebung von `targetSdk` (Play-Vorgabe jeweils bis Ende August) einplanen.

## Zeitbedarf (grob)

| Schritt | Dauer |
|---|---|
| S1 Konto + Verifizierung | 1–3 Tage (Organisation: Wochen) |
| S2–S5 Build, App, Formulare, Eintrag | 1–2 Wochen Arbeit |
| S6 geschlossener Test | mind. 14 Tage + Freigaben |
| S7 Prüfung Produktion | einige Tage |
| **Gesamt** | **etwa 4–6 Wochen** |

## Risiken

- **12 Tester über 14 Tage** zu finden ist für Einzelentwickler die größte Hürde; Tester müssen ein Google-Konto
  haben und eingeschrieben bleiben.
- **Upload-Schlüssel verlieren** → Updates blockiert, bis der Support ihn zurücksetzt.
- **Speicher auf schwachen Geräten:** Play verteilt an viele Geräte; Absturzrate in Android vitals beobachten
  (Dekodiergrenzen 64 MP / 256 MiB gelten bereits).
- **Namensgleichheit** mit bestehenden Apps oder Marken.
- **Regeländerungen** bei Google (z. B. zusätzliche Prüfungen beim geschlossenen Test); vor dem Start prüfen.
