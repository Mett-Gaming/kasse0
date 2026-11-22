# KASSE 0

Liminaler Supermarkt-Horror auf VHS. Du willst nur kurz bei **KAUF NULL** einkaufen, einem ganz normalen Discounter. Du gehst in die Getränkeabteilung, kommst wieder heraus, und auf einmal ist alles komisch.

Eigene Engine in Rust mit Vulkan-Renderer (ash, winit). Die Shader sind in WGSL geschrieben und werden beim Bauen per naga zu SPIR-V kompiliert. Ein Vulkan SDK ist nicht nötig. Alle Sounds werden beim Start synthetisiert.

- Design: [docs/game-design.md](docs/game-design.md)
- Credits und Lizenzen der Modelle: [CREDITS.md](CREDITS.md)

## Starten
```powershell
cargo run --release            # Hauptmenü, zufälliges Band (Seed)
cargo run --release -- 1994    # "Neuer Einkauf" nutzt beim ersten Mal Band #1994
```

## So spielst du
1. Hol das **Wasser** aus der Getränkeabteilung (hinten rechts, blaues Schild "Getränke & Pfand").
2. Wenn du sie verlässt, ist der Markt **gekippt**. Arbeite den Einkaufszettel ab (**TAB**).
3. Jeder Artikel existiert **billig** (schwer erreichbar, unten oder oben, tief im Gang) und **teuer** (gut sichtbar). Wie schlimm es wird, bestimmt das Schlimmste aus drei Dingen: **Warenwert** im Wagen (ab 10 €, 25 €, 50 €), **Fortschritt auf dem Zettel** (ab 40 %, 70 %, komplett) und **Zeit** im gekippten Markt (ab 3, 6, 9 Minuten). Wer billig einkauft, kommt also auch ans Ende, nur langsamer.
4. Falsch beschriftete Ware ("Mlich", unsinnige Preise) gehört nicht auf den Zettel.
5. Die **Kunden** kaufen ganz normal ein, solange du hinsiehst. Siehst du weg, starren sie herüber, und einige verfolgen dich: Sie schleichen sich an und holen deutlich auf, wenn du rennst. Je teurer dein Einkauf, desto mehr Verfolger. Nimm ihnen nicht ihren Tomatensaft weg, renn nicht, starr sie nicht an und remple sie nicht an. Wenn einer dich erreicht, ist der Run vorbei.
6. Die **gesichtslosen Mitarbeiter** sprechen dich an. Antworte unauffällig (1, 2 oder 3). Schlechte Antworten machen dich verdächtig, und sie folgen dir.
7. Geh zur **geöffneten Kasse** (abgesperrt, bis dein Zettel komplett ist) (beleuchtetes grünes Schild, dort steht die Kassiererin). Zieh den Wagen heran und leg die Ware **Stück für Stück mit E aufs Band** (Mausrad wählt, welche). Dann beginnt das **Kassen-Minispiel** (siehe unten). Bezahlt ist, wenn die Karte durchgezogen und die PIN eingetippt ist (oder du bar und passend gezahlt hast).
8. **Flucht:** Kaum hast du bezahlt, kommt die Durchsage *"Kunde 0 hat ein Gesicht. Schnappt ihn!"*. Der Alarm geht los, und alle Kunden und Mitarbeiter rennen auf dich zu. Lauf zum Ausgang! Wer dich jetzt erwischt, beendet den Einkauf sofort.

### Das Kassen-Minispiel
In den **Kassenbereich kommen keine NPCs** hinein. Sobald die erste Ware auf dem Band liegt, sind **Kunden und Mitarbeiter harmlos**, solange du an der Kasse bleibst. Dafür beobachtet dich jetzt die **Kassiererin**. Unten links zeigt ein Balken ihr Misstrauen, daneben stehen deine **drei Kassen-Leben**.
- **Ware richtig aufs Band:** Getränke (schwer) zuerst. Falsch beschriftete Ware sortiert sie als "ARTIKEL UNBEKANNT" aus, und das fällt auf.
- **Fragen beantworten (1, 2, 3):** Während sie scannt, fragt sie zweimal etwas ("Sammeln Sie Punkte?"). Antworte unauffällig. Schweigen fällt auch auf.
- **Karte durchziehen:** E am Kartenterminal, dann die Maus ruhig nach unten ziehen. Zu schnell, und die Karte wird nicht gelesen.
- **PIN eintippen (0–9):** Das Terminal zeigt die PIN kurz an. Merk sie dir und tipp sie ein, bevor die Zeit abläuft.
- **Manchmal nur bar:** *"Kartenzahlung geht heute nicht."* Dann öffnet sich deine Geldbörse. Wähle Scheine und Münzen mit A/D oder dem Mausrad, leg sie mit E hin (F nimmt zurück) und zahl **genau passend**, bevor die Zeit abläuft. Zu viel fällt auf.
- **Nicht weggehen:** Entfernst du dich von der Kasse, steigt ihr Misstrauen, und die Verfolger können dich wieder erwischen.

**Direkte Fehler** (Getränke nicht zuerst, falsche Ware, schlechte Antwort, Karte zu schnell, falsche PIN, zu viel Geld) kosten **sofort ein Kassen-Leben**. Weiche Fehler (neutrale Antwort, Schweigen, zu langsam, Weggehen) füllen nur ihren Misstrauens-Balken. Ist der voll, hebt sie den Kopf: ebenfalls ein Leben weg, und der Balken beginnt wieder bei null. Nach dem dritten Mal hat sie dich durchschaut, und der Run ist vorbei. Die Kassen-Leben sind getrennt von den drei Leben im Markt.

Deine Auffälligkeit zeigt im gekippten Markt ein kleiner Balken unten links (grün → gelb → rot, blinkt bei 100 %), sobald sie über null liegt.

**Drei Leben:** Zweimal kannst du erwischt werden. Du sackst zusammen und wachst am Eingang auf, dein Wagen bleibt, wo es passiert ist. Jedes Mal bleibt eine stärkere rote Vignette. Beim dritten Mal ist der Einkauf vorbei.

## Steuerung
| Taste | Aktion |
|---|---|
| WASD / Pfeile | Gehen |
| Shift | Rennen (fällt auf, kostet Ausdauer; die Leiste erscheint, wenn sie nicht voll ist) |
| Maus | Umsehen |
| E / Leertaste / Linksklick | Ware aus dem Regal greifen; mit Ware in der Hand: auf den Wagen zielen = hineinlegen, aufs Band zielen = aufs Band legen, sonst fallen lassen; auf das Band ohne Ware: die gewählte Ware aus dem Wagen aufs Band; am Kartenterminal: Karte durchziehen (nochmal E bricht ab) |
| Q (oder E auf den Wagen) | Wagen stehen lassen / wieder schieben |
| F (am Wagen) | Ware wieder herausnehmen: die anvisierte (sie leuchtet auf), sonst die zuletzt eingelegte |
| Mausrad (am Wagen oder Band) | Andere Ware im Wagen anwählen |
| Maus nach unten (am Terminal) | Karte durchziehen, ruhig und nicht zu schnell |
| TAB (halten) | Einkaufszettel und Warenwert |
| 1 / 2 / 3 | Antwort im Gespräch und an der Kasse |
| 0–9 | PIN am Kartenterminal |
| A / D oder Mausrad, E, F (beim Barzahlen) | Schein oder Münze wählen, hinlegen, zurücknehmen |
| Esc / P | Pause (mit Tastenübersicht) |
| F1 | Cheat-Menü zum Testen (zwei Spalten, Links/Rechts wechselt): u. a. jeden Jumpscare einzeln auslösen |
| F12 | Screenshot (Ordner *Bilder*) |

Beim ersten Einkauf zeigt ein Tutorial-Panel alle Tasten und hakt sie ab. Abschalten kannst du es in den Optionen. Zum Ausräumen aufs Band muss der Wagen an der Kasse stehen. Unterwegs kannst du ihn aber parken und Waren zu Fuß holen.

### Der Wagen
- **Er quietscht.** Im gekippten Markt hören Mitarbeiter in der Nähe das Rad und kommen nachsehen. Wer schleichen will, lässt ihn stehen.
- **Er passt nicht überall durch.** Die Gänge verbiegen sich, und zwischen versetzten Regalen öffnen sich schmale Durchgänge. Zu Fuß kommst du durch, mit dem Wagen nicht.
- **Mitarbeiter räumen auf.** Ein unbeaufsichtigter Wagen wird zur **Wagenbox am Eingang** geschoben, eine Durchsage verrät es. Erwischst du ihn dabei: E, und du sagst *"Hey, das ist mein Wagen!"*. Danach folgt ein kurzes Gespräch, und er lässt ihn los.
- **Zum Ausräumen aufs Band** muss der Wagen an der Kasse stehen.

### Der Markt
- Aufgebaut wie ein deutscher Discounter: Obst & Gemüse am Eingang (Stufenregal und Insel), durchgehende Kühlwand mit Kopfschildern, eine Reihe niedriger Kühltruhen (man sieht darüber hinweg), Regalgänge mit Ware in Kartons, Gitterboxen mit roten "AKTION"-Schildern, dunkelblaue Banderole mit Sprüchen.
- **Jede Ware im Regal ist ein echtes Modell** (rund 125 Produkte in 19 Unterabteilungen) und lässt sich einzeln greifen. Gelbe Preisleisten, echte Preise.
- Sortiert ist logisch nach Warengruppen. Mit steigender Eskalation verschieben sich die Abteilungen: erst tauschen Unterabteilungen (Fisch ↔ Käse), dann wandern ganze Abteilungen in fremde Einrichtungen, am Ende steht Ware wild durcheinander und **die Schilder lügen**.
- Im gekippten Markt verderben die Sprüche ("Frisch & ewig", "Bleib & kauf"), die Kassen zeigen "0 Kasse", das Zigaretten-Rollo ist offen, die Überwachungskameras drehen sich nach dir, hinter den Fenstern ist nur noch Schwärze.

### Gespräche
- Während eines Gesprächs (und 2,5 s danach) können dich die Kunden nicht erwischen.
- Rennst du an einem Mitarbeiter vorbei, kann er dich nicht ansprechen. Das fällt aber auf (+20 %), und er folgt dir.

### Der Markt verändert sich
Je teurer dein Einkauf, desto mehr verändert sich der Markt:
- Die Gänge werden länger.
- Röhren wechseln zufällig ihr Flacker-Muster: Stroboskop, Pulsieren, fast aus, Unterspannung.
- Die Gänge biegen sich zu echten S-Kurven, die Regale folgen dem Bogen. Mittendrin öffnen sich Durchgänge, wo vorher keine waren, und es stehen Wände, wo vorher keine standen.
- Werbeposter verrotten immer weiter.
- Auf den hohen Stufen biegen sich die Gänge live: Regale schwanken, der Boden hebt und senkt sich (Steigung). Bereiche wandern beim Umbau: Kühlabschnitte tauschen die Plätze, die Tiefkühlreihe steht in einem anderen Gang, die Obst-Insel rückt.
- Kunden flüstern miteinander, wenn sie sich begegnen.
- Der Boden verfällt: erst fehlende und gesprungene Fliesen und Ameisenstraßen, dann Fliesenstapel, Löcher (Hindernisse, hineinfallen kann man nicht) und Ratten. In der Decke klaffen Löcher mit Kabeln, am Ende tropft Blut von der Decke und sammelt sich am Boden.
- Ware verdirbt: verfärbt, mit Maden und Fliegen; manche schwebt, kippt oder steht kopfüber. Riesige Waren stehen im Weg.
- Mitarbeiter tragen kräftig blaue Dienstkleidung und leuchten im Dunkeln leicht, damit man sie von der Kundschaft unterscheiden kann.
- Fata Morganen tauchen auf: eine Kundin, ein Mitarbeiter, ein voller Wagen, ein Ausgangsschild. Sie wirken echt, bis du auf etwa 3 m heran bist, dann flackern sie als Hologramm auf und sind weg.
- Jumpscares, mit kurzen Ruhepausen dazwischen. Das Licht fällt aus und danach steht jemand vor dir, ein Verfolger steht plötzlich hinter dir, die Kassiererin beugt sich ruckartig zu dir vor, dein abgestellter Wagen verschwindet und rollt von selbst heran, aus Deckenlöchern stürzen Kabel, und auf dem Überwachungsmonitor an der Kasse steht jemand hinter dir, der nicht da ist, wenn du dich umdrehst.

Ab der zweiten Stufe baut sich der Markt um, wenn du gerade nicht hinsiehst (umdrehen, Boden oder Decke anschauen).

In Menüs: Pfeiltasten oder W/S, Enter, Esc oder die Maus. In den Optionen ändern Links/Rechts (bzw. Links- und Rechtsklick) die Werte.

## Optionen
Mausempfindlichkeit, Lautstärke, VHS-Effekt-Stärke, Sichtfeld, Vollbild, Tutorial an/aus. Sie werden in `%APPDATA%\kasse0\settings.txt` gespeichert. Dazu gibt es „Jumpscares“: Die lassen sich nicht abschalten. Probier es ruhig.

## Entwicklung
```powershell
cargo test                     # Generator, Gameplay-Regeln, UI, Audio, Assets
```

Frame-Zeit messen: `$env:KASSE0_PERF = 1` setzen, dann schreibt das Spiel alle 5 s die mittlere Frame-Zeit auf die Konsole (auch im Auto-Screenshot-Durchlauf).

Mit dem Auto-Screenshot-Durchlauf rendert das Spiel automatisch alle Screens (Titel, Markt, gekippt, Dialog, Pause, Optionen, Game Over, Kassenbon …) als BMP in einen Ordner und beendet sich dann:

```powershell
$env:KASSE0_AUTOSHOT = "$PWD\shots"; cargo run --release -- 1994
```

Das Cheat-Menü (**F1**) kippt den Markt, füllt den Zettel, erhöht den Warenwert, teleportiert, ruft Mitarbeiter usw. Die rechte Spalte („Schreck: …“) teleportiert an eine passende Stelle und löst genau einen Jumpscare aus (Hinter dir, Stromausfall, Kassiererin, Geisterwagen, Fata Morgana, Deckenkabel, Kopfüber, Monitor, Tod ohne Lebensverlust, Troll-Option). Für Validation-Layer im Debug-Build das LunarG Vulkan SDK installieren.

### Modelle neu konvertieren
Waren (Katalog `tools/catalog.json`, Quell-Packs in `assets_src/packs`):
```powershell
D:AppsBlenderlender.exe -b --factory-startup -P tools/prepare_products.py -- .
```
Kundschaft und Personal (Varianten in `tools/people.json`, Quelle `assets_src/packs/quaternius_modular_people`):
```powershell
D:AppsBlenderlender.exe -b --factory-startup -P tools/prepare_people.py -- .
```
Figuren und Requisiten (Quaternius-Mannequins, Poly-Haven-Modelle; mit `$env:KASSE0_ONLY = "cashier,chair"` nur einzelne):
Die Quell-Modelle liegen in `assets_src/downloads` (Web-Downloads) bzw. auf `F:\Dev\_work\source-assets\quaternius`. Das Skript reduziert die Polygone, friert die Posen ein und ersetzt Texturen durch Durchschnittsfarben:
```powershell
D:\Apps\Blender\blender.exe -b --factory-startup -P tools/prepare_assets.py -- F:\Dev\_work\source-assets\quaternius assets_src\downloads assets
```

## Aufbau
| Pfad | Inhalt |
|---|---|
| `src/render/` | Vulkan: Kontext, Swapchain, Pipelines (Boxen, Meshes, UI, VHS-Post), Screenshots |
| `src/world/` | Seedbarer Markt-Generator (`gen.rs`, Einrichtungen in `gen/fixtures.rs`), Abteilungs-Layout (`layout.rs`), Kollision, Sichtlinien, Navigationsraster |
| `src/catalog.rs`, `tools/catalog.json` | Warenkatalog: Unterabteilungen, Modelle, Preise, Zettel-Einträge |
| `src/signs.rs` | Schild-Atlas: alle Schriftzüge (Archivo Black) werden beim Start gerendert |
| `src/game/` | Run-Logik, Hände und Wagen (`hands.rs`), Kasse mit Band (`checkout.rs`), Kassen-Minispiel (`till.rs`, Barzahlung `till/cash.rs`, HUD in `src/hud/till.rs`), Flucht nach dem Bezahlen (`escape.rs`), NPCs (`crowd.rs`), Dialoge und Ereignisse (`tension.rs`), Eskalation |
| `src/people.rs`, `shaders/skin.wgsl` | Animierte Figuren: Knochen-Gewichte, Animationen abtasten, Skinning |
| `src/hud.rs`, `src/ui.rs` | Menüs, HUD, Bitmap-Text |
| `src/audio.rs` | Synthetisierte Musik und Effekte (rodio) |
| `shaders/` | WGSL (`_common.wgsl` wird per `#include` eingebunden) |
| `assets/` | Konvertierte Modelle (`.k0m`, Waren in `assets/products/`) und die Schrift, fest ins Spiel eingebaut |

## Builds und Releases
GitHub Actions (`.github/workflows/build.yml`) baut bei jedem Push auf `main` und bei Pull Requests: Tests, Release-Build und das Paket (Exe, `LIESMICH.txt`, Credits, Lizenzen) als Artefakt des Laufs.

Ein Release entsteht durch einen Tag:
```powershell
git tag v0.2.0
git push origin v0.2.0
```
Danach liegt `KASSE0-v0.2.0.zip` unter *Releases* im Repository. Die Original-Packs (`assets_src/`) sind nicht im Repository; gebaut wird aus den konvertierten Dateien in `assets/`.
