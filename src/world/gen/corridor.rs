//! Der Fluchtkorridor hinter der Ladentuer. Es gibt ihn nur im gekippten Markt, wenn nach dem
//! Bezahlen die Tuer offen ist: ein langer, enger, niedriger Gang mit rot flackernden Roehren,
//! Hindernissen im Slalom, Postern ("NICHT UMDREHEN") und am Ende einem grossen Spiegel, in dem
//! man sieht, was hinter einem her ist. Der Ausgang liegt kurz vor dem Spiegel links (+x).

use super::*;

/// Laenge des Gangs (ab der Ladentuer)
pub const CORRIDOR_LEN: f32 = 42.0;
/// Deckenhoehe im Gang (niedriger als im Markt)
pub const CORRIDOR_H: f32 = 2.7;
/// Spiegel am Ende: halbe Groesse (16:9 wie das Kamerabild) und Hoehe der Mitte
pub const MIRROR_HALF: Vec3 = Vec3::new(1.1, 0.62, 0.01);
pub const MIRROR_Y: f32 = 1.2;
/// Ausgang: Oeffnung in der linken Wand (+x), so weit vor dem Ende
const EXIT_FROM: f32 = 2.6;
const EXIT_TO: f32 = 0.5;
/// Tiefe des Ausgangsraums
const EXIT_DEPTH: f32 = 3.4;

/// Texte der Poster an den Waenden (Reihenfolge = Abstand zur Tuer)
pub const POSTER_TEXTS: &[&str] = &[
    "NICHT UMDREHEN",
    "SIE SIND DIREKT HINTER DIR",
    "DREH DICH NICHT UM",
    "NUR NACH VORNE",
    "SCHAU NICHT ZURÜCK",
    "KUNDE 0 HAT EIN GESICHT",
    "NICHT UMDREHEN",
    "SIEH IN DEN SPIEGEL",
];

/// Was das Spiel ueber den Gang wissen muss.
#[derive(Clone, Copy, Debug)]
pub struct Corridor {
    /// Innenkanten der Seitenwaende
    pub x0: f32,
    pub x1: f32,
    /// Ladentuer und Ende (Spiegelwand)
    pub z0: f32,
    pub z1: f32,
    /// Spiegelflaeche (zeigt nach -z, zum Spieler)
    pub mirror_center: Vec3,
    pub mirror_half: Vec3,
}

impl Corridor {
    /// Ist der Punkt im Gang (zwischen Tuer und Spiegelwand)?
    pub fn contains(&self, p: Vec3) -> bool {
        p.x > self.x0 - 0.2 && p.x < self.x1 + 0.2 && p.z > self.z0 && p.z < self.z1
    }
}

/// Baut Gang, Hindernisse, Licht, Poster, Spiegel und Ausgangsraum. Liefert den Gang und den
/// Bereich, in dem man draussen ist (gewonnen).
/// `seed`: Zufall fuer Hindernisse und Blutspuren (bei jedem Bezahlen neu, siehe WorldParams).
pub(super) fn build(b: &mut Builder, seed: u64, x0: f32, x1: f32, z0: f32) -> (Corridor, Aabb) {
    let mut rng = Pcg64::seed_from_u64(seed ^ 0xC0_221D_0);
    let rng = &mut rng;
    let cx = (x0 + x1) * 0.5;
    let hw = (x1 - x0) * 0.5;
    let z1 = z0 + CORRIDOR_LEN;
    let zc = (z0 + z1) * 0.5;
    let h = CORRIDOR_H;
    let wall_c = vec3(0.2, 0.24, 0.2);
    let low_c = vec3(0.1, 0.11, 0.09);

    // Boden (schmutzig), Decke (niedrig, fleckig)
    b.add(
        vec3(cx, -0.05, zc),
        vec3(hw + WALL_T, 0.05, CORRIDOR_LEN * 0.5),
        vec3(0.17, 0.16, 0.14),
        FLOOR,
    );
    b.add(
        vec3(cx, h + 0.05, zc),
        vec3(hw + WALL_T, 0.05, CORRIDOR_LEN * 0.5),
        vec3(0.22, 0.22, 0.2),
        CEILING,
    );
    // rechte Wand (-x) ganz, linke Wand (+x) mit Ausgangsoeffnung kurz vor dem Ende
    b.wall(x0 - WALL_T, z0, x0 - WALL_T, z1, 0.0, h, wall_c);
    let (ea, eb) = (z1 - EXIT_FROM, z1 - EXIT_TO);
    b.wall(x1 + WALL_T, z0, x1 + WALL_T, ea, 0.0, h, wall_c);
    b.wall(x1 + WALL_T, eb, x1 + WALL_T, z1, 0.0, h, wall_c);
    b.add(
        vec3(x1 + WALL_T, (2.2 + h) * 0.5, (ea + eb) * 0.5),
        vec3(WALL_T, (h - 2.2) * 0.5, (eb - ea) * 0.5),
        wall_c,
        PLAIN,
    );
    // Spiegelwand am Ende
    b.wall(
        x0 - WALL_T,
        z1 + WALL_T,
        x1 + WALL_T,
        z1 + WALL_T,
        0.0,
        h,
        wall_c,
    );
    // dunkler Sockel an beiden Seiten
    for (x, s) in [(x0 + 0.004, 1.0f32), (x1 - 0.004, -1.0)] {
        let len = if s > 0.0 { CORRIDOR_LEN } else { ea - z0 };
        b.add(
            vec3(x, 0.5, z0 + len * 0.5),
            vec3(0.004, 0.5, len * 0.5),
            low_c,
            PLAIN,
        );
    }

    // Roehren: rot, flackernd, manche tot
    let mut z = z0 + 2.5;
    let mut k = 0;
    while z < z1 - 1.0 {
        let dead = k % 4 == 2;
        let col = vec3(1.0, 0.22, 0.16);
        b.add_ex(
            vec3(cx, h - 0.03, z),
            vec3(0.08, 0.02, 0.6),
            if dead { vec3(0.15, 0.1, 0.1) } else { col },
            EMISSIVE,
            k as f32,
            if dead { 0.0 } else { 1.2 },
            0.7,
        );
        if !dead {
            b.light(vec3(cx, h - 0.3, z), 6.5, col, 0.9, k as f32 * 1.7, 0.7);
        }
        z += 5.0;
        k += 1;
    }

    // Blutspuren an den Waenden
    for _ in 0..7 {
        let zz = rng.gen_range(z0 + 3.0..z1 - 4.0);
        let side = rng.gen_bool(0.5);
        let x = if side { x0 + 0.006 } else { x1 - 0.006 };
        b.add(
            vec3(x, rng.gen_range(0.4..1.9), zz),
            vec3(0.003, rng.gen_range(0.1..0.35), rng.gen_range(0.06..0.4)),
            vec3(0.3, 0.02, 0.02),
            PLAIN,
        );
    }

    // Hindernisse: bei jedem Lauf neu gewuerfelt (Lage der Luecke, Art, Abstand). In jeder
    // Reihe bleibt eine Luecke von mindestens 1,25 m (Spieler und Verfolger kommen durch).
    let mut z = z0 + rng.gen_range(4.0..6.5);
    while z < z1 - EXIT_FROM - 3.0 {
        let gap_w = rng.gen_range(1.25..1.7);
        let gap_c = rng.gen_range(x0 + gap_w * 0.5..x1 - gap_w * 0.5);
        let (ga, gb) = (gap_c - gap_w * 0.5, gap_c + gap_w * 0.5);
        for (bx0, bx1) in [(x0, ga), (gb, x1)] {
            if bx1 - bx0 > 0.3 {
                let zz = z + rng.gen_range(-0.4..0.4);
                let kind = rng.gen_range(0..4);
                obstacle(b, rng, bx0, bx1, zz, kind);
            }
        }
        // manchmal liegt noch Kleinkram herum (nicht im Weg, nur Stimmung)
        if rng.gen_bool(0.5) {
            let p = vec3(
                rng.gen_range(x0 + 0.2..x1 - 0.2),
                0.03,
                z + rng.gen_range(1.4..2.2),
            );
            b.add_floor(
                p,
                vec3(rng.gen_range(0.05..0.2), 0.03, rng.gen_range(0.05..0.15)),
                rng.gen_range(0.0..PI),
                [
                    vec3(0.55, 0.06, 0.05),
                    vec3(0.85, 0.82, 0.75),
                    vec3(0.4, 0.3, 0.18),
                ][rng.gen_range(0..3)],
                PLAIN,
            );
        }
        z += rng.gen_range(3.4..5.6);
    }

    // Poster: schwarz-rot, leicht leuchtend; dazwischen verrottende Werbeposter
    let mut z = z0 + 3.5;
    let mut i = 0;
    while z < z1 - 3.5 {
        let on_right = i % 2 == 0;
        // rechte Wand (-x) zeigt nach +x, linke Wand (+x) nach -x
        let (x, yaw) = if on_right {
            (x0 + 0.012, FRAC_PI_2)
        } else {
            (x1 - 0.012, -FRAC_PI_2)
        };
        if !(z > z1 - EXIT_FROM - 0.6 && !on_right) {
            let text = POSTER_TEXTS[i % POSTER_TEXTS.len()];
            b.add(
                vec3(x, 1.65, z),
                vec3(0.004, 0.42, 0.72),
                vec3(0.08, 0.01, 0.01),
                PLAIN,
            );
            b.label(
                vec3(x + if on_right { 0.004 } else { -0.004 }, 1.65, z),
                yaw,
                0.66,
                0.16,
                text,
                vec3(0.25, 0.02, 0.02),
                0.0,
                0.55,
                0.25,
            );
            // dazwischen ein verrottendes Maskottchen-Plakat
            if i % 3 == 1 {
                let pz = z + 2.6;
                if pz < z1 - EXIT_FROM - 1.0 {
                    let px = if on_right { x1 - 0.012 } else { x0 + 0.012 };
                    b.add_ex(
                        vec3(px, 1.6, pz),
                        vec3(0.004, 0.6, 0.45),
                        vec3(0.85, 0.75, 0.55),
                        POSTER,
                        40.0 + i as f32,
                        0.0,
                        0.0,
                    );
                }
            }
        }
        z += 5.5;
        i += 1;
    }

    // Spiegel am Ende, mit dunklem Rahmen und einem Schild darueber
    let mirror_center = vec3(cx, MIRROR_Y, z1 - 0.04);
    b.add(
        vec3(cx, MIRROR_Y, z1 - 0.02),
        MIRROR_HALF + vec3(0.08, 0.08, 0.01),
        vec3(0.06, 0.05, 0.04),
        PLAIN,
    );
    b.add_ex(
        mirror_center,
        MIRROR_HALF,
        vec3(1.0, 1.0, 1.0),
        MIRROR,
        0.0,
        1.0,
        0.0,
    );
    b.label(
        vec3(cx, MIRROR_Y + MIRROR_HALF.y + 0.2, z1 - 0.03),
        PI,
        0.9,
        0.11,
        "SIEH IN DEN SPIEGEL",
        vec3(0.1, 0.01, 0.01),
        4.0,
        0.4,
        0.3,
    );

    // Ausgangsraum links vom Ende: grelles Licht, AUSGANG
    let (rx0, rx1) = (x1 + WALL_T * 2.0, x1 + EXIT_DEPTH);
    let (rz0, rz1) = (z1 - EXIT_FROM - 0.3, z1);
    let rxc = (rx0 + rx1) * 0.5;
    let rzc = (rz0 + rz1) * 0.5;
    b.add(
        vec3(rxc, -0.05, rzc),
        vec3((rx1 - rx0) * 0.5 + WALL_T, 0.05, (rz1 - rz0) * 0.5),
        vec3(0.5, 0.5, 0.5),
        FLOOR,
    );
    b.add(
        vec3(rxc, h + 0.05, rzc),
        vec3((rx1 - rx0) * 0.5 + WALL_T, 0.05, (rz1 - rz0) * 0.5),
        vec3(0.9, 0.9, 0.9),
        CEILING,
    );
    b.wall(x1, rz0 - WALL_T, rx1, rz0 - WALL_T, 0.0, h, wall_c);
    b.wall(x1, rz1 + WALL_T, rx1, rz1 + WALL_T, 0.0, h, wall_c);
    b.add_ex(
        vec3(rx1 + 0.05, h * 0.5, rzc),
        vec3(0.05, h * 0.5, (rz1 - rz0) * 0.5),
        vec3(0.95, 0.97, 1.0),
        EMISSIVE,
        0.0,
        2.0,
        0.0,
    );
    b.solid(
        vec3(rx1 + 0.15, h * 0.5, rzc),
        vec3(0.1, h * 0.5, (rz1 - rz0) * 0.5),
    );
    b.light(
        vec3(rxc, h - 0.4, rzc),
        7.0,
        vec3(0.9, 0.95, 1.0),
        1.4,
        0.0,
        0.0,
    );
    b.label(
        vec3(x1 - 0.02, 2.4, (ea + eb) * 0.5),
        -FRAC_PI_2,
        0.8,
        0.14,
        "AUSGANG",
        vec3(0.05, 0.35, 0.12),
        0.0,
        1.2,
        0.0,
    );

    let corridor = Corridor {
        x0,
        x1,
        z0,
        z1,
        mirror_center,
        mirror_half: MIRROR_HALF,
    };
    let exit = Aabb {
        min: vec3(x1 + 1.0, 0.0, rz0),
        max: vec3(rx1, h, rz1),
    };
    (corridor, exit)
}

/// Ein Hindernis zwischen `bx0` und `bx1` (quer) bei `z`: 0 Kistenstapel, 1 umgekipptes Regal,
/// 2 Palette mit Tomatensaft, 3 Kartonsaeule.
fn obstacle(b: &mut Builder, rng: &mut Pcg64, bx0: f32, bx1: f32, z: f32, kind: u32) {
    let c = vec3((bx0 + bx1) * 0.5, 0.0, z);
    let half_w = (bx1 - bx0) * 0.5;
    match kind {
        0 => {
            let hh = vec3(half_w, 0.55, 0.4);
            b.solid(c + vec3(0.0, hh.y, 0.0), hh);
            let n = ((bx1 - bx0) / 0.8).ceil().max(1.0) as usize;
            let w = half_w / n as f32;
            for i in 0..n {
                let x = bx0 + w * (2 * i + 1) as f32;
                let tall = rng.gen_bool(0.6);
                for level in 0..(1 + tall as usize) {
                    b.add_rot(
                        vec3(
                            x,
                            0.27 + 0.55 * level as f32,
                            z + rng.gen_range(-0.08..0.08),
                        ),
                        vec3(w * 0.95, 0.27, 0.38),
                        rng.gen_range(-0.18..0.18),
                        vec3(0.45, 0.32, 0.18) * rng.gen_range(0.75..1.1),
                        PLAIN,
                    );
                }
            }
        }
        1 => {
            let hh = vec3(half_w, 0.35, 0.28);
            b.add_solid(c + vec3(0.0, hh.y, 0.0), hh, vec3(0.3, 0.31, 0.33));
            b.add(
                c + vec3(0.0, 0.72, 0.0),
                vec3(half_w, 0.02, 0.3),
                vec3(0.95, 0.8, 0.2),
                PLAIN,
            );
        }
        2 => {
            let hh = vec3(half_w, 0.45, 0.45);
            b.solid(c + vec3(0.0, hh.y, 0.0), hh);
            b.add(
                c + vec3(0.0, 0.07, 0.0),
                vec3(half_w, 0.07, 0.45),
                vec3(0.5, 0.4, 0.25),
                PLAIN,
            );
            b.add(
                c + vec3(0.0, 0.5, 0.0),
                vec3(half_w * 0.95, 0.36, 0.42),
                vec3(0.55, 0.06, 0.05),
                PLAIN,
            );
        }
        _ => {
            // Kartonsaeule, an einer zufaelligen Stelle im Block
            let pw = half_w.min(0.3);
            let px = rng.gen_range(bx0 + pw..=bx1 - pw);
            let hh = vec3(pw, 0.9, 0.3);
            b.add_solid(vec3(px, hh.y, z), hh, vec3(0.42, 0.3, 0.17));
        }
    }
}
