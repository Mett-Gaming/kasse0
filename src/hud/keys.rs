//! Bildschirm "Tasten belegen".

use super::*;
use crate::controls::{self, Action, Keys};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeyRow {
    Action(Action),
    Reset,
    Back,
}

const TOP: f32 = 58.0;
const STEP: f32 = 13.0;
const LEFT: f32 = 60.0;
const WIDTH: f32 = 360.0;
const KEY_X: f32 = 220.0;
const PAD_X: f32 = 325.0;

pub fn count() -> usize {
    controls::ACTIONS + 2
}

pub fn row(i: usize) -> KeyRow {
    match controls::all().nth(i) {
        Some(a) => KeyRow::Action(a),
        None if i == controls::ACTIONS => KeyRow::Reset,
        None => KeyRow::Back,
    }
}

fn top(i: usize) -> f32 {
    TOP + i as f32 * STEP + if i >= controls::ACTIONS { 3.0 } else { 0.0 }
}

pub fn hit(x: f32, y: f32) -> Option<usize> {
    if !(LEFT..LEFT + WIDTH).contains(&x) {
        return None;
    }
    (0..count()).find(|&i| (top(i) - 2.0..top(i) - 2.0 + STEP).contains(&y))
}

pub fn draw(u: &mut Ui, sel: usize, time: f32, keys: &Keys, waiting: Option<Action>, dark: bool) {
    let shade = if dark { 0.7 } else { 0.5 };
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, shade]);
    u.text_center(14.0, "TASTEN BELEGEN", WHITE, 2.0);
    u.rect(LEFT, 38.0, WIDTH, 208.0, [0.03, 0.03, 0.04, 0.8]);
    u.text(KEY_X, 43.0, "TASTATUR", DIM, 1.0);
    u.text(PAD_X, 43.0, "CONTROLLER", DIM, 1.0);
    for i in 0..count() {
        let y = top(i);
        let on = i == sel;
        let color = if on { YELLOW } else { WHITE };
        if on {
            let pulse = 0.12 + 0.05 * (time * 4.0).sin();
            u.rect(
                LEFT + 4.0,
                y - 2.0,
                WIDTH - 8.0,
                12.0,
                [0.95, 0.95, 0.92, pulse],
            );
        }
        match row(i) {
            KeyRow::Action(a) => {
                u.text_s(LEFT + 12.0, y, controls::name(a), color, 1.0);
                if waiting == Some(a) {
                    u.text_s(KEY_X, y, "Taste oder Knopf ...", RED, 1.0);
                } else {
                    u.text_s(KEY_X, y, &controls::label(keys, a, false), color, 1.0);
                    u.text_s(PAD_X, y, &controls::label(keys, a, true), color, 1.0);
                }
            }
            KeyRow::Reset => u.text_s(LEFT + 12.0, y, "Alles zurücksetzen", color, 1.0),
            KeyRow::Back => u.text_s(LEFT + 12.0, y, "Zurück", color, 1.0),
        }
    }
    let hint = if waiting.is_some() {
        "Esc: abbrechen"
    } else {
        "Enter / A: neu belegen - Esc / B: zurück"
    };
    u.text_center(254.0, hint, DIM, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_rows_fit_and_can_be_hit() {
        assert!(top(count() - 1) + 8.0 <= 246.0);
        assert_eq!(row(count() - 1), KeyRow::Back);
        for i in 0..count() {
            assert_eq!(hit(LEFT + 20.0, top(i) + 2.0), Some(i));
        }
        for a in controls::all() {
            assert!(LEFT + 12.0 + Ui::text_width(controls::name(a), 1.0) <= KEY_X);
            let k = controls::defaults();
            for (pad, x, end) in [(false, KEY_X, PAD_X), (true, PAD_X, LEFT + WIDTH)] {
                let w = Ui::text_width(&controls::label(&k, a, pad), 1.0);
                assert!(x + w <= end, "{a:?} zu breit");
            }
        }
    }
}
