//! Fenster am Kiosk (Logik und Layout: game/pfand/kiosk.rs).

use super::KeyHints;
use crate::catalog::misspell;
use crate::game::belt_window::{inside, Rect};
use crate::game::escalation::Tier;
use crate::game::pfand::items::Item;
use crate::game::pfand::kiosk::{
    first_bon, Column, KioskWindow, BONS, BON_ROWS, BUY, NOTE, ROW, SHELF, WINDOW,
};
use crate::game::Game;
use crate::ui::{self, euro, Ui, DIM, GREEN, WHITE, YELLOW};

fn panel(u: &mut Ui, r: Rect, title: &str, active: bool) {
    u.rect(r.0, r.1, r.2, r.3, [0.07, 0.07, 0.08, 0.95]);
    let c = if active { YELLOW } else { DIM };
    u.rect(r.0, r.1, r.2, 1.0, c);
    u.hint(r.0 + 4.0, r.1 + 3.0, title, c, 1.0);
}

fn mark(u: &mut Ui, r: Rect, k: usize, time: f32) {
    let pulse = 0.14 + 0.06 * (time * 4.0).sin();
    u.rect(
        r.0 + 2.0,
        r.1 + 12.0 + k as f32 * ROW,
        r.2 - 4.0,
        ROW,
        [0.95, 0.95, 0.9, pulse],
    );
}

pub fn draw(u: &mut Ui, g: &Game, w: &KioskWindow, time: f32) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.45]);
    let (x, y, ww, hh) = WINDOW;
    u.rect(x - 1.0, y - 1.0, ww + 2.0, hh + 2.0, [0.6, 0.6, 0.58, 1.0]);
    u.rect(x, y, ww, hh, [0.02, 0.02, 0.03, 0.97]);
    u.hint(x + 8.0, y + 5.0, "KIOSK", YELLOW, 1.0);
    u.hint(x + 8.0, y + 17.0, &w.text, WHITE, 1.0);

    panel(u, BONS, "PFANDBONS", w.column == Column::Bons);
    let first = first_bon(w.bon);
    for (k, bon) in g.pfand.bons.iter().skip(first).take(BON_ROWS).enumerate() {
        let on = first + k == w.bon;
        if on {
            mark(u, BONS, k, time);
        }
        let c = if on { YELLOW } else { WHITE };
        let s = euro(*bon);
        u.hint(BONS.0 + 10.0, BONS.1 + 14.0 + k as f32 * ROW, &s, c, 1.0);
    }
    if g.pfand.bons.is_empty() {
        u.hint(BONS.0 + 10.0, BONS.1 + 16.0, "keine", DIM, 1.0);
    }

    panel(u, SHELF, "AUSLAGE", w.column == Column::Items);
    let bon = g.pfand.bons.get(w.bon).copied().unwrap_or(0);
    for (k, item) in Item::ALL.iter().enumerate() {
        let ry = SHELF.1 + 14.0 + k as f32 * ROW;
        let on = k == w.item;
        if on {
            mark(u, SHELF, k, time);
        }
        let c = match (bon >= item.cents(), on) {
            (true, true) => YELLOW,
            (true, false) => WHITE,
            _ => DIM,
        };
        // ab "falsch" stimmt die Beschriftung nicht mehr (Preis und Wirkung schon)
        let name = if g.tier >= Tier::Wrong {
            misspell(item.name(), g.seed + k as u64)
        } else {
            item.name().to_string()
        };
        u.text(SHELF.0 + 10.0, ry, &name, c, 1.0);
        let p = euro(item.cents());
        let px = SHELF.0 + SHELF.2 - 8.0 - Ui::text_width(&p, 1.0);
        u.text(px, ry, &p, c, 1.0);
    }
    let item = Item::ALL[w.item.min(Item::ALL.len() - 1)];
    u.hint(x + 10.0, BUY.1 - 14.0, item.about(), DIM, 1.0);
    let hot = inside(BUY, w.mouse);
    let back = match (w.confirm, hot) {
        (true, _) => [0.1, 0.3, 0.12, 1.0],
        (_, true) => [0.3, 0.3, 0.22, 1.0],
        _ => [0.14, 0.14, 0.15, 1.0],
    };
    u.rect(BUY.0, BUY.1, BUY.2, BUY.3, back);
    let label = if w.confirm { "WIRKLICH" } else { "KAUFEN" };
    let lx = BUY.0 + (BUY.2 - Ui::text_width(label, 1.0)) * 0.5;
    let c = if w.confirm { GREEN } else { WHITE };
    u.text(lx, BUY.1 + 4.0, label, c, 1.0);
    u.hint(
        x + 10.0,
        y + hh - 22.0,
        "Ein Bon pro Kauf, Rest verfällt",
        DIM,
        1.0,
    );
    u.hint(
        x + 10.0,
        y + hh - 11.0,
        "W/S  A/D wählen   E kaufen   Q zu",
        DIM,
        1.0,
    );
    if w.note {
        draw_note(u, time);
        return;
    }
    let (mx, my) = w.mouse;
    u.rect(mx - 3.0, my, 7.0, 1.0, WHITE);
    u.rect(mx, my - 3.0, 1.0, 7.0, WHITE);
}

/// Zettel des Erbauers: vergilbtes Papier mit dunkler Schrift, leicht zitternd.
pub const NOTE_RECT: Rect = (90.0, 60.0, 300.0, 130.0);

fn draw_note(u: &mut Ui, time: f32) {
    let (x, y, w, h) = NOTE_RECT;
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.55]);
    u.rect(x + 3.0, y + 3.0, w, h, [0.0, 0.0, 0.0, 0.6]);
    u.rect(x, y, w, h, [0.78, 0.72, 0.56, 1.0]);
    // Flecken und eine eingerissene Ecke
    u.rect(x + w - 36.0, y + 8.0, 22.0, 14.0, [0.6, 0.5, 0.36, 0.6]);
    u.rect(x + 14.0, y + h - 30.0, 30.0, 10.0, [0.5, 0.2, 0.15, 0.35]);
    u.rect(
        x + w - 10.0,
        y + h - 10.0,
        10.0,
        10.0,
        [0.0, 0.0, 0.0, 0.55],
    );
    let ink = [0.16, 0.1, 0.06, 1.0];
    for (k, line) in NOTE.iter().enumerate() {
        let jitter = ((time * 7.0 + k as f32 * 1.9).sin() * 0.5).round();
        u.text(
            x + 14.0 + jitter,
            y + 16.0 + k as f32 * 16.0,
            line,
            ink,
            1.0,
        );
    }
    u.hint(
        x + 14.0,
        y + h - 14.0,
        "E / Klick: weglegen",
        [0.3, 0.24, 0.16, 1.0],
        1.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_prices_and_texts_fit() {
        for i in Item::ALL {
            let w = Ui::text_width(i.name(), 1.0) + Ui::text_width(&euro(i.cents()), 1.0) + 26.0;
            assert!(w <= SHELF.2, "{i:?} zu breit");
            assert!(Ui::text_width(i.about(), 1.0) <= WINDOW.2 - 20.0, "{i:?}");
        }
        assert!(14.0 + Item::ALL.len() as f32 * ROW <= SHELF.3 + 6.0);
        assert!(14.0 + BON_ROWS as f32 * ROW <= BONS.3);
        assert!(BUY.1 + BUY.3 + 6.0 <= WINDOW.1 + WINDOW.3);
        assert!(Ui::text_width("Ein Bon pro Kauf, Rest verfällt", 1.0) <= BUY.0 - WINDOW.0);
    }
}
