//! HUD des Kassen-Minispiels: Misstrauen der Kassiererin, Kassen-Leben, ihre Fragen,
//! Karte durchziehen und PIN-Eingabe am Terminal.

use crate::game::till::{Pay, PIN_TIME, QUESTION_TIME, TILL_LIVES};
use crate::game::Game;
use crate::ui::{self, Ui, DIM, RED, WHITE, YELLOW};

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
