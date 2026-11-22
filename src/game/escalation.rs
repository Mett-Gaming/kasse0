//! Eskalationsstufen nach Warenwert im Einkaufswagen.
//! Nicht die Anzahl der Artikel zaehlt, sondern was der Einkauf kostet.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// < 10 EUR: falsche Etiketten, unsinnige Preise
    Calm,
    /// 10-25 EUR: Durchsagen, mehr Kunden, Gaenge werden laenger
    Uneasy,
    /// 25-50 EUR: Gaenge werden dunkel, Kunden schneller, Mitarbeiter patrouillieren
    Wrong,
    /// > 50 EUR: Grundriss verschiebt sich, die Kasse wandert
    Collapse,
}

impl Tier {
    pub fn level(self) -> u8 {
        self as u8
    }
    /// Wie viele uncanny Kunden im Markt sind
    pub fn customers(self) -> usize {
        [4, 6, 8, 10][self as usize]
    }
    /// Live-Verbiegen der Gaenge: (seitlich, Hoehenwelle) in Metern
    pub fn sway(self) -> (f32, f32) {
        // nur auf hohen Stufen spuerbar: "falsch" kaum, im Kollaps richtig
        [(0.0, 0.0), (0.0, 0.0), (0.04, 0.08), (0.25, 0.55)][self as usize]
    }
    /// Loecher im Boden (Hindernisse) und in der Decke
    pub fn holes(self) -> u32 {
        [0, 0, 2, 4][self as usize]
    }
    pub fn ceiling_holes(self) -> u32 {
        [0, 0, 3, 6][self as usize]
    }
    /// Anteil verdorbener Ware
    pub fn rot(self) -> f32 {
        [0.0, 0.03, 0.08, 0.15][self as usize]
    }
    /// riesige Waren als Hindernisse
    pub fn giants(self) -> u32 {
        [0, 0, 1, 3][self as usize]
    }
    /// Wie viele davon Verfolger sind (der Rest kauft nur ein)
    pub fn stalkers(self) -> usize {
        [2, 3, 5, 7][self as usize]
    }
    /// Wie viele gesichtslose Mitarbeiter unterwegs sind
    pub fn employees(self) -> usize {
        [1, 1, 2, 3][self as usize]
    }
    /// Grundtempo der Kunden (m/s), wenn man nicht hinsieht
    pub fn customer_speed(self) -> f32 {
        [0.45, 0.6, 0.8, 1.05][self as usize]
    }
    /// Ganglaenge relativ zum normalen Markt
    pub fn stretch(self) -> f32 {
        if self >= Tier::Uneasy {
            2.0
        } else {
            1.6
        }
    }
    /// Zusaetzlich tote Roehren
    pub fn darkness(self) -> f32 {
        match self {
            Tier::Calm | Tier::Uneasy => 0.0,
            Tier::Wrong => 0.15,
            Tier::Collapse => 0.25,
        }
    }
    /// Wie krumm die Gaenge sind (versetzte Regalsegmente, Durchgaenge)
    pub fn warp(self) -> f32 {
        [0.0, 0.5, 1.0, 1.0][self as usize]
    }
    /// Sortiment-Mischstufe: Abteilungen verschieben sich (siehe world::layout)
    pub fn mix(self) -> u8 {
        self as u8
    }
    /// Neue Querwaende in Gaengen
    pub fn walls(self) -> u32 {
        [0, 0, 2, 3][self as usize]
    }
    /// Sekunden zwischen Umbauten (nur, wenn man gerade nicht hinsieht); None = kein Umbau
    pub fn rebuild_interval(self) -> Option<f32> {
        [None, Some(45.0), Some(35.0), Some(20.0)][self as usize]
    }
    /// Durchsagen-Intervall (min, max) in Sekunden
    pub fn announce_interval(self) -> (f32, f32) {
        match self {
            Tier::Calm => (70.0, 100.0),
            Tier::Uneasy | Tier::Wrong => (35.0, 60.0),
            Tier::Collapse => (20.0, 35.0),
        }
    }
    pub fn name(self) -> &'static str {
        ["ruhig", "unruhig", "falsch", "Kollaps"][self as usize]
    }
}

/// Warenwert in Cent, damit es keine Rundungsfehler gibt.
pub fn tier_for_value(cents: u32) -> Tier {
    match cents {
        0..=999 => Tier::Calm,
        1000..=2499 => Tier::Uneasy,
        2500..=4999 => Tier::Wrong,
        _ => Tier::Collapse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_value_table() {
        assert_eq!(tier_for_value(0), Tier::Calm);
        assert_eq!(tier_for_value(999), Tier::Calm);
        assert_eq!(tier_for_value(1000), Tier::Uneasy);
        assert_eq!(tier_for_value(2499), Tier::Uneasy);
        assert_eq!(tier_for_value(2500), Tier::Wrong);
        assert_eq!(tier_for_value(4999), Tier::Wrong);
        assert_eq!(tier_for_value(5000), Tier::Collapse);
        assert_eq!(tier_for_value(u32::MAX), Tier::Collapse);
    }

    #[test]
    fn tiers_never_decrease_with_value() {
        let mut last = Tier::Calm;
        for cents in (0..20_000).step_by(37) {
            let t = tier_for_value(cents);
            assert!(t >= last);
            last = t;
        }
    }
}
