//! Gestalt in der Ferne: Im gekippten Markt steht ab und zu am Ende einer langen Sichtlinie
//! eine reglose, dunkle Figur im Nebel und sieht einen an. Kommt man naeher oder sieht weg
//! und wieder hin, ist sie fort. Im Kollaps steht sie nach dem Wegsehen manchmal ein Stueck
//! naeher. Sie ist kein NPC: keine Kollision, sie faengt nie.

use super::*;
use crate::people::{Clip, PersonDraw, PoseFx};

/// Abstand beim Auftauchen (Meter)
const SPAWN_DIST: (f32, f32) = (14.0, 25.0);
/// Naeher als das: weg
const VANISH_DIST: f32 = 10.0;
/// So nah kommt sie im Kollaps hoechstens
pub const CLOSEST: f32 = 8.0;
/// So oft rueckt sie im Kollaps naeher
const MAX_STEPS: u8 = 3;
/// So lange muss man wegsehen, damit sie weg ist bzw. naeher rueckt (s)
const AWAY_TIME: f32 = 0.4;
/// Hoechstens so lange steht sie da (s)
const LIFE: f32 = 15.0;
/// Augenhoehe der Figur fuer die Sichtpruefung
const HEAD: f32 = 1.5;

#[derive(Clone, Debug)]
pub struct Watcher {
    pub pos: Vec3,
    pub yaw: f32,
    pub variant: usize,
    pub life: f32,
    /// schon einmal gesehen?
    pub seen: bool,
    /// so lange nicht im Bild (s)
    pub away: f32,
    /// so oft schon naeher gerueckt (Kollaps)
    pub steps: u8,
}

impl Game {
    /// Liegt etwas zwischen dem Spieler und einem Geraeusch? (dann klingt es dumpf)
    pub fn sound_hidden(&self, at: Vec3) -> bool {
        let eye = self.eye();
        // Geraeusche am Boden kommen aus Huefthoehe; der letzte halbe Meter zaehlt nicht
        // (Klopfen an einer Tuer liegt in der Tuer)
        let at = vec3(at.x, at.y.max(1.0), at.z);
        let d = at - eye;
        let len = d.length();
        len > 1.0 && !self.world.line_of_sight(eye, eye + d * ((len - 0.5) / len))
    }

    /// Cheat: Markt kippen und in den ersten Raum des Seitentrakts stellen, Blick nach Westen.
    pub(super) fn debug_to_wing(&mut self) -> String {
        self.tip();
        let Some(room) = self
            .world
            .wing
            .as_ref()
            .and_then(|w| w.rooms.first().copied())
        else {
            return "Kein Seitentrakt.".into();
        };
        let c = (room.min + room.max) * 0.5;
        self.player.pos = self
            .world
            .collide(vec3(room.max.x - 1.0, 0.0, c.z), PLAYER_RADIUS);
        self.player.yaw = -std::f32::consts::FRAC_PI_2;
        self.player.pitch = 0.0;
        "Seitentrakt. Hier ist niemand.".into()
    }

    /// Figur fuer Gestalt und Deja-vu (eigener Zufall, siehe `eerie_rng`).
    pub(super) fn eerie_person(&mut self) -> usize {
        let pool = crate::assets::people_of(false);
        pool[self.eerie_rng.gen_range(0..pool.len())]
    }

    /// Zeit bis zur naechsten Gestalt (s), je Stufe.
    fn watcher_wait(&mut self) -> f32 {
        if self.tier == Tier::Collapse {
            self.eerie_rng.gen_range(18.0..35.0)
        } else {
            self.eerie_rng.gen_range(30.0..60.0)
        }
    }

    pub(super) fn update_watcher(&mut self, dt: f32) {
        if !self.tipped || self.paid || self.umbau_running() || self.checking_out() {
            self.watcher = None;
            return;
        }
        let Some(mut w) = self.watcher.take() else {
            self.watcher_timer -= dt;
            if self.watcher_timer <= 0.0 {
                self.watcher_timer = self.watcher_wait();
                self.spawn_watcher();
            }
            return;
        };
        w.life -= dt;
        let player = self.player.pos;
        let d = vec3(w.pos.x - player.x, 0.0, w.pos.z - player.z).length();
        let visible = self.sees(w.pos + vec3(0.0, HEAD, 0.0));
        if d < VANISH_DIST {
            self.vanish_watcher();
            return;
        }
        if visible {
            w.seen = true;
            w.away = 0.0;
            self.watcher = Some(w);
            return;
        }
        if !w.seen {
            // noch nie gesehen: nach Ablauf still verschwinden
            if w.life > 0.0 {
                self.watcher = Some(w);
            }
            return;
        }
        w.away += dt;
        if w.away < AWAY_TIME {
            self.watcher = Some(w);
            return;
        }
        // weggesehen: im Kollaps manchmal naeher, sonst weg
        if self.tier == Tier::Collapse && w.steps < MAX_STEPS && self.eerie_rng.gen_bool(0.6) {
            let to = vec3(w.pos.x - player.x, 0.0, w.pos.z - player.z);
            let nd = (d * 0.7).max(CLOSEST);
            let p = player + to.normalize_or_zero() * nd;
            if self.watcher_spot_ok(p) {
                w.pos = vec3(p.x, 0.0, p.z);
                w.yaw = npc::yaw_towards(w.pos, player);
                w.steps += 1;
                w.seen = false;
                w.away = 0.0;
                w.life = LIFE;
                self.watcher = Some(w);
                return;
            }
        }
        self.vanish_watcher();
    }

    /// Leises Rauschen, Gestalt weg.
    fn vanish_watcher(&mut self) {
        self.watcher = None;
        self.play(Sfx::Static, 0.08);
    }

    /// Steht man dort (begehbar, frei sichtbar vom Spieler)?
    fn watcher_spot_ok(&self, p: Vec3) -> bool {
        let nav = &self.world.nav;
        let walkable = nav.cell_of(p).is_some_and(|(x, z)| nav.is_walkable(x, z));
        walkable
            && !self.world.drinks_zone.contains_xz(p)
            && self
                .world
                .line_of_sight(self.eye(), p + vec3(0.0, HEAD, 0.0))
    }

    /// Eine Gestalt am Ende der Sichtlinie aufstellen (wenn es eine lange freie Sicht gibt).
    pub(super) fn spawn_watcher(&mut self) -> bool {
        // nie gleichzeitig mit der Deja-vu-Szene im Bild
        if let Some(s) = &self.dejavu {
            if self.sees(s.pos + vec3(0.0, 0.8, 0.0)) {
                return false;
            }
        }
        let player = self.player.pos;
        let f = self.view_dir();
        let fwd = vec3(f.x, 0.0, f.z).normalize_or_zero();
        let mut spot = None;
        // im Trakt: bevorzugt am Ende des Endlos-Flurs
        if let Some(end) = self
            .world
            .wing
            .as_ref()
            .filter(|w| w.contains(player))
            .and_then(|w| w.hall_end)
        {
            let p = end + vec3(1.5, 0.0, 0.0);
            if (p - player).length() > VANISH_DIST && self.watcher_spot_ok(p) {
                spot = Some(p);
            }
        }
        for _ in 0..40 {
            if spot.is_some() {
                break;
            }
            let a = self.eerie_rng.gen_range(-0.25..0.25f32);
            let dir = Quat::from_rotation_y(a) * fwd;
            let d = self.eerie_rng.gen_range(SPAWN_DIST.0..SPAWN_DIST.1);
            let p = player + dir * d;
            if self.watcher_spot_ok(p) && self.sees(p + vec3(0.0, HEAD, 0.0)) {
                spot = Some(p);
            }
        }
        let Some(p) = spot else { return false };
        let pos = vec3(p.x, 0.0, p.z);
        let variant = self.eerie_person();
        self.watcher = Some(Watcher {
            pos,
            yaw: npc::yaw_towards(pos, player),
            variant,
            life: LIFE,
            seen: false,
            away: 0.0,
            steps: 0,
        });
        true
    }

    /// Die Gestalt: reglos, fast schwarz, leicht gestreckt.
    pub(super) fn collect_watcher(&self, out: &mut Vec<PersonDraw>) {
        let Some(w) = &self.watcher else { return };
        let mut fx = PoseFx::default();
        fx.stretch = 0.25;
        out.push(PersonDraw {
            variant: w.variant,
            clip: Clip::Idle,
            time: 0.0,
            pos: w.pos + vec3(0.0, self.sway_at(w.pos).y, 0.0),
            yaw: w.yaw,
            tint: Vec4::new(0.06, 0.06, 0.07, 0.0),
            ghost: false,
            flip: false,
            look_at: None,
            fx,
        });
    }
}
