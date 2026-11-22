//! Cheats zum Testen der Jumpscares: jeder teleportiert an eine passende Stelle und loest
//! genau einen Schreck aus (Menue mit F1, untere Haelfte).

use super::mirage::Hologram;
use super::scares::CartGhost;
use super::*;

/// Index des ersten Schreck-Cheats in `CHEATS`
pub const FIRST_SCARE_CHEAT: usize = 15;

impl Game {
    pub(super) fn scare_cheat(&mut self, k: usize) -> String {
        match k {
            0 => self.cheat_behind(),
            1 => self.cheat_blackout(),
            2 => self.cheat_cashier(),
            3 => self.cheat_cart_ghost(),
            4 => self.cheat_mirage(),
            5 => self.cheat_ceiling(false),
            6 => self.cheat_ceiling(true),
            7 => self.cheat_monitor(),
            8 => self.cheat_death(),
            _ => {
                self.troll_request = true;
                "Jumpscares abschalten ...".into()
            }
        }
    }

    /// Markt kippen, mindestens Stufe `t`, Schreck sofort erlaubt, nichts im Weg.
    fn scare_stage(&mut self, t: Tier) {
        self.tip();
        if self.tier < t {
            self.tier = t;
            self.apply_tier_params();
            self.rebuild_world();
        }
        self.scare_cd = 0.0;
        self.dialog = None;
        self.dying = None;
        self.behind = None;
        self.blackout = None;
        self.monitor_ghost = None;
        self.trolley.pushing = false;
    }

    /// Eine Stelle mitten in einem Gang mit freier Sicht nach vorn und Platz nach hinten.
    /// Der Spieler steht dort und schaut den Gang entlang (+z).
    fn stand_in_aisle(&mut self) {
        let w = &self.world;
        let mut best = None;
        'search: for k in [2.0f32, 1.0, 3.0, 0.0, 4.0] {
            for dz in [6.0f32, 4.0, 8.0, 2.5] {
                let want = vec3(world::AISLES_X0 + 3.4 * k, 0.0, w.aisle_z0 + dz);
                let p = w.collide(want, 0.3);
                let ahead = p + vec3(0.0, 0.0, 3.0);
                let behind = p - vec3(0.0, 0.0, 1.8);
                let free = |q: Vec3| (w.collide(q, 0.3) - q).length() < 0.05;
                if (p - want).length() < 0.4 && free(ahead) && free(behind) {
                    best = Some(p);
                    break 'search;
                }
            }
        }
        self.player.pos = best.unwrap_or(self.world.spawn);
        self.player.vel = Vec3::ZERO;
        self.player.yaw = std::f32::consts::PI;
        self.player.pitch = 0.0;
    }

    /// Kundin weit weg holen (oder neu erzeugen); `stalker` = soll sie verfolgen?
    fn cheat_customer(&mut self, stalker: bool) -> usize {
        let i = match (0..self.customers.len()).find(|&i| self.customers[i].stalker == stalker) {
            Some(i) => i,
            None => {
                let p = self.far_walkable_cell(10.0).unwrap_or(self.world.spawn);
                self.spawn_customer_at(p);
                self.customers.len() - 1
            }
        };
        self.customers[i].stalker = stalker;
        i
    }

    fn cheat_behind(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        self.stand_in_aisle();
        let i = self.cheat_customer(true);
        let player = self.player.pos;
        let spot = self.world.collide(player - vec3(0.0, 0.0, 1.6), 0.25);
        let c = &mut self.customers[i];
        c.pos = spot;
        c.yaw = npc::yaw_towards(spot, player);
        c.stare = 9.0;
        c.seen = false;
        c.field.clear();
        self.behind = Some(i);
        self.hush_t = 1.0;
        "Dreh dich um.".into()
    }

    fn cheat_blackout(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        self.stand_in_aisle();
        self.cheat_customer(false);
        self.blackout = Some((0.0, None));
        self.sfx.push((Sfx::Spark, 1.0));
        "Stromausfall.".into()
    }

    fn cheat_cashier(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        let lane = self.open_lane().unwrap();
        self.player.pos = lane.scan_point;
        let to = vec3(lane.seat.x, 1.2, lane.seat.z) - self.eye();
        self.player.yaw = to.x.atan2(-to.z);
        self.player.pitch = (to.y / to.length()).asin().min(0.2);
        self.cashier_done = false;
        self.start_cashier_scare();
        "Kassiererin.".into()
    }

    fn cheat_cart_ghost(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        self.stand_in_aisle();
        let p = self.player.pos;
        // Wagen weiter hinten im Gang, rollt sofort quietschend heran
        let mut spot = p;
        for d in [9.0f32, 7.0, 5.0, 4.0] {
            let want = p + vec3(0.0, 0.0, d);
            let q = self.world.collide(want, 0.42);
            if (q - want).length() < 0.2 {
                spot = q;
                break;
            }
        }
        self.trolley.pos = spot;
        self.trolley.yaw = 0.0;
        self.cart_ghost = CartGhost::Rolling;
        "Dein Wagen kommt.".into()
    }

    fn cheat_mirage(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        self.stand_in_aisle();
        let p = self.player.pos;
        let pos = self.world.collide(p + vec3(0.0, 0.0, 3.4), 0.25);
        self.holograms.clear();
        let person = Some(self.pick_person(false));
        self.holograms.push(Hologram {
            pos,
            yaw: npc::yaw_towards(pos, p) + std::f32::consts::PI,
            mesh: 0,
            person,
            tint: vec3(1.0, 0.55, 0.5),
            scale: Vec3::ONE,
            life: 30.0,
            real_tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            revealed: -1.0,
            scary: false,
        });
        self.mirage_force_scary = true;
        "Fata Morgana: einen Schritt vorgehen (W).".into()
    }

    fn cheat_ceiling(&mut self, hanging: bool) -> String {
        self.scare_stage(if hanging { Tier::Collapse } else { Tier::Wrong });
        if self.world.ceiling_holes.is_empty() {
            return "Kein Deckenloch im Markt.".into();
        }
        let h = self.world.ceiling_holes[0];
        // nur dieses eine Loch scharf schalten
        self.hole_done = vec![true; self.world.ceiling_holes.len()];
        self.hole_done[0] = false;
        // daneben stehen, mit Blick aufs Loch
        let mut stand = self.world.collide(vec3(h.x, 0.0, h.z - 1.5), 0.3);
        for (dx, dz) in [
            (0.0f32, -1.5f32),
            (0.0, 1.5),
            (1.5, 0.0),
            (-1.5, 0.0),
            (0.0, -1.0),
        ] {
            let want = vec3(h.x + dx, 0.0, h.z + dz);
            let q = self.world.collide(want, 0.3);
            if (q - want).length() < 0.05 {
                stand = q;
                break;
            }
        }
        self.player.pos = stand;
        let target = if hanging {
            vec3(h.x, crate::world::CEILING_HEIGHT - 0.4, h.z)
        } else {
            vec3(h.x, 1.0, h.z)
        };
        let to = target - self.eye();
        self.player.yaw = to.x.atan2(-to.z);
        self.player.pitch = (to.y / to.length()).asin();
        if hanging {
            "Schau hoch.".into()
        } else {
            "Unter dem Deckenloch.".into()
        }
    }

    fn cheat_monitor(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        let lane = self.open_lane().unwrap();
        self.player.pos = vec3(lane.scan_point.x, 0.0, lane.monitor.z - 2.5);
        let to = lane.monitor - self.eye();
        self.player.yaw = to.x.atan2(-to.z);
        self.player.pitch = (to.y / to.length()).asin();
        self.monitor_watch = 10.0;
        "Schau auf den Monitor.".into()
    }

    fn cheat_death(&mut self) -> String {
        self.scare_stage(Tier::Uneasy);
        self.stand_in_aisle();
        let catcher = self
            .world
            .collide(self.player.pos + vec3(0.0, 0.0, 0.7), 0.25);
        let variant = self.pick_person(false);
        // kostet kein Leben
        let lives = self.deaths;
        self.deaths = 0;
        self.caught_by(catcher, variant);
        self.deaths = lives;
        "Erwischt (kostet kein Leben).".into()
    }
}

impl Game {
    /// Hat ein Cheat den Troll-Jumpscare angefordert? (setzt die Anfrage zurueck)
    pub fn take_troll_request(&mut self) -> bool {
        std::mem::take(&mut self.troll_request)
    }
}
