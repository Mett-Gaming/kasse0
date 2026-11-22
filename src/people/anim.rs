//! Knochen-Animation mit Hierarchie (.k0a v2): lokale Transformationen je Bild, zwischen
//! Bildern und Clips per slerp gemischt, danach Eingriffe pro Knochen (Kopf dreht sich zu
//! einem Ziel, Zucken, gestreckte Glieder, Stop-Motion). Ergebnis: Skinning-Matrizen.

use glam::{Mat4, Quat, Vec3, Vec4};

/// Animation einer Figur (Reihenfolge wie CLIPS in tools/prepare_people.py)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Clip {
    #[default]
    Idle = 0,
    Walk,
    Reach,
    Run,
    /// zupacken (Ausfallschritt mit ausgestrecktem Arm)
    Grab,
    Wave,
    Hit,
    Death,
    CrouchWalk,
    CrouchIdle,
    ZombieWalk,
    ZombieIdle,
    Scratch,
    /// vom Boden aufstehen
    GetUp,
    FoldArms,
    Phone,
    Talk,
    PickUp,
    /// Wagen schieben
    Push,
    Carry,
    /// kniend ins Regal einraeumen
    Kneel,
    No,
    WalkFormal,
}

impl Clip {
    /// Laeuft die Figur mit diesem Clip vorwaerts (Zeit an die Strecke gekoppelt)?
    pub fn locomotion(self) -> bool {
        matches!(
            self,
            Clip::Walk
                | Clip::Run
                | Clip::CrouchWalk
                | Clip::ZombieWalk
                | Clip::Push
                | Clip::Carry
                | Clip::WalkFormal
        )
    }
}

/// Lokale Transformation eines Knochens relativ zu seinem Eltern-Knochen.
#[derive(Clone, Copy, Debug)]
pub struct Local {
    pub rot: Quat,
    pub pos: Vec3,
    pub scale: Vec3,
}

impl Local {
    fn mix(self, o: Local, w: f32) -> Local {
        Local {
            rot: self.rot.slerp(o.rot, w),
            pos: self.pos.lerp(o.pos, w),
            scale: self.scale.lerp(o.scale, w),
        }
    }
    fn mat(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }
}

struct ClipData {
    fps: f32,
    frames: usize,
    looped: bool,
    /// frames * bones
    locals: Vec<Local>,
}

/// Eingriffe in die Pose (alles optional, Standard = nichts).
#[derive(Clone, Copy, Debug, Default)]
pub struct PoseFx {
    /// Ueberblenden: vorheriger Clip, dessen Zeit, Gewicht (1 = nur der alte, 0 = nur der neue)
    pub blend: Option<(Clip, f32, f32)>,
    /// Blickziel des Kopfes in Figuren-Koordinaten (vorne = +z, Fuesse bei y = 0)
    pub look: Option<Vec3>,
    /// wie weit sich der Kopf zur Seite drehen darf (Radiant; ueber 1.6 = unmenschlich)
    pub look_limit: f32,
    /// Zucken 0..1: kurze Rucke in Kopf, Hals und Armen
    pub twitch: f32,
    /// Strecken 0..1: Hals und Arme werden laenger
    pub stretch: f32,
    /// Stop-Motion: Bilder pro Sekunde (0 = fluessig)
    pub step_fps: f32,
    /// Zufallswert der Figur (fuer Zucken)
    pub seed: f32,
    /// Kopf um die Blickachse gedreht (Radiant; PI = kopfueber)
    pub head_roll: f32,
    /// Zerreissen der Figur im Shader (0..1)
    pub distort: f32,
}

pub struct AnimSet {
    names: Vec<String>,
    parents: Vec<i32>,
    rest_inv: Vec<Mat4>,
    clips: Vec<ClipData>,
    clip_names: Vec<String>,
    head: Option<usize>,
    neck: Option<usize>,
    arms: Vec<usize>,
    twitchy: Vec<usize>,
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn mat34(b: &[u8], o: usize) -> Mat4 {
    let m = |r: usize, c: usize| f32_at(b, o + (r * 4 + c) * 4);
    Mat4::from_cols(
        Vec4::new(m(0, 0), m(1, 0), m(2, 0), 0.0),
        Vec4::new(m(0, 1), m(1, 1), m(2, 1), 0.0),
        Vec4::new(m(0, 2), m(1, 2), m(2, 2), 0.0),
        Vec4::new(m(0, 3), m(1, 3), m(2, 3), 1.0),
    )
}

pub fn parse_anim(b: &[u8]) -> AnimSet {
    assert_eq!(&b[0..4], b"K0A2", "keine K0A2-Animation");
    let bones = u32_at(b, 4) as usize;
    let count = u32_at(b, 8) as usize;
    let mut o = 12;
    let (mut names, mut parents, mut rest_inv) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..bones {
        let len = b[o] as usize;
        names.push(String::from_utf8_lossy(&b[o + 1..o + 1 + len]).into_owned());
        o += 1 + len;
        let p = i32::from_le_bytes(b[o..o + 4].try_into().unwrap());
        assert!(p < i as i32, "Eltern muessen vor ihren Kindern stehen");
        parents.push(p);
        rest_inv.push(mat34(b, o + 4).inverse());
        o += 4 + 48;
    }
    let (mut clips, mut clip_names) = (Vec::new(), Vec::new());
    for _ in 0..count {
        let len = b[o] as usize;
        clip_names.push(String::from_utf8_lossy(&b[o + 1..o + 1 + len]).into_owned());
        o += 1 + len;
        let fps = f32_at(b, o);
        let frames = u32_at(b, o + 4) as usize;
        let looped = b[o + 8] != 0;
        o += 9;
        let mut locals = Vec::with_capacity(frames * bones);
        for _ in 0..frames * bones {
            let f = |k: usize| f32_at(b, o + k * 4);
            locals.push(Local {
                rot: Quat::from_xyzw(f(0), f(1), f(2), f(3)).normalize(),
                pos: Vec3::new(f(4), f(5), f(6)),
                scale: Vec3::new(f(7), f(8), f(9)),
            });
            o += 40;
        }
        clips.push(ClipData {
            fps,
            frames,
            looped,
            locals,
        });
    }
    let find = |n: &str| names.iter().position(|x| x == n);
    let arms = ["UpperArm.L", "UpperArm.R", "LowerArm.L", "LowerArm.R"]
        .iter()
        .filter_map(|n| find(n))
        .collect();
    let twitchy = ["Neck", "Head", "UpperArm.L", "UpperArm.R", "Chest"]
        .iter()
        .filter_map(|n| find(n))
        .collect();
    AnimSet {
        head: find("Head"),
        neck: find("Neck"),
        arms,
        twitchy,
        names,
        parents,
        rest_inv,
        clips,
        clip_names,
    }
}

fn hash(x: f32) -> f32 {
    ((x * 12.9898).sin() * 43_758.547).fract()
}

impl AnimSet {
    pub fn bones(&self) -> usize {
        self.parents.len()
    }

    pub fn bone(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|x| x == name)
    }

    pub fn has_clip(&self, clip: Clip) -> bool {
        (clip as usize) < self.clips.len()
    }

    /// Name eines Clips in der Datei (zum Pruefen der Reihenfolge).
    pub fn clip_name(&self, clip: Clip) -> &str {
        &self.clip_names[clip as usize]
    }

    /// Dauer eines Clips in Sekunden.
    pub fn duration(&self, clip: Clip) -> f32 {
        let c = &self.clips[(clip as usize).min(self.clips.len() - 1)];
        c.frames as f32 / c.fps
    }

    /// Lokale Pose eines Clips zur Zeit `t` (Schleife oder am Ende stehen bleiben).
    fn sample_locals(&self, clip: Clip, t: f32, out: &mut Vec<Local>) {
        let c = &self.clips[(clip as usize).min(self.clips.len() - 1)];
        let n = self.bones();
        let pos = if c.looped {
            (t * c.fps).rem_euclid(c.frames as f32)
        } else {
            (t * c.fps).clamp(0.0, (c.frames - 1) as f32)
        };
        let f0 = pos.floor() as usize % c.frames;
        let f1 = if c.looped {
            (f0 + 1) % c.frames
        } else {
            (f0 + 1).min(c.frames - 1)
        };
        let a = pos.fract();
        out.clear();
        for b in 0..n {
            out.push(c.locals[f0 * n + b].mix(c.locals[f1 * n + b], a));
        }
    }

    /// Skinning-Matrizen ohne Eingriffe (fuer Tests und einfache Figuren).
    pub fn sample(&self, clip: Clip, t: f32, out: &mut Vec<Mat4>) {
        self.pose(clip, t, &PoseFx::default(), out);
    }

    /// Skinning-Matrizen fuer Clip `clip` zur Zeit `t` mit allen Eingriffen aus `fx`.
    pub fn pose(&self, clip: Clip, t: f32, fx: &PoseFx, out: &mut Vec<Mat4>) {
        let n = self.bones();
        let step = |t: f32| {
            if fx.step_fps > 0.0 {
                (t * fx.step_fps).floor() / fx.step_fps
            } else {
                t
            }
        };
        let mut locals = Vec::with_capacity(n);
        self.sample_locals(clip, step(t), &mut locals);
        if let Some((prev, pt, w)) = fx.blend {
            if w > 0.0 {
                let mut other = Vec::with_capacity(n);
                self.sample_locals(prev, step(pt), &mut other);
                for (l, o) in locals.iter_mut().zip(&other) {
                    *l = l.mix(*o, w.min(1.0));
                }
            }
        }
        // Zucken: kurze, harte Rucke (wechseln mehrmals pro Sekunde)
        if fx.twitch > 0.0 {
            let slot = (t * 7.0).floor();
            for (k, &b) in self.twitchy.iter().enumerate() {
                let h = hash(slot + fx.seed * 3.1 + k as f32 * 1.7);
                if h > 1.0 - 0.35 * fx.twitch {
                    let ax = Vec3::new(
                        hash(h + 1.0) - 0.5,
                        hash(h + 2.0) - 0.5,
                        hash(h + 3.0) - 0.5,
                    )
                    .normalize_or_zero();
                    locals[b].rot *= Quat::from_axis_angle(ax, 0.6 * fx.twitch);
                }
            }
        }
        // Strecken: Hals und Arme laenger (Knochen-Achse = lokales y)
        if fx.stretch > 0.0 {
            if let Some(nk) = self.neck {
                locals[nk].scale.y *= 1.0 + 1.2 * fx.stretch;
            }
            for &a in &self.arms {
                locals[a].scale.y *= 1.0 + 0.35 * fx.stretch;
            }
        }
        // Hierarchie durchrechnen; Kopf und Hals drehen sich zum Blickziel
        let mut global: Vec<Mat4> = Vec::with_capacity(n);
        for b in 0..n {
            let mut g = match self.parents[b] {
                p if p >= 0 => global[p as usize] * locals[b].mat(),
                _ => locals[b].mat(),
            };
            if let Some(target) = fx.look {
                let share = if Some(b) == self.neck {
                    0.35
                } else if Some(b) == self.head {
                    1.0
                } else {
                    0.0
                };
                if share > 0.0 {
                    g = look_towards(g, &self.rest_inv[b], target, fx.look_limit, share);
                }
            }
            if fx.head_roll != 0.0 && Some(b) == self.head {
                // Kopf um die eigene Blickachse drehen (bis kopfueber)
                let pivot = g.w_axis.truncate();
                let axis = (g * self.rest_inv[b])
                    .transform_vector3(Vec3::Z)
                    .normalize_or_zero();
                if axis != Vec3::ZERO {
                    g = Mat4::from_translation(pivot)
                        * Mat4::from_quat(Quat::from_axis_angle(axis, fx.head_roll))
                        * Mat4::from_translation(-pivot)
                        * g;
                }
            }
            global.push(g);
        }
        out.extend(global.iter().zip(&self.rest_inv).map(|(g, r)| *g * *r));
    }
}

/// Dreht die globale Knochen-Matrix `g` um ihren Ursprung, sodass die Blickrichtung
/// (Vorwaerts der Ruhepose) zum Ziel zeigt, seitlich begrenzt auf `limit`, nach oben/unten
/// auf 0.7 rad. `share` = Anteil der Drehung (Hals nur teilweise, Kopf den Rest).
fn look_towards(g: Mat4, rest_inv: &Mat4, target: Vec3, limit: f32, share: f32) -> Mat4 {
    let pivot = g.w_axis.truncate();
    // aktuelle Blickrichtung: Ruhepose schaut nach +z; g * rest_inv bildet Ruhe -> jetzt ab
    let now = (g * *rest_inv)
        .transform_vector3(Vec3::Z)
        .normalize_or_zero();
    let want = (target - pivot).normalize_or_zero();
    if now == Vec3::ZERO || want == Vec3::ZERO {
        return g;
    }
    use std::f32::consts::{PI, TAU};
    let wrap = |a: f32| (a + PI).rem_euclid(TAU) - PI;
    let yaw_now = now.x.atan2(now.z);
    // seitlich relativ zum Koerper (vorne = +z) begrenzen, dann den fehlenden Anteil drehen
    let desired = want.x.atan2(want.z).clamp(-limit, limit);
    let dyaw = wrap(desired - yaw_now);
    let pitch_now = now.y.clamp(-1.0, 1.0).asin();
    let dpitch = (want.y.clamp(-1.0, 1.0).asin().clamp(-0.7, 0.7) - pitch_now) * share;
    let yaw_q = Quat::from_rotation_y(dyaw * share);
    let right = (yaw_q * now).cross(Vec3::Y).normalize_or_zero();
    let pitch_q = if right == Vec3::ZERO {
        Quat::IDENTITY
    } else {
        Quat::from_axis_angle(right, -dpitch)
    };
    let r = pitch_q * yaw_q;
    Mat4::from_translation(pivot) * Mat4::from_quat(r) * Mat4::from_translation(-pivot) * g
}

/// Clip-Wechsel mit Ueberblenden, pro Figur. Die Zeit liefert der Aufrufer (bei Laufclips an
/// die Strecke gekoppelt, sonst normale Zeit); Einmal-Clips zaehlen ab ihrem Start.
#[derive(Clone, Copy, Debug, Default)]
pub struct Animator {
    pub clip: Clip,
    /// Zeit beim Start des aktuellen Clips
    start: f32,
    /// zuletzt gemeldete Zeit
    now: f32,
    prev: Option<(Clip, f32)>,
    fade: f32,
    fade_len: f32,
}

impl Animator {
    /// Gewuenschten Clip melden. `time` laeuft stetig (Sekunden), `fade` = Ueberblendzeit.
    pub fn update(&mut self, clip: Clip, time: f32, dt: f32, fade: f32) {
        if clip != self.clip {
            self.prev = Some((self.clip, self.clip_time()));
            self.clip = clip;
            self.start = time;
            self.fade = 0.0;
            self.fade_len = fade.max(1e-3);
        } else if let Some((_, t)) = &mut self.prev {
            *t += dt;
        }
        self.now = time;
        self.fade += dt;
        if self.fade >= self.fade_len {
            self.prev = None;
        }
    }

    /// Zeit im aktuellen Clip (Schleifen: stetige Zeit, Einmal-Clips: ab Start).
    pub fn clip_time(&self) -> f32 {
        if self.clip.locomotion() || is_loop_idle(self.clip) {
            self.now
        } else {
            self.now - self.start
        }
    }

    /// Ueberblend-Angabe fuer PoseFx.
    pub fn blend(&self) -> Option<(Clip, f32, f32)> {
        self.prev
            .map(|(c, t)| (c, t, (1.0 - self.fade / self.fade_len).clamp(0.0, 1.0)))
            .filter(|b| b.2 > 0.0)
    }

    /// Einmal-Clip von vorn (z. B. dieselbe Ware immer wieder nehmen).
    pub fn restart(&mut self) {
        self.start = self.now;
    }

    /// Ist ein Einmal-Clip durchgelaufen?
    pub fn finished(&self, duration: f32) -> bool {
        !is_loop_idle(self.clip) && !self.clip.locomotion() && self.now - self.start >= duration
    }
}

/// Clips, die als Schleife im Stand laufen (stetige Zeit)
fn is_loop_idle(c: Clip) -> bool {
    !matches!(
        c,
        Clip::Grab | Clip::Hit | Clip::Death | Clip::GetUp | Clip::PickUp
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::people::{anims, load_skins, PEOPLE};

    const ALL: [Clip; 23] = [
        Clip::Idle,
        Clip::Walk,
        Clip::Reach,
        Clip::Run,
        Clip::Grab,
        Clip::Wave,
        Clip::Hit,
        Clip::Death,
        Clip::CrouchWalk,
        Clip::CrouchIdle,
        Clip::ZombieWalk,
        Clip::ZombieIdle,
        Clip::Scratch,
        Clip::GetUp,
        Clip::FoldArms,
        Clip::Phone,
        Clip::Talk,
        Clip::PickUp,
        Clip::Push,
        Clip::Carry,
        Clip::Kneel,
        Clip::No,
        Clip::WalkFormal,
    ];

    #[test]
    fn clip_order_matches_the_file() {
        let names = [
            "idle",
            "walk",
            "reach",
            "run",
            "grab",
            "wave",
            "hit",
            "death",
            "crouch_walk",
            "crouch_idle",
            "zombie_walk",
            "zombie_idle",
            "scratch",
            "get_up",
            "fold_arms",
            "phone",
            "talk",
            "pick_up",
            "push",
            "carry",
            "kneel",
            "no",
            "walk_formal",
        ];
        for female in [false, true] {
            let a = anims(female);
            for (c, n) in ALL.iter().zip(names) {
                assert!(a.has_clip(*c));
                assert_eq!(a.clip_name(*c), n);
            }
        }
    }

    /// Tiefster und hoechster Punkt einer Figur in einer Pose.
    fn extent(skin: &crate::people::SkinData, m: &[Mat4]) -> (f32, f32) {
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for vx in skin.vertices.iter().step_by(5) {
            let p = Vec3::from(vx.pos).extend(1.0);
            let mut r = Vec4::ZERO;
            for k in 0..4 {
                r += m[vx.joints[k] as usize] * p * vx.weights[k];
            }
            lo = lo.min(r.y);
            hi = hi.max(r.y);
        }
        (lo, hi)
    }

    #[test]
    fn every_clip_keeps_the_figure_on_the_ground() {
        let skins = load_skins();
        for v in [0usize, PEOPLE.len() - 1] {
            let a = anims(PEOPLE[v].1);
            for c in ALL {
                let dur = a.duration(c);
                let mut worst = (f32::MAX, f32::MIN);
                for k in 0..8 {
                    let mut m = Vec::new();
                    a.sample(c, dur * k as f32 / 8.0, &mut m);
                    let (lo, hi) = extent(&skins[v], &m);
                    worst = (worst.0.min(lo), worst.1.max(hi));
                }
                assert!(
                    worst.0 > -0.09 && worst.0 < 0.15,
                    "{} {:?}: tiefster Punkt {}",
                    PEOPLE[v].0,
                    c,
                    worst.0
                );
                assert!(worst.1 < 2.1, "{} {:?}: {} m hoch", PEOPLE[v].0, c, worst.1);
                if !matches!(c, Clip::GetUp | Clip::Death) {
                    assert!(
                        worst.1 > 0.9,
                        "{} {:?}: nur {} m hoch",
                        PEOPLE[v].0,
                        c,
                        worst.1
                    );
                }
            }
        }
    }

    #[test]
    fn blending_goes_smoothly_from_old_to_new_clip() {
        let a = anims(false);
        let pose = |w: f32| {
            let mut m = Vec::new();
            let fx = PoseFx {
                blend: Some((Clip::Walk, 0.3, w)),
                ..Default::default()
            };
            a.pose(Clip::Idle, 0.5, &fx, &mut m);
            m
        };
        let (mut walk, mut idle) = (Vec::new(), Vec::new());
        a.sample(Clip::Walk, 0.3, &mut walk);
        a.sample(Clip::Idle, 0.5, &mut idle);
        let diff = |x: &[Mat4], y: &[Mat4]| {
            x.iter()
                .zip(y)
                .map(|(p, q)| (p.w_axis - q.w_axis).length())
                .fold(0.0f32, f32::max)
        };
        assert!(diff(&pose(1.0), &walk) < 1e-3);
        assert!(diff(&pose(0.0), &idle) < 1e-3);
        // kleine Gewichtsschritte = kleine Posen-Spruenge (kein Umweg ueber die falsche Seite)
        let total = diff(&pose(0.0), &pose(1.0));
        for k in 0..10 {
            let (w0, w1) = (k as f32 / 10.0, (k + 1) as f32 / 10.0);
            let d = diff(&pose(w0), &pose(w1));
            assert!(d < total * 0.25 + 0.005, "Sprung bei {w0}: {d} von {total}");
        }
    }

    /// Blickrichtung des Kopfes (Figuren-Koordinaten) in einer Pose.
    fn head_dir(a: &AnimSet, fx: &PoseFx) -> Vec3 {
        let mut m = Vec::new();
        a.pose(Clip::Idle, 0.0, fx, &mut m);
        let h = a.bone("Head").unwrap();
        m[h].transform_vector3(Vec3::Z).normalize()
    }

    #[test]
    fn head_turns_to_the_target_within_its_limit() {
        let a = anims(false);
        let side = Vec3::new(5.0, 1.6, 0.0); // rechts neben der Figur (+x)
        let free = head_dir(
            a,
            &PoseFx {
                look: Some(side),
                look_limit: 2.0,
                ..Default::default()
            },
        );
        let yaw = free.x.atan2(free.z);
        assert!(
            (yaw - std::f32::consts::FRAC_PI_2).abs() < 0.2,
            "Kopf schaut nach {yaw}"
        );
        let limited = head_dir(
            a,
            &PoseFx {
                look: Some(side),
                look_limit: 0.5,
                ..Default::default()
            },
        );
        let yaw = limited.x.atan2(limited.z);
        assert!((yaw - 0.5).abs() < 0.15, "Grenze nicht eingehalten: {yaw}");
        // ganz nach hinten (unmenschlich), wenn die Grenze es erlaubt
        let back = head_dir(
            a,
            &PoseFx {
                look: Some(Vec3::new(0.01, 1.6, -5.0)),
                look_limit: 3.2,
                ..Default::default()
            },
        );
        assert!(back.z < -0.8, "Kopf dreht sich nicht nach hinten: {back}");
    }

    #[test]
    fn head_can_be_turned_upside_down() {
        let a = anims(false);
        let mut m = Vec::new();
        a.pose(
            Clip::Idle,
            0.0,
            &PoseFx {
                head_roll: std::f32::consts::PI,
                ..Default::default()
            },
            &mut m,
        );
        let h = a.bone("Head").unwrap();
        let up = m[h].transform_vector3(Vec3::Y).normalize();
        assert!(up.y < -0.8, "Kopf nicht kopfueber: {up}");
    }

    #[test]
    fn stretching_makes_the_neck_longer() {
        let skins = load_skins();
        let a = anims(PEOPLE[0].1);
        let mut plain = Vec::new();
        let mut long = Vec::new();
        a.sample(Clip::Idle, 0.0, &mut plain);
        a.pose(
            Clip::Idle,
            0.0,
            &PoseFx {
                stretch: 1.0,
                ..Default::default()
            },
            &mut long,
        );
        let h0 = extent(&skins[0], &plain).1;
        let h1 = extent(&skins[0], &long).1;
        assert!(h1 > h0 + 0.08, "Hals nicht laenger: {h0} -> {h1}");
    }

    #[test]
    fn animator_crossfades_and_counts_one_shots_from_their_start() {
        let mut an = Animator::default();
        an.update(Clip::Walk, 5.0, 0.1, 0.2);
        an.update(Clip::Walk, 5.1, 0.1, 0.2);
        an.update(Clip::Grab, 5.2, 0.1, 0.2);
        assert!(an.blend().is_some(), "kein Ueberblenden");
        assert!(an.clip_time() < 1e-4, "Einmal-Clip startet nicht bei 0");
        for k in 0..5 {
            an.update(Clip::Grab, 5.3 + k as f32 * 0.1, 0.1, 0.2);
        }
        assert!(an.blend().is_none(), "Ueberblenden endet nicht");
        assert!(an.finished(0.4));
    }
}
