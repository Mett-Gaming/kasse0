//! Aufklaerung ueber Cannabis-Sucht am Beispiel von Leon (erfundene Figur), mit einem
//! erfundenen Sucht-Telefon. Im gekippten Markt wird aus der Aufklaerung etwas anderes.

use super::*;

/// Erfundene Nummer (keine echte Rufnummer)
pub const PHONE: &str = "Sucht-Telefon: 000 / 000 000";

/// Alle Schriftzuege dieser Plakate (fuer den Schrift-Atlas)
pub const TEXTS: &[&str] = &[
    PHONE,
    "Das ist Leon.",
    "Leon kifft jeden Tag.",
    "Leon kifft seit immer.",
    "Leon ist noch hier.",
    "Leon braucht Hilfe.",
    "Leon schläft nicht mehr.",
    "Er steht hinter dir.",
    "Anonym und kostenlos",
    "Niemand geht ran",
    "Die Nummer ruft dich an",
    "Cannabis macht süchtig.",
    "Leon dachte, er hat es im Griff.",
    "Leon hat es nicht im Griff.",
    "Es hat Leon im Griff.",
    "Wir hören zu.",
    "Niemand hört zu.",
    "Wir hören dich atmen.",
    "Leon vor 1 Jahr",
    "Leon heute",
    "Leon heute (wach)",
    "Leon heute: ?",
    "Kiffen verändert dich.",
    "Kiffen verändert alles.",
    "Leon ist nicht mehr Leon.",
    "Du bist nicht allein.",
    "Du bist nie allein.",
    "AUFKLÄRUNG",
];

/// A: Portraet - "Das ist Leon.", Leon im Kapuzenpulli, Sucht-Telefon.
pub fn leon_portrait(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.82, 0.88, 0.94));
    let band = cv.fade(vec3(0.1, 0.2, 0.42));
    let skin = cv.fade(vec3(0.9, 0.74, 0.6));
    let hoodie = cv.fade(vec3(0.25, 0.45, 0.3));
    cv.board(bg);
    cv.text(0.0, 0.58, 0.3, 0.035, "AUFKLÄRUNG", bg, BLUE);
    cv.text(0.0, 0.47, 0.42, 0.07, "Das ist Leon.", bg, BLUE);
    cv.bust(0.0, 0.16, 1.6, skin, hoodie);
    let a = cv.pick(
        "Leon kifft jeden Tag.",
        "Leon kifft seit immer.",
        "Leon ist noch hier.",
    );
    let b = cv.pick(
        "Leon braucht Hilfe.",
        "Leon schläft nicht mehr.",
        "Er steht hinter dir.",
    );
    let ink = if cv.level >= 3 { RED } else { BLACK };
    cv.text(0.0, -0.21, 0.44, 0.05, a, bg, ink);
    cv.text(0.0, -0.33, 0.44, 0.045, b, bg, ink);
    cv.rect(0.0, -0.53, 0.5, 0.08, 1.0, band);
    cv.text(0.0, -0.53, 0.45, 0.045, PHONE, band, WHITE);
    let foot = cv.pick(
        "Anonym und kostenlos",
        "Niemand geht ran",
        "Die Nummer ruft dich an",
    );
    cv.text(0.0, -0.65, 0.35, 0.035, foot, bg, BLUE);
}

/// B: Verbotsschild - stilisierte Pflanze, roter Rahmen mit Balken, Gespraechsangebot.
pub fn leon_leaf(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.9, 0.95, 0.88));
    let green = cv.fade(vec3(0.2, 0.55, 0.25));
    let red = cv.fade(vec3(0.82, 0.1, 0.1));
    let band = cv.fade(vec3(0.1, 0.35, 0.18));
    cv.board(bg);
    cv.text(0.0, 0.56, 0.46, 0.06, "Cannabis macht süchtig.", bg, BLACK);
    // Pflanze: Mittelblatt, zwei Seitenblaetter, Stiel
    cv.rect(0.0, 0.14, 0.03, 0.17, 1.5, green);
    for k in [-1.0f32, 1.0] {
        cv.rect(k * 0.08, 0.1, 0.025, 0.11, 1.5, green);
        cv.rect(k * 0.15, 0.05, 0.02, 0.07, 1.5, green);
    }
    cv.rect(0.0, -0.08, 0.008, 0.06, 1.5, green);
    // roter Rahmen mit Querbalken
    let (r, t) = (0.27, 0.02);
    cv.rect(0.0, 0.08 + r, r, t, 2.0, red);
    cv.rect(0.0, 0.08 - r, r, t, 2.0, red);
    cv.rect(r, 0.08, t, r, 2.0, red);
    cv.rect(-r, 0.08, t, r, 2.0, red);
    cv.rect(0.0, 0.08, r, t, 2.5, red);
    let line = cv.pick(
        "Leon dachte, er hat es im Griff.",
        "Leon hat es nicht im Griff.",
        "Es hat Leon im Griff.",
    );
    cv.text(0.0, -0.3, 0.46, 0.045, line, bg, BLACK);
    cv.rect(0.0, -0.5, 0.5, 0.08, 1.0, band);
    cv.text(0.0, -0.5, 0.45, 0.045, PHONE, band, WHITE);
    let foot = cv.pick("Wir hören zu.", "Niemand hört zu.", "Wir hören dich atmen.");
    let ink = if cv.level >= 2 { RED } else { BLACK };
    cv.text(0.0, -0.63, 0.3, 0.04, foot, bg, ink);
}

/// C: Vorher/Nachher - Leon vor einem Jahr und heute (blass, spaeter nur noch schwarz).
pub fn leon_before_after(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.88, 0.88, 0.86));
    let band = cv.fade(vec3(0.12, 0.12, 0.14));
    let skin = cv.fade(vec3(0.9, 0.74, 0.6));
    let pale = cv.fade(vec3(0.75, 0.75, 0.7));
    cv.board(bg);
    cv.bust(-0.24, 0.24, 1.0, skin, cv.fade(vec3(0.2, 0.4, 0.65)));
    if cv.level >= 3 {
        // heute: nur noch ein schwarzer Umriss
        let black = vec3(0.01, 0.01, 0.01);
        cv.rect(0.24, 0.08, 0.2, 0.08, 1.0, black);
        cv.rect(0.24, 0.3, 0.09, 0.11, 1.0, black);
    } else {
        cv.bust(0.24, 0.24, 1.0, pale, cv.fade(vec3(0.4, 0.4, 0.42)));
    }
    cv.text(-0.24, -0.02, 0.22, 0.035, "Leon vor 1 Jahr", bg, BLACK);
    let today = cv.pick("Leon heute", "Leon heute (wach)", "Leon heute: ?");
    cv.text(0.24, -0.02, 0.22, 0.035, today, bg, BLACK);
    let line = cv.pick(
        "Kiffen verändert dich.",
        "Kiffen verändert alles.",
        "Leon ist nicht mehr Leon.",
    );
    let ink = if cv.level >= 3 { RED } else { BLACK };
    cv.text(0.0, -0.25, 0.46, 0.055, line, bg, ink);
    cv.rect(0.0, -0.5, 0.5, 0.08, 1.0, band);
    cv.text(0.0, -0.5, 0.45, 0.045, PHONE, band, WHITE);
    let foot = cv.pick(
        "Du bist nicht allein.",
        "Du bist nicht allein.",
        "Du bist nie allein.",
    );
    cv.text(0.0, -0.63, 0.3, 0.04, foot, bg, BLACK);
}
