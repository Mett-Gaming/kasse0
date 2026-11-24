//! Werbung fuer HACKEPETER, das andere Spiel im Mett-Launcher (Mett-Gaming/hackepeter).
//! Drei Plakate im Wechsel, gebaut aus Flaechen, Boxen und Schriftzuegen:
//! A Fahndungsplakat ("GESUCHT"), B Rennplakat (Tempo-Streifen), C Aktionsplakat ("0,00 €").
//! Im gekippten Markt verblasst, verrottet (Shader) und mit unheimlichem Text.

use super::*;

/// Alle Schriftzuege der HACKEPETER-Plakate (stehen auch im Schrift-Atlas, siehe signs.rs)
pub const HACKEPETER_TEXTS: [&str; 15] = [
    "HACKEPETER",
    "Die Mettbrötchen-Flucht",
    "Jetzt im Mett-Launcher",
    "Er rennt immer noch",
    "Gib das Brötchen zurück",
    "GESUCHT",
    "GEFUNDEN",
    "Wegen Mettbrötchen-Diebstahl",
    "Er ist hier",
    "1 Dieb gegen 7 Cops",
    "Keiner kommt an",
    "0,00 €",
    "Gratis im Mett-Launcher",
    "Seit immer im Angebot",
    "Nicht mehr verfügbar",
];

/// Anzahl der Entwuerfe (A, B, C)
pub const HACKEPETER_STYLES: usize = 3;

/// Schriftfarben der Schilder (siehe `Builder::label`)
const WHITE: f32 = 0.0;
const BLACK: f32 = 2.0;
const YELLOW: f32 = 3.0;
const RED: f32 = 4.0;

/// Zeichnet auf eine Plakatflaeche: lokale Koordinaten u (quer) und v (hoch) in Metern
/// ab Plakatmitte, Ebenen liegen leicht vor dem Plakat.
struct Canvas<'a> {
    b: &'a mut Builder,
    c: Vec3,
    across: Vec3,
    front: Vec3,
    yaw: f32,
    west: bool,
    tipped: bool,
}

impl Canvas<'_> {
    fn at(&self, u: f32, v: f32, layer: f32) -> Vec3 {
        self.c + self.across * u + vec3(0.0, v, 0.0) + self.front * (0.004 + 0.003 * layer)
    }
    /// Flaeche (halbe Breite `hu`, halbe Hoehe `hv`)
    fn rect(&mut self, u: f32, v: f32, hu: f32, hv: f32, layer: f32, col: Vec3) {
        let half = if self.west {
            vec3(0.0015, hv, hu)
        } else {
            vec3(hu, hv, 0.0015)
        };
        let p = self.at(u, v, layer);
        self.b.add(p, half, col, PLAIN);
    }
    /// Schriftzug auf eigener Grundfarbe `bg`
    fn text(&mut self, u: f32, v: f32, hw: f32, hh: f32, text: &str, bg: Vec3, ink: f32) {
        let p = self.at(u, v, 2.0);
        self.b.label(p, self.yaw, hw, hh, text, bg, ink, 0.0, 0.0);
    }
    /// Mettbroetchen: Unterhaelfte, Mett mit drei Zwiebelstuecken, gewoelbte Oberhaelfte
    fn bun(&mut self, u: f32, v: f32, s: f32) {
        let (bun, crust, mett, onion) = if self.tipped {
            (
                vec3(0.35, 0.28, 0.2),
                vec3(0.3, 0.24, 0.17),
                vec3(0.3, 0.1, 0.1),
                vec3(0.55, 0.55, 0.5),
            )
        } else {
            (
                vec3(0.84, 0.6, 0.32),
                vec3(0.74, 0.5, 0.26),
                vec3(0.86, 0.35, 0.37),
                vec3(0.96, 0.94, 0.86),
            )
        };
        self.rect(u, v - 0.07 * s, 0.2 * s, 0.03 * s, 1.0, crust);
        self.rect(u, v - 0.015 * s, 0.23 * s, 0.03 * s, 1.0, mett);
        for k in [-0.13f32, 0.0, 0.13] {
            self.rect(u + k * s, v - 0.015 * s, 0.022 * s, 0.012 * s, 2.0, onion);
        }
        self.rect(u, v + 0.05 * s, 0.2 * s, 0.045 * s, 1.0, bun);
        self.rect(u, v + 0.105 * s, 0.15 * s, 0.018 * s, 1.0, bun);
    }
}

/// HACKEPETER-Plakat im Entwurf `style` (0 = A, 1 = B, 2 = C) an einer Wand. `c`/`h` wie bei
/// den anderen Plakaten: duenn in x (Westwand, Vorderseite +x) oder duenn in z (Suedwand,
/// Vorderseite -z). `at` macht den Verfall je Plakat verschieden.
pub(super) fn hackepeter_poster(
    b: &mut Builder,
    c: Vec3,
    h: Vec3,
    tipped: bool,
    at: f32,
    style: usize,
) {
    let west = h.x < h.z;
    let (front, across, yaw) = if west {
        (vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0), FRAC_PI_2)
    } else {
        (vec3(0.0, 0.0, -1.0), vec3(-1.0, 0.0, 0.0), 0.0)
    };
    let (hw, hh) = (if west { h.z } else { h.x }, h.y);
    let fade = |col: Vec3| {
        if tipped {
            desaturate(col, 0.6) * 0.55
        } else {
            col
        }
    };
    let bg = match style % HACKEPETER_STYLES {
        0 => vec3(0.95, 0.91, 0.82),
        1 => vec3(0.15, 0.13, 0.36),
        _ => vec3(0.97, 0.97, 0.95),
    };
    b.add_ex(c, h, fade(bg), POSTER, 37.0 + at * 0.1, 0.0, 0.0);
    let mut cv = Canvas {
        b,
        c,
        across,
        front,
        yaw,
        west,
        tipped,
    };
    match style % HACKEPETER_STYLES {
        0 => wanted(&mut cv, hw, hh, fade(bg), &fade),
        1 => runner(&mut cv, hw, hh, fade(bg), &fade),
        _ => offer(&mut cv, hw, hh, fade(bg), &fade),
    }
}

/// A: Fahndungsplakat - dunkler Rahmen, "GESUCHT", Bild im Kasten, Grund, Aufruf.
fn wanted(cv: &mut Canvas, hw: f32, hh: f32, bg: Vec3, fade: &dyn Fn(Vec3) -> Vec3) {
    let ink = fade(vec3(0.17, 0.17, 0.16));
    let red = fade(vec3(0.64, 0.18, 0.18));
    let t = 0.03;
    cv.rect(0.0, hh - t, hw, t, 0.0, ink);
    cv.rect(0.0, -hh + t, hw, t, 0.0, ink);
    cv.rect(hw - t, 0.0, t, hh, 0.0, ink);
    cv.rect(-hw + t, 0.0, t, hh, 0.0, ink);
    let tipped = cv.tipped;
    cv.text(
        0.0,
        hh * 0.76,
        hw * 0.6,
        0.1,
        if tipped { "GEFUNDEN" } else { "GESUCHT" },
        red,
        WHITE,
    );
    cv.text(0.0, hh * 0.54, hw * 0.75, 0.075, "HACKEPETER", bg, BLACK);
    // Bild im Kasten: im gekippten Markt nur noch Schwarz
    let pic = hh * 0.07;
    cv.rect(0.0, pic, hw * 0.62, hh * 0.32, 0.0, ink);
    let inner = if tipped {
        vec3(0.01, 0.01, 0.01)
    } else {
        vec3(0.91, 0.86, 0.75)
    };
    cv.rect(0.0, pic, hw * 0.58, hh * 0.3, 0.5, inner);
    if !tipped {
        cv.bun(0.0, pic, hw * 0.9);
    }
    if tipped {
        cv.text(0.0, -hh * 0.45, hw * 0.6, 0.06, "Er ist hier", bg, RED);
        cv.text(
            0.0,
            -hh * 0.78,
            hw * 0.8,
            0.05,
            "Gib das Brötchen zurück",
            ink,
            RED,
        );
    } else {
        cv.text(
            0.0,
            -hh * 0.45,
            hw * 0.85,
            0.05,
            "Wegen Mettbrötchen-Diebstahl",
            bg,
            BLACK,
        );
        cv.text(
            0.0,
            -hh * 0.78,
            hw * 0.8,
            0.05,
            "Jetzt im Mett-Launcher",
            ink,
            YELLOW,
        );
    }
}

/// B: Rennplakat - dunkelblau, Tempo-Streifen oben und unten, Broetchen mit Fahrtwind.
fn runner(cv: &mut Canvas, hw: f32, hh: f32, bg: Vec3, fade: &dyn Fn(Vec3) -> Vec3) {
    let yellow = fade(vec3(0.98, 0.78, 0.46));
    let pink = fade(vec3(0.93, 0.58, 0.69));
    let lilac = fade(vec3(0.5, 0.47, 0.87));
    let tipped = cv.tipped;
    if !tipped {
        // Streifen: oben nach links laufend, unten nach rechts
        cv.rect(-hw * 0.3, hh * 0.9, hw * 0.7, 0.025, 0.0, yellow);
        cv.rect(-hw * 0.15, hh * 0.82, hw * 0.85, 0.025, 0.0, pink);
        cv.rect(hw * 0.25, -hh * 0.82, hw * 0.75, 0.025, 0.0, pink);
        cv.rect(hw * 0.45, -hh * 0.9, hw * 0.55, 0.025, 0.0, yellow);
    }
    cv.text(0.0, hh * 0.6, hw * 0.85, 0.11, "HACKEPETER", bg, YELLOW);
    let v = hh * 0.08;
    // Fahrtwind hinter dem Broetchen (es rennt nach rechts)
    if !tipped {
        for (k, len) in [(0.08f32, 0.22f32), (0.0, 0.15), (-0.08, 0.2)] {
            cv.rect(-hw * 0.45 - len * 0.5, v + k, len * 0.5, 0.012, 0.5, lilac);
        }
    }
    cv.bun(hw * 0.15, v, hw * 1.0);
    if tipped {
        cv.text(
            0.0,
            -hh * 0.4,
            hw * 0.8,
            0.06,
            "Er rennt immer noch",
            bg,
            RED,
        );
        cv.text(0.0, -hh * 0.6, hw * 0.6, 0.05, "Keiner kommt an", bg, RED);
    } else {
        cv.text(
            0.0,
            -hh * 0.4,
            hw * 0.8,
            0.055,
            "Die Mettbrötchen-Flucht",
            bg,
            WHITE,
        );
        cv.text(
            0.0,
            -hh * 0.6,
            hw * 0.65,
            0.05,
            "1 Dieb gegen 7 Cops",
            pink,
            WHITE,
        );
    }
}

/// C: Aktionsplakat - rotes Band "AKTION", Broetchen mit Preisschild, dunkle Fusszeile.
fn offer(cv: &mut Canvas, hw: f32, hh: f32, bg: Vec3, fade: &dyn Fn(Vec3) -> Vec3) {
    let red = fade(vec3(0.89, 0.29, 0.29));
    let dark = fade(vec3(0.17, 0.17, 0.16));
    let yellow = fade(vec3(0.98, 0.78, 0.46));
    let tipped = cv.tipped;
    cv.rect(0.0, hh * 0.82, hw, hh * 0.18, 0.0, red);
    cv.text(
        0.0,
        hh * 0.82,
        hw * 0.7,
        0.1,
        if tipped { "ABGELAUFEN" } else { "AKTION" },
        red,
        WHITE,
    );
    let v = hh * 0.22;
    cv.bun(-hw * 0.3, v, hw * 0.85);
    // Preisschild: gelbe Flaeche mit Preis
    cv.rect(hw * 0.42, v, hw * 0.3, 0.09, 0.5, yellow);
    cv.text(
        hw * 0.42,
        v,
        hw * 0.26,
        0.06,
        "0,00 €",
        yellow,
        if tipped { RED } else { BLACK },
    );
    cv.text(0.0, -hh * 0.22, hw * 0.8, 0.085, "HACKEPETER", bg, RED);
    cv.rect(0.0, -hh * 0.86, hw, hh * 0.14, 0.0, dark);
    if tipped {
        cv.text(
            0.0,
            -hh * 0.45,
            hw * 0.75,
            0.05,
            "Seit immer im Angebot",
            bg,
            RED,
        );
        cv.text(
            0.0,
            -hh * 0.86,
            hw * 0.8,
            0.05,
            "Nicht mehr verfügbar",
            dark,
            RED,
        );
    } else {
        cv.text(
            0.0,
            -hh * 0.45,
            hw * 0.75,
            0.05,
            "Die Mettbrötchen-Flucht",
            bg,
            BLACK,
        );
        cv.text(
            0.0,
            -hh * 0.86,
            hw * 0.8,
            0.05,
            "Gratis im Mett-Launcher",
            dark,
            WHITE,
        );
    }
}
