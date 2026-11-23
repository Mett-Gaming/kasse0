//! Rohbau des Korridors entlang des gewuerfelten Wegs: Waende (mit Eckstuecken), Boden, Decke,
//! rot flackernde Roehren, Blutspuren, Poster, Spiegel am Ende und der Ausgangsraum seitlich.

use super::*;

/// Lokaler Rahmen eines geraden Stuecks: Ursprung, Richtung `d`, Seite `n` (achsparallel).
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub o: Vec3,
    pub d: Vec3,
    pub n: Vec3,
}

impl Frame {
    pub fn new(o: Vec3, d: Vec3) -> Self {
        Self {
            o,
            d,
            n: side_of(d),
        }
    }
    /// Punkt bei `u` (entlang), `v` (quer), Hoehe `y`.
    pub fn at(&self, u: f32, v: f32, y: f32) -> Vec3 {
        self.o + self.d * u + self.n * v + vec3(0.0, y, 0.0)
    }
    /// Halbe Groesse einer achsparallelen Box: `hu` entlang, `hv` quer, `hy` hoch.
    pub fn half(&self, hu: f32, hv: f32, hy: f32) -> Vec3 {
        if self.d.x.abs() > 0.5 {
            vec3(hu, hy, hv)
        } else {
            vec3(hv, hy, hu)
        }
    }
    /// Drehung (Gierwinkel) fuer etwas, das in Richtung `f` schaut.
    pub fn yaw_of(f: Vec3) -> f32 {
        f.x.atan2(f.z)
    }
    /// Gierwinkel, bei dem lokales +z in Gangrichtung zeigt.
    pub fn yaw(&self) -> f32 {
        Self::yaw_of(self.d)
    }
}

const WALL_C: Vec3 = Vec3::new(0.2, 0.24, 0.2);
const LOW_C: Vec3 = Vec3::new(0.1, 0.11, 0.09);

/// Wand von Punkt `a` nach `b` (achsparallel, Boden bis Decke).
fn wall(b: &mut Builder, a: Vec3, e: Vec3, y0: f32, y1: f32) {
    b.wall(a.x, a.z, e.x, e.z, y0, y1, WALL_C);
}

/// Baut die Huelle und liefert den Bereich hinter der Ausgangstuer (gewonnen).
pub(super) fn shell(b: &mut Builder, rng: &mut Pcg64, c: &mut Corridor) -> Aabb {
    let hw = CORRIDOR_HALF_W;
    let h = CORRIDOR_H;
    let n = c.segments();
    let wo = hw + WALL_T; // Wandmitte

    for i in 0..n {
        let (a, d, l, _) = c.segment(i);
        let f = Frame::new(a, d);
        // gerader Teil ohne die Eckquadrate
        let ua = if i == 0 { 0.0 } else { hw };
        let ub = if i + 1 == n { l } else { l - hw };
        let len = (ub - ua).max(0.0);
        let um = (ua + ub) * 0.5;
        // Boden (schmutzig), Decke (niedrig, fleckig)
        b.add(
            f.at(um, 0.0, -0.05),
            f.half(len * 0.5, hw + WALL_T, 0.05),
            vec3(0.17, 0.16, 0.14),
            FLOOR,
        );
        b.add(
            f.at(um, 0.0, h + 0.05),
            f.half(len * 0.5, hw + WALL_T, 0.05),
            vec3(0.22, 0.22, 0.2),
            CEILING,
        );
        // Waende links (+n) und rechts (-n); am letzten Stueck links die Ausgangsoeffnung
        let last = i + 1 == n;
        wall(b, f.at(ua, -wo, 0.0), f.at(ub, -wo, 0.0), 0.0, h);
        if last {
            let (ea, eb) = (l - EXIT_FROM, l - EXIT_TO);
            wall(b, f.at(ua, wo, 0.0), f.at(ea, wo, 0.0), 0.0, h);
            wall(b, f.at(eb, wo, 0.0), f.at(ub, wo, 0.0), 0.0, h);
            wall(b, f.at(ea, wo, 0.0), f.at(eb, wo, 0.0), 2.2, h);
            // Spiegelwand am Ende
            wall(
                b,
                f.at(l + WALL_T, -wo, 0.0),
                f.at(l + WALL_T, wo, 0.0),
                0.0,
                h,
            );
        } else {
            wall(b, f.at(ua, wo, 0.0), f.at(ub, wo, 0.0), 0.0, h);
        }
        // dunkler Sockel an beiden Seiten
        for s in [-1.0f32, 1.0] {
            let end = if last && s > 0.0 { l - EXIT_FROM } else { ub };
            let ln = (end - ua).max(0.0);
            b.add(
                f.at((ua + end) * 0.5, s * (hw - 0.004), 0.5),
                f.half(ln * 0.5, 0.004, 0.5),
                LOW_C,
                PLAIN,
            );
        }
    }

    // Eckquadrate: Boden, Decke, und die zwei Seiten zu, durch die man nicht kommt
    for k in 1..n {
        let cpt = c.pts[k];
        let d1 = c.segment(k - 1).1;
        let d2 = c.segment(k).1;
        let f1 = Frame::new(cpt, d1);
        b.add(
            cpt + vec3(0.0, -0.05, 0.0),
            vec3(hw + WALL_T, 0.05, hw + WALL_T),
            vec3(0.16, 0.15, 0.13),
            FLOOR,
        );
        b.add(
            cpt + vec3(0.0, h + 0.05, 0.0),
            vec3(hw + WALL_T, 0.05, hw + WALL_T),
            vec3(0.21, 0.21, 0.19),
            CEILING,
        );
        // geradeaus (Fortsetzung von d1) ist zu
        wall(b, f1.at(wo, -wo, 0.0), f1.at(wo, wo, 0.0), 0.0, h);
        // und die Seite gegenueber der Abzweigung
        let f2 = Frame::new(cpt, d2);
        wall(b, f2.at(-wo, -wo, 0.0), f2.at(-wo, wo, 0.0), 0.0, h);
    }

    // Roehren: rot, flackernd, jede dritte tot; nur die lebenden leuchten (Licht kostet)
    let mut s = 2.5;
    let mut k = 0;
    while s < c.len - 1.0 {
        let p = c.point_at(s);
        let d = c.dir_at_s(s);
        let f = Frame::new(p, d);
        let dead = k % 3 == 2;
        let col = vec3(1.0, 0.22, 0.16);
        b.add_ex(
            f.at(0.0, 0.0, h - 0.03),
            f.half(0.6, 0.08, 0.02),
            if dead { vec3(0.15, 0.1, 0.1) } else { col },
            EMISSIVE,
            k as f32,
            if dead { 0.0 } else { 1.2 },
            0.7,
        );
        if !dead {
            b.light(
                p + vec3(0.0, h - 0.3, 0.0),
                6.5,
                col,
                0.9,
                k as f32 * 1.7,
                0.7,
            );
        }
        s += 6.0;
        k += 1;
    }

    // Blutspuren an den Waenden (nur auf geraden Stuecken)
    for _ in 0..(c.len / 6.0) as usize {
        let s = rng.gen_range(3.0..c.len - 4.0);
        if near_corner(c, s, hw + 0.3) {
            continue;
        }
        let f = Frame::new(c.point_at(s), c.dir_at_s(s));
        let v = if rng.gen_bool(0.5) {
            hw - 0.006
        } else {
            -(hw - 0.006)
        };
        b.add(
            f.at(0.0, v, rng.gen_range(0.4..1.9)),
            f.half(rng.gen_range(0.06..0.4), 0.003, rng.gen_range(0.1..0.35)),
            vec3(0.3, 0.02, 0.02),
            PLAIN,
        );
    }

    // Poster: schwarz-rot, leicht leuchtend; dazwischen verrottende Werbeposter
    let mut s = 3.5;
    let mut i = 0;
    while s < c.len - 3.5 {
        if near_corner(c, s, hw + 0.9) {
            s += 1.0;
            continue;
        }
        let left = i % 2 == 1;
        // nicht auf die Ausgangsoeffnung
        let at_exit = left && s > c.len - EXIT_FROM - 0.9;
        let f = Frame::new(c.point_at(s), c.dir_at_s(s));
        let v = if left { hw - 0.012 } else { -(hw - 0.012) };
        // Poster zeigt in den Gang hinein
        let face = if left { -f.n } else { f.n };
        let yaw = Frame::yaw_of(face);
        if !at_exit {
            let text = POSTER_TEXTS[i % POSTER_TEXTS.len()];
            b.add(
                f.at(0.0, v, 1.65),
                f.half(0.72, 0.004, 0.42),
                vec3(0.08, 0.01, 0.01),
                PLAIN,
            );
            b.label(
                f.at(0.0, v - face.dot(f.n) * -0.004, 1.65),
                yaw,
                0.66,
                0.16,
                text,
                vec3(0.25, 0.02, 0.02),
                0.0,
                0.55,
                0.25,
            );
            // dazwischen ein verrottendes Maskottchen-Plakat (gegenueber)
            if i % 3 == 1 && s + 2.6 < c.len - EXIT_FROM - 1.0 && !near_corner(c, s + 2.6, hw + 0.9)
            {
                let f2 = Frame::new(c.point_at(s + 2.6), c.dir_at_s(s + 2.6));
                b.add_ex(
                    f2.at(0.0, -v, 1.6),
                    f2.half(0.45, 0.004, 0.6),
                    vec3(0.85, 0.75, 0.55),
                    POSTER,
                    40.0 + i as f32,
                    0.0,
                    0.0,
                );
            }
        }
        s += 5.5;
        i += 1;
    }

    // Spiegel am Ende (letztes Stueck laeuft immer nach +z), dunkler Rahmen, Schild darueber
    let end = *c.pts.last().unwrap();
    let mirror_center = vec3(end.x, MIRROR_Y, end.z - 0.04);
    c.mirror_center = mirror_center;
    b.add(
        vec3(end.x, MIRROR_Y, end.z - 0.02),
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
        vec3(end.x, MIRROR_Y + MIRROR_HALF.y + 0.2, end.z - 0.03),
        PI,
        0.9,
        0.11,
        "SIEH IN DEN SPIEGEL",
        vec3(0.1, 0.01, 0.01),
        4.0,
        0.4,
        0.3,
    );

    // Ausgangsraum links (+x) vom Ende: grelles Licht, AUSGANG
    let x1 = end.x + hw;
    let z1 = end.z;
    let (ea, eb) = (z1 - EXIT_FROM, z1 - EXIT_TO);
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
    b.wall(x1, rz0 - WALL_T, rx1, rz0 - WALL_T, 0.0, h, WALL_C);
    b.wall(x1, rz1 + WALL_T, rx1, rz1 + WALL_T, 0.0, h, WALL_C);
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
    Aabb {
        min: vec3(x1 + 1.0, 0.0, rz0),
        max: vec3(rx1, h, rz1),
    }
}

/// Liegt die Stelle `s` so nah an einer Ecke, dass dort nichts hin soll?
pub(super) fn near_corner(c: &Corridor, s: f32, margin: f32) -> bool {
    (1..c.segments()).any(|k| (c.segment(k).3 - s).abs() < margin)
}
