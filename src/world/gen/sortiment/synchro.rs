//! Werbung fuer die Fandubs der Gruppe Synchro (Satire-Synchros von Anime):
//! STEIN-HART (Zeitreise-Satire: Mikrowelle, Laborkittel, Uhr) und OVERCOCK (Satire auf einen
//! untoten Herrscher: Skelettkoenig, Gilde, Vorschau). Nur die Parodie-Titel, keine echten
//! Serientitel oder Figuren.

use super::*;

/// Alle Schriftzuege dieser Plakate (fuer den Schrift-Atlas)
pub const TEXTS: &[&str] = &[
    "Synchro",
    "STEIN-HART",
    "OVERCOCK",
    "Ein Synchro-Fandub",
    "Synchro · Fandub-Satire",
    "Zeitreise per Mikrowelle",
    "Die Mikrowelle läuft noch",
    "Du bist in der falschen Zeitlinie",
    "Folge 1 jetzt online",
    "Folge 1 läuft. Seit immer.",
    "Es gibt keine Folge 2",
    "Alles nach Plan.",
    "Das war nicht der Plan.",
    "Es gibt keinen Plan.",
    "Neue Folge jeden Freitag",
    "Jeden Freitag. Für immer.",
    "Diese Folge endet nie.",
    "Staffel 1 jetzt online",
    "Er regiert diesen Markt",
    "Knie nieder, Kunde 0",
    "Die Gilde sucht Mitglieder",
    "Die Gilde hat dich gefunden",
    "Du bist jetzt Untertan",
    "Nächste Folge:",
    "Der Hahn erwacht",
    "Der Hahn ist erwacht",
    "Er kräht nach dir",
];

/// Kopfzeile aller Synchro-Plakate: Gruppenname klein, Titel gross.
fn head(cv: &mut Canvas, title: &str, bg: Vec3, ink: f32) {
    cv.text(0.0, 0.6, 0.22, 0.035, "Synchro", bg, WHITE);
    cv.text(0.0, 0.48, 0.46, 0.1, title, bg, ink);
}

/// STEIN-HART A: Mikrowelle als Zeitmaschine, gruen-schwarz wie ein Laborbildschirm.
pub fn stein_microwave(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.05, 0.12, 0.08));
    let green = cv.fade(vec3(0.3, 0.85, 0.45));
    let metal = cv.fade(vec3(0.7, 0.7, 0.72));
    cv.board(bg);
    head(cv, "STEIN-HART", bg, YELLOW);
    cv.text(0.0, 0.34, 0.3, 0.04, "Ein Synchro-Fandub", bg, WHITE);
    // Mikrowelle: Gehaeuse, dunkles Fenster (spaeter gluehend rot), Bedienfeld, Griff
    cv.rect(0.0, 0.03, 0.27, 0.17, 1.0, metal);
    let inside = match cv.level {
        0 | 1 => vec3(0.08, 0.1, 0.1),
        2 => vec3(0.35, 0.15, 0.05),
        _ => vec3(0.75, 0.08, 0.05),
    };
    cv.rect(-0.05, 0.03, 0.17, 0.12, 1.5, inside);
    cv.rect(0.19, 0.06, 0.05, 0.08, 1.5, cv.fade(vec3(0.2, 0.22, 0.22)));
    for v in [0.11f32, 0.06, 0.01] {
        cv.rect(0.19, v, 0.03, 0.012, 2.0, green);
    }
    cv.rect(0.12, 0.03, 0.01, 0.1, 2.0, cv.fade(vec3(0.3, 0.3, 0.3)));
    let line = cv.pick(
        "Zeitreise per Mikrowelle",
        "Die Mikrowelle läuft noch",
        "Du bist in der falschen Zeitlinie",
    );
    cv.text(0.0, -0.3, 0.46, 0.05, line, bg, WHITE);
    cv.rect(0.0, -0.55, 0.5, 0.08, 1.0, green);
    let foot = cv.pick(
        "Folge 1 jetzt online",
        "Folge 1 läuft. Seit immer.",
        "Es gibt keine Folge 2",
    );
    cv.text(0.0, -0.55, 0.44, 0.045, foot, green, BLACK);
}

/// STEIN-HART B: Der Laborkittel - Portraet im weissen Kittel mit wildem Haar, Sprechblase.
pub fn stein_labcoat(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.86, 0.84, 0.78));
    let dark = cv.fade(vec3(0.12, 0.12, 0.14));
    let skin = cv.fade(vec3(0.92, 0.78, 0.66));
    let coat = cv.fade(vec3(0.97, 0.97, 0.97));
    let hair = cv.fade(vec3(0.15, 0.1, 0.08));
    cv.board(bg);
    cv.rect(0.0, 0.53, 0.5, 0.17, 1.0, dark);
    head(cv, "STEIN-HART", dark, YELLOW);
    cv.bust(-0.18, 0.02, 1.5, skin, coat);
    // wildes Haar ueber dem Kopf
    for (u, h) in [
        (-0.27f32, 0.04f32),
        (-0.2, 0.06),
        (-0.13, 0.04),
        (-0.09, 0.03),
    ] {
        cv.rect(u, 0.22 + h * 0.5, 0.03, h, 2.0, hair);
    }
    cv.rect(0.24, 0.15, 0.24, 0.09, 1.0, coat);
    let quote = cv.pick(
        "Alles nach Plan.",
        "Das war nicht der Plan.",
        "Es gibt keinen Plan.",
    );
    cv.text(0.24, 0.15, 0.21, 0.045, quote, coat, BLACK);
    cv.text(0.0, -0.55, 0.42, 0.04, "Synchro · Fandub-Satire", bg, BLACK);
}

/// STEIN-HART C: Die Uhr - Zifferblatt mit Zeigern, die sich spaeter rueckwaerts drehen.
pub fn stein_clock(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.12, 0.14, 0.3));
    let face = cv.fade(vec3(0.95, 0.93, 0.85));
    let ink = cv.fade(vec3(0.1, 0.1, 0.12));
    let gold = cv.fade(vec3(0.9, 0.72, 0.3));
    cv.board(bg);
    head(cv, "STEIN-HART", bg, YELLOW);
    // Zifferblatt mit Rand und Stundenmarken, Zeiger je Stufe anders
    cv.rect(0.0, 0.05, 0.22, 0.22, 1.0, gold);
    cv.rect(0.0, 0.05, 0.19, 0.19, 1.5, face);
    for (u, v) in [(0.0f32, 0.2f32), (0.15, 0.05), (0.0, -0.1), (-0.15, 0.05)] {
        cv.rect(u, v, 0.012, 0.012, 2.0, ink);
    }
    match cv.level {
        0 | 1 => {
            cv.rect(0.0, 0.11, 0.008, 0.06, 2.5, ink);
            cv.rect(0.05, 0.05, 0.05, 0.008, 2.5, ink);
        }
        2 => {
            cv.rect(0.0, -0.01, 0.008, 0.06, 2.5, ink);
            cv.rect(-0.05, 0.05, 0.05, 0.008, 2.5, ink);
        }
        _ => {
            // keine Zeiger mehr, nur ein rotes Auge in der Mitte
            cv.rect(0.0, 0.05, 0.03, 0.02, 2.5, vec3(0.7, 0.03, 0.03));
        }
    }
    let line = cv.pick(
        "Neue Folge jeden Freitag",
        "Jeden Freitag. Für immer.",
        "Diese Folge endet nie.",
    );
    cv.text(0.0, -0.32, 0.46, 0.05, line, bg, WHITE);
    cv.text(0.0, -0.55, 0.38, 0.04, "Ein Synchro-Fandub", bg, YELLOW);
}

/// OVERCOCK A: Der Skelettkoenig - Schaedel mit Krone und roten Augen, dunkle Robe.
pub fn cock_king(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.16, 0.06, 0.22));
    let bone = cv.fade(vec3(0.92, 0.9, 0.82));
    let gold = cv.fade(vec3(0.9, 0.72, 0.25));
    let robe = cv.fade(vec3(0.06, 0.04, 0.08));
    let red = vec3(0.9, 0.05, 0.05);
    cv.board(bg);
    head(cv, "OVERCOCK", bg, YELLOW);
    cv.text(0.0, 0.36, 0.3, 0.04, "Ein Synchro-Fandub", bg, WHITE);
    // Robe mit goldenem Saum, Schaedel, Augenhoehlen mit roten Punkten, Zaehne, Krone
    cv.rect(0.0, -0.16, 0.3, 0.1, 1.0, robe);
    cv.rect(0.0, -0.07, 0.3, 0.012, 1.5, gold);
    cv.rect(0.0, 0.07, 0.11, 0.12, 1.0, bone);
    let glow = if cv.level >= 3 { 0.025 } else { 0.012 };
    for k in [-1.0f32, 1.0] {
        cv.rect(k * 0.045, 0.1, 0.03, 0.03, 1.5, vec3(0.02, 0.02, 0.02));
        cv.rect(k * 0.045, 0.1, glow, glow, 2.0, red);
    }
    cv.rect(0.0, 0.0, 0.06, 0.012, 1.5, vec3(0.1, 0.1, 0.1));
    for (u, h) in [(-0.08f32, 0.03f32), (0.0, 0.05), (0.08, 0.03)] {
        cv.rect(u, 0.2 + h, 0.025, h, 1.5, gold);
    }
    let line = cv.pick(
        "Staffel 1 jetzt online",
        "Er regiert diesen Markt",
        "Knie nieder, Kunde 0",
    );
    let ink = if cv.level >= 2 { RED } else { WHITE };
    cv.text(0.0, -0.38, 0.46, 0.05, line, bg, ink);
    cv.text(
        0.0,
        -0.57,
        0.4,
        0.035,
        "Synchro · Fandub-Satire",
        bg,
        YELLOW,
    );
}

/// OVERCOCK B: Die Gilde - schwarz mit goldenem Wappen (Kreuz und Ring), Aufruf.
pub fn cock_guild(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.05, 0.05, 0.06));
    let gold = cv.fade(vec3(0.85, 0.68, 0.25));
    let band = cv.fade(vec3(0.45, 0.06, 0.08));
    cv.board(bg);
    head(cv, "OVERCOCK", bg, YELLOW);
    // Wappen: Ring aus vier Balken, Kreuz in der Mitte
    let (r, t) = (0.2, 0.018);
    cv.rect(0.0, 0.08 + r, r, t, 1.0, gold);
    cv.rect(0.0, 0.08 - r, r, t, 1.0, gold);
    cv.rect(r, 0.08, t, r, 1.0, gold);
    cv.rect(-r, 0.08, t, r, 1.0, gold);
    cv.rect(0.0, 0.08, 0.02, 0.15, 1.5, gold);
    cv.rect(0.0, 0.12, 0.1, 0.02, 1.5, gold);
    if cv.level >= 3 {
        // im Wappen oeffnen sich zwei rote Augen
        for k in [-1.0f32, 1.0] {
            cv.rect(k * 0.09, 0.02, 0.02, 0.012, 2.0, vec3(0.85, 0.04, 0.04));
        }
    }
    cv.rect(0.0, -0.34, 0.5, 0.08, 1.0, band);
    let line = cv.pick(
        "Die Gilde sucht Mitglieder",
        "Die Gilde hat dich gefunden",
        "Du bist jetzt Untertan",
    );
    cv.text(0.0, -0.34, 0.44, 0.045, line, band, WHITE);
    cv.text(0.0, -0.55, 0.38, 0.04, "Ein Synchro-Fandub", bg, YELLOW);
}

/// OVERCOCK C: Vorschau - dunkel, nur rote Augen und ein Stab, "Naechste Folge".
pub fn cock_preview(cv: &mut Canvas) {
    let bg = cv.fade(vec3(0.08, 0.02, 0.03));
    let staff = cv.fade(vec3(0.6, 0.5, 0.3));
    let gem = cv.fade(vec3(0.3, 0.2, 0.8));
    let red = vec3(0.9, 0.05, 0.05);
    cv.board(bg);
    head(cv, "OVERCOCK", bg, RED);
    // Stab mit Kugel links, Augen im Dunkeln (spaeter mehr Augenpaare)
    cv.rect(-0.25, 0.0, 0.012, 0.26, 1.0, staff);
    cv.rect(-0.25, 0.28, 0.035, 0.035, 1.5, gem);
    let pairs: &[(f32, f32)] = match cv.level {
        0 | 1 => &[(0.08, 0.1)],
        2 => &[(0.08, 0.1), (0.25, -0.05)],
        _ => &[
            (0.08, 0.1),
            (0.25, -0.05),
            (-0.05, -0.12),
            (0.3, 0.25),
            (0.12, 0.28),
        ],
    };
    for &(u, v) in pairs {
        for k in [-1.0f32, 1.0] {
            cv.rect(u + k * 0.035, v, 0.018, 0.01, 2.0, red);
        }
    }
    cv.text(0.0, -0.3, 0.3, 0.04, "Nächste Folge:", bg, WHITE);
    let line = cv.pick(
        "Der Hahn erwacht",
        "Der Hahn ist erwacht",
        "Er kräht nach dir",
    );
    cv.text(0.0, -0.42, 0.44, 0.06, line, bg, YELLOW);
    cv.text(0.0, -0.6, 0.38, 0.035, "Synchro · Fandub-Satire", bg, WHITE);
}
