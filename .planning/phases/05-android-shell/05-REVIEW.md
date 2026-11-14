# Phase 5 Code Review (code-reviewer, 2026-11-14)

Scope: uncommitted diff against 262306a plus untracked android/ and platform/{android,native_events}.rs.

**Ergebnis:** ⚠️ Freigabe mit Auflagen – keine kritischen Befunde; Warnungen sind kleine Härtungen.

## Automatische Prüfungen
| Prüfung | Ergebnis |
|---|---|
| fmt | ✅ sauber |
| clippy (Host, `--workspace --all-targets -D warnings`) | ✅ keine Warnungen |
| test | ✅ alle bestanden |
| `cargo ndk … clippy -p dimuli-app --lib` | ⚠️ im Reviewer nicht ausführbar (ANDROID_JAR/ANDROID_HOME fehlten) |
| cargo audit | nicht installiert |

## Kritisch
keine

Geprüft und in Ordnung: unsafe/SAFETY (2 Blöcke + `#[unsafe(no_mangle)]`, `undocumented_unsafe_blocks = deny`); keine JNI-Objekte in Statics; alle 5 Natives via `native_method!` + `LogErrorAndDefault` + `RegisterNatives`; Natives rufen nur `post_event`; `with_local_frame(16, …)` pro Aufruf; kein unwrap/expect/panic/Indexing; ShareProvider (`exported=false`, `grantUriPermissions=true`, ganze URI verglichen, `mode != "r"` → SecurityException, keine Schreibpfade); MediaStore `IS_PENDING` 1→0, eigene Zeile bei Fehler gelöscht; keine Permission, nur freigegebene Abhängigkeiten, kein AndroidX; D-11 umgesetzt und getestet; keine Artefakte im Diff.

## Warnungen
1. **`MainActivity.kt:74` (`pickImages`), `:143` (`share`)** – nur `ActivityNotFoundException` gefangen; `SecurityException`/`IllegalStateException` aus `startActivity*` crasht die App auf dem UI-Thread. Fix: `RuntimeException` fangen, `nativePickFailed()` bzw. `nativeShared(false)` melden.
2. **`MainActivity.kt:101`** – `nativeSaved(true)` im selben `try` wie das Schreiben; Exception danach löscht die fertige Galerie-Zeile. Fix: `val ok = try { …; true } catch … { …; false }`, `nativeSaved(ok)` außerhalb.
3. **`controller.rs:176`** – Doc-Kommentar veraltet seit D-11 (`Shared(Ok)` zeigt keine Meldung).
4. **`MainActivity.kt:41` (`io.shutdown()`) mit Aufrufern `:85`, `:122`, `:192`** – `io.execute` nach `shutdown` wirft `RejectedExecutionException`. Fix: `RejectedExecutionException` abfangen bzw. `io.isShutdown` prüfen und Fehler melden.

## Hinweise
1. `@Deprecated` am `onActivityResult`-Override unnötig; `@Suppress("OVERRIDE_DEPRECATION")` reicht (kosmetisch).
2. `smoke.sh:51-54` – Start-Screenshot vor dem ersten Slint-Frame.
3. `smoke.sh:37` – `adb install | grep -q Success` unter `pipefail` kann durch SIGPIPE fälschlich scheitern.
4. Share-Datei bleibt in `cacheDir/share` (dokumentiert); optional beim Start aufräumen.
5. `ShareProvider.query` liefert Länge 0 bei fehlender Datei; besser `FileNotFoundException` (optional).
6. Kotlin-Logik (ShareProvider-Prüfungen, MediaStore-Rollback) nur durch Geräte-/Emulatortests abgedeckt; in der Doku vermerken.
7. Doku: Slint-Safe-Area-Modell in PLAN/ARCHITECTURE vermerken (in 05-08 Task 2 erledigt).

## Positiv
Saubere Schichtung (JNI nur in `android.rs`, pures host-getestetes Mapping in `native_events.rs`); Compile-Zeit-Signaturprüfung durch `native_method!`; I/O nicht auf dem UI-Thread, Cache-Namen nicht aus URIs; Back-Callback-Reihenfolge kommentiert; `check-apk.sh` erzwingt 16 KB/keine Permissions/SDK-Stufen; neue State-Tests für save/share.
