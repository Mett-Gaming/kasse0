//! Fenster am Pfandautomaten (Logik und Layout: game/pfand/machine.rs).

use super::KeyHints;
use crate::game::belt_window::{inside, Rect};
use crate::game::pfand::machine::{
    MachineWindow, ARM_TIME, BTN_INSERT, BTN_LEFT, BTN_PRINT, BTN_RIGHT, SLOT, TURNS, WINDOW,
};
use crate::game::pfand::{Empty, BAG_MAX};
use crate::game::Game;
use crate::ui::{self, euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

fn button(u: &mut Ui, r: Rect, label: &str, hot: bool) {
    let back = if hot {
        [0.3, 0.3, 0.22, 1.0]
    } else {
        [0.14, 0.14, 0.15, 1.0]
    };
    u.rect(r.0, r.1, r.2, r.3, back);
    u.rect(r.0, r.1, r.2, 1.0, [0.8, 0.8, 0.75, 0.6]);
    let x = r.0 + (r.2 - Ui::text_width(label, 1.0)) * 0.5;
    u.text(x, r.1 + 5.0, label, if hot { YELLOW } else { WHITE }, 1.0);
}

/// Flasche im Schacht als kleines Bild: der Strichcode wandert mit der Drehung um sie herum.
fn bottle(u: &mut Ui, kind: Empty, turn: u8, cx: f32, cy: f32) {
    let (w, h, color) = match kind {
        Empty::Glass => (70.0, 22.0, [0.2, 0.42, 0.22, 1.0]),
        Empty::Reusable => (78.0, 24.0, [0.45, 0.6, 0.7, 1.0]),
        Empty::Plastic => (78.0, 26.0, [0.62, 0.72, 0.7, 1.0]),
        Empty::Can => (46.0, 24.0, [0.7, 0.7, 0.72, 1.0]),
    };
    u.rect(cx - w * 0.5, cy - h * 0.5, w, h, color);
    if kind != Empty::Can {
        u.rect(cx + w * 0.5, cy - 5.0, 12.0, 10.0, color);
        u.rect(
            cx + w * 0.5 + 12.0,
            cy - 6.0,
            4.0,
            12.0,
            [0.8, 0.2, 0.15, 1.0],
        );
    }
    u.rect(cx - w * 0.5, cy - h * 0.5, w, 3.0, [1.0, 1.0, 1.0, 0.25]);
    // Lage des Strichcodes: Winkel 0 = oben (zum Leser), vorn sichtbar von -90 bis +90 Grad
    let a = turn as f32 / TURNS as f32 * std::f32::consts::TAU;
    let (s, c) = a.sin_cos();
    // Ansicht von vorn: oben = -h/2, unten = +h/2; hinten verdeckt
    let y = cy - c * h * 0.5;
    let facing = turn == 0 || s > 0.05;
    if facing {
        let th = 4.0 + s.abs() * 8.0;
        let y0 = (y - th * 0.5).clamp(cy - h * 0.5, cy + h * 0.5 - th.min(h));
        u.rect(cx - 16.0, y0, 32.0, th, [0.95, 0.95, 0.92, 1.0]);
        for k in 0..10 {
            let bw = if k % 3 == 0 { 2.0 } else { 1.0 };
            u.rect(
                cx - 14.0 + k as f32 * 2.9,
                y0 + 1.0,
                bw,
                th - 2.0,
                [0.05, 0.05, 0.05, 1.0],
            );
        }
    }
    if turn == 0 {
        u.rect(
            cx - 20.0,
            cy - h * 0.5 - 4.0,
            40.0,
            2.0,
            [0.4, 1.0, 0.5, 0.9],
        );
    }
}

fn arm(u: &mut Ui, left: f32, time: f32) {
    // faehrt schnell heraus, zittert, faellt zurueck
    let k = 1.0 - (left / ARM_TIME).clamp(0.0, 1.0);
    let out = (k * 6.0).min(1.0) * ((1.0 - k) * 4.0).min(1.0);
    let shake = (time * 70.0).sin() * 3.0 * out;
    let (x, y, _, h) = SLOT;
    let skin = [0.62, 0.55, 0.5, 1.0];
    let len = 40.0 + 260.0 * out;
    let top = y + h * 0.5 - 14.0 + shake;
    u.rect(x + 20.0, top, len, 28.0, skin);
    u.rect(x + 20.0, top + 22.0, len, 6.0, [0.3, 0.22, 0.2, 1.0]);
    for f in 0..5 {
        let fy = top - 10.0 + f as f32 * 11.0;
        u.rect(
            x + 20.0 + len - 4.0,
            fy,
            42.0 - (f as f32 - 2.0).abs() * 6.0,
            7.0,
            skin,
        );
    }
}

pub fn draw(u: &mut Ui, g: &Game, w: &MachineWindow, time: f32) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.45]);
    let (x, y, ww, hh) = WINDOW;
    let hurt = w.arm > 0.0;
    let frame = if hurt { RED } else { [0.6, 0.6, 0.58, 1.0] };
    u.rect(x - 1.0, y - 1.0, ww + 2.0, hh + 2.0, frame);
    u.rect(x, y, ww, hh, [0.02, 0.02, 0.03, 0.97]);
    u.hint(x + 8.0, y + 6.0, "PFANDRÜCKGABE", YELLOW, 1.0);

    // Schacht mit Leser oben
    let (sx, sy, sw, sh) = SLOT;
    u.rect(sx, sy, sw, sh, [0.06, 0.07, 0.07, 1.0]);
    u.rect(
        sx + sw * 0.5 - 24.0,
        sy,
        48.0,
        3.0,
        [0.8, 0.1, 0.1, 0.5 + 0.4 * (time * 5.0).sin().abs()],
    );
    let (cx, cy) = (sx + sw * 0.5, sy + sh * 0.55);
    match w.bottle {
        Some((k, _)) => {
            bottle(u, k, w.turn, cx, cy);
            u.hint(
                sx + 4.0,
                sy + sh - 11.0,
                &format!("Pfand {}", euro(k.cents())),
                DIM,
                1.0,
            );
        }
        None => {
            let s = "leer";
            u.hint(cx - Ui::text_width(s, 1.0) * 0.5, cy - 4.0, s, DIM, 1.0);
        }
    }
    // Lampe
    let lamp = if w.green() { GREEN } else { RED };
    u.rect(sx + sw - 16.0, sy + 8.0, 10.0, 10.0, lamp);

    // Anzeige rechts
    let rx = sx + sw + 14.0;
    u.rect(rx, sy, 106.0, 30.0, [0.03, 0.12, 0.05, 1.0]);
    let ink = if hurt { RED } else { [0.5, 1.0, 0.55, 1.0] };
    for (k, l) in Ui::wrap(&w.text, 12).iter().take(3).enumerate() {
        u.text(rx + 4.0, sy + 3.0 + k as f32 * 9.0, l, ink, 1.0);
    }
    let sum = g.pfand.sums.get(w.machine).copied().unwrap_or(0);
    u.hint(rx, sy + 38.0, "SUMME", DIM, 1.0);
    u.hint(rx, sy + 49.0, &euro(sum), WHITE, 2.0);
    u.hint(
        rx,
        sy + 72.0,
        &format!("Tüte {}/{}", g.pfand.bag.len(), BAG_MAX),
        DIM,
        1.0,
    );

    let hot = |r: Rect| inside(r, w.mouse);
    button(u, BTN_LEFT, "<", hot(BTN_LEFT));
    button(u, BTN_INSERT, "REIN", hot(BTN_INSERT));
    button(u, BTN_RIGHT, ">", hot(BTN_RIGHT));
    button(u, BTN_PRINT, "BON DRUCKEN", hot(BTN_PRINT));
    u.hint_center(y + hh - 22.0, "A/D drehen   E einlegen   F Bon", DIM, 1.0);
    u.hint_center(
        y + hh - 11.0,
        "Strichcode oben, Lampe grün   Q zu",
        DIM,
        1.0,
    );

    if hurt {
        arm(u, w.arm, time);
    }
    // Zeiger
    let (mx, my) = w.mouse;
    u.rect(mx - 3.0, my, 7.0, 1.0, WHITE);
    u.rect(mx, my - 3.0, 1.0, 7.0, WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_lie_inside_the_window_and_do_not_overlap() {
        let all = [SLOT, BTN_LEFT, BTN_INSERT, BTN_RIGHT, BTN_PRINT];
        for (i, a) in all.iter().enumerate() {
            assert!(a.0 >= WINDOW.0 && a.0 + a.2 <= WINDOW.0 + WINDOW.2);
            assert!(a.1 >= WINDOW.1 && a.1 + a.3 <= WINDOW.1 + WINDOW.3 - 24.0);
            for b in &all[i + 1..] {
                let apart =
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
                assert!(apart, "{a:?} {b:?}");
            }
        }
        assert!(Ui::text_width("BON DRUCKEN", 1.0) <= BTN_PRINT.2);
    }
}
