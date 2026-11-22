//! HUD des Kassen-Minispiels: Misstrauen der Kassiererin, Kassen-Leben, ihre Fragen,
//! Karte durchziehen und PIN-Eingabe am Terminal.

use crate::game::till::cash::{denom_label, CASH_TIME, DENOMS};
use crate::game::till::{Pay, Till, PIN_TIME, QUESTION_TIME, TILL_LIVES};
use crate::game::Game;
use crate::ui::{self, euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

/// Anzahl Scheine (die ersten Eintraege in DENOMS); der Rest sind Muenzen.
const NOTES: usize = 4;

/// Geldboerse: oben gross, was noch offen ist; darunter eine Reihe Scheine und eine Reihe
/// Muenzen als Kacheln (Wert und Anzahl), die gewaehlte gelb umrandet.
fn draw_cash(u: &mut Ui, g: &Game, t: &Till, left: f32) {
    let (x0, y0, w, h) = (60.0, 28.0, 360.0, 162.0);
    u.rect(x0, y0, w, h, [0.04, 0.04, 0.05, 0.94]);
    u.rect(x0, y0, w, 1.0, [0.9, 0.9, 0.85, 0.6]);
    let total = g.scanned_value();
    let given = t.given_sum();
    let rest = total - given;

    u.text_center(y0 + 6.0, "BAR ZAHLEN - GENAU PASSEND", YELLOW, 1.0);
    u.text_center(y0 + 19.0, "NOCH OFFEN", DIM, 1.0);
    u.text_center(y0 + 29.0, &euro(rest), WHITE, 2.0);
    u.text_center(
        y0 + 50.0,
        &format!("Betrag {}   -   hingelegt {}", euro(total), euro(given)),
        if given > 0 { GREEN } else { DIM },
        1.0,
    );

    // Scheine (eine Reihe, breit) und Muenzen (eine Reihe, schmaler)
    let row = |u: &mut Ui, label: &str, y: f32, range: std::ops::Range<usize>, tw: f32| {
        let gap = 6.0;
        let n = range.len() as f32;
        let x_start = (ui::W - (n * tw + (n - 1.0) * gap)) * 0.5;
        u.text(x_start, y - 10.0, label, DIM, 1.0);
        for (k, i) in range.enumerate() {
            let x = x_start + k as f32 * (tw + gap);
            let th = 24.0;
            let have = t.wallet[i];
            let sel = i == t.sel;
            let fill = if have == 0 {
                [0.18, 0.18, 0.2, 1.0]
            } else if i < NOTES {
                [0.55, 0.75, 0.62, 1.0] // Schein: hellgruen
            } else if DENOMS[i] >= 10 {
                [0.92, 0.8, 0.38, 1.0] // Gold
            } else {
                [0.82, 0.55, 0.35, 1.0] // Kupfer
            };
            if sel {
                u.rect(x - 2.0, y - 2.0, tw + 4.0, th + 4.0, YELLOW);
            }
            u.rect(x, y, tw, th, fill);
            let ink = if have == 0 {
                DIM
            } else {
                [0.06, 0.06, 0.08, 1.0]
            };
            let v = denom_label(i);
            u.text(
                x + (tw - Ui::text_width(&v, 1.0)) * 0.5,
                y + 3.0,
                &v,
                ink,
                1.0,
            );
            let c = format!("x{have}");
            u.text(
                x + (tw - Ui::text_width(&c, 1.0)) * 0.5,
                y + 13.0,
                &c,
                ink,
                1.0,
            );
        }
    };
    row(u, "SCHEINE", y0 + 74.0, 0..NOTES, 62.0);
    row(u, "MÜNZEN", y0 + 112.0, NOTES..DENOMS.len(), 38.0);

    let frac = (left / CASH_TIME).clamp(0.0, 1.0);
    let c = if frac < 0.3 {
        RED
    } else {
        [0.9, 0.3, 0.2, 0.8]
    };
    u.rect(x0 + 8.0, y0 + h - 18.0, (w - 16.0) * frac, 2.0, c);
    u.text_center(
        y0 + h - 12.0,
        "A/D wählen  -  E hinlegen  -  F zurück",
        WHITE,
        1.0,
    );
}

pub fn draw(u: &mut Ui, g: &Game, time: f32) {
    let Some(t) = g.till.as_ref().filter(|_| g.till_active()) else {
        return;
    };

    // Misstrauen unten links, daneben die drei Kassen-Leben
    super::meter(u, "KASSIERERIN", t.doubt, time);
    u.text_s(92.0, 239.0, "LEBEN", DIM, 1.0);
    for k in 0..TILL_LIVES {
        let x = 92.0 + k as f32 * 10.0;
        u.rect(x - 1.0, 248.0, 9.0, 9.0, [0.0, 0.0, 0.0, 0.6]);
        if k < t.lives {
            u.rect(x, 249.0, 7.0, 7.0, RED);
        }
    }

    // Frage der Kassiererin
    if let Some(a) = &t.asked {
        let q = a.question();
        u.rect(40.0, 160.0, 400.0, 86.0, [0.02, 0.02, 0.03, 0.85]);
        u.rect(40.0, 160.0, 400.0, 1.0, [0.9, 0.9, 0.85, 0.6]);
        u.text(48.0, 166.0, "KASSIERERIN:", DIM, 1.0);
        match a.reply {
            Some((reply, _)) => u.text(48.0, 180.0, reply, WHITE, 1.0),
            None => {
                u.text(48.0, 180.0, q.text, WHITE, 1.0);
                for k in 0..3 {
                    let line = format!("{}) {}", k + 1, q.answers[a.order[k]]);
                    u.text(56.0, 198.0 + k as f32 * 12.0, &line, YELLOW, 1.0);
                }
                let frac = (a.timer / QUESTION_TIME).clamp(0.0, 1.0);
                u.rect(48.0, 238.0, 384.0 * frac, 2.0, [0.9, 0.3, 0.2, 0.8]);
            }
        }
    }

    // Kartenterminal
    let (w, h) = (120.0, 112.0);
    let (x0, y0) = ((ui::W - w) * 0.5, 54.0);
    let panel = |u: &mut Ui, title: &str| {
        u.rect(x0, y0, w, h, [0.05, 0.05, 0.06, 0.9]);
        u.rect(x0, y0, w, 1.0, [0.9, 0.9, 0.85, 0.6]);
        u.text_center(y0 + 6.0, title, YELLOW, 1.0);
    };
    match t.pay {
        Pay::Idle => {}
        Pay::Cash(left) => draw_cash(u, g, t, left),
        Pay::Swipe { progress, .. } => {
            panel(u, "KARTE DURCHZIEHEN");
            // Schlitz und Karte, die mit der Maus nach unten wandert
            let cx = ui::W * 0.5;
            u.rect(cx - 24.0, y0 + 20.0, 48.0, 3.0, [0.0, 0.0, 0.0, 1.0]);
            let cy = y0 + 22.0 + progress.min(1.0) * 52.0;
            u.rect(cx - 20.0, cy, 40.0, 26.0, [0.25, 0.45, 0.8, 1.0]);
            u.rect(cx - 20.0, cy + 5.0, 40.0, 4.0, [0.05, 0.05, 0.05, 1.0]);
            u.rect(cx - 16.0, cy + 14.0, 8.0, 6.0, [0.85, 0.75, 0.3, 1.0]);
            u.text_center(y0 + h - 22.0, "Maus ruhig nach unten", WHITE, 1.0);
            u.text_center(y0 + h - 11.0, "nicht zu schnell  [E] Abbruch", DIM, 1.0);
        }
        Pay::ShowPin(_) => {
            panel(u, "IHRE PIN");
            let pin: String = t
                .pin
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            u.text_center(y0 + 40.0, &pin, WHITE, 2.0);
            u.text_center(y0 + h - 16.0, "Merk sie dir.", DIM, 1.0);
        }
        Pay::Pin(left) => {
            panel(u, "PIN EINGEBEN");
            let shown: String = (0..4)
                .map(|k| if k < t.typed.len() { "*" } else { "_" })
                .collect::<Vec<_>>()
                .join(" ");
            u.text_center(y0 + 40.0, &shown, WHITE, 2.0);
            let frac = (left / PIN_TIME).clamp(0.0, 1.0);
            let c = if frac < 0.3 {
                RED
            } else {
                [0.9, 0.3, 0.2, 0.8]
            };
            u.rect(x0 + 8.0, y0 + h - 26.0, (w - 16.0) * frac, 2.0, c);
            u.text_center(y0 + h - 16.0, "Zifferntasten 0-9", DIM, 1.0);
        }
    }
}
