//! Schild-Atlas: alle Schriftzuege im Markt (Banderolen, Abteilungsschilder, Aktion, Kassen)
//! werden beim Start mit Archivo Black (OFL) auf der CPU in eine Graustufen-Textur gerendert.
//! Jede Zeile des Atlas ist ein Schriftzug; der Welt-Shader liest sie ueber die Zeilennummer.

use crate::catalog::catalog;
use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Hoehe einer Atlas-Zeile in Pixeln
pub const ROW_H: u32 = 32;
/// Breite des Atlas in Pixeln
pub const ATLAS_W: u32 = 512;

const FONT: &[u8] = include_bytes!("../assets/fonts/ArchivoBlack-Regular.ttf");

/// Ladenname (eigene Marke, keine echte Kette)
pub const BRAND: &str = "KAUF NULL";

/// Banderolen-Sprueche: normal und die verdorbene Fassung im gekippten Markt.
pub const SLOGANS: [(&str, &str); 6] = [
    ("Frisch & fröhlich", "Frisch & ewig"),
    ("Bunt & reichlich", "Bunt & verdorben"),
    ("Komm & kauf", "Bleib & kauf"),
    ("Satt & zufrieden", "Satt & leer"),
    ("Jeden Tag & billig", "Jeden Tag & immer"),
    ("Gut & nah", "Nah & näher"),
];

/// Weitere feste Schriftzuege
const FIXED: &[&str] = &[
    BRAND,
    "KAUF NULL NULL NULL",
    "AKTION",
    "ABGELAUFEN",
    "Kasse",
    "1 Kasse",
    "2 Kasse",
    "3 Kasse",
    "4 Kasse",
    "0 Kasse",
    "Kühlung",
    "Tiefkühl",
    "Getränke",
    "Getränke & Pfand",
    "Eingang",
    "Kein Ausgang",
    "Danke für Ihren Einkauf",
    "Wir sehen Sie",
    "Bitte nicht stören",
    "Nächster Kunde bitte",
    "Wer kauft, bleibt",
    "Heute frisch",
    "Seit immer frisch",
];

pub struct SignAtlas {
    /// R8-Pixel, ATLAS_W x height
    pub pixels: Vec<u8>,
    pub height: u32,
    ids: HashMap<String, u32>,
    /// Breite des Schriftzugs relativ zur Zeilenhoehe
    aspects: Vec<f32>,
}

impl SignAtlas {
    /// Zeile eines Schriftzugs. Unbekannte Texte landen auf Zeile 0 (Ladenname).
    pub fn id(&self, text: &str) -> u32 {
        debug_assert!(
            self.ids.contains_key(text),
            "Schriftzug fehlt im Atlas: {text}"
        );
        self.ids.get(text).copied().unwrap_or(0)
    }
    /// Breite/Hoehe des gezeichneten Schriftzugs (ohne Rand).
    pub fn aspect(&self, id: u32) -> f32 {
        self.aspects[id as usize]
    }
}

fn all_texts() -> Vec<String> {
    let mut v: Vec<String> = FIXED.iter().map(|s| s.to_string()).collect();
    for (a, b) in SLOGANS {
        v.push(a.into());
        v.push(b.into());
    }
    let cat = catalog();
    for s in &cat.subs {
        v.push(s.name.clone());
        v.push(s.dept.clone());
    }
    let mut seen = std::collections::HashSet::new();
    v.retain(|s| seen.insert(s.clone()));
    v
}

fn build() -> SignAtlas {
    let font = FontRef::try_from_slice(FONT).expect("Archivo Black kaputt");
    let texts = all_texts();
    let height = ROW_H * texts.len() as u32;
    let mut pixels = vec![0u8; (ATLAS_W * height) as usize];
    let mut ids = HashMap::new();
    let mut aspects = Vec::new();
    for (row, text) in texts.iter().enumerate() {
        // Schriftgroesse: Versalhoehe fuellt ~70 % der Zeile, notfalls kleiner, damit es passt
        let mut px = ROW_H as f32 * 0.95;
        let measure = |px: f32| {
            let sf = font.as_scaled(PxScale::from(px));
            let mut w = 0.0;
            let mut prev = None;
            for c in text.chars() {
                let g = sf.glyph_id(c);
                if let Some(p) = prev {
                    w += sf.kern(p, g);
                }
                w += sf.h_advance(g);
                prev = Some(g);
            }
            w
        };
        let max_w = (ATLAS_W - 8) as f32;
        let w = measure(px);
        if w > max_w {
            px *= max_w / w;
        }
        let w = measure(px);
        let sf = font.as_scaled(PxScale::from(px));
        let x0 = (ATLAS_W as f32 - w) * 0.5;
        // Grundlinie so, dass Versalien vertikal mittig sitzen
        let cap = sf.ascent() * 0.72;
        let baseline = row as f32 * ROW_H as f32 + (ROW_H as f32 + cap) * 0.5;
        let mut x = x0;
        let mut prev = None;
        for c in text.chars() {
            let gid = sf.glyph_id(c);
            if let Some(p) = prev {
                x += sf.kern(p, gid);
            }
            let glyph = gid.with_scale_and_position(PxScale::from(px), point(x, baseline));
            if let Some(og) = font.outline_glyph(glyph) {
                let b = og.px_bounds();
                og.draw(|gx, gy, cov| {
                    let px_x = b.min.x as i32 + gx as i32;
                    let px_y = b.min.y as i32 + gy as i32;
                    let row_top = (row as u32 * ROW_H) as i32;
                    if px_x >= 0
                        && px_x < ATLAS_W as i32
                        && px_y >= row_top
                        && px_y < row_top + ROW_H as i32
                    {
                        let i = (px_y as u32 * ATLAS_W + px_x as u32) as usize;
                        pixels[i] = pixels[i].max((cov.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                });
            }
            x += sf.h_advance(gid);
            prev = Some(gid);
        }
        ids.insert(text.clone(), row as u32);
        aspects.push(w / ROW_H as f32);
    }
    SignAtlas {
        pixels,
        height,
        ids,
        aspects,
    }
}

pub fn atlas() -> &'static SignAtlas {
    static A: OnceLock<SignAtlas> = OnceLock::new();
    A.get_or_init(build)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_contains_all_texts_with_ink() {
        let a = atlas();
        assert_eq!(a.pixels.len(), (ATLAS_W * a.height) as usize);
        for t in all_texts() {
            let id = a.id(&t);
            let row =
                &a.pixels[(id * ROW_H * ATLAS_W) as usize..((id + 1) * ROW_H * ATLAS_W) as usize];
            let ink = row.iter().filter(|&&p| p > 128).count();
            assert!(ink > 20, "kein Text gezeichnet: {t}");
            assert!(
                a.aspect(id) > 0.5 && a.aspect(id) <= ATLAS_W as f32 / ROW_H as f32,
                "{t}"
            );
        }
        assert!(a.aspect(a.id("Frisch & ewig")) > a.aspect(a.id("AKTION")));
    }
}
