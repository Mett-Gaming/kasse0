//! Plakat-Sortiment: 12 Plakate in 4 Kategorien (Fandubs von Synchro: STEIN-HART und OVERCOCK,
//! Gluecksspiel und Tabak, Aufklaerung ueber Leons Cannabis-Sucht), je 3 Entwuerfe.
//! Bei jedem Spielstart neu gemischt (Seed des Runs) und auf die festen Plakatplaetze an der
//! West- und Suedwand verteilt; beim Umbau des Markts bleibt die Verteilung gleich.
//!
//! Alle Plakate werden mit der Stufe des Markts gruseliger (`level_of`): Farben verblassen,
//! Texte kippen (normal -> unheimlich -> grauenhaft), dazu Flecken und eine abgerissene
//! Ecke, ab "falsch" ein Aufkleber "Sie waren schon hier", im Kollaps Blutlaeufe.

use super::canvas::*;
use super::*;

mod aware;
mod synchro;
mod vice;

/// Ein Plakat-Entwurf: zeichnet die ganze Tafel (inklusive `board`).
pub type Design = fn(&mut Canvas);

/// Die vier Kategorien mit je drei Entwuerfen (Reihenfolge = Kategorie, Entwurf).
pub const DESIGNS: [(&str, [Design; 3]); 4] = [
    (
        "Synchro STEIN-HART",
        [
            synchro::stein_microwave,
            synchro::stein_labcoat,
            synchro::stein_clock,
        ],
    ),
    (
        "Synchro OVERCOCK",
        [
            synchro::cock_king,
            synchro::cock_guild,
            synchro::cock_preview,
        ],
    ),
    (
        "Glücksspiel und Tabak",
        [vice::lotto, vice::scratch, vice::tobacco],
    ),
    (
        "Aufklärung Leon",
        [
            aware::leon_portrait,
            aware::leon_leaf,
            aware::leon_before_after,
        ],
    ),
];

/// Alle Schriftzuege des Sortiments (fuer den Schrift-Atlas, siehe signs.rs).
pub fn texts() -> Vec<&'static str> {
    let mut v: Vec<&str> = Vec::new();
    v.extend_from_slice(synchro::TEXTS);
    v.extend_from_slice(vice::TEXTS);
    v.extend_from_slice(aware::TEXTS);
    v.push(POSTER_SAME);
    v
}

/// Gruselstufe der Plakate: 0 normaler Markt, 1 ruhig, 2 unruhig, 3 falsch, 4 Kollaps.
/// Aus den Stufen-Parametern (Verfall, Mischstufe, Seitentrakt) des Markts.
pub fn level_of(p: &WorldParams) -> u8 {
    if !p.tipped {
        return 0;
    }
    1 + p.decay.max(p.mix).max(p.wing.saturating_sub(1)).min(3)
}

/// Alle 12 Entwuerfe in gemischter Reihenfolge fuer einen Markt (fester Zufall je Seed).
pub fn shuffled(seed: u64) -> Vec<Design> {
    let mut all: Vec<Design> = DESIGNS
        .iter()
        .flat_map(|(_, d)| d.iter().copied())
        .collect();
    let mut rng = Pcg64::seed_from_u64(seed ^ 0x5047_5345_5254);
    all.shuffle(&mut rng);
    all
}

impl Gen<'_> {
    /// Sortiment auf die Plakatplaetze haengen: `spots` = (Mitte auf der Wand, aus der Wand
    /// heraus). Sind es mehr als 12 Plaetze, faengt die Mischung von vorn an.
    pub(super) fn place_sortiment(&mut self, spots: &[(Vec3, Vec3)]) {
        let level = level_of(self.params);
        let order = shuffled(self.params.seed);
        for (k, (wall, front)) in spots.iter().enumerate() {
            let c = *wall + *front * POSTER_OUT;
            let mut cv = Canvas::new(&mut self.b, c, *front, level);
            order[k % order.len()](&mut cv);
            creep(&mut cv, k);
        }
    }
}

/// Gruselspuren ueber dem Plakat, je Stufe mehr. `k` macht sie je Plakat verschieden.
fn creep(cv: &mut Canvas, k: usize) {
    let r = |s: f32| {
        ((k as f32 * 12.9898 + s * 78.233).sin() * 43_758.547)
            .fract()
            .abs()
    };
    let (hw, hh) = (cv.hw, cv.hh);
    if cv.level >= 2 {
        // braune Flecken und eine abgerissene Ecke
        let stain = vec3(0.22, 0.15, 0.08);
        for s in 0..3 {
            let (u, v) = (
                (r(s as f32) - 0.5) * hw * 1.6,
                (r(s as f32 + 7.0) - 0.5) * hh * 1.6,
            );
            cv.rect(u, v, 0.04 + 0.04 * r(s as f32 + 3.0), 0.03, 3.0, stain);
        }
        let corner = if r(11.0) > 0.5 { 1.0 } else { -1.0 };
        cv.rect(
            corner * (hw - 0.07),
            hh - 0.07,
            0.07,
            0.07,
            3.0,
            vec3(0.12, 0.12, 0.11),
        );
    }
    if cv.level >= 3 {
        // Aufkleber: Wiederholung, ueberall derselbe Satz
        let red = vec3(0.55, 0.04, 0.04);
        let v = -hh * 0.25 + (r(5.0) - 0.5) * 0.2;
        cv.rect(0.05, v, 0.4, 0.05, 4.0, red);
        let p = cv.at(0.05, v, 5.0);
        cv.b.label(p, cv.yaw, 0.37, 0.035, POSTER_SAME, red, WHITE, 0.0, 0.0);
    }
    if cv.level >= 4 {
        // Blut laeuft von oben herab
        let blood = vec3(0.35, 0.02, 0.02);
        for s in 0..4 {
            let u = (r(s as f32 + 20.0) - 0.5) * hw * 1.8;
            let len = 0.1 + 0.35 * r(s as f32 + 30.0);
            cv.rect(u, hh - len, 0.012, len, 4.0, blood);
        }
    }
}
