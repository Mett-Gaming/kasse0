# Credits

Spiel, Engine, Code, Shader und alle Sounds (synthetisiert): eigenes Projekt.

## 3D-Modelle (grob an den Stil angepasst: Polygone reduziert, Texturen durch Durchschnittsfarben ersetzt, Posen eingefroren)

| Modell | Quelle | Autor | Lizenz |
|---|---|---|---|
| Kundschaft und Personal (24 + 8 Varianten, Teile gemischt, Personal in Dienstkleidung umgefärbt und gesichtslos) | [Ultimate Modular Men Pack](https://poly.pizza/bundle/ZiH8muWqwQ), [Ultimate Modular Women Pack](https://poly.pizza/bundle/aCBDXDdTNN) | Quaternius | CC0 1.0 |
| Mannequin in Sitzpose (frühere Kassiererin, noch in den Spieldateien) und Sitz-Animation `Sitting_Idle_Loop` (auf die Personal-Figur der Kassiererin übertragen) | Universal Animation Library 1 | Quaternius | CC0 1.0 |
| Figuren-Animationen: Ducken, Zombie-Gang, Aufstehen, Arme verschränkt, Telefonieren, Reden, Aufheben, Wagen schieben, Tragen, kniend Einräumen, Kopfschütteln, förmliches Gehen (auf das Skelett der Modular-Figuren übertragen) | [Universal Animation Library](https://quaternius.itch.io/universal-animation-library) 1 und [2](https://quaternius.itch.io/universal-animation-library-2) | Quaternius (mit Gonzalo Furnier) | CC0 1.0 |
| Einkaufswagen ("Shopping cart"), im Spiel verändert: Rahmen entfernt, Drahtkorb und Gestell aus Stäben ergänzt (Räder und Griff aus dem Original) | [poly.pizza/m/4lhRcMbl1pw](https://poly.pizza/m/4lhRcMbl1pw) | Poly by Google | **CC-BY 3.0** |
| Ratte ("Rat") | [poly.pizza/m/6hsesZHvcPI](https://poly.pizza/m/6hsesZHvcPI) | Poly by Google | **CC-BY 3.0** |
| Zigarette ("Cigarette") | [poly.pizza/m/2RL6h8MbDPb](https://poly.pizza/m/2RL6h8MbDPb) | Zgeal | **CC-BY 3.0** |
| Kasse ("POS Machine") | [poly.pizza/m/ju1CrAV1UY](https://poly.pizza/m/ju1CrAV1UY) | J-Toastie | **CC-BY 3.0** |
| Stuhl der Kassiererin ("Desk Chair") | [poly.pizza/m/CKSz6PB1vO](https://poly.pizza/m/CKSz6PB1vO) | Kenney | CC0 1.0 |
| Warnschild ("Wet Floor Sign 01"), im Spiel mit Aufschrift und Warndreieck versehen | [polyhaven.com/a/WetFloorSign_01](https://polyhaven.com/a/WetFloorSign_01) | Poly Haven | CC0 1.0 |
| Konservendose ("Russian Food Cans 01") | [polyhaven.com/a/russian_food_cans_01](https://polyhaven.com/a/russian_food_cans_01) | Poly Haven | CC0 1.0 |

### Waren-Packs (der Grossteil der rund 125 Waren im Katalog `tools/catalog.json`)
| Pack | Autor | Lizenz |
|---|---|---|
| [Food Kit](https://kenney.nl/assets/food-kit) | Kenney | CC0 1.0 |
| [Mini Market / Mini Shop](https://kenney.nl/assets/mini-market) | Kenney | CC0 1.0 |
| [Ultimate Food Pack](https://quaternius.com) | Quaternius | CC0 1.0 |

Bezogen ueber opengameart.org, Quelldateien in `assets_src/packs/`. Kenney-Modelle teilen sich eine Farbtextur; die Farbe wird pro Dreieck abgetastet (`tools/prepare_products.py`).

### Einzelne Waren (von [poly.pizza](https://poly.pizza), Link: `https://poly.pizza/m/<ID>`)
| Ware | Modell (ID) | Autor | Lizenz |
|---|---|---|---|
| Mineralwasser | Water Bottle (KpxDpidn1Z) | Quaternius | CC0 1.0 |
| Butter | Butter (awZTt6mwv70) | Poly by Google | CC-BY 3.0 |
| Nudeln | Carton Small (9upNan7f8q) | Kenney | CC0 1.0 |
| Reis | Sack (yNTXvNLVyw) | Pixel | CC-BY 3.0 |
| Kaffee | Bag (fLNcjJnsJi) | Kenney | CC0 1.0 |
| Batterien | AA battery (2TKK0wpiWPw) | Poly by Google | CC-BY 3.0 |
| Klopapier | Toilet Paper stack (6jlZSAxsYb) | Quaternius | CC0 1.0 |
| Spülmittel | Hand Wash Bottle (d4b5btUs1F) | MiniPoly | CC0 1.0 |
| Zahnpasta | Lotion Tube (AmwlZLfBQr) | MilkAndBanana | CC0 1.0 |
| Kekse | Oreos (feX-JtR3bCX) | Jarlan Perez | CC-BY 3.0 |
| Kerzen | Candle (8apROCXjeN4) | Jarlan Perez | CC-BY 3.0 |
| Glühbirne | Light bulb (eKaX3HRxj58) | jeremy | CC-BY 3.0 |

Alle CC-BY-3.0-Modelle (Einkaufswagen, Ratte, Zigarette, Kasse, Butter, Reis, Batterien, Kekse, Kerzen, Glühbirne) müssen beim Weitergeben mit Titel, Autor und Lizenz genannt werden (https://creativecommons.org/licenses/by/3.0/).

Der Einkaufswagen steht unter CC-BY 3.0 und muss beim Weitergeben des Spiels genannt werden: "Shopping cart" by Poly by Google, CC-BY 3.0 (https://creativecommons.org/licenses/by/3.0/).

### Kasse (Poly Haven, CC0, https://polyhaven.com/a/<id>)
- Zigaretten-Rollo: `rollershutter_window_01`, Zigaretten: `cigarette_pack`
- Ueberwachungskamera: `security_camera_01`
- Kartenterminal: eigenes Modell (`tools/models/kartenterminal.blend`), nicht von Poly Haven
- Kasse und Stuhl: siehe Tabelle oben (POS Machine, Desk Chair). Der Theken-Scanner ist Einrichtung aus Boxen.

## Schrift
8x8-Bitmapfont aus dem Crate `font8x8` (Public Domain, basierend auf Daniel Hepper's font8x8).

Schilder und Werbung im Markt: **Archivo Black** von Omnibus-Type, SIL Open Font License 1.1 (`assets/fonts/OFL.txt`). Wird beim Start mit `ab_glyph` in einen Schild-Atlas gerendert.

## Werkzeuge
Konvertierung: `tools/prepare_assets.py` (Blender, headless).

## Startbild der Engine
„Gemacht mit Mettgine“: Schriftzug in Archivo Black (OFL), Mettbrötchen eigene Zeichnung, Dino aus dem Logo des Mett Launchers (Herkunft und Lizenz des Fotos noch nicht belegt).
