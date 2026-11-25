//! Modelle im .k0m-Format (erzeugt von tools/prepare_assets.py) - in den Spieldaten.

use glam::{vec3, Vec3};
use mettgine::asset::data;

pub use mettgine::gpu::{MeshData, MeshVertex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum MeshId {
    Cube = 0,
    Cart,
    Register,
    WetFloorSign,
    Can,
    /// Mannequin in Sitzpose: frueher die Kassiererin, jetzt steht dort eine Personal-Figur
    /// (people.rs). Bleibt geladen, damit die Reihenfolge der Modelle gleich bleibt.
    #[allow(dead_code)]
    Cashier,
    Chair,
    Shutter,
    Camera,
    Terminal,
    Cigarettes,
    Rat,
    /// einzelne Zigarette (smoke.rs): 8,5 cm laengs z, Filter bei -z, Glutspitze bei +z
    Cigarette,
    /// rostige Maschine am Kiosk (Poly Haven, CC0): 0,74 x 1,45 x 2,08 m, lang in z
    Machine,
    /// liegendes Menschen-Skelett neben der Maschine (OpenGameArt, CC0): 1,75 m lang in x, Kopf bei -x
    Skeleton,
    /// Pfandtuete am Wagen (Quaternius Trash Bag, CC0): 0,32 x 0,36 x 0,32 m, Boden bei y = 0
    PfandBag,
}

/// Anzahl fester Modelle (danach folgen die Waren aus dem Katalog)
pub const FIXED_COUNT: usize = MeshId::PfandBag as usize + 1;

mod gen {
    include!(concat!(env!("OUT_DIR"), "/products_gen.rs"));
}
pub use gen::{CATALOG_JSON, PRODUCT_MESH_IDS};

pub use crate::people::people_of;

const SOURCES: [&str; FIXED_COUNT - 1] = [
    "cart.k0m",
    "register.k0m",
    "wet_floor_sign.k0m",
    "can.k0m",
    "cashier.k0m",
    "chair.k0m",
    "shutter.k0m",
    "camera.k0m",
    "terminal.k0m",
    "cigarettes.k0m",
    "rat.k0m",
    "cigarette.k0m",
    "machine.k0m",
    "skeleton.k0m",
    "pfand_bag.k0m",
];

/// Alle Modelle: feste (Index = MeshId) gefolgt von den Waren (Index = FIXED_COUNT + Katalog-Mesh).
pub fn load_all() -> Vec<MeshData> {
    let mut v = vec![cube()];
    v.extend(SOURCES.iter().map(|n| data::load::<MeshData>(n)));
    add_cart_basket(&mut v[MeshId::Cart as usize]);
    // Warnschild: Rohmodell fuer die gruseligen Fassungen merken, dann Aufschrift drauf
    let s = &v[MeshId::WetFloorSign as usize];
    let (raw_verts, raw_min, raw_max) = (s.vertices.clone(), s.min, s.max);
    v[MeshId::WetFloorSign as usize]
        .vertices
        .extend(crate::wet_sign::decal(0));
    v.extend(
        PRODUCT_MESH_IDS
            .iter()
            .map(|id| data::load::<MeshData>(&format!("products/{id}.k0m"))),
    );
    // nach den Waren: die Warnschild-Fassungen 1 und 2 (siehe `wet_sign_mesh`)
    for variant in 1..crate::wet_sign::VARIANTS {
        let mut m = MeshData {
            vertices: raw_verts.clone(),
            min: raw_min,
            max: raw_max,
        };
        m.vertices.extend(crate::wet_sign::decal(variant));
        v.push(m);
    }
    v
}

/// Modell des Warnschilds in der Fassung `variant` (0 normal, 1 unheimlich, 2 grauenhaft).
pub fn wet_sign_mesh(variant: usize) -> usize {
    match variant.min(crate::wet_sign::VARIANTS - 1) {
        0 => MeshId::WetFloorSign as usize,
        v => FIXED_COUNT + PRODUCT_MESH_IDS.len() + v - 1,
    }
}

// Einkaufswagen aus Drahtkorb. Das Modell (Poly by Google) ist nur ein Rahmen aus Stangen:
// Rand auf 0,57 m und eine untere Ablage, ohne Korbboden und ohne Korbwaende - Waren darin
// sahen schwebend aus. Darum fliegt der Rahmen raus; Raeder, Radgabeln, Pfosten und Griff
// bleiben, dazu kommen ein Gitterkorb und ein Gestell aus duennen Staeben (Modellkoordinaten).

/// Korb innen: halbe Breite (x, Meter)
pub const CART_BASKET_X: f32 = 0.262;
/// Korb vorne / hinten (z, Meter; der Griff liegt bei +z)
pub const CART_BASKET_Z0: f32 = -0.49;
pub const CART_BASKET_Z1: f32 = 0.232;
/// Oberkante des Gitterbodens = darauf liegen die Waren (Meter)
pub const CART_BASKET_FLOOR: f32 = 0.57;
/// Oberkante der Korbwaende (knapp unter dem Griff, Meter)
pub const CART_BASKET_TOP: f32 = 0.80;
/// halbe Dicke eines Drahts (Meter)
const ROD: f32 = 0.003;
/// Laengsdraehte im Boden, Staebe je Seitenwand, Staebe je Stirnwand
const FLOOR_RODS: usize = 12;
const SIDE_RODS: usize = 16;
const END_RODS: usize = 12;

/// Ein Draht des Korbs als Kasten: Mitte und halbe Groesse (Modellkoordinaten, Meter).
#[derive(Clone, Copy, Debug)]
pub struct Rod {
    pub center: Vec3,
    pub half: Vec3,
}

/// Alle Draehte des Korbs: Gitterboden (Oberkante genau auf CART_BASKET_FLOOR), vier Waende
/// mit senkrechten Staeben, Mittel- und Randstrebe rundherum.
pub fn cart_basket() -> Vec<Rod> {
    let (x1, z0, z1) = (CART_BASKET_X, CART_BASKET_Z0, CART_BASKET_Z1);
    let (floor, top) = (CART_BASKET_FLOOR, CART_BASKET_TOP);
    let zm = (z0 + z1) * 0.5;
    let rod = |center: Vec3, half: Vec3| Rod { center, half };
    let step = |a: f32, b: f32, n: usize, i: usize| a + (b - a) * i as f32 / (n - 1) as f32;
    let mut out = Vec::new();
    // Boden: Laengsdraehte obenauf, Querstreben darunter
    for i in 0..FLOOR_RODS {
        let x = step(-x1, x1, FLOOR_RODS, i);
        out.push(rod(
            vec3(x, floor - ROD, zm),
            vec3(ROD, ROD, (z1 - z0) * 0.5),
        ));
    }
    for i in 0..4 {
        let z = step(z0, z1, 4, i);
        out.push(rod(vec3(0.0, floor - 3.0 * ROD, z), vec3(x1, ROD, ROD)));
    }
    // Waende: senkrechte Staebe
    let ym = (floor + top) * 0.5;
    let up = vec3(ROD, (top - floor) * 0.5 + ROD, ROD);
    for i in 0..SIDE_RODS {
        let z = step(z0, z1, SIDE_RODS, i);
        out.push(rod(vec3(-x1, ym, z), up));
        out.push(rod(vec3(x1, ym, z), up));
    }
    for i in 1..END_RODS - 1 {
        let x = step(-x1, x1, END_RODS, i);
        out.push(rod(vec3(x, ym, z0), up));
        out.push(rod(vec3(x, ym, z1), up));
    }
    // Mittel- und Randstrebe rundherum
    for y in [ym, top] {
        for x in [-x1, x1] {
            out.push(rod(vec3(x, y, zm), vec3(ROD, ROD, (z1 - z0) * 0.5)));
        }
        for z in [z0, z1] {
            out.push(rod(vec3(0.0, y, z), vec3(x1, ROD, ROD)));
        }
    }
    out
}

/// Gestell, das den Korb mit Raedern und Griff verbindet (Modellkoordinaten, gemessen):
/// Radgabeln enden oben bei y 0,223 (vorn z -0,371, hinten z 0,134), Mitte x +-0,176;
/// die hinteren Pfosten laufen schraeg zum Griff und kreuzen die Bodenhoehe bei z ~0,26.
const LEG_X: f32 = 0.176;
const FORK_TOP: f32 = 0.215;
const FRONT_Z: f32 = -0.371;
const REAR_Z: f32 = 0.134;
const POST_Z: f32 = 0.262;

/// Gestell aus Staeben: zwei Laengsholme ueber den Radgabeln, eine Querstrebe vorn,
/// zwei Stuetzen vorn bis unter den Korb, hinten zwei kurze Verbinder zu den Pfosten.
pub fn cart_chassis() -> Vec<Rod> {
    let r = 1.8 * ROD;
    let floor = CART_BASKET_FLOOR - 4.0 * ROD;
    let mut out = Vec::new();
    for x in [-LEG_X, LEG_X] {
        // Laengsholm vorne bis hinten
        out.push(Rod {
            center: vec3(x, FORK_TOP, (FRONT_Z + REAR_Z) * 0.5),
            half: vec3(r, r, (REAR_Z - FRONT_Z) * 0.5 + r),
        });
        // Stuetze vorn: von der Radgabel bis unter den Gitterboden
        out.push(Rod {
            center: vec3(x, (FORK_TOP + floor) * 0.5, FRONT_Z),
            half: vec3(r, (floor - FORK_TOP) * 0.5 + r, r),
        });
        // hinten: Korb-Rueckwand an den Pfosten zum Griff
        for y in [floor, CART_BASKET_FLOOR + 0.08] {
            out.push(Rod {
                center: vec3(x, y, (CART_BASKET_Z1 + POST_Z) * 0.5),
                half: vec3(r, r, (POST_Z - CART_BASKET_Z1) * 0.5 + r),
            });
        }
    }
    out.push(Rod {
        center: vec3(0.0, FORK_TOP, FRONT_Z),
        half: vec3(LEG_X, r, r),
    });
    out
}

/// Ist das ein Teil des alten Rahmens (Rand, untere Ablage)? Erkennbar an seiner Farbe:
/// Raeder sind dunkel, Radgabeln, Pfosten und Griff heller als der Rahmen.
fn is_old_frame(v: &MeshVertex) -> bool {
    (FRAME_RED - 0.02..FRAME_RED + 0.02).contains(&v.color[0])
}
/// Rotanteil der Rahmenfarbe im Modell (gemessen)
const FRAME_RED: f32 = 0.62;

/// Wagen neu zusammensetzen: alten Rahmen entfernen (Raeder, Radgabeln, Pfosten und Griff
/// bleiben), dann Drahtkorb und Gestell in der Farbe des alten Rahmens dazu.
fn add_cart_basket(cart: &mut MeshData) {
    let color = cart
        .vertices
        .iter()
        .find(|v| is_old_frame(v))
        .map_or(Vec3::splat(0.6), |v| Vec3::from(v.color));
    let kept: Vec<MeshVertex> = cart
        .vertices
        .chunks_exact(3)
        .filter(|t| !is_old_frame(&t[0]))
        .flatten()
        .copied()
        .collect();
    cart.vertices = kept;
    for r in cart_basket().into_iter().chain(cart_chassis()) {
        push_box(&mut cart.vertices, r.center, r.half, color);
    }
    cart.min = Vec3::splat(f32::MAX);
    cart.max = Vec3::splat(f32::MIN);
    for v in &cart.vertices {
        cart.min = cart.min.min(Vec3::from(v.pos));
        cart.max = cart.max.max(Vec3::from(v.pos));
    }
}

/// Einen Kasten (Mitte, halbe Groesse) als 12 Dreiecke anhaengen.
fn push_box(vertices: &mut Vec<MeshVertex>, center: Vec3, half: Vec3, color: Vec3) {
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
    ];
    for (n, u, v) in faces {
        let (n, u, v) = (Vec3::from(n), Vec3::from(u), Vec3::from(v));
        for (a, b) in [
            (-1.0, -1.0),
            (1.0, -1.0),
            (1.0, 1.0),
            (-1.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
        ] {
            let p = center + (n + u * a + v * b) * half;
            vertices.push(MeshVertex {
                pos: p.to_array(),
                normal: n.to_array(),
                color: color.to_array(),
            });
        }
    }
}

pub fn mesh_count() -> usize {
    FIXED_COUNT + PRODUCT_MESH_IDS.len() + crate::wet_sign::VARIANTS - 1
}

/// Halbe Groesse (Bounding Box) eines Modells; Modelle stehen auf y = 0, Mitte x/z = 0.
pub fn half_extent_idx(index: usize) -> Vec3 {
    static HALVES: std::sync::OnceLock<Vec<Vec3>> = std::sync::OnceLock::new();
    let h = HALVES.get_or_init(|| load_all().iter().map(|m| (m.max - m.min) * 0.5).collect());
    h[index]
}

#[allow(dead_code)]
pub fn half_extent(id: MeshId) -> Vec3 {
    half_extent_idx(id as usize)
}

/// Einheitswuerfel [-0.5, 0.5]^3 fuer dynamische Boxen (Waren, Wagen-Inhalt).
fn cube() -> MeshData {
    let mut vertices = Vec::with_capacity(36);
    push_box(&mut vertices, Vec3::ZERO, Vec3::splat(0.5), Vec3::ONE);
    MeshData {
        vertices,
        min: Vec3::splat(-0.5),
        max: Vec3::splat(0.5),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_meshes_load_and_stand_on_ground() {
        let meshes = load_all();
        assert_eq!(meshes.len(), mesh_count());
        let products = FIXED_COUNT..FIXED_COUNT + PRODUCT_MESH_IDS.len();
        // nach den Waren kommen die Warnschild-Fassungen (wet_sign.rs)
        assert_eq!(mesh_count(), products.end + crate::wet_sign::VARIANTS - 1);
        assert_eq!(wet_sign_mesh(2), mesh_count() - 1);
        // Waren passen in die Regalfaecher (Bodenabstand 0,43 m)
        for i in products {
            assert!(
                half_extent_idx(i).y * 2.0 < 0.36,
                "{} zu hoch",
                PRODUCT_MESH_IDS[i - FIXED_COUNT]
            );
        }
        for (i, m) in meshes.iter().enumerate() {
            assert!(!m.vertices.is_empty(), "Mesh {i} leer");
            assert_eq!(m.vertices.len() % 3, 0);
            if i > 0 {
                assert!(
                    m.min.y.abs() < 1e-3,
                    "Mesh {i} steht nicht auf dem Boden: {}",
                    m.min.y
                );
            }
        }
        // Wagen: das Rohmodell hat keinen Korb, der Drahtkorb kommt dazu
        let raw = data::load::<MeshData>(SOURCES[MeshId::Cart as usize - 1]);
        let cart = &meshes[MeshId::Cart as usize];
        assert!(cart.vertices.len() > raw.vertices.len());
        // Raeder und Griff bleiben, die alte untere Ablage (Mitte, ~0,22 m) ist weg
        assert!(
            cart.vertices.iter().any(|v| v.color[0] < 0.2),
            "keine Raeder"
        );
        assert!(cart.max.y > 0.82, "kein Griff");
        assert!(
            !cart
                .vertices
                .iter()
                .any(|v| v.pos[0].abs() < 0.1 && (0.18..0.26).contains(&v.pos[1])),
            "untere Ablage noch da"
        );
        assert!(
            !cart.vertices.iter().any(is_old_frame) || {
                // Korb und Gestell haben die alte Rahmenfarbe: dann nur so viele wie Staebe
                let rods = cart_basket().len() + cart_chassis().len();
                cart.vertices.iter().filter(|v| is_old_frame(v)).count() == rods * 36
            }
        );
        // Mitarbeiter und Kunden gibt es in mehreren Varianten (Groesse: people::tests)
        assert!(people_of(true).len() >= 4 && people_of(false).len() >= 12);
    }

    #[test]
    fn cart_basket_has_a_wire_floor_under_every_place_and_walls() {
        let rods = cart_basket();
        // Bodendraehte: Oberkante genau auf dem Korbboden
        let floor: Vec<&Rod> = rods
            .iter()
            .filter(|r| ((r.center.y + r.half.y) - CART_BASKET_FLOOR).abs() < 0.002)
            .collect();
        assert!(floor.len() >= 10, "kein Gitterboden");
        // unter jedem Warenplatz (3 quer x 4 laengs) liegt ein Draht in hoechstens 2,5 cm
        for row in 0..4 {
            for col in [-1.0f32, 0.0, 1.0] {
                let p = vec3(col * 0.135, 0.0, -0.4 + row as f32 * 0.175);
                assert!(
                    floor.iter().any(|r| (r.center.x - p.x).abs() <= 0.025
                        && (p.z - r.center.z).abs() <= r.half.z),
                    "kein Boden unter Platz {row}/{col}"
                );
            }
        }
        // Waende reichen bis oben, an beiden Seiten
        let top = rods
            .iter()
            .map(|r| r.center.y + r.half.y)
            .fold(0.0f32, f32::max);
        assert!(
            (top - CART_BASKET_TOP).abs() < 0.01,
            "Wand zu niedrig: {top}"
        );
        for side in [-1.0f32, 1.0] {
            assert!(rods
                .iter()
                .any(|r| (r.center.x - side * CART_BASKET_X).abs() < 1e-4 && r.half.y > 0.1));
        }
    }
}
