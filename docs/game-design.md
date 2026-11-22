# KASSE 0: Game Design (lebendes Dokument)

## Prämisse
Du willst ganz normal einkaufen. Du gehst in die Getränkeabteilung, kommst wieder heraus, und auf einmal ist alles komisch.

Der Laden heißt **KAUF NULL** und sieht aus wie ein moderner deutscher Discounter (Designsprache: Holz und Grau, dunkelblaue Banderolen mit weißer Schrift, gelbe Preisleisten, grüne Kassenschilder, Ware in offenen Kartons). Keine echten Marken, Logos oder Slogans, nur eigene Sprüche mit Horror-Twist.

Analog/liminaler Horror, 3D First-Person, kurze Runs mit zufälligem Seed, damit man das Spiel immer mal wieder kurz anwerfen kann.

## Ablauf eines Runs
1. **Normaler Markt:** hell, Tageslicht an der Tür. Der Einkaufszettel wird zufällig erzeugt; der Seed bestimmt auch den Grundriss.
2. **Schwelle Getränkeabteilung:** Betreten und wieder Verlassen kippt den Markt. Er ist dann leer, die Neonröhren brummen und flackern, die Gänge sind länger, und draußen hinter der Glastür ist es schwarz.
3. Den **Zettel abarbeiten** und die Waren in den Einkaufswagen legen.
4. **Der Warenwert im Wagen (€) bestimmt die Eskalation**, nicht die Anzahl der Artikel.
5. An der **Kasse** den Wagen aufs Band räumen, die gesichtslose Kassiererin scannt (das dauert und ist verwundbare Zeit), am **Kartenterminal** bezahlen, dann den Laden verlassen. Damit ist der Run gewonnen.

## Kunden (uncanny)
- Sie roleplayen das Einkaufen und greifen immer nur zu einer Sorte **abgelaufenem Tomatensaft**.
- **Nicht stören:** nicht anrempeln, nicht im Weg stehen, nicht ihren Tomatensaft nehmen.
- **Nicht auffallen:** nicht rennen, nicht anstarren, keine falschen Waren in den Wagen.
- **Nicht hinsehen:** Sobald du wegschaust (außer Sicht oder verdeckt), gehen sie langsam auf dich zu. Erreichen sie dich, ist der Run verloren.

## Gesichtslose Mitarbeiter
- Sie sprechen dich an, verfolgen dich und verwickeln dich in einen Dialog mit Antwortoptionen.
- Gute Antworten bringen Abstand und Zeit. Falsche Antworten erhöhen die Auffälligkeit, und sie folgen dir weiter.

## Auffälligkeit
Ein verstecktes Meter. Es wird über stärkere VHS-Störungen und Tracking-Fehler im Bild sichtbar, nicht über das HUD.

## Eskalation nach Warenwert (Startwerte)
| Warenwert | Stufe | Effekte |
|---|---|---|
| < 10 € | Ruhig | falsche Etiketten („Mlich“), unsinnige Preise, die Musik leiert leicht |
| 10–25 € | Unruhig | Durchsagen („Kunde Nummer 0 bitte zur Kasse 0“), mehr Kunden, die Gänge verlängern sich |
| 25–50 € | Falsch | Gänge werden dunkel, die Kunden kommen schneller näher, Mitarbeiter patrouillieren |
| > 50 € | Kollaps | Grundriss verschiebt sich, wenn man nicht hinsieht; die Kasse wandert |

Umgesetzt in `src/game/escalation.rs`.

## Risiko-Idee
Jeder Zettelartikel gibt es in einer billigen und einer teuren Variante. Die billige liegt an schwer erreichbaren oder gefährlichen Stellen.

## Look
- Render-Target 480×270, mit Nearest-Filter hochskaliert.
- VHS-Post-Pass: Zeilen-Wobble, Tracking-Band, chromatische Verschiebung, Rauschen, Scanlines, Vignette, Posterize.
- Box-Geometrie für Einrichtung und Raum, prozedural.
- Jede Ware ist ein echtes Modell (Kenney Food Kit, Kenney Mini Shop, Quaternius Ultimate Food, einzelne Poly-Pizza-Modelle). Statisch instanziert: ein Draw-Call pro Modelltyp, 12.000–21.000 Waren bei der VSync-Grenze (~170 fps, RTX 5090).
- Kundschaft und Personal sind animiert (Skinning im Vertex-Shader): 32 Figuren mit je 4 Knochen-Gewichten pro Vertex, Animationen Stehen, Gehen, Greifen und Laufen als Knochen-Matrizen (Quaternius-Skelett, 62 Knochen). Die CPU tastet pro Figur Clip und Zeit ab und mischt zwischen zwei Bildern. Kunden greifen ins Regal, solange man hinsieht, und gehen los, sobald man wegsieht.
- Schriftzüge kommen aus einem Schild-Atlas (Archivo Black, beim Start mit `ab_glyph` gerendert) und zerfallen im gekippten Markt (Flecken, ausfallende Buchstaben).

## Meilensteine
- [x] M0 Vulkan-Grundgerüst
- [x] M1 3D-Kamera, Low-Res-Offscreen, VHS-Post, Nebel, Neonlicht
- [x] M2 Seedbarer Markt-Generator, Kollision, Getränke-Schwelle
- [x] M3 Einkaufen: Waren (billig/teuer/falsch), Wagen, Zettel-UI, Warenwert, Kasse mit Scannen, Ausgang
- [x] M4 NPCs: Kunden (Tomatensaft, Nicht-Hinsehen, Stör-Regeln), Mitarbeiter-Dialoge, Auffälligkeit
- [x] M5 Eskalation (Gänge länger, dunkler, schneller, Umbau und wandernde Kasse) und Audio
- [x] Hauptmenü (Neuer Einkauf, Optionen, Beenden), Pause, minimales HUD, End-Screens mit Kassenbon
- [x] Discounter-Umbau: Warenkatalog, echte Modelle für jede Ware, Abteilungen und Unterabteilungen, Verschiebung nach Eskalation, Schilder und Werbung, Kasse mit Band und Kassiererin, Fensterfront

## Der Wagen als Risiko
- **Quietschen:** Wer den Wagen schiebt, macht Lärm. Mitarbeiter im Umkreis von 14 m (20 m beim Rennen) kommen nachsehen.
- **Zu breit:** Kollisionsradius 0,42 m gegen Durchgänge von 0,75 m. Ein Mensch (0,3 m) kommt durch, der Wagen nicht.
- **Aufräumende Mitarbeiter:** Ein Wagen, der weiter als 6 m vom Spieler entfernt steht, wird zur Wagenbox geschoben (Durchsage). Ansprechen mit E: "Hey, das ist mein Wagen!" Danach folgt ein Dialog mit den üblichen Folgen, und er lässt den Wagen los. Danach hat er 40 s Pause vom Aufräumen.
- **Ausräumen aufs Band** geht nur, wenn der Wagen an der Kasse steht (bis 3,2 m). Einzelne Ware aus der Hand kann man immer aufs Band legen.

## Veränderungen im Markt
| Stufe | Gänge | Durchgänge/Wände | Umbau |
|---|---|---|---|
| ruhig | 1,6× länger, gerade | – | – |
| unruhig | 2× länger, leicht gebogen | Durchgänge | alle 45 s (beim Wegsehen) |
| falsch | stark gebogen (S-Kurven) | Durchgänge + 2 Wände | alle 35 s |
| Kollaps | stark gebogen (S-Kurven) | Durchgänge + 3 Wände | alle 20 s, die Kasse wandert |

Neue Wände werden nur gebaut, wenn danach Getränkeraum, Kassen, Wagenbox und alle Waren erreichbar bleiben. Röhren wechseln alle paar Sekunden zufällig ihr Flacker-Muster. Die Werbeposter (ein Maskottchen-Gesicht, prozedural im Shader) verrotten mit Stufe und Zeit: Augen werden zu Löchern, es fließen blutige Tränen, das Lächeln kippt in einen aufgerissenen Mund. Fata Morganen erscheinen alle 14–28 s in 9–20 m Entfernung und verschwinden, wenn man auf 5 m herankommt.

## Sortiment und Abteilungen
- Katalog `tools/catalog.json`: 19 Unterabteilungen (Obst, Gemüse, Brot, Milch, Käse, Fleisch, Fisch, Fertiggerichte, Eis, TK, Nudeln, Konserven, Frühstück, Kaffee, Süßes, Snacks, Drogerie, Haushalt, Getränke), jede mit einer Einrichtungsart (Obstkisten, Regal, Kühlwand, Truhe).
- Jeder Zettelpunkt hat billige und teure Varianten. Billige liegen nur auf dem untersten oder obersten Boden. Der Generator stellt sicher, dass jede Zettel-Ware irgendwo erreichbar im Regal steht.
- Mischstufen nach Eskalation: **ruhig** logisch, **unruhig** Unterabteilungen tauschen innerhalb ihrer Einrichtungsart, **falsch** Abteilungen landen in fremden Einrichtungen, **Kollaps** 35 % der Ware ist fremd und die Schilder zeigen die logische statt der echten Belegung.
- Im gekippten Markt sind 6 % der Zettel-Ware falsch beschriftet (verdrehter Name, unsinniger Preis).

## Kasse
- Band mit Warentrennern, Scannerfenster, Ablage, Kartenterminal auf der Stange, sitzende Kassiererin (Quaternius-Mannequin, Pose `Driving_Loop`), Schrank mit Zigaretten-Rollo, Überwachungskamera, Quengelware.
- Ablauf: Wagen ausräumen (0,35 s pro Ware) oder Ware einzeln aufs Band legen; Band 0,45 m/s; Scannen 0,9 s pro Ware, nur solange man an der Kasse steht; falsche Ware wird aussortiert (+0,15 Auffälligkeit); bezahlen am Terminal, wenn der ganze Zettel gescannt ist.
- Gekippt: alle Kassen zeigen "0 Kasse", das Rollo ist offen (Zigaretten), die Kameras folgen dem Spieler.

## Umsetzung im Detail
| Regel | Umsetzung |
|---|---|
| Nicht hinsehen | Sichtkegel und Sichtlinie (Strahl gegen Regale). Unbeobachtete Kunden laufen über ein Distanzfeld (Breitensuche) zum Spieler. |
| Erwischt | Ein Kunde unter 0,8 m beendet den Run ("DU BIST AUFGEFALLEN"). |
| Nicht stören | Anrempeln: +0,15. Ihren Tomatensaft nehmen: +0,35, und dieser Kunde wird schneller. |
| Nicht auffallen | Rennen in Sichtweite: +0,15/s. Anstarren (über 1,5 s): +0,12/s. Falsche Ware: +0,25. Unbekannter Artikel an der Kasse: +0,15. |
| Auffälligkeit 100 % | Die Kunden bewegen sich auch unter Beobachtung (langsam). |
| Gesprächs-Schutz | Während eines Gesprächs und 2,5 s danach kann kein Kunde erwischen. |
| Vorbeisprinten | Wer schneller als 2,4 m/s an einem Mitarbeiter vorbeirennt, wird nicht angesprochen, bekommt aber +0,2 Auffälligkeit und wird 12 s verfolgt. |
| Mitarbeiter | Patrouillieren, sprechen an, 9 s Antwortzeit. Gut: 25 s Ruhe. Neutral: +0,08. Schlecht oder Schweigen: +0,3 bzw. +0,2, danach 20 s Verfolgung. |
| Kollaps | Der Umbau passiert nur, während man sich umdreht oder Boden bzw. Decke anschaut (spätestens nach 60 s). |

## Assets
Möglichst alles aus bestehenden, freien Assets, nur grob an den Stil angepasst (Polygone reduziert, Farben statt Texturen, Posen eingefroren). Gesichtslose Mannequins von Quaternius (CC0) in eingefrorenen Posen (Idle, Greifen ans Regal, Reden, Gehen, Sitzen), Waren aus Kenney- und Quaternius-Packs (CC0), Kasse, Stuhl, Rollo, Kamera, Terminal, Warnschild und Dose von Poly Haven (CC0), Einkaufswagen von Poly by Google (CC-BY 3.0). Details stehen in `CREDITS.md`.

## Technik: gebogene Gänge
Die Engine kann Boxen um die Hochachse drehen. Im Shader steht der Winkel in `center.w`, für Kollision, Sichtlinie und Navigation gibt es `world::Obb`. Jede Gondel folgt einer gemeinsamen S-Kurve `x(z) = x0 + A·sin(φ + z/L·k·π)`, mit leicht unterschiedlicher Amplitude pro Gondel, damit die Gänge mal enger, mal weiter werden. Sie wird aus etwa 0,75 m langen, gedrehten Teilstücken zusammengesetzt (Böden, Preisleisten und Waren folgen dem Bogen). Ein ausgelassenes Teilstück ergibt einen Durchgang.

## Feinschliff v0.2
- **Wagen:** F nimmt die anvisierte Ware heraus (leuchtet auf, Name im Prompt), Mausrad blättert. Quietschen alle 3–5,5 m (beim Rennen 1,6–3 m): leise beim Schieben, Klappern in Kurven, schrill beim Rennen; Lärmradius 8/10/18 m.
- **Kunden:** normale Kundschaft bummelt (eigenes Regal, Plätze in 7 m Umkreis). Verfolger pro Stufe 1/2/3/4, dazu jeder, dem man den Tomatensaft genommen hat. Verfolger spielen Kundschaft, solange man hinsieht; nur sie können einen erwischen.
- **Personal:** kräftig blaue Dienstkleidung, schwarze Hosen, gesichtslos, leichtes Eigenleuchten.
- **Weniger Bildstörung:** VHS-Effekt, Nebel und tote Röhren pro Stufe deutlich reduziert; der Schrecken kommt aus dem Markt selbst.
- **Verfall pro Stufe** (ruhig/unruhig/falsch/Kollaps): Bodenlöcher 0/1/3/5 (Hindernisse), Deckenlöcher 0/2/4/7, verdorbene Ware 0/3/8/15 % (Maden, Fliegen, kurios schwebend/gekippt/kopfüber), Riesenwaren 0/0/1/3. Live-Verbiegen der Gänge (seitlich/Steigung in m): 0/0, 0,06/0,18, 0,15/0,35, 0,25/0,55. Gleiche Formel im Shader und im Spiel (`world::sway`), Spieler, Wagen und Figuren gehen die Steigung mit.

## Feinschliff v0.3
- **Kunden wieder gefährlich:** Kunden 4/6/8/10, davon Verfolger 2/3/5/7, Grundtempo 0,45/0,6/0,8/1,05 m/s, beim Rennen des Spielers ×2,2. Unbeobachtete normale Kunden bleiben stehen und starren herüber (bis 16 m).
- **Durchgänge für NPCs:** Navigationsraster 0,25 m mit 0,18 m Abstand; Kollisionskörper neben Durchgängen um 3,5 cm gekürzt (Menschen passen durch, der Wagen nicht).
- **Auffälligkeits-Meter** unten links (nur gekippt).
- **Kasse abgesperrt** (Flatterband, beide Seiten der Gassen), bis der Zettel komplett ist (Wagen, Band, Gescanntes zählen).
- **Verfall in Reihenfolge:** Stufe 1 fehlende/gesprungene Fliesen und Ameisenstraßen; Stufe 2 dazu Fliesenstapel, Bodenlöcher, Ratten, Deckenlöcher mit Kabeln; Stufe 3 Blut tropft von der Decke in Lachen. Bodennahes trägt die Markierung `seed = -1` und folgt beim Verbiegen nur der Höhenwelle.

## Feinschliff v0.4
- **Tempo:** Spieler rennt 2,56 m/s (−20 %), Ausdauer 5 s Rennen, 4 s bis voll, erschöpft bis ein Drittel. Verfolger beim Rennen ×1,6.
- **HUD:** Ausdauer- und Auffälligkeitsleiste nur, wenn etwas fehlt bzw. anliegt.
- **Drei Leben** (`game/death.rs`): zweimal erwischt = Todesanimation (2,2 s: herumreißen, zusammensacken, Abblende), Aufwachen am Eingang mit 5 s Schonfrist; Wagen bleibt am Todespunkt; Verfolger im Umkreis von 8 m gehen an ihr Regal zurück; rote Vignette +0,25 je Tod. Dritter Fang = Game Over.
- **Fata Morgana:** wirkt echt (undurchsichtig, normale Farben), entpuppt sich ab 3 m, flackert 0,8 s, ist weg.
- **Live-Verbiegen** nur auf hohen Stufen: falsch 0,04/0,08 m, Kollaps 0,25/0,55 m.
- **Bereiche wandern** beim Umbau: ab unruhig tauschen Kühlabschnitte die Plätze, ab falsch wandert die Tiefkühlreihe in einen anderen Gang und die Obst-Insel rückt.
- **Blut** rund (vier gedrehte Rechtecke), Tropfnasen, mehrere Tropfstellen je Fleck, alle 0,7–1,5 s ein Tropfen; ab falsch etwas, im Kollaps viel. **Kabel** bunt, teils geknickt, mit Kupferenden; manche sprühen Funken (Knistern bis 14 m).
- **Wagen:** Waren liegen auf dem Korbboden (0,57 m). **Flüstern**, wenn sich zwei Kunden begegnen (1,4 m), danach 9–18 s Ruhe.

## Feinschliff v0.5
- **Eskalation erreichbar:** Stufe = Maximum aus Warenwert (10/25/50 €), Zettel-Fortschritt (40 %/70 %/komplett) und Zeit im gekippten Markt (3/6/9 min). Der komplette Zettel löst den Kollaps aus: der Weg zur Kasse ist das Finale.
- **Füße rutschen nicht mehr:** Die Lauf- und Rennanimation läuft mit der zurückgelegten Strecke (`people::Gait`, gemessene Schrittgeschwindigkeit: Männer 1,21 m/s gehend, 2,5 m/s rennend; Frauen 0,97 bzw. 2,0 m/s). Ab 1,7 m/s rennen Figuren. Bodenkontakt aller Clips gemessen: Sohle höchstens 3 cm über bzw. 2 cm unter dem Boden.

## Jumpscares (v0.6)
Selten und aus den Spielregeln heraus, nie zwei hintereinander: nach einem großen Schreck 75–130 s Ruhe (Start: 45 s). Kleine Schrecken ab „unruhig“, große erst ab „falsch“. Vor angekündigten Schrecken wird es kurz still (Musik `hush`).
- **Tod:** das leere Gesicht des Fängers kurz in Großaufnahme, dann kippt das Bild weg.
- **Umdrehen:** Wer abgelenkt ist (Zettel offen, Blick ins Regal), hat manchmal einen Verfolger 1,6 m hinter sich, der wartet, bis man sich umdreht.
- **Kassiererin** steht nach dem letzten gescannten Artikel ruckartig auf und beugt sich vor.
- **Wagen:** Ein abgestellter, unbeobachteter Wagen ist irgendwann weg und rollt dann quietschend von selbst heran.
- **Fata Morgana:** Manche Hologramme drehen beim Auflösen den Kopf ruckartig zum Spieler.
- **Deckenloch:** Unter einem Loch stürzen Kabel funkensprühend herab; im Kollaps hängt beim Hochschauen jemand kopfüber darin.
- **Überwachungsmonitor:** Hinter jedem Scanner hängt ein Monitor mit dem Livebild der Deckenkamera (zweite Kamera, Render-to-Texture 160×90, grau mit Zeilen und Rauschen). Man sieht sich selbst von hinten. Schaut man ab „unruhig“ 1,2 s hin, steht auf dem Bild eine gesichtslose Gestalt hinter einem, kommt näher, dann der Schreck. Dreht man sich um: niemand da. Einmal pro Run.
- **Optionen → „Jumpscares“:** lässt sich nicht abschalten. Der Versuch löst einen Jumpscare aus: „Peasant, did you really think...“
