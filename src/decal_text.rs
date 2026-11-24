//! Text als kleines Pixelbild (Archivo Black, `crate::signs::FONT`) fuer Aufschriften,
//! die als flache Streifen in Modelle gebacken werden (Warnschilder, Brust-Schriftzug).

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};

/// Schriftzug `s` zentriert in ein Bild der Breite `w` Pixel rastern: Zeile ab `y_top`,
/// Versalhoehe `h` Pixel; ist der Text zu breit, wird er kleiner (1 Pixel Rand je Seite).
/// `set(x, y)` bekommt jedes gefuellte Pixel (kann ausserhalb des Bilds liegen).
pub fn raster(s: &str, w: usize, y_top: f32, h: f32, set: &mut dyn FnMut(i32, i32)) {
    let font = FontRef::try_from_slice(crate::signs::FONT).expect("Archivo Black kaputt");
    let width = |px: f32| {
        let sf = font.as_scaled(PxScale::from(px));
        s.chars().map(|c| sf.h_advance(sf.glyph_id(c))).sum::<f32>()
    };
    // Versalhoehe ~ 0,72 der Schriftgroesse; notfalls kleiner, damit es in die Breite passt
    let mut size = h / 0.72;
    let max_w = w as f32 - 2.0;
    if width(size) > max_w {
        size *= max_w / width(size);
    }
    let sf = font.as_scaled(PxScale::from(size));
    let mut x = (w as f32 - width(size)) * 0.5;
    let baseline = y_top + sf.ascent() * 0.72;
    for c in s.chars() {
        let gid = sf.glyph_id(c);
        let g = gid.with_scale_and_position(PxScale::from(size), point(x, baseline));
        if let Some(og) = font.outline_glyph(g) {
            let bb = og.px_bounds();
            og.draw(|gx, gy, cov| {
                if cov > 0.5 {
                    set(bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32);
                }
            });
        }
        x += sf.h_advance(gid);
    }
}

/// Pixelbild `w` x `h` (zeilenweise, true = Farbe) in waagrechte Streifen zerlegen:
/// (Zeile, erstes Pixel, Pixel hinter dem letzten).
pub fn runs(px: &[bool], w: usize, h: usize) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for row in 0..h {
        let mut x = 0;
        while x < w {
            if !px[row * w + x] {
                x += 1;
                continue;
            }
            let start = x;
            while x < w && px[row * w + x] {
                x += 1;
            }
            out.push((row, start, x));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_lands_in_the_image_and_splits_into_runs() {
        let (w, h) = (40usize, 16usize);
        let mut px = vec![false; w * h];
        raster("Ab", w, 2.0, 10.0, &mut |x, y| {
            if (0..w as i32).contains(&x) && (0..h as i32).contains(&y) {
                px[y as usize * w + x as usize] = true;
            }
        });
        let r = runs(&px, w, h);
        assert!(!r.is_empty());
        let filled: usize = r.iter().map(|&(_, a, b)| b - a).sum();
        assert_eq!(filled, px.iter().filter(|&&p| p).count());
    }
}
