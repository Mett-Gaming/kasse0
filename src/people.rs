//! Animierte Figuren (Kundschaft und Personal): Meshes mit Knochen-Gewichten (.k0s) und
//! Animationen mit Knochen-Hierarchie (.k0a v2), erzeugt von tools/prepare_people.py.
//! Die CPU rechnet die Pose (anim.rs), die GPU verformt die Figur (Skinning im Vertex-Shader).

use glam::{Vec3, Vec4};
use std::sync::OnceLock;

mod clips;
pub use clips::{Clip, SIT_BACK, SIT_UP};
pub use mettgine::anim::AnimSet;
/// Eingriffe in die Pose und Clip-Wechsel, fuer die KASSE-0-Clips
pub type PoseFx = mettgine::anim::PoseFx<Clip>;
pub type Animator = mettgine::anim::Animator<Clip>;

#[cfg(test)]
mod anim_tests;

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

pub use mettgine::gpu::{parse_skin, SkinData};

/// Animationen fuer Maenner- bzw. Frauen-Skelett.
pub fn anims(female: bool) -> &'static AnimSet {
    static M: OnceLock<AnimSet> = OnceLock::new();
    static F: OnceLock<AnimSet> = OnceLock::new();
    if female {
        F.get_or_init(|| {
            mettgine::anim::parse_anim(gen::ANIM_F_BYTES).expect("eingebaute Animation defekt")
        })
    } else {
        M.get_or_init(|| {
            mettgine::anim::parse_anim(gen::ANIM_M_BYTES).expect("eingebaute Animation defekt")
        })
    }
}

/// Alle Figuren, Index = Variante.
pub fn load_skins() -> Vec<SkinData> {
    gen::PEOPLE_SKIN_BYTES
        .iter()
        .map(|b| parse_skin(b).expect("eingebaute Figur defekt"))
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
    /// Blickziel des Kopfes (Welt), None = Kopf folgt der Animation
    pub look_at: Option<Vec3>,
    /// Ueberblenden, Blickgrenze, Zucken, Strecken, Stop-Motion
    pub fx: PoseFx,
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

    /// Alle Vertices einer Figur in der Pose `clip` bei t = 0 (Engine-Koordinaten).
    fn posed(v: usize, s: &SkinData, clip: Clip) -> Vec<(Vec3, Vec3)> {
        let mut m = Vec::new();
        anims(PEOPLE[v].1).sample(clip, 0.0, &mut m);
        s.vertices
            .iter()
            .map(|vx| {
                let p = Vec3::from(vx.pos).extend(1.0);
                let mut r = Vec4::ZERO;
                for k in 0..4 {
                    r += m[vx.joints[k] as usize] * p * vx.weights[k];
                }
                (Vec3::from(vx.pos), r.truncate())
            })
            .collect()
    }

    #[test]
    fn the_cashier_can_sit_on_a_chair() {
        // Sitzpose (Clip "sit"): deutlich tiefer als im Stehen, Fuesse am Boden, Gesaess
        // etwa SIT_BACK hinter dem Ursprung in Sitzhoehe - dort steht der Stuhl
        let skins = load_skins();
        for (v, s) in skins.iter().enumerate() {
            assert!(anims(PEOPLE[v].1).has_clip(Clip::Sit), "Sitz-Clip fehlt");
            let stand = posed(v, s, Clip::Idle);
            let sit = posed(v, s, Clip::Sit);
            let top = |pts: &[(Vec3, Vec3)]| pts.iter().map(|p| p.1.y).fold(f32::MIN, f32::max);
            let low = sit.iter().map(|p| p.1.y).fold(f32::MAX, f32::min);
            assert!(
                top(&sit) < top(&stand) - 0.25,
                "{} sitzt nicht",
                PEOPLE[v].0
            );
            assert!(
                low.abs() < 0.15,
                "{} Fuesse schweben im Sitzen: {low}",
                PEOPLE[v].0
            );
            // Gesaess: tiefster Punkt der Vertices, die in Ruhe auf Hueft-Hoehe liegen
            let butt = sit
                .iter()
                .filter(|(rest, _)| rest.y > 0.75 && rest.y < 0.95)
                .map(|p| p.1)
                .min_by(|a, b| a.y.total_cmp(&b.y))
                .unwrap();
            assert!(
                butt.y > 0.3 && butt.y < 0.6,
                "{} Gesaess auf {} m",
                PEOPLE[v].0,
                butt.y
            );
            assert!(
                (butt.z + SIT_BACK).abs() < 0.15,
                "{} Gesaess {} m vor/hinter dem Ursprung",
                PEOPLE[v].0,
                butt.z
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
