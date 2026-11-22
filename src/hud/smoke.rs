//! Rauch der nutzlosen Zigarette (die Zigarette selbst ist ein 3D-Modell, siehe
//! game/smoke.rs): eine Rauchfahne, die an der Glut aufsteigt, und nach jedem Zug eine
//! Wolke ausgeatmeter Rauch uebers Bild.

use crate::game::smoke::Cigarette;
use crate::game::Game;
use crate::ui::{self, Ui};

pub fn draw(u: &mut Ui, g: &Game, time: f32) {
    // ausgeatmeter Rauch: breitet sich aus und wird duenner
    if g.exhale > 0.0 {
        let a = (g.exhale / 2.2).clamp(0.0, 1.0);
        let spread = 1.0 - a;
        for k in 0..12 {
            let ang = k as f32 * 0.52 + time * 0.3;
            let r = 10.0 + spread * (40.0 + (k % 3) as f32 * 18.0);
            let x = 240.0 + ang.cos() * r * 1.6;
            let y = 196.0 - spread * 50.0 + ang.sin() * r * 0.6;
            let s = 14.0 + spread * 26.0;
            u.rect(x - s * 0.5, y - s * 0.5, s, s, [0.72, 0.72, 0.74, 0.16 * a]);
        }
    }

    // Rauchfahne an der Glut (nicht im Flug)
    if matches!(g.cig, Cigarette::None | Cigarette::Flick { .. }) {
        return;
    }
    let Some(ember) = g.cigarette_ember() else {
        return;
    };
    let clip = g.view_proj(g.aspect) * ember.extend(1.0);
    if clip.w <= 0.01 {
        return;
    }
    let ex = (clip.x / clip.w * 0.5 + 0.5) * ui::W;
    let ey = (0.5 - clip.y / clip.w * 0.5) * ui::H;
    for i in 0..7 {
        let p = (time * 0.55 + i as f32 / 7.0).fract();
        let x = ex + (p * 7.0 + i as f32).sin() * (2.0 + p * 6.0);
        let y = ey - 2.0 - p * 40.0;
        let s = 2.0 + p * 5.0;
        u.rect(
            x - s * 0.5,
            y - s * 0.5,
            s,
            s,
            [0.8, 0.8, 0.82, 0.3 * (1.0 - p)],
        );
    }
}
