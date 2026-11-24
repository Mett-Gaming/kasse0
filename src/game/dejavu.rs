//! Deja-vu: Im gekippten Markt (ab "unruhig") steht irgendwo vor einem eine kleine, auffaellige
//! Szene - ein schraeg stehender leerer Wagen, eine umgekippte Dose, ein Warnschild und oft
//! eine reglose Kundin, die ins Regal starrt. Biegt man in einen anderen Gang ab, steht
//! **genau dieselbe Szene** wieder vor einem, gleich ausgerichtet; die alte ist weg. Beim
//! dritten Mal hat sich die Kundin zu einem umgedreht.
//!
//! Die Szene wird nur dort hingestellt, wo man gerade nicht hinsieht (sie "taucht" nie auf).
//! Sie ist kein NPC und faengt nie; nur gegen den Wagen laeuft man wie gegen einen echten.

use super::*;
use crate::people::{Clip, PersonDraw};
use std::f32::consts::FRAC_PI_2;

/// Abstand vor dem Spieler (Meter, entlang des Gangs)
const AHEAD: [f32; 4] = [9.0, 8.0, 10.0, 7.0];
/// So oft taucht dieselbe Szene auf
pub const SHOWINGS: u8 = 3;
/// Radius des Wagens fuer die Spielerkollision
const CART_R: f32 = 0.45;

/// Teile der Szene relativ zu ihrer Mitte (Szene schaut zum Spieler, lokal +z = zu ihm hin).
const CART_AT: Vec3 = Vec3::new(0.35, 0.0, 0.0);
const CAN_AT: Vec3 = Vec3::new(-0.35, 0.0, 0.5);
const SIGN_AT: Vec3 = Vec3::new(0.6, 0.0, -0.9);
const CUSTOMER_AT: Vec3 = Vec3::new(-0.75, 0.0, -0.5);

#[derive(Clone, Debug)]
pub struct Dejavu {
    /// Mitte der Szene und ihre Ausrichtung (zum Spieler)
    pub pos: Vec3,
    pub yaw: f32,
    /// Gang, in dem sie gerade steht
    pub aisle: i32,
    /// so oft schon gesehen (je Platz einmal gezaehlt)
    pub shown: u8,
    /// am aktuellen Platz schon gesehen?
    pub seen_here: bool,
    /// so lange nicht mehr im Bild (s)
    pub unseen_for: f32,
    /// Kundin (Figur-Variante), falls dabei
    pub customer: Option<usize>,
    /// beim dritten Mal: Kundin dreht sich zum Spieler
    pub turned: bool,
    /// Abstand vor dem Spieler beim Aufstellen (wird beim Wiederauftauchen bevorzugt)
    pub ahead: f32,
}

impl Dejavu {
    fn world(&self, local: Vec3) -> Vec3 {
        self.pos + Quat::from_rotation_y(self.yaw) * local
    }
}

impl Game {
    fn dejavu_wait(&mut self) -> f32 {
        let k = self.tier.level() as f32;
        self.eerie_rng.gen_range(40.0..70.0) - k * 6.0
    }

    /// Gang (Spalte zwischen zwei Regalreihen), in dem ein Punkt liegt; None ausserhalb.
    pub(super) fn aisle_of(&self, p: Vec3) -> Option<i32> {
        let w = &self.world;
        if p.z < w.aisle_z0 || p.z > w.aisle_z1 || p.x < world::AISLES_X0 || p.x > w.aisle_x1 {
            return None;
        }
        let pitch = (w.aisle_x1 - world::AISLES_X0) / w.aisle_count.max(1) as f32;
        Some(((p.x - world::AISLES_X0) / pitch).round() as i32)
    }

    pub(super) fn update_dejavu(&mut self, dt: f32) {
        if !self.tipped
            || self.tier < Tier::Uneasy
            || self.paid
            || self.umbau_running()
            || self.checking_out()
        {
            self.dejavu = None;
            return;
        }
        let Some(mut s) = self.dejavu.take() else {
            self.dejavu_timer -= dt;
            if self.dejavu_timer <= 0.0 {
                // kein passender Platz: gleich noch einmal versuchen
                self.dejavu_timer = if self.place_dejavu(None) {
                    self.dejavu_wait()
                } else {
                    1.0
                };
            }
            return;
        };
        let visible = self.sees(s.pos + vec3(0.0, 0.8, 0.0));
        if visible {
            if !s.seen_here {
                s.seen_here = true;
                s.shown += 1;
            }
            s.unseen_for = 0.0;
        } else {
            s.unseen_for += dt;
        }
        // nicht durch den Wagen laufen
        let cart = s.world(CART_AT);
        let to = vec3(self.player.pos.x - cart.x, 0.0, self.player.pos.z - cart.z);
        let r = CART_R + PLAYER_RADIUS;
        if to.length() < r && to.length() > 1e-4 {
            self.player.pos = cart + to.normalize() * r + vec3(0.0, self.player.pos.y, 0.0);
        }
        // anderer Gang, Szene gesehen und gerade nicht im Bild: dieselbe Szene wieder vor einen
        let here = self.aisle_of(self.player.pos);
        if s.seen_here && !visible && s.shown < SHOWINGS && here.is_some_and(|a| a != s.aisle) {
            let keep = s.clone();
            if self.place_dejavu(Some(keep)) {
                return;
            }
        }
        if (s.shown >= SHOWINGS && s.unseen_for > 8.0) || s.unseen_for > 40.0 {
            self.dejavu_timer = self.dejavu_wait();
            return;
        }
        self.dejavu = Some(s);
    }

    /// Szene vor den Spieler stellen (neu oder dieselbe wieder), nur wo er gerade nicht
    /// hinsieht. Liefert, ob es geklappt hat.
    pub(super) fn place_dejavu(&mut self, again: Option<Dejavu>) -> bool {
        let player = self.player.pos;
        let Some(aisle) = self.aisle_of(player) else {
            if let Some(s) = again {
                self.dejavu = Some(s);
            }
            return false;
        };
        let f = self.view_dir();
        let dir = if f.z >= 0.0 { 1.0 } else { -1.0 };
        let eye = self.eye();
        // beim Wiederauftauchen zuerst derselbe Abstand wie zuvor
        let first = again.as_ref().map(|s| s.ahead);
        for d in first.into_iter().chain(AHEAD) {
            let pos = vec3(player.x, 0.0, player.z + dir * d);
            let yaw = npc::yaw_towards(pos, player);
            let probe = Dejavu {
                pos,
                yaw,
                aisle,
                shown: 0,
                seen_here: false,
                unseen_for: 0.0,
                customer: None,
                turned: false,
                ahead: d,
            };
            let parts = [CART_AT, CAN_AT, SIGN_AT, CUSTOMER_AT].map(|l| probe.world(l));
            let nav = &self.world.nav;
            let free = parts.iter().all(|p| {
                nav.cell_of(*p).is_some_and(|(x, z)| nav.is_walkable(x, z))
                    && self.aisle_of(*p).is_some()
            });
            let hidden = parts.iter().all(|p| {
                !self.sees(*p + vec3(0.0, 0.5, 0.0)) && !self.sees(*p + vec3(0.0, 1.5, 0.0))
            });
            if !free || !hidden || !self.world.line_of_sight(eye, pos + vec3(0.0, 1.0, 0.0)) {
                continue;
            }
            let s = match again {
                Some(old) => Dejavu {
                    pos,
                    yaw,
                    aisle,
                    seen_here: false,
                    unseen_for: 0.0,
                    turned: old.shown + 1 >= SHOWINGS,
                    ahead: d,
                    ..old
                },
                None => {
                    let customer = self.eerie_rng.gen_bool(0.75).then(|| self.eerie_person());
                    Dejavu { customer, ..probe }
                }
            };
            self.dejavu = Some(s);
            return true;
        }
        if let Some(s) = again {
            self.dejavu = Some(s);
        }
        false
    }

    /// Wagen, Dose, Warnschild der Szene.
    pub(super) fn collect_dejavu_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some(s) = &self.dejavu else { return };
        let lift = |p: Vec3| p + vec3(0.0, self.sway_at(p).y, 0.0);
        let cart = s.world(CART_AT);
        out.push(MeshDraw {
            mesh: MeshId::Cart as usize,
            model: Mat4::from_rotation_translation(Quat::from_rotation_y(s.yaw + 0.6), lift(cart)),
            tint: Vec4::new(0.85, 0.85, 0.85, 0.0),
            ..Default::default()
        });
        // Dose liegt auf der Seite
        let can = s.world(CAN_AT);
        let r = crate::assets::half_extent(MeshId::Can).x;
        out.push(MeshDraw {
            mesh: MeshId::Can as usize,
            model: Mat4::from_rotation_translation(
                Quat::from_rotation_y(s.yaw + 1.1) * Quat::from_rotation_z(FRAC_PI_2),
                lift(can) + vec3(r, r, 0.0),
            ),
            tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ..Default::default()
        });
        out.push(MeshDraw {
            mesh: MeshId::WetFloorSign as usize,
            model: Mat4::from_rotation_translation(
                Quat::from_rotation_y(s.yaw - 0.3),
                lift(s.world(SIGN_AT)),
            ),
            tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ..Default::default()
        });
    }

    /// Die reglose Kundin der Szene: starrt seitlich ins Regal, beim dritten Mal zu einem.
    pub(super) fn collect_dejavu_people(&self, out: &mut Vec<PersonDraw>) {
        let Some(s) = &self.dejavu else { return };
        let Some(v) = s.customer else { return };
        let pos = s.world(CUSTOMER_AT);
        let yaw = if s.turned {
            npc::yaw_towards(pos, self.player.pos)
        } else {
            s.yaw + FRAC_PI_2
        };
        out.push(PersonDraw {
            variant: v,
            clip: Clip::Idle,
            time: 0.0,
            pos: pos + vec3(0.0, self.sway_at(pos).y, 0.0),
            yaw,
            tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ghost: false,
            flip: false,
            look_at: None,
            fx: Default::default(),
        });
    }
}
