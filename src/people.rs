//! Animierte Figuren (Kundschaft und Personal): Meshes mit Knochen-Gewichten (.k0s) und
//! Animationen als Knochen-Matrizen pro Bild (.k0a), erzeugt von tools/prepare_people.py.
//! Die CPU tastet die Animation ab, die GPU verformt die Figur (Skinning im Vertex-Shader).

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use std::sync::OnceLock;

mod gen {
    include!(concat!(env!("OUT_DIR"), "/people_gen.rs"));
}
/// Figuren (Quaternius Modular Men/Women): (Name, weiblich, Mitarbeiter)
pub use gen::PEOPLE;

/// Indizes der Kunden- bzw. Mitarbeiter-Varianten.
pub fn people_of(staff: bool) -> Vec<usize> {
    (0..PEOPLE.len())
        .filter(|&i| PEOPLE[i].2 == staff)
        .collect()
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SkinVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    /// Knochen-Indizes (als Gleitkommazahl)
    pub joints: [f32; 4],
    pub weights: [f32; 4],
}

pub struct SkinData {
    pub vertices: Vec<SkinVertex>,
}

/// Animation einer Figur (Reihenfolge wie CLIPS in tools/prepare_people.py)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clip {
    Idle = 0,
    Walk = 1,
    Reach = 2,
    Run = 3,
    /// Sitzen (Kassiererin): aus der Animations-Bibliothek uebertragen (tools/prepare_people.py)
    Sit = 4,
}

/// Sitzpose: so weit liegt das Gesaess hinter dem Figur-Ursprung (Meter, gegen die
/// Blickrichtung) - so stellt man die Figur, damit sie auf einem Stuhl landet
pub const SIT_BACK: f32 = 0.25;
/// und so viel hoeher, damit es auf der Sitzflaeche aufliegt
pub const SIT_UP: f32 = 0.03;

struct ClipData {
    fps: f32,
    frames: usize,
    /// frames * bones Matrizen
    mats: Vec<Mat4>,
}

pub struct AnimSet {
    bones: usize,
    clips: Vec<ClipData>,
}

fn f32_at(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}

pub fn parse_skin(b: &[u8]) -> SkinData {
    assert_eq!(&b[0..4], b"K0S1", "keine K0S1-Figur");
    let n = u32_at(b, 4) as usize;
    let data = &b[32..32 + n * 68];
    let vertices = data
        .chunks_exact(68)
        .map(|v| {
            let f = |i: usize| f32_at(v, i * 4);
            SkinVertex {
                pos: [f(0), f(1), f(2)],
                normal: [f(3), f(4), f(5)],
                color: [f(6), f(7), f(8)],
                joints: [f(9), f(10), f(11), f(12)],
                weights: [f(13), f(14), f(15), f(16)],
            }
        })
        .collect();
    SkinData { vertices }
}

fn parse_anim(b: &[u8]) -> AnimSet {
    assert_eq!(&b[0..4], b"K0A1", "keine K0A1-Animation");
    let bones = u32_at(b, 4) as usize;
    let count = u32_at(b, 8) as usize;
    let mut o = 12;
    let mut clips = Vec::with_capacity(count);
    for _ in 0..count {
        let len = b[o] as usize;
        o += 1 + len;
        let fps = f32_at(b, o);
        let frames = u32_at(b, o + 4) as usize;
        o += 8;
        let mut mats = Vec::with_capacity(frames * bones);
        for _ in 0..frames * bones {
            let m = |r: usize, c: usize| f32_at(b, o + (r * 4 + c) * 4);
            // 3x4 zeilenweise -> Spalten fuer glam
            mats.push(Mat4::from_cols(
                Vec4::new(m(0, 0), m(1, 0), m(2, 0), 0.0),
                Vec4::new(m(0, 1), m(1, 1), m(2, 1), 0.0),
                Vec4::new(m(0, 2), m(1, 2), m(2, 2), 0.0),
                Vec4::new(m(0, 3), m(1, 3), m(2, 3), 1.0),
            ));
            o += 48;
        }
        clips.push(ClipData { fps, frames, mats });
    }
    AnimSet { bones, clips }
}

impl AnimSet {
    pub fn bones(&self) -> usize {
        self.bones
    }

    /// Knochen-Matrizen zur Zeit `t` (Sekunden, laeuft in Schleife), zwischen zwei Bildern gemischt.
    pub fn sample(&self, clip: Clip, t: f32, out: &mut Vec<Mat4>) {
        let c = &self.clips[clip as usize];
        let pos = (t * c.fps).rem_euclid(c.frames as f32);
        let f0 = pos.floor() as usize % c.frames;
        let f1 = (f0 + 1) % c.frames;
        let a = pos.fract();
        for b in 0..self.bones {
            let m0 = c.mats[f0 * self.bones + b];
            let m1 = c.mats[f1 * self.bones + b];
            out.push(Mat4::from_cols(
                m0.x_axis.lerp(m1.x_axis, a),
                m0.y_axis.lerp(m1.y_axis, a),
                m0.z_axis.lerp(m1.z_axis, a),
                m0.w_axis.lerp(m1.w_axis, a),
            ));
        }
    }

    /// Dauer eines Clips in Sekunden.
    #[allow(dead_code)]
    pub fn duration(&self, clip: Clip) -> f32 {
        let c = &self.clips[clip as usize];
        c.frames as f32 / c.fps
    }
}

/// Animationen fuer Maenner- bzw. Frauen-Skelett.
pub fn anims(female: bool) -> &'static AnimSet {
    static M: OnceLock<AnimSet> = OnceLock::new();
    static F: OnceLock<AnimSet> = OnceLock::new();
    if female {
        F.get_or_init(|| parse_anim(gen::ANIM_F_BYTES))
    } else {
        M.get_or_init(|| parse_anim(gen::ANIM_M_BYTES))
    }
}

/// Alle Figuren, Index = Variante.
pub fn load_skins() -> Vec<SkinData> {
    gen::PEOPLE_SKIN_BYTES
        .iter()
        .map(|b| parse_skin(b))
        .collect()
}

/// Koerpergroesse einer Figur (Meter).
pub fn height(variant: usize) -> f32 {
    if PEOPLE[variant % PEOPLE.len()].1 {
        1.68
    } else {
        1.78
    }
}

/// Wie das Spiel eine Figur zeichnen moechte.
#[derive(Clone, Copy, Debug)]
pub struct PersonDraw {
    pub variant: usize,
    pub clip: Clip,
    /// Animationszeit in Sekunden
    pub time: f32,
    pub pos: Vec3,
    pub yaw: f32,
    pub tint: Vec4,
    /// halbtransparentes Hologramm
    pub ghost: bool,
    /// kopfueber (Fuesse oben an `pos`)
    pub flip: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skins_and_anims_fit_together() {
        let skins = load_skins();
        assert_eq!(skins.len(), PEOPLE.len());
        for (v, s) in skins.iter().enumerate() {
            let bones = anims(PEOPLE[v].1).bones() as f32;
            assert!(!s.vertices.is_empty());
            for vx in &s.vertices {
                let w: f32 = vx.weights.iter().sum();
                assert!((w - 1.0).abs() < 1e-3, "{}: Gewichte {w}", PEOPLE[v].0);
                assert!(vx.joints.iter().all(|&j| j >= 0.0 && j < bones));
            }
        }
        // Gehen bewegt die Knochen, die Figur bleibt ungefaehr am Platz und aufrecht
        for female in [false, true] {
            let a = anims(female);
            let (mut p, mut q) = (Vec::new(), Vec::new());
            a.sample(Clip::Walk, 0.0, &mut p);
            a.sample(Clip::Walk, 0.3, &mut q);
            assert_eq!(p.len(), a.bones());
            assert!(
                p.iter().zip(&q).any(|(x, y)| !x.abs_diff_eq(*y, 1e-3)),
                "Animation steht still"
            );
        }
    }

    #[test]
    fn sitting_pose_is_lower_and_has_the_seat_behind() {
        // Sitzpose: Kopf deutlich tiefer als im Stehen, Fuesse weiter am Boden
        let skins = load_skins();
        for (v, s) in skins.iter().enumerate() {
            let a = anims(PEOPLE[v].1);
            let top = |clip: Clip| {
                let mut m = Vec::new();
                a.sample(clip, 0.0, &mut m);
                let (mut lo, mut hi) = (f32::MAX, f32::MIN);
                for vx in s.vertices.iter().step_by(5) {
                    let p = Vec3::from(vx.pos).extend(1.0);
                    let mut r = Vec4::ZERO;
                    for k in 0..4 {
                        r += m[vx.joints[k] as usize] * p * vx.weights[k];
                    }
                    lo = lo.min(r.y);
                    hi = hi.max(r.y);
                }
                (lo, hi)
            };
            let (_, stand) = top(Clip::Idle);
            let (feet, sit) = top(Clip::Sit);
            assert!(
                sit < stand - 0.25,
                "{} sitzt nicht: {stand} -> {sit}",
                PEOPLE[v].0
            );
            assert!(
                feet.abs() < 0.15,
                "{} Fuesse schweben im Sitzen: {feet}",
                PEOPLE[v].0
            );
        }
    }

    #[test]
    fn idle_pose_is_human_sized_and_on_the_ground() {
        // Ruhepose durch die erste Idle-Matrix schicken: steht auf dem Boden, Menschengroesse
        let skins = load_skins();
        for (v, s) in skins.iter().enumerate() {
            let a = anims(PEOPLE[v].1);
            let mut m = Vec::new();
            a.sample(Clip::Idle, 0.0, &mut m);
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for vx in s.vertices.iter().step_by(7) {
                let p = Vec3::from(vx.pos).extend(1.0);
                let mut r = Vec4::ZERO;
                for k in 0..4 {
                    r += m[vx.joints[k] as usize] * p * vx.weights[k];
                }
                lo = lo.min(r.y);
                hi = hi.max(r.y);
            }
            assert!(lo.abs() < 0.15, "{} schwebt/versinkt: {lo}", PEOPLE[v].0);
            assert!(hi > 1.4 && hi < 2.05, "{} ist {hi} m gross", PEOPLE[v].0);
        }
    }
}

/// Schrittgeschwindigkeit der Animationen (gemessen: so schnell wandert der aufgesetzte
/// Fuss nach hinten). Laeuft eine Figur mit anderem Tempo, wird die Animation entsprechend
/// schneller oder langsamer abgespielt, damit die Fuesse nicht rutschen.
pub fn stride_speed(female: bool, run: bool) -> f32 {
    match (female, run) {
        (false, false) => 1.21,
        (true, false) => 0.97,
        (false, true) => 2.5,
        (true, true) => 2.0,
    }
}

/// Ab diesem Tempo (m/s) rennt eine Figur statt zu gehen.
pub const RUN_FROM: f32 = 1.7;

/// Gangart einer Figur: Animationszeit, die beim Gehen an die zurueckgelegte Strecke
/// gekoppelt ist (Fuesse rutschen nicht), im Stehen normal weiterlaeuft.
#[derive(Clone, Copy, Debug, Default)]
pub struct Gait {
    last: Option<Vec3>,
    /// Animationszeit (Sekunden Clip-Zeit)
    pub t: f32,
    pub moving: bool,
    pub run: bool,
}

impl Gait {
    pub fn advance(&mut self, pos: Vec3, dt: f32, female: bool) {
        let d = self
            .last
            .map_or(0.0, |l| vec3_flat(pos - l).length())
            .min(0.25); // Sprung (Teleport, Respawn) nicht als Laufen werten
        self.last = Some(pos);
        let speed = if dt > 0.0 { d / dt } else { 0.0 };
        self.moving = d > 1e-4;
        if self.moving {
            self.run = speed > RUN_FROM;
            self.t += d / stride_speed(female, self.run);
        } else {
            self.run = false;
            self.t += dt;
        }
    }
}

fn vec3_flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}

#[cfg(test)]
mod gait_tests {
    use super::*;

    #[test]
    fn walking_animation_follows_the_distance() {
        // Wer mit halber Schrittgeschwindigkeit geht, spielt die Animation halb so schnell ab
        let mut g = Gait::default();
        let v = stride_speed(false, false) * 0.5;
        let mut p = Vec3::ZERO;
        g.advance(p, 1.0 / 60.0, false);
        let t0 = g.t;
        for _ in 0..60 {
            p.z += v / 60.0;
            g.advance(p, 1.0 / 60.0, false);
        }
        assert!(g.moving && !g.run);
        assert!(
            ((g.t - t0) - 0.5).abs() < 0.02,
            "Animationszeit {}",
            g.t - t0
        );
        // schnell: Rennanimation
        for _ in 0..10 {
            p.z += 2.2 / 60.0;
            g.advance(p, 1.0 / 60.0, false);
        }
        assert!(g.run);
        // Teleport zaehlt nicht als Laufen
        let before = g.t;
        g.advance(p + Vec3::new(30.0, 0.0, 0.0), 1.0 / 60.0, false);
        assert!(g.t - before < 0.3);
    }
}
