//! Generator fuer KAUF NULL: Discounter-Grundriss mit Abteilungen und echten Waren.

mod corridor;
mod decay;
mod fixtures;

pub use corridor::{Corridor, POSTER_TEXTS};

use super::layout::{self, Layout};
use super::material::*;
use super::*;
use crate::catalog::{catalog, Catalog};
use crate::signs::{BRAND, SLOGANS};
use decay::DecayArea;
use fixtures::*;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;
use std::f32::consts::{FRAC_PI_2, PI};

const AISLE_PITCH: f32 = 3.4;
/// So viel wird der Kollisionskoerper neben einem Durchgang gekuerzt (Meter)
const PASSAGE_TRIM: f32 = 0.035;
const GONDOLA_HALF_W: f32 = 0.6;
const DRINKS_W: f32 = 7.0;
const DRINKS_D: f32 = 7.0;
const WALL_T: f32 = 0.1;

pub const PALETTE: [Vec3; 10] = [
    Vec3::new(0.85, 0.15, 0.12),
    Vec3::new(0.95, 0.75, 0.10),
    Vec3::new(0.10, 0.35, 0.75),
    Vec3::new(0.15, 0.60, 0.25),
    Vec3::new(0.95, 0.95, 0.92),
    Vec3::new(0.55, 0.30, 0.15),
    Vec3::new(0.90, 0.45, 0.10),
    Vec3::new(0.50, 0.20, 0.60),
    Vec3::new(0.20, 0.70, 0.75),
    Vec3::new(0.95, 0.50, 0.60),
];
const CRATE_COLORS: [Vec3; 5] = [
    Vec3::new(0.50, 0.28, 0.10),
    Vec3::new(0.15, 0.35, 0.70),
    Vec3::new(0.75, 0.10, 0.10),
    Vec3::new(0.90, 0.75, 0.10),
    Vec3::new(0.10, 0.45, 0.20),
];
const CARDBOARD: Vec3 = Vec3::new(0.62, 0.47, 0.3);

pub fn desaturate(c: Vec3, amount: f32) -> Vec3 {
    let l = c.dot(vec3(0.299, 0.587, 0.114));
    c.lerp(Vec3::splat(l), amount)
}

/// Erzeugt den Markt. Gleiche Parameter ergeben exakt den gleichen Markt.
pub fn generate(params: &WorldParams) -> World {
    let cat = catalog();
    let seed = params.seed;
    let tipped = params.tipped;
    let mut layout_rng = Pcg64::seed_from_u64(seed);
    let mut rng = Pcg64::seed_from_u64(seed ^ params.shuffle.wrapping_mul(0x2545_F491_4F6C_DD1D));
    let mut weird = Pcg64::seed_from_u64(seed ^ 0x9E37_79B9_7F4A_7C15 ^ params.shuffle);
    let mut drinks_rng = Pcg64::seed_from_u64(seed.wrapping_mul(31) ^ 0xD121_C0DE);
    let mut mix_rng =
        Pcg64::seed_from_u64(seed ^ params.shuffle.wrapping_mul(0x51ED_270B) ^ 0xA1D1);

    let aisle_count: u32 = layout_rng.gen_range(5..=7);
    let base_len: f32 = layout_rng.gen_range(14.0..20.0);
    let brand = PALETTE[layout_rng.gen_range(0..PALETTE.len())];
    let lanes_n: u32 = layout_rng.gen_range(3..=4);
    let aisle_len = base_len * params.stretch;

    let aisles_x0 = 2.5;
    let aisles_x1 = aisles_x0 + aisle_count as f32 * AISLE_PITCH;
    let width = aisles_x1 + 2.5 + DRINKS_W;
    let dx0 = width - DRINKS_W;
    let aisle_z0 = 4.0;
    let aisle_z1 = aisle_z0 + aisle_len;
    let checkout_z = aisle_z1 + 3.0;
    let depth = checkout_z + 3.0 + 3.0;
    let h = CEILING_HEIGHT;

    let wall_color = if tipped {
        vec3(0.55, 0.57, 0.5)
    } else {
        vec3(0.86, 0.84, 0.78)
    };
    let shelf_color = if tipped {
        vec3(0.42, 0.44, 0.43)
    } else {
        vec3(0.6, 0.62, 0.64)
    };
    let lamp_color = if tipped {
        vec3(0.82, 1.0, 0.86)
    } else {
        vec3(1.0, 0.97, 0.92)
    };
    let strip = vec3(0.98, 0.85, 0.2);

    let mut cooler_xs = Vec::new();
    let mut cx0 = 1.0;
    while cx0 + 2.4 <= dx0 - 1.0 {
        cooler_xs.push(cx0);
        cx0 += 2.45;
    }
    let sides = 2 * (aisle_count as usize - 1);
    let lay = layout::build(sides, cooler_xs.len(), 4, params.mix, &mut mix_rng);

    let mut g = Gen {
        b: Builder::default(),
        rng: &mut rng,
        weird: &mut weird,
        params,
        cat,
        layout: lay,
        items: Vec::new(),
        spots: Vec::new(),
        missing: (0..cat.products.len())
            .filter(|&p| cat.products[p].list.is_some())
            .collect(),
        giants: Vec::new(),
        ant_trails: Vec::new(),
        rat_runs: Vec::new(),
        drips: Vec::new(),
        sparks: Vec::new(),
        ceiling_holes: Vec::new(),
    };

    // --- Huelle ---
    let floor_c = if tipped {
        vec3(0.5, 0.52, 0.47)
    } else {
        vec3(0.82, 0.8, 0.76)
    };
    if tipped {
        // Im gekippten Markt biegt sich der Boden im Gangbereich (Shader): dort als Raster
        let (gx0, gx1) = (1.0f32, (aisles_x1 + 1.5).min(width));
        let (gz0, gz1) = (aisle_z0, aisle_z1);
        let slab = |b: &mut Builder, x0: f32, x1: f32, z0: f32, z1: f32| {
            if x1 > x0 && z1 > z0 {
                b.add(
                    vec3((x0 + x1) * 0.5, -0.05, (z0 + z1) * 0.5),
                    vec3((x1 - x0) * 0.5, 0.05, (z1 - z0) * 0.5),
                    floor_c,
                    FLOOR,
                );
            }
        };
        slab(&mut g.b, 0.0, width, 0.0, gz0);
        slab(&mut g.b, 0.0, width, gz1, depth);
        slab(&mut g.b, 0.0, gx0, gz0, gz1);
        slab(&mut g.b, gx1, width, gz0, gz1);
        let (nx, nz) = (
            ((gx1 - gx0) / 1.0).ceil() as usize,
            ((gz1 - gz0) / 0.75).ceil() as usize,
        );
        for i in 0..nx {
            for k in 0..nz {
                let x0 = gx0 + (gx1 - gx0) * i as f32 / nx as f32;
                let x1 = gx0 + (gx1 - gx0) * (i + 1) as f32 / nx as f32;
                let z0 = gz0 + (gz1 - gz0) * k as f32 / nz as f32;
                let z1 = gz0 + (gz1 - gz0) * (k + 1) as f32 / nz as f32;
                slab(&mut g.b, x0, x1, z0, z1);
            }
        }
    } else {
        g.b.add(
            vec3(width * 0.5, -0.05, depth * 0.5),
            vec3(width * 0.5, 0.05, depth * 0.5),
            floor_c,
            FLOOR,
        );
    }
    g.b.add(
        vec3(width * 0.5, h + 0.05, depth * 0.5),
        vec3(width * 0.5, 0.05, depth * 0.5),
        vec3(0.88, 0.88, 0.86),
        CEILING,
    );
    g.b.wall(0.0, -WALL_T, width, -WALL_T, 0.0, h, wall_color);
    g.b.wall(-WALL_T, 0.0, -WALL_T, depth, 0.0, h, wall_color);
    g.b.wall(
        width + WALL_T,
        0.0,
        width + WALL_T,
        depth,
        0.0,
        h,
        wall_color,
    );

    // Sued mit Eingangstuer
    let door_cx = aisles_x1 + 2.5;
    let (door_x0, door_x1) = (door_cx - 1.5, door_cx + 1.5);
    let sz = depth + WALL_T;
    g.b.wall(0.0, sz, door_x0, sz, 0.0, h, wall_color);
    g.b.wall(door_x1, sz, width, sz, 0.0, h, wall_color);
    g.b.wall(door_x0, sz, door_x1, sz, 2.6, h, wall_color);
    let (outside, glow) = if tipped {
        (vec3(0.01, 0.01, 0.012), 1.0)
    } else {
        (vec3(0.85, 0.9, 0.95), 1.3)
    };
    // Nach dem Bezahlen im gekippten Markt: hinter der Tuer beginnt der Fluchtkorridor
    let mut corridor = None;
    let mut exit_zone = Aabb {
        min: vec3(door_x0, 0.0, depth + 1.2),
        max: vec3(door_x1, h, depth + 7.0),
    };
    if params.door_open && tipped {
        let (c, exit) = corridor::build(
            &mut g.b,
            params.corridor_seed,
            door_x0,
            door_x1,
            depth + WALL_T * 2.0,
        );
        corridor = Some(c);
        exit_zone = exit;
    } else if params.door_open {
        g.b.add(
            vec3(door_cx, -0.05, depth + 3.0),
            vec3(1.6, 0.05, 3.0),
            vec3(0.35, 0.35, 0.36),
            PLAIN,
        );
        g.b.add_ex(
            vec3(door_cx, 1.5, depth + 6.2),
            vec3(1.6, 1.5, 0.05),
            outside,
            EMISSIVE,
            0.0,
            glow,
            0.0,
        );
        g.b.wall(
            door_x0 - WALL_T,
            depth,
            door_x0 - WALL_T,
            depth + 6.0,
            0.0,
            h,
            vec3(0.2, 0.2, 0.2),
        );
        g.b.wall(
            door_x1 + WALL_T,
            depth,
            door_x1 + WALL_T,
            depth + 6.0,
            0.0,
            h,
            vec3(0.2, 0.2, 0.2),
        );
        g.b.solid(vec3(door_cx, 1.5, depth + 6.3), vec3(1.6, 1.5, 0.1));
    } else {
        let door_c = vec3(door_cx, 1.3, depth + 0.05);
        let door_h = vec3(1.5, 1.3, 0.04);
        g.b.add_ex(door_c, door_h, outside, EMISSIVE, 0.0, glow, 0.0);
        g.b.solid(door_c, door_h);
    }
    for x in [door_x0, door_cx, door_x1] {
        if !(params.door_open && x == door_cx) {
            g.b.add(
                vec3(x, 1.3, depth + 0.02),
                vec3(0.04, 1.3, 0.06),
                vec3(0.25, 0.26, 0.28),
                PLAIN,
            );
        }
    }
    if !tipped {
        g.b.light(
            vec3(door_cx, 1.6, depth - 1.0),
            6.0,
            vec3(0.85, 0.92, 1.0),
            0.5,
            0.0,
            0.0,
        );
    }

    // Fensterfront rechts vom Eingang: Tageslicht bzw. Schwaerze im gekippten Markt
    {
        let (x0, x1) = (door_x1 + 0.6, width - 0.6);
        let n = ((x1 - x0) / 1.5).floor().max(0.0) as usize;
        let frame_c = vec3(0.3, 0.31, 0.33);
        for k in 0..n {
            let xa = x0 + k as f32 * 1.5;
            g.b.add_ex(
                vec3(xa + 0.75, 1.45, depth - 0.012),
                vec3(0.7, 0.95, 0.008),
                outside,
                EMISSIVE,
                20.0 + k as f32,
                glow * 0.9,
                0.0,
            );
            g.b.add(
                vec3(xa + 0.02, 1.45, depth - 0.03),
                vec3(0.03, 1.0, 0.025),
                frame_c,
                PLAIN,
            );
        }
        if n > 0 {
            let xe = x0 + n as f32 * 1.5;
            g.b.add(
                vec3(xe + 0.02, 1.45, depth - 0.03),
                vec3(0.03, 1.0, 0.025),
                frame_c,
                PLAIN,
            );
            g.b.add(
                vec3((x0 + xe) * 0.5, 0.48, depth - 0.03),
                vec3((xe - x0) * 0.5 + 0.03, 0.03, 0.03),
                frame_c,
                PLAIN,
            );
            g.b.add(
                vec3((x0 + xe) * 0.5, 2.42, depth - 0.03),
                vec3((xe - x0) * 0.5 + 0.03, 0.03, 0.03),
                frame_c,
                PLAIN,
            );
            if !tipped {
                g.b.light(
                    vec3((x0 + xe) * 0.5, 1.6, depth - 1.2),
                    5.0,
                    vec3(0.85, 0.92, 1.0),
                    0.35,
                    0.0,
                    0.0,
                );
            }
        }
    }

    // Umlaufende dunkelblaue Banderole
    let band = if tipped {
        vec3(0.05, 0.07, 0.12)
    } else {
        vec3(0.1, 0.16, 0.32)
    };
    g.b.add(
        vec3(width * 0.5, 2.9, 0.012),
        vec3(width * 0.5, 0.3, 0.01),
        band,
        PLAIN,
    );
    g.b.add(
        vec3(0.012, 2.9, depth * 0.5),
        vec3(0.01, 0.3, depth * 0.5),
        band,
        PLAIN,
    );
    g.b.add(
        vec3(width - 0.012, 2.9, depth * 0.5),
        vec3(0.01, 0.3, depth * 0.5),
        band,
        PLAIN,
    );
    let _ = brand;
    // Sprueche auf der Banderole, abwechselnd mit dem Ladennamen
    {
        let mut k = (seed % SLOGANS.len() as u64) as usize;
        let text_at = |g: &mut Gen, c: Vec3, yaw: f32, k: &mut usize| {
            let text = if *k % 3 == 0 {
                if tipped && g.weird.gen_bool(0.4) {
                    "KAUF NULL NULL NULL"
                } else {
                    BRAND
                }
            } else {
                let s = SLOGANS[*k % SLOGANS.len()];
                if tipped {
                    s.1
                } else {
                    s.0
                }
            };
            g.b.label(c, yaw, 3.2, 0.2, text, band, 0.0, 0.0, 0.0);
            *k += 1;
        };
        let mut x = 4.0;
        while x < dx0 - 2.5 {
            text_at(&mut g, vec3(x, 2.9, 0.03), 0.0, &mut k);
            x += 9.0;
        }
        let mut z = 4.0;
        while z < depth - 3.0 {
            text_at(&mut g, vec3(0.03, 2.9, z), FRAC_PI_2, &mut k);
            if z > DRINKS_D + 2.0 {
                text_at(&mut g, vec3(width - 0.03, 2.9, z), FRAC_PI_2, &mut k);
            }
            z += 9.0;
        }
    }

    // --- Regalgaenge: alle Gondeln folgen einer gemeinsamen S-Kurve (im verbogenen Markt) ---
    let curve_phase = g.weird.gen_range(0.0..std::f32::consts::TAU);
    let curve_waves = g.weird.gen_range(1.3..2.6);
    let amps: Vec<f32> = (0..aisle_count)
        .map(|_| 1.6 * params.warp * g.weird.gen_range(0.85..1.15))
        .collect();
    let center_x = |i: usize, z: f32| {
        aisles_x0
            + AISLE_PITCH * (i as f32 + 0.5)
            + amps[i] * (curve_phase + (z - aisle_z0) / aisle_len * curve_waves * PI).sin()
    };
    let mut passages: Vec<(Vec3, Vec3)> = Vec::new();
    // Bereiche wandern (ab Stufe "falsch"): die Tiefkuehlreihe steht in einem anderen Gang
    let freezer_aisle = if tipped && params.mix >= 2 {
        g.weird.gen_range(0..aisle_count) as usize
    } else {
        0
    };
    // Index der Regalgondel ohne die Tiefkuehlreihe (fuer die Seitenbelegung im Layout)
    let gondola = |iu: usize| if iu < freezer_aisle { iu + 1 } else { iu };
    for i in 0..aisle_count {
        let iu = i as usize;
        let freezer_row = iu == freezer_aisle;
        let n = if params.warp > 0.0 {
            (aisle_len / PASSAGE_WIDTH).ceil() as usize
        } else {
            1
        };
        let n = if freezer_row && params.warp <= 0.0 {
            4
        } else {
            n
        };
        let pts: Vec<Vec3> = (0..=n)
            .map(|k| {
                let z = aisle_z0 + aisle_len * k as f32 / n as f32;
                vec3(center_x(iu, z), 0.0, z)
            })
            .collect();
        let mut gaps: Vec<usize> = Vec::new();
        if params.warp > 0.0 && n > 6 && !freezer_row {
            let count = if g.weird.gen_bool((0.4 + 0.5 * params.warp as f64).min(1.0)) {
                g.weird.gen_range(1..=2)
            } else {
                0
            };
            for _ in 0..count {
                let k = g.weird.gen_range(2..n - 2);
                if !gaps.iter().any(|&o| o.abs_diff(k) < 3) {
                    gaps.push(k);
                }
            }
        }
        for &k in &gaps {
            let d = pts[k + 1] - pts[k];
            let f = Frame::new(pts[k], d.x.atan2(d.z));
            passages.push(((pts[k] + pts[k + 1]) * 0.5, f.normal));
        }
        // Abschnitte zusammenfassen, damit gerade Gondeln nicht in Mini-Stuecke zerfallen
        for k in (0..n).filter(|k| !gaps.contains(k)) {
            let (p0, p1) = (pts[k], pts[k + 1]);
            let d = p1 - p0;
            let len = d.length();
            let yaw = d.x.atan2(d.z);
            let f = Frame::new(p0, yaw);
            let mid = (p0 + p1) * 0.5;
            let t = ((mid.z - aisle_z0) / aisle_len).clamp(0.0, 0.999);
            let hl = len * 0.5 + 0.02;
            if freezer_row {
                // Kuehltruhen: niedrig, man sieht darueber hinweg
                let fl = &g.layout.freezer;
                let sub = fl[((t * fl.len() as f32) as usize).min(fl.len() - 1)];
                let white = if tipped {
                    vec3(0.6, 0.62, 0.62)
                } else {
                    vec3(0.9, 0.92, 0.94)
                };
                g.b.add_rot(
                    mid + vec3(0.0, 0.27, 0.0),
                    f.half(hl, 0.62, 0.27),
                    yaw,
                    white,
                    PLAIN,
                );
                for side in [-1.0f32, 1.0] {
                    g.b.add_rot(
                        f.pos(len * 0.5, side * 0.6, 0.7),
                        f.half(hl, 0.02, 0.17),
                        yaw,
                        white,
                        PLAIN,
                    );
                }
                g.b.add_rot(
                    mid + vec3(0.0, 0.7, 0.0),
                    f.half(hl, 0.015, 0.17),
                    yaw,
                    white * 0.9,
                    PLAIN,
                );
                g.b.add_rot(
                    mid + vec3(0.0, 0.545, 0.0),
                    f.half(hl, 0.6, 0.005),
                    yaw,
                    vec3(0.55, 0.68, 0.78),
                    PLAIN,
                );
                let flick = if tipped && g.weird.gen_bool(0.3) {
                    0.5
                } else {
                    0.0
                };
                for side in [-1.0f32, 1.0] {
                    g.b.add_rot_ex(
                        f.pos(len * 0.5, side * 0.59, 0.86),
                        f.half(hl, 0.01, 0.008),
                        yaw,
                        vec3(0.7, 0.9, 1.0),
                        EMISSIVE,
                        k as f32,
                        1.2,
                        flick,
                    );
                }
                g.b.solid_rot(
                    mid + vec3(0.0, 0.45, 0.0),
                    f.half(hl, GONDOLA_HALF_W + 0.02, 0.45),
                    yaw,
                );
                // Ware liegt in zwei Reihen in der Truhe
                for side in [-1.0f32, 1.0] {
                    let mut a = 0.05;
                    while a < len - 0.1 {
                        let Some(p) = g.pick(sub, true, 0.3, 0.5) else {
                            break;
                        };
                        let half = cat.products[p].half;
                        let w = half.x * 2.0 + 0.015;
                        let run = g.rng.gen_range(2..=5);
                        for _ in 0..run {
                            if a + w > len - 0.05 {
                                break;
                            }
                            for row in [0.15f32, 0.42] {
                                let pos = f.pos(a + w * 0.5, side * row, 0.55);
                                let jitter = g.weird.gen_range(-0.3..0.3);
                                g.place(p, pos, f.item_yaw(side) + jitter);
                            }
                            a += w;
                        }
                        a += 0.03;
                    }
                }
                continue;
            }
            g.b.add_rot(
                mid + vec3(0.0, 0.06, 0.0),
                f.half(hl, GONDOLA_HALF_W, 0.06),
                yaw,
                shelf_color * 0.6,
                PLAIN,
            );
            g.b.add_rot(
                mid + vec3(0.0, 1.05, 0.0),
                f.half(hl, 0.04, 1.0),
                yaw,
                shelf_color,
                PLAIN,
            );
            // Neben einem Durchgang den Kollisionskoerper etwas kuerzen: in Kurven ist die Luecke
            // sonst keilfoermig zu eng fuer Menschen (der Wagen passt weiterhin nicht durch)
            let cut_start = if k > 0 && gaps.contains(&(k - 1)) {
                PASSAGE_TRIM
            } else {
                0.0
            };
            let cut_end = if gaps.contains(&(k + 1)) {
                PASSAGE_TRIM
            } else {
                0.0
            };
            g.b.solid_rot(
                f.pos(len * 0.5 + (cut_start - cut_end) * 0.5, 0.0, 1.05),
                f.half(
                    hl - (cut_start + cut_end) * 0.5,
                    GONDOLA_HALF_W + 0.02,
                    1.05,
                ),
                yaw,
            );
            for side in [-1i8, 1] {
                let facing = side as f32;
                let side_idx =
                    (aisle_count as usize - 1 - gondola(iu)) * 2 + if side == 1 { 0 } else { 1 };
                // Die Seite nach Abteilungsgrenzen aufteilen (ein gerades Segment kann mehrere Abteilungen tragen)
                let (ts, te) = ((p0.z - aisle_z0) / aisle_len, (p1.z - aisle_z0) / aisle_len);
                let parts: Vec<(usize, f32, f32)> = g.layout.shelf_sides[side_idx]
                    .iter()
                    .filter(|x| x.1 < te && x.2 > ts)
                    .map(|x| {
                        (
                            x.0,
                            (x.1.max(ts) - ts) / (te - ts) * len,
                            (x.2.min(te) - ts) / (te - ts) * len,
                        )
                    })
                    .collect();
                let parts = if parts.is_empty() {
                    vec![(sub_at(&g.layout.shelf_sides[side_idx], t), 0.0, len)]
                } else {
                    parts
                };
                let mut spot = tipped && g.weird.gen_bool(0.12);
                for (sub, l0, l1) in parts {
                    g.shelf_side(&ShelfSpec {
                        frame: Frame::new(f.pos(l0, 0.0, 0.0), yaw),
                        len: l1 - l0,
                        front: facing * GONDOLA_HALF_W,
                        facing,
                        depth: 0.56,
                        boards: GONDOLA_BOARDS,
                        sub,
                        board_color: shelf_color * 0.8,
                        strip_color: strip,
                        trays: true,
                        spot,
                    });
                    spot = false;
                }
            }
        }
        // Kopfenden: Norden Paletten mit Kartons, Sueden Gitterbox mit Aktionsware
        let gx_n = center_x(iu, aisle_z0);
        let end_n = aisle_z0 - 0.55;
        g.b.add(
            vec3(gx_n, 0.07, end_n),
            vec3(0.55, 0.07, 0.4),
            vec3(0.6, 0.45, 0.28),
            PLAIN,
        );
        let layers = g.rng.gen_range(2..=4);
        for l in 0..layers {
            g.b.add(
                vec3(gx_n, 0.34 + l as f32 * 0.4, end_n),
                vec3(0.5, 0.19, 0.36),
                CARDBOARD * (0.85 + 0.05 * l as f32),
                PLAIN,
            );
        }
        g.b.solid(vec3(gx_n, 0.6, end_n), vec3(0.58, 0.6, 0.43));
        let gx_s = center_x(iu, aisle_z1);
        g.gitterbox(vec3(gx_s, 0.0, aisle_z1 + 0.6));
        // rotes "Aktion"-Haengeschild
        let red = if tipped {
            vec3(0.45, 0.05, 0.04)
        } else {
            vec3(0.85, 0.1, 0.08)
        };
        let aktion = if tipped && g.weird.gen_bool(0.35) {
            "ABGELAUFEN"
        } else {
            "AKTION"
        };
        g.b.label(
            vec3(gx_s, 2.55, aisle_z1 + 0.6),
            0.0,
            0.6,
            0.2,
            aktion,
            red,
            0.0,
            0.9,
            0.0,
        );
        // Haengeschild ueber dem Gang: Unterabteilung (im Chaos luegt es)
        let zs = aisle_z0 + 1.2;
        if freezer_row {
            g.b.label(
                vec3(center_x(iu, zs), 2.6, zs),
                0.0,
                0.9,
                0.15,
                "Tiefkühl",
                band,
                0.0,
                0.0,
                0.0,
            );
        } else {
            for side in [-1i8, 1] {
                let side_idx =
                    (aisle_count as usize - 1 - gondola(iu)) * 2 + if side == 1 { 0 } else { 1 };
                let sub = g.layout.sign_shelf[side_idx];
                let x = center_x(iu, zs) + side as f32 * (GONDOLA_HALF_W + 0.55);
                g.b.add(
                    vec3(x, 3.35, zs),
                    vec3(0.008, 0.65, 0.008),
                    vec3(0.4, 0.4, 0.4),
                    PLAIN,
                );
                g.b.label(
                    vec3(x, 2.6, zs),
                    0.0,
                    0.5,
                    0.12,
                    &cat.subs[sub].name,
                    band,
                    0.0,
                    0.0,
                    0.0,
                );
            }
        }
        g.b.add(
            vec3(gx_s, 3.3, aisle_z1 + 0.6),
            vec3(0.01, 0.55, 0.01),
            vec3(0.4, 0.4, 0.4),
            PLAIN,
        );
    }

    // --- Kuehlwand an der Nordwand (durchgehend, Abschnitte je Unterabteilung) ---
    let cooler_body = if tipped {
        vec3(0.2, 0.22, 0.24)
    } else {
        vec3(0.32, 0.35, 0.38)
    };
    let cold = if tipped {
        vec3(0.5, 0.75, 0.7)
    } else {
        vec3(0.8, 0.9, 1.0)
    };
    // Bereiche wandern (ab Stufe "unruhig"): die Kuehlabschnitte tauschen ihre Plaetze an der Wand
    let mut cooler_slots = cooler_xs.clone();
    if tipped && params.mix >= 1 {
        cooler_slots.shuffle(&mut *g.weird);
    }
    for k in 0..cooler_xs.len() {
        let x = cooler_slots[k];
        let xm = x + 1.2;
        g.b.add(
            vec3(xm, 1.15, 0.2),
            vec3(1.25, 1.15, 0.2),
            cooler_body,
            PLAIN,
        );
        g.b.add(
            vec3(xm, 2.2, 0.55),
            vec3(1.25, 0.1, 0.45),
            cooler_body,
            PLAIN,
        );
        g.b.add(
            vec3(xm, 0.1, 0.95),
            vec3(1.25, 0.1, 0.06),
            vec3(0.75, 0.77, 0.8),
            PLAIN,
        );
        // Kopfleiste mit Abteilungsschild (im Chaos luegt es)
        g.b.add(vec3(xm, 2.45, 0.97), vec3(1.22, 0.14, 0.02), band, PLAIN);
        let shown = g.layout.sign_cooler[k];
        g.b.label(
            vec3(xm, 2.45, 0.995),
            0.0,
            1.15,
            0.1,
            &cat.subs[shown].name,
            band,
            0.0,
            0.0,
            0.0,
        );
        let seed_l = xm * 2.3;
        let flicker = if tipped && g.weird.gen_bool(0.35) {
            0.6
        } else {
            0.0
        };
        g.b.add_ex(
            vec3(xm, 2.08, 0.92),
            vec3(1.2, 0.015, 0.03),
            cold,
            EMISSIVE,
            seed_l,
            1.4,
            flicker,
        );
        g.b.light(vec3(xm, 1.5, 1.4), 3.2, cold, 0.35, seed_l, flicker);
        g.b.solid(vec3(xm, 1.15, 0.5), vec3(1.25, 1.15, 0.5));
        let sub = g.layout.cooler[k];
        g.shelf_side(&ShelfSpec {
            frame: Frame::new(vec3(x - 0.04, 0.0, 0.0), FRAC_PI_2),
            len: 2.48,
            front: -1.0,
            facing: -1.0,
            depth: 0.6,
            boards: COOLER_BOARDS,
            sub,
            board_color: cooler_body * 1.5,
            strip_color: strip,
            trays: false,
            spot: false,
        });
    }

    // --- Obst & Gemuese im Eingangsbereich: Stufenregal an der Ostwand + Insel ---
    let pz0 = DRINKS_D + 1.4;
    let pz1 = checkout_z - 1.8;
    if pz1 - pz0 > 3.0 {
        let wood = if tipped {
            vec3(0.4, 0.33, 0.25)
        } else {
            vec3(0.78, 0.66, 0.5)
        };
        let len = pz1 - pz0;
        let f = Frame::new(vec3(width, 0.0, pz0), 0.0);
        for (perp, y) in [(-1.05f32, 0.55f32), (-0.65, 0.8), (-0.25, 1.05)] {
            g.b.add(
                f.pos(len * 0.5, perp, (y - 0.14) * 0.5),
                f.half(len * 0.5, 0.2, (y - 0.14) * 0.5),
                wood,
                PLAIN,
            );
            let sub = g.layout.produce_rack;
            g.crate_row(f, len, perp, y, -1.0, sub);
        }
        g.b.solid(f.pos(len * 0.5, -0.65, 0.55), f.half(len * 0.5, 0.65, 0.55));
        // Insel (beidseitig), Abstand zur Gondelreihe und zum Wandregal
        // Obst-Insel rueckt ab Stufe "falsch" an eine andere Stelle
        let (shift_x, shift_z, shorter): (f32, f32, f32) = if tipped && params.mix >= 2 {
            (
                g.weird.gen_range(-0.35..0.35),
                g.weird.gen_range(0.0..1.8),
                g.weird.gen_range(0.6..2.0),
            )
        } else {
            (0.0, 0.0, 0.0)
        };
        let ix = aisles_x1 + 4.7 + shift_x;
        let iz0 = pz0 + 0.8 + shift_z;
        let ilen = len - 1.6 - shorter.max(shift_z);
        if ilen > 2.0 {
            let fi = Frame::new(vec3(ix, 0.0, iz0), 0.0);
            for side in [-1.0f32, 1.0] {
                for (perp, y) in [(0.62f32, 0.55f32), (0.22, 0.75)] {
                    g.b.add(
                        fi.pos(ilen * 0.5, side * perp, (y - 0.14) * 0.5),
                        fi.half(ilen * 0.5, 0.2, (y - 0.14) * 0.5),
                        wood,
                        PLAIN,
                    );
                    let sub = g.layout.produce_island;
                    g.crate_row(fi, ilen, side * perp, y, side, sub);
                }
            }
            g.b.solid(fi.pos(ilen * 0.5, 0.0, 0.4), fi.half(ilen * 0.5, 0.85, 0.4));
        }
    }

    // Schild ueber dem Ausgang (innen) und ueber dem Obstregal
    let exit_text = if tipped {
        "Wer kauft, bleibt"
    } else {
        "Danke für Ihren Einkauf"
    };
    g.b.label(
        vec3(door_cx, 3.05, depth - 0.03),
        0.0,
        1.6,
        0.16,
        exit_text,
        band,
        0.0,
        0.0,
        0.0,
    );
    if pz1 - pz0 > 3.0 {
        let zc = (pz0 + pz1) * 0.5;
        g.b.add(
            vec3(width - 0.9, 3.4, zc),
            vec3(0.008, 0.6, 0.008),
            vec3(0.4, 0.4, 0.4),
            PLAIN,
        );
        g.b.label(
            vec3(width - 0.9, 2.7, zc),
            FRAC_PI_2,
            1.4,
            0.18,
            "Obst & Gemüse",
            vec3(0.16, 0.42, 0.2),
            0.0,
            0.0,
            0.0,
        );
    }

    // --- Kassen ---
    let spacing = (aisles_x1 - aisles_x0) / lanes_n as f32;
    let mut lanes = Vec::new();
    for i in 0..lanes_n {
        let cx = aisles_x0 + spacing * (i as f32 + 0.5);
        let z0 = checkout_z;
        let open = !tipped || i as usize == params.open_lane % lanes_n as usize;
        g.b.add_solid(
            vec3(cx, 0.45, z0 + 1.5),
            vec3(0.35, 0.45, 1.5),
            vec3(0.3, 0.32, 0.35),
        );
        g.b.add(
            vec3(cx, 0.906, z0 + 1.2),
            vec3(0.3, 0.006, 1.2),
            vec3(0.06, 0.06, 0.06),
            PLAIN,
        );
        // Scannerfenster am Bandende, Ablage dahinter
        g.b.add_ex(
            vec3(cx, 0.913, z0 + 2.55),
            vec3(0.2, 0.004, 0.09),
            if tipped {
                vec3(0.6, 0.05, 0.05)
            } else {
                vec3(0.9, 0.25, 0.2)
            },
            EMISSIVE,
            i as f32 * 3.1,
            if open { 1.1 } else { 0.15 },
            if tipped { 0.4 } else { 0.0 },
        );
        // Kassiererplatz (Stuhl + Person sind Modelle) und Schrank mit Zigaretten-Rollo
        g.b.solid(vec3(cx + 0.85, 0.6, z0 + 2.4), vec3(0.45, 0.6, 0.65));
        g.b.add_solid(
            vec3(cx + 1.42, 0.5, z0 + 2.4),
            vec3(0.12, 0.5, 0.68),
            vec3(0.22, 0.23, 0.26),
        );
        // Kartenterminal auf einer Stange (Kundenseite, am Scanner)
        g.b.add(
            vec3(cx - 0.36, 1.0, z0 + 2.35),
            vec3(0.012, 0.1, 0.012),
            vec3(0.2, 0.2, 0.22),
            PLAIN,
        );
        // Warentrenner mit Ladennamen auf der Ablageleiste
        for k in 0..2 {
            g.b.label(
                vec3(cx + 0.27, 0.935, z0 + 0.35 + k as f32 * 0.45),
                FRAC_PI_2,
                0.19,
                0.022,
                BRAND,
                vec3(0.45, 0.47, 0.5),
                0.0,
                0.0,
                0.0,
            );
        }
        g.b.add_solid(
            vec3(cx - 1.25, 0.6, z0 + 1.0),
            vec3(0.2, 0.6, 0.6),
            vec3(0.3, 0.3, 0.32),
        );
        g.b.add(
            vec3(cx - 0.3, 1.4, z0 + 0.1),
            vec3(0.02, 0.5, 0.02),
            vec3(0.4, 0.4, 0.4),
            PLAIN,
        );
        let kasse = if tipped {
            "0 Kasse".to_string()
        } else {
            format!("{} Kasse", i + 1)
        };
        if open {
            g.b.label(
                vec3(cx - 0.3, 1.97, z0 + 0.1),
                0.0,
                0.3,
                0.08,
                &kasse,
                vec3(0.1, 0.55, 0.22),
                0.0,
                1.3,
                0.0,
            );
            if tipped {
                g.b.light(
                    vec3(cx - 0.4, 1.6, z0 + 1.5),
                    3.5,
                    vec3(0.3, 1.0, 0.4),
                    0.45,
                    0.0,
                    0.0,
                );
            }
        } else {
            g.b.label(
                vec3(cx - 0.3, 1.97, z0 + 0.1),
                0.0,
                0.3,
                0.08,
                &kasse,
                vec3(0.22, 0.24, 0.24),
                0.0,
                0.0,
                0.0,
            );
        }
        // Quengelware neben dem Band
        let suess = cat.sub_index("suess");
        let qf = Frame::new(vec3(cx - 1.25, 0.0, z0 + 0.42), 0.0);
        for &y in &[0.62f32, 0.92, 1.2] {
            g.b.add(
                qf.pos(0.6, 0.0, y - 0.01),
                qf.half(0.6, 0.2, 0.01),
                vec3(0.4, 0.4, 0.42),
                PLAIN,
            );
            let mut a = 0.02;
            while a < 1.15 {
                let Some(p) = g.pick(suess, true, 0.25, 0.35) else {
                    break;
                };
                let w = cat.products[p].half.x * 2.0 + 0.01;
                if a + w > 1.18 {
                    break;
                }
                g.place(p, qf.pos(a + w * 0.5, 0.05, y), qf.item_yaw(1.0));
                a += w;
            }
        }
        // Ueberwachungsmonitor hinter dem Scanner: zeigt das Bild der Deckenkamera
        let monitor = vec3(cx - 0.75, 2.45, z0 + 3.3);
        g.b.add(
            monitor + vec3(0.0, 0.0, 0.17),
            vec3(0.3, 0.23, 0.16),
            vec3(0.16, 0.16, 0.17),
            PLAIN,
        );
        g.b.add(
            monitor + vec3(0.0, (CEILING_HEIGHT - monitor.y) * 0.5 + 0.11, 0.17),
            vec3(0.02, (CEILING_HEIGHT - monitor.y) * 0.5 - 0.11, 0.02),
            vec3(0.35, 0.35, 0.36),
            PLAIN,
        );
        g.b.add_ex(
            monitor,
            vec3(0.25, 0.185, 0.008),
            vec3(0.05, 0.06, 0.05),
            MONITOR,
            i as f32,
            1.2,
            0.0,
        );
        lanes.push(Lane {
            scan_point: vec3(cx - 0.7, 0.0, z0 + 2.4),
            register: vec3(cx + 0.12, 0.91, z0 + 2.75),
            open,
            belt_start: vec3(cx, 0.912, z0 + 0.15),
            belt_end: vec3(cx, 0.912, z0 + 2.32),
            bag: vec3(cx - 0.12, 0.906, z0 + 2.85),
            terminal: vec3(cx - 0.36, 1.1, z0 + 2.35),
            seat: vec3(cx + 0.8, 0.0, z0 + 2.5),
            shutter: vec3(cx + 1.32, 1.0, z0 + 2.4),
            camera: vec3(cx + 0.4, 3.78, z0 + 1.2),
            monitor,
        });
    }

    // --- Getraenkeabteilung (Nordost-Ecke) ---
    let drinks_door_cx = dx0 + 1.8;
    let (dd0, dd1) = (drinks_door_cx - 1.2, drinks_door_cx + 1.2);
    g.b.wall(dx0, 0.0, dx0, DRINKS_D, 0.0, h, wall_color);
    g.b.wall(dx0, DRINKS_D, dd0, DRINKS_D, 0.0, h, wall_color);
    g.b.wall(dd1, DRINKS_D, width, DRINKS_D, 0.0, h, wall_color);
    g.b.wall(dd0, DRINKS_D, dd1, DRINKS_D, 2.6, h, wall_color);
    let sign_c = if tipped {
        vec3(0.15, 0.3, 0.55)
    } else {
        vec3(0.2, 0.45, 0.95)
    };
    g.b.label(
        vec3(drinks_door_cx, 3.0, DRINKS_D + WALL_T + 0.02),
        0.0,
        1.1,
        0.2,
        "Getränke & Pfand",
        sign_c,
        0.0,
        1.2,
        0.0,
    );
    // Getraenkeregal an der Nordwand des Raums
    let gr_len = width - dx0 - 0.8;
    g.b.add(
        vec3(dx0 + 0.4 + gr_len * 0.5, 1.05, 0.06),
        vec3(gr_len * 0.5, 1.0, 0.04),
        shelf_color,
        PLAIN,
    );
    g.b.solid(
        vec3(dx0 + 0.4 + gr_len * 0.5, 1.05, 0.3),
        vec3(gr_len * 0.5, 1.05, 0.3),
    );
    let getraenke = cat.sub_index("getraenke");
    g.shelf_side(&ShelfSpec {
        frame: Frame::new(vec3(dx0 + 0.4, 0.0, 0.0), FRAC_PI_2),
        len: gr_len,
        front: -0.6,
        facing: -1.0,
        depth: 0.55,
        boards: GONDOLA_BOARDS,
        sub: getraenke,
        board_color: shelf_color * 0.8,
        strip_color: strip,
        trays: true,
        spot: false,
    });
    let mut z = 1.6;
    while z < DRINKS_D - 1.2 {
        let mut x = dx0 + 1.2;
        while x < width - 0.8 {
            let near_door = x < dx0 + 3.6 && z > DRINKS_D - 2.6;
            if !near_door && !drinks_rng.gen_bool(0.2) {
                g.b.add(
                    vec3(x, 0.07, z),
                    vec3(0.5, 0.07, 0.4),
                    vec3(0.6, 0.45, 0.28),
                    PLAIN,
                );
                let crate_c = CRATE_COLORS[drinks_rng.gen_range(0..CRATE_COLORS.len())];
                let mut top: f32 = 0.0;
                for (ox, oz) in [(-0.21, -0.17), (0.21, -0.17), (-0.21, 0.17), (0.21, 0.17)] {
                    let n = drinks_rng.gen_range(2..=5);
                    for k in 0..n {
                        let y = 0.14 + 0.13 + k as f32 * 0.27;
                        let tint = drinks_rng.gen_range(0.85..1.05);
                        g.b.add(
                            vec3(x + ox, y, z + oz),
                            vec3(0.19, 0.13, 0.16),
                            crate_c * tint,
                            PLAIN,
                        );
                    }
                    let y_top = 0.14 + n as f32 * 0.27;
                    g.b.add(
                        vec3(x + ox, y_top + 0.03, z + oz),
                        vec3(0.17, 0.03, 0.14),
                        crate_c * 1.3,
                        PLAIN,
                    );
                    top = top.max(y_top);
                }
                g.b.solid(vec3(x, top * 0.5, z), vec3(0.5, top * 0.5, 0.4));
            }
            x += 1.5;
        }
        z += 1.7;
    }
    // Aktionstisch mit Wasser (Ziel des ersten Zettelpunkts): billig links, teuer rechts
    let table_c = vec3(width - 0.7, 0.45, 6.2);
    g.b.add_solid(table_c, vec3(0.35, 0.45, 0.55), vec3(0.55, 0.42, 0.3));
    g.b.add_ex(
        vec3(width - 0.4, 1.6, 6.2),
        vec3(0.02, 0.25, 0.5),
        vec3(0.95, 0.85, 0.2),
        EMISSIVE,
        9.0,
        0.8,
        0.0,
    );
    if let Some(water) = cat.list_index("Mineralwasser") {
        let entry = &cat.list[water];
        for (variant, z0) in [(entry.cheap[0], 5.7f32), (entry.pricey[0], 6.25)] {
            let half = cat.products[variant].half;
            let step = half.x * 2.0 + 0.01;
            let cols = ((0.5 / step) as usize).max(1);
            for ci in 0..cols {
                for xi in 0..3 {
                    let pos = vec3(
                        width - 0.95 + xi as f32 * 0.22,
                        0.9,
                        z0 + (ci as f32 + 0.5) * step,
                    );
                    g.items.push(ShelfItem {
                        product: variant as u32,
                        pos,
                        yaw: -FRAC_PI_2,
                        fake: false,
                        taken: false,
                        rot: 0,
                    });
                }
            }
        }
    }
    for lx in [dx0 + 1.75, dx0 + 5.25] {
        for lz in [1.75, 5.25] {
            g.ceiling_lamp(lx, lz, lamp_color);
        }
    }

    // --- Lichtbaender ueber den Gaengen ---
    let mut xs: Vec<f32> = (0..=aisle_count)
        .map(|i| aisles_x0 + i as f32 * AISLE_PITCH)
        .collect();
    let mut ex = aisles_x1 + 2.0;
    while ex < width - 0.5 {
        xs.push(ex);
        ex += 3.0;
    }
    let mut lz = 2.0;
    while lz < depth - 0.5 {
        for &lx in &xs {
            if lx > dx0 - 0.3 && lz < DRINKS_D + 0.3 {
                continue;
            }
            g.ceiling_lamp(lx, lz, lamp_color);
        }
        lz += 3.0;
    }

    // --- Neue Querwaende in Gaengen ---
    for _ in 0..params.walls {
        if aisle_count < 3 {
            break;
        }
        let j = g.weird.gen_range(2..aisle_count) as usize;
        let wz = g.weird.gen_range(aisle_z0 + 2.0..aisle_z1 - 2.0);
        let xa = center_x(j - 1, wz);
        let xb = center_x(j, wz);
        g.b.add_solid(
            vec3((xa + xb) * 0.5, 1.4, wz),
            vec3(((xb - xa) * 0.5).abs(), 1.4, 0.1),
            wall_color * 0.9,
        );
    }

    // --- Wagenbox am Eingang ---
    let corral = vec3(door_x1 + 1.8, 0.0, depth - 1.0);
    g.b.solid(corral + vec3(1.05, 0.45, 0.0), vec3(0.6, 0.45, 0.45));
    g.b.add(
        corral + vec3(0.0, 0.01, 0.0),
        vec3(1.7, 0.01, 0.6),
        vec3(0.2, 0.35, 0.7),
        PLAIN,
    );
    g.b.add_ex(
        corral + vec3(0.7, 2.3, 0.9),
        vec3(0.8, 0.18, 0.02),
        vec3(0.25, 0.5, 0.95),
        EMISSIVE,
        7.0,
        1.0,
        if tipped { 0.4 } else { 0.0 },
    );

    // --- Werbeposter (verrotten im gekippten Markt, siehe Shader) ---
    let mut poster_seed = 0.0f32;
    let mut poster = |b: &mut Builder, c: Vec3, h: Vec3| {
        let col = PALETTE[(poster_seed as usize * 3 + seed as usize) % PALETTE.len()];
        b.add_ex(c, h, col, POSTER, poster_seed, 0.0, 0.0);
        // Schriftzug oben auf dem Plakat
        let texts: [&str; 2] = if tipped {
            ["Seit immer frisch", "Wir sehen Sie"]
        } else {
            ["Heute frisch", "AKTION"]
        };
        let text = texts[poster_seed as usize % 2];
        let (lc, yaw, hw) = if h.x < h.z {
            (
                vec3(c.x + 0.008, c.y + h.y * 0.72, c.z),
                FRAC_PI_2,
                h.z * 0.85,
            )
        } else {
            (vec3(c.x, c.y + h.y * 0.72, c.z - 0.008), 0.0, h.x * 0.85)
        };
        b.label(
            lc,
            yaw,
            hw,
            0.09,
            text,
            vec3(0.96, 0.94, 0.88),
            4.0,
            0.0,
            0.0,
        );
        poster_seed += 1.0;
    };
    let mut pz = aisle_z0 + 1.5;
    while pz < aisle_z1 - 1.0 {
        poster(&mut g.b, vec3(0.012, 1.75, pz), vec3(0.004, 0.7, 0.5));
        pz += 7.0;
    }
    let mut px = 2.0;
    while px < door_x0 - 1.5 {
        poster(
            &mut g.b,
            vec3(px, 1.9, depth - 0.012),
            vec3(0.5, 0.7, 0.004),
        );
        px += 6.0;
    }

    // --- Verfall: Loecher, verdorbene Ware, Riesenwaren (nur gekippt, je nach Stufe) ---
    {
        let mut keep_free = vec![
            vec3(door_cx, 0.0, depth - 1.6),
            vec3(door_cx, 0.0, depth),
            vec3(drinks_door_cx, 0.0, DRINKS_D + 0.5),
            corral,
        ];
        keep_free.extend(lanes.iter().map(|l| l.scan_point));
        let area = DecayArea {
            x0: 1.0,
            x1: aisles_x1 + 1.0,
            z0: aisle_z0 + 0.5,
            z1: aisle_z1 - 0.5,
            width,
            depth,
            keep_free,
        };
        g.decay(&area);
        g.floor_and_ceiling_decay(&area, floor_c);
    }

    // Warnschilder "Rutschgefahr"
    let mut signs = Vec::new();
    let sign_count = if tipped { 4 } else { 1 };
    for _ in 0..sign_count {
        let i = g.rng.gen_range(1..=aisle_count) as f32;
        let sx = aisles_x0 + i * AISLE_PITCH;
        let sz = g.rng.gen_range(aisle_z0 + 1.0..aisle_z1 - 1.0);
        signs.push((
            vec3(sx, 0.0, sz),
            g.rng.gen_range(0.0..std::f32::consts::TAU),
        ));
    }

    let (fog_color, fog_density) = if tipped {
        (vec3(0.03, 0.035, 0.03), 0.045 + params.darkness * 0.03)
    } else {
        (vec3(0.75, 0.74, 0.7), 0.012)
    };

    let Gen {
        b,
        items,
        spots,
        layout,
        giants,
        ant_trails,
        rat_runs,
        drips,
        sparks,
        ceiling_holes,
        ..
    } = g;
    // Das Navigationsraster reicht bis ans Ende des Korridors (Verfolger rennen hinterher)
    let nav_depth = corridor.map_or(depth, |c| c.z1 + 0.5);
    let nav = build_nav(&b.solids, &b.obbs, width, nav_depth);
    let mut world = World {
        instances: b.instances,
        lights: b.lights,
        solids: b.solids,
        obbs: b.obbs,
        passages,
        items,
        item_grid: HashMap::new(),
        layout,
        drinks_zone: Aabb {
            min: vec3(dx0 + WALL_T, 0.0, 0.0),
            max: vec3(width, h, DRINKS_D - WALL_T),
        },
        spawn: vec3(door_cx, 0.0, depth - 1.6),
        spawn_yaw: 0.0,
        size: vec3(width, h, depth),
        fog_color,
        fog_density,
        aisle_count,
        aisle_z0,
        aisle_z1,
        aisle_x1: aisles_x1,
        lanes,
        door_center: vec3(door_cx, 0.0, depth),
        exit_zone,
        corridor,
        spots,
        nav,
        signs,
        giants,
        rotten: Vec::new(),
        ant_trails,
        rat_runs,
        drips,
        sparks,
        ceiling_holes,
        corral,
    };
    world.index_items();
    // In den Kassenbereich kommen keine NPCs (der Spieler schon)
    world.block_checkout_for_npcs();
    // Neue Waende duerfen nichts Wichtiges unerreichbar machen
    // Neue Waende, Loecher und Riesenwaren duerfen nichts Wichtiges unerreichbar machen
    if !world.all_reachable() {
        let mut p = params.clone();
        if p.walls > 0 {
            p.walls -= 1;
        } else if p.holes > 0 {
            p.holes -= 1;
        } else if p.giants > 0 {
            p.giants -= 1;
        } else if p.decay > 0 {
            p.decay -= 1;
        } else {
            return world;
        }
        return generate(&p);
    }
    world
}
