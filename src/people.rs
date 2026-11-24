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

use mettgine::gpu::SkinVertex;
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

/// Alle Figuren, Index = Variante. Mitarbeiter bekommen den Brust-Schriftzug.
pub fn load_skins() -> Vec<SkinData> {
    gen::PEOPLE_SKIN_BYTES
        .iter()
        .enumerate()
        .map(|(v, b)| {
            let mut s = parse_skin(b).expect("eingebaute Figur defekt");
            if PEOPLE[v].2 {
                let badge = badge(&s, height(v));
                s.vertices.extend(badge);
            }
            s
        })
        .collect()
}

/// Brust-Schriftzug der Mitarbeiter
pub const BADGE_TEXT: &str = "Mettarbeiter";
/// Pixelbild: Breite, Hoehe (Pixel) und Pixelgroesse (Meter) -> 22 cm breit
const BADGE_W: usize = 110;
const BADGE_H: usize = 22;
const BADGE_PIX: f32 = 0.002;
/// Mitte des Schriftzugs (Anteil der Koerpergroesse) und Abstand vor der Brust (Meter)
const BADGE_Y: f32 = 0.74;
const BADGE_LIFT: f32 = 0.008;

/// Vorderste Stelle der Dreiecke `tris` bei (x, y): groesstes z der Dreiecke, die in der
/// Ansicht von vorn dort liegen (None = keins).
pub(crate) fn surface_z(tris: &[SkinVertex], x: f32, y: f32) -> Option<f32> {
    tris.chunks_exact(3)
        .filter_map(|t| {
            let [a, b, c] = [t[0].pos, t[1].pos, t[2].pos].map(Vec3::from);
            let d = (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y);
            if d.abs() < 1e-9 {
                return None;
            }
            let l1 = ((x - a.x) * (c.y - a.y) - (c.x - a.x) * (y - a.y)) / d;
            let l2 = ((b.x - a.x) * (y - a.y) - (x - a.x) * (b.y - a.y)) / d;
            let inside = l1 >= 0.0 && l2 >= 0.0 && l1 + l2 <= 1.0;
            inside.then(|| a.z + l1 * (b.z - a.z) + l2 * (c.z - a.z))
        })
        .reduce(f32::max)
}

/// Dreiecke des Schriftzugs "Mettarbeiter" vorn auf der Brust (Figur schaut nach +z, Fuss
/// bei y = 0). Der Schriftzug folgt der Brustform (immer `BADGE_LIFT` vor der Oberflaeche)
/// und jeder Vertex bewegt sich mit dem naechsten Brust-Vertex der Figur mit.
fn badge(s: &SkinData, body_h: f32) -> Vec<SkinVertex> {
    let mut px = vec![false; BADGE_W * BADGE_H];
    crate::decal_text::raster(
        BADGE_TEXT,
        BADGE_W,
        2.0,
        BADGE_H as f32 - 6.0,
        &mut |x, y| {
            if (0..BADGE_W as i32).contains(&x) && (0..BADGE_H as i32).contains(&y) {
                px[y as usize * BADGE_W + x as usize] = true;
            }
        },
    );
    let cy = body_h * BADGE_Y;
    let half_w = BADGE_W as f32 * BADGE_PIX * 0.5;
    let half_h = BADGE_H as f32 * BADGE_PIX * 0.5;
    // Brust: Dreiecke der Vorderseite im Bereich des Schriftzugs (etwas grosszuegiger)
    let near_chest =
        |v: &SkinVertex| v.pos[0].abs() < half_w + 0.03 && (v.pos[1] - cy).abs() < half_h + 0.06;
    let tris: Vec<SkinVertex> = s
        .vertices
        .chunks_exact(3)
        .filter(|t| t.iter().any(|v| near_chest(v) && v.pos[2] > 0.0))
        .flatten()
        .copied()
        .collect();
    let chest: Vec<&SkinVertex> = tris
        .iter()
        .filter(|v| near_chest(v) && v.pos[2] > 0.0)
        .collect();
    let Some(front) = chest.iter().map(|v| v.pos[2]).reduce(f32::max) else {
        return Vec::new();
    };
    let top = cy + half_h;
    let at = |u: f32, y: f32| {
        let z = surface_z(&tris, u, y).unwrap_or(front) + BADGE_LIFT;
        let p = Vec3::new(u, y, z);
        let near = chest
            .iter()
            .min_by(|a, b| {
                let d = |v: &&&SkinVertex| Vec3::from(v.pos).distance_squared(p);
                d(a).total_cmp(&d(b))
            })
            .expect("Brust ohne Vertices");
        SkinVertex {
            pos: p.to_array(),
            normal: [0.0, 0.0, 1.0],
            color: [0.95, 0.95, 0.95],
            joints: near.joints,
            weights: near.weights,
        }
    };
    let mut out = Vec::new();
    for (row, x0, x1) in crate::decal_text::runs(&px, BADGE_W, BADGE_H) {
        let u0 = (x0 as f32 - BADGE_W as f32 * 0.5) * BADGE_PIX;
        let u1 = (x1 as f32 - BADGE_W as f32 * 0.5) * BADGE_PIX;
        let (y0, y1) = (
            top - (row + 1) as f32 * BADGE_PIX,
            top - row as f32 * BADGE_PIX,
        );
        out.extend([
            at(u0, y0),
            at(u1, y0),
            at(u1, y1),
            at(u0, y0),
            at(u1, y1),
            at(u0, y1),
        ]);
    }
    out
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

    #[test]
    fn staff_wear_the_lettering_on_the_chest() {
        let skins = load_skins();
        for (v, s) in skins.iter().enumerate() {
            let raw = parse_skin(gen::PEOPLE_SKIN_BYTES[v]).unwrap();
            let n = raw.vertices.len();
            if !PEOPLE[v].2 {
                // Kunden bleiben, wie sie sind
                assert_eq!(s.vertices.len(), n, "{}", PEOPLE[v].0);
                assert!(s
                    .vertices
                    .iter()
                    .zip(&raw.vertices)
                    .all(|(a, b)| a.pos == b.pos && a.color == b.color));
                continue;
            }
            assert!(s.vertices.len() > n, "{}: kein Schriftzug", PEOPLE[v].0);
            let added = &s.vertices[n..];
            assert_eq!(added.len() % 3, 0);
            let front = raw
                .vertices
                .iter()
                .map(|vx| vx.pos[2])
                .fold(f32::MIN, f32::max);
            let cy = height(v) * BADGE_Y;
            let (mut lo, mut hi, mut worst) = (f32::MAX, f32::MIN, 0.0f32);
            for vx in added {
                let [x, y, z] = vx.pos;
                lo = lo.min(x);
                hi = hi.max(x);
                assert!((y - cy).abs() < 0.03, "{}: Hoehe {y}", PEOPLE[v].0);
                assert!(z > 0.05 && z <= front + 0.01, "{}: z {z}", PEOPLE[v].0);
                assert_eq!(vx.color, [0.95; 3]);
                let w: f32 = vx.weights.iter().sum();
                assert!((w - 1.0).abs() < 1e-3, "{}: Gewichte {w}", PEOPLE[v].0);
                // liegt auf der Brust, nicht frei in der Luft und nicht darin: Abstand zur
                // Koerperoberflaeche direkt dahinter (Strahl nach -z) wenige Millimeter
                let body = surface_z(&raw.vertices, x, y).expect("keine Brust dahinter");
                worst = worst.max(z - body);
                assert!(z - body > 0.004, "{}: steckt in der Brust", PEOPLE[v].0);
            }
            assert!(worst < 0.015, "{}: {worst} m vor der Brust", PEOPLE[v].0);
            // ungefaehr 22 cm breit, mittig
            assert!(hi - lo > 0.18 && hi - lo < 0.23, "Breite {}", hi - lo);
            assert!((hi + lo).abs() < 0.02);
        }
        assert!(!people_of(true).is_empty() && !people_of(false).is_empty());
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
