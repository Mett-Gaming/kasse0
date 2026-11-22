//! Kassenbon und Fehlscans: Die Kassiererin scannt manchmal etwas doppelt oder viel zu teuer.
//! Auf dem Kassendisplay (HUD) ist das nicht markiert - man muss es selbst bemerken und mit R
//! reklamieren. Wer bezahlt, ohne es zu merken, verliert dafuer ein Kassen-Leben.
//! Wer reklamiert, obwohl alles stimmt, macht sie misstrauisch.

use super::*;

/// Hoechstens so viele Fehlscans pro Einkauf
const MAX_WRONG: usize = 2;
/// Chance pro gescannter Ware (Grundwert + Anteil mal Stress)
const WRONG_BASE: f64 = 0.08;
const WRONG_STRESS: f64 = 0.22;
/// Misstrauen bei einer unberechtigten Reklamation
pub const SUS_FALSE_COMPLAINT: f32 = 0.3;

/// Was an einer Bonzeile falsch ist.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wrong {
    /// dieselbe Ware wurde ein zweites Mal gescannt (diese Zeile ist zu viel)
    Double,
    /// viel zu teuer; `extra` = so viel zu viel (Cent)
    Overpriced { extra: i64 },
}

/// Eine Zeile auf dem Kassendisplay.
#[derive(Clone, Debug, PartialEq)]
pub struct ReceiptLine {
    pub name: String,
    pub cents: i64,
    pub wrong: Option<Wrong>,
    pub reclaimed: bool,
}

impl ReceiptLine {
    /// Zu viel berechnet (Cent), solange nicht reklamiert.
    fn overcharge(&self) -> i64 {
        match (self.wrong, self.reclaimed) {
            (Some(Wrong::Double), false) => self.cents,
            (Some(Wrong::Overpriced { extra }), false) => extra,
            _ => 0,
        }
    }

    /// Was auf dem Display steht (reklamierte Doppel verschwinden, Preise werden korrigiert).
    pub fn shown(&self) -> Option<(&str, i64)> {
        match (self.wrong, self.reclaimed) {
            (Some(Wrong::Double), true) => None,
            (Some(Wrong::Overpriced { extra }), true) => Some((&self.name, self.cents - extra)),
            _ => Some((&self.name, self.cents)),
        }
    }
}

impl Till {
    /// Zu viel berechnet, insgesamt (Cent).
    pub fn overcharge(&self) -> i64 {
        self.receipt.iter().map(|l| l.overcharge()).sum()
    }

    /// Wie viele Fehlscans sind noch nicht reklamiert?
    pub fn wrong_open(&self) -> usize {
        self.receipt
            .iter()
            .filter(|l| l.wrong.is_some() && !l.reclaimed)
            .count()
    }

    fn wrong_count(&self) -> usize {
        self.receipt.iter().filter(|l| l.wrong.is_some()).count()
    }
}

impl Game {
    /// Gescannte Ware auf den Bon - manchmal falsch (doppelt oder zu teuer).
    pub(in crate::game) fn till_receipt_scan(&mut self, item: &ItemData) {
        let s = self.till_stress() as f64;
        let calm = self.calm_till;
        let Some(t) = &mut self.till else { return };
        t.receipt.push(ReceiptLine {
            name: item.name.clone(),
            cents: item.cents,
            wrong: None,
            reclaimed: false,
        });
        if calm || t.wrong_count() >= MAX_WRONG || item.cents <= 0 {
            return;
        }
        if !self.rng.gen_bool((WRONG_BASE + WRONG_STRESS * s).min(1.0)) {
            return;
        }
        let double = self.rng.gen_bool(0.5);
        let mult = self.rng.gen_range(5..=20);
        Self::make_wrong(t, double, mult);
    }

    /// Letzte Bonzeile falsch machen: doppelt scannen oder Preis mal `mult`.
    fn make_wrong(t: &mut Till, double: bool, mult: i64) {
        let Some(last) = t.receipt.last() else {
            return;
        };
        // vom fairen Preis der letzten Ware ausgehen
        let (name, fair) = match last.wrong {
            Some(Wrong::Overpriced { extra }) => (last.name.clone(), last.cents - extra),
            _ => (last.name.clone(), last.cents),
        };
        // ist die letzte Zeile schon ein Fehlscan, wird es ein Doppel-Scan (Bon und Summe
        // passen dann immer zusammen)
        let double = double || last.wrong.is_some();
        let line = if double {
            ReceiptLine {
                name,
                cents: fair,
                wrong: Some(Wrong::Double),
                reclaimed: false,
            }
        } else {
            let extra = fair * (mult - 1);
            ReceiptLine {
                name,
                cents: fair + extra,
                wrong: Some(Wrong::Overpriced { extra }),
                reclaimed: false,
            }
        };
        if double {
            t.receipt.push(line);
        } else if let Some(l) = t.receipt.last_mut() {
            *l = line;
        }
    }

    /// Fuer Tests: die letzte Bonzeile sicher falsch machen.
    #[cfg(test)]
    pub fn debug_force_wrong_scan(&mut self, double: bool) {
        if let Some(t) = &mut self.till {
            Self::make_wrong(t, double, 10);
        }
    }

    /// R: reklamieren. Stimmt etwas nicht, korrigiert sie es; sonst wird sie misstrauisch.
    pub(in crate::game) fn till_complain(&mut self) {
        if !self.till_active() || self.till_paying() {
            return;
        }
        let Some(t) = &mut self.till else { return };
        let open = t
            .receipt
            .iter_mut()
            .rev()
            .find(|l| l.wrong.is_some() && !l.reclaimed);
        match open {
            Some(line) => {
                line.reclaimed = true;
                self.say("Kassiererin: \"Oh. Entschuldigung.\" (korrigiert)", 3.0);
                self.play(Sfx::Beep, 0.5);
            }
            None => self.till_doubt(
                SUS_FALSE_COMPLAINT,
                "Kassiererin: \"Da stimmt alles.\" Sie mustert dich.",
            ),
        }
    }

    /// Beim Bezahlen: jeder nicht bemerkte Fehlscan kostet ein Leben (und wird dann still
    /// korrigiert). Liefert, ob es weitergeht (noch Leben uebrig).
    pub(in crate::game) fn till_charge_unnoticed(&mut self) -> bool {
        let open = self.till.as_ref().map_or(0, |t| t.wrong_open());
        for _ in 0..open {
            if let Some(t) = &mut self.till {
                if let Some(l) = t
                    .receipt
                    .iter_mut()
                    .find(|l| l.wrong.is_some() && !l.reclaimed)
                {
                    l.reclaimed = true;
                }
            }
            self.till_mistake("Du hast den falschen Preis auf dem Display nicht bemerkt.");
            if self.outcome != Outcome::Running {
                return false;
            }
        }
        true
    }
}
