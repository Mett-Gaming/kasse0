//! Barzahlen: Manchmal "geht die Kartenzahlung heute nicht". Dann muss man aus der
//! Geldboerse schnell und passend bezahlen. Scheine und Muenzen waehlt man mit A/D oder dem
//! Mausrad, E (Linksklick) legt hin, F nimmt das zuletzt Hingelegte zurueck. Sobald genau der
//! Betrag auf dem Tresen liegt, ist bezahlt. Zu viel oder zu langsam macht sie misstrauisch.

use super::*;

/// Scheine und Muenzen in Cent (gross nach klein)
pub const DENOMS: [i64; 12] = [5000, 2000, 1000, 500, 200, 100, 50, 20, 10, 5, 2, 1];
/// So oft muss man bar zahlen (sonst mit Karte): halbe-halbe
pub const CASH_CHANCE: f64 = 0.5;
/// So lange hat man, um passend hinzulegen: je nach Stress an der Kasse (till/pressure.rs)
/// von `CASH_TIME_MAX` bis `CASH_TIME_MIN` Sekunden
pub const CASH_TIME_MAX: f32 = 18.0;
pub const CASH_TIME_MIN: f32 = 12.0;

pub fn cash_time(stress: f32) -> f32 {
    CASH_TIME_MAX - (CASH_TIME_MAX - CASH_TIME_MIN) * stress.clamp(0.0, 1.0)
}
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

/// Geldboerse: genug, um `total` passend zu zahlen, dazu Extra-Stuecke.
/// Mit `stress` (Kassen-Druck, None = ruhig) wird es fieser: mehr Kleingeld-Chaos, und
/// manchmal fehlt genau ein Stueck der naheliegenden Loesung - es liegt dann in kleineren
/// Stuecken in der Boerse, man muss anders kombinieren. Passend zahlen geht immer.
pub fn fill_wallet(total: i64, rng: &mut Pcg64, stress: Option<f32>) -> [u32; DENOMS.len()] {
    let mut w = [0u32; DENOMS.len()];
    let mut rest = total.max(0);
    for (i, &d) in DENOMS.iter().enumerate() {
        let n = rest / d;
        w[i] += n as u32;
        rest -= n * d;
    }
    let Some(s) = stress else {
        for _ in 0..rng.gen_range(3..=6) {
            w[rng.gen_range(0..DENOMS.len())] += 1;
        }
        return w;
    };
    // fehlendes Stueck: eins aus der naheliegenden Loesung durch kleinere ersetzen
    let mut missing = None;
    if rng.gen_bool((0.3 + 0.4 * s as f64).min(1.0)) {
        let used: Vec<usize> = (0..DENOMS.len() - 1).filter(|&i| w[i] > 0).collect();
        if !used.is_empty() {
            let i = used[rng.gen_range(0..used.len())];
            w[i] -= 1;
            let mut rest = DENOMS[i];
            for (j, &d) in DENOMS.iter().enumerate().skip(i + 1) {
                let n = rest / d;
                w[j] += n as u32;
                rest -= n * d;
            }
            missing = Some(i);
        }
    }
    // Kleingeld-Chaos: viele Extra-Stuecke, meist Muenzen (aber nie das fehlende)
    for _ in 0..rng.gen_range(6..=12) {
        let i = if rng.gen_bool(0.75) {
            rng.gen_range(4..DENOMS.len())
        } else {
            rng.gen_range(0..4)
        };
        if Some(i) != missing {
            w[i] += 1;
        }
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
        // unter Druck: weniger Zeit und eine chaotischere Geldboerse
        let stress = (!self.calm_till).then(|| self.till_stress());
        let Some(t) = &mut self.till else { return };
        t.cash = true;
        t.wallet = fill_wallet(total, &mut self.rng, stress);
        t.given.clear();
        t.sel = 0;
        t.cash_limit = stress.map_or(CASH_TIME_MAX, cash_time);
        t.pay = Pay::Cash(t.cash_limit);
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
        t.pay = Pay::Cash(t.cash_limit);
        self.till_doubt(SUS_CASH_SLOW, "\"Das dauert. Die anderen warten.\"");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Laesst sich `total` mit genau diesen Stuecken legen? (beschraenkte Teilsummen)
    fn payable(total: i64, w: &[u32; DENOMS.len()]) -> bool {
        let n = total as usize;
        let mut can = vec![false; n + 1];
        can[0] = true;
        for (i, &d) in DENOMS.iter().enumerate() {
            let d = d as usize;
            for _ in 0..w[i] {
                for s in (d..=n).rev() {
                    if can[s - d] {
                        can[s] = true;
                    }
                }
            }
        }
        can[n]
    }

    #[test]
    fn wallet_always_allows_exact_payment() {
        let mut rng = Pcg64::seed_from_u64(3);
        for stress in [None, Some(0.0), Some(0.5), Some(1.0)] {
            for total in [1, 99, 1234, 4999, 8765, 23, 2000, 150] {
                let w = fill_wallet(total, &mut rng, stress);
                assert!(
                    payable(total, &w),
                    "{total} ct nicht passend zahlbar ({stress:?})"
                );
            }
        }
    }

    #[test]
    fn under_pressure_the_wallet_is_messier_and_time_shorter() {
        let mut rng = Pcg64::seed_from_u64(9);
        let pieces = |w: [u32; DENOMS.len()]| w.iter().sum::<u32>();
        let mut calm = 0;
        let mut hard = 0;
        for k in 0..60 {
            let total = 1234 + k * 17;
            calm += pieces(fill_wallet(total, &mut rng, None));
            hard += pieces(fill_wallet(total, &mut rng, Some(1.0)));
        }
        assert!(
            hard > calm + 60 * 3,
            "kein Kleingeld-Chaos: {calm} vs {hard}"
        );
        assert!(cash_time(1.0) < cash_time(0.0));
        assert_eq!(cash_time(0.0), CASH_TIME_MAX);
        assert_eq!(cash_time(1.0), CASH_TIME_MIN);
    }
}
