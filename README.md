# dimuli

Collage-Maker für Android (Rust + Slint). Plan und Fortschritt: [`docs/PLAN.md`](docs/PLAN.md),
Architektur: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Debugging auf einem echten Android-Gerät (WSL2)

Die Entwicklung läuft komplett in WSL2. Ein per USB angeschlossenes Gerät sieht WSL nicht automatisch:
Es wird mit [usbipd-win](https://github.com/dorssel/usbipd-win) von Windows an WSL weitergereicht.
`adb` läuft nur in WSL, unter Windows wird es nicht gebraucht.

### Einmalig: Gerät vorbereiten

1. Auf dem Gerät: Einstellungen → Über das Telefon → 7× auf **Build-Nummer** tippen.
2. Entwickleroptionen → **USB-Debugging** einschalten.
3. Per USB anschließen (möglichst direkt am Laptop, Datenkabel). USB-Modus „Dateiübertragung“ wählen.

Windows muss das Gerät mit ADB-Schnittstelle erkennen. In WSL:

```sh
usbipd.exe list
# 1-1    18d1:4ee9  MIDI function, ADB Interface    Not shared
```

Steht dort nur „MIDI function“, ist USB-Debugging aus. Steht dort „Unbekanntes USB-Gerät“
(`0000:0002`), Kabel neu einstecken oder das Gerät neu starten.

### Einmalig: Gerät für WSL freigeben (Windows, Admin)

In einer **Admin-PowerShell** (BUSID aus `usbipd list`):

```powershell
usbipd bind --busid 1-1
```

Die Freigabe bleibt über Neustarts bestehen.

### Einmalig: udev-Regel in WSL

Ohne Regel meldet `adb devices` `no permissions`. Regel für die Hersteller-ID des Geräts anlegen
(VID aus `usbipd.exe list`, z. B. `18d1` für Google/OnePlus):

```sh
echo 'SUBSYSTEM=="usb", ATTR{idVendor}=="18d1", MODE="0666", GROUP="plugdev"' \
  | sudo tee /etc/udev/rules.d/51-android.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

Der Benutzer muss in der Gruppe `plugdev` sein (`id`). War das Gerät beim Anlegen der Regel schon
angehängt, einmal trennen und neu anhängen (siehe unten), damit die Regel greift.

### Jedes Mal: Gerät an WSL anhängen

Nach jedem Ein-/Ausstecken und nach jedem Neustart von Windows oder WSL:

```sh
usbipd.exe list                          # BUSID prüfen, kann sich ändern
usbipd.exe attach --wsl --busid 1-1
adb devices -l
# 8200fcc9    device usb:1-1 product:... model:...
```

Beim ersten Verbinden auf dem Gerät „USB-Debugging zulassen?“ bestätigen
(„Immer von diesem Computer zulassen“), sonst steht es als `unauthorized` in der Liste.

Probleme:

- `no permissions`: udev-Regel fehlt oder greift noch nicht → `usbipd.exe detach --busid 1-1`,
  dann erneut `attach`.
- `Device in error state` beim Attach: ein paar Sekunden warten und `attach` wiederholen.
- `adb` hängt: `adb kill-server`, dann `adb devices`.
- Attach meldet keinen Fehler, aber `adb devices` bleibt leer (`dmesg`: `vhci_hcd: connection closed`):
  Gerät entsperren, Kabel neu einstecken, erneut `attach`.
- OnePlus/OxygenOS: `adb shell settings put …` (z. B. Drehung) scheitert mit `WRITE_SETTINGS`-Fehler,
  bis in den Entwickleroptionen **„Berechtigungsüberwachung deaktivieren“** eingeschaltet ist.
  Alternative ohne diese Option: `adb shell cmd window user-rotation lock 1` bzw. `free`.

### Bauen, installieren, prüfen

```sh
android/gradlew -p android assembleDebug   # Debug-APK, beide ABIs
android/scripts/check-apk.sh               # 16-KB-Ausrichtung, Berechtigungen, SDK-Versionen
android/scripts/smoke.sh -s <SERIAL>       # installieren, starten, Start-Log prüfen, Zurück schließt die App
adb -s <SERIAL> logcat -s dimuli           # App-Logs
```

Ohne `-s` nimmt `smoke.sh` das erste verbundene Gerät. Sind Emulator und Gerät gleichzeitig verbunden,
die Seriennummer aus `adb devices` angeben.
