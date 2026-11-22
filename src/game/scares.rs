//! Jumpscares: aus den Spielregeln heraus, mit kurzer Abklingzeit und Stille davor.
//! Kleine Schrecken sobald der Markt gekippt ist, grosse ab "unruhig". Nie zwei direkt hintereinander.
//!
//! - Stromausfall: kurz stockdunkel, wenn das Licht wiederkommt, steht jemand vor einem
//!
//! - Umdrehen: ein Verfolger steht ploetzlich direkt hinter einem
//! - Kassiererin steht nach dem letzten gescannten Artikel ruckartig auf
//! - der abgestellte Wagen ist weg und rollt dann quietschend von selbst heran
//! - Deckenloch: Kabel stuerzen funkensprühend herab; im Kollaps haengt jemand kopfueber darin
//! (Tod mit Gesicht in Grossaufnahme: death.rs, Fata Morgana: mirage.rs,
//! Ueberwachungsmonitor: monitor.rs, "Jumpscares aus" im Menue: main.rs menu_scare)

use super::*;
use crate::people::{Clip, PersonDraw};

/// Abklingzeit nach einem grossen Schreck (Sekunden, zufaellig dazwischen)
const SCARE_COOLDOWN: (f32, f32) = (18.0, 32.0);
/// Stille vor einem angekuendigten Schreck
const HUSH_TIME: f32 = 1.2;
/// "Hinter dir": wie weit hinter dem Spieler, wie lange er wartet
const BEHIND_DIST: f32 = 1.2;
const BEHIND_WAIT: f32 = 9.0;
/// Wagen-Spuk: so lange unbeobachtet, bevor er verschwindet; Rolltempo; Halteabstand
const CART_UNSEEN: f32 = 3.0;
const CART_GONE: f32 = 2.5;
const CART_ROLL_SPEED: f32 = 1.6;
const CART_STOP: f32 = 2.3;
/// Kassiererin: Dauer des Aufstehens
const CASHIER_TIME: f32 = 1.8;
/// Stromausfall: Dauer, ab wann die Gestalt steht, wie weit vor einem, Chance pro Sekunde
const BLACKOUT_TIME: f32 = 1.0;
const BLACKOUT_PLACE: f32 = 0.35;
const BLACKOUT_DIST: f32 = 2.6;
const BLACKOUT_CHANCE: f32 = 0.035;
/// Schreck-Effekt: so lange laufen Farbumkehr-Stroboskop, Lichtblitz und Zoom-Ruck
const SCARE_FX: f32 = 0.55;
/// Unterschwellige Farbumkehr-Blitze ab "falsch": Abstand in Sekunden, Dauer
const SUBLIMINAL_EVERY: (f32, f32) = (14.0, 30.0);
const SUBLIMINAL_TIME: f32 = 0.09;
/// "Hinter dir": Chance pro Sekunde, abgelenkt bzw. sonst
const BEHIND_CHANCE: (f32, f32) = (0.6, 0.04);

/// Spuk um den abgestellten Wagen.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum CartGhost {
    #[default]
    Idle,
    /// verschwunden (steht unsichtbar weit weg), Sekunden bis er losrollt
    Gone(f32),
    /// rollt quietschend auf den Spieler zu
    Rolling,
}

impl Game {
    /// Darf jetzt ein Schreck kommen? Gross: ab "falsch", klein: ab "unruhig".
    pub(super) fn can_scare(&self, big: bool) -> bool {
        let stage = if big { Tier::Uneasy } else { Tier::Calm };
        self.tipped
            && self.tier >= stage
            && self.scare_cd <= 0.0
            && self.outcome == Outcome::Running
            && self.dying.is_none()
            && self.dialog.is_none()
            && !self.paid
            && !self.till_paying()
    }

    /// Schreck ausloesen: harter Klang, Bildriss, kurzer roter Blitz, danach Abklingzeit.
    pub(super) fn fire_scare(&mut self, strength: f32) {
        self.sfx.push((Sfx::Scare, 0.6 + 0.4 * strength));
        self.glitch = self.glitch.max(0.6 + 0.4 * strength);
        self.red = self.red.max(0.35 * strength);
        self.hush_t = 0.0;
        self.scare_cd = self.rng.gen_range(SCARE_COOLDOWN.0..SCARE_COOLDOWN.1);
        self.scare_fx = SCARE_FX;
        self.scare_fx_strength = strength;
    }

    /// 0..1 wie weit der Schreck-Effekt noch laeuft (1 = gerade ausgeloest)
    fn scare_fx_k(&self) -> f32 {
        (self.scare_fx / SCARE_FX).clamp(0.0, 1.0)
    }

    /// Farbumkehr (0/1): Stroboskop beim Schreck, Flackern am Ende des Stromausfalls,
    /// einzelne unterschwellige Blitze.
    pub fn scare_invert(&self) -> f32 {
        let t = self.time;
        let k = self.scare_fx_k();
        let strobe = |hz: f32| ((t * hz) as i64 % 2 == 0) as i32 as f32;
        let mut inv = 0.0f32;
        if k > 0.35 {
            inv = strobe(18.0);
        }
        if let Some((bt, _)) = self.blackout {
            if bt > BLACKOUT_TIME - 0.3 {
                inv = inv.max(strobe(24.0));
            }
        }
        if self.subliminal > 0.0 && self.subliminal < SUBLIMINAL_TIME {
            inv = 1.0;
        }
        inv
    }

    /// Ueberbelichtung: das Licht schlaegt beim Schreck grell aus und flackert ab.
    pub fn scare_flash(&self) -> f32 {
        let k = self.scare_fx_k();
        if k <= 0.0 {
            return 0.0;
        }
        let flick = 0.6 + 0.4 * ((self.time * 40.0).sin() * 0.5 + 0.5);
        k * k * flick * (0.5 + 0.5 * self.scare_fx_strength)
    }

    /// Zoom-Ruck: das Sichtfeld zieht beim Schreck ruckartig zusammen (Faktor < 1).
    pub fn scare_zoom(&self) -> f32 {
        let k = self.scare_fx_k();
        1.0 - 0.3 * k * k * (0.5 + 0.5 * self.scare_fx_strength)
    }

    fn update_scare_fx(&mut self, dt: f32) {
        self.scare_fx = (self.scare_fx - dt).max(0.0);
        // unterschwellige Blitze: nur im gekippten Markt ab "falsch"
        if !self.tipped || self.tier < Tier::Wrong || self.dying.is_some() {
            self.subliminal = -1.0;
            return;
        }
        if self.subliminal < 0.0 {
            self.subliminal_wait = self.rng.gen_range(SUBLIMINAL_EVERY.0..SUBLIMINAL_EVERY.1);
            self.subliminal = SUBLIMINAL_TIME;
        }
        if self.subliminal < SUBLIMINAL_TIME {
            self.subliminal += dt;
        } else {
            self.subliminal_wait -= dt;
            if self.subliminal_wait <= 0.0 {
                self.subliminal = 0.0;
                self.subliminal_wait = self.rng.gen_range(SUBLIMINAL_EVERY.0..SUBLIMINAL_EVERY.1);
            }
        }
    }

    /// 0..1: wie still es gerade ist (fuer die Musik)
    pub fn hush(&self) -> f32 {
        (self.hush_t / 0.6).clamp(0.0, 1.0)
    }

    pub(super) fn update_scares(&mut self, dt: f32, input: &Input) {
        self.scare_cd = (self.scare_cd - dt).max(0.0);
        self.hush_t = (self.hush_t - dt).max(0.0);
        self.update_scare_fx(dt);
        self.update_behind(dt, input);
        self.update_cashier_scare(dt);
        self.update_cart_ghost(dt);
        self.update_ceiling_scares(dt);
        self.update_monitor_scare(dt);
        self.update_blackout(dt);
    }

    // ------------------------------------------------------------ Hinter dir

    fn update_behind(&mut self, dt: f32, input: &Input) {
        if let Some(i) = self.behind {
            let Some(c) = self.customers.get_mut(i) else {
                self.behind = None;
                return;
            };
            if c.seen {
                // umgedreht: er steht direkt vor einem
                c.stare = 1.5;
                self.behind = None;
                self.fire_scare(1.0);
            } else if c.stare <= 0.0 {
                self.behind = None;
            }
            return;
        }
        if !self.can_scare(true) {
            return;
        }
        // abgelenkt: Zettel offen oder Blick ins Regal, dazu ein bisschen Zufall
        let distracted = input.list_open || matches!(self.focus, Some(Focus::Shelf(_)));
        let chance = if distracted {
            BEHIND_CHANCE.0
        } else {
            BEHIND_CHANCE.1
        } * dt;
        if !self.rng.gen_bool(chance.clamp(0.0, 1.0) as f64) {
            return;
        }
        let player = self.player.pos;
        let Some(i) = (0..self.customers.len()).find(|&i| {
            let c = &self.customers[i];
            c.stalker && !c.seen && (c.pos - player).length() > 7.0
        }) else {
            return;
        };
        let fwd = self.view_dir();
        let back = vec3(-fwd.x, 0.0, -fwd.z).normalize_or_zero();
        let spot = self.world.collide(player + back * BEHIND_DIST, 0.25);
        if (spot - player).length() < 1.0 || self.sees(spot + vec3(0.0, 1.2, 0.0)) {
            return;
        }
        let c = &mut self.customers[i];
        c.pos = spot;
        c.yaw = npc::yaw_towards(spot, player);
        c.stare = BEHIND_WAIT;
        c.field.clear();
        self.behind = Some(i);
        self.hush_t = HUSH_TIME;
    }

    // ------------------------------------------------------------ Kassiererin

    /// Nach dem letzten gescannten Artikel: sie steht auf und beugt sich vor.
    pub(super) fn start_cashier_scare(&mut self) {
        if self.cashier_done || !self.tipped {
            return;
        }
        self.cashier_done = true;
        self.cashier_scare = Some(0.0);
        self.fire_scare(1.0);
    }

    fn update_cashier_scare(&mut self, dt: f32) {
        if let Some(t) = &mut self.cashier_scare {
            *t += dt;
            if *t > CASHIER_TIME {
                self.cashier_scare = None;
            }
        }
    }

    // ------------------------------------------------------------ Stromausfall

    fn update_blackout(&mut self, dt: f32) {
        if let Some((t, figure)) = self.blackout {
            let now = t + dt;
            let mut figure = figure;
            if t < BLACKOUT_PLACE && now >= BLACKOUT_PLACE {
                figure = self.place_blackout_figure();
            }
            if now >= BLACKOUT_TIME {
                self.blackout = None;
                if figure.is_some() {
                    self.fire_scare(1.0);
                } else {
                    self.sfx.push((Sfx::Sting, 0.6));
                }
            } else {
                self.blackout = Some((now, figure));
            }
            return;
        }
        if !self.can_scare(true) || self.checking_out() {
            return;
        }
        if !self
            .rng
            .gen_bool((BLACKOUT_CHANCE * dt).clamp(0.0, 1.0) as f64)
        {
            return;
        }
        self.blackout = Some((0.0, None));
        self.sfx.push((Sfx::Spark, 1.0));
        self.hush_t = HUSH_TIME;
    }

    /// Im Dunkeln: eine (harmlose) Kundin direkt vor den Spieler stellen.
    fn place_blackout_figure(&mut self) -> Option<usize> {
        let player = self.player.pos;
        let fwd = self.view_dir();
        let flat = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let want = player + flat * BLACKOUT_DIST;
        let spot = self.world.collide(want, 0.25);
        if (spot - want).length() > 0.3 || !self.sees(spot + vec3(0.0, 1.2, 0.0)) {
            return None;
        }
        let i = (0..self.customers.len())
            .filter(|&i| !self.customers[i].stalker)
            .max_by(|&a, &b| {
                let da = (self.customers[a].pos - player).length();
                let db = (self.customers[b].pos - player).length();
                da.total_cmp(&db)
            })?;
        let c = &mut self.customers[i];
        c.pos = spot;
        c.yaw = npc::yaw_towards(spot, player);
        c.stare = 3.0;
        c.field.clear();
        Some(i)
    }

    /// Wie dunkel der Stromausfall das Bild gerade macht (0..1); am Ende flackert das Licht.
    pub fn blackout(&self) -> f32 {
        match self.blackout {
            Some((t, _)) if t > BLACKOUT_TIME - 0.3 => {
                if ((t * 30.0) as i32) % 2 == 0 {
                    1.0
                } else {
                    0.3
                }
            }
            Some(_) => 1.0,
            None => 0.0,
        }
    }

    /// Steht die Kassiererin gerade (statt zu sitzen)?
    pub fn cashier_standing(&self) -> bool {
        self.cashier_scare.is_some()
    }

    // ------------------------------------------------------------ Wagen-Spuk

    fn update_cart_ghost(&mut self, dt: f32) {
        let player = self.player.pos;
        let carried = self.employees.iter().any(|e| e.carrying);
        match self.cart_ghost {
            CartGhost::Idle => {
                let watched = self.sees(self.trolley.pos + vec3(0.0, 0.5, 0.0));
                let far = (self.trolley.pos - player).length() > 6.0;
                if self.trolley.pushing || carried || watched || !far || self.checking_out() {
                    self.cart_unseen = 0.0;
                    return;
                }
                self.cart_unseen += dt;
                if self.cart_unseen < CART_UNSEEN || !self.can_scare(false) {
                    return;
                }
                // weg: an einen unbeobachteten Platz weit weg
                if let Some(p) = self.far_walkable_cell(12.0) {
                    self.trolley.pos = p;
                    self.cart_ghost = CartGhost::Gone(CART_GONE);
                    self.cart_unseen = 0.0;
                    self.scare_cd = self.rng.gen_range(20.0..35.0);
                }
            }
            CartGhost::Gone(t) => {
                if self.trolley.pushing {
                    self.cart_ghost = CartGhost::Idle;
                    return;
                }
                let t = t - dt;
                self.cart_ghost = if t <= 0.0 {
                    CartGhost::Rolling
                } else {
                    CartGhost::Gone(t)
                };
            }
            CartGhost::Rolling => {
                if self.trolley.pushing || carried {
                    self.cart_ghost = CartGhost::Idle;
                    return;
                }
                if (self.trolley.pos - player).length() < CART_STOP {
                    // knallt direkt vor einem zum Stehen
                    self.cart_ghost = CartGhost::Idle;
                    self.sfx.push((Sfx::Rattle, 1.0));
                    self.fire_scare(0.7);
                    return;
                }
                let old = self.trolley.pos;
                let new = npc::step_along(
                    &self.world,
                    &self.player_field,
                    old,
                    player,
                    CART_ROLL_SPEED,
                    dt,
                );
                self.trolley.pos = self.world.collide(new, 0.3);
                if (new - old).length_squared() > 1e-8 {
                    // Griff zeigt nach hinten: der Wagen rollt mit der Front voran
                    self.trolley.yaw = std::f32::consts::PI - npc::yaw_towards(old, new);
                }
                self.squeak_acc += (self.trolley.pos - old).length();
                if self.squeak_acc > 1.3 {
                    self.squeak_acc = 0.0;
                    let d = (self.trolley.pos - player).length();
                    self.sfx
                        .push((Sfx::SqueakSoft, (1.0 - d / 25.0).max(0.05) * 0.45));
                }
            }
        }
    }

    // ------------------------------------------------------------ Deckenloch

    fn update_ceiling_scares(&mut self, dt: f32) {
        let holes = self.world.ceiling_holes.len();
        if self.hole_done.len() != holes {
            self.hole_done = vec![false; holes];
        }
        if let Some((_, t)) = &mut self.hanging {
            *t += dt;
            if *t > 1.6 {
                self.hanging = None;
            }
        }
        for d in &mut self.cable_drops {
            d.1 += dt;
        }
        self.cable_drops.retain(|d| d.1 < 12.0);
        if !self.can_scare(false) {
            return;
        }
        let player = self.player.pos;
        for i in 0..holes {
            if self.hole_done[i] {
                continue;
            }
            let h = self.world.ceiling_holes[i];
            let d = vec3(h.x - player.x, 0.0, h.z - player.z).length();
            // im Kollaps: wer hochschaut, sieht jemanden kopfueber im Loch haengen
            let up = self.player.pitch > 0.45 && d < 4.0 && self.sees(vec3(h.x, 3.6, h.z));
            if self.tier >= Tier::Collapse && up && self.can_scare(true) {
                self.hole_done[i] = true;
                self.hanging = Some((h, 0.0));
                self.fire_scare(1.0);
                return;
            }
            // darunter durchlaufen: Kabel stuerzen funkensprühend herab
            if d < 2.2 {
                self.hole_done[i] = true;
                let fwd = self.view_dir();
                let spot = player + vec3(fwd.x, 0.0, fwd.z).normalize_or_zero() * 1.1;
                self.cable_drops.push((vec3(spot.x, 0.0, spot.z), 0.0));
                self.sfx.push((Sfx::Spark, 0.9));
                self.fire_scare(0.5);
                return;
            }
        }
    }

    /// Herabstuerzende Kabel mit Funken.
    pub(super) fn collect_scare_draws(&self, out: &mut Vec<MeshDraw>) {
        let cube = MeshId::Cube as usize;
        for &(p, t) in &self.cable_drops {
            for k in 0..5 {
                let h = crate::world::hash01(p + vec3(k as f32, 0.0, 0.0));
                let color = CABLE_TINTS[k % CABLE_TINTS.len()];
                let fall = (crate::world::CEILING_HEIGHT - 0.5 * 9.81 * t * t).max(0.03);
                let off = vec3((h - 0.5) * 0.5, 0.0, (h * 7.0).fract() * 0.5 - 0.25);
                let lying = fall <= 0.03;
                let model = if lying {
                    Mat4::from_scale_rotation_translation(
                        vec3(0.02, 0.02, 0.9),
                        Quat::from_rotation_y(h * 6.0),
                        p + off + vec3(0.0, 0.01, 0.0),
                    )
                } else {
                    Mat4::from_scale_rotation_translation(
                        vec3(0.02, 0.9, 0.02),
                        Quat::from_rotation_z((h - 0.5) * 0.6),
                        p + off + vec3(0.0, fall + 0.45, 0.0),
                    )
                };
                out.push(MeshDraw {
                    mesh: cube,
                    model,
                    tint: color.extend(0.0),
                });
            }
            // Funkenregen beim Aufschlag
            if (0.5..1.2).contains(&t) {
                let c = t - 0.5;
                for k in 0..16 {
                    let h = crate::world::hash01(p + vec3(k as f32 * 1.7, 3.0, 0.0));
                    let a = h * std::f32::consts::TAU;
                    let v = vec3(a.cos() * (0.8 + h), 1.0 + h * 1.5, a.sin() * (0.8 + h));
                    let q = p + vec3(0.0, 0.05, 0.0) + v * c - vec3(0.0, 0.5 * 9.81 * c * c, 0.0);
                    if q.y < 0.0 {
                        continue;
                    }
                    out.push(MeshDraw {
                        mesh: cube,
                        model: Mat4::from_scale_rotation_translation(
                            Vec3::splat(0.008),
                            Quat::IDENTITY,
                            q,
                        ),
                        tint: Vec4::new(1.0, 0.85, 0.45, 4.0 * (1.0 - c / 0.7)),
                    });
                }
            }
        }
    }

    /// Figuren der Schrecken: aufgestandene Kassiererin, Gestalt kopfueber im Deckenloch,
    /// Gesicht in Grossaufnahme beim Tod.
    pub(super) fn collect_scare_people(&self, out: &mut Vec<PersonDraw>) {
        let staff = crate::people::people_of(true);
        // Kassiererin steht auf und beugt sich zum Spieler
        if let (Some(t), Some(lane)) = (self.cashier_scare, self.open_lane()) {
            let k = (t / 0.25).min(1.0);
            let lean = vec3(-0.55 * k, 0.0, 0.0);
            let pos = lane.seat + lean;
            out.push(PersonDraw {
                // dieselbe Figur wie die Kassiererin an der Kasse
                variant: self.cashier_variant,
                clip: Clip::Idle,
                time: t,
                pos,
                yaw: npc::yaw_towards(pos, self.player.pos),
                tint: Vec4::new(0.8, 0.85, 0.9, 0.3),
                ghost: false,
                flip: false,
            });
        }
        // kopfueber im Deckenloch
        if let Some((h, t)) = self.hanging {
            let variant = staff[staff.len() / 2];
            let drop = (t / 0.15).min(1.0) * 0.4;
            out.push(PersonDraw {
                variant,
                clip: Clip::Idle,
                time: t * 0.3,
                pos: vec3(h.x, crate::world::CEILING_HEIGHT + 0.3 - drop, h.z),
                yaw: npc::yaw_towards(h, self.player.pos),
                tint: Vec4::new(0.7, 0.7, 0.7, 0.15),
                ghost: false,
                flip: true,
            });
        }
        // Tod: das leere Gesicht direkt vor der Kamera, bevor das Bild wegkippt
        if let (Some(k), Some(v)) = (self.dying_progress(), self.catcher_variant) {
            if k < 0.22 {
                out.push(self.face_in_view(v, 0.42));
            }
        }
    }

    /// Ein leeres Gesicht direkt vor der Kamera (Tod, Troll-Option im Menue).
    /// `dist` = Abstand vor dem Auge (Meter).
    pub fn face_in_view(&self, variant: usize, dist: f32) -> PersonDraw {
        let eye = self.eye();
        let fwd = self.view_dir();
        let flat = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let height = crate::people::height(variant);
        let pos = vec3(eye.x, eye.y - height * 0.93, eye.z) + flat * dist;
        PersonDraw {
            variant,
            clip: Clip::Idle,
            time: 0.0,
            pos,
            yaw: npc::yaw_towards(pos, eye),
            tint: Vec4::new(0.85, 0.85, 0.85, 0.2),
            ghost: false,
            flip: false,
        }
    }
}

/// Farben der herabstuerzenden Kabel
const CABLE_TINTS: [Vec3; 5] = [
    Vec3::new(0.75, 0.08, 0.06),
    Vec3::new(0.08, 0.25, 0.75),
    Vec3::new(0.9, 0.75, 0.1),
    Vec3::new(0.1, 0.55, 0.2),
    Vec3::new(0.05, 0.05, 0.05),
];
