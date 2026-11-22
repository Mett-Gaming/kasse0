//! Die Welt: ein Discounter "KAUF NULL", seedbar erzeugt (siehe gen.rs).
//!
//! Koordinaten: x nach Osten, y nach oben, z nach Sueden. Die Nordwand liegt bei z = 0,
//! der Eingang in der Suedwand. Die Getraenkeabteilung ist ein Raum in der Nordost-Ecke.

mod gen;
pub mod layout;

pub use gen::generate;
pub use gen::POSTER_TEXTS;

use crate::catalog::catalog;
use crate::render::MeshDraw;
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Mat4, Quat, Vec3, Vec4};
use std::collections::HashMap;

pub const CEILING_HEIGHT: f32 = 4.0;
/// Breite der Durchgaenge zwischen gebogenen Regalteilen: zu Fuss ja, mit Wagen nein
pub const PASSAGE_WIDTH: f32 = 0.75;
pub const GONDOLA_BOARDS: &[f32] = &[0.12, 0.55, 0.98, 1.41, 1.84];
pub const COOLER_BOARDS: &[f32] = &[0.25, 0.65, 1.05, 1.45];

pub mod material {
    pub const PLAIN: f32 = 0.0;
    pub const FLOOR: f32 = 1.0;
    pub const CEILING: f32 = 2.0;
    pub const EMISSIVE: f32 = 3.0;
    pub const POSTER: f32 = 4.0;
    /// Schild mit Schriftzug aus dem Atlas (extra.y = Zeile, color.a = Textfarbe)
    pub const SIGN: f32 = 5.0;
    /// Ueberwachungsmonitor: zeigt das Bild der Kassenkamera (extra.z = Helligkeit)
    pub const MONITOR: f32 = 6.0;
    /// Spiegel am Ende des Fluchtkorridors: Bild der zweiten Kamera, gespiegelt und in Farbe
    pub const MIRROR: f32 = 7.0;
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuInstance {
    /// xyz, w = Drehung um die Hochachse
    pub center: [f32; 4],
    pub half: [f32; 4],
    pub color: [f32; 4],
    /// x = Material, y = Seed, z = Leuchtstaerke, w = Flackern
    pub extra: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct GpuLight {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    pub extra: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn from_center(c: Vec3, h: Vec3) -> Self {
        Self {
            min: c - h,
            max: c + h,
        }
    }
    pub fn contains_xz(&self, p: Vec3) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.z >= self.min.z && p.z <= self.max.z
    }
    /// Strahl-Test (Slab). Liefert den Eintrittsparameter t in [0, max_t].
    pub fn ray(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<f32> {
        let mut t0 = 0.0f32;
        let mut t1 = max_t;
        for i in 0..3 {
            let (oi, di, lo, hi) = (o[i], d[i], self.min[i], self.max[i]);
            if di.abs() < 1e-8 {
                if oi < lo || oi > hi {
                    return None;
                }
            } else {
                let inv = 1.0 / di;
                let (mut a, mut b) = ((lo - oi) * inv, (hi - oi) * inv);
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                t0 = t0.max(a);
                t1 = t1.min(b);
                if t0 > t1 {
                    return None;
                }
            }
        }
        Some(t0)
    }
}

/// Um die Hochachse gedrehte Box. Lokale x-Achse = (cos, 0, -sin), lokale z-Achse = (sin, 0, cos).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obb {
    pub center: Vec3,
    pub half: Vec3,
    pub yaw: f32,
}

impl Obb {
    pub fn to_local(&self, p: Vec3) -> Vec3 {
        let d = p - self.center;
        let (s, c) = self.yaw.sin_cos();
        vec3(d.x * c - d.z * s, d.y, d.x * s + d.z * c)
    }
    pub fn to_world_dir(&self, v: Vec3) -> Vec3 {
        let (s, c) = self.yaw.sin_cos();
        vec3(v.x * c + v.z * s, v.y, -v.x * s + v.z * c)
    }
    pub fn ray(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<f32> {
        let lo = self.to_local(o);
        let (s, c) = self.yaw.sin_cos();
        let ld = vec3(d.x * c - d.z * s, d.y, d.x * s + d.z * c);
        Aabb {
            min: -self.half,
            max: self.half,
        }
        .ray(lo, ld, max_t)
    }
    pub fn min_y(&self) -> f32 {
        self.center.y - self.half.y
    }
    pub fn max_y(&self) -> f32 {
        self.center.y + self.half.y
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldParams {
    pub seed: u64,
    pub tipped: bool,
    /// Faktor fuer die Ganglaenge (1.0 normal)
    pub stretch: f32,
    /// Zusaetzliche Wahrscheinlichkeit fuer tote Roehren (0..1)
    pub darkness: f32,
    /// Neu sortierte Ware/Lichter bei gleichem Grundriss
    pub shuffle: u64,
    pub door_open: bool,
    pub open_lane: usize,
    /// 0..1: wie stark die Gaenge gebogen sind
    pub warp: f32,
    /// Anzahl neuer Querwaende in Gaengen
    pub walls: u32,
    /// Sortiment-Mischstufe (siehe layout::MixLevel)
    pub mix: u8,
    /// Loecher im Boden (Hindernisse) und in der Decke
    pub holes: u32,
    pub ceiling_holes: u32,
    /// Anteil verdorbener Ware im Regal (0..1)
    pub rot: f32,
    /// riesige Waren als Hindernisse in den Gaengen
    pub giants: u32,
    /// Verfallsstufe 0..3 (Fliesen, Ameisen, Ratten, Blut; siehe gen/decay.rs)
    pub decay: u8,
    /// Zufall fuer die Hindernisse im Fluchtkorridor (wird bei jedem Bezahlen neu gewuerfelt)
    pub corridor_seed: u64,
}

impl WorldParams {
    pub fn normal(seed: u64) -> Self {
        Self {
            seed,
            tipped: false,
            stretch: 1.0,
            darkness: 0.0,
            shuffle: 0,
            door_open: false,
            open_lane: 0,
            warp: 0.0,
            walls: 0,
            mix: 0,
            holes: 0,
            ceiling_holes: 0,
            rot: 0.0,
            giants: 0,
            decay: 0,
            corridor_seed: 0,
        }
    }
}

/// Gangbereich beginnt immer hier (siehe gen.rs: aisles_x0, aisle_z0)
pub const AISLES_X0: f32 = 2.5;
pub const AISLES_Z0: f32 = 4.0;

/// Navigationsraster der NPCs: Zellgroesse und Abstand zu Hindernissen (Meter)
pub const NAV_CELL: f32 = 0.25;
pub const NAV_CLEARANCE: f32 = 0.18;

/// Live-Verbiegen der Gaenge (gleiche Formel im Shader, siehe _common.wgsl `sway`):
/// x = seitliches Schwanken der Regale, y = Hoehenwelle des Bodens (Steigung).
/// `w` = (seitlich, hoch, Gangende z, Gangende x), `t` = Spielzeit. Ausserhalb des
/// Gangbereichs ist alles fest, damit Boden und Waende zusammenpassen.
pub fn sway(w: Vec4, p: Vec3, t: f32) -> Vec3 {
    if w.x == 0.0 && w.y == 0.0 {
        return Vec3::ZERO;
    }
    let smooth = |a: f32, b: f32, x: f32| {
        let k = ((x - a) / (b - a)).clamp(0.0, 1.0);
        k * k * (3.0 - 2.0 * k)
    };
    let mask = smooth(AISLES_Z0, AISLES_Z0 + 2.0, p.z)
        * (1.0 - smooth(w.z - 2.0, w.z, p.z))
        * smooth(AISLES_X0 - 1.0, AISLES_X0 + 0.5, p.x)
        * (1.0 - smooth(w.w - 0.5, w.w + 1.0, p.x));
    let lat = w.x * mask * (p.z * 0.45 + t * 0.7).sin() * (t * 0.27 + 1.3).sin();
    let up = w.y * mask * (p.z * 0.38 - t * 0.35 + 0.8).sin();
    Vec3::new(lat, up, 0.0)
}

#[derive(Clone, Copy, Debug)]
pub struct Lane {
    /// Wo der Spieler zum Scannen steht
    pub scan_point: Vec3,
    /// Position der Kasse (Modell)
    pub register: Vec3,
    pub open: bool,
    /// Band: Anfang (Ablage) und Ende (Scanner), Oberkante
    pub belt_start: Vec3,
    pub belt_end: Vec3,
    /// Ablage nach dem Scanner (bezahlte Ware)
    pub bag: Vec3,
    /// Kartenterminal auf der Stange
    pub terminal: Vec3,
    /// Stuhl der Kassierer:in (Boden), Blick nach -x
    pub seat: Vec3,
    /// Zigaretten-Rollo (Mitte unten), Front nach -x
    pub shutter: Vec3,
    /// Ueberwachungskamera an der Decke
    pub camera: Vec3,
    /// Mitte des Monitorbilds (haengt hinter dem Scanner, Bild nach -z)
    pub monitor: Vec3,
}

/// Ein Kundenplatz vor einem Tomatensaft-Regal.
#[derive(Clone, Copy, Debug)]
pub struct Spot {
    pub pos: Vec3,
    pub facing: Vec3,
    pub shelf: Vec3,
}

/// Eine echte Ware im Laden (greifbar).
#[derive(Clone, Copy, Debug)]
pub struct ShelfItem {
    pub product: u32,
    pub pos: Vec3,
    pub yaw: f32,
    /// falsch beschriftete Anomalie
    pub fake: bool,
    pub taken: bool,
    /// Verfall: 0 frisch, 1 verdorben (Maden, Fliegen), 2 kurios (schwebt, gekippt, kopfueber)
    pub rot: u8,
}

/// Riesige Ware als Hindernis im Gang (nicht greifbar).
#[derive(Clone, Copy, Debug)]
pub struct Giant {
    pub product: u32,
    pub pos: Vec3,
    pub yaw: f32,
    pub scale: f32,
}

impl ShelfItem {
    pub fn half(&self) -> Vec3 {
        catalog().products[self.product as usize].half
    }
}

/// Begehbarkeitsraster (0.5 m) fuer die NPC-Navigation.
#[derive(Clone)]
pub struct NavGrid {
    pub origin: Vec3,
    pub cell: f32,
    pub w: usize,
    pub h: usize,
    pub walkable: Vec<bool>,
    /// Zellen, die NPCs nicht betreten (Kassenbereich)
    pub npc_block: Vec<bool>,
}

impl NavGrid {
    pub fn cell_of(&self, p: Vec3) -> Option<(usize, usize)> {
        let x = ((p.x - self.origin.x) / self.cell).floor();
        let z = ((p.z - self.origin.z) / self.cell).floor();
        if x < 0.0 || z < 0.0 || x as usize >= self.w || z as usize >= self.h {
            return None;
        }
        Some((x as usize, z as usize))
    }
    pub fn center(&self, x: usize, z: usize) -> Vec3 {
        vec3(
            self.origin.x + (x as f32 + 0.5) * self.cell,
            0.0,
            self.origin.z + (z as f32 + 0.5) * self.cell,
        )
    }
    pub fn is_walkable(&self, x: usize, z: usize) -> bool {
        x < self.w && z < self.h && self.walkable[z * self.w + x]
    }
    /// Begehbar fuer NPCs: wie `is_walkable`, aber der Kassenbereich ist fuer sie gesperrt.
    pub fn npc_walkable(&self, x: usize, z: usize) -> bool {
        self.is_walkable(x, z) && !self.npc_block.get(z * self.w + x).copied().unwrap_or(false)
    }
}

impl World {
    /// Kassenbereich (Gassen mit etwas Rand davor und dahinter): (x von, x bis, z Nord, z Sued).
    pub fn checkout_zone(&self) -> Option<(f32, f32, f32, f32)> {
        let lanes = &self.lanes;
        if lanes.is_empty() {
            return None;
        }
        let xs: Vec<f32> = lanes.iter().map(|l| l.belt_start.x).collect();
        let spacing = if xs.len() > 1 {
            (xs[1] - xs[0]).abs()
        } else {
            4.0
        };
        let lo = xs.iter().copied().fold(f32::MAX, f32::min) - spacing * 0.5;
        let hi = xs.iter().copied().fold(f32::MIN, f32::max) + spacing * 0.5;
        let z0 = lanes[0].belt_start.z - 0.15 - CHECKOUT_GAP;
        let z1 = z0 + CHECKOUT_GAP * 2.0 + 3.0;
        Some((lo.max(0.05), hi, z0, z1))
    }

    /// Liegt der Punkt (mit Radius `r`) im Kassenbereich?
    pub fn in_checkout_zone(&self, p: Vec3, r: f32) -> bool {
        self.checkout_zone().is_some_and(|(x0, x1, z0, z1)| {
            p.x > x0 - r && p.x < x1 + r && p.z > z0 - r && p.z < z1 + r
        })
    }

    /// Kassenbereich im Navigationsraster fuer NPCs sperren.
    pub fn block_checkout_for_npcs(&mut self) {
        let nav = &self.nav;
        let mut block = vec![false; nav.w * nav.h];
        for z in 0..nav.h {
            for x in 0..nav.w {
                block[z * nav.w + x] = self.in_checkout_zone(nav.center(x, z), 0.25);
            }
        }
        self.nav.npc_block = block;
    }
}

/// Rand des Kassenbereichs vor und hinter den Gassen (dort steht auch das Absperrband)
pub const CHECKOUT_GAP: f32 = 0.4;

pub struct World {
    pub instances: Vec<GpuInstance>,
    pub lights: Vec<GpuLight>,
    pub solids: Vec<Aabb>,
    pub obbs: Vec<Obb>,
    /// Durchgaenge in gebogenen Regalen: (Mitte, Richtung quer durch)
    pub passages: Vec<(Vec3, Vec3)>,
    /// alle echten Waren
    pub items: Vec<ShelfItem>,
    item_grid: HashMap<(i32, i32), Vec<u32>>,
    pub layout: layout::Layout,
    pub drinks_zone: Aabb,
    pub spawn: Vec3,
    pub spawn_yaw: f32,
    pub size: Vec3,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub aisle_count: u32,
    pub aisle_z0: f32,
    pub aisle_z1: f32,
    /// Ende des Gangbereichs in x (Beginn: AISLES_X0)
    pub aisle_x1: f32,
    pub lanes: Vec<Lane>,
    pub door_center: Vec3,
    /// Bereich, in dem der Spieler draussen ist (Ende des Korridors bzw. hinter der Tuer)
    pub exit_zone: Aabb,
    /// Fluchtkorridor hinter der Tuer (nur nach dem Bezahlen im gekippten Markt)
    pub corridor: Option<gen::Corridor>,
    pub spots: Vec<Spot>,
    /// Kuehltruhen-Stuecke: (Mitte, Laengsrichtung, Laenge)
    pub freezers: Vec<(Vec3, f32, f32)>,
    pub nav: NavGrid,
    /// Deko: Warnschilder "Rutschgefahr"
    pub signs: Vec<(Vec3, f32)>,
    /// riesige Waren in den Gaengen
    pub giants: Vec<Giant>,
    /// verdorbene Ware (Index in items), um die Fliegen kreisen
    pub rotten: Vec<usize>,
    /// Ameisenstrassen (von, bis), Rattenwege (von, bis), tropfendes Blut (Stelle an der Decke)
    pub ant_trails: Vec<(Vec3, Vec3)>,
    pub rat_runs: Vec<(Vec3, Vec3)>,
    pub drips: Vec<Vec3>,
    /// Kabelenden, die Funken spruehen
    pub sparks: Vec<Vec3>,
    /// Mitten der Deckenloecher (fuer Schrecken)
    pub ceiling_holes: Vec<Vec3>,
    /// Wagenbox am Eingang
    pub corral: Vec3,
}

const ITEM_CELL: f32 = 1.0;

fn grid_key(p: Vec3) -> (i32, i32) {
    (
        (p.x / ITEM_CELL).floor() as i32,
        (p.z / ITEM_CELL).floor() as i32,
    )
}

impl World {
    fn index_items(&mut self) {
        self.item_grid.clear();
        self.rotten = (0..self.items.len())
            .filter(|&i| self.items[i].rot == 1)
            .collect();
        for (i, it) in self.items.iter().enumerate() {
            self.item_grid
                .entry(grid_key(it.pos))
                .or_default()
                .push(i as u32);
        }
    }

    /// Naechste (nicht genommene) Ware entlang eines Strahls.
    pub fn item_ray(&self, o: Vec3, d: Vec3, max_t: f32) -> Option<(usize, f32)> {
        let (k0, k1) = (
            grid_key(o - Vec3::splat(max_t)),
            grid_key(o + Vec3::splat(max_t)),
        );
        let mut best: Option<(usize, f32)> = None;
        for gx in k0.0..=k1.0 {
            for gz in k0.1..=k1.1 {
                let Some(list) = self.item_grid.get(&(gx, gz)) else {
                    continue;
                };
                for &i in list {
                    let it = &self.items[i as usize];
                    if it.taken {
                        continue;
                    }
                    let h = it.half();
                    let r = h.x.max(h.z) + 0.02;
                    let bx =
                        Aabb::from_center(it.pos + vec3(0.0, h.y, 0.0), vec3(r, h.y + 0.02, r));
                    if let Some(t) = bx.ray(o, d, best.map_or(max_t, |b| b.1)) {
                        best = Some((i as usize, t));
                    }
                }
            }
        }
        best
    }

    /// Instanzen fuer den Renderer (statisch, neu bei Weltwechsel oder genommener Ware).
    pub fn item_draws(&self, out: &mut Vec<MeshDraw>) {
        let cat = catalog();
        for it in &self.items {
            if it.taken {
                continue;
            }
            let p = &cat.products[it.product as usize];
            let mut tint = if it.fake {
                p.tint * vec3(0.7, 1.05, 0.85)
            } else {
                p.tint
            };
            let mut model = Mat4::from_rotation_translation(Quat::from_rotation_y(it.yaw), it.pos);
            match it.rot {
                // verdorben: braun-gruenlich, fleckig
                1 => tint = tint * vec3(0.5, 0.55, 0.3) + vec3(0.04, 0.05, 0.0),
                // kurios: schwebt, gekippt oder kopfueber
                2 => {
                    let h = hash01(it.pos);
                    let lift = 0.04 + h * 0.08;
                    let tilt = if h > 0.6 {
                        std::f32::consts::PI
                    } else {
                        0.5 + h
                    };
                    let up = vec3(0.0, p.half.y, 0.0);
                    model = Mat4::from_translation(it.pos + up + vec3(0.0, lift, 0.0))
                        * Mat4::from_quat(
                            Quat::from_rotation_y(it.yaw + h * 9.0) * Quat::from_rotation_x(tilt),
                        )
                        * Mat4::from_translation(-up);
                }
                _ => {}
            }
            out.push(MeshDraw {
                mesh: p.mesh,
                model,
                tint: Vec4::new(tint.x, tint.y, tint.z, 0.0),
            });
        }
        // riesige Waren in den Gaengen
        for g in &self.giants {
            let p = &cat.products[g.product as usize];
            out.push(MeshDraw {
                mesh: p.mesh,
                model: Mat4::from_scale_rotation_translation(
                    Vec3::splat(g.scale),
                    Quat::from_rotation_y(g.yaw),
                    g.pos,
                ),
                tint: (p.tint * vec3(0.75, 0.72, 0.68)).extend(0.0),
            });
        }
    }

    /// Schiebt einen Kreis (Radius `r`) in der XZ-Ebene aus allen Hindernissen.
    pub fn collide(&self, mut p: Vec3, r: f32) -> Vec3 {
        for _ in 0..4 {
            let mut moved = false;
            for b in &self.solids {
                if b.max.y < 0.2 || b.min.y > 1.7 {
                    continue;
                }
                let cx = p.x.clamp(b.min.x, b.max.x);
                let cz = p.z.clamp(b.min.z, b.max.z);
                let (dx, dz) = (p.x - cx, p.z - cz);
                let d2 = dx * dx + dz * dz;
                if d2 >= r * r {
                    continue;
                }
                if d2 > 1e-10 {
                    let d = d2.sqrt();
                    let push = (r - d) / d;
                    p.x += dx * push;
                    p.z += dz * push;
                } else {
                    let left = p.x - b.min.x;
                    let right = b.max.x - p.x;
                    let north = p.z - b.min.z;
                    let south = b.max.z - p.z;
                    let m = left.min(right).min(north).min(south);
                    if m == left {
                        p.x = b.min.x - r;
                    } else if m == right {
                        p.x = b.max.x + r;
                    } else if m == north {
                        p.z = b.min.z - r;
                    } else {
                        p.z = b.max.z + r;
                    }
                }
                moved = true;
            }
            for o in &self.obbs {
                if o.max_y() < 0.2 || o.min_y() > 1.7 {
                    continue;
                }
                let l = o.to_local(p);
                let cx = l.x.clamp(-o.half.x, o.half.x);
                let cz = l.z.clamp(-o.half.z, o.half.z);
                let (dx, dz) = (l.x - cx, l.z - cz);
                let d2 = dx * dx + dz * dz;
                if d2 >= r * r {
                    continue;
                }
                let push_local = if d2 > 1e-10 {
                    let d = d2.sqrt();
                    vec3(dx, 0.0, dz) * ((r - d) / d)
                } else {
                    // quer zum Regal hinaus (entlang wuerde nur ins naechste Teilstueck schieben)
                    let side = if l.x.abs() < 1e-6 { 1.0 } else { l.x.signum() };
                    vec3((o.half.x - l.x.abs() + r) * side, 0.0, 0.0)
                };
                p += o.to_world_dir(push_local);
                moved = true;
            }
            if !moved {
                break;
            }
        }
        p
    }

    /// Ist die Sicht zwischen a und b frei?
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return true;
        }
        let dir = d / len;
        !self.solids.iter().any(|s| {
            s.max.y > 0.05 && s.min.y < CEILING_HEIGHT - 0.1 && s.ray(a, dir, len - 0.05).is_some()
        }) && !self
            .obbs
            .iter()
            .any(|o| o.ray(a, dir, len - 0.05).is_some())
    }

    /// Bildet eine Position aus einer Welt mit anderer Ganglaenge auf diese ab.
    pub fn remap_from(&self, old: &World, p: Vec3) -> Vec3 {
        let mut q = p;
        if p.z >= old.aisle_z0 && p.z <= old.aisle_z1 {
            let t = (p.z - old.aisle_z0) / (old.aisle_z1 - old.aisle_z0).max(0.01);
            q.z = self.aisle_z0 + t * (self.aisle_z1 - self.aisle_z0);
        } else if p.z > old.aisle_z1 {
            q.z = p.z - old.aisle_z1 + self.aisle_z1;
        }
        q
    }

    /// Erreichbare Navigationszellen ab dem Eingang.
    fn reachable_cells(&self) -> Vec<bool> {
        let nav = &self.nav;
        let mut seen = vec![false; nav.w * nav.h];
        let Some(start) = nav.cell_of(self.spawn) else {
            return seen;
        };
        let mut q = std::collections::VecDeque::from([start]);
        seen[start.1 * nav.w + start.0] = true;
        while let Some((x, z)) = q.pop_front() {
            for (dx, dz) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, nz) = (x as i32 + dx, z as i32 + dz);
                if nx < 0 || nz < 0 {
                    continue;
                }
                let (nx, nz) = (nx as usize, nz as usize);
                if nav.is_walkable(nx, nz) && !seen[nz * nav.w + nx] {
                    seen[nz * nav.w + nx] = true;
                    q.push_back((nx, nz));
                }
            }
        }
        seen
    }

    fn near_reachable(&self, seen: &[bool], p: Vec3, r: f32) -> bool {
        let nav = &self.nav;
        let steps = (r / nav.cell).ceil() as i32;
        let Some((cx, cz)) = nav.cell_of(vec3(
            p.x.clamp(0.0, self.size.x - 0.01),
            0.0,
            p.z.clamp(0.0, self.size.z - 0.01),
        )) else {
            return false;
        };
        (-steps..=steps).any(|dz| {
            (-steps..=steps).any(|dx| {
                let (x, z) = (cx as i32 + dx, cz as i32 + dz);
                x >= 0
                    && z >= 0
                    && (x as usize) < nav.w
                    && (z as usize) < nav.h
                    && seen[z as usize * nav.w + x as usize]
            })
        })
    }

    /// Ist eine Ware vom Eingang aus zu Fuss erreichbar?
    pub fn item_reachable(&self, seen: &[bool], i: usize) -> bool {
        self.near_reachable(seen, self.items[i].pos, 1.4)
    }

    /// Sind Getraenkeraum, Kassen, Wagenbox und je Zettelpunkt eine billige und eine teure Ware erreichbar?
    pub fn all_reachable(&self) -> bool {
        let seen = self.reachable_cells();
        let drinks = vec3(
            self.drinks_zone.min.x + 1.7,
            0.0,
            self.drinks_zone.max.z - 0.8,
        );
        if !(self.near_reachable(&seen, drinks, 0.6)
            && self.near_reachable(&seen, self.corral, 1.2)
            && self
                .lanes
                .iter()
                .all(|l| self.near_reachable(&seen, l.scan_point, 0.6)))
        {
            return false;
        }
        let cat = catalog();
        cat.list.iter().all(|l| {
            [&l.cheap, &l.pricey].iter().all(|variants| {
                self.items.iter().enumerate().any(|(i, it)| {
                    !it.fake
                        && variants.contains(&(it.product as usize))
                        && self.item_reachable(&seen, i)
                })
            })
        })
    }
}

fn build_nav(solids: &[Aabb], obbs: &[Obb], width: f32, depth: f32) -> NavGrid {
    // Feines Raster: auch die schmalen Durchgaenge (0,75 m) zwischen versetzten Regalen
    // sind fuer NPCs (Radius 0,25 m) begehbar.
    let cell = NAV_CELL;
    let origin = Vec3::ZERO;
    let w = (width / cell).ceil() as usize;
    let h = (depth / cell).ceil() as usize;
    let r = NAV_CLEARANCE;
    let mut walkable = vec![true; w * h];
    let range = |lo: f32, hi: f32, n: usize| {
        let a = ((lo / cell) - 0.5).ceil().max(0.0) as usize;
        let b = (((hi / cell) - 0.5).floor().max(-1.0) + 1.0) as usize;
        (a.min(n), b.min(n))
    };
    // Rechteckige Hindernisse: alle Zellmitten im erweiterten Rechteck sperren
    for s in solids {
        if s.max.y < 0.2 || s.min.y > 1.7 {
            continue;
        }
        let (x0, x1) = range(s.min.x - r + 1e-4, s.max.x + r - 1e-4, w);
        let (z0, z1) = range(s.min.z - r + 1e-4, s.max.z + r - 1e-4, h);
        for z in z0..z1 {
            for x in x0..x1 {
                walkable[z * w + x] = false;
            }
        }
    }
    // Gedrehte Hindernisse: Umriss grob, dann genau pruefen
    for o in obbs {
        if o.max_y() < 0.2 {
            continue;
        }
        let ext = o.half.x.max(o.half.z) * 1.42 + r;
        let (x0, x1) = range(o.center.x - ext, o.center.x + ext, w);
        let (z0, z1) = range(o.center.z - ext, o.center.z + ext, h);
        for z in z0..z1 {
            for x in x0..x1 {
                let c = vec3((x as f32 + 0.5) * cell, 0.0, (z as f32 + 0.5) * cell);
                let l = o.to_local(c);
                if l.x.abs() < o.half.x + r && l.z.abs() < o.half.z + r {
                    walkable[z * w + x] = false;
                }
            }
        }
    }
    NavGrid {
        origin,
        cell,
        w,
        h,
        walkable,
        npc_block: Vec::new(),
    }
}

#[cfg(test)]
mod tests;

/// Fester Pseudo-Zufall 0..1 aus einer Position (fuer Verfallsvarianten).
pub fn hash01(p: Vec3) -> f32 {
    ((p.x * 12.9898 + p.z * 78.233 + p.y * 37.719).sin() * 43_758.547)
        .fract()
        .abs()
}
