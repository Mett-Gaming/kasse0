//! Dynamische Zeichenaufrufe: Kasse, Wagen, Hand (Regalware ist statisch) und die
//! animierten Figuren (Kundschaft, Personal, Hologramme).

use super::*;
use crate::people::{Clip, PersonDraw};

/// Versatz der Animation je Figur, damit nicht alle im Gleichschritt laufen.
fn phase(i: usize, salt: f32) -> f32 {
    ((i as f32 * 12.9898 + salt * 78.233).sin() * 43_758.547)
        .fract()
        .abs()
        * 10.0
}

impl Game {
    pub fn collect_draws(&self, out: &mut Vec<MeshDraw>) {
        let rot = |yaw: f32| Quat::from_rotation_y(yaw);

        self.collect_hand_draws(out);

        // Kassen: Kassierer:in, Band, Terminal, Rollo, Kamera
        self.collect_checkout_draws(out);
        // Fliegen um verdorbene Ware in der Naehe
        let mut swarms = 0;
        for &i in &self.world.rotten {
            let it = &self.world.items[i];
            if it.taken || (it.pos - self.player.pos).length() > 9.0 {
                continue;
            }
            let base = it.pos + vec3(0.0, it.half().y * 2.0 + 0.12, 0.0) + self.sway_at(it.pos);
            for k in 0..3 {
                let a = self.time * (5.0 + k as f32) + i as f32 + k as f32 * 2.1;
                let p = base
                    + vec3(
                        a.cos() * 0.12,
                        (a * 1.7).sin() * 0.06,
                        (a * 1.3).sin() * 0.12,
                    );
                out.push(MeshDraw {
                    mesh: MeshId::Cube as usize,
                    model: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.012),
                        Quat::IDENTITY,
                        p,
                    ),
                    tint: Vec4::new(0.02, 0.02, 0.02, 0.0),
                });
            }
            swarms += 1;
            if swarms >= 24 {
                break;
            }
        }
        // Warnschilder
        for &(pos, yaw) in &self.world.signs {
            out.push(MeshDraw {
                mesh: MeshId::WetFloorSign as usize,
                model: Mat4::from_rotation_translation(rot(yaw), pos),
                tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            });
        }
    }

    /// Animierte Figuren: welcher Clip, welche Zeit, wo.
    pub fn collect_people(&self, out: &mut Vec<PersonDraw>) {
        let t = self.time;
        let person = |variant, clip, time, pos, yaw, tint| PersonDraw {
            variant,
            clip,
            time,
            // auf der Hoehenwelle des Bodens
            pos: pos + vec3(0.0, self.sway_at(pos).y, 0.0),
            yaw,
            tint,
            ghost: false,
        };

        // Normale Kundschaft: geht zum naechsten Regal, wartet dort
        for (i, s) in self.shoppers.iter().enumerate() {
            let clip = if s.wait > 0.0 { Clip::Idle } else { Clip::Walk };
            out.push(person(
                s.variant,
                clip,
                t + phase(i, 1.0),
                s.pos,
                s.yaw,
                s.tint.extend(0.0),
            ));
        }

        // Uncanny Kunden: greifen immer wieder ins Regal; wenn man wegsieht, kommen sie naeher
        for (i, c) in self.customers.iter().enumerate() {
            let clip = if c.moving {
                Clip::Walk
            } else if c.pose == Pose::Reach {
                Clip::Reach
            } else {
                Clip::Idle
            };
            // leicht fahles Eigenleuchten: Silhouetten bleiben im Dunkeln erkennbar
            out.push(person(
                c.variant,
                clip,
                t + phase(i, 2.0),
                c.pos,
                c.yaw,
                c.tint.extend(0.1),
            ));
        }

        // Gesichtsloses Personal; wer einem folgt, wird schneller
        for (i, e) in self.employees.iter().enumerate() {
            let clip = match (e.moving, e.follow > 0.0 || e.carrying) {
                (false, _) => Clip::Idle,
                (true, false) => Clip::Walk,
                (true, true) => Clip::Walk,
            };
            let speedup = if e.follow > 0.0 && e.moving {
                1.35
            } else {
                1.0
            };
            out.push(person(
                e.variant,
                clip,
                (t + phase(i, 3.0)) * speedup,
                e.pos,
                e.yaw,
                // Personal leuchtet leicht: im dunklen Markt sofort vom Publikum zu unterscheiden
                Vec4::new(1.0, 1.0, 1.0, 0.22),
            ));
        }

        // Fata Morgana: halbtransparente Figuren
        for (i, h) in self.holograms.iter().enumerate() {
            let Some(v) = h.person else { continue };
            if !self.hologram_visible(i) {
                continue;
            }
            out.push(PersonDraw {
                variant: v,
                clip: Clip::Idle,
                time: t + phase(i, 4.0),
                pos: h.pos,
                yaw: h.yaw,
                tint: h.tint.extend(2.2),
                ghost: true,
            });
        }
    }
}
