//! Kassenfenster (Logik und Layout: game/belt_window.rs): links der Wagen als Kacheln, in der
//! Mitte das Band mit den Waren auf dem Weg zum Scanner, rechts Kassiererin und Bon.
//! Die gezogene Ware haengt am Mauszeiger.

use crate::game::belt_window::{
    cart_tile, weight_text, DragSrc, Rect, BELT_PANEL, CART_PANEL, HAND_TILE, TILE_H, TILE_ROWS,
    TILE_W, TILL_PANEL, WINDOW,
};
use crate::game::{Game, ItemData};
use crate::ui::{euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

const PANEL: [f32; 4] = [0.07, 0.07, 0.08, 0.95];
const FRAME: [f32; 4] = [0.85, 0.85, 0.8, 0.5];

fn panel(u: &mut Ui, r: Rect, title: &str, color: [f32; 4]) {
    u.rect(r.0, r.1, r.2, r.3, PANEL);
    u.rect(r.0, r.1, r.2, 1.0, FRAME);
    u.text(r.0 + 4.0, r.1 + 3.0, title, color, 1.0);
}

/// Eine Ware als Kachel: Farbstreifen in der Farbe der Ware, oben der Name, darunter das
/// Gewicht und ein Gewichtsbalken. Schwere Ware (muss zuerst aufs Band) ist rot.
fn tile(u: &mut Ui, r: Rect, item: &ItemData, lifted: bool) {
    let heavy = item.is_heavy();
    let bg = if lifted {
        [0.95, 0.9, 0.55, 1.0]
    } else if heavy {
        [0.95, 0.8, 0.78, 1.0]
    } else {
        [0.88, 0.87, 0.82, 1.0]
    };
    let t = item.tint;
    u.rect(r.0, r.1, r.2, r.3, bg);
    u.rect(r.0 + 2.0, r.1 + 2.0, 3.0, r.3 - 4.0, [t.x, t.y, t.z, 1.0]);
    let ink = [0.08, 0.08, 0.1, 1.0];
    let red = [0.7, 0.05, 0.05, 1.0];
    let kg = item.weight_kg();
    let w = weight_text(kg);
    let wc = if heavy { red } else { [0.3, 0.3, 0.32, 1.0] };
    let (tx, right) = (r.0 + 7.0, r.0 + r.2 - 3.0);
    if r.3 >= TILE_H {
        // zweizeilig: Name, darunter Gewicht und Balken (voll bei 3 kg)
        let short: String = item
            .name
            .chars()
            .take(((right - tx) / 8.0) as usize)
            .collect();
        u.text(tx, r.1 + 1.0, &short, ink, 1.0);
        u.text(tx, r.1 + 10.0, &w, wc, 1.0);
        let (bx, bw) = (r.0 + 60.0, right - r.0 - 60.0);
        u.rect(bx, r.1 + 12.0, bw, 4.0, [0.0, 0.0, 0.0, 0.25]);
        u.rect(bx, r.1 + 12.0, bw * (kg / 3.0).clamp(0.04, 1.0), 4.0, wc);
    } else {
        // einzeilig (Hand): Name links, Gewicht rechts
        let ww = Ui::text_width(&w, 1.0);
        let n = ((right - ww - 6.0 - tx) / 8.0) as usize;
        let short: String = item.name.chars().take(n).collect();
        let ty = r.1 + (r.3 - 8.0) * 0.5;
        u.text(tx, ty, &short, ink, 1.0);
        u.text(right - ww, ty, &w, wc, 1.0);
    }
}

pub fn draw(u: &mut Ui, g: &Game, time: f32) {
    let Some(w) = &g.belt_window else { return };
    // Markt dahinter abdunkeln, Fensterrahmen; starrt sie, blinkt der Rahmen rot
    u.rect(0.0, 0.0, crate::ui::W, crate::ui::H, [0.0, 0.0, 0.0, 0.45]);
    let staring = g.cashier_staring();
    let warn = g.cashier_looking() && !staring;
    let frame = if staring && (time * 6.0).fract() < 0.5 {
        RED
    } else {
        [0.6, 0.6, 0.58, 1.0]
    };
    let (x, y, ww, hh) = WINDOW;
    u.rect(x - 1.0, y - 1.0, ww + 2.0, hh + 2.0, frame);
    u.rect(x, y, ww, hh, [0.02, 0.02, 0.03, 0.97]);
    u.text_center(y + 4.0, "KASSE 0 - WARE AUFS BAND LEGEN", YELLOW, 1.0);

    // Wagen
    panel(u, CART_PANEL, &format!("WAGEN ({})", g.cart.len()), WHITE);
    let dragging = w.drag;
    for (i, item) in g.cart.iter().enumerate() {
        let Some(r) = cart_tile(i) else { break };
        if dragging == Some(DragSrc::Cart(i)) {
            u.rect(r.0, r.1, r.2, r.3, [0.2, 0.2, 0.22, 1.0]);
            continue;
        }
        tile(u, r, item, false);
    }
    let shown = TILE_ROWS * 2;
    if g.cart.len() > shown {
        let more = format!("+{} weitere", g.cart.len() - shown);
        let cp = CART_PANEL;
        let x = cp.0 + cp.2 - 4.0 - Ui::text_width(&more, 1.0);
        u.text(x, cp.1 + 3.0, &more, DIM, 1.0);
    }
    match &g.held {
        Some(item) if dragging != Some(DragSrc::Hand) => {
            u.text(HAND_TILE.0, HAND_TILE.1 - 9.0, "IN DER HAND", DIM, 1.0);
            tile(u, HAND_TILE, item, false);
        }
        _ => {}
    }

    // Band: Waren laufen von links zum Scanner rechts
    let bp = BELT_PANEL;
    panel(u, bp, "BAND", WHITE);
    let (bx, by, bw) = (bp.0 + 6.0, bp.1 + 44.0, bp.2 - 12.0);
    u.rect(bx, by, bw, 30.0, [0.05, 0.05, 0.05, 1.0]);
    // Laufrillen bewegen sich mit
    let off = (time * 18.0) % 10.0;
    let mut gx = bx + off;
    while gx < bx + bw {
        u.rect(gx, by, 1.0, 30.0, [0.12, 0.12, 0.12, 1.0]);
        gx += 10.0;
    }
    // Scanner am Ende: rotes Licht
    u.rect(
        bx + bw - 6.0,
        by - 4.0,
        6.0,
        38.0,
        [0.9, 0.1, 0.08, 0.5 + 0.3 * (time * 9.0).sin().abs()],
    );
    if let Some(lane) = g.open_lane() {
        let len = (lane.belt_end - lane.belt_start).length().max(0.1);
        for b in &g.belt {
            let k = (b.s / len).clamp(0.0, 1.0);
            let px = bx + 2.0 + k * (bw - 30.0);
            u.rect(
                px,
                by + 7.0,
                22.0,
                16.0,
                [b.data.tint.x, b.data.tint.y, b.data.tint.z, 1.0],
            );
        }
    }
    let hint = if dragging.is_some() {
        "HIER LOSLASSEN"
    } else {
        "hierher ziehen"
    };
    u.text(
        bp.0 + (bp.2 - Ui::text_width(hint, 1.0)) * 0.5,
        by + 40.0,
        hint,
        if dragging.is_some() { GREEN } else { DIM },
        1.0,
    );
    u.text(bp.0 + 4.0, bp.1 + 16.0, "Ab 1 kg zuerst!", DIM, 1.0);

    // Kassiererin und Bon
    let tp = TILL_PANEL;
    let (who, c) = if staring {
        ("SIE STARRT", RED)
    } else if warn {
        ("SIE SCHAUT AUF", RED)
    } else if g.scanned.is_empty() && g.belt.is_empty() {
        ("wartet", DIM)
    } else {
        ("scannt", GREEN)
    };
    panel(u, tp, who, c);
    if let Some(t) = &g.till {
        let lines: Vec<(&str, i64)> = t.receipt.iter().filter_map(|l| l.shown()).collect();
        let lines = &lines[lines.len().saturating_sub(7)..];
        let ink = [0.55, 0.95, 0.6, 1.0];
        for (k, (name, cents)) in lines.iter().enumerate() {
            let ly = tp.1 + 26.0 + k as f32 * 10.0;
            let short: String = name.chars().take(6).collect();
            u.text(tp.0 + 4.0, ly, &short, ink, 1.0);
            let price = euro(*cents);
            u.text(
                tp.0 + tp.2 - 4.0 - Ui::text_width(&price, 1.0),
                ly,
                &price,
                ink,
                1.0,
            );
        }
        let sum = euro(g.scanned_value());
        let sy = tp.1 + tp.3 - 12.0;
        u.rect(tp.0 + 4.0, sy - 2.0, tp.2 - 8.0, 1.0, [0.5, 0.9, 0.55, 0.4]);
        u.text(tp.0 + 4.0, sy, "SUMME", ink, 1.0);
        u.text(
            tp.0 + tp.2 - 4.0 - Ui::text_width(&sum, 1.0),
            sy,
            &sum,
            ink,
            1.0,
        );
    }

    // Steuerung
    u.text_center(
        y + hh - 10.0,
        "Ziehen: Ware aufs Band   R: reklamieren   E/Esc: schließen",
        DIM,
        1.0,
    );

    // gezogene Ware am Mauszeiger
    let item = match dragging {
        Some(DragSrc::Cart(i)) => g.cart.get(i),
        Some(DragSrc::Hand) => g.held.as_ref(),
        None => None,
    };
    if let Some(item) = item {
        let (mx, my) = w.mouse;
        tile(
            u,
            (mx - TILE_W * 0.5, my - TILE_H * 0.5, TILE_W, TILE_H),
            item,
            true,
        );
    }
}
