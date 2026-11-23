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
5. An der **Kasse** beginnt das **Kassen-Minispiel**: Ware richtig aufs Band, die Fragen der gesichtslosen Kassiererin beantworten, am **Kartenterminal** die Karte durchziehen und die PIN eintippen (oder manchmal bar und passend zahlen), ohne ihr aufzufallen.
6. **Flucht:** Nach dem Bezahlen „Kunde 0 hat ein Gesicht“, der Alarm geht los, alle rennen auf einen zu. Wer den Ausgang erreicht, hat den Run gewonnen.

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
- Band mit Warentrennern, **Theken-Scanner** aus Boxen (Glasfenster in der Theke mit rotem Laser-Kreuz und senkrechter Scanner-Turm mit Glasfront), Ablage, **Kasse** (POS-Station mit Bildschirm, Schublade und Bondrucker, zur Kassiererin gedreht), Kartenterminal auf der Stange, **Bürostuhl** 0,95 m neben der Bandmitte, darauf die **sitzende Kassiererin** (gesichtslose Personal-Figur, fest aus dem Seed; Sitz-Animation aus der Universal Animation Library übertragen, siehe unten; dreht sich im gekippten Markt mit dem Stuhl zum Spieler), Schrank mit Zigaretten-Rollo, Überwachungskamera, Quengelware-Regal mit 1,3 m freiem Gang zum Band.
- **Sitz-Animation** (Clip `sit`, letzter Eintrag in `CLIPS` in `tools/prepare_people.py`): `Sitting_Idle_Loop` aus der Universal Animation Library 1, übertragen wie die übrigen UAL-Clips. Mit `KASSE0_ADD_CLIPS=sit` erzeugt das Skript nur diesen Clip und hängt ihn an die vorhandenen `anim_*.k0a` an (dafür reicht UAL1, UAL2 wird nicht gebraucht). Die Figur steht `SIT_BACK` = 0,25 m vor dem Stuhl und `SIT_UP` = 3 cm höher, dann liegt das Gesäß auf der Sitzfläche (Test in `src/people.rs`).
- **Kassenfenster** (`src/game/belt_window.rs`, Anzeige `src/hud/belt_window.rs`): E am Band öffnet ein eigenes Fenster, der Markt dahinter wird abgedunkelt, der Mauszeiger ist frei, man steht still. Links der Wagen als Kacheln (dazu die Ware in der Hand), in der Mitte das Band mit den Waren auf dem Weg zum Scanner, rechts Status der Kassiererin und der Bon mit Summe. Waren zieht man mit gedrückter Maustaste aufs Band (gleiche Regeln: Getränke zuerst ...). E oder Esc schließt, beim Bezahlen schließt es sich von selbst. Schaut sie auf, blinkt der Rahmen rot, „MAUS STILL!“: Wer die Maus jetzt bewegt (über 1 HUD-Pixel pro Bild) oder etwas zieht, fällt auf.
- **Kassenbereich gesperrt für NPCs:** Gassen plus 0,4 m Rand davor und dahinter (dort hängt auch das Flatterband). Im Navigationsraster sind diese Zellen für NPCs gesperrt (`NavGrid::npc_walkable`); wer trotzdem hineingeschoben würde, wird zurückgesetzt. Steht der Spieler drin, warten sie am Rand.
- Ablauf: Ware im Kassenfenster Stück für Stück aus dem Wagen oder aus der Hand aufs Band ziehen; Band 0,45 m/s; Scannen 0,9 s pro Ware, nur solange man an der Kasse steht; falsche Ware wird aussortiert; bezahlen am Terminal (Karte + PIN), wenn der ganze Zettel gescannt ist.

## Kassen-Minispiel (v0.5, `src/game/till.rs`)
Die Kasse ist ein eigenes Minispiel. Es beginnt mit der ersten Ware auf dem Band und endet mit dem Bezahlen.
- **NPCs harmlos:** Solange das Minispiel läuft und man höchstens 3,5 m von der Kasse entfernt ist, können Kunden nicht erwischen und Mitarbeiter sprechen einen nicht an. Im übrigen Markt bleibt alles wie bisher.
- **Drei Kassen-Leben**, getrennt von den drei Leben im Markt. Nach dem dritten ist der Run vorbei ("Die Kassiererin hat dich durchschaut.").
- **Direkte Fehler kosten sofort ein Leben** (rote Vignette, Schock). **Weiche Fehler** füllen nur das **Misstrauen** (0–100 %, Balken unten links statt der Auffälligkeit). Ist es voll, ist ebenfalls ein Leben weg, und der Balken startet wieder bei 0.

| Handlung | Regel | Fehler kostet |
|---|---|---|
| Ware aufs Band | Getränke zuerst (Getränk nach anderer Ware) | 1 Leben |
| | falsch beschriftete Ware wird beim Scannen aussortiert | 1 Leben |
| Fragen (2 pro Einkauf, 7 s Zeit, 1/2/3) | schlechte Antwort / neutrale Antwort / Schweigen | 1 Leben / 20 % / 35 % |
| Karte durchziehen (Kartenleser: E nimmt die Karte aus dem Portemonnaie, dann Maus nach rechts, Blick steht still, F bricht ab) | zu schnell (unter 0,45 s) / zu langsam (über 1,3 s oder unterwegs 0,35 s angehalten); danach noch mal. Das Display zeigt KARTE EINFÜHREN, BITTE DURCHZIEHEN, LESE ..., ZU SCHNELL, ZU LANGSAM, ANGENOMMEN | 1 Leben / 20 % |
| PIN (4 Ziffern, 2,5 s angezeigt, 10 s zum Tippen) | falsche Ziffer / Zeit abgelaufen, danach wird sie wieder angezeigt | 1 Leben / 20 % |
| Barzahlung (halbe-halbe: 50 % bar, 50 % Karte, 18–12 s je nach Stress) | genau passend hinlegen; zu viel = alles zurück / Zeit abgelaufen = alles zurück, neue 25 s | 1 Leben / 25 % |
| An der Kasse bleiben | weiter als 3,5 m weg: steigt laufend, und man ist nicht mehr geschützt | 8 % pro Sekunde |

Es gibt 15 Fragen, davon 9 absurd-lustige („Wieso Kondome und eine Gurke?“, „Haben Sie Kleingeld? Ich sammle Zähne.“, „Haben Sie Ihr Gesicht selbst ausgesucht?“). Die Ware, nach der sie fragt, gibt es im Laden gar nicht. Die gute Antwort ist immer die langweilige. Die Fragen kommen nach der 1.–2. gescannten Ware und 2–3 Waren später. Hat man weniger Ware, stellt sie die restlichen am Ende, bevor man bezahlen darf. Im Kollaps wandert die Kasse nicht, solange das Minispiel läuft.

## Druck an der Kasse (`src/game/till/pressure.rs`, `src/game/till/receipt.rs`)
Ein **Stress-Faktor** S (0..1) treibt alles: Start 0,2, +0,5 über 90 s an der Kasse, +0,2 je verlorenem Kassen-Leben.
- **Ungeduld:** Ihr Misstrauen steigt von allein um 0,004 + 0,016·S pro Sekunde (nicht beim Bezahlen). Dazu nörgelt sie alle 15–25 s („Geht das auch schneller?“), bei hohem S öfter.
- **Sie schaut auf:** alle 6–14 s geteilt durch (0,5 + S), nicht beim Bezahlen. 0,6 s Vorwarnung (Knarzen, der Scanner stoppt), dann starrt sie 1,5–2,5 s, ruckartig zum Spieler gedreht, starr und heller leuchtend. Wer sich jetzt bewegt (über 0,3 m/s) oder ihr ins leere Gesicht schaut (Kosinus über 0,97), bekommt +40 % Misstrauen, einmal pro Aufblick.
- **Atmosphäre:** Herzschlag (schneller mit S und Misstrauen), eigene Durchsagen („Kasse 0 schließt gleich. Kasse 0 schließt nie.“), flackerndes Licht und rote Pulse, und wer am Absperrband steht (bis 1,2 m), klopft.
- **Falsch gescannt:** Mit der Chance 0,08 + 0,22·S pro Ware, höchstens zweimal pro Einkauf, scannt sie doppelt oder 5–20-mal zu teuer. Das **Kassendisplay** rechts oben (letzte 8 Zeilen, Summe) markiert das nicht. **R** reklamiert und korrigiert den letzten Fehler. R ohne Fehler kostet 30 % Misstrauen. Wer bezahlt, ohne es bemerkt zu haben, verliert je Fehlscan ein Leben. Die Summe (Karte und bar) enthält die Fehlscans, bis man sie reklamiert.
- **Härteres Barzahlen:** 18 s bei S = 0 bis 12 s bei S = 1. Die Geldbörse hat 6–12 Extra-Stücke (meist Münzen), und mit der Chance 0,3 + 0,4·S fehlt ein Stück der naheliegenden Lösung (es liegt in kleineren Stücken bei). Passend zahlen geht immer.

**Geldbörse** (`src/game/till/cash.rs`): 50/20/10/5 €-Scheine, 2/1 €- und 50/20/10/5/2/1-ct-Münzen. Sie enthält immer genug, um passend zu zahlen, dazu 3–6 zufällige Extra-Stücke. Wählen mit A/D oder dem Mausrad, E (Linksklick) legt hin, F nimmt das zuletzt Hingelegte zurück. Anzeige: oben groß „NOCH OFFEN“, darunter je eine Reihe Kacheln für Scheine und Münzen (Wert und Anzahl, die gewählte gelb umrandet, leere grau).

**Die Karte** (`src/hud/till.rs`, beim Durchziehen groß zu sehen): „SCAMCART“, Nummer 0000 0000 0666, „KUNDE 0“, gültig bis 01/89, roter Stempel „ABGELAUFEN“. Kränklicher Farbverlauf von Lila nach Gallengrün, dessen Bänder leicht wabern, ein Auge, das sich umsieht und manchmal blinzelt, ein Riss, und der Schriftzug flackert ab und zu rot. Abgelaufen ist sie nur zum Spaß, die Zahlung klappt trotzdem.

Cheats (F1) zum Testen: „Kasse: direkt zur Kartenzahlung“ bzw. „… zur Barzahlung“. Sie kippen den Markt, legen den Zettel als gescannt hin, überspringen die Fragen und stellen dich ans Terminal.

## Die nutzlose Zigarette (`src/game/smoke.rs`, Anzeige `src/hud/smoke.rs`)
Ohne jeden Spielzweck. Die Zigarette ist ein 3D-Modell (Zgeal, CC-BY 3.0) vor der Kamera, unten rechts, die Glut zeigt schräg nach vorn. **1** zündet an (Feuerzeug-Ratschen). **2** nimmt einen Zug: Sie wird in 1,2 s zum Mund geführt (Filter an die Lippen), die Glut leuchtet auf und knistert, danach wird Rauch übers Bild ausgeatmet. Jeder Zug macht sie sichtbar kürzer (das Modell wird von der Glutseite her gekürzt). **3** schnippt sie weg: Sie überschlägt sich im Bogen, und die Kippe liegt 2,2 m vor einem am Boden, glimmt 12 s und verschwindet nach 40 s. **Nach dem 4. Zug** fliegt sie von allein weg. Unbenutzt brennt sie langsam von allein ab. Eine Rauchfahne steigt an der Glut auf. Während eines Gesprächs, einer Frage der Kassiererin oder beim Bezahlen gehören die Zifferntasten den Antworten bzw. der PIN, dann passiert mit der Zigarette nichts.

## Geräusche der Flucht (`src/game/escape.rs`)
Während der ganzen Flucht hört man die **Schritte der Meute**: schwere Rennschritte, ab 28 m Entfernung hörbar und lauter, je näher der nächste Verfolger ist. Je mehr Verfolger in 12 m Umkreis sind, desto dichter folgen die Schritte aufeinander (bis fast viermal so oft). Dazu der **eigene Herzschlag**: ein Schlag pro Sekunde, wenn niemand nah ist, bis zu 0,38 s, wenn sie einen fast haben (ab 15 m schneller), und dann auch lauter.

## Der Fluchtkorridor (`src/world/gen/corridor.rs`, Spiel: `src/game/corridor.rs`)
Nur im gekippten Markt nach dem Bezahlen: Hinter der Ladentür liegt statt des Windfangs ein 42 m langer, 3 m breiter Gang mit niedriger Decke (2,7 m). Schmutziger Boden, grünlich-graue Wände mit dunklem Sockel, Blutspuren, rot flackernde Röhren alle 5 m (jede vierte tot).
- **Hindernisse, bei jedem Lauf neu gewürfelt** (eigener Zufall `WorldParams::corridor_seed`, beim Bezahlen aus Uhrzeit und Spiel-Zufall, also auch beim gleichen Seed jedes Mal anders): Reihen alle 3,4–5,6 m, in jeder Reihe eine Lücke von 1,25–1,7 m an zufälliger Stelle, links und rechts davon Kistenstapel (verschieden hoch), umgekippte Regale, Paletten mit Tomatensaft oder Kartonsäulen. Dazu manchmal Kleinkram am Boden. Das Navigationsraster reicht bis ans Ende, die Verfolger rennen hinterher (im Gang 1,8 m/s statt 2,05 m/s).
- **Poster** alle 5,5 m, abwechselnd an beiden Wänden, schwarz-rot und leicht leuchtend: NICHT UMDREHEN, SIE SIND DIREKT HINTER DIR, DREH DICH NICHT UM, NUR NACH VORNE, SCHAU NICHT ZURÜCK, KUNDE 0 HAT EIN GESICHT, SIEH IN DEN SPIEGEL. Dazwischen verrottende Maskottchen-Plakate.
- **Nicht umdrehen:** Gangrichtung ist +z. Ab einem Seitenblick von gut 78° (Kosinus 0,2) wird das Bild rot und „NICHT UMDREHEN“ blinkt immer schneller. Ab gut 110° (Kosinus −0,35) ist man sofort tot, mit **Jumpscare** im Stil der anderen Schrecken: Schrei (wie beim Troll-Jumpscare), Schreck-Effekt mit Farbumkehr-Stroboskop, Lichtblitz und Zoom-Ruck, und das leere Gesicht des nächsten Verfolgers rast in 0,3 s von 1,4 m auf 0,3 m an die Kamera heran und bleibt dort zitternd 1,4 s stehen. Danach Game Over: „Du hast dich umgedreht.“
- **Spiegel** (Material 7) am Ende, 2,2 × 1,24 m (16:9). Die zweite Kamera (sonst der Überwachungsmonitor) filmt im Gang vom am Spiegel gespiegelten Auge aus, mit schiefem Frustum genau durch die Spiegelfläche. Der Shader zeigt das Bild seitenverkehrt, in Farbe, mit blinden Flecken und Kratzern. Man sieht sich selbst und die Meute hinter sich. Die Auflösung ist die des Monitors (160 × 90).
- **Ausgang** links (+x) kurz vor dem Spiegel: grell beleuchteter Raum, Schild AUSGANG. Wer dort ankommt, hat gewonnen.
- Cheat (F1 → Kasse & Flucht): „Flucht: Korridor“ setzt einen bezahlt direkt hinter die Tür.

## Flucht zum Ausgang (`src/game/escape.rs`)
Direkt nach dem Bezahlen: Durchsage „ACHTUNG: Kunde 0 hat ein Gesicht. Schnappt ihn!“, Sirene alle 1,3 s, rote Blinkschrift „ALARM - LAUF ZUM AUSGANG!“. Alle Kunden und Mitarbeiter rennen mit 2,05 m/s auf einen zu (Gehen 1,5, Rennen 2,56 m/s mit 5 s Ausdauer), egal ob man hinsieht. Wer beim Alarm näher als 10 m ist, startet an seinem Regal bzw. an einem Platz mindestens 10 m entfernt. Auch Mitarbeiter packen jetzt zu. Erwischt = sofort Game Over, ohne Aufwachen („Sie haben Kunde 0 geschnappt“). In den Kassenbereich kommen sie weiterhin nicht. Gewonnen ist, wer durch die Tür hinaus ist.
- Gekippt: alle Kassen zeigen "0 Kasse", das Rollo ist offen (Zigaretten), die Kameras folgen dem Spieler.

## Umsetzung im Detail
| Regel | Umsetzung |
|---|---|
| Nicht hinsehen | Sichtkegel und Sichtlinie (Strahl gegen Regale). Unbeobachtete Kunden laufen über ein Distanzfeld (Breitensuche) zum Spieler. |
| Erwischt | Ein Kunde unter 0,8 m beendet den Run ("DU BIST AUFGEFALLEN"). |
| Nicht stören | Anrempeln: +0,15. Ihren Tomatensaft nehmen: +0,35, und dieser Kunde wird schneller. |
| Nicht auffallen | Rennen in Sichtweite: +0,15/s. Anstarren (über 1,5 s): +0,12/s. Falsche Ware: +0,25. Unbekannter Artikel an der Kasse: zählt zum Misstrauen der Kassiererin (siehe Kassen-Minispiel). |
| Auffälligkeit 100 % | Die Kunden bewegen sich auch unter Beobachtung (langsam). |
| Gesprächs-Schutz | Während eines Gesprächs bewegt sich kein Kunde (Pose eingefroren, nur langsames Herüberdrehen), der Geisterwagen pausiert; danach 2,5 s Schonfrist, in der niemand fangen kann. |
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

## Jumpscares (v0.7, schneller und creepier seit v0.11)
**Handschrift je Schreck (v0.18):** Jeder Schreck hat einen eigenen Stil (`game/scare_kinds.rs`, `ScareKind::style`): zwei Klänge, Negativ-Flackern ja/nein, Lichtblitz, Zoom hinein (positiv) oder auseinander (negativ, das Sichtfeld wird weiter), Rotstich, Bildriss, Kamerawackeln. Beispiele: Umdrehen = dumpfer Schlag, heranziehen, wackeln; Stromausfall = greller Blitz und Rauschen; Geisterwagen = Metallkrach, starkes Wackeln; Hologramm = Rauschen, Negativ, Bildriss; kopfüber im Loch = Kreischen, Grollen, Bild reißt auseinander, rot. Neue synthetische Klänge: Kreischen (Bogen auf Metall), dumpfer Schlag, Fernsehrauschen, anschwellendes Grollen. Abwechslung: Die letzten drei Arten kommen nicht sofort wieder (`can_scare_kind`).
**Neue Schrecken (v0.18):** Kühltruhe (in eine Truhe hinabschauen: jemand liegt darin und richtet sich auf, einmal pro Markt), An der Decke (Gestalt rennt kopfüber quer durchs Bild, 4,5 m/s), Alle schauen (alle sichtbaren Kunden drehen sich gleichzeitig zu dir, erstarren 2,5 s, Flüsterchor), Schritte im Dunkeln (2 s dunkel, Schritte rennen von hinten heran, Atem im Nacken, Licht: niemand da), Regal-Gesicht (beim Greifen zu 15 % ein Gesicht zwischen der Ware), Im Wagen (Wagen 8 s unbeobachtet allein gelassen, bei der Rückkehr hockt jemand darin), Durchsage (Gong, „Kundin an Kasse Null“, dann Kreischen aus den Lautsprechern), Regal kracht (bis zu 7 Waren stürzen auf dich zu). Die fünf ortsunabhängigen kommen zufällig (3 %/s, wenn ein großer Schreck erlaubt ist), Kühltruhe, Regal-Gesicht und Wagen aus der Situation. Alle als Cheats (Schrecken, zweispaltig).
**Personal bleibt menschlich (v0.18):** Mitarbeiter und Kassiererin werden nicht verformt und drehen den Kopf höchstens 1,1 rad; nur die Kunden werden unheimlich.
**Schreck-Effekt (v0.11):** Jeder Schreck lässt 0,55 s lang das Bild im Negativ flackern (Farbumkehr-Stroboskop, 18 Hz), das Licht grell ausschlagen (Überbelichtung) und das Sichtfeld ruckartig um bis zu 30 % zusammenziehen. Ab „falsch“ zucken alle 14–30 s einzelne unterschwellige Negativ-Blitze (0,09 s) durchs Bild. Tempo: 18–32 s Ruhe zwischen Schrecken, 1,2 s Stille davor, Verfolger 1,2 m hinter dir, Stromausfall 1 s, Geisterwagen rollt mit 1,6 m/s heran und knallt vor dir zum Stehen, Monitor-Gestalt schlägt nach 0,8 s zu.
**Cheats:** Im Cheat-Menü (F1, Kategorie „Schrecken“) lässt sich jeder Schreck einzeln auslösen; der Cheat teleportiert an eine passende Stelle.

## Cheat-Menü (F1, `src/game/cheats.rs`, Anzeige in `src/hud.rs`)
Zwei Ebenen. Zuerst die Kategorien mit Anzahl und einer kurzen Beschreibung der gewählten: **Spieler** (Unverwundbar, Leben auffüllen, Tutorial überspringen, Auffälligkeit 0/100 %), **Markt & Einkauf** (kippen, Zettel füllen, +10 €, zur Getränkeabteilung, umbauen, Durchsage), **Kasse & Flucht** (zur Kasse, Karten-/Barzahlung, Flucht: Korridor), **Leute** (Mitarbeiter rufen, alle entfernen, Wagen wird mitgenommen), **Schrecken** (jeder Jumpscare einzeln). Enter oder Rechts öffnet eine Kategorie, Enter löst einen Cheat aus (und es geht weiter im Spiel), Esc oder Links geht eine Ebene zurück. Jeder Cheat steht in genau einer Kategorie (Test).
Aus den Spielregeln heraus, nie zwei direkt hintereinander: nach einem Schreck 25–45 s Ruhe (Start: 15 s nach dem Kippen). Kleine Schrecken sobald der Markt gekippt ist, große ab „unruhig“ (v0.8: deutlich häufiger als in v0.7). Vor angekündigten Schrecken wird es kurz still (Musik `hush`).
- **Tod:** das leere Gesicht des Fängers kurz in Großaufnahme, dann kippt das Bild weg.
- **Umdrehen:** Wer abgelenkt ist (Zettel offen, Blick ins Regal: 60 %/s, sonst 4 %/s), hat bald einen Verfolger 1,6 m hinter sich, der wartet, bis man sich umdreht.
- **Stromausfall:** 1,4 s stockdunkel (Funkenknall, Stille), im Dunkeln wird eine Kundin 2,6 m vor einen gestellt; wenn das Licht flackernd zurückkommt, steht sie da und starrt.
- **Kassiererin** steht nach dem letzten gescannten Artikel ruckartig auf und beugt sich vor.
- **Wagen:** Ein abgestellter, unbeobachteter Wagen ist irgendwann weg und rollt dann quietschend von selbst heran.
- **Fata Morgana:** Ab „unruhig“ drehen 60 % der Figuren-Hologramme beim Auflösen den Kopf ruckartig zum Spieler.
- **Deckenloch:** Unter einem Loch stürzen Kabel funkensprühend herab; im Kollaps hängt beim Hochschauen jemand kopfüber darin.
- **Überwachungsmonitor:** Hinter jedem Scanner hängt ein Monitor mit dem Livebild der Deckenkamera (zweite Kamera, Render-to-Texture 160×90, grau mit Zeilen und Rauschen). Man sieht sich selbst von hinten. Schaut man im gekippten Markt 0,8 s hin, steht auf dem Bild eine gesichtslose Gestalt hinter einem, kommt näher, dann der Schreck. Dreht man sich um: niemand da. Wiederholt sich nach der Abklingzeit.
- **Optionen → „Jumpscares“:** lässt sich nicht abschalten. Der Versuch löst einen Jumpscare aus (3,4 s): weißer Blitz, gesichtsloses Gesicht 24 cm vor der Kamera, Stroboskop, Bildwackeln, voller Rotfilter und Bildriss, verzerrter Schrei (`Sfx::Shriek`) mit Basseinschlag; dann knallt Zeile für Zeile „PEASANT, / did you really think... / ...you could turn us off?“ zitternd ins Bild, mit zweitem Einschlag.

## NPC-Overhaul (ab v0.14)
Ziel: Die Figuren selbst sollen unheimlich sein, nicht nur die Scares. Erst uncanny-menschlich, ab „falsch“ zusätzlich körperlich verformt. Umsetzung in vier Etappen (Animation, Navigation und Wahrnehmung, Verhalten mit Director, Verformung und Klang).
- **Etappe 1 (v0.14):** Animationen mit Knochen-Hierarchie (`.k0a` v2: lokale Rotation, Position, Skalierung je Knochen und Bild), Mischen per slerp, Überblenden zwischen Clips (0,25 s, `people::Animator`), Einmal-Clips. 23 Clips: die eigenen der Modular-Figuren plus 15 aus der Universal Animation Library (übertragen: Arme und Beine übernehmen die Knochenrichtung, alles andere die Bewegung relativ zur Ruhepose; Füße danach auf den Boden). Eingriffe pro Knochen (`people::PoseFx`): Kopf zum Blickziel (begrenzter Winkel), Zucken, gestreckter Hals und Arme, Stop-Motion. Erste sichtbare Wirkung: Im gekippten Markt folgen dir die Köpfe der Kunden in bis zu 14 m, während der Körper weitermacht. Wie weit sie sich drehen, wächst mit der Stufe (ruhig 1,2 rad, unruhig 1,6, falsch 2,3, Kollaps 180°). Personal schaut einen in 9 m an, aber nur menschlich weit (seit v0.18).
- **Etappe 2 (v0.15):** Navigation: gewichtete Distanzfelder (gerade 5, diagonal 7), Wegglättung (12 Zellen voraus, die am weitesten frei erreichbare Zelle wird angesteuert, ein eine Zelle breiterer Streifen muss frei sein), bei Hängenbleiben Zelle für Zelle; Startzelle am Rand eines Durchgangs = Nachbar mit kleinstem Feldwert (nicht die nächste hinter einem dünnen Regal). Der rollende Geisterwagen hat ein eigenes Feld ohne die engen Durchgänge. Ziele werden nur gewählt, wenn sie erreichbar sind; Feststecken (1,5 s, < 20 cm) führt zu neuem Ziel bzw. bei Jägern zu einem Ausweichschritt. Alle Menschen halten 0,55 m Abstand zueinander, 0,45 m zum Spieler, 0,55 m zum abgestellten Wagen (jagende Verfolger drängen sich am Wagen vorbei). Drehungen überall weich.
  Wahrnehmung (`game/senses.rs`): Verfolger sehen rundum 22 m (unter 3 m immer), Personal mit Sichtkegel (±63°, 9 m, unter 2,5 m immer); Prüfung alle 0,25 s. Geräusche: Schritte rennend 15 m, gehend 3,5 m, Wagenquietschen wie bisher (8/10/18 m), aufschlagende Ware 8 m. Gedächtnis: zuletzt bemerkte Position; „frisch“ (gesehen, < 0,6 s) = direkter Weg zum Spieler, sonst zur gemerkten Stelle, dort im Umkreis von 4 m suchen, nach 25 s vergessen. Verdacht 100 % und die Flucht machen weiterhin allwissend. Personal geht Geräuschen nach und sieht sich 2,5 s um. Zange: Von den jagenden Verfolgern flankiert jeder zweite (nicht der nächste, ab 6 m); sein Weg meidet einen 1,2 m breiten Streifen entlang der Linie vom vorderen Verfolger zum Spieler.
  Gemessen: 0,11 ms pro Spiel-Update im Kollaps mit 13 NPCs (Release).
- **Etappe 3 (v0.16):** Verhalten (`game/brain.rs`) und Director (`game/director.rs`).
  Tätigkeiten im Stehen je Figur (fest aus dem Zufallswert): Greifen, Endlosschleife (PickUp wird bei Ende neu gestartet), Telefonieren, Arme verschränkt, Reden; die normale Kundschaft vor dem Kippen ebenso. Nachahmen (gekippt, bis 12 m, nur sichtbare, nicht jagende Kunden): Spieler steht 2 s still → sie erstarren (Pose und Position eingefroren), Spieler greift ins Regal → sie greifen 1,3 s mit, Spieler starrt (Kosinus > 0,995, < 8 m) → sie drehen sich weg. Ertappt: Verfolger, der beim Heranschleichen gesehen wird, erstarrt 1,8 s mitten im Schritt. Gang der Jäger ab „falsch“ geduckt (CrouchWalk/-Idle), im Kollaps Zombie-Gang; unter 1,4 m holen sie 0,8 s zum Griff aus (Grab-Clip, 30 % schneller), gefangen wird weiter unter 0,8 m. Personal: 45 % der Streifen führen zu einem Regalplatz, dort kniend Einräumen 6–12 s (ab „falsch“ 20–40 s), Wagen wegbringen mit Schiebe-Animation, Gespräch mit Rede-Animation.
  Director: Aufbau (Druck 0,35 → 1 in 55 s; die nächsten ⌈Druck × Anzahl⌉ Verfolger mit frischem Wissen rücken heran, die anderen lauern ab 9 m und beobachten), Höhepunkt 20 s (alle, keine Scares), Ruhe 15 s (niemand rückt heran). Nach einem Tod 20 s Ruhe, nach einem großen Schrecken (Stärke ≥ 0,9) 8 s; eine laufende längere Ruhe wird nicht verkürzt. Im Gespräch und vor dem Kippen steht die Kurve. Verdacht 100 % und die Flucht übergehen den Director.
- **Etappe 4 (v0.17):** Verformung je Stufe (`Game::customer_fx`): unruhig Zucken 0,15 und Stop-Motion-Fenster (8 Bilder/s); falsch Hals/Arme gestreckt 0,3, Zucken 0,45, Stop-Motion 10 Bilder/s beim Gehen; Kollaps Strecken 0,75, Zucken 0,75, Stop-Motion 7 Bilder/s, beim Ins-Blickfeld-Kommen zu 20 % 2 s lang Kopf kopfüber (`PoseFx::head_roll`). Direkt hinsehen (Kosinus > 0,97, < 12 m) ab „falsch“: Zerreißen im Skin-Shader (Instanz-Extra z: Scheiben verrutschen, Fläche flimmert; falsch 0,25, Kollaps 1,0, flackernd).
  Klang mit Ort (`Game::sfx_at`, Stereo-Panorama in `Audio::play_at`, hörbar bis 18 m): Schritte alle 0,65 m (Verfolger auf der Jagd lauter, rennend am lautesten, im Kollaps synthetisches Schlurfen), Personal mit Schlüsselbund alle 6 m, normale Kundschaft leise; gesehene Kunden sind stumm. Flüstern kommt aus der Richtung der Flüsternden. Alle neuen Klänge sind synthetisiert (keine neuen Assets).
  Cheats (Kategorie „NPCs & Stufen“): Stufe +1, Director Höhepunkt/Ruhe, Verfolger 6 m hinter dir, NPC-Zustände über den Köpfen (kauft, Verfolger, jagt, lauert, sucht, erstarrt; Personal: Streife, kommt, horcht, räumt ein, Wagen, redet; „x“ = gemerkte Stelle) plus Director-Phase oben links.
- **Gespräche:** Während eines Dialogs bewegt sich kein Kunde (v0.12.1).

## Kartenterminal (v0.20.8)
Eigenes Modell, ausdrücklich gewünschte Ausnahme von der Regel „keine selbst gebauten Modelle“: Quelle `tools/models/kartenterminal.blend`, umgewandelt mit `tools/prepare_assets.py` (`KASSE0_ONLY=terminal`). Durchzieh-Rille mit Pfeilen nach rechts (wie im Minispiel), Display, Tastenfeld mit Sichtschutz, Chip-Schlitz, Spiralkabel, 3532 Dreiecke.

## Passende Ware leuchtet (v0.21)
`game/highlight.rs`: Die nächste Ware im Umkreis von 1 m (in der Ebene), die zu einem offenen Zettel-Eintrag gehört, pulsiert hell und wird 6–10 % größer gezeichnet. Immer nur eine, entschieden nach Abstand zum Auge; eine andere muss 5 cm näher sein, damit es nicht flackert. Teure, verdorbene und falsch beschriftete Ausgaben zählen mit, die Hervorhebung verrät die Fälschung nicht. Auch fallen gelassene Ware leuchtet. Nicht durch Wände oder auf die andere Regalseite (Sichtlinie bis 40 cm vor die Ware). Aus mit Ware in der Hand, im Gespräch, beim Kassieren und beim Sterben.
