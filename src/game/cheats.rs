//! Cheats zum Testen (Menue mit F1).

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
];

impl Game {
    pub fn cheat(&mut self, i: usize) {
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
            12 | 13 => {
                let cash = i == 13;
                self.debug_jump_to_pay(cash);
                if cash {
                    "Barzahlung.".into()
                } else {
                    "Kartenzahlung.".into()
                }
            }
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
        // Unverwundbar: Kunde direkt neben dem Spieler erwischt ihn nicht
        g.spawn_customer_at(g.player.pos + vec3(0.3, 0.0, 0.0));
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(g.outcome, Outcome::Running);
    }

    #[test]
    fn every_scare_cheat_scares() {
        for i in FIRST_SCARE_CHEAT..CHEATS.len() {
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
                scared |= g.sfx.iter().any(|(s, _)| matches!(s, Sfx::Scare));
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
    fn cheat_labels_fit_two_columns() {
        for c in CHEATS {
            assert!(c.chars().count() <= 26, "{c} zu lang");
        }
    }
}
