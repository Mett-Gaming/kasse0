//! Kleines Tutorial im (noch) normalen Markt: zeigt alle Tasten und hakt sie ab.

/// (Taste, Erklaerung)
pub const STEPS: &[(&str, &str)] = &[
    ("Maus", "Umsehen"),
    ("W A S D", "Gehen"),
    ("TAB halten", "Einkaufszettel"),
    ("Q", "Wagen abstellen/schieben"),
    ("E / Klick", "Ware greifen (Wasser!)"),
    ("E auf Wagen", "In den Wagen legen"),
    ("F am Wagen", "Ware herausnehmen"),
    ("Shift / Esc", "Rennen (auffällig)/Pause"),
];

#[derive(Default)]
pub struct Tutorial {
    pub step: usize,
    pub looked: f32,
    pub walked: f32,
    pub listed: bool,
    pub toggles: u32,
    pub took_out: bool,
    pub timer: f32,
}

impl Tutorial {
    /// Prueft den aktuellen Schritt. Gibt `true` zurueck, wenn das Tutorial fertig ist.
    pub fn advance(&mut self, dt: f32, holding: bool, cart_items: usize) -> bool {
        let done = match self.step {
            0 => self.looked > 2.0,
            1 => self.walked > 3.0,
            2 => self.listed,
            3 => self.toggles >= 2,
            4 => holding,
            5 => cart_items > 0,
            6 => self.took_out,
            _ => {
                self.timer += dt;
                self.timer > 6.0
            }
        };
        if done {
            self.step += 1;
            self.timer = 0.0;
        }
        self.step >= STEPS.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_advance_in_order() {
        let mut t = Tutorial::default();
        assert!(!t.advance(0.1, false, 0));
        assert_eq!(t.step, 0);
        t.looked = 3.0;
        t.advance(0.1, false, 0);
        assert_eq!(t.step, 1);
        t.walked = 5.0;
        t.listed = true;
        t.toggles = 2;
        t.advance(0.1, false, 0);
        t.advance(0.1, false, 0);
        t.advance(0.1, false, 0);
        assert_eq!(t.step, 4);
        t.advance(0.1, true, 0);
        t.advance(0.1, false, 1);
        t.took_out = true;
        t.advance(0.1, false, 1);
        assert_eq!(t.step, 7);
        assert!(!t.advance(1.0, false, 1));
        assert!(t.advance(6.0, false, 1));
    }
}
