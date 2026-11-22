//! Modelle im .k0m-Format (erzeugt von tools/prepare_assets.py) - fest ins Spiel eingebaut.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MeshVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
}

pub struct MeshData {
    pub vertices: Vec<MeshVertex>,
    #[allow(dead_code)]
    pub min: Vec3,
    #[allow(dead_code)]
    pub max: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum MeshId {
    Cube = 0,
    Cart,
    Register,
    WetFloorSign,
    Can,
    Cashier,
    Chair,
    Shutter,
    Camera,
    Terminal,
    Cigarettes,
}

/// Anzahl fester Modelle (danach folgen die Waren aus dem Katalog)
pub const FIXED_COUNT: usize = MeshId::Cigarettes as usize + 1;

mod gen {
    include!(concat!(env!("OUT_DIR"), "/products_gen.rs"));
}
pub use gen::{CATALOG_JSON, PRODUCT_MESH_IDS};

pub use crate::people::people_of;

const SOURCES: [&[u8]; FIXED_COUNT - 1] = [
    include_bytes!("../assets/cart.k0m"),
    include_bytes!("../assets/register.k0m"),
    include_bytes!("../assets/wet_floor_sign.k0m"),
    include_bytes!("../assets/can.k0m"),
    include_bytes!("../assets/cashier.k0m"),
    include_bytes!("../assets/chair.k0m"),
    include_bytes!("../assets/shutter.k0m"),
    include_bytes!("../assets/camera.k0m"),
    include_bytes!("../assets/terminal.k0m"),
    include_bytes!("../assets/cigarettes.k0m"),
];

/// Alle Modelle: feste (Index = MeshId) gefolgt von den Waren (Index = FIXED_COUNT + Katalog-Mesh).
pub fn load_all() -> Vec<MeshData> {
    let mut v = vec![cube()];
    v.extend(SOURCES.iter().map(|b| parse(b)));
    v.extend(gen::PRODUCT_MESH_BYTES.iter().map(|b| parse(b)));
    v
}

pub fn mesh_count() -> usize {
    FIXED_COUNT + gen::PRODUCT_MESH_BYTES.len()
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

fn read_f32(b: &[u8], off: usize) -> f32 {
    f32::from_le_bytes(b[off..off + 4].try_into().unwrap())
}

pub fn parse(bytes: &[u8]) -> MeshData {
    assert_eq!(&bytes[0..4], b"K0M1", "kein K0M1-Mesh");
    let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    let min = Vec3::new(read_f32(bytes, 8), read_f32(bytes, 12), read_f32(bytes, 16));
    let max = Vec3::new(
        read_f32(bytes, 20),
        read_f32(bytes, 24),
        read_f32(bytes, 28),
    );
    let data = &bytes[32..32 + count * 36];
    let mut vertices = Vec::with_capacity(count);
    for v in data.chunks_exact(36) {
        let f = |i: usize| read_f32(v, i * 4);
        vertices.push(MeshVertex {
            pos: [f(0), f(1), f(2)],
            normal: [f(3), f(4), f(5)],
            color: [f(6), f(7), f(8)],
        });
    }
    MeshData { vertices, min, max }
}

/// Einheitswuerfel [-0.5, 0.5]^3 fuer dynamische Boxen (Waren, Wagen-Inhalt).
fn cube() -> MeshData {
    let mut vertices = Vec::with_capacity(36);
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
            let p = (n + u * a + v * b) * 0.5;
            vertices.push(MeshVertex {
                pos: p.to_array(),
                normal: n.to_array(),
                color: [1.0; 3],
            });
        }
    }
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
        assert_eq!(mesh_count(), products.end);
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
        // Mitarbeiter und Kunden gibt es in mehreren Varianten (Groesse: people::tests)
        assert!(people_of(true).len() >= 4 && people_of(false).len() >= 12);
    }
}
