//! Abgelaufener Tomatensaft: Wer ihn nimmt, fuehrt die Dose automatisch zum Mund und trinkt.
//! Danach 30 s Rausch wie auf LSD - der Nebel schimmert in Regenbogenfarben, Boden und Gaenge
//! wogen, das Bild schwankt und atmet, Figuren schillern, leuchtende Wuerfel schweben um einen
//! herum. Am Ende muss man sich uebergeben (laut, eine Pfuetze bleibt), dann ist alles normal.
//!
//! Alles nur mit dem, was das Spiel der Engine ohnehin uebergibt (Nebelfarbe, Wellen der Gaenge,
//! Kamera, Bildstoerung, Farben der Figuren, Boxen) - die Engine bleibt unveraendert.

use super::*;

/// Trinken: Dose zum Mund, ein paar Schlucke
pub const DRINK_TIME: f32 = 1.8;
/// So lange haelt der Rausch, so lange kommt und geht er
pub const TRIP_TIME: f32 = 30.0;
pub const TRIP_RAMP: f32 = 4.0;
/// Kotzen: so lange dauert es, nach so langer Zeit liegt die Pfuetze da
pub const VOMIT_TIME: f32 = 2.6;
/// Strahl: ab wann, bis wann, wie oft ein Brocken, wie lange einer fliegt (s), wie schnell (m/s)
const STREAM_START: f32 = 0.8;
const STREAM_END: f32 = 2.0;
const STREAM_STEP: f32 = 0.012;
const STREAM_FLIGHT: f32 = 0.6;
const STREAM_SPEED: f32 = 2.4;
/// Erster Brocken trifft den Boden: Pfuetze entsteht
const VOMIT_SPLASH: f32 = 1.3;
/// Farbe der Kotze: giftgruen
const VOMIT_GREEN: Vec3 = Vec3::new(0.32, 0.62, 0.08);
/// So lange bleibt die Pfuetze liegen
pub const PUDDLE_LIFE: f32 = 120.0;
/// Kotzen ist laut: so weit hoeren es die NPCs
const VOMIT_NOISE: f32 = 12.0;
/// Schwebende Wuerfel im Rausch
const BLOBS: usize = 45;

/// Wo man gerade steht (Rausch-Ablauf).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Trip {
    #[default]
    None,
    /// Dose am Mund (`t` seit Beginn)
    Drinking { t: f32 },
    /// im Rausch (`t` seit Beginn)
    Tripping { t: f32 },
    /// uebergibt sich (`t` seit Beginn)
    Vomiting { t: f32 },
}

/// Eine Pfuetze am Boden.
#[derive(Clone, Copy, Debug)]
pub struct Puddle {
    pub pos: Vec3,
    pub yaw: f32,
    pub age: f32,
}

/// Regenbogenfarbe zu einem Farbton (0..1, wiederholt sich).
pub fn rainbow(h: f32) -> Vec3 {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = |o: f32| (((h + o) % 6.0 - 3.0).abs() - 1.0).clamp(0.0, 1.0);
    vec3(c(0.0), c(4.0), c(2.0))
}

impl Game {
    /// Tomatensaft genommen: Dose zum Mund (nur wenn gerade nichts laeuft).
    pub(super) fn drink_tomato(&mut self) {
        if self.trip != Trip::None {
            // schon im Rausch: der neue Saft verlaengert ihn (von vorn, ohne neues Trinken)
            if let Trip::Tripping { t } = &mut self.trip {
                *t = t.min(TRIP_RAMP);
            }
            return;
        }
        self.trip = Trip::Drinking { t: 0.0 };
    }

    pub(super) fn update_trip(&mut self, dt: f32) {
        for p in &mut self.puddles {
            p.age += dt;
        }
        self.puddles.retain(|p| p.age < PUDDLE_LIFE);
        match self.trip {
            Trip::None => {}
            Trip::Drinking { t } => {
                let now = t + dt;
                // Schlucke
                for at in [0.55, 0.9, 1.25] {
                    if t < at && now >= at {
                        self.play(Sfx::Gulp, 0.8);
                    }
                }
                if now >= DRINK_TIME {
                    self.trip = Trip::Tripping { t: 0.0 };
                    self.say("Der schmeckt ... nach Farben.", 3.0);
                } else {
                    self.trip = Trip::Drinking { t: now };
                }
            }
            Trip::Tripping { t } => {
                let now = t + dt;
                // ab und zu leises Fluestern und Rauschen
                if (t * 0.4).floor() != (now * 0.4).floor() {
                    self.play(Sfx::Whisper, 0.25);
                }
                if now >= TRIP_TIME {
                    self.trip = Trip::Vomiting { t: 0.0 };
                    self.play(Sfx::Vomit, 1.0);
                    self.say("Dir wird schlecht ...", 2.0);
                } else {
                    self.trip = Trip::Tripping { t: now };
                }
            }
            Trip::Vomiting { t } => {
                let now = t + dt;
                if t < VOMIT_SPLASH && now >= VOMIT_SPLASH {
                    self.splash();
                }
                if now >= VOMIT_TIME {
                    self.trip = Trip::None;
                    self.say("Besser. Alles wieder normal.", 2.5);
                } else {
                    self.trip = Trip::Vomiting { t: now };
                }
            }
        }
    }

    /// Kotze trifft den Boden: Pfuetze vor einem, laut, Bild ruckt.
    fn splash(&mut self) {
        // dort, wo der Strahl landet (Bogen aus vomit_mouth, ohne Streuung)
        let (mouth, v0) = self.vomit_mouth();
        let drop = mouth.y - self.player.pos.y;
        let fall = (v0.y + (v0.y * v0.y + 2.0 * 9.8 * drop.max(0.1)).sqrt()) / 9.8;
        let land = vec3(mouth.x + v0.x * fall, 0.0, mouth.z + v0.z * fall);
        let at = self.world.collide(land, 0.2);
        self.puddles.push(Puddle {
            pos: vec3(at.x, 0.0, at.z),
            yaw: self.rng.gen_range(0.0..std::f32::consts::TAU),
            age: 0.0,
        });
        self.play(Sfx::Thud, 0.4);
        self.make_noise(VOMIT_NOISE);
        self.glitch = self.glitch.max(0.6);
    }

    /// Wie stark der Rausch gerade ist (0..1): kommt in TRIP_RAMP, geht in TRIP_RAMP.
    pub fn trip_level(&self) -> f32 {
        let smooth = |k: f32| {
            let k = k.clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        match self.trip {
            Trip::Tripping { t } => smooth(t / TRIP_RAMP).min(smooth((TRIP_TIME - t) / TRIP_RAMP)),
            _ => 0.0,
        }
    }

    /// Nebelfarbe im Rausch: wechselnde Regenbogenfarben ueber der normalen.
    pub fn trip_fog(&self, base: Vec3) -> Vec3 {
        let l = self.trip_level();
        if l <= 0.0 {
            return base;
        }
        // schnell wechselnde, voll gesaettigte Farben, hell pulsierend
        let c = rainbow(self.time * 0.35 + 0.1 * (self.time * 2.1).sin())
            * (0.75 + 0.35 * (self.time * 3.3).sin());
        base.lerp(c, l)
    }

    /// Wellen der Gaenge im Rausch (zusaetzlich, auch im normalen Markt).
    pub(super) fn trip_sway(&self, mut w: Vec4) -> Vec4 {
        let l = self.trip_level();
        if l <= 0.0 {
            return w;
        }
        if w.z == 0.0 && w.w == 0.0 {
            // normaler Markt: Bereich der Gaenge setzen (sonst ist dort alles ruhig)
            w.z = self.world.aisle_z1;
            w.w = self.world.aisle_x1;
        }
        w.x += 0.5 * l;
        w.y += 0.7 * l;
        w
    }

    /// Sichtfeld atmet (Faktor auf das Sichtfeld).
    pub fn trip_fov(&self) -> f32 {
        let t = self.time;
        1.0 + self.trip_level() * (0.22 * (t * 1.1).sin() + 0.06 * (t * 4.3).sin())
    }

    /// Bild schwankt: Neigung um die Blickachse (Bogenmass); beim Kotzen ein Ruck.
    pub fn trip_roll(&self) -> f32 {
        let l = self.trip_level();
        let vomit = match self.trip {
            Trip::Vomiting { t } => (t * 9.0).sin() * 0.05 * (1.0 - t / VOMIT_TIME),
            _ => 0.0,
        };
        l * (0.15 * (self.time * 0.6).sin() + 0.04 * (self.time * 2.2).sin()) + vomit
    }

    /// Beim Kotzen kippt der Blick nach unten (Zusatz zum Neigungswinkel, Bogenmass).
    pub fn vomit_pitch(&self) -> f32 {
        match self.trip {
            Trip::Vomiting { t } => {
                let k = (t / VOMIT_TIME).clamp(0.0, 1.0);
                -0.7 * (k * std::f32::consts::PI).sin().powf(0.6)
            }
            _ => 0.0,
        }
    }

    /// Figuren schillern im Rausch: Farbe zur Regenbogenfarbe ziehen.
    pub(super) fn trip_tint(&self, tint: Vec4, seed: f32) -> Vec4 {
        let l = self.trip_level();
        if l <= 0.0 {
            return tint;
        }
        let c = rainbow(self.time * 0.6 + seed * 0.37);
        tint.truncate()
            .lerp(c * 1.5, l * 0.95)
            .extend(tint.w.max(0.6 * l))
    }

    /// Dose beim Trinken: (Boden, Achse nach oben, rechts). Hoch zum Mund, kippt dabei.
    pub(super) fn drink_can(&self) -> Option<(Vec3, Vec3, Vec3)> {
        let Trip::Drinking { t } = self.trip else {
            return None;
        };
        let f = self.view_dir();
        let r = f.cross(Vec3::Y).normalize_or_zero();
        let u = r.cross(f);
        let eye = self.eye();
        let h = crate::assets::half_extent(MeshId::Can).y * 2.0;
        let raise = (t / 0.35).clamp(0.0, 1.0);
        let lower = ((t - (DRINK_TIME - 0.3)) / 0.3).clamp(0.0, 1.0);
        let k = raise * (1.0 - lower);
        // Neigung: von aufrecht ueber schraeg bis fast waagerecht (letzte Schlucke)
        let tilt = k * (0.9 + 0.8 * ((t - 0.35) / (DRINK_TIME - 0.65)).clamp(0.0, 1.0));
        let up = (u * tilt.cos() - f * tilt.sin()).normalize();
        let rest = eye + f * 0.32 + r * 0.14 - u * 0.3;
        let mouth = eye + f * 0.07 - u * 0.03;
        let top = rest.lerp(mouth, k);
        Some((top - up * h, up, r))
    }

    /// Dose am Mund, schwebende Wuerfel im Rausch, Pfuetzen am Boden.
    pub(super) fn collect_trip_draws(&self, out: &mut Vec<MeshDraw>) {
        let cube = MeshId::Cube as usize;
        // Dose: hoch zum Mund, kippt beim Trinken
        if let Some((base, up, r)) = self.drink_can() {
            let z = r.cross(up).normalize_or_zero();
            out.push(MeshDraw {
                mesh: MeshId::Can as usize,
                model: Mat4::from_cols(
                    r.extend(0.0),
                    up.extend(0.0),
                    z.extend(0.0),
                    base.extend(1.0),
                ),
                tint: Vec4::new(1.0, 1.0, 1.0, 0.05),
                ..Default::default()
            });
        }
        // schwebende, leuchtende Wuerfel um einen herum (drehen sich, pulsieren)
        let l = self.trip_level();
        if l > 0.0 {
            let center = self.player.pos + vec3(0.0, 1.3, 0.0);
            for k in 0..BLOBS {
                let s = k as f32;
                // drei Ringe in verschiedenen Hoehen, abwechselnd links- und rechtsherum
                let ring = (k % 3) as f32;
                let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
                let a = dir * self.time * (0.3 + 0.04 * s) + s * 2.4;
                let rad = 1.4 + 0.9 * ring + 0.8 * (s * 1.7).sin().abs();
                let p = center
                    + vec3(
                        a.cos() * rad,
                        (ring - 1.0) * 0.9 + (self.time * 0.9 + s).sin() * 0.5,
                        a.sin() * rad,
                    );
                let size = (0.05 + 0.12 * (self.time * 2.5 + s).sin().abs()) * l;
                let rot = Quat::from_euler(glam::EulerRot::XYZ, a * 1.3, a * 0.7, s);
                out.push(MeshDraw {
                    mesh: cube,
                    model: Mat4::from_scale_rotation_translation(Vec3::splat(size), rot, p),
                    tint: rainbow(self.time * 0.5 + s / BLOBS as f32).extend(2.2 * l),
                    ..Default::default()
                });
            }
        }
        self.collect_vomit_stream(out);
        // Pfuetzen: unregelmaessig aus flachen Boxen, giftgruen, wachsen erst, verblassen am Ende
        for p in &self.puddles {
            let fade = (1.0 - (p.age - (PUDDLE_LIFE - 10.0)) / 10.0).clamp(0.0, 1.0)
                * (0.25 + p.age / (STREAM_END - VOMIT_SPLASH)).min(1.0);
            let base = p.pos + vec3(0.0, 0.004 + self.sway_at(p.pos).y, 0.0);
            for (k, (dx, dz, sx, sz)) in [
                (0.0f32, 0.0f32, 0.55f32, 0.4f32),
                (0.18, 0.1, 0.3, 0.25),
                (-0.15, -0.12, 0.28, 0.2),
                (0.05, -0.2, 0.18, 0.15),
            ]
            .iter()
            .enumerate()
            {
                let rot = Quat::from_rotation_y(p.yaw + k as f32 * 0.7);
                out.push(MeshDraw {
                    mesh: cube,
                    model: Mat4::from_scale_rotation_translation(
                        vec3(*sx, 0.006 + k as f32 * 0.001, *sz) * (0.4 + 0.6 * fade.max(0.3)),
                        rot,
                        base + Quat::from_rotation_y(p.yaw) * vec3(*dx, 0.0, *dz),
                    ),
                    tint: VOMIT_GREEN.extend(0.08),
                    ..Default::default()
                });
            }
        }
    }

    /// Wo der Strahl den Mund verlaesst und mit welcher Geschwindigkeit (m, m/s).
    fn vomit_mouth(&self) -> (Vec3, Vec3) {
        let (s, c) = self.player.yaw.sin_cos();
        let fwd = vec3(s, 0.0, -c);
        let mouth = self.eye() + fwd * 0.18 - Vec3::Y * 0.12;
        (mouth, fwd * STREAM_SPEED - Vec3::Y * 0.4)
    }

    /// Kotz-Strahl: gruene Brocken schiessen aus dem Mund im Bogen auf den Boden.
    /// Jeder Brocken ist durch seine Startzeit festgelegt (kein Zustand noetig).
    fn collect_vomit_stream(&self, out: &mut Vec<MeshDraw>) {
        let Trip::Vomiting { t } = self.trip else {
            return;
        };
        let (mouth, v0) = self.vomit_mouth();
        let (s, c) = self.player.yaw.sin_cos();
        let side = vec3(c, 0.0, s);
        let n = ((STREAM_END - STREAM_START) / STREAM_STEP) as usize;
        for k in 0..n {
            let born = STREAM_START + k as f32 * STREAM_STEP;
            let age = t - born;
            if !(0.0..STREAM_FLIGHT).contains(&age) {
                continue;
            }
            // jeder Brocken streut etwas anders (fester Zufall aus k)
            let h = |m: f32| ((k as f32 * m).sin() * 43758.545).fract() - 0.5;
            let v = v0 * (0.85 + 0.3 * h(12.9)) + side * h(78.2) * 0.9 + Vec3::Y * h(37.7) * 0.8;
            let mut p = mouth + v * age - Vec3::Y * 0.5 * 9.8 * age * age;
            let floor = self.sway_at(p).y + 0.02;
            if p.y < floor {
                p.y = floor; // am Boden zerplatzt: flach liegen lassen
            }
            let size = 0.035 + 0.035 * (h(5.3) + 0.5);
            let rot = Quat::from_euler(glam::EulerRot::XYZ, age * 9.0 + k as f32, age * 6.0, 0.0);
            let shade = 0.8 + 0.4 * (h(91.1) + 0.5);
            out.push(MeshDraw {
                mesh: MeshId::Cube as usize,
                model: Mat4::from_scale_rotation_translation(Vec3::splat(size), rot, p),
                tint: (VOMIT_GREEN * shade).extend(0.15),
                ..Default::default()
            });
        }
    }

    /// Bildstoerung im Rausch: Farben laufen auseinander, Bild flimmert.
    pub fn trip_glitch(&self) -> f32 {
        let l = self.trip_level();
        l * (0.22 + 0.18 * (self.time * 2.3).sin().max(0.0))
    }

    /// Im Rausch blitzt das Bild im Takt kurz ins Negativ.
    pub fn trip_invert(&self) -> f32 {
        let l = self.trip_level();
        if l < 0.5 {
            return 0.0;
        }
        let beat = (self.time / 2.7).fract();
        if beat < 0.06 {
            l
        } else {
            0.0
        }
    }

    /// Ueberbelichtung im Rausch: das Bild pulsiert hell.
    pub fn trip_flash(&self) -> f32 {
        let l = self.trip_level();
        l * 0.18 * (self.time * 3.1).sin().max(0.0).powi(3)
    }

    /// Dichterer Nebel im Rausch, damit die Farben das Bild fuellen.
    pub fn trip_fog_density(&self, base: f32) -> f32 {
        base * (1.0 + 1.2 * self.trip_level())
    }

    /// Alle beweglichen Dinge (Wagen, Waren in der Hand, Kassen ...) schillern im Rausch.
    pub fn trip_colorize(&self, draws: &mut [MeshDraw]) {
        if self.trip_level() <= 0.0 {
            return;
        }
        for (k, d) in draws.iter_mut().enumerate() {
            d.tint = self.trip_tint(d.tint, k as f32 * 0.31);
        }
    }
}
