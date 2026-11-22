//! Welche Unterabteilung steht wo? Normal logisch sortiert wie im Discounter,
//! mit steigender Eskalation zunehmend durcheinander.

use crate::catalog::{catalog, FixtureKind};
use rand::seq::SliceRandom;
use rand::Rng;
use rand_pcg::Pcg64;

/// Mischstufe des Sortiments.
/// 0 = logisch, 1 = Unterabteilungen tauschen innerhalb ihrer Einrichtungsart,
/// 2 = Abteilungen landen in fremder Einrichtung, 3 = Chaos (einzelne Waren falsch, Schilder luegen)
pub type MixLevel = u8;

#[derive(Clone, Debug)]
pub struct Layout {
    /// je Gondelseite (in Laufrichtung vom Eingang aus): (Unterabteilung, von, bis) als Anteil 0..1 der Seite
    pub shelf_sides: Vec<Vec<(usize, f32, f32)>>,
    /// je Kuehlwand-Abschnitt eine Unterabteilung
    pub cooler: Vec<usize>,
    /// je Kuehltruhen-Abschnitt eine Unterabteilung
    pub freezer: Vec<usize>,
    /// Obst-Regal an der Wand und Gemuese-Insel
    pub produce_rack: usize,
    pub produce_island: usize,
    /// Wahrscheinlichkeit, dass eine einzelne Ware aus einer fremden Abteilung stammt
    pub chaos: f32,
    /// Schilder zeigen die logische Belegung statt der echten
    pub signs_lie: bool,
    /// Was auf dem Schild steht: je Gondelseite / Kuehlabschnitt die logische Unterabteilung
    pub sign_shelf: Vec<usize>,
    pub sign_cooler: Vec<usize>,
}

fn subs_of(kind: FixtureKind) -> Vec<usize> {
    let c = catalog();
    (0..c.subs.len())
        .filter(|&i| c.subs[i].fixture == kind)
        .collect()
}

/// Verteilt `subs` gleichmaessig auf `n` Abschnitte (aneinandergereiht).
fn spread(subs: &[usize], n: usize) -> Vec<Vec<(usize, f32, f32)>> {
    let mut out = vec![Vec::new(); n];
    if subs.is_empty() || n == 0 {
        return out;
    }
    let per = n as f32 / subs.len() as f32; // Abschnitte pro Unterabteilung
    for (k, &s) in subs.iter().enumerate() {
        let (a, b) = (k as f32 * per, (k + 1) as f32 * per);
        for (i, slot) in out.iter_mut().enumerate() {
            let (lo, hi) = (a.max(i as f32), b.min(i as f32 + 1.0));
            if hi - lo > 1e-4 {
                slot.push((s, lo - i as f32, hi - i as f32));
            }
        }
    }
    out
}

fn first_of(v: &[(usize, f32, f32)]) -> usize {
    v.first().map(|x| x.0).unwrap_or(0)
}

pub fn build(
    sides: usize,
    coolers: usize,
    freezers: usize,
    mix: MixLevel,
    rng: &mut Pcg64,
) -> Layout {
    let mut shelf = subs_of(FixtureKind::Shelf);
    // Getraenke stehen zusaetzlich im Getraenkeraum; im Gang reicht ein Anteil
    let mut cooler = subs_of(FixtureKind::Cooler);
    let mut freezer = subs_of(FixtureKind::Freezer);
    let produce = subs_of(FixtureKind::Produce);
    let canon_shelf = spread(&shelf, sides);
    let canon_cooler: Vec<usize> = spread(&cooler, coolers)
        .iter()
        .map(|v| first_of(v))
        .collect();

    if mix >= 1 {
        shelf.shuffle(rng);
        cooler.shuffle(rng);
        freezer.shuffle(rng);
    }
    if mix >= 2 {
        // Abteilungen wandern in fremde Einrichtungen (Brot in die Kuehlung, Eis ins Regal ...)
        let mut pool: Vec<usize> = shelf
            .iter()
            .chain(cooler.iter())
            .chain(freezer.iter())
            .copied()
            .collect();
        pool.shuffle(rng);
        let (ns, nc) = (shelf.len(), cooler.len());
        shelf = pool[..ns].to_vec();
        cooler = pool[ns..ns + nc].to_vec();
        freezer = pool[ns + nc..].to_vec();
    }
    let (produce_rack, produce_island) = if mix >= 2 && rng.gen_bool(0.5) {
        (produce[1], produce[0])
    } else {
        (produce[0], produce[1])
    };

    let shelf_sides = spread(&shelf, sides);
    let cooler_v: Vec<usize> = spread(&cooler, coolers)
        .iter()
        .map(|v| first_of(v))
        .collect();
    let freezer_v: Vec<usize> = spread(&freezer, freezers)
        .iter()
        .map(|v| first_of(v))
        .collect();
    let signs_lie = mix >= 3;
    Layout {
        sign_shelf: if signs_lie {
            canon_shelf.iter().map(|v| first_of(v)).collect()
        } else {
            shelf_sides.iter().map(|v| first_of(v)).collect()
        },
        sign_cooler: if signs_lie {
            canon_cooler
        } else {
            cooler_v.clone()
        },
        shelf_sides,
        cooler: cooler_v,
        freezer: freezer_v,
        produce_rack,
        produce_island,
        chaos: if mix >= 3 { 0.35 } else { 0.0 },
        signs_lie,
    }
}

impl Layout {
    /// Alle Unterabteilungen, die irgendwo im Laden stehen.
    pub fn placed_subs(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.shelf_sides.iter().flatten().map(|x| x.0).collect();
        v.extend(&self.cooler);
        v.extend(&self.freezer);
        v.push(self.produce_rack);
        v.push(self.produce_island);
        v.sort();
        v.dedup();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn every_department_gets_a_place_at_every_mix_level() {
        let all: Vec<usize> = (0..catalog().subs.len()).collect();
        for mix in 0..=3 {
            for seed in 0..30 {
                let mut rng = Pcg64::seed_from_u64(seed);
                let l = build(8 + (seed % 5) as usize, 8, 6, mix, &mut rng);
                assert_eq!(
                    l.placed_subs(),
                    all,
                    "mix {mix}, seed {seed}: Abteilung fehlt"
                );
            }
        }
    }

    #[test]
    fn logical_order_keeps_cooling_together() {
        let mut rng = Pcg64::seed_from_u64(1);
        let l = build(10, 10, 6, 0, &mut rng);
        let c = catalog();
        assert!(l
            .cooler
            .iter()
            .all(|&s| c.subs[s].fixture == FixtureKind::Cooler));
        assert!(l
            .freezer
            .iter()
            .all(|&s| c.subs[s].fixture == FixtureKind::Freezer));
        // Milch steht direkt neben Kaese (Reihenfolge wie im Katalog)
        let milch = l
            .cooler
            .iter()
            .position(|&s| c.subs[s].id == "milch")
            .unwrap();
        let kaese = l
            .cooler
            .iter()
            .position(|&s| c.subs[s].id == "kaese")
            .unwrap();
        assert!(kaese >= milch && kaese - milch <= 2);
        assert!(!l.signs_lie);
    }

    #[test]
    fn spread_covers_every_side_completely() {
        let v = spread(&[1, 2, 3], 7);
        for side in &v {
            let covered: f32 = side.iter().map(|x| x.2 - x.1).sum();
            assert!((covered - 1.0).abs() < 1e-4);
        }
    }
}
