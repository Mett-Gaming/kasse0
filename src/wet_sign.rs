//! Aufschrift der Warnschilder "Rutschgefahr": Das Schild-Modell (Asset-Pack) ist nur gelb.
//! Beim Laden wird auf beide schraegen Seiten ein Warndreieck mit "!" und ein Text gebacken:
//! ein kleines Pixelbild (Text mit Archivo Black gerastert), dessen Zeilen als flache Streifen
//! genau auf der Schildflaeche liegen. Drei Fassungen, je gruseliger der Markt:
//! 0 "VORSICHT / Frisch / gewischt", 1 "VORSICHT / Nicht / hinsehen", 2 "LAUF / Frisch /
//! geblutet" (rot).

use crate::decal_text::{raster, runs};
use glam::{vec3, Vec3};
use mettgine::gpu::MeshVertex;

/// Anzahl der Fassungen (normal, unheimlich, grauenhaft)
pub const VARIANTS: usize = 3;

/// Texte je Fassung: grosse Zeile, zwei kleine Zeilen
pub const TEXTS: [[&str; 3]; VARIANTS] = [
    ["VORSICHT", "Frisch", "gewischt"],
    ["VORSICHT", "Nicht", "hinsehen"],
    ["LAUF", "Frisch", "geblutet"],
];

/// Pixelbild: Breite, Hoehe (Pixel) und Pixelgroesse auf dem Schild (Meter)
const W: usize = 52;
const H: usize = 84;
const PIX: f32 = 0.005;
/// Oberkante des Bilds auf dem Schild (Meter ueber dem Boden, unter dem Griff)
const TOP: f32 = 0.5;

/// Schildflaeche (gemessen am Modell): Vorderseite bei z(y) = Z0 - SLOPE * (y - Y0)
const Z0: f32 = 0.177;
const Y0: f32 = 0.004;
const SLOPE: f32 = 0.2597;
/// so weit liegt die Aufschrift vor der Flaeche (Meter)
const LIFT: f32 = 0.003;

/// Pixelbild der Fassung `variant`: true = Farbe (Dreieck, "!", Text).
pub fn bitmap(variant: usize) -> Vec<bool> {
    let mut px = vec![false; W * H];
    let mut set = |x: i32, y: i32| {
        if (0..W as i32).contains(&x) && (0..H as i32).contains(&y) {
            px[y as usize * W + x as usize] = true;
        }
    };
    // Warndreieck: Spitze oben, Rand 3 Pixel, "!" in der Mitte
    let (top, tri_h, cx) = (1i32, 27i32, W as i32 / 2);
    for r in 0..tri_h {
        let hw = (r as f32 / tri_h as f32 * 15.0).round() as i32;
        for x in cx - hw..=cx + hw {
            let edge = (x - cx).abs() >= hw - 3 || r >= tri_h - 3;
            if edge {
                set(x, top + r);
            }
        }
    }
    for y in top + 9..top + 19 {
        for x in cx - 1..=cx + 1 {
            set(x, y);
        }
    }
    for y in top + 20..top + 23 {
        for x in cx - 1..=cx + 1 {
            set(x, y);
        }
    }
    // Text: grosse Zeile, zwei kleine Zeilen, je auf die Breite eingepasst
    let [big, a, b] = TEXTS[variant.min(VARIANTS - 1)];
    let mut text = |s: &str, y_top: f32, h: f32| {
        raster(s, W, y_top, h, &mut |x, y| set(x, y));
    };
    text(big, 33.0, 15.0);
    text(a, 54.0, 12.0);
    text(b, 68.0, 12.0);
    px
}

/// Dreiecke der Aufschrift fuer beide Schildseiten (Modellkoordinaten des Schilds).
pub fn decal(variant: usize) -> Vec<MeshVertex> {
    let ink = if variant >= 2 {
        vec3(0.55, 0.02, 0.02)
    } else {
        vec3(0.04, 0.04, 0.04)
    };
    let px = bitmap(variant);
    let n_len = (SLOPE * SLOPE + 1.0).sqrt();
    let mut out = Vec::new();
    for side in [1.0f32, -1.0] {
        // Rueckseite gespiegelt, damit sie von hinten richtig herum zu lesen ist
        let normal = vec3(0.0, SLOPE / n_len, side / n_len);
        let at = |u: f32, y: f32| {
            let z = Z0 - SLOPE * (y - Y0);
            vec3(u * side, y, z * side) + normal * LIFT
        };
        // Streifen von Pixel `start` bis `x` in Zeile `row`
        for (row, start, x) in runs(&px, W, H) {
            let (u0, u1) = (
                (start as f32 - W as f32 * 0.5) * PIX,
                (x as f32 - W as f32 * 0.5) * PIX,
            );
            let (y0, y1) = (TOP - (row + 1) as f32 * PIX, TOP - row as f32 * PIX);
            let quad = [
                at(u0, y0),
                at(u1, y0),
                at(u1, y1),
                at(u0, y0),
                at(u1, y1),
                at(u0, y1),
            ];
            out.extend(quad.iter().map(|p: &Vec3| MeshVertex {
                pos: p.to_array(),
                normal: normal.to_array(),
                color: ink.to_array(),
            }));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_triangle_and_text_on_both_sides() {
        for v in 0..VARIANTS {
            let px = bitmap(v);
            let rows = |a: usize, b: usize| (a..b).any(|r| (0..W).any(|x| px[r * W + x]));
            assert!(rows(0, 28), "{v}: kein Warndreieck");
            assert!(
                rows(33, 50) && rows(54, 67) && rows(68, H),
                "{v}: Text fehlt"
            );
            let d = decal(v);
            assert!(!d.is_empty() && d.len() % 3 == 0);
            // beide Seiten, alles auf der Schildflaeche (innerhalb des Modells)
            assert!(d.iter().any(|p| p.pos[2] > 0.0) && d.iter().any(|p| p.pos[2] < 0.0));
            for p in &d {
                assert!(p.pos[0].abs() <= 0.14 && p.pos[1] >= 0.07 && p.pos[1] <= 0.51);
                assert!(p.pos[2].abs() <= 0.185, "steht ab: {:?}", p.pos);
            }
        }
        // die grauenhafte Fassung ist rot
        assert!(decal(2)[0].color[0] > 0.4 && decal(0)[0].color[0] < 0.1);
    }
}
