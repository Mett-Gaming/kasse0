//! Cheats zum Testen (Menue mit F1).

use super::pfand::items::Item;
use super::*;

/// Kurze Namen: das Menue hat zwei Spalten (je hoechstens 26 Zeichen).
/// Ab `FIRST_SCARE_CHEAT` folgen die Jumpscares (scare_cheats.rs).
pub const CHEATS: &[&str] = &[
    "Unverwundbar an/aus",
    "Markt kippen",
    "Zettel füllen (billig)",
    "+10 € (nächste Stufe)",
    "Auffälligkeit 0 %",
    "Auffälligkeit 100 %",
    "Zur Kasse (mit Wagen)",
    "Zur Getränkeabteilung",
    "Mitarbeiter herrufen",
    "Alle Leute entfernen",
    "Durchsage auslösen",
    "Markt umbauen",
    "Kasse: Kartenzahlung",
    "Kasse: Barzahlung",
    "Leben auffüllen (alle)",
    "Tutorial überspringen",
    "Flucht: Korridor",
    "Wagen wird mitgenommen",
    "Schreck: Hinter dir",
    "Schreck: Stromausfall",
    "Schreck: Kassiererin",
    "Schreck: Geisterwagen",
    "Schreck: Fata Morgana",
    "Schreck: Deckenkabel",
    "Schreck: Kopfüber",
    "Schreck: Monitor",
    "Schreck: Tod (Gesicht)",
    "Schreck: Troll-Option",
    "Schreck: Kühltruhe",
    "Schreck: An der Decke",
    "Schreck: Alle schauen",
    "Schreck: Schritte (dunkel)",
    "Schreck: Regal-Gesicht",
    "Schreck: Im Wagen",
    "Schreck: Durchsage",
    "Schreck: Regal kracht",
    "Stufe +1 (sofort)",
    "Director: Höhepunkt",
    "Director: Ruhe",
    "Verfolger 6 m hinter dir",
    "NPC-Zustände an/aus",
    "Tutorial starten",
    "Flucht: langer Korridor",
    "Sofort sterben",
    "Tomatensaft trinken",
    "Seitentrakt: hinein",
    "Gestalt in der Ferne",
    "Déjà-vu-Szene",
    "Gang sperren lassen",
    "Zum Pfandautomaten",
    "Pfand: Tüte füllen",
    "Pfandbon 2,50 €",
    "Zum Kiosk",
    "Unendlich Ausdauer",
    "Pfandbon 0,50 €",
    "Pfandbon 1,00 €",
    "Pfandbon 5,00 €",
    "Pfandbon 10,00 €",
    "Item: Sticker Kunde 0",
    "Item: Energy-Drink",
    "Item: Wunderkerze",
    "Item: Beruhigungstee",
    "Item: Öl",
    "Item: Mettbrötchen",
    "Item: Filzsohlen",
    "Item: Glöckchen",
    "Item: Taschenlampe",
    "Item: Mobility Scooter",
];

/// Stelle eines Cheats in `CHEATS` (zur Uebersetzungszeit, fehlt er: Fehler beim Bauen)
const fn cheat_index(name: &str) -> usize {
    let mut i = 0;
    while i < CHEATS.len() {
        let (a, b) = (CHEATS[i].as_bytes(), name.as_bytes());
        if a.len() == b.len() {
            let mut k = 0;
            while k < a.len() && a[k] == b[k] {
                k += 1;
            }
            if k == a.len() {
                return i;
            }
        }
        i += 1;
    }
    panic!("Cheat fehlt")
}

/// Ausdauer geht nie aus (an/aus)
pub const STAMINA_CHEAT: usize = cheat_index("Unendlich Ausdauer");
/// Pfandbons in verschiedenen Werten (Cent), in dieser Reihenfolge ab `BON_CHEATS`
pub const BON_CHEATS: usize = cheat_index("Pfandbon 0,50 €");
pub const BON_VALUES: [i64; 4] = [50, 100, 500, 1000];
/// Je ein Item vom Kiosk, in der Reihenfolge von `Item::ALL`
pub const ITEM_CHEATS: usize = cheat_index("Item: Sticker Kunde 0");

/// Neuen Einkauf im Tutorial-Modus starten (main.rs holt die Anfrage ab)
pub const TUTORIAL_CHEAT: usize = cheat_index("Tutorial starten");
/// Direkt in den laengsten Fluchtkorridor (140 m)
pub const LONG_CORRIDOR_CHEAT: usize = cheat_index("Flucht: langer Korridor");
/// Run sofort verloren (Schreck, Endbildschirm) - zum Testen der Todes-Schriftzuege
pub const DIE_CHEAT: usize = cheat_index("Sofort sterben");
/// Abgelaufenen Tomatensaft trinken: 30 s Rausch, dann kotzen (trip.rs)
pub const TRIP_CHEAT: usize = cheat_index("Tomatensaft trinken");
/// In den ersten Raum des Seitentrakts (kippt den Markt vorher, world/gen/wing.rs)
pub const WING_CHEAT: usize = cheat_index("Seitentrakt: hinein");
/// Gestalt in der Ferne aufstellen (watcher.rs)
pub const WATCHER_CHEAT: usize = cheat_index("Gestalt in der Ferne");
/// Deja-vu-Szene vor einen stellen (dejavu.rs)
pub const DEJAVU_CHEAT: usize = cheat_index("Déjà-vu-Szene");
/// Ein Mettarbeiter sperrt sofort einen Gang (closures.rs)
pub const CLOSURE_CHEAT: usize = cheat_index("Gang sperren lassen");
/// Vor einen laufenden Pfandautomaten, Tuete voll, Bon
pub const PFAND_CHEATS: usize = cheat_index("Zum Pfandautomaten");

/// Index des ersten NPC-Cheats in `CHEATS` (nach den Schrecken)
pub const FIRST_NPC_CHEAT: usize = FIRST_SCARE_CHEAT + 18;

/// Kategorien des Cheat-Menues: Name, kurze Beschreibung, Cheats (Index in `CHEATS`).
/// Jeder Cheat steht in genau einer Kategorie (siehe Test).
pub fn cheat_groups() -> Vec<(&'static str, &'static str, Vec<usize>)> {
    vec![
        (
            "Spieler",
            "Unverwundbar, Ausdauer, Leben, Tutorial, Rausch",
            vec![
                0,
                STAMINA_CHEAT,
                14,
                TUTORIAL_CHEAT,
                15,
                4,
                5,
                DIE_CHEAT,
                TRIP_CHEAT,
            ],
        ),
        (
            "Markt & Einkauf",
            "Kippen, Zettel, Umbau, Seitentrakt, Gestalt, Déjà-vu",
            vec![1, 2, 3, 7, 11, 10, WING_CHEAT, WATCHER_CHEAT, DEJAVU_CHEAT],
        ),
        (
            "Kasse & Flucht",
            "Kasse, Bezahlen, Korridor",
            vec![6, 12, 13, 16, LONG_CORRIDOR_CHEAT],
        ),
        (
            "Pfand",
            "Automat, Tüte, Kiosk, Pfandbons in jedem Wert",
            // Bons nach Wert geordnet: 0,50 / 1,00 / 2,50 / 5,00 / 10,00
            vec![
                PFAND_CHEATS,
                PFAND_CHEATS + 1,
                PFAND_CHEATS + 3,
                BON_CHEATS,
                BON_CHEATS + 1,
                PFAND_CHEATS + 2,
                BON_CHEATS + 2,
                BON_CHEATS + 3,
            ],
        ),
        (
            "Items",
            "Jedes Item vom Kiosk sofort bekommen",
            (ITEM_CHEATS..ITEM_CHEATS + Item::ALL.len()).collect(),
        ),
        (
            "Leute",
            "Mitarbeiter rufen, alle entfernen, Wagen, Gang sperren",
            vec![8, 9, 17, CLOSURE_CHEAT],
        ),
        (
            "Schrecken",
            "Jeden Jumpscare einzeln auslösen",
            (FIRST_SCARE_CHEAT..FIRST_NPC_CHEAT).collect(),
        ),
        (
            "NPCs & Stufen",
            "Stufe, Spannungskurve, Verfolger, Zustände anzeigen",
            (FIRST_NPC_CHEAT..TUTORIAL_CHEAT).collect(),
        ),
    ]
}

impl Game {
    /// Cheat: sofort tot - wie erwischt (rotes Bild, Schreck, Gesicht), dann der Endbildschirm
    /// mit einem der gruseligen Schriftzuege. Gilt auch im Tutorial und mit Unverwundbar.
    pub fn debug_die(&mut self) {
        if self.outcome != Outcome::Running {
            return;
        }
        self.dying = None;
        self.dialog = None;
        self.close_pointer_window();
        self.outcome = Outcome::Caught;
        self.outcome_timer = 0.0;
        self.red = 1.0;
        self.glitch = 1.0;
        self.fire_scare(1.0);
        self.play(Sfx::Shriek, 1.2);
        self.play(Sfx::Sting, 1.0);
        let pool = crate::people::people_of(false);
        self.catcher_variant = Some(pool[self.seed as usize % pool.len()]);
        let f = self.view_dir();
        self.catcher = Some(self.player.pos + vec3(f.x, 0.0, f.z).normalize_or_zero() * 0.6);
    }

    /// Cheat: Pfandbon ueber `cents` in die Tasche (kippt den Markt, sonst gibt es keinen Kiosk).
    fn bon_cheat(&mut self, cents: i64) -> String {
        self.tip();
        self.pfand.bons.push(cents);
        self.pfand.printed = true;
        format!("Bon über {}.", crate::ui::euro(cents))
    }

    /// Cheat: ein Item vom Kiosk geschenkt, wie gekauft (volle Taschen bleiben voll).
    fn item_cheat(&mut self, item: Item) -> String {
        if let Some(why) = self.item_refused(item) {
            return why.into();
        }
        self.receive_item(item);
        format!("{} bekommen.", item.name())
    }

    /// Cheat ausloesen. Danach zaehlt dieser Einkauf nicht mehr fuer Erfolge.
    pub fn cheat(&mut self, i: usize) {
        self.cheated = true;
        let msg: String = match i {
            0 => {
                self.god = !self.god;
                format!("Unverwundbar: {}", if self.god { "AN" } else { "AUS" })
            }
            1 => {
                self.tip();
                "Markt gekippt.".into()
            }
            2 => {
                self.tip();
                self.fill_list_cheap();
                "Zettel aufgefüllt.".into()
            }
            3 => {
                self.tip();
                // Ware ohne Zettelbezug: zaehlt nur zum Warenwert
                let cat = catalog();
                let p = cat
                    .products
                    .iter()
                    .position(|p| p.list.is_none() && p.cents >= 500)
                    .unwrap_or(0);
                self.cart.push(ItemData {
                    name: "Cheat-Ware".into(),
                    cents: 1000,
                    kind: PickupKind::Product {
                        product: p,
                        fake: false,
                    },
                    mesh: cat.products[p].mesh,
                    tint: vec3(1.0, 0.3, 1.0),
                });
                self.check_tier();
                format!(
                    "Warenwert {} - Stufe: {}",
                    crate::ui::euro(self.cart_value()),
                    self.tier.name()
                )
            }
            4 => {
                self.suspicion = 0.0;
                "Auffälligkeit 0 %".into()
            }
            5 => {
                self.suspicion = 1.0;
                "Auffälligkeit 100 %".into()
            }
            6 => {
                let lane = self.open_lane().unwrap();
                self.player.pos = lane.scan_point;
                self.player.yaw = 0.0;
                self.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
                self.trolley.pushing = false;
                "An der Kasse.".into()
            }
            7 => {
                let z = self.world.drinks_zone;
                self.player.pos = vec3(z.min.x + 1.7, 0.0, z.max.z - 0.8);
                self.trolley.pos = self.player.pos + vec3(0.0, 0.0, 1.5);
                self.trolley.pushing = false;
                "In der Getränkeabteilung.".into()
            }
            8 => {
                let (s, c) = self.player.yaw.sin_cos();
                let pos = self
                    .world
                    .collide(self.player.pos + vec3(s, 0.0, -c) * 1.4, 0.25);
                let mut e = npc::Employee::new(pos);
                e.variant = self.pick_person(true);
                e.ignore = 0.0;
                e.retarget = 5.0;
                self.employees.push(e);
                "Ein Mitarbeiter kommt.".into()
            }
            9 => {
                self.customers.clear();
                self.employees.clear();
                self.dialog = None;
                "Alle weg.".into()
            }
            10 => {
                self.next_announce = 0.0;
                if !self.tipped {
                    let text = ANNOUNCEMENTS[0];
                    self.announcement = Some((text.into(), 6.0));
                    self.play(Sfx::Chime, 0.7);
                }
                "Durchsage.".into()
            }
            11 => {
                self.params.shuffle = self.params.shuffle.wrapping_add(1);
                self.params.open_lane = self.rng.gen_range(0..4);
                self.rebuild_world();
                self.glitch = 0.7;
                "Markt umgebaut.".into()
            }
            14 => {
                // Markt: keine Tode mehr, rote Vignette weg; Kasse: drei Leben, kein Misstrauen
                self.deaths = 0;
                self.red = 0.0;
                if let Some(t) = &mut self.till {
                    t.lives = till::TILL_LIVES;
                    t.doubt = 0.0;
                }
                "Leben aufgefüllt.".into()
            }
            15 => {
                // beendet Anleitung, Wegweiser und Entschaerfung
                self.tutorial = None;
                self.tutorial_run = false;
                "Tutorial übersprungen.".into()
            }
            16 => {
                self.debug_jump_to_corridor(None);
                "Lauf! Nicht umdrehen.".into()
            }
            LONG_CORRIDOR_CHEAT => {
                self.debug_jump_to_corridor(Some(crate::world::CORRIDOR_LEN_MAX));
                "Langer Korridor. Lauf!".into()
            }
            WING_CHEAT => self.debug_to_wing(),
            CLOSURE_CHEAT => self.debug_closure(),
            STAMINA_CHEAT => {
                self.endless_stamina = !self.endless_stamina;
                format!(
                    "Unendlich Ausdauer: {}",
                    if self.endless_stamina { "AN" } else { "AUS" }
                )
            }
            i if i >= ITEM_CHEATS => self.item_cheat(Item::ALL[i - ITEM_CHEATS]),
            i if i >= BON_CHEATS => self.bon_cheat(BON_VALUES[i - BON_CHEATS]),
            i if i >= PFAND_CHEATS => self.pfand_cheat(i - PFAND_CHEATS),
            WATCHER_CHEAT => {
                self.tip();
                self.watcher = None;
                if self.spawn_watcher() {
                    "Da hinten steht jemand.".into()
                } else {
                    "Keine freie Sicht. In einen langen Gang schauen.".into()
                }
            }
            DEJAVU_CHEAT => {
                self.tip();
                if self.tier < Tier::Uneasy {
                    self.force_tier(Tier::Uneasy);
                }
                self.dejavu = None;
                if self.place_dejavu(None) {
                    "Dreh dich um. Und dann in einen anderen Gang.".into()
                } else {
                    "Kein Platz. Im Gang stehen und zur Seite schauen.".into()
                }
            }
            TRIP_CHEAT => {
                self.drink_tomato();
                "Prost.".into()
            }
            DIE_CHEAT => {
                self.debug_die();
                "Du bist tot.".into()
            }
            12 | 13 => {
                let cash = i == 13;
                self.debug_jump_to_pay(cash);
                if cash {
                    "Barzahlung.".into()
                } else {
                    "Kartenzahlung.".into()
                }
            }
            TUTORIAL_CHEAT => {
                self.tutorial_request = true;
                "Tutorial startet.".into()
            }
            i if i >= FIRST_NPC_CHEAT => self.npc_cheat(i - FIRST_NPC_CHEAT),
            i if i >= FIRST_SCARE_CHEAT => self.scare_cheat(i - FIRST_SCARE_CHEAT),
            _ => {
                // Wagen 2 m vor dir abstellen, ein Mitarbeiter schnappt ihn sich
                self.tip();
                let (s, c) = self.player.yaw.sin_cos();
                let fwd = vec3(s, 0.0, -c);
                self.trolley.pushing = false;
                self.trolley.pos = self.world.collide(self.player.pos + fwd * 2.0, 0.42);
                let pos = self.world.collide(self.trolley.pos + fwd * 0.9, 0.25);
                let mut e = npc::Employee::new(pos);
                e.variant = self.pick_person(true);
                e.ignore = 30.0;
                self.employees.push(e);
                let i = self.employees.len() - 1;
                self.start_carrying(i);
                "Ein Mitarbeiter nimmt deinen Wagen mit.".into()
            }
        };
        self.message = Some((format!("CHEAT: {msg}"), 3.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cheat_is_in_exactly_one_category() {
        let mut seen = vec![0; CHEATS.len()];
        for (name, _, list) in cheat_groups() {
            assert!(!list.is_empty(), "leere Kategorie {name}");
            for i in list {
                seen[i] += 1;
            }
        }
        for (i, n) in seen.iter().enumerate() {
            assert_eq!(*n, 1, "Cheat '{}' steht {n}-mal im Menü", CHEATS[i]);
        }
    }

    #[test]
    fn every_cheat_runs_and_does_its_job() {
        let mut g = Game::new(55);
        for i in 0..FIRST_SCARE_CHEAT {
            g.cheat(i);
            let mut inp = Input::default();
            for _ in 0..30 {
                g.update(1.0 / 60.0, &mut inp);
            }
        }
        assert!(g.tipped);
        assert!(g.god);
        assert!(g.list_complete(), "Zettel nicht aufgefuellt");
        assert!(g.tier >= Tier::Uneasy, "Warenwert-Cheat ohne Wirkung");
        assert!(g.customers.is_empty());
        assert!(g.tutorial.is_none());
    }

    #[test]
    fn refill_lives_cheat_restores_market_and_till_lives() {
        let mut g = Game::new(56);
        g.debug_jump_to_pay(false);
        g.deaths = 2;
        let t = g.till.as_mut().unwrap();
        t.lives = 1;
        t.doubt = 0.7;
        let i = CHEATS.iter().position(|c| c.starts_with("Leben")).unwrap();
        g.cheat(i);
        assert_eq!(g.deaths, 0);
        assert_eq!(g.death_vignette(), 0.0);
        let t = g.till.as_ref().unwrap();
        assert_eq!(t.lives, till::TILL_LIVES);
        assert_eq!(t.doubt, 0.0);

        let mut g = Game::new_tutorial(57);
        let i = CHEATS
            .iter()
            .position(|c| c.starts_with("Tutorial"))
            .unwrap();
        g.cheat(i);
        assert!(g.tutorial.is_none());
        assert!(
            !g.tutorial_run,
            "Entschaerfung bleibt nach dem Ueberspringen"
        );
        // "Tutorial starten": fordert einen neuen Tutorial-Einkauf an (einmal abholbar)
        assert_eq!(CHEATS[TUTORIAL_CHEAT], "Tutorial starten");
        assert!(cheat_groups()[0].2.contains(&TUTORIAL_CHEAT));
        assert!(!g.take_tutorial_request());
        g.cheat(TUTORIAL_CHEAT);
        assert!(g.take_tutorial_request());
        assert!(!g.take_tutorial_request());
        // "Sofort sterben": auch unverwundbar und im Tutorial ist der Run vorbei
        assert_eq!(CHEATS[DIE_CHEAT], "Sofort sterben");
        assert_eq!(CHEATS[LONG_CORRIDOR_CHEAT], "Flucht: langer Korridor");
        assert!(cheat_groups()[0].2.contains(&DIE_CHEAT));
        let mut d = Game::new_tutorial(58);
        d.god = true;
        d.cheat(DIE_CHEAT);
        assert_eq!(d.outcome, Outcome::Caught);
        for _ in 0..200 {
            d.update(1.0 / 60.0, &mut Input::default());
        }
        assert_eq!(d.outcome, Outcome::Caught, "wieder lebendig");
        // Unverwundbar: Kunde direkt neben dem Spieler erwischt ihn nicht
        g.spawn_customer_at(g.player.pos + vec3(0.3, 0.0, 0.0));
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(g.outcome, Outcome::Running);
    }

    #[test]
    fn every_scare_cheat_scares() {
        for i in FIRST_SCARE_CHEAT..FIRST_NPC_CHEAT {
            let mut g = Game::new(56);
            g.cheat(i);
            let mut scared = false;
            let mut troll = false;
            for k in 0..60 * 12 {
                let mut inp = Input::default();
                // Fata Morgana: vorgehen; Hinter dir: nach 1 s umdrehen
                if CHEATS[i].contains("Fata") {
                    inp.forward = true;
                }
                if CHEATS[i].contains("Hinter") && k == 60 {
                    g.player.yaw += std::f32::consts::PI;
                }
                g.update(1.0 / 60.0, &mut inp);
                scared |= !g.scares.is_empty();
                troll |= g.take_troll_request();
                g.sfx.clear();
                if scared || troll {
                    break;
                }
            }
            assert!(
                scared || troll,
                "Cheat {:?} loest keinen Schreck aus",
                CHEATS[i]
            );
            assert_eq!(g.deaths, 0, "Cheat {:?} kostet ein Leben", CHEATS[i]);
        }
    }

    #[test]
    fn every_scare_has_its_own_look_and_sound() {
        use super::super::scare_kinds::ScareKind::*;
        let kinds = [
            Behind,
            Blackout,
            Cashier,
            CartGhost,
            Mirage,
            Cables,
            Hanging,
            Monitor,
            Freezer,
            Crawler,
            AllLook,
            DarkSteps,
            ShelfFace,
            CartRider,
            Announcement,
            ShelfShake,
        ];
        let sig = |k: super::super::scare_kinds::ScareKind| {
            let st = k.style();
            format!(
                "{:?}{:?}{}{:.1}{:.1}{:.1}{:.1}",
                st.sounds[0].0, st.sounds[1].0, st.invert, st.flash, st.zoom, st.red, st.shake
            )
        };
        let all: std::collections::HashSet<String> = kinds.iter().map(|&k| sig(k)).collect();
        assert_eq!(
            all.len(),
            kinds.len(),
            "zwei Schrecken fuehlen sich gleich an"
        );
        // nicht alle mit Negativ-Flackern und Blitz wie frueher
        let inverted = kinds.iter().filter(|k| k.style().invert).count();
        assert!(inverted <= 4, "{inverted} Schrecken flackern negativ");
    }

    #[test]
    fn the_same_scare_does_not_come_twice_in_a_row() {
        use super::super::scare_kinds::ScareKind;
        let mut g = Game::new(59);
        g.tip();
        g.tier = Tier::Wrong;
        g.scare_cd = 0.0;
        g.fire_scare_kind(ScareKind::Crawler, 0.7);
        g.scare_cd = 0.0;
        assert!(
            !g.can_scare_kind(true, ScareKind::Crawler),
            "gleiche Art sofort wieder"
        );
        assert!(g.can_scare_kind(true, ScareKind::DarkSteps));
        for k in [
            ScareKind::DarkSteps,
            ScareKind::Announcement,
            ScareKind::AllLook,
        ] {
            g.fire_scare_kind(k, 0.7);
        }
        g.scare_cd = 0.0;
        g.director = Default::default();
        assert!(
            g.can_scare_kind(true, ScareKind::Crawler),
            "nach drei anderen wieder erlaubt"
        );
    }

    #[test]
    fn npc_cheats_work() {
        let mut g = Game::new(58);
        g.cheat(FIRST_NPC_CHEAT);
        assert!(g.tipped && g.tier >= Tier::Uneasy, "Stufe steigt nicht");
        g.cheat(FIRST_NPC_CHEAT + 1);
        assert_eq!(g.director.phase, super::super::director::Phase::Peak);
        g.cheat(FIRST_NPC_CHEAT + 2);
        assert_eq!(g.director.phase, super::super::director::Phase::Relax);
        g.cheat(FIRST_NPC_CHEAT + 3);
        let behind = g
            .customers
            .iter()
            .any(|c| c.stalker && c.brain.fresh() && (c.pos - g.player.pos).length() < 7.5);
        assert!(behind, "kein Verfolger hinter dir");
        g.cheat(FIRST_NPC_CHEAT + 4);
        assert!(g.npc_debug && !g.npc_debug_labels().is_empty());
    }

    #[test]
    fn stamina_pfand_and_item_cheats_work() {
        // unendlich Ausdauer: 30 s rennen, nie erschoepft; wieder aus: Ausdauer sinkt
        let mut g = Game::new(61);
        g.cheat(STAMINA_CHEAT);
        assert!(g.endless_stamina);
        let run = || Input {
            forward: true,
            run: true,
            ..Default::default()
        };
        for _ in 0..60 * 30 {
            g.update(1.0 / 60.0, &mut run());
        }
        assert!(!g.player.exhausted && g.player.stamina > 0.99);
        g.cheat(STAMINA_CHEAT);
        assert!(!g.endless_stamina);
        for _ in 0..60 {
            g.update(1.0 / 60.0, &mut run());
        }
        assert!(
            g.player.stamina < 0.95,
            "Ausdauer sinkt nach dem Ausschalten nicht"
        );
        // Pfandbons in jedem Wert
        let mut g = Game::new(62);
        for (k, &v) in BON_VALUES.iter().enumerate() {
            g.cheat(BON_CHEATS + k);
            assert_eq!(*g.pfand.bons.last().unwrap(), v);
        }
        assert!(g.tipped && g.pfand.printed);
        assert_eq!(CHEATS[BON_CHEATS], "Pfandbon 0,50 €");
        // jedes Item, mit seinem Namen; Verbrauchbares bis die Taschen voll sind
        for (k, item) in Item::ALL.iter().enumerate() {
            assert_eq!(CHEATS[ITEM_CHEATS + k], format!("Item: {}", item.name()));
            let mut g = Game::new(63);
            g.cheat(ITEM_CHEATS + k);
            let o = &g.pfand.owned;
            assert!(o.pocket.contains(item) || o.has(*item), "{item:?} fehlt");
        }
        let mut g = Game::new(64);
        let mett = ITEM_CHEATS + Item::ALL.iter().position(|i| *i == Item::Mett).unwrap();
        for _ in 0..10 {
            g.cheat(mett);
        }
        assert_eq!(
            g.pfand.owned.pocket.len(),
            super::super::pfand::items::POCKET_MAX
        );
        // eigene Kategorien "Pfand" und "Items"
        let groups = cheat_groups();
        let names: Vec<_> = groups.iter().map(|c| c.0).collect();
        assert!(names.contains(&"Pfand") && names.contains(&"Items"));
        let items = &groups.iter().find(|c| c.0 == "Items").unwrap().2;
        assert_eq!(items.len(), Item::ALL.len());
    }

    #[test]
    fn cheat_labels_fit_two_columns() {
        for c in CHEATS {
            assert!(c.chars().count() <= 26, "{c} zu lang");
        }
    }
}
