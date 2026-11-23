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
        self.collect_barrier_draws(out);
        // Tutorial-Modus: Pfeile am Boden, rote Rahmen um benoetigte Waren
        self.collect_guide_draws(out);
        // Zigarette vor der Kamera und weggeschnippte Kippen
        self.collect_cigarette_draws(out);
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
        self.collect_vermin_draws(out);
        self.collect_mirage_draws(out);
        self.collect_scare_draws(out);
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
            flip: false,
            look_at: None,
            fx: Default::default(),
        };

        // Figur mit eigenem Animator: Clip, Zeit, Ueberblenden
        let animated = |variant, a: &crate::people::Animator, seed: f32, pos, yaw, tint| {
            let mut d = person(variant, a.clip, a.clip_time(), pos, yaw, tint);
            d.fx.blend = a.blend();
            d.fx.seed = seed;
            d
        };

        // Normale Kundschaft: geht zum naechsten Regal, wartet dort
        for s in &self.shoppers {
            out.push(animated(
                s.variant,
                &s.anim,
                s.seed,
                s.pos,
                s.yaw,
                s.tint.extend(0.0),
            ));
        }

        // Uncanny Kunden: greifen immer wieder ins Regal; wenn man wegsieht, kommen sie naeher
        let eye = self.eye();
        for c in &self.customers {
            // leicht fahles Eigenleuchten: Silhouetten bleiben im Dunkeln erkennbar
            let mut d = animated(c.variant, &c.anim, c.seed, c.pos, c.yaw, c.tint.extend(0.1));
            // im gekippten Markt folgt der Kopf dir, waehrend der Koerper weitermacht
            if let Some(limit) = self.head_follow_limit(c.pos) {
                d.look_at = Some(eye);
                d.fx.look_limit = limit;
            }
            self.customer_fx(c, &mut d.fx);
            out.push(d);
        }

        // Gesichtsloses Personal
        for e in &self.employees {
            let mut d = animated(
                e.variant,
                &e.anim,
                e.seed,
                e.pos,
                e.yaw,
                // Personal leuchtet leicht: im dunklen Markt sofort vom Publikum zu unterscheiden
                Vec4::new(1.0, 1.0, 1.0, 0.22),
            );
            // Personal bleibt menschlich: schaut einen an, aber nur so weit, wie ein Mensch
            // den Kopf drehen kann (keine Verformung, kein Verdrehen)
            if self.tipped && (e.pos - self.player.pos).length() < STAFF_LOOK_DIST {
                d.look_at = Some(eye);
                d.fx.look_limit = STAFF_LOOK_LIMIT;
            }
            out.push(d);
        }

        // Kassiererin an der offenen Kasse: gesichtslose Personal-Figur. Beim Scannen greift sie
        // zum Band, im gekippten Markt dreht sie sich sonst immer zu dir. Waehrend des Schrecks
        // "sie beugt sich vor" zeichnet scares.rs sie.
        if let Some(lane) = self.open_lane().filter(|_| !self.cashier_standing()) {
            // schaut sie auf (till/pressure.rs), ruckt sie zum Spieler herum und leuchtet heller
            let looking = self.cashier_looking();
            let scanning = self.scan_timer > 0.0 && !looking;
            let yaw = if looking || (self.tipped && !scanning) {
                npc::yaw_towards(lane.seat, self.player.pos)
            } else {
                -std::f32::consts::FRAC_PI_2
            };
            let glow = if self.cashier_staring() {
                0.5 + 0.1 * (t * 17.0).sin()
            } else if self.tipped {
                0.22
            } else {
                0.0
            };
            // beim Aufschauen friert sie ein (keine Bewegung im Clip)
            let anim_t = if looking { 0.0 } else { t };
            // sie sitzt auf dem Buerostuhl: Figur so stellen, dass das Gesaess auf der
            // Sitzflaeche liegt (dreht sich mit dem Stuhl zu dir, wenn sie dich ansieht)
            let facing = vec3(yaw.sin(), 0.0, yaw.cos());
            let pos = lane.seat
                + facing * crate::people::SIT_BACK
                + vec3(0.0, crate::people::SIT_UP, 0.0);
            out.push(person(
                self.cashier_variant,
                Clip::Sit,
                anim_t,
                pos,
                yaw,
                Vec4::new(1.0, 1.0, 1.0, glow),
            ));
        }

        // Schrecken: aufgestandene Kassiererin, Gestalt im Deckenloch, Gesicht beim Tod
        self.collect_scare_people(out);

        // Fata Morgana: wirkt echt, bis man auf 3 m heran ist - dann flackert sie als Hologramm
        for (i, h) in self.holograms.iter().enumerate() {
            let Some(v) = h.person else { continue };
            // die unheimliche dreht sich erst um (noch echt), dann zerfaellt sie
            let ghost = h.revealed >= 0.0 && !(h.scary && h.revealed < super::mirage::SCARY_HOLD);
            if ghost && !self.hologram_visible(i) {
                continue;
            }
            out.push(PersonDraw {
                variant: v,
                clip: Clip::Idle,
                time: t + phase(i, 4.0),
                pos: h.pos + vec3(0.0, self.sway_at(h.pos).y, 0.0),
                yaw: h.yaw,
                tint: if ghost {
                    h.tint.extend(2.2)
                } else {
                    h.real_tint
                },
                ghost,
                flip: false,
                look_at: None,
                fx: Default::default(),
            });
        }
    }
}

impl Game {
    /// Bewegter Verfall: Ameisenstrassen, huschende Ratten, Blut, das von der Decke tropft.
    fn collect_vermin_draws(&self, out: &mut Vec<MeshDraw>) {
        let cube = MeshId::Cube as usize;
        let t = self.time;
        let w = &self.world;
        let on_floor = |p: Vec3| p + vec3(0.0, self.sway_at(p).y, 0.0);
        // Ameisen: 24 Punkte wandern die Strasse entlang, leicht wackelnd
        for (i, &(a, b)) in w.ant_trails.iter().enumerate() {
            if (a - self.player.pos).length() > 14.0 {
                continue;
            }
            let side = vec3(b.z - a.z, 0.0, a.x - b.x).normalize_or_zero();
            for k in 0..24 {
                let s = (k as f32 / 24.0 + t * 0.06 + i as f32 * 0.37).fract();
                let wobble = (s * 40.0 + t * 3.0 + k as f32).sin() * 0.012;
                let p = on_floor(a.lerp(b, s) + side * wobble) + vec3(0.0, 0.004, 0.0);
                out.push(MeshDraw {
                    mesh: cube,
                    model: Mat4::from_scale_rotation_translation(
                        vec3(0.006, 0.004, 0.009),
                        Quat::from_rotation_y((b - a).x.atan2((b - a).z)),
                        p,
                    ),
                    tint: Vec4::new(0.03, 0.02, 0.02, 0.0),
                });
            }
        }
        // Ratten: huschen hin und her, halten an den Enden kurz inne
        for (i, &(a, b)) in w.rat_runs.iter().enumerate() {
            let len = (b - a).length().max(0.1);
            let run = len / RAT_SPEED;
            let cycle = 2.0 * (run + RAT_PAUSE);
            let c = (t + i as f32 * 1.7).rem_euclid(cycle);
            let (pos, dir, moving) = if c < run {
                (a.lerp(b, c / run), b - a, true)
            } else if c < run + RAT_PAUSE {
                (b, b - a, false)
            } else if c < 2.0 * run + RAT_PAUSE {
                (b.lerp(a, (c - run - RAT_PAUSE) / run), a - b, true)
            } else {
                (a, a - b, false)
            };
            let hop = if moving {
                (t * 22.0 + i as f32).sin().abs() * 0.012
            } else {
                0.0
            };
            out.push(MeshDraw {
                mesh: MeshId::Rat as usize,
                model: Mat4::from_rotation_translation(
                    Quat::from_rotation_y(dir.x.atan2(dir.z)),
                    on_floor(pos) + vec3(0.0, hop, 0.0),
                ),
                tint: Vec4::new(0.55, 0.5, 0.48, 0.0),
            });
        }
        // Blut: Tropfen fallen von der Decke, kleiner Spritzer auf der Lache
        for (i, &d) in w.drips.iter().enumerate() {
            let period = 0.7 + crate::world::hash01(d) * 0.8;
            let c = (t + i as f32 * 0.53).rem_euclid(period);
            let fall = (2.0 * d.y / 9.81).sqrt();
            let red = Vec4::new(0.35, 0.01, 0.01, 0.05);
            if c < fall {
                let y = d.y - 0.5 * 9.81 * c * c;
                out.push(MeshDraw {
                    mesh: cube,
                    model: Mat4::from_scale_rotation_translation(
                        vec3(0.012, 0.03, 0.012),
                        Quat::IDENTITY,
                        vec3(d.x, y.max(0.02), d.z)
                            + vec3(0.0, self.sway_at(d).y * (1.0 - y / d.y), 0.0),
                    ),
                    tint: red,
                });
            } else if c < fall + 0.15 {
                // Spritzer: zwei gedrehte flache Scheiben (rund)
                let k = (c - fall) / 0.15;
                for a in [0.0f32, std::f32::consts::FRAC_PI_4] {
                    out.push(MeshDraw {
                        mesh: cube,
                        model: Mat4::from_scale_rotation_translation(
                            vec3(0.05 + k * 0.08, 0.006 * (1.0 - k), 0.05 + k * 0.08),
                            Quat::from_rotation_y(a),
                            on_floor(vec3(d.x, 0.01, d.z)),
                        ),
                        tint: red,
                    });
                }
            }
        }
        // Funken an blanken Kabelenden: kurze Schauer gluehender Teilchen
        for (i, &s) in w.sparks.iter().enumerate() {
            if (s - self.player.pos).length() > 18.0 {
                continue;
            }
            let period = spark_period(s);
            let c = (t + i as f32 * 0.77).rem_euclid(period);
            if c > 0.35 {
                continue;
            }
            for k in 0..12 {
                let h = crate::world::hash01(s + vec3(k as f32 * 1.37, (t / period).floor(), 0.0));
                let a = h * std::f32::consts::TAU;
                let v = vec3(a.cos() * (0.6 + h), 0.4 + h * 1.2, a.sin() * (0.6 + h));
                let p = s + v * c - vec3(0.0, 0.5 * 9.81 * c * c, 0.0);
                out.push(MeshDraw {
                    mesh: cube,
                    model: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.007),
                        Quat::IDENTITY,
                        p,
                    ),
                    tint: Vec4::new(1.0, 0.85, 0.45, 4.0 * (1.0 - c / 0.35)),
                });
            }
        }
    }
}

/// Ratten: Tempo (m/s) und Pause an den Enden (s)
const RAT_SPEED: f32 = 1.6;
const RAT_PAUSE: f32 = 1.3;

/// Abstand der Funkenschauer an einem Kabelende (Sekunden).
pub(super) fn spark_period(s: Vec3) -> f32 {
    1.6 + crate::world::hash01(s) * 2.6
}

impl Game {
    /// Knistern, wenn in der Naehe ein Funkenschauer beginnt.
    pub(super) fn update_spark_sounds(&mut self) {
        let n = self.world.sparks.len();
        if self.spark_bursts.len() != n {
            self.spark_bursts = vec![i64::MIN; n];
        }
        for i in 0..n {
            let s = self.world.sparks[i];
            let burst = ((self.time + i as f32 * 0.77) / spark_period(s)).floor() as i64;
            if burst != self.spark_bursts[i] {
                let first = self.spark_bursts[i] == i64::MIN;
                self.spark_bursts[i] = burst;
                let d = (s - self.player.pos).length();
                if !first && d < 14.0 {
                    self.sfx.push((Sfx::Spark, (1.0 - d / 14.0) * 0.5));
                }
            }
        }
    }
}

/// So nah schauen Kunden bzw. Mitarbeiter einem mit dem Kopf hinterher (Meter)
const CUSTOMER_LOOK_DIST: f32 = 14.0;
const STAFF_LOOK_DIST: f32 = 9.0;
/// so weit dreht das Personal den Kopf (Radiant, menschlich)
const STAFF_LOOK_LIMIT: f32 = 1.1;

impl Game {
    /// Wie weit eine Kundin an `pos` den Kopf nach dir drehen darf (None = gar nicht).
    /// Je spaeter die Stufe, desto unmenschlicher: am Ende dreht sich der Kopf ganz herum.
    pub fn head_follow_limit(&self, pos: Vec3) -> Option<f32> {
        if !self.tipped || (pos - self.player.pos).length() > CUSTOMER_LOOK_DIST {
            return None;
        }
        Some(match self.tier {
            Tier::Calm => 1.2,
            Tier::Uneasy => 1.6,
            Tier::Wrong => 2.3,
            Tier::Collapse => std::f32::consts::PI,
        })
    }
}
