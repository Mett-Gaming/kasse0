//! Bausteine des Generators: Box-Builder, gedrehte Rahmen und die Einrichtungen
//! (Regalseiten, Obstkisten, Gitterboxen, Lichtbaender), die Ware platzieren.

use super::*;

#[derive(Default)]
pub(super) struct Builder {
    pub(super) instances: Vec<GpuInstance>,
    pub(super) lights: Vec<GpuLight>,
    pub(super) solids: Vec<Aabb>,
    pub(super) obbs: Vec<Obb>,
}

impl Builder {
    pub(super) fn add(&mut self, c: Vec3, h: Vec3, color: Vec3, material: f32) {
        self.add_ex(c, h, color, material, 0.0, 0.0, 0.0);
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_ex(
        &mut self,
        c: Vec3,
        h: Vec3,
        color: Vec3,
        material: f32,
        seed: f32,
        emissive: f32,
        flicker: f32,
    ) {
        self.add_rot_ex(c, h, 0.0, color, material, seed, emissive, flicker);
    }
    pub(super) fn add_rot(&mut self, c: Vec3, h: Vec3, yaw: f32, color: Vec3, material: f32) {
        self.add_rot_ex(c, h, yaw, color, material, 0.0, 0.0, 0.0);
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_rot_ex(
        &mut self,
        c: Vec3,
        h: Vec3,
        yaw: f32,
        color: Vec3,
        material: f32,
        seed: f32,
        emissive: f32,
        flicker: f32,
    ) {
        self.instances.push(GpuInstance {
            center: c.extend(yaw).to_array(),
            half: h.extend(0.0).to_array(),
            color: color.extend(1.0).to_array(),
            extra: [material, seed, emissive, flicker],
        });
    }
    /// Schild mit Schriftzug aus dem Atlas, Vorderseite lokal +z (beide Seiten lesbar).
    /// Die Schrifthoehe passt sich an, damit der Text in `max_hw` passt. Liefert die halbe Breite.
    /// `ink`: 0 weiss, 1 dunkelblau, 2 schwarz, 3 gelb, 4 rot. `glow` > 0 = selbstleuchtend.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn label(
        &mut self,
        c: Vec3,
        yaw: f32,
        max_hw: f32,
        max_hh: f32,
        text: &str,
        bg: Vec3,
        ink: f32,
        glow: f32,
        flicker: f32,
    ) -> f32 {
        let a = crate::signs::atlas();
        let id = a.id(text);
        let asp = a.aspect(id) + 0.6;
        let hh = max_hh.min(max_hw / asp);
        let hw = hh * asp;
        self.instances.push(GpuInstance {
            center: c.extend(yaw).to_array(),
            half: [hw, hh, 0.006, 0.0],
            color: bg.extend(ink).to_array(),
            extra: [SIGN, id as f32, glow, flicker],
        });
        hw
    }
    /// Flach auf dem Boden Liegendes: folgt beim Verbiegen nur der Hoehenwelle (Markierung seed = -1).
    pub(super) fn add_floor(&mut self, c: Vec3, h: Vec3, yaw: f32, color: Vec3, material: f32) {
        self.add_rot_ex(c, h, yaw, color, material, -1.0, 0.0, 0.0);
    }
    pub(super) fn solid(&mut self, c: Vec3, h: Vec3) {
        self.solids.push(Aabb::from_center(c, h));
    }
    pub(super) fn solid_rot(&mut self, c: Vec3, h: Vec3, yaw: f32) {
        if yaw.abs() < 1e-5 {
            self.solid(c, h);
        } else {
            self.obbs.push(Obb {
                center: c,
                half: h,
                yaw,
            });
        }
    }
    pub(super) fn add_solid(&mut self, c: Vec3, h: Vec3, color: Vec3) {
        self.add(c, h, color, PLAIN);
        self.solid(c, h);
    }
    pub(super) fn light(
        &mut self,
        pos: Vec3,
        radius: f32,
        color: Vec3,
        intensity: f32,
        seed: f32,
        flicker: f32,
    ) {
        self.lights.push(GpuLight {
            pos: pos.extend(radius).to_array(),
            color: color.extend(intensity).to_array(),
            extra: [seed, flicker, 0.0, 0.0],
        });
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn wall(
        &mut self,
        x0: f32,
        z0: f32,
        x1: f32,
        z1: f32,
        y0: f32,
        y1: f32,
        color: Vec3,
    ) {
        let c = vec3((x0 + x1) * 0.5, (y0 + y1) * 0.5, (z0 + z1) * 0.5);
        let h = vec3(
            ((x1 - x0) * 0.5).abs().max(WALL_T),
            (y1 - y0) * 0.5,
            ((z1 - z0) * 0.5).abs().max(WALL_T),
        );
        self.add_solid(c, h, color);
    }
}

/// Lokales Koordinatensystem: entlang (dir), quer (normal), Hoehe. Beliebig um die Hochachse gedreht.
#[derive(Clone, Copy)]
pub(super) struct Frame {
    pub(super) origin: Vec3,
    pub(super) dir: Vec3,
    pub(super) normal: Vec3,
    pub(super) yaw: f32,
}

impl Frame {
    pub(super) fn new(origin: Vec3, yaw: f32) -> Self {
        let (s, c) = yaw.sin_cos();
        Self {
            origin,
            dir: vec3(s, 0.0, c),
            normal: vec3(c, 0.0, -s),
            yaw,
        }
    }
    pub(super) fn pos(&self, along: f32, perp: f32, y: f32) -> Vec3 {
        self.origin + self.dir * along + self.normal * perp + vec3(0.0, y, 0.0)
    }
    /// Halbe Ausdehnung in lokalen Box-Achsen (x = quer, z = entlang)
    pub(super) fn half(&self, along: f32, perp: f32, y: f32) -> Vec3 {
        vec3(perp, y, along)
    }
    /// Drehung einer Ware, deren Vorderseite in Richtung `normal * facing` zeigt
    pub(super) fn item_yaw(&self, facing: f32) -> f32 {
        self.yaw + facing * FRAC_PI_2
    }
}

pub(super) struct ShelfSpec {
    pub(super) frame: Frame,
    pub(super) len: f32,
    /// Querversatz der Vorderkante, Richtung in die das Regal zeigt (+1/-1 entlang normal)
    pub(super) front: f32,
    pub(super) facing: f32,
    pub(super) depth: f32,
    pub(super) boards: &'static [f32],
    pub(super) sub: usize,
    pub(super) board_color: Vec3,
    pub(super) strip_color: Vec3,
    /// Ware in Kartons (Discounter-Look)
    pub(super) trays: bool,
    /// Kundenplatz vor diesem Abschnitt anlegen
    pub(super) spot: bool,
}

pub(super) struct Gen<'a> {
    pub(super) b: Builder,
    pub(super) rng: &'a mut Pcg64,
    pub(super) weird: &'a mut Pcg64,
    pub(super) params: &'a WorldParams,
    pub(super) cat: &'static Catalog,
    pub(super) layout: Layout,
    pub(super) items: Vec<ShelfItem>,
    pub(super) spots: Vec<Spot>,
    /// Kuehltruhen-Stuecke: (Mitte, Laengsrichtung, Laenge)
    pub(super) freezers: Vec<(Vec3, f32, f32)>,
    /// Zettel-Waren, die noch nirgends stehen (werden bevorzugt platziert)
    pub(super) missing: std::collections::HashSet<usize>,
    /// riesige Waren (Verfall)
    pub(super) giants: Vec<Giant>,
    /// bewegte Verfallsdetails (zeichnet das Spiel)
    pub(super) ant_trails: Vec<(Vec3, Vec3)>,
    pub(super) rat_runs: Vec<(Vec3, Vec3)>,
    pub(super) drips: Vec<Vec3>,
    pub(super) sparks: Vec<Vec3>,
    pub(super) ceiling_holes: Vec<Vec3>,
}

impl Gen<'_> {
    pub(super) fn fits(&self, p: usize, clearance: f32, depth: f32) -> bool {
        let h = self.cat.products[p].half;
        h.y * 2.0 <= clearance && h.z * 2.0 <= depth + 0.12
    }

    /// Ware fuer ein Fach waehlen. Billige Zettel-Varianten liegen nur ganz unten/oben (`edge`).
    pub(super) fn pick(
        &mut self,
        sub: usize,
        edge: bool,
        clearance: f32,
        depth: f32,
    ) -> Option<usize> {
        let cat = self.cat;
        if self.layout.chaos > 0.0 && self.weird.gen_bool(self.layout.chaos as f64) {
            for _ in 0..8 {
                let p = self.weird.gen_range(0..cat.products.len());
                if self.fits(p, clearance, depth) {
                    return Some(p);
                }
            }
        }
        let list = &cat.subs[sub].products;
        let mut ok: Vec<usize> = list
            .iter()
            .copied()
            .filter(|&p| {
                self.fits(p, clearance, depth)
                    && (edge || !(cat.products[p].list.is_some() && !cat.products[p].pricey))
            })
            .collect();
        if let Some(&p) = ok.iter().find(|p| self.missing.contains(p)) {
            return Some(p);
        }
        if ok.is_empty() {
            ok = list
                .iter()
                .copied()
                .filter(|&p| self.fits(p, clearance, depth))
                .collect();
        }
        if ok.is_empty() {
            None
        } else {
            Some(ok[self.rng.gen_range(0..ok.len())])
        }
    }

    pub(super) fn place(&mut self, product: usize, pos: Vec3, yaw: f32) {
        let rot = self.rot_state();
        // Im gekippten Markt sind manche Zettel-Waren falsch beschriftet (Anomalie)
        let fake = self.params.tipped
            && self.cat.products[product].list.is_some()
            && self.weird.gen_bool(0.06);
        self.items.push(ShelfItem {
            product: product as u32,
            pos,
            yaw,
            fake,
            taken: false,
            rot,
        });
        if !fake {
            self.missing.remove(&product);
        }
        if rot == 1 {
            self.maggots(product, pos);
        }
    }

    /// Eine Regalseite: Boeden, Preisleisten, Waren (in Kartons) aus einer Unterabteilung.
    pub(super) fn shelf_side(&mut self, s: &ShelfSpec) {
        let tipped = self.params.tipped;
        let f = s.frame;
        let len = s.len;
        let board_perp = s.front - s.facing * s.depth * 0.5;
        let hl = len * 0.5 + 0.02;
        for &h in s.boards {
            self.b.add_rot(
                f.pos(len * 0.5, board_perp, h),
                f.half(hl, s.depth * 0.5, 0.015),
                f.yaw,
                s.board_color,
                PLAIN,
            );
            self.b.add_rot(
                f.pos(len * 0.5, s.front + s.facing * 0.006, h - 0.02),
                f.half(hl, 0.006, 0.028),
                f.yaw,
                s.strip_color,
                PLAIN,
            );
        }
        let yaw = f.item_yaw(s.facing);
        let n = s.boards.len();
        for (bi, &h) in s.boards.iter().enumerate() {
            let clearance = s.boards.get(bi + 1).map_or(0.4, |next| next - h - 0.05);
            let edge = bi == 0 || bi + 1 == n;
            let mut a = 0.03;
            while a < len - 0.06 {
                let Some(p) = self.pick(s.sub, edge, clearance, s.depth) else {
                    break;
                };
                let half = self.cat.products[p].half;
                let w = half.x * 2.0 + 0.012;
                if a + w > len - 0.03 {
                    break;
                }
                let start = a;
                let run = self.rng.gen_range(2..=6);
                for _ in 0..run {
                    if a + w > len - 0.03 {
                        break;
                    }
                    let gap = tipped && self.weird.gen_bool(0.08);
                    if !gap {
                        self.place(
                            p,
                            f.pos(a + w * 0.5, s.front - s.facing * (half.z + 0.03), h + 0.016),
                            yaw,
                        );
                    }
                    a += w;
                }
                if s.trays {
                    let td = (half.z + 0.04).min(s.depth * 0.5);
                    self.b.add_rot(
                        f.pos(
                            (start + a) * 0.5,
                            s.front - s.facing * (td + 0.01),
                            h + 0.028,
                        ),
                        f.half((a - start) * 0.5, td, 0.012),
                        f.yaw,
                        CARDBOARD * if tipped { 0.6 } else { 1.0 },
                        PLAIN,
                    );
                }
                a += 0.02;
            }
        }
        if s.spot {
            let bi = 2.min(n - 1);
            self.spots.push(Spot {
                pos: f.pos(len * 0.5, s.front + s.facing * 0.55, 0.0),
                facing: f.normal * -s.facing,
                shelf: f.pos(len * 0.5, s.front - s.facing * 0.1, s.boards[bi] + 0.02),
            });
        }
    }

    /// Eine Reihe Obst-/Gemuesekisten auf Hoehe `y`, Kisten 0,42 m breit.
    pub(super) fn crate_row(
        &mut self,
        f: Frame,
        len: f32,
        perp: f32,
        y: f32,
        facing: f32,
        sub: usize,
    ) {
        let mut a = 0.02;
        let mut k = 0;
        while a + 0.42 <= len {
            let Some(p) = self.pick(sub, true, 0.4, 0.36) else {
                break;
            };
            let run = self.rng.gen_range(1..=3);
            for _ in 0..run {
                if a + 0.42 > len {
                    break;
                }
                let c = if k % 2 == 0 {
                    vec3(0.12, 0.38, 0.18)
                } else {
                    vec3(0.08, 0.08, 0.09)
                };
                self.b.add_rot(
                    f.pos(a + 0.21, perp, y - 0.07),
                    f.half(0.2, 0.18, 0.07),
                    f.yaw,
                    c,
                    PLAIN,
                );
                let half = self.cat.products[p].half;
                let cols = ((0.38 / (half.x * 2.0 + 0.01)) as usize).clamp(1, 4);
                let rows = ((0.34 / (half.z * 2.0 + 0.01)) as usize).clamp(1, 3);
                let (cw, rd) = (0.38 / cols as f32, 0.34 / rows as f32);
                for ci in 0..cols {
                    for ri in 0..rows {
                        let pos = f.pos(
                            a + 0.02 + (ci as f32 + 0.5) * cw,
                            perp - 0.17 + (ri as f32 + 0.5) * rd,
                            y,
                        );
                        let jitter = self.weird.gen_range(-0.4..0.4);
                        self.place(p, pos, f.item_yaw(facing) + jitter);
                    }
                }
                a += 0.42;
                k += 1;
            }
        }
    }

    /// Gitterbox im Aktionsbereich, gefuellt mit zufaelliger Aktionsware.
    pub(super) fn gitterbox(&mut self, c: Vec3) {
        let metal = vec3(0.55, 0.56, 0.58);
        let (hx, hy, hz) = (0.5, 0.45, 0.4);
        for (sx, sz) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            self.b.add(
                c + vec3(sx * hx, hy, sz * hz),
                vec3(0.015, hy, 0.015),
                metal,
                PLAIN,
            );
        }
        for y in [0.03, hy * 2.0] {
            self.b
                .add(c + vec3(0.0, y, -hz), vec3(hx, 0.01, 0.01), metal, PLAIN);
            self.b
                .add(c + vec3(0.0, y, hz), vec3(hx, 0.01, 0.01), metal, PLAIN);
            self.b
                .add(c + vec3(-hx, y, 0.0), vec3(0.01, 0.01, hz), metal, PLAIN);
            self.b
                .add(c + vec3(hx, y, 0.0), vec3(0.01, 0.01, hz), metal, PLAIN);
        }
        // Gitter-Andeutung
        for k in 1..6 {
            let x = -hx + k as f32 * (2.0 * hx / 6.0);
            self.b.add(
                c + vec3(x, hy, hz),
                vec3(0.004, hy, 0.004),
                metal * 0.8,
                PLAIN,
            );
            self.b.add(
                c + vec3(x, hy, -hz),
                vec3(0.004, hy, 0.004),
                metal * 0.8,
                PLAIN,
            );
        }
        self.b.add(
            c + vec3(0.0, 0.06, 0.0),
            vec3(hx, 0.02, hz),
            metal * 0.7,
            PLAIN,
        );
        let pool = ["haushalt", "snacks", "suess", "getraenke"];
        let sub = self.cat.sub_index(pool[self.rng.gen_range(0..pool.len())]);
        let Some(p) = self.pick(sub, true, 0.35, 0.8) else {
            return;
        };
        let half = self.cat.products[p].half;
        let cols = ((0.95 / (half.x * 2.0 + 0.01)) as usize).clamp(1, 8);
        let rows = ((0.75 / (half.z * 2.0 + 0.01)) as usize).clamp(1, 6);
        for layer in 0..2 {
            let y = 0.08 + layer as f32 * (half.y * 2.0 + 0.01);
            for ci in 0..cols {
                for ri in 0..rows {
                    let x = -0.47 + (ci as f32 + 0.5) * (0.94 / cols as f32);
                    let z = -0.37 + (ri as f32 + 0.5) * (0.74 / rows as f32);
                    let jitter = self.weird.gen_range(-0.5..0.5);
                    self.place(p, c + vec3(x, y, z), jitter);
                }
            }
        }
        self.b
            .solid(c + vec3(0.0, hy, 0.0), vec3(hx + 0.05, hy, hz + 0.05));
    }

    /// Ein Stueck Lichtband an der Decke (+ Punktlicht). Im gekippten Markt tot oder flackernd.
    pub(super) fn ceiling_lamp(&mut self, x: f32, z: f32, color: Vec3) {
        let seed = x * 1.37 + z * 0.71;
        let (dead, flicker) = if self.params.tipped {
            let dead = self
                .weird
                .gen_bool((0.3 + self.params.darkness as f64).min(0.95));
            let flicker = if self.weird.gen_bool(0.55) {
                self.weird.gen_range(0.25..0.8)
            } else {
                0.0
            };
            (dead, flicker)
        } else {
            (false, if self.weird.gen_bool(0.03) { 0.12 } else { 0.0 })
        };
        let y = CEILING_HEIGHT;
        // durchgehendes Lichtband: Stuecke von 3 m, aneinandergereiht
        self.b.add(
            vec3(x, y - 0.03, z),
            vec3(0.09, 0.03, 1.5),
            vec3(0.62, 0.63, 0.62),
            PLAIN,
        );
        let tube_c = vec3(x, y - 0.065, z);
        let tube_h = vec3(0.06, 0.008, 1.47);
        if dead {
            self.b.add(tube_c, tube_h, vec3(0.2, 0.2, 0.19), PLAIN);
        } else {
            self.b
                .add_ex(tube_c, tube_h, color, EMISSIVE, seed, 1.7, flicker);
            let intensity = if self.params.tipped { 0.55 } else { 0.6 };
            self.b
                .light(vec3(x, y - 0.5, z), 7.5, color, intensity, seed, flicker);
        }
    }
}

pub(super) fn sub_at(list: &[(usize, f32, f32)], t: f32) -> usize {
    list.iter()
        .find(|x| t >= x.1 && t < x.2)
        .or(list.last())
        .map(|x| x.0)
        .unwrap_or(0)
}
