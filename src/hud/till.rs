//! HUD des Kassen-Minispiels: Misstrauen der Kassiererin, Kassen-Leben, ihre Fragen,
//! Karte durchziehen und PIN-Eingabe am Terminal.

use crate::game::till::cash::{denom_label, CASH_TIME, DENOMS};
use crate::game::till::{Pay, Till, PIN_TIME, QUESTION_TIME, TILL_LIVES};
use crate::game::Game;
use crate::ui::{self, euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

pub const CARD_W: f32 = 144.0;
pub const CARD_H: f32 = 88.0;

/// Die Karte des Spielers: "SCAMCART", seit 1989 abgelaufen. Kraenklicher Farbverlauf, der
/// leicht wabert, ein Auge, das sich umsieht, ein Riss, ein roter Stempel.
fn draw_scamcart(u: &mut Ui, x: f32, y: f32, time: f32) {
    // Farbverlauf in Baendern: Lila oben, Gallengruen unten, jedes Band zittert seitlich
    let bands = 8;
    for b in 0..bands {
        let k = b as f32 / (bands - 1) as f32;
        let c = [
            0.42 + (0.22 - 0.42) * k,
            0.1 + (0.4 - 0.1) * k,
            0.45 + (0.18 - 0.45) * k,
            1.0,
        ];
        let wob = ((time * 3.0 + b as f32 * 1.7).sin() * 1.2).round();
        let bh = CARD_H / bands as f32;
        u.rect(
            x + wob.max(0.0),
            y + b as f32 * bh,
            CARD_W - wob.abs(),
            bh + 0.5,
            c,
        );
    }
    // duenner dunkler Rand
    let edge = [0.05, 0.02, 0.06, 1.0];
    u.rect(x, y, CARD_W, 1.0, edge);
    u.rect(x, y + CARD_H - 1.0, CARD_W, 1.0, edge);
    u.rect(x, y, 1.0, CARD_H, edge);
    u.rect(x + CARD_W - 1.0, y, 1.0, CARD_H, edge);

    // Schriftzug; flackert ab und zu rot versetzt
    let ink = [0.95, 0.9, 0.55, 1.0];
    if (time * 0.9).fract() < 0.12 {
        u.text(x + 9.0, y + 5.0, "SCAMCART", [0.9, 0.1, 0.1, 0.8], 1.0);
    }
    u.text(x + 8.0, y + 5.0, "SCAMCART", ink, 1.0);

    // Chip mit Kontaktlinien
    let (chx, chy) = (x + 10.0, y + 20.0);
    u.rect(chx, chy, 18.0, 13.0, [0.85, 0.72, 0.3, 1.0]);
    for l in 0..3 {
        u.rect(
            chx,
            chy + 3.0 + l as f32 * 3.5,
            18.0,
            1.0,
            [0.55, 0.42, 0.15, 1.0],
        );
    }
    u.rect(chx + 8.0, chy, 1.0, 13.0, [0.55, 0.42, 0.15, 1.0]);

    // Auge oben rechts, die Pupille sieht sich um
    let (ex, ey) = (x + CARD_W - 40.0, y + 18.0);
    let white = [0.92, 0.9, 0.85, 1.0];
    u.rect(ex + 6.0, ey, 20.0, 14.0, white);
    u.rect(ex, ey + 3.0, 32.0, 8.0, white);
    let look = (time * 0.7).sin() * 8.0 + (time * 2.3).sin() * 2.0;
    let blink = (time * 0.37).fract() < 0.04;
    if blink {
        u.rect(ex, ey, 32.0, 14.0, [0.32, 0.18, 0.4, 1.0]);
        u.rect(ex, ey + 7.0, 32.0, 1.0, edge);
    } else {
        let px = ex + 12.0 + look;
        u.rect(px, ey + 3.0, 8.0, 8.0, [0.7, 0.08, 0.1, 1.0]);
        u.rect(px + 2.0, ey + 5.0, 4.0, 4.0, [0.02, 0.0, 0.02, 1.0]);
    }

    // Nummer, Name, Ablaufdatum
    let txt = [0.92, 0.92, 0.88, 1.0];
    u.text(x + 8.0, y + 40.0, "0000 0000 0666", txt, 1.0);
    u.text(x + 8.0, y + 54.0, "KUNDE 0", txt, 1.0);
    u.text(
        x + 8.0,
        y + 70.0,
        "GÜLTIG BIS 01/89",
        [0.8, 0.85, 0.7, 1.0],
        1.0,
    );

    // Riss von oben rechts schraeg nach unten
    let crack = [0.06, 0.03, 0.06, 0.9];
    let mut cxp = x + CARD_W - 14.0;
    for s in 0..9 {
        let cyp = y + 1.0 + s as f32 * 4.0;
        u.rect(cxp, cyp, 1.0, 4.0, crack);
        cxp -= if s % 3 == 1 { 3.0 } else { 1.0 };
    }

    // Stempel "ABGELAUFEN" quer ueber die Karte, pulsiert leicht
    let a = 0.75 + 0.2 * (time * 4.0).sin();
    let red = [0.9, 0.12, 0.1, a];
    let (sx, sy, sw, sh) = (x + 48.0, y + 50.0, 88.0, 14.0);
    u.rect(sx, sy, sw, 1.0, red);
    u.rect(sx, sy + sh - 1.0, sw, 1.0, red);
    u.rect(sx, sy, 1.0, sh, red);
    u.rect(sx + sw - 1.0, sy, 1.0, sh, red);
    u.text(sx + 4.0, sy + 3.0, "ABGELAUFEN", red, 1.0);
}

/// Fenster des Kartenlesers: oben ein Display, darunter der Leser mit Schlitz. Erst steckt die
/// Karte im Portemonnaie, dann wandert sie mit der Maus waagerecht durch den Leser.
pub const READER_W: f32 = 300.0;
pub const READER_H: f32 = 176.0;
pub const READER_Y: f32 = 20.0;

fn draw_reader(u: &mut Ui, t: &Till, progress: f32, moving: bool, ready: bool, time: f32) {
    let (w, h) = (READER_W, READER_H);
    let (x0, y0) = ((ui::W - w) * 0.5, READER_Y);
    u.rect(x0, y0, w, h, [0.05, 0.05, 0.06, 0.92]);
    u.rect(x0, y0, w, 1.0, [0.9, 0.9, 0.85, 0.6]);
    u.text_center(y0 + 5.0, "KARTE DURCHZIEHEN", YELLOW, 1.0);

    // Display: Rueckmeldung des Lesers
    let (text, color) = match t.reader {
        Some((msg, _)) if msg == "ANGENOMMEN" => (msg, GREEN),
        Some((msg, _)) => (msg, RED),
        None if !ready => ("KARTE EINFÜHREN", [0.5, 0.9, 0.55, 1.0]),
        None if moving => ("LESE ...", [0.5, 0.9, 0.55, 1.0]),
        None => ("BITTE DURCHZIEHEN", [0.5, 0.9, 0.55, 1.0]),
    };
    let (lw, lx) = (184.0, (ui::W - 184.0) * 0.5);
    u.rect(lx, y0 + 17.0, lw, 14.0, [0.03, 0.12, 0.06, 1.0]);
    let tx = (ui::W - Ui::text_width(text, 1.0)) * 0.5;
    u.text(tx, y0 + 20.0, text, color, 1.0);

    let (rx, rw) = (x0 + 8.0, w - 16.0);
    let body = [0.2, 0.2, 0.23, 1.0];
    if ready {
        // Leser: Gehaeuse, Karte im Schlitz, vordere Lippe ueber der Oberkante der Karte
        u.rect(rx, y0 + 36.0, rw, 16.0, body);
        u.rect(rx, y0 + 44.0, rw, 2.0, [0.0, 0.0, 0.0, 1.0]);
        let cx = rx + progress.clamp(0.0, 1.0) * (rw - CARD_W);
        draw_scamcart(u, cx, y0 + 42.0, time);
        u.rect(rx, y0 + 36.0, rw, 12.0, [0.26, 0.26, 0.29, 1.0]);
        u.rect(rx, y0 + 47.0, rw, 1.0, [0.08, 0.08, 0.09, 1.0]);
        // Pfeil: in diese Richtung ziehen
        let ay = y0 + 40.0;
        for k in 0..3 {
            let ax = rx + rw * 0.5 - 12.0 + k as f32 * 10.0;
            u.text(ax, ay, ">", DIM, 1.0);
        }
    } else {
        u.rect(rx, y0 + 36.0, rw, 12.0, [0.26, 0.26, 0.29, 1.0]);
        u.rect(rx, y0 + 44.0, rw, 2.0, [0.0, 0.0, 0.0, 1.0]);
        // Portemonnaie: die Karte schaut oben heraus
        let cx = (ui::W - CARD_W) * 0.5;
        draw_scamcart(u, cx, y0 + 56.0, time);
        let (px, py, pw, ph) = (x0 + 62.0, y0 + 90.0, w - 124.0, 56.0);
        u.rect(px, py, pw, ph, [0.34, 0.19, 0.1, 1.0]);
        u.rect(px, py, pw, 3.0, [0.24, 0.13, 0.07, 1.0]);
        for k in 0..((pw / 8.0) as usize) {
            u.rect(
                px + 4.0 + k as f32 * 8.0,
                py + 6.0,
                4.0,
                1.0,
                [0.8, 0.65, 0.35, 1.0],
            );
        }
    }
    // Lampe am rechten Ende: gruen = angenommen, rot = Fehler, sonst gelb blinkend
    let lamp = match t.reader {
        Some(("ANGENOMMEN", _)) => GREEN,
        Some(_) => RED,
        None if (time * 2.0).fract() < 0.5 => YELLOW,
        None => [0.3, 0.25, 0.1, 1.0],
    };
    u.rect(rx + rw - 10.0, y0 + 38.0, 6.0, 6.0, lamp);

    let hint = if ready {
        "Maus gleichmäßig nach rechts"
    } else {
        "[E] Karte aus dem Portemonnaie nehmen"
    };
    u.text_center(y0 + h - 30.0, hint, WHITE, 1.0);
    if ready {
        u.text_center(
            y0 + h - 20.0,
            "nicht zu schnell - nicht zu langsam",
            DIM,
            1.0,
        );
    }
    u.text_center(y0 + h - 10.0, "F: Abbruch", DIM, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payment_screens_draw_and_the_card_fits_its_window() {
        // Karte im Leser bzw. im Portemonnaie: bleibt ueber den Hinweisen (unterste 30 px)
        let in_reader = READER_Y + 42.0 + CARD_H;
        let in_wallet = READER_Y + 56.0 + CARD_H;
        let pocket_bottom = READER_Y + 90.0 + 56.0;
        let hints = READER_Y + READER_H - 30.0;
        assert!(in_reader <= hints, "Karte im Leser verdeckt die Hinweise");
        assert!(
            in_wallet <= pocket_bottom,
            "Karte schaut unten aus dem Portemonnaie"
        );
        assert!(pocket_bottom <= hints, "Portemonnaie verdeckt die Hinweise");
        assert!(
            READER_W - 16.0 > CARD_W + 100.0,
            "zu wenig Weg zum Durchziehen"
        );
        // 12 = Kartenzahlung, 13 = Barzahlung (Cheats)
        for k in [12, 13] {
            let mut g = Game::new(60 + k as u64);
            g.cheat(k);
            let mut u = Ui::default();
            for i in 0..40 {
                draw(&mut u, &g, i as f32 * 0.37);
            }
            assert!(u.verts.len() > 500, "Bezahlfenster leer");
        }
        // alle Zeichen von SCAMCART/ABGELAUFEN/GUELTIG sind im Schrift-Atlas
        let mut u = Ui::default();
        draw_scamcart(&mut u, 100.0, 50.0, 0.0);
        assert!(!u.verts.is_empty());
    }
}

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
        Pay::Swipe {
            progress,
            started,
            ready,
            ..
        } => draw_reader(u, t, progress, started.is_some(), ready, time),
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
