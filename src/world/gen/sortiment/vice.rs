//! Gluecksspiel- und Tabakwerbung (Fantasie-Marken): LOTTO NULL, Rubbellos GOLDENE NULL,
//! Zigaretten NEBELBANK. Mit Pflicht-Hinweisen, die im gekippten Markt kippen.

use super::*;

/// Alle Schriftzuege dieser Plakate (fuer den Schrift-Atlas)
pub const TEXTS: &[&str] = &[
    "LOTTO NULL",
    "0",
    "Ihr Glück wartet an Kasse 0",
    "Gewinnzahlen: 0 0 0 0 0 0",
    "Sie haben schon verloren",
    "Glücksspiel kann süchtig machen",
    "Spielen bis zum Schluss",
    "Der Einsatz sind Sie",
    "GOLDENE NULL",
    "Rubbellos",
    "€ € €",
    "€ € ?",
    "DU",
    "Rubbeln & gewinnen!",
    "Rubbeln Sie weiter",
    "Unter dem Feld ist ein Gesicht",
    "Nur 1 €",
    "Nur noch 1 Versuch",
    "Kostet alles",
    "NEBELBANK",
    "Zigaretten",
    "Der Geschmack von Freiheit",
    "Der Geschmack von Asche",
    "Der Rauch hat ein Gesicht",
    "Rauchen kann tödlich sein",
    "Rauchen ist tödlich. Bleiben Sie.",
    "Sie rauchen schon zu lange",
];

/// LOTTO NULL: gelb, rotes Band, sechs Kugeln - alle mit einer Null.
pub fn lotto(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.98, 0.85, 0.2));
    let red = cv.fade(vec3(0.8, 0.12, 0.12));
    let dark = cv.fade(vec3(0.15, 0.13, 0.12));
    let ball = if cv.level >= 3 {
        cv.fade(vec3(0.45, 0.05, 0.05))
    } else {
        cv.fade(vec3(0.98, 0.98, 0.96))
    };
    let ink = if cv.level >= 3 { WHITE } else { BLACK };
    cv.board(bg);
    cv.rect(0.0, 0.52, 0.5, 0.11, 1.0, red);
    cv.text(0.0, 0.52, 0.44, 0.08, "LOTTO NULL", red, WHITE);
    for (k, (u, v)) in [
        (-0.26f32, 0.22f32),
        (0.0, 0.25),
        (0.26, 0.22),
        (-0.2, 0.0),
        (0.06, 0.02),
        (0.3, -0.02),
    ]
    .into_iter()
    .enumerate()
    {
        let v = v + (k as f32 * 1.7).sin() * 0.015;
        cv.rect(u, v, 0.075, 0.075, 1.0, ball);
        cv.text(u, v, 0.05, 0.05, "0", ball, ink);
    }
    let line = cv.pick(
        "Ihr Glück wartet an Kasse 0",
        "Gewinnzahlen: 0 0 0 0 0 0",
        "Sie haben schon verloren",
    );
    cv.text(0.0, -0.28, 0.46, 0.05, line, bg, BLACK);
    cv.rect(0.0, -0.58, 0.5, 0.09, 1.0, dark);
    let foot = cv.pick(
        "Glücksspiel kann süchtig machen",
        "Spielen bis zum Schluss",
        "Der Einsatz sind Sie",
    );
    cv.text(0.0, -0.58, 0.44, 0.04, foot, dark, WHITE);
}

/// Rubbellos GOLDENE NULL: gruen, goldenes Los mit halb freigerubbeltem Feld.
pub fn scratch(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.15, 0.55, 0.3));
    let gold = cv.fade(vec3(0.85, 0.7, 0.25));
    let silver = cv.fade(vec3(0.7, 0.7, 0.72));
    let open = cv.fade(vec3(0.97, 0.96, 0.92));
    let yellow = cv.fade(vec3(0.98, 0.82, 0.2));
    cv.board(bg);
    cv.text(0.0, 0.56, 0.44, 0.08, "GOLDENE NULL", bg, YELLOW);
    cv.text(0.0, 0.42, 0.22, 0.045, "Rubbellos", bg, WHITE);
    cv.rect(0.0, 0.08, 0.32, 0.22, 1.0, gold);
    cv.rect(0.0, 0.05, 0.26, 0.11, 1.5, silver);
    cv.rect(-0.11, 0.05, 0.15, 0.11, 2.0, open);
    let found = cv.pick("€ € €", "€ € ?", "DU");
    let ink = if cv.level >= 3 { RED } else { BLACK };
    cv.text(-0.11, 0.05, 0.13, 0.06, found, open, ink);
    let line = cv.pick(
        "Rubbeln & gewinnen!",
        "Rubbeln Sie weiter",
        "Unter dem Feld ist ein Gesicht",
    );
    cv.text(0.0, -0.3, 0.46, 0.05, line, bg, WHITE);
    cv.rect(0.0, -0.48, 0.3, 0.07, 1.0, yellow);
    let price = cv.pick("Nur 1 €", "Nur noch 1 Versuch", "Kostet alles");
    cv.text(0.0, -0.48, 0.27, 0.045, price, yellow, BLACK);
    cv.text(
        0.0,
        -0.63,
        0.4,
        0.03,
        "Glücksspiel kann süchtig machen",
        bg,
        WHITE,
    );
}

/// Zigaretten NEBELBANK: weiss, rote Packungsspitze, drei Zigaretten, Warnhinweis.
/// Im Kollaps formt der Rauch ein Gesicht.
pub fn tobacco(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.95, 0.95, 0.93));
    let red = cv.fade(vec3(0.75, 0.1, 0.12));
    let paper = cv.fade(vec3(0.99, 0.99, 0.97));
    let filter = cv.fade(vec3(0.85, 0.6, 0.3));
    let smoke = cv.fade(vec3(0.6, 0.6, 0.62));
    cv.board(bg);
    cv.rect(0.0, 0.47, 0.5, 0.23, 1.0, red);
    cv.text(0.0, 0.54, 0.42, 0.08, "NEBELBANK", red, WHITE);
    cv.text(0.0, 0.38, 0.2, 0.04, "Zigaretten", red, WHITE);
    for u in [-0.1f32, 0.0, 0.1] {
        let up = if u == 0.0 { 0.03 } else { 0.0 };
        cv.rect(u, 0.08 + up, 0.025, 0.1, 1.5, paper);
        cv.rect(u, -0.04 + up, 0.025, 0.03, 1.5, filter);
    }
    // Rauch ueber der mittleren Zigarette, im Kollaps mit Augen und Mund
    cv.rect(0.03, 0.27, 0.05, 0.03, 1.5, smoke);
    if cv.level >= 4 {
        cv.rect(0.03, 0.27, 0.11, 0.08, 1.5, smoke);
        for k in [-1.0f32, 1.0] {
            cv.rect(
                0.03 + k * 0.04,
                0.29,
                0.012,
                0.012,
                2.5,
                vec3(0.0, 0.0, 0.0),
            );
        }
        cv.rect(0.03, 0.24, 0.02, 0.012, 2.5, vec3(0.0, 0.0, 0.0));
    }
    let line = cv.pick(
        "Der Geschmack von Freiheit",
        "Der Geschmack von Asche",
        "Der Rauch hat ein Gesicht",
    );
    cv.text(0.0, -0.18, 0.46, 0.05, line, bg, BLACK);
    // Warnhinweis: schwarzer Rahmen, weisse Flaeche
    cv.rect(0.0, -0.48, 0.46, 0.16, 1.0, vec3(0.02, 0.02, 0.02));
    cv.rect(0.0, -0.48, 0.43, 0.13, 1.5, vec3(0.97, 0.97, 0.97));
    let warn = cv.pick(
        "Rauchen kann tödlich sein",
        "Rauchen ist tödlich. Bleiben Sie.",
        "Sie rauchen schon zu lange",
    );
    cv.text(0.0, -0.48, 0.4, 0.05, warn, vec3(0.97, 0.97, 0.97), BLACK);
}
