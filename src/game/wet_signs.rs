//! Warnschilder "Rutschgefahr" stehen nicht nur herum: Wer dagegen laeuft (oder den Wagen
//! dagegen schiebt), wirft sie um. Das scheppert, NPCs in der Naehe hoeren es, und im
//! gekippten Markt steigt die Auffaelligkeit. Umgefallen bleiben sie liegen (bis der Markt
//! sich umbaut) und sind kein Hindernis mehr.

use super::*;

/// Radius eines stehenden Schilds (Meter, Grundflaeche 0,3 x 0,35 m)
pub const SIGN_RADIUS: f32 = 0.2;
/// So lange dauert das Umfallen (s)
pub const FALL_TIME: f32 = 0.35;
/// Auffaelligkeit je umgeworfenem Schild (wie Anrempeln: 0,15)
pub const FALL_SUSPICION: f32 = 0.12;
/// So weit hoeren NPCs das Scheppern (Meter)
const FALL_NOISE: f32 = 10.0;
/// halbe Dicke des Schilds: so hoch liegt es umgefallen (Meter)
const LYING_LIFT: f32 = 0.177;

/// Ein umfallendes oder umgefallenes Schild: Richtung (waagrecht) und Zeit seit dem Stoss.
#[derive(Clone, Copy, Debug)]
pub struct SignFall {
    pub dir: Vec3,
    pub t: f32,
}

impl Game {
    /// Schilder anstossen: Spieler und geschobener Wagen werfen stehende Schilder um.
    pub(super) fn update_wet_signs(&mut self, dt: f32) {
        // neuer Markt (Umbau): Schilder stehen wieder
        if self.sign_falls.len() != self.world.signs.len() {
            self.sign_falls = vec![None; self.world.signs.len()];
        }
        for f in self.sign_falls.iter_mut().flatten() {
            f.t += dt;
        }
        let player = self.player.pos;
        let mut bodies = vec![(player, PLAYER_RADIUS)];
        if self.trolley.pushing {
            bodies.push((self.trolley.pos, hands::CART_PUSH_RADIUS));
        }
        let mut knocked = false;
        for k in 0..self.world.signs.len() {
            if self.sign_falls[k].is_some() || self.sign_lying_anyway(k) {
                continue;
            }
            let pos = self.world.signs[k].0;
            for &(b, r) in &bodies {
                let d = vec3(pos.x - b.x, 0.0, pos.z - b.z);
                if d.length() < SIGN_RADIUS + r {
                    self.sign_falls[k] = Some(SignFall {
                        dir: d.normalize_or(vec3(0.0, 0.0, 1.0)),
                        t: 0.0,
                    });
                    knocked = true;
                    break;
                }
            }
        }
        if knocked {
            self.play(Sfx::Thud, 0.45);
            self.play(Sfx::Rattle, 0.5);
            self.make_noise(FALL_NOISE);
            if self.tipped {
                self.suspicion = (self.suspicion + FALL_SUSPICION).min(1.0);
            }
            self.hint("Das Schild ist umgefallen. Das war laut.");
        }
    }

    /// Liegt Schild `k` im Kollaps ohnehin am Boden (jedes zweite, siehe draws.rs)?
    pub(super) fn sign_lying_anyway(&self, k: usize) -> bool {
        self.wet_sign_level() >= 4 && k % 2 == 1
    }

    /// Lage eines umfallenden Schilds: kippt in Fallrichtung um die Grundflaeche, liegt
    /// am Ende flach (None = steht noch).
    pub(super) fn sign_fall_model(&self, k: usize, pos: Vec3, yaw: f32) -> Option<Mat4> {
        let f = self.sign_falls.get(k).copied().flatten()?;
        let s = (f.t / FALL_TIME).clamp(0.0, 1.0);
        let angle = s * s * std::f32::consts::FRAC_PI_2;
        let axis = Vec3::Y.cross(f.dir).normalize_or(Vec3::X);
        let tilt = Quat::from_axis_angle(axis, angle);
        Some(Mat4::from_rotation_translation(
            tilt * Quat::from_rotation_y(yaw),
            pos + vec3(0.0, LYING_LIFT * angle.sin(), 0.0),
        ))
    }
}
