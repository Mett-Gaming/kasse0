//! Barzahlen: Manchmal "geht die Kartenzahlung heute nicht". Dann muss man aus der
//! Geldboerse schnell und passend bezahlen. Scheine und Muenzen waehlt man mit A/D oder dem
//! Mausrad, E (Linksklick) legt hin, F nimmt das zuletzt Hingelegte zurueck. Sobald genau der
//! Betrag auf dem Tresen liegt, ist bezahlt. Zu viel oder zu langsam macht sie misstrauisch.

use super::*;

/// Scheine und Muenzen in Cent (gross nach klein)
pub const DENOMS: [i64; 12] = [5000, 2000, 1000, 500, 200, 100, 50, 20, 10, 5, 2, 1];
/// So oft muss man bar zahlen
pub const CASH_CHANCE: f64 = 0.4;
/// So lange hat man, um passend hinzulegen
pub const CASH_TIME: f32 = 25.0;
/// Zu langsam ist nur ein weicher Fehler (Misstrauen); zu viel kostet sofort ein Leben
pub const SUS_CASH_SLOW: f32 = 0.25;

/// "50 €", "2 €", "20 ct" ...
pub fn denom_label(i: usize) -> String {
    let c = DENOMS[i];
    if c >= 100 {
        format!("{} €", c / 100)
    } else {
        format!("{c} ct")
    }
}

/// Geldboerse: genug, um `total` passend zu zahlen, dazu ein paar Scheine und Muenzen extra.
pub fn fill_wallet(total: i64, rng: &mut Pcg64) -> [u32; DENOMS.len()] {
    let mut w = [0u32; DENOMS.len()];
    let mut rest = total.max(0);
    for (i, &d) in DENOMS.iter().enumerate() {
        let n = rest / d;
        w[i] += n as u32;
        rest -= n * d;
    }
    for _ in 0..rng.gen_range(3..=6) {
        w[rng.gen_range(0..DENOMS.len())] += 1;
    }
    w
}

impl Till {
    /// Was schon auf dem Tresen liegt (Cent).
    pub fn given_sum(&self) -> i64 {
        self.given.iter().map(|&i| DENOMS[i]).sum()
    }

    /// Alles Hingelegte zurueck in die Geldboerse.
    fn take_back_all(&mut self) {
        for i in self.given.drain(..) {
            self.wallet[i] += 1;
        }
    }
}

impl Game {
    pub(in crate::game) fn start_cash(&mut self) {
        let total = self.scanned_value();
        let Some(t) = &mut self.till else { return };
        t.cash = true;
        t.wallet = fill_wallet(total, &mut self.rng);
        t.given.clear();
        t.sel = 0;
        t.pay = Pay::Cash(CASH_TIME);
        self.player.vel = Vec3::ZERO;
        self.say(
            format!(
                "Kassiererin: \"Kartenzahlung geht heute nicht. {}, passend bitte.\"",
                crate::ui::euro(total)
            ),
            4.0,
        );
        self.play(Sfx::Error, 0.4);
    }

    pub(in crate::game) fn update_cash(&mut self, left: f32, dt: f32, input: &Input) {
        let total = self.scanned_value();
        let Some(t) = &mut self.till else { return };
        // Schein/Muenze waehlen: A/D (je Druck ein Schritt) oder Mausrad
        let mut step = -input.wheel;
        if input.right && !t.lr_prev.1 {
            step += 1;
        }
        if input.left && !t.lr_prev.0 {
            step -= 1;
        }
        t.lr_prev = (input.left, input.right);
        if step != 0 {
            t.sel = (t.sel as i32 + step).rem_euclid(DENOMS.len() as i32) as usize;
        }
        // F: das zuletzt Hingelegte zurueck
        if input.take_out {
            if let Some(i) = t.given.pop() {
                t.wallet[i] += 1;
            }
        }
        // E: hinlegen
        if input.interact {
            if t.wallet[t.sel] == 0 {
                self.hint("Davon hast du nichts mehr in der Geldbörse.");
                return;
            }
            t.wallet[t.sel] -= 1;
            t.given.push(t.sel);
            let sum = t.given_sum();
            if sum == total {
                t.given.clear();
                t.pay = Pay::Idle;
                self.play(Sfx::Coin, 0.7);
                self.pay();
                return;
            }
            if sum > total {
                t.take_back_all();
                self.till_mistake("\"Das ist zu viel. PASSEND, bitte.\"");
                return;
            }
            self.play(Sfx::Coin, 0.5);
        }
        let Some(t) = &mut self.till else { return };
        let left = left - dt;
        if left > 0.0 {
            t.pay = Pay::Cash(left);
            return;
        }
        t.take_back_all();
        t.pay = Pay::Cash(CASH_TIME);
        self.till_doubt(SUS_CASH_SLOW, "\"Das dauert. Die anderen warten.\"");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_always_allows_exact_payment() {
        let mut rng = Pcg64::seed_from_u64(3);
        for total in [1, 99, 1234, 4999, 8765, 23] {
            let w = fill_wallet(total, &mut rng);
            // gierig von gross nach klein mit dem, was da ist
            let mut rest = total;
            for (i, &d) in DENOMS.iter().enumerate() {
                let n = (rest / d).min(w[i] as i64);
                rest -= n * d;
            }
            assert_eq!(rest, 0, "{total} ct nicht passend zahlbar");
            let extra: u32 = w.iter().sum();
            assert!(extra > 0);
        }
    }
}
