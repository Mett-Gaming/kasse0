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
7. Geh zur **geöffneten Kasse** (abgesperrt, bis dein Zettel komplett ist) (beleuchtetes grünes Schild, dort sitzt die Kassiererin). Zieh den Wagen heran und drück **E am Band**: Es öffnet sich das **Kassenfenster** mit deinem Wagen links, dem Band in der Mitte und dem Bon rechts. Zieh die Waren **mit gedrückter Maustaste** aufs Band. E oder Esc schließt das Fenster. Dann beginnt das **Kassen-Minispiel** (siehe unten). Bezahlt ist, wenn die Karte durchgezogen und die PIN eingetippt ist (oder du bar und passend gezahlt hast).
8. **Flucht:** Kaum hast du bezahlt, kommt die Durchsage *"Kunde 0 hat ein Gesicht. Schnappt ihn!"*. Der Alarm geht los, und alle Kunden und Mitarbeiter rennen auf dich zu. Du hörst ihre Schritte immer näher kommen und dein Herz immer schneller schlagen. Lauf zur Tür! Wer dich jetzt erwischt, beendet den Einkauf sofort.
9. **Der Korridor:** Hinter der Tür wartet ein langer, niedriger Gang mit rot flackernden Röhren, der **bei jedem Lauf neu gewürfelt** wird: mal links, mal rechts um die Ecke, nie zurück. **Je länger dein Run gedauert hat, desto länger ist er** (40 m bis 140 m). Hinter dir knallt die Tür zu. Im Gang bist du allein, aber es klingt nicht so: Schritte, Atmen, Herzschlag. Etwas rückt nach, und **wer stehen bleibt oder zu langsam ist, wird eingeholt** („SCHNELLER“). Ausdauer ist hier unendlich. Unterwegs: Kistenreihen mit Lücke, Rohre und Rollgitter zum **Ducken** (C oder Strg), Barrieren zum **Springen** (Leertaste), **Schwingtüren** zum Aufdrücken (E, manche klemmen), **Regale, die umkippen**, **Einkaufswagen, die dir entgegenrollen** (ein Treffer lässt dich stolpern), Engstellen, eine **Decke, die sich senkt**, und **Wände, die zusammenrücken** (dazwischen kannst du weiter sprinten). Gestalten stehen im Gang und verschwinden, Lautsprecher sagen Dinge, die du nicht hören willst. Und an den Wänden hängen Poster: *NICHT UMDREHEN*. Das ist ernst gemeint: **Wer sich umdreht, ist sofort tot** (schon beim Seitenblick färbt sich das Bild rot). Am Ende hängt ein großer Spiegel, und in ihm siehst du, dass jemand direkt hinter dir steht. Die Ausgangstür davor klemmt. Drück sie auf, bevor er dich hat.

### Das Kassen-Minispiel
In den **Kassenbereich kommen keine NPCs** hinein. Sobald die erste Ware auf dem Band liegt, sind **Kunden und Mitarbeiter harmlos**, solange du an der Kasse bleibst. Dafür beobachtet dich jetzt die **Kassiererin**. Unten links zeigt ein Balken ihr Misstrauen, daneben stehen deine **drei Kassen-Leben**.
- **Ware richtig aufs Band:** Schweres zuerst: alles ab 1 kg (z. B. Wasser, Melone, Milch) vor alles Leichte. Im Kassenfenster steht bei jeder Ware das Gewicht, schwere Ware ist rot. Falsch beschriftete Ware sortiert sie als "ARTIKEL UNBEKANNT" aus, und das fällt auf.
- **Fragen beantworten (1, 2, 3):** Während sie scannt, fragt sie zweimal etwas ("Sammeln Sie Punkte?"). Antworte unauffällig. Schweigen fällt auch auf.
- **Karte durchziehen:** E am Kartenterminal öffnet den Kartenleser. Mit E nimmst du die SCAMCART aus dem Portemonnaie, dann ziehst du sie mit der Maus **gleichmäßig nach rechts** durch den Leser. Das Display meldet „ZU SCHNELL“ (kostet ein Leben) oder „ZU LANGSAM“ (macht sie misstrauisch; auch, wenn du unterwegs anhältst). Dann noch mal. Bei „ANGENOMMEN“ geht es weiter zur PIN.
- **PIN eintippen (0–9):** Das Terminal zeigt die PIN kurz an. Merk sie dir und tipp sie ein, bevor die Zeit abläuft.
- **Manchmal nur bar:** *"Kartenzahlung geht heute nicht."* Dann öffnet sich deine Geldbörse. Wähle Scheine und Münzen mit A/D oder dem Mausrad, leg sie mit E hin (F nimmt zurück) und zahl **genau passend**, bevor die Zeit abläuft. Zu viel fällt auf.
- **Sie wird ungeduldig:** Ihr Misstrauen steigt von allein, je länger du brauchst, desto schneller. Sie nörgelt, das Licht flackert, dein Herz rast, und am Absperrband klopft es.
- **Sie schaut auf:** Ab und zu stoppt der Scanner, es knarzt, und sie starrt dich an. Dann **still stehen und nicht ins Gesicht schauen**.
- **Kassendisplay prüfen:** Rechts oben läuft der Bon mit. Manchmal scannt sie etwas doppelt oder viel zu teuer. Bemerk es und drück **R** zum Reklamieren. Wer es übersieht, verliert beim Bezahlen ein Leben. Wer grundlos reklamiert, macht sie misstrauisch.
- Je länger du an der Kasse stehst und je mehr Leben schon weg sind, desto schlimmer wird alles (auch beim Barzahlen: weniger Zeit, mehr Kleingeld-Chaos).
- **Nicht weggehen:** Entfernst du dich von der Kasse, steigt ihr Misstrauen, und die Verfolger können dich wieder erwischen.

**Direkte Fehler** (Schweres nicht zuerst, falsche Ware, schlechte Antwort, Karte zu schnell, falsche PIN, zu viel Geld) kosten **sofort ein Kassen-Leben**. Weiche Fehler (neutrale Antwort, Schweigen, zu langsam, Weggehen) füllen nur ihren Misstrauens-Balken. Ist der voll, hebt sie den Kopf: ebenfalls ein Leben weg, und der Balken beginnt wieder bei null. Nach dem dritten Mal hat sie dich durchschaut, und der Run ist vorbei. Die Kassen-Leben sind getrennt von den drei Leben im Markt.

Deine Auffälligkeit zeigt im gekippten Markt ein kleiner Balken unten links (grün → gelb → rot, blinkt bei 100 %), sobald sie über null liegt.

**Drei Leben:** Zweimal kannst du erwischt werden. Du sackst zusammen und wachst am Eingang auf, dein Wagen bleibt, wo es passiert ist. Jedes Mal bleibt eine stärkere rote Vignette. Beim dritten Mal ist der Einkauf vorbei.

## Steuerung
| Taste | Aktion |
|---|---|
| WASD / Pfeile | Gehen |
| Shift | Rennen (fällt auf, kostet Ausdauer; die Leiste erscheint, wenn sie nicht voll ist) |
| Leertaste | Springen (im Fluchtkorridor über niedrige Hindernisse). Mit dem Wagen nur ein kleiner Hopser, der Wagen hopst mit, und beim Aufsetzen fällt zufällig Ware heraus: mal keine, mal eine, mal mehrere |
| C oder Strg (halten) | Ducken (im Fluchtkorridor unter Rohren und Gittern durch) |
| Maus | Umsehen |
| E / Linksklick | Ware aus dem Regal greifen; im Fluchtkorridor: Tür aufdrücken; mit Ware in der Hand: auf den Wagen zielen = hineinlegen, aufs Band zielen = aufs Band legen, sonst fallen lassen; auf das Band ohne Ware: die gewählte Ware aus dem Wagen aufs Band; am Kartenterminal: Kartenleser öffnen, dann Karte aus dem Portemonnaie nehmen (F bricht ab) |
| Q (oder E auf den Wagen) | Wagen stehen lassen / wieder schieben |
| F (am Wagen) | Ware wieder herausnehmen: die anvisierte (sie leuchtet auf), sonst die zuletzt eingelegte |
| Mausrad (am Wagen oder Band) | Andere Ware im Wagen anwählen |
| Maus nach rechts (am Kartenleser) | Karte durchziehen: gleichmäßig, nicht zu schnell, nicht zu langsam |
| TAB (halten) | Einkaufszettel und Warenwert |
| 1 / 2 / 3 | Antwort im Gespräch und an der Kasse. Sonst völlig nutzlos: **1** zündet eine Zigarette an, **2** führt sie zum Mund und nimmt einen Zug (sie wird jedes Mal kürzer), **3** schnippt sie weg (die Kippe glimmt am Boden weiter). Nach dem 4. Zug wirfst du sie von allein weg |
| 0–9 | PIN am Kartenterminal |
| R | An der Kasse reklamieren (falsch gescannt) |
| A / D oder Mausrad, E, F (beim Barzahlen) | Schein oder Münze wählen, hinlegen, zurücknehmen |
| Esc / P | Pause (mit übersichtlicher Steuerung: Bewegen und Einkauf & Kasse, Tasten als Tastenkappen) |
| F1 | Cheat-Menü zum Testen, nach Kategorien: Spieler, Markt & Einkauf, Kasse & Flucht, Leute, Schrecken. Enter oder Rechts öffnet eine Kategorie, Esc oder Links geht zurück |
| F12 | Screenshot (Ordner *Bilder*) |

Beim **allerersten Einkauf** fragt das Spiel, ob du das **Tutorial** spielen willst (Ja/Nein, wird nur einmal gefragt). Bei „Nein“ gibt es einen kurzen Spruch, dann geht der normale Einkauf los. Das Tutorial ist ein ganzer Einkauf mit Anleitung. Erst hakt er alle Tasten ab, danach erklärt ein Kasten jede Phase (Wasser, Einkaufen, Kasse, Bezahlen, Flucht). **Pfeile am Boden** zeigen den Weg (am Anfang zum Wasser in der Getränkeabteilung), und die Waren, die dir noch fehlen, **leuchten** (alle in der Nähe, nicht nur die nächste wie im normalen Spiel). Im Tutorial kannst du nicht verlieren: Kunden und Mitarbeiter verfolgen dich gar nicht (auch nicht bei der Flucht), und Fehler an der Kasse werden nur erklärt. Beim Einkaufen wechselt der Kasten zwischen drei Seiten: Einkaufen, Kunden (kommen näher, wenn du nicht hinsiehst) und Mitarbeiter (sprechen dich an, Antwort mit 1/2/3). Im Kassenfenster und beim Bezahlen (Karte, PIN, Bargeld) erklärt eine kleine Leiste oben auf dem Fenster die Bedienung. Schaffst du es und gehst danach ins Hauptmenü, heißt es: „Du bist jetzt schlau genug, um einkaufen zu können. Wow.“ Im normalen Einkauf gibt es kein Tutorial-Fenster. Zum Ausräumen aufs Band muss der Wagen an der Kasse stehen. Unterwegs kannst du ihn aber parken und Waren zu Fuß holen.

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

### Passende Ware leuchtet
Stehst du höchstens **1 m** neben einer Ware, die zu einem offenen Zettel-Eintrag passt, pulsiert sie hell. Es leuchtet immer nur **eine**, die nächste. Auch **falsch beschriftete** und teure Ware leuchtet: Hinsehen musst du selbst. Mit Ware in der Hand, in Gesprächen und an der Kasse leuchtet nichts.

### Was die NPCs wahrnehmen
- **Verfolger** sehen dich rundum bis 22 m (ganz nah spüren sie dich immer) und hören deine Schritte: Rennen hört man 15 m weit, Gehen kaum, ein quietschender Wagen oder herunterfallende Ware locken sie an. Sie jagen die Stelle, an der sie dich zuletzt bemerkt haben. Hängst du sie ab, suchen sie dort und geben nach einer Weile auf.
- Zwei oder mehr Verfolger **nehmen dich in die Zange**: Der zweite meidet den Weg des ersten und kommt durch einen anderen Gang.
- **Mitarbeiter** sehen nur nach vorne (bis 9 m, ganz nah immer). Hören sie Lärm, gehen sie zur Geräuschquelle und sehen sich um.
- Niemand läuft mehr durch andere hindurch, Wege werden geglättet, und wer irgendwo hängen bleibt, sucht sich einen neuen Weg.

### Wie sich die NPCs verhalten
- Kunden tun im Stehen etwas: ins Regal greifen, **dieselbe Ware immer wieder nehmen**, telefonieren, Arme verschränken, reden. Im gekippten Markt folgt ihr Kopf dir.
- Sie **ahmen dich nach**: Stehst du still, erstarren sie. Greifst du ins Regal, greifen sie mit. Starrst du eine an, dreht sie sich weg.
- Erwischst du einen Verfolger beim Heranschleichen, **erstarrt er mitten im Schritt**. Ab „falsch“ schleichen Verfolger geduckt, im Kollaps wie Untote. Auf den letzten Metern holen sie zum Griff aus.
- Mitarbeiter **räumen kniend Regale ein** (später endlos dieselbe Lücke), schieben deinen Wagen weg und reden mit dir.
- Ab „unruhig“ werden die Kunden **körperlich falsch**: Kopf zu weit gedreht, kurze Ruckler wie Stop-Motion. Ab „falsch“ langer Hals, Zucken, zu lange Arme. Im Kollaps lange Glieder, Köpfe bis ganz herum oder verkehrt herum, und wer genau hinsieht, sieht sie **zerreißen**.
- **Du hörst sie:** Schritte der Verfolger mit Richtung (links/rechts), im Kollaps ein Schlurfen, das Schlüsselbund des Personals, Flüstern aus der Richtung der Flüsternden. Schaust du einen Verfolger an, verstummt er.
- Ein **Director** steuert die Spannung: Erst rücken einzelne Verfolger heran, dann alle, dann ist eine Weile Ruhe, und die anderen lauern nur auf Abstand. Nach einem Tod sind 20 s Ruhe, auf dem Höhepunkt gibt es keine Scares.

### Gespräche
- Während eines Gesprächs bewegt sich kein Kunde (alle frieren in ihrer Pose ein und schauen nur langsam zu dir), auch der Geisterwagen hält an. Danach hast du noch 2,5 s Schonfrist.
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
- Jumpscares, mit kurzen Ruhepausen dazwischen. Das Licht fällt aus und danach steht jemand vor dir, ein Verfolger steht plötzlich hinter dir, die Kassiererin beugt sich ruckartig zu dir vor, dein abgestellter Wagen verschwindet und rollt von selbst heran, aus Deckenlöchern stürzen Kabel, und auf dem Überwachungsmonitor an der Kasse steht jemand hinter dir, der nicht da ist, wenn du dich umdrehst. Dazu: jemand liegt in der Kühltruhe und richtet sich auf, eine Gestalt rennt kopfüber über die Decke, alle Kunden drehen sich gleichzeitig zu dir, im Dunkeln rennt jemand von hinten heran, ein Gesicht schiebt sich zwischen die Ware, in deinem Wagen hockt jemand, die Durchsage kreischt, ein Regal kracht auf dich. Jeder Schreck klingt und wirkt anders, und dieselbe Art kommt nicht direkt wieder.

Ab der zweiten Stufe baut sich der Markt um, wenn du gerade nicht hinsiehst (umdrehen, Boden oder Decke anschauen).

In Menüs: Pfeiltasten oder W/S, Enter, Esc oder die Maus. In den Optionen ändern Links/Rechts (bzw. Links- und Rechtsklick) die Werte.

## Optionen
Mausempfindlichkeit, Lautstärke, VHS-Effekt-Stärke, Sichtfeld, Vollbild. Hast du das Tutorial einmal geschafft, gibt es hier auch **„Tutorial spielen“**. Das Spiel startet im **randlosen Vollbild**; abgeschaltet läuft es im Fenster (1280×720). Sie werden in `%APPDATA%\.mettlauncher\kasse0\settings.txt` gespeichert. Dazu gibt es „Jumpscares“: Die lassen sich nicht abschalten. Probier es ruhig.

## Entwicklung
Die Engine **Mettgine** liegt als Git-Submodule in `mettgine/` ([Mett-Gaming/mettgine](https://github.com/Mett-Gaming/mettgine)). Darum mit `--recursive` klonen bzw. nach dem Holen die Engine mitziehen:

```powershell
git clone --recursive https://github.com/Mett-Gaming/kasse0.git
git submodule foreach "git checkout main"   # einmal nach dem Klonen
git pull
git submodule update --init --remote --merge
```

Nach dem Klonen steht die Engine kurz auf einem losen Commit; der einmalige `git checkout main` setzt sie auf ihren Zweig. Danach holt `update --remote --merge` neue Engine-Stände per Merge (in `.gitmodules`: `branch = main`, `update = merge`). Engine-Änderungen werden ohne Pull-Requests direkt gemacht:
1. In `mettgine/` ändern und dort committen, dann `git fetch origin`, `git merge origin/main`, `git push origin main` (bei neuer Funktion oder Korrektur mit Tag `vX.Y.Z`).
2. In kasse0 den neuen Engine-Stand festhalten: `git add mettgine`, mit den Spieländerungen committen und wie gewohnt pushen.

```powershell
cargo test                     # Spiel und Engine: Generator, Gameplay-Regeln, UI, Audio, Assets, Animation, Absturzberichte
```

Frame-Zeit messen: `$env:KASSE0_PERF = 1` setzen, dann schreibt das Spiel alle 5 s die mittlere Frame-Zeit auf die Konsole (auch im Auto-Screenshot-Durchlauf).

Mit dem Auto-Screenshot-Durchlauf rendert das Spiel automatisch alle Screens (Titel, Markt, gekippt, Dialog, Pause, Optionen, Game Over, Kassenbon …) als BMP in einen Ordner und beendet sich dann:

```powershell
$env:KASSE0_AUTOSHOT = "$PWD\shots"; cargo run --release -- 1994
```

Das Cheat-Menü (**F1**, zwei Spalten) kippt den Markt, füllt den Zettel, erhöht den Warenwert, teleportiert, ruft Mitarbeiter, springt direkt zur Karten- oder Barzahlung, füllt die Leben wieder auf (Markt und Kasse), startet einen Einkauf im Tutorial-Modus oder beendet dort die Anleitung, lässt dich sofort sterben (zum Ansehen der Todes-Schriftzüge) usw. Die rechte Spalte („Schreck: …“) teleportiert an eine passende Stelle und löst genau einen Jumpscare aus (Hinter dir, Stromausfall, Kassiererin, Geisterwagen, Fata Morgana, Deckenkabel, Kopfüber, Monitor, Tod ohne Lebensverlust, Troll-Option). Für Validation-Layer im Debug-Build das LunarG Vulkan SDK installieren.

### Modelle neu konvertieren
Waren (Katalog `tools/catalog.json`, Quell-Packs in `assets_src/packs`):
```powershell
D:\Apps\Blender\blender.exe -b --factory-startup -P tools/prepare_products.py -- .
```
Kundschaft und Personal (Varianten in `tools/people.json`, Quelle `assets_src/packs/quaternius_modular_people`, zusätzliche Animationen aus `assets_src/packs/quaternius_ual/UAL1_Standard.glb` und `UAL2_Standard.glb`, Quaternius Universal Animation Library 1+2, CC0). Das Skript schreibt die Knochen-Hierarchie und überträgt die UAL-Clips auf das Figuren-Skelett:
```powershell
D:\Apps\Blender\blender.exe -b --factory-startup -P tools/prepare_people.py -- .
```
Figuren und Requisiten (Quaternius-Mannequins, Poly-Haven-Modelle; mit `$env:KASSE0_ONLY = "cashier,chair"` nur einzelne):
Die Quell-Modelle liegen in `assets_src/downloads` (Web-Downloads) bzw. auf `F:\Dev\_work\source-assets\quaternius`. Das Skript reduziert die Polygone, friert die Posen ein und ersetzt Texturen durch Durchschnittsfarben:
```powershell
D:\Apps\Blender\blender.exe -b --factory-startup -P tools/prepare_assets.py -- F:\Dev\_work\source-assets\quaternius assets_src\downloads assets
```

## Aufbau
Das Repository ist ein Cargo-Workspace aus zwei Teilen: **Mettgine** (`mettgine/`, die Engine der Mett-Gaming-Spiele, als Submodule aus [Mett-Gaming/mettgine](https://github.com/Mett-Gaming/mettgine)) und dem Spiel KASSE 0 (`src/`). `cargo test` prüft beide.

| Pfad | Inhalt |
|---|---|
| `mettgine/src/render/` | Vulkan: Kontext, Swapchain, Pipelines (Boxen, Meshes, Figuren, Hologramme, Überwachungskamera, UI, VHS-Post), Screenshots |
| `mettgine/src/anim.rs` | Knochen-Animation (`.k0a` v2): Überblenden, Kopf-Blickziel, Zucken, Strecken, Stop-Motion; die Clip-Liste legt das Spiel fest (`src/people/clips.rs`) |
| `mettgine/src/gpu.rs` | Datenformate zwischen Spiel und Renderer (Welt-Boxen, Lichter, Modelle, Figuren `.k0s`) |
| `mettgine/src/crash.rs`, `crash_window.rs` | Protokoll, Absturzberichte, Absturzfenster |
| `mettgine/src/ui.rs` | Bitmap-Text und Formen für Menüs und HUD |
| `mettgine/shaders/` | WGSL (`_common.wgsl` wird per `#include` eingebunden); die Welt-Materialien (Poster, Schilder, Monitor) stammen noch aus KASSE 0 |
| `src/world/` | Seedbarer Markt-Generator (`gen.rs`, Einrichtungen in `gen/fixtures.rs`), Abteilungs-Layout (`layout.rs`), Kollision, Sichtlinien, Navigationsraster |
| `src/catalog.rs`, `tools/catalog.json` | Warenkatalog: Unterabteilungen, Modelle, Preise, Zettel-Einträge |
| `src/signs.rs` | Schild-Atlas: alle Schriftzüge (Archivo Black) werden beim Start gerendert |
| `src/game/` | Run-Logik, Hände und Wagen (`hands.rs`), Kasse mit Band (`checkout.rs`), Kassen-Minispiel (`till.rs`, Barzahlung `till/cash.rs`, HUD in `src/hud/till.rs`), Flucht nach dem Bezahlen (`escape.rs`), NPCs (`crowd.rs`), Dialoge und Ereignisse (`tension.rs`), Eskalation |
| `src/people.rs`, `src/people/clips.rs` | Figuren von KASSE 0: Varianten, Clip-Liste, Gangart |
| `src/hud.rs` | Menüs und HUD |
| `src/audio.rs` | Synthetisierte Musik und Effekte (rodio) |
| `assets/` | Konvertierte Modelle (`.k0m`, Waren in `assets/products/`) und die Schrift, fest ins Spiel eingebaut |

## Builds und Releases
GitHub Actions (`.github/workflows/build.yml`) baut bei jedem Push auf `main` und bei Pull Requests: Tests, Release-Build und das Paket (Exe, `LIESMICH.txt`, Credits, Lizenzen) als Artefakt des Laufs.

Ein Release entsteht durch einen Tag:
```powershell
git tag v0.2.0
git push origin v0.2.0
```
Danach liegt `KASSE0-v0.2.0.zip` unter *Releases* im Repository.

### Absturzberichte
Stürzt das Spiel ab (auch im Grafiktreiber), schreibt es einen Bericht nach `%APPDATA%\.mettlauncher\kasse0\crashes\` (Version, Fehler, Aufrufliste, Grafikkarte, Spielzustand, letzte Protokollzeilen) und öffnet ein Absturzfenster mit „Bericht kopieren“, „Ordner öffnen“, „Neu starten“. Das laufende Protokoll steht in `%APPDATA%\.mettlauncher\kasse0\logs\latest.log`, auch im Release-Build ohne Konsole. Mit `kasse0.pdb` neben der exe enthält die Aufrufliste Funktionsnamen und Zeilen. Die Original-Packs (`assets_src/`) sind nicht im Repository; gebaut wird aus den konvertierten Dateien in `assets/`.
