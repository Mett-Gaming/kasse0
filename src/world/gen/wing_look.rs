//! Aussehen der Pfandraeume im Seitentrakt (wing.rs): ein gepflegter Rueckgaberaum mit
//! Bodenfliesen, Wandfliesen bis Hueft hoehe, ruhigen Deckenleuchten, Schildern, einer
//! Bodenmarkierung vor dem Automaten und Getraenkekisten. Je weiter der Trakt waechst
//! (`params.wing` 1..4), desto mehr verfaellt er: Fliesen fehlen, Flecken, das Licht
//! flackert, Kisten liegen umgekippt. Die Pfandautomaten selbst bleiben, wie sie sind.
//!
//! Alles aus Boxen (Raeume und Einrichtung).

use super::*;

/// Schilder im Trakt (stehen im Schild-Atlas, signs.rs)
pub const WING_TEXTS: [&str; 4] = [
    "Pfandrückgabe",
    "Danke fürs Zurückbringen",
    "Nichts kommt zurück",
    "Du bringst immer wieder",
];
/// Hoehe der Wandfliesen (m)
pub const TILE_TOP: f32 = 1.2;
/// Kantenlaenge der Bodenfliesen (m)
const FLOOR_TILE: f32 = 0.6;
/// Getraenkekiste: halbe Groesse
const CRATE: Vec3 = Vec3::new(0.2, 0.15, 0.15);

/// Ein Raum, wie wing.rs ihn baut: Innenmasse und die Oeffnungen in seinen Waenden.
pub(super) struct RoomShape {
    pub x_w: f32,
    pub x_e: f32,
    pub z0: f32,
    pub z1: f32,
    /// Oeffnungen (von, bis) je Wand: Westen, Osten (in z), Sueden, Norden (in x)
    pub gaps: [Vec<(f32, f32)>; 4],
}

/// Teilstrecken von `a` bis `b` ohne die Oeffnungen.
fn spans(a: f32, b: f32, gaps: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out = vec![(a, b)];
    for &(g0, g1) in gaps {
        out = out
            .into_iter()
            .flat_map(|(s, e)| {
                let mut v = Vec::new();
                if g0 > s {
                    v.push((s, g0.min(e)));
                }
                if g1 < e {
                    v.push((g1.max(s), e));
                }
                v
            })
            .filter(|(s, e)| e - s > 0.05)
            .collect();
    }
    out
}

/// Fester Zufall je Raum und Stelle (0..1), gleich bei jedem Umbau.
fn hash(seed: u64, k: usize, i: usize) -> f32 {
    let x = (seed as f32 * 0.013 + k as f32 * 12.9898 + i as f32 * 78.233).sin() * 43_758.547;
    x.fract().abs()
}

impl Gen<'_> {
    /// Stufe des Trakts (1..4): im normalen Markt 1, gekippt `params.wing`.
    pub(super) fn wing_level(&self) -> u8 {
        if self.params.tipped {
            self.params.wing.max(1)
        } else {
            1
        }
    }

    /// Verfall des Trakts: 0 (gepflegt, Stufe 1) bis 1 (Kollaps).
    pub(super) fn wing_decay(&self) -> f32 {
        ((self.wing_level() - 1) as f32 / 3.0).clamp(0.0, 1.0)
    }

    /// Wandfarbe ueber den Fliesen: sauber hell, mit dem Verfall vergilbt.
    pub(super) fn wing_wall_color(&self, market_wall: Vec3) -> Vec3 {
        let clean = vec3(0.86, 0.87, 0.84);
        let yellowed = market_wall * vec3(1.05, 1.0, 0.82);
        clean.lerp(yellowed, self.wing_decay())
    }

    /// Einrichtung eines Raums. `machine_z`: wo der Automat an der Westwand steht.
    pub(super) fn dress_wing_room(&mut self, k: usize, r: &RoomShape, machine_z: f32) {
        let d = self.wing_decay();
        let seed = self.params.seed;
        let (xc, zc) = ((r.x_w + r.x_e) * 0.5, (r.z0 + r.z1) * 0.5);

        // --- Bodenfliesen: helles Schachbrett, mit dem Verfall fleckig und lueckenhaft ---
        let nx = ((r.x_e - r.x_w) / FLOOR_TILE).floor() as i32;
        let nz = ((r.z1 - r.z0) / FLOOR_TILE).floor() as i32;
        let ox = xc - nx as f32 * FLOOR_TILE * 0.5;
        let oz = zc - nz as f32 * FLOOR_TILE * 0.5;
        for i in 0..nx {
            for j in 0..nz {
                let c = vec3(
                    ox + (i as f32 + 0.5) * FLOOR_TILE,
                    0.002,
                    oz + (j as f32 + 0.5) * FLOOR_TILE,
                );
                let h = hash(seed, k, (i * 97 + j) as usize);
                let col = if h < d * 0.12 {
                    vec3(0.16, 0.15, 0.13) // Fliese fehlt
                } else if h < d * 0.3 {
                    vec3(0.42, 0.36, 0.26) // Fleck
                } else if (i + j) % 2 == 0 {
                    vec3(0.74, 0.75, 0.73)
                } else {
                    vec3(0.56, 0.6, 0.64)
                };
                let half = FLOOR_TILE * 0.5 - 0.008;
                self.b
                    .add_floor(c, vec3(half, 0.002, half), 0.0, col, FLOOR);
            }
        }

        // --- Wandfliesen bis Hueft hoehe mit Zierleiste und Sockel, an allen vier Waenden ---
        let tile = vec3(0.82, 0.88, 0.9);
        let trim = vec3(0.18, 0.36, 0.62);
        let skirt = vec3(0.3, 0.32, 0.34);
        let walls: [(bool, f32, f32, f32, f32); 4] = [
            // (laengs z?, Wand-Lage, Richtung nach innen, von, bis)
            (true, r.x_w, 1.0, r.z0, r.z1),
            (true, r.x_e, -1.0, r.z0, r.z1),
            (false, r.z0, 1.0, r.x_w, r.x_e),
            (false, r.z1, -1.0, r.x_w, r.x_e),
        ];
        for (w, &(along_z, at, inward, a, b)) in walls.iter().enumerate() {
            for (s, e) in spans(a, b, &r.gaps[w]) {
                let mid = (s + e) * 0.5;
                let len = (e - s) * 0.5;
                let place = |off: f32, y: f32, hy: f32| -> (Vec3, Vec3) {
                    let p = at + inward * off;
                    if along_z {
                        (vec3(p, y, mid), vec3(off, hy, len))
                    } else {
                        (vec3(mid, y, p), vec3(len, hy, off))
                    }
                };
                let (c, h) = place(0.008, TILE_TOP * 0.5, TILE_TOP * 0.5);
                self.b.add(c, h, tile, PLAIN);
                let (c, h) = place(0.014, TILE_TOP, 0.025);
                self.b.add(c, h, trim, PLAIN);
                let (c, h) = place(0.014, 0.05, 0.05);
                self.b.add(c, h, skirt, PLAIN);
                // Fugen: zwei waagerechte Linien
                for y in [0.4f32, 0.8] {
                    let (c, h) = place(0.01, y, 0.004);
                    self.b.add(c, h, tile * 0.82, PLAIN);
                }
                // Verfall: herausgebrochene Fliesen (dunkel) und Wasserflecken
                let n = (len * 2.0 * d * 2.5) as usize;
                for t in 0..n {
                    let u = hash(seed, k * 4 + w, t * 3);
                    let v = hash(seed, k * 4 + w, t * 3 + 1);
                    let along = s + 0.2 + u * (e - s - 0.4).max(0.0);
                    let y = 0.25 + v * 0.8;
                    let stain = hash(seed, k * 4 + w, t * 3 + 2) < 0.5;
                    let (col, hy, hl) = if stain {
                        (vec3(0.45, 0.38, 0.24), 0.18, 0.25)
                    } else {
                        (vec3(0.22, 0.21, 0.19), 0.09, 0.09)
                    };
                    let p = at + inward * 0.011;
                    let (c, h) = if along_z {
                        (vec3(p, y, along), vec3(0.011, hy, hl))
                    } else {
                        (vec3(along, y, p), vec3(hl, hy, 0.011))
                    };
                    self.b.add(c, h, col, PLAIN);
                }
            }
        }

        // --- Schilder: ueber dem Automaten und an der Suedwand ---
        let sign_bg = vec3(0.16, 0.34, 0.6);
        let (main, thanks) = if d >= 0.66 {
            (WING_TEXTS[2], WING_TEXTS[3])
        } else {
            (WING_TEXTS[0], WING_TEXTS[1])
        };
        let flick = if d >= 0.66 { 0.3 } else { 0.0 };
        self.b.add(
            vec3(r.x_w + 0.015, 2.25, machine_z),
            vec3(0.012, 0.15, 0.6),
            sign_bg,
            PLAIN,
        );
        self.b.label(
            vec3(r.x_w + 0.03, 2.25, machine_z),
            FRAC_PI_2,
            0.56,
            0.11,
            main,
            sign_bg,
            0.0,
            0.6,
            flick,
        );
        let tz = r.z0 + 0.015;
        let tx = xc + 1.8;
        self.b.add(
            vec3(tx, 1.75, tz),
            vec3(0.8, 0.16, 0.012),
            vec3(0.95, 0.95, 0.92),
            PLAIN,
        );
        self.b.label(
            vec3(tx, 1.75, tz + 0.015),
            0.0,
            0.76,
            0.1,
            thanks,
            vec3(0.95, 0.95, 0.92),
            1.0,
            0.0,
            flick,
        );

        // --- Bodenmarkierung vor dem Automaten: gelber Rahmen "hier stehen" ---
        let yellow = vec3(0.9, 0.75, 0.1) * (1.0 - 0.4 * d);
        let fx = r.x_w + 0.72 + 0.55;
        for (c, h) in [
            (vec3(fx - 0.5, 0.004, machine_z), vec3(0.03, 0.002, 0.5)),
            (vec3(fx + 0.5, 0.004, machine_z), vec3(0.03, 0.002, 0.5)),
            (vec3(fx, 0.004, machine_z - 0.5), vec3(0.53, 0.002, 0.03)),
            (vec3(fx, 0.004, machine_z + 0.5), vec3(0.53, 0.002, 0.03)),
        ] {
            self.b.add_floor(c, h, 0.0, yellow, FLOOR);
        }

        // --- Getraenkekisten: zwei Stapel in den Ecken, ab "falsch" einer umgekippt ---
        let colors = [
            vec3(0.15, 0.45, 0.25),
            vec3(0.18, 0.3, 0.6),
            vec3(0.6, 0.18, 0.15),
        ];
        let stacks = [
            vec3(r.x_w + CRATE.x + 0.05, 0.0, r.z1 - CRATE.z - 0.05),
            vec3(r.x_e - CRATE.x - 0.05, 0.0, r.z0 + CRATE.z + 0.05),
        ];
        for (s, &base) in stacks.iter().enumerate() {
            let fallen = d >= 0.66 && s == k % 2;
            let n = if fallen { 1 } else { 3 };
            let col = colors[(k + s) % colors.len()];
            for i in 0..n {
                let c = base + vec3(0.0, CRATE.y + i as f32 * CRATE.y * 2.0, 0.0);
                self.crate_at(c, 0.0, col);
            }
            self.b.solid(
                base + vec3(0.0, CRATE.y * n as f32, 0.0),
                vec3(CRATE.x, CRATE.y * n as f32, CRATE.z),
            );
            if fallen {
                // die anderen beiden liegen daneben auf der Seite, schief
                let into = vec3(
                    if s == 0 { 1.0 } else { -1.0 },
                    0.0,
                    if s == 0 { -1.0 } else { 1.0 },
                );
                for i in 0..2 {
                    let yaw = 0.4 + i as f32 * 0.9;
                    let c = base + into * (0.55 + i as f32 * 0.45) + vec3(0.0, CRATE.y, 0.0);
                    self.crate_at(c, yaw, col);
                    self.b.solid_rot(c, CRATE, yaw);
                }
            }
        }

        // Moebel und Kleinkram (wing_furniture.rs)
        self.furnish_wing_room(k, r, machine_z, d, !r.gaps[1].is_empty());
    }

    /// Eine Getraenkekiste mit Flaschenhaelsen.
    fn crate_at(&mut self, c: Vec3, yaw: f32, col: Vec3) {
        self.b.add_rot(c, CRATE, yaw, col, PLAIN);
        let (s, co) = yaw.sin_cos();
        let rot = |x: f32, z: f32| vec3(x * co + z * s, 0.0, -x * s + z * co);
        for i in 0..3 {
            for j in 0..2 {
                let off = rot((i as f32 - 1.0) * 0.12, (j as f32 - 0.5) * 0.14);
                self.b.add_rot(
                    c + off + vec3(0.0, CRATE.y + 0.03, 0.0),
                    vec3(0.022, 0.03, 0.022),
                    yaw,
                    vec3(0.25, 0.35, 0.22),
                    PLAIN,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::spans;

    #[test]
    fn wall_spans_leave_the_openings_free() {
        let s = spans(0.0, 7.0, &[(3.0, 4.0)]);
        assert_eq!(s, vec![(0.0, 3.0), (4.0, 7.0)]);
        assert_eq!(spans(0.0, 7.0, &[]), vec![(0.0, 7.0)]);
        assert_eq!(
            spans(0.0, 7.0, &[(-1.0, 1.0), (6.5, 8.0)]),
            vec![(1.0, 6.5)]
        );
    }
}
