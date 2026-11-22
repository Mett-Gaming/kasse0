//! Automatischer Screenshot-Durchlauf (`KASSE0_AUTOSHOT=<ordner>`): stellt Szenen nach und fotografiert sie.

use super::*;

impl App {
    /// Ein Schritt des Screenshot-Durchlaufs. Liefert Wartezeit bis zum naechsten Schritt.
    pub(super) fn autoshot_step(&mut self, step: usize, dir: &std::path::Path) -> Option<f32> {
        use glam::vec3;
        let shot = |name: &str| dir.join(format!("{name}.bmp"));
        let seed = self.fixed_seed.unwrap_or(1994);
        match step {
            0 => return Some(1.5),
            1 => self.shot_request = Some(shot("01_title")),
            2 => {
                self.start_game(seed);
                return Some(2.5);
            }
            3 => self.shot_request = Some(shot("02_normal")),
            4 => {
                self.game.tip();
                return Some(2.0);
            }
            5 => self.shot_request = Some(shot("03_tipped")),
            6 => {
                // In einen Gang stellen, ein Kunde steht vorne am Regal
                let w = &self.game.world;
                let x = 2.5 + 3.4;
                let z = w.aisle_z1 - 1.0;
                self.game.player.pos = vec3(x, 0.0, z);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = -0.05;
                self.game.employees.clear();
                if let Some(c) = self.game.customers.first_mut() {
                    c.pos = vec3(x - 0.55, 0.0, z - 6.0);
                    c.home = c.pos;
                    c.home_facing = vec3(-1.0, 0.0, 0.0);
                }
                self.game.customers.truncate(1);
                return Some(1.0);
            }
            7 => self.shot_request = Some(shot("04_aisle_customer")),
            8 => {
                self.show_list = true;
                return Some(0.5);
            }
            9 => self.shot_request = Some(shot("05_list")),
            10 => {
                self.show_list = false;
                // Mitarbeiter spricht an
                self.game.customers.clear();
                let p = self.game.player.pos;
                let mut e = game::npc::Employee::new(p + vec3(0.0, 0.0, -1.4));
                e.ignore = 0.0;
                e.retarget = 5.0;
                self.game.employees.push(e);
                return Some(1.2);
            }
            11 => self.shot_request = Some(shot("06_dialog")),
            12 => {
                self.game.dialog = None;
                self.game.employees.clear();
                self.go(Screen::Paused);
                return Some(0.5);
            }
            13 => self.shot_request = Some(shot("07_pause")),
            14 => {
                self.go(Screen::Options { from_pause: true });
                return Some(0.5);
            }
            15 => self.shot_request = Some(shot("08_options")),
            16 => {
                // Erwischt werden
                self.go(Screen::Playing);
                self.game.tip();
                self.game.deaths = 2; // letztes Leben: direkt Game Over
                let p = self.game.player.pos;
                self.game.spawn_customer_at(p + vec3(0.6, 0.0, 1.2));
                self.game.player.yaw = 0.0;
                return Some(0.4);
            }
            17 => self.shot_request = Some(shot("09_caught")),
            18 => return Some(3.0),
            19 => self.shot_request = Some(shot("10_caught_end")),
            20 => {
                // Gewinnen
                self.start_game(seed);
                self.game.tip();
                self.game.debug_finish_shopping();
                return Some(3.0);
            }
            21 => self.shot_request = Some(shot("11_won")),
            22 => {
                self.to_title();
                self.go(Screen::Options { from_pause: false });
                return Some(0.6);
            }
            23 => self.shot_request = Some(shot("12_options_title")),
            24 => {
                // Vor eine Ware stellen und sie anschauen (Interaktions-Prompt)
                self.start_game(seed);
                self.game.tip();
                self.game.customers.clear();
                self.game.employees.clear();
                let cat = catalog::catalog();
                let target = self
                    .game
                    .world
                    .items
                    .iter()
                    .find(|it| {
                        !it.fake
                            && cat.products[it.product as usize].pricey
                            && cat.products[it.product as usize].list.is_some()
                    })
                    .map(|it| it.pos + vec3(0.0, it.half().y, 0.0));
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                if let Some(t) = target {
                    self.game.glitch = 0.0;
                    for (dx, dz) in [(1.1f32, 0.0f32), (-1.1, 0.0), (0.0, 1.1), (0.0, -1.1)] {
                        let stand = vec3(t.x + dx, 0.0, t.z + dz);
                        if self.game.world.collide(stand, 0.3).distance(stand) < 1e-3 {
                            self.game.player.pos = stand;
                            let to = t + vec3(0.0, 0.05, 0.0)
                                - (stand + vec3(0.0, game::EYE_HEIGHT, 0.0));
                            self.game.player.yaw = to.x.atan2(-to.z);
                            self.game.player.pitch = (to.y / to.length()).asin();
                            break;
                        }
                    }
                }
                return Some(1.0);
            }
            25 => self.shot_request = Some(shot("13_focus")),
            26 => {
                // Ware greifen (liegt dann in der Hand)
                self.input.interact = true;
                return Some(0.4);
            }
            27 => self.shot_request = Some(shot("15_held")),
            28 => {
                // in den geschobenen Wagen schauen und hineinlegen
                self.game.player.pitch = -0.75;
                self.input.interact = true;
                return Some(0.5);
            }
            29 => self.shot_request = Some(shot("16_cart")),
            32 => {
                // Getraenke-Aktionstisch im normalen Markt
                self.start_game(seed);
                let w = &self.game.world;
                self.game.player.pos = vec3(w.size.x - 2.0, 0.0, 6.2);
                self.game.player.yaw = std::f32::consts::FRAC_PI_2;
                self.game.player.pitch = -0.45;
                self.game.trolley.pushing = false;
                self.game.glitch = 0.0;
                return Some(1.2);
            }
            33 => self.shot_request = Some(shot("17_drinks_display")),
            34 => {
                self.go(Screen::Cheats);
                return Some(0.5);
            }
            35 => self.shot_request = Some(shot("18_cheats")),
            36 => {
                // frisches Werbeposter an der Westwand
                self.start_game(seed);
                self.game.tutorial = None;
                let z = self.game.world.aisle_z0 + 1.5;
                self.game.player.pos = vec3(1.7, 0.0, z);
                self.game.player.yaw = -std::f32::consts::FRAC_PI_2;
                self.game.player.pitch = 0.08;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = vec3(1.7, 0.0, z + 3.0);
                self.game.glitch = 0.0;
                self.game.message = None;
                return Some(1.0);
            }
            37 => self.shot_request = Some(shot("19_poster_fresh")),
            38 => {
                // ... und im Kollaps
                for _ in 0..6 {
                    self.game.cheat(3);
                }
                self.game.cheat(9);
                let z = self.game.world.aisle_z0 + 1.5;
                self.game.player.pos = vec3(1.7, 0.0, z);
                self.game.player.yaw = -std::f32::consts::FRAC_PI_2;
                self.game.player.pitch = 0.08;
                self.game.glitch = 0.0;
                self.game.message = None;
                self.game.tipped_time_add(400.0);
                return Some(1.0);
            }
            39 => self.shot_request = Some(shot("20_poster_rotten")),
            40 => {
                // krummer Gang mit Durchgaengen: in einen Gang schauen
                self.game.cheat(9);
                // fuer das Bild ohne Querwaende umbauen
                self.game.params.walls = 0;
                self.game.cheat(11);
                let w = &self.game.world;
                let z = w.aisle_z1 + 1.5;
                self.game.player.pos = vec3(2.5 + 3.4 * 2.0, 0.0, z);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = -0.05;
                self.game.trolley.pos = self.game.player.pos + vec3(0.0, 0.0, 2.0);
                self.game.glitch = 0.0;
                self.game.message = None;
                return Some(0.8);
            }
            41 => self.shot_request = Some(shot("21_crooked")),
            42 => {
                // Fata Morgana erzwingen und hinschauen
                self.game.holograms.clear();
                self.game.force_mirage();
                if let Some(h) = self.game.holograms.first_mut() {
                    // fuer das Bild: eine Kundin in den freien Gang vor uns
                    h.pos = self.game.player.pos + vec3(0.0, 0.0, -8.0);
                    h.person = Some(0);
                    h.scale = glam::Vec3::ONE;
                    h.life = 30.0;
                }
                if let Some(h) = self.game.holograms.first() {
                    let to = h.pos + vec3(0.0, 1.0, 0.0) - self.game.eye();
                    self.game.player.yaw = to.x.atan2(-to.z);
                    self.game.player.pitch = (to.y / to.length()).asin();
                }
                self.game.glitch = 0.0;
                return Some(0.5);
            }
            43 => self.shot_request = Some(shot("22_hologram")),
            44 => {
                // Mitarbeiter nimmt den Wagen mit, wir sehen zu
                self.game.holograms.clear();
                self.game.params.walls = 0;
                self.game.cheat(11);
                self.game.cheat(game::CHEATS.len() - 1);
                let e = self
                    .game
                    .employees
                    .last()
                    .map(|e| e.pos)
                    .unwrap_or_default();
                let to = e + vec3(0.0, 0.9, 0.0) - self.game.eye();
                self.game.player.yaw = to.x.atan2(-to.z);
                self.game.player.pitch = (to.y / to.length()).asin();
                self.game.glitch = 0.0;
                return Some(1.0);
            }
            45 => self.shot_request = Some(shot("23_cart_thief")),
            30 => {
                // Kunde in Sichtweite im dunklen Gang
                self.game
                    .spawn_customer_at(self.game.player.pos + vec3(0.0, 0.0, -4.0));
                self.game.player.yaw = 0.0;
                self.game.player.pitch = 0.0;
                if let Some(c) = self.game.customers.last_mut() {
                    c.angry = 0.0;
                    c.yaw = std::f32::consts::PI;
                }
                return Some(0.6);
            }
            31 => self.shot_request = Some(shot("14_customer_close")),
            46 | 50 => {
                // Kassenzone von den Gaengen aus (normal bzw. gekippt: "0 Kasse")
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.message = None;
                if step == 50 {
                    self.game.tip();
                    self.game.customers.clear();
                    self.game.employees.clear();
                    self.game.message = None;
                }
                let w = &self.game.world;
                let x = w.lanes.first().map_or(5.0, |l| l.scan_point.x + 1.6);
                self.game.player.pos = vec3(x, 0.0, w.aisle_z1 + 0.9);
                self.game.player.yaw = std::f32::consts::PI;
                self.game.player.pitch = 0.12;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(1.0);
            }
            47 => self.shot_request = Some(shot("24_checkout_signs")),
            51 => self.shot_request = Some(shot("26_checkout_zero")),
            48 => {
                // Kuehlwand mit Kopfschildern und Haengeschilder ueber den Gaengen
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.message = None;
                let z = self.game.world.aisle_z0 - 0.4;
                self.game.player.pos = vec3(9.0, 0.0, z + 2.6);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = 0.22;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(1.0);
            }
            49 => self.shot_request = Some(shot("25_cooler_signs")),
            52 | 54 => {
                // Mitten im Kassieren: Ware auf dem Band (normal bzw. gekippt)
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.tip();
                self.game.customers.clear();
                self.game.employees.clear();
                self.game.fill_list_cheap();
                let lane = self.game.open_lane().unwrap();
                self.game.trolley.pushing = false;
                self.game.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.2);
                self.game.player.pos = lane.scan_point + vec3(-0.25, 0.0, -1.4);
                let to = lane.belt_end + vec3(0.55, 0.25, 0.0)
                    - (self.game.player.pos + vec3(0.0, game::EYE_HEIGHT, 0.0));
                self.game.player.yaw = to.x.atan2(-to.z);
                self.game.player.pitch = (to.y / to.length()).asin();
                self.game.debug_unload_cart();
                if step == 54 {
                    self.game.held = None;
                }
                self.game.message = None;
                self.game.glitch = 0.0;
                return Some(if step == 52 { 2.5 } else { 6.0 });
            }
            53 => self.shot_request = Some(shot("27_belt")),
            55 => self.shot_request = Some(shot("28_belt_scanned")),
            56 | 58 => {
                // Blick zurueck zum Eingang mit Fensterfront
                self.start_game(seed);
                self.game.tutorial = None;
                if step == 58 {
                    self.game.tip();
                    self.game.customers.clear();
                    self.game.employees.clear();
                }
                self.game.message = None;
                let w = &self.game.world;
                self.game.player.pos = vec3(w.door_center.x - 1.0, 0.0, w.door_center.z - 6.0);
                self.game.player.yaw = std::f32::consts::PI * 0.82;
                self.game.player.pitch = 0.05;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(1.0);
            }
            57 => self.shot_request = Some(shot("29_windows")),
            59 => self.shot_request = Some(shot("30_windows_tipped")),
            60 | 62 | 64 => {
                // Aufstellung im Eingangsbereich: Kundschaft (zwei Bilder) bzw. Personal
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.message = None;
                self.game.shoppers.clear();
                self.game.customers.clear();
                self.game.employees.clear();
                let staff = step == 64;
                let pool = assets::people_of(staff);
                let skip = if step == 62 { 7 } else { 0 };
                let w = &self.game.world;
                let (cx, z) = (w.door_center.x, w.door_center.z - 3.6);
                let n = pool.len().saturating_sub(skip).min(8);
                for k in 0..n {
                    let pos = vec3(cx + (k as f32 - (n as f32 - 1.0) * 0.5) * 0.75, 0.0, z);
                    if staff {
                        let mut e = game::npc::Employee::new(pos);
                        e.variant = pool[k];
                        e.ignore = 60.0;
                        self.game.employees.push(e);
                    } else {
                        self.game.spawn_customer_at(pos);
                        if let Some(c) = self.game.customers.last_mut() {
                            c.variant = pool[k + skip];
                            c.home_facing = vec3(0.0, 0.0, 1.0);
                            c.angry = 0.0;
                        }
                    }
                }
                self.game.player.pos = vec3(cx, 0.0, z + 3.0);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = -0.05;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(0.4);
            }
            61 => self.shot_request = Some(shot("31_customers_a")),
            63 => self.shot_request = Some(shot("32_customers_b")),
            65 => {
                self.shot_request = Some(shot("33_staff"));
                return Some(0.3);
            }
            66 => {
                // Nahaufnahme: zwei Kunden und ein Mitarbeiter nebeneinander
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.message = None;
                self.game.shoppers.clear();
                self.game.customers.clear();
                self.game.employees.clear();
                let w = &self.game.world;
                let (cx, z) = (w.door_center.x, w.door_center.z - 3.6);
                for (k, v) in [1usize, 13, 24].into_iter().enumerate() {
                    self.game
                        .spawn_customer_at(vec3(cx + (k as f32 - 1.0) * 0.65, 0.0, z));
                    if let Some(c) = self.game.customers.last_mut() {
                        c.variant = v % people::PEOPLE.len();
                        c.home_facing = vec3(0.0, 0.0, 1.0);
                        c.angry = 0.0;
                    }
                }
                self.game.player.pos = vec3(cx, 0.0, z + 1.5);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = -0.15;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(0.4);
            }
            67 => self.shot_request = Some(shot("34_people_close")),
            68 => {
                // Bildfolge: ein Mitarbeiter laeuft quer durchs Bild (Animation pruefen)
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.message = None;
                self.game.shoppers.clear();
                self.game.customers.clear();
                self.game.employees.clear();
                let w = &self.game.world;
                let (cx, z) = (w.door_center.x, w.door_center.z - 3.0);
                let mut e = game::npc::Employee::new(vec3(cx - 2.5, 0.0, z));
                e.variant = people::people_of(true)[0];
                e.ignore = 60.0;
                self.game.employees.push(e);
                self.game.player.pos = vec3(cx, 0.0, z + 2.6);
                self.game.player.yaw = 0.0;
                self.game.player.pitch = -0.1;
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(0.5);
            }
            69..=74 => {
                self.shot_request = Some(shot(&format!("35_walk_{}", step - 69)));
                return Some(0.12);
            }
            76 | 78 | 80 => {
                // Kollaps: Loecher, verdorbene Ware, Riesenwaren, Steigung im Gang
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.tip();
                for _ in 0..6 {
                    self.game.cheat(3);
                }
                self.game.message = None;
                self.game.announcement = None;
                let w = &self.game.world;
                let x = world::AISLES_X0 + 3.4 * 2.0;
                let z = w.aisle_z0 + if step == 78 { 6.0 } else { 1.0 };
                self.game.player.pos = self.game.world.collide(vec3(x, 0.0, z), 0.3);
                self.game.player.yaw = match step {
                    76 => std::f32::consts::PI,
                    78 => std::f32::consts::FRAC_PI_2,
                    _ => std::f32::consts::PI * 0.9,
                };
                self.game.player.pitch = match step {
                    76 => 0.05,
                    78 => -0.25,
                    _ => 0.6,
                };
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                self.game.suspicion = 0.0;
                return Some(1.5);
            }
            77 => self.shot_request = Some(shot("36_collapse_aisle")),
            79 => self.shot_request = Some(shot("37_collapse_shelf")),
            81 => self.shot_request = Some(shot("38_collapse_ceiling")),
            82 => {
                // Kassengang, Blick auf den Ueberwachungsmonitor (gekippt)
                self.start_game(seed);
                self.game.tutorial = None;
                self.game.tip();
                self.game.customers.clear();
                self.game.employees.clear();
                self.game.message = None;
                let lane = self.game.open_lane().unwrap();
                self.game.player.pos = vec3(lane.scan_point.x, 0.0, lane.monitor.z - 2.5);
                let to = lane.monitor - (self.game.player.pos + vec3(0.0, game::EYE_HEIGHT, 0.0));
                self.game.player.yaw = to.x.atan2(-to.z);
                self.game.player.pitch = (to.y / to.length()).asin();
                self.game.trolley.pushing = false;
                self.game.trolley.pos = self.game.world.corral;
                self.game.glitch = 0.0;
                return Some(0.8);
            }
            83 => self.shot_request = Some(shot("39_monitor")),
            _ => return None,
        }
        Some(0.6)
    }
}
