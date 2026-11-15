# Phase 8: More Than 9 Images - Discussion Log

**Date:** 2026-11-15
Human reference only; downstream agents read 08-CONTEXT.md.

## Areas offered
Obergrenze · Raster-Vorlagen ab 10 · Abstand & kleine Felder · Speicher & Testgrößen
**Selected:** Abstand & kleine Felder (others left to Claude's discretion)

## Abstand & kleine Felder
1. Abstand/Rand bei vielen Bildern — options: Regler bleibt, Wirkung skaliert (rec.) / Unverändert / Maximum je nach Anzahl begrenzen → **Regler bleibt, Wirkung skaliert**
2. Trennlinien bei kleinen Feldern — options: Wie heute, nächster Griff gewinnt (rec.) / Griffe ausblenden / Nur ganze Zeilen/Spalten → **Wie heute**
3. Eckenradius — options: Skaliert wie der Abstand (rec.) / Unverändert → **Skaliert wie der Abstand**
4. Ab wann skalieren — options: ab mehr als 3 pro Reihe / nur > 9 Bilder / immer → owner picked "mehr als 3"; Claude corrected that current 1-9 templates have up to 4 per line → follow-up: **ab 5 pro Reihe** (keeps 1-9 pixel-identical)

## After research (2026-11-15)
Research found handles cover most cells from ~42 images (100 is the picker max).
1. Obergrenze — options: 36 (rec.) / 100 / 25 → **36** (D-06)
2. Verdeckte Griffe bei 16:9/25 — options: hinnehmen (D-05) / Griff-Tippfläche verkleinern (rec.) → **hinnehmen** (D-07)
3. Ladeanzeige ab dem Picken — options: ja (rec.) / später → **ja** (D-08)

## Claude's discretion
Upper limit (later decided by owner: 36), even-grid templates for 10+, preview size scaled by count, test sizes 12 and 25.
