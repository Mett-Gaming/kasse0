//! Tests des Animationssystems mit den echten KASSE-0-Figuren.

use crate::people::{anims, load_skins, AnimSet, Clip, PoseFx, PEOPLE};
use glam::{Mat4, Vec3, Vec4};

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
