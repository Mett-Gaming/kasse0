//! Verfall im gekippten Markt: Loecher im Boden (Hindernisse) und in der Decke, verdorbene
//! und kuriose Ware mit Maden, riesige Waren als Hindernisse. Alles mit dem `weird`-Zufall,
//! damit der normale Grundriss davon unberuehrt bleibt.

use super::*;

const VOID: Vec3 = Vec3::new(0.004, 0.004, 0.005);
const MAGGOT: Vec3 = Vec3::new(0.9, 0.87, 0.74);

/// Bereich, in dem Verfall entstehen darf, und Stellen, die frei bleiben muessen.
pub(super) struct DecayArea {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    pub width: f32,
    pub depth: f32,
    /// Spawn, Tueren, Kassen, Wagenbox ...
    pub keep_free: Vec<Vec3>,
}

impl Gen<'_> {
    /// Verfall einer Ware: 0 frisch, 1 verdorben, 2 kurios.
    pub(super) fn rot_state(&mut self) -> u8 {
        if !self.params.tipped || self.params.rot <= 0.0 {
            return 0;
        }
        if !self.weird.gen_bool(self.params.rot.min(1.0) as f64) {
            return 0;
        }
        if self.weird.gen_bool(0.25) {
            2
        } else {
            1
        }
    }

    /// Ein paar Maden auf verdorbener Ware.
    pub(super) fn maggots(&mut self, product: usize, pos: Vec3) {
        if !self.weird.gen_bool(0.35) {
            return;
        }
        let h = self.cat.products[product].half;
        for _ in 0..self.weird.gen_range(3..7) {
            let p = pos
                + vec3(
                    self.weird.gen_range(-h.x..h.x),
                    h.y * 2.0 + 0.004,
                    self.weird.gen_range(-h.z..h.z),
                );
            let yaw = self.weird.gen_range(0.0..std::f32::consts::TAU);
            self.b
                .add_rot(p, vec3(0.004, 0.0035, 0.011), yaw, MAGGOT, PLAIN);
        }
    }

    /// Freie Stelle (Kreis mit Radius `r`) im Bereich, weit genug weg von `keep_free`.
    fn free_spot(&mut self, a: &DecayArea, r: f32) -> Option<Vec3> {
        'tries: for _ in 0..80 {
            let p = vec3(
                self.weird.gen_range(a.x0..a.x1),
                0.0,
                self.weird.gen_range(a.z0..a.z1),
            );
            if a.keep_free
                .iter()
                .any(|k| (vec3(k.x, 0.0, k.z) - p).length() < 3.0)
            {
                continue;
            }
            for s in &self.b.solids {
                if s.max.y < 0.2 || s.min.y > 1.7 {
                    continue;
                }
                if p.x > s.min.x - r && p.x < s.max.x + r && p.z > s.min.z - r && p.z < s.max.z + r
                {
                    continue 'tries;
                }
            }
            for o in &self.b.obbs {
                let l = o.to_local(vec3(p.x, o.center.y, p.z));
                if l.x.abs() < o.half.x + r && l.z.abs() < o.half.z + r {
                    continue 'tries;
                }
            }
            return Some(p);
        }
        None
    }

    /// Schwarzes Loch im Boden mit abgebrochenen Fliesen; man kommt nicht daran vorbei.
    fn floor_hole(&mut self, c: Vec3) {
        let r = self.weird.gen_range(0.45..0.75);
        for _ in 0..3 {
            let off = vec3(
                self.weird.gen_range(-0.2..0.2),
                0.0,
                self.weird.gen_range(-0.2..0.2),
            );
            let s = r * self.weird.gen_range(0.65..0.85);
            let yaw = self.weird.gen_range(0.0..std::f32::consts::TAU);
            self.b.add_rot_ex(
                c + off + vec3(0.0, 0.003, 0.0),
                vec3(s, 0.003, s * self.weird.gen_range(0.7..1.0)),
                yaw,
                VOID,
                EMISSIVE,
                -1.0,
                0.0,
                0.0,
            );
        }
        // Rand: hochstehende Fliesenscherben
        for k in 0..7 {
            let a = k as f32 / 7.0 * std::f32::consts::TAU + self.weird.gen_range(-0.3..0.3);
            let p = c + vec3(a.cos(), 0.0, a.sin()) * (r * 0.95);
            let size = self.weird.gen_range(0.06..0.14);
            self.b.add_floor(
                p + vec3(0.0, self.weird.gen_range(0.008..0.03), 0.0),
                vec3(size, 0.008, size * 0.7),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                vec3(0.32, 0.33, 0.3),
                PLAIN,
            );
        }
        self.b
            .solid(c + vec3(0.0, 0.6, 0.0), vec3(r * 0.85, 0.6, r * 0.85));
    }

    /// Loch in der Decke: Leere, herabhaengende Kabel, Truemmer darunter.
    fn ceiling_hole(&mut self, c: Vec3) {
        let h = CEILING_HEIGHT;
        let r = self.weird.gen_range(0.5..1.0);
        for _ in 0..3 {
            let off = vec3(
                self.weird.gen_range(-0.25..0.25),
                0.0,
                self.weird.gen_range(-0.25..0.25),
            );
            let s = r * self.weird.gen_range(0.6..0.85);
            self.b.add_rot_ex(
                vec3(c.x, h - 0.004, c.z) + off,
                vec3(s, 0.005, s * self.weird.gen_range(0.6..1.0)),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                VOID,
                EMISSIVE,
                0.0,
                0.0,
                0.0,
            );
        }
        for _ in 0..self.weird.gen_range(2..5) {
            let len = self.weird.gen_range(0.4..1.4);
            let p = vec3(
                c.x + self.weird.gen_range(-r..r) * 0.6,
                h - len * 0.5,
                c.z + self.weird.gen_range(-r..r) * 0.6,
            );
            self.b.add(
                p,
                vec3(0.008, len * 0.5, 0.008),
                vec3(0.05, 0.05, 0.06),
                PLAIN,
            );
        }
        // heruntergefallene Deckenplatten
        for _ in 0..self.weird.gen_range(2..5) {
            let p = c + vec3(
                self.weird.gen_range(-r..r),
                0.01,
                self.weird.gen_range(-r..r),
            );
            let size = self.weird.gen_range(0.12..0.3);
            self.b.add_floor(
                p,
                vec3(size, 0.01, size * 0.8),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                vec3(0.7, 0.7, 0.68),
                PLAIN,
            );
        }
    }

    /// Fehlende Fliese: nackter Estrich, etwas tiefer als der Boden wirkt.
    fn missing_tile(&mut self, c: Vec3) {
        let shade = self.weird.gen_range(0.85..1.1);
        self.b.add_floor(
            c + vec3(0.0, 0.002, 0.0),
            vec3(0.3, 0.002, 0.3),
            0.0,
            vec3(0.3, 0.29, 0.26) * shade,
            PLAIN,
        );
        // Kleberreste
        for _ in 0..self.weird.gen_range(2..5) {
            let p = c + vec3(
                self.weird.gen_range(-0.22..0.22),
                0.003,
                self.weird.gen_range(-0.22..0.22),
            );
            self.b.add_floor(
                p,
                vec3(0.04, 0.002, 0.012),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                vec3(0.2, 0.19, 0.16),
                PLAIN,
            );
        }
    }

    /// Gesprungene Fliese: Scherben mit dunklen Fugen dazwischen.
    fn broken_tile(&mut self, c: Vec3, floor: Vec3) {
        self.b.add_floor(
            c + vec3(0.0, 0.0015, 0.0),
            vec3(0.3, 0.0015, 0.3),
            0.0,
            vec3(0.08, 0.08, 0.07),
            PLAIN,
        );
        for _ in 0..self.weird.gen_range(3..6) {
            let p = c + vec3(
                self.weird.gen_range(-0.17..0.17),
                0.004,
                self.weird.gen_range(-0.17..0.17),
            );
            let s = self.weird.gen_range(0.07..0.13);
            self.b.add_floor(
                p,
                vec3(s, 0.003, s * self.weird.gen_range(0.5..1.0)),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                floor * self.weird.gen_range(0.75..0.95),
                PLAIN,
            );
        }
    }

    /// Stapel herausgerissener Fliesen (Hindernis).
    fn tile_stack(&mut self, c: Vec3, floor: Vec3) {
        let n = self.weird.gen_range(4..10);
        for k in 0..n {
            self.b.add_floor(
                c + vec3(
                    self.weird.gen_range(-0.03..0.03),
                    0.007 + k as f32 * 0.013,
                    self.weird.gen_range(-0.03..0.03),
                ),
                vec3(0.3, 0.006, 0.3),
                self.weird.gen_range(-0.15..0.15),
                floor * self.weird.gen_range(0.7..0.95),
                PLAIN,
            );
        }
        self.b.solid(c + vec3(0.0, 0.3, 0.0), vec3(0.33, 0.3, 0.33));
    }

    /// Blut tropft von der Decke: Fleck oben, Lache unten (die Tropfen zeichnet das Spiel).
    fn blood(&mut self, c: Vec3) {
        let red = vec3(0.24, 0.01, 0.01);
        self.b.add(
            vec3(c.x, CEILING_HEIGHT - 0.003, c.z),
            vec3(0.18, 0.003, 0.14),
            red * 0.8,
            PLAIN,
        );
        for _ in 0..3 {
            let off = vec3(
                self.weird.gen_range(-0.15..0.15),
                0.0,
                self.weird.gen_range(-0.15..0.15),
            );
            let s = self.weird.gen_range(0.12..0.32);
            self.b.add_floor(
                c + off + vec3(0.0, 0.004, 0.0),
                vec3(s, 0.002, s * self.weird.gen_range(0.6..1.0)),
                self.weird.gen_range(0.0..std::f32::consts::TAU),
                red,
                PLAIN,
            );
        }
        self.drips.push(vec3(c.x, CEILING_HEIGHT - 0.01, c.z));
    }

    /// Strecke auf freiem Boden (fuer Ameisen und Ratten).
    fn floor_run(&mut self, a: &DecayArea, len: (f32, f32)) -> Option<(Vec3, Vec3)> {
        for _ in 0..20 {
            let p = self.free_spot(a, 0.25)?;
            let yaw = self.weird.gen_range(0.0..std::f32::consts::TAU);
            let l = self.weird.gen_range(len.0..len.1);
            let q = p + vec3(yaw.sin(), 0.0, yaw.cos()) * l;
            let mid = (p + q) * 0.5;
            let free = |b: &Builder| {
                !b.solids.iter().any(|s| {
                    s.max.y >= 0.2
                        && [p, mid, q].iter().any(|x| {
                            x.x > s.min.x - 0.2
                                && x.x < s.max.x + 0.2
                                && x.z > s.min.z - 0.2
                                && x.z < s.max.z + 0.2
                        })
                })
            };
            if free(&self.b) {
                return Some((p, q));
            }
        }
        None
    }

    /// Boden und Decke verfallen in sinnvoller Reihenfolge (Stufe 1: Fliesen, Ameisen;
    /// Stufe 2: Fliesenstapel, Ratten, Deckenloecher; Stufe 3: Blut von der Decke).
    pub(super) fn floor_and_ceiling_decay(&mut self, a: &DecayArea, floor: Vec3) {
        let lv = self.params.decay.min(3) as usize;
        if !self.params.tipped || lv == 0 {
            return;
        }
        let count = |t: [usize; 4]| t[lv];
        for _ in 0..count([0, 6, 14, 24]) {
            if let Some(p) = self.free_spot(a, 0.35) {
                self.missing_tile(p);
            }
        }
        for _ in 0..count([0, 4, 10, 16]) {
            if let Some(p) = self.free_spot(a, 0.35) {
                self.broken_tile(p, floor);
            }
        }
        for _ in 0..count([0, 2, 4, 7]) {
            if let Some(run) = self.floor_run(a, (1.5, 3.5)) {
                self.ant_trails.push(run);
            }
        }
        for _ in 0..count([0, 0, 2, 4]) {
            if let Some(p) = self.free_spot(a, 0.5) {
                self.tile_stack(p, floor);
            }
        }
        for _ in 0..count([0, 0, 2, 5]) {
            if let Some(run) = self.floor_run(a, (2.0, 4.5)) {
                self.rat_runs.push(run);
            }
        }
        for _ in 0..count([0, 0, 0, 7]) {
            if let Some(p) = self.free_spot(a, 0.4) {
                self.blood(p);
            }
        }
    }

    /// Alle Verfallserscheinungen des gekippten Markts (je nach Stufe).
    pub(super) fn decay(&mut self, a: &DecayArea) {
        if !self.params.tipped {
            return;
        }
        for _ in 0..self.params.holes {
            if let Some(p) = self.free_spot(a, 0.8) {
                self.floor_hole(p);
            }
        }
        let ceiling = DecayArea {
            x0: 1.0,
            x1: a.width - 1.0,
            z0: 1.0,
            z1: a.depth - 1.0,
            width: a.width,
            depth: a.depth,
            keep_free: Vec::new(),
        };
        for _ in 0..self.params.ceiling_holes {
            let p = vec3(
                self.weird.gen_range(ceiling.x0..ceiling.x1),
                0.0,
                self.weird.gen_range(ceiling.z0..ceiling.z1),
            );
            self.ceiling_hole(p);
        }
        for _ in 0..self.params.giants {
            let product = self.weird.gen_range(0..self.cat.products.len());
            let half = self.cat.products[product].half;
            let scale = (1.4 / (half.y * 2.0)).clamp(4.0, 14.0);
            let r = half.x.max(half.z) * scale;
            if r > 1.0 {
                continue;
            }
            if let Some(p) = self.free_spot(a, r + 0.2) {
                let yaw = self.weird.gen_range(0.0..std::f32::consts::TAU);
                self.giants.push(Giant {
                    product: product as u32,
                    pos: p,
                    yaw,
                    scale,
                });
                self.b.solid(
                    p + vec3(0.0, half.y * scale, 0.0),
                    vec3(r, half.y * scale, r),
                );
            }
        }
    }
}
