//! Voellig nutzlos, aber schoen: eine Zigarette (3D-Modell vor der Kamera).
//! 1 zuendet sie an, 2 fuehrt sie zum Mund und nimmt einen Zug (sie wird jedes Mal kuerzer),
//! 3 schnippt sie weg. Nach dem 4. Zug fliegt sie von allein weg. Die Kippe glimmt dann noch
//! eine Weile am Boden.
//! Waehrend eines Gespraechs, einer Frage der Kassiererin oder beim Bezahlen gehoeren die
//! Zifferntasten den Antworten bzw. der PIN, dann passiert hier nichts.

use super::*;

/// So lange dauert ein Zug (Hand zum Mund und zurueck)
pub const PUFF_TIME: f32 = 1.2;
/// So viel Zigarette kostet ein Zug
pub const PUFF_BURN: f32 = 0.2;
/// Nach so vielen Zuegen wird sie automatisch weggeschnippt
pub const MAX_PUFFS: u8 = 4;
/// So schnell brennt sie von allein ab (pro Sekunde)
const IDLE_BURN: f32 = 0.004;
/// Flugzeit beim Wegschnippen
pub const FLICK_TIME: f32 = 0.5;
/// Wie lange die Kippe am Boden liegt (glimmt die ersten Sekunden)
pub const BUTT_LIFE: f32 = 40.0;
pub const BUTT_GLOW: f32 = 12.0;

/// Laenge des Modells (MeshId::Cigarette): Filter bei -z, Glut bei +z, Boden bei y = 0
const CIG_LEN: f32 = 0.085;
const CIG_HALF_Y: f32 = 0.0042;
/// Wie lang sie bei `len` noch ist (das Modell wird von der Glutseite her gekuerzt)
fn shown_len(len: f32) -> f32 {
    0.3 + 0.7 * len.clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cigarette {
    /// keine Zigarette in der Hand
    None,
    /// brennt; `len` = wie viel noch uebrig ist (1 = ganz), `puffs` = schon genommene Zuege
    Lit { len: f32, puffs: u8 },
    /// gerade ein Zug (`t` seit Beginn)
    Puff { len: f32, puffs: u8, t: f32 },
    /// fliegt gerade weg
    Flick { len: f32, t: f32 },
}

/// Eine weggeschnippte Kippe am Boden.
#[derive(Clone, Copy, Debug)]
pub struct Butt {
    pub pos: Vec3,
    pub yaw: f32,
    pub len: f32,
    pub age: f32,
}

impl Game {
    /// Sind die Zifferntasten gerade fuer etwas anderes da (Antwort, PIN)?
    pub(super) fn digits_busy(&self) -> bool {
        self.dialog.is_some()
            || self.till_paying()
            || self.till.as_ref().is_some_and(|t| t.asked.is_some())
    }

    /// 1/2/3 und das Abbrennen.
    pub(super) fn update_cigarette(&mut self, dt: f32, key: Option<u8>) {
        for b in &mut self.butts {
            b.age += dt;
        }
        self.butts.retain(|b| b.age < BUTT_LIFE);
        self.exhale = (self.exhale - dt).max(0.0);

        match (self.cig, key) {
            (Cigarette::None, Some(1)) => {
                self.cig = Cigarette::Lit { len: 1.0, puffs: 0 };
                self.play(Sfx::Lighter, 0.6);
            }
            (Cigarette::Lit { len, puffs }, Some(2)) => {
                self.cig = Cigarette::Puff { len, puffs, t: 0.0 };
                self.play(Sfx::Puff, 0.4);
            }
            (Cigarette::Lit { len, .. }, Some(3)) => self.flick(len),
            (Cigarette::None, Some(2 | 3)) => self.hint("Erst anzünden: [1]"),
            _ => {}
        }

        match self.cig {
            Cigarette::None => {}
            Cigarette::Lit { len, puffs } => {
                let len = len - IDLE_BURN * dt;
                if len <= 0.05 {
                    self.say("Die Zigarette ist runtergebrannt.", 2.5);
                    self.cig = Cigarette::None;
                } else {
                    self.cig = Cigarette::Lit { len, puffs };
                }
            }
            Cigarette::Puff { len, puffs, t } => {
                let t = t + dt;
                if t < PUFF_TIME {
                    self.cig = Cigarette::Puff { len, puffs, t };
                    return;
                }
                // ausatmen; der Zug hat sie kuerzer gemacht
                self.exhale = 2.2;
                self.play(Sfx::Exhale, 0.45);
                let len = (len - PUFF_BURN).max(0.06);
                let puffs = puffs + 1;
                if puffs >= MAX_PUFFS {
                    self.say("Aufgeraucht. Weg damit.", 2.0);
                    self.flick(len);
                } else {
                    self.cig = Cigarette::Lit { len, puffs };
                }
            }
            Cigarette::Flick { len, t } => {
                let t = t + dt;
                if t < FLICK_TIME {
                    self.cig = Cigarette::Flick { len, t };
                    return;
                }
                // landet gut zwei Meter vor einem auf dem Boden
                let f = self.view_dir();
                let flat = vec3(f.x, 0.0, f.z).normalize_or_zero();
                let pos = self.world.collide(self.player.pos + flat * 2.2, 0.05);
                let yaw = self.player.yaw + 0.9 + self.butts.len() as f32 * 1.7;
                self.butts.push(Butt {
                    pos,
                    yaw,
                    len,
                    age: 0.0,
                });
                self.cig = Cigarette::None;
            }
        }
    }

    fn flick(&mut self, len: f32) {
        self.cig = Cigarette::Flick { len, t: 0.0 };
        self.play(Sfx::Click, 0.3);
    }

    /// Kamera-Achsen: vorwaerts, rechts, oben.
    fn view_axes(&self) -> (Vec3, Vec3, Vec3) {
        let f = self.view_dir();
        let r = f.cross(Vec3::Y).normalize_or_zero();
        let u = r.cross(f);
        (f, r, u)
    }

    /// Lage der Zigarette in der Welt: Filterende, Richtung zur Glut, Laenge (0..1).
    fn cigarette_pose(&self) -> Option<(Vec3, Vec3, f32)> {
        let (f, r, u) = self.view_axes();
        let eye = self.eye();
        // Ruhelage: unten rechts, Glut zeigt schraeg nach vorn links
        let rest = eye + f * 0.26 + r * 0.15 - u * 0.13;
        let rest_dir = (f - r * 0.45 + u * 0.12).normalize();
        // am Mund: Filter an den Lippen, Glut nach vorn und etwas nach unten
        let mouth = eye + f * 0.045 - u * 0.055;
        let mouth_dir = (f - u * 0.2).normalize();
        Some(match self.cig {
            Cigarette::None => return None,
            Cigarette::Lit { len, .. } => (rest, rest_dir, len),
            Cigarette::Puff { len, t, .. } => {
                // hin, kurz halten, zurueck
                let k = ((t / PUFF_TIME * std::f32::consts::PI).sin() * 1.25).min(1.0);
                (
                    rest.lerp(mouth, k),
                    rest_dir.lerp(mouth_dir, k).normalize(),
                    len,
                )
            }
            Cigarette::Flick { len, t } => {
                let k = (t / FLICK_TIME).clamp(0.0, 1.0);
                let pos = rest + f * (1.9 * k) + r * (0.25 * k) + u * (0.5 * k - 1.6 * k * k);
                // ueberschlaegt sich im Flug
                let spin = Quat::from_axis_angle(r, k * 11.0);
                (pos, (spin * rest_dir).normalize(), len)
            }
        })
    }

    /// Wo die Glut gerade ist (fuer den Rauch im HUD).
    pub fn cigarette_ember(&self) -> Option<Vec3> {
        let (filter, dir, len) = self.cigarette_pose()?;
        Some(filter + dir * CIG_LEN * shown_len(len))
    }

    /// Zigarette vor der Kamera und Kippen am Boden (3D-Modell, Glut leuchtet).
    pub(super) fn collect_cigarette_draws(&self, out: &mut Vec<MeshDraw>) {
        if let Some((filter, dir, len)) = self.cigarette_pose() {
            let s = shown_len(len);
            let z = dir;
            let x = Vec3::Y.cross(z).normalize_or_zero();
            let x = if x == Vec3::ZERO { Vec3::X } else { x };
            let y = z.cross(x);
            let basis = Mat4::from_cols(
                x.extend(0.0),
                y.extend(0.0),
                z.extend(0.0),
                filter.extend(1.0),
            );
            // Filterende (-z) an den Ursprung, von der Glutseite her kuerzen
            let local = Mat4::from_scale(vec3(1.0, 1.0, s))
                * Mat4::from_translation(vec3(0.0, -CIG_HALF_Y, CIG_LEN * 0.5));
            out.push(MeshDraw {
                mesh: MeshId::Cigarette as usize,
                model: basis * local,
                tint: Vec4::new(1.0, 1.0, 1.0, 0.08),
            });
            // Glut: kleines gluehendes Stueck an der Spitze, beim Zug heller
            let pulling = matches!(self.cig, Cigarette::Puff { .. });
            let flick = 0.75 + 0.25 * (self.time * 13.0).sin();
            let glow = if pulling { 2.2 } else { 1.2 * flick };
            let tip = filter + z * (CIG_LEN * s);
            out.push(MeshDraw {
                mesh: MeshId::Cube as usize,
                model: Mat4::from_cols(
                    (x * 0.0075).extend(0.0),
                    (y * 0.0075).extend(0.0),
                    (z * 0.004).extend(0.0),
                    tip.extend(1.0),
                ),
                tint: Vec4::new(1.0, 0.35, 0.06, glow),
            });
        }
        // Kippen am Boden
        for (i, b) in self.butts.iter().enumerate() {
            let glow = ((BUTT_GLOW - b.age) / BUTT_GLOW).clamp(0.0, 1.0);
            let rot = Quat::from_rotation_y(b.yaw);
            let p = b.pos + vec3(0.0, self.sway_at(b.pos).y, 0.0);
            let s = shown_len(b.len);
            let model = Mat4::from_rotation_translation(rot, p)
                * Mat4::from_scale(vec3(1.0, 1.0, s))
                * Mat4::from_translation(vec3(0.0, 0.0, CIG_LEN * 0.5));
            out.push(MeshDraw {
                mesh: MeshId::Cigarette as usize,
                model,
                tint: Vec4::new(0.85, 0.83, 0.8, 0.0),
            });
            let tip = p + rot * vec3(0.0, CIG_HALF_Y, CIG_LEN * s);
            out.push(MeshDraw {
                mesh: MeshId::Cube as usize,
                model: Mat4::from_scale_rotation_translation(Vec3::splat(0.007), rot, tip),
                tint: Vec4::new(
                    0.25 + 0.75 * glow,
                    0.2 + 0.2 * glow,
                    0.15,
                    1.5 * glow * (0.7 + 0.3 * (self.time * 9.0 + i as f32).sin()),
                ),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(g: &mut Game, d: u8) {
        let mut inp = Input {
            digit: Some(d),
            choice: (1..=3).contains(&d).then(|| d as usize - 1),
            ..Default::default()
        };
        g.update(1.0 / 60.0, &mut inp);
    }

    fn wait(g: &mut Game, secs: f32) {
        let mut inp = Input::default();
        for _ in 0..(secs * 60.0) as usize {
            g.update(1.0 / 60.0, &mut inp);
        }
    }

    fn cig_len(g: &Game) -> f32 {
        match g.cig {
            Cigarette::Lit { len, .. } | Cigarette::Puff { len, .. } => len,
            _ => 0.0,
        }
    }

    #[test]
    fn light_smoke_and_flick_a_cigarette() {
        let mut g = Game::new(70);
        assert_eq!(g.cig, Cigarette::None);
        press(&mut g, 2); // ohne Zigarette passiert nichts
        assert_eq!(g.cig, Cigarette::None);
        press(&mut g, 1);
        assert!(matches!(g.cig, Cigarette::Lit { .. }), "nicht angezuendet");
        let full = cig_len(&g);
        press(&mut g, 2);
        assert!(matches!(g.cig, Cigarette::Puff { .. }), "kein Zug");
        wait(&mut g, PUFF_TIME + 0.1);
        assert!(matches!(g.cig, Cigarette::Lit { puffs: 1, .. }));
        assert!(
            cig_len(&g) < full - PUFF_BURN * 0.9,
            "Zug macht sie nicht kuerzer"
        );
        assert!(g.exhale > 0.0, "kein Rauch ausgeatmet");
        press(&mut g, 3);
        wait(&mut g, FLICK_TIME + 0.1);
        assert_eq!(g.cig, Cigarette::None);
        assert_eq!(g.butts.len(), 1, "keine Kippe am Boden");
        assert!((g.butts[0].pos - g.player.pos).length() < 3.0);
        wait(&mut g, BUTT_LIFE + 0.5);
        assert!(g.butts.is_empty());
    }

    #[test]
    fn after_four_puffs_it_is_thrown_away_by_itself() {
        let mut g = Game::new(71);
        press(&mut g, 1);
        let mut last = cig_len(&g);
        for k in 1..MAX_PUFFS {
            press(&mut g, 2);
            wait(&mut g, PUFF_TIME + 0.1);
            assert!(
                matches!(g.cig, Cigarette::Lit { .. }),
                "nach {k} Zuegen schon weg"
            );
            assert!(cig_len(&g) < last, "Zug {k} macht sie nicht kuerzer");
            last = cig_len(&g);
        }
        press(&mut g, 2);
        wait(&mut g, PUFF_TIME + 0.1);
        assert!(
            matches!(g.cig, Cigarette::Flick { .. }),
            "nach 4 Zuegen nicht weggeworfen"
        );
        wait(&mut g, FLICK_TIME + 0.1);
        assert_eq!(g.cig, Cigarette::None);
        assert_eq!(g.butts.len(), 1);
    }

    #[test]
    fn puff_brings_the_cigarette_to_the_mouth() {
        let mut g = Game::new(73);
        press(&mut g, 1);
        let eye = g.eye();
        let rest = g.cigarette_pose().unwrap().0;
        press(&mut g, 2);
        wait(&mut g, PUFF_TIME * 0.5);
        let at_mouth = g.cigarette_pose().unwrap().0;
        assert!(
            (at_mouth - eye).length() < 0.09,
            "nicht am Mund: {}",
            (at_mouth - eye).length()
        );
        assert!((rest - eye).length() > 0.2);
        // 3D-Modell wird gezeichnet
        let mut draws = Vec::new();
        g.collect_cigarette_draws(&mut draws);
        assert!(draws.iter().any(|d| d.mesh == MeshId::Cigarette as usize));
    }

    #[test]
    fn answer_keys_do_not_light_a_cigarette() {
        // beim Bezahlen ist 1 eine PIN-Ziffer, keine Zigarette
        let mut g = Game::new(72);
        g.debug_jump_to_pay(false);
        assert!(g.digits_busy(), "beim Bezahlen sind die Ziffern frei");
        press(&mut g, 1);
        assert_eq!(g.cig, Cigarette::None);
    }
}
