//! Werbung fuer HACKEPETER, das andere Spiel im Mett-Launcher (Mett-Gaming/hackepeter).
//! Drei Plakate im Wechsel, gebaut aus Flaechen, Boxen und Schriftzuegen:
//! A Fahndungsplakat ("GESUCHT"), B Rennplakat (Tempo-Streifen), C Aktionsplakat ("0,00 €").
//! Eigene Flaeche (nicht das Plakat-Material: das malt das Maskottchen-Gesicht darauf).
//! Im gekippten Markt verblasst und mit unheimlichem Text.

use super::canvas::*;
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

/// HACKEPETER-Plakat im Entwurf `style` (0 = A, 1 = B, 2 = C) an einer Wand. `c` = Mitte der
/// Plakatflaeche, `front` = Richtung aus der Wand heraus (+-x oder +-z).
pub(super) fn hackepeter_poster(b: &mut Builder, c: Vec3, front: Vec3, level: u8, style: usize) {
    let tipped = level > 0;
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
    let mut cv = Canvas::new(b, c, front, level);
    cv.board(fade(bg));
    let (hw, hh) = (cv.hw, cv.hh);
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
    let tipped = cv.tipped();
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
    let tipped = cv.tipped();
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
    let tipped = cv.tipped();
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

/// So viele HACKEPETER-Plakate werden aufgehaengt (wenn genug Platz ist)
pub const HACKEPETER_COUNT: usize = 8;
/// So viel Platz bleibt um jedes Plakat frei (Meter)
const MARGIN: f32 = 0.25;
/// So tief darf vor dem Plakat nichts stehen (Meter)
const CLEAR_DEPTH: f32 = 0.3;
/// Haengeschilder (Abteilungen) duerfen bis so weit vor der Wand nicht vor dem Plakat haengen
const SIGN_DEPTH: f32 = 1.2;
/// Hoehe der Plakatmitte: in Augenhoehe bzw. hoch ueber Regalen an der Wand (Meter)
const LOW: (f32, f32) = (1.55, 1.95);
const HIGH: (f32, f32) = (2.4, 2.65);
/// So viele Plakate kommen an die Wand der Obstabteilung
pub const PRODUCE_COUNT: usize = 2;

/// Achsparalleler Umriss einer Instanz (gedrehte grob ueber die groessere Halbachse).
fn instance_box(i: &GpuInstance) -> (Vec3, Vec3) {
    let c = vec3(i.center[0], i.center[1], i.center[2]);
    let h = vec3(i.half[0], i.half[1], i.half[2]);
    if i.center[3].abs() > 1e-4 {
        let r = h.x.max(h.z);
        (c, vec3(r, h.y, r))
    } else {
        (c, h)
    }
}

impl Gen<'_> {
    /// HACKEPETER-Plakate an zufaelligen freien Stellen aller Aussenwaende, in ungleichen
    /// Abstaenden und leicht verschiedener Hoehe. Muss nach allem anderen laufen: geprueft
    /// wird, dass hinter dem Plakat Wand ist (keine Tuer, kein Fenster, keine Oeffnung) und
    /// davor nichts haengt oder steht (Regale, Schilder, andere Plakate). Fester Zufall je
    /// Markt (unabhaengig vom Umbau-Zufall).
    pub(super) fn place_hackepeter_posters(
        &mut self,
        width: f32,
        depth: f32,
        produce: Option<(f32, f32)>,
    ) {
        let mut rng = Pcg64::seed_from_u64(self.params.seed ^ 0x4AC3_E9E7_E200);
        // Waende: Punkt am Anfang, Richtung entlang, Laenge, Richtung in den Markt
        let walls = [
            (Vec3::ZERO, vec3(0.0, 0.0, 1.0), depth, vec3(1.0, 0.0, 0.0)),
            (
                vec3(width, 0.0, 0.0),
                vec3(0.0, 0.0, 1.0),
                depth,
                vec3(-1.0, 0.0, 0.0),
            ),
            (Vec3::ZERO, vec3(1.0, 0.0, 0.0), width, vec3(0.0, 0.0, 1.0)),
            (
                vec3(0.0, 0.0, depth),
                vec3(1.0, 0.0, 0.0),
                width,
                vec3(0.0, 0.0, -1.0),
            ),
        ];
        let total: f32 = walls.iter().map(|w| w.2).sum();
        let mut style = self.params.seed as usize % HACKEPETER_STYLES;
        let mut placed = 0;
        // erst die Wand der Obstabteilung (Ostwand hinter dem Stufenregal): dort hoch ueber
        // dem Regal, bis zu PRODUCE_COUNT Plakate
        if let Some((z0, z1)) = produce {
            let mut here = 0;
            for _ in 0..80 {
                if here >= PRODUCE_COUNT || z1 - z0 < 2.0 * POSTER_HALF.0 + 1.0 {
                    break;
                }
                let z = rng.gen_range(z0 + POSTER_HALF.0..z1 - POSTER_HALF.0);
                let y = rng.gen_range(HIGH.0..HIGH.1);
                let (_, along, _, front) = walls[1];
                if self.try_poster(vec3(width, y, z), along, front, style) {
                    style += 1;
                    placed += 1;
                    here += 1;
                }
            }
        }
        for _ in 0..600 {
            if placed >= HACKEPETER_COUNT {
                break;
            }
            // Wand nach Laenge gewichtet, Stelle und Hoehe zufaellig
            let mut pick = rng.gen_range(0.0..total);
            let wall = walls
                .iter()
                .find(|w| {
                    pick -= w.2;
                    pick < 0.0
                })
                .unwrap_or(&walls[0]);
            let (start, along, len, front) = *wall;
            let (hw, _) = POSTER_HALF;
            if len < 2.0 * (hw + MARGIN) + 1.0 {
                continue;
            }
            let s = rng.gen_range(hw + MARGIN + 0.5..len - hw - MARGIN - 0.5);
            // in Augenhoehe; ist dort etwas im Weg (Regal an der Wand), hoeher darueber
            let low = rng.gen_range(LOW.0..LOW.1);
            let high = rng.gen_range(HIGH.0..HIGH.1);
            let at = |y: f32| start + along * s + vec3(0.0, y, 0.0);
            if self.try_poster(at(low), along, front, style)
                || self.try_poster(at(high), along, front, style)
            {
                style += 1;
                placed += 1;
            }
        }
    }

    /// Plakat an Wandpunkt `p` (auf der Wand, Mitte des Plakats) aufhaengen, wenn Wand dahinter
    /// und davor frei ist.
    fn try_poster(&mut self, p: Vec3, along: Vec3, front: Vec3, style: usize) -> bool {
        let (hw, hh) = POSTER_HALF;
        let c = p + front * POSTER_OUT;
        if !self.wall_behind(c, along, front, hw, hh) || !self.free_in_front(c, along, front) {
            return false;
        }
        hackepeter_poster(
            &mut self.b,
            c,
            front,
            super::sortiment::level_of(self.params),
            style,
        );
        true
    }

    /// Ist hinter der ganzen Plakatflaeche Wand (fester Koerper)?
    fn wall_behind(&self, c: Vec3, along: Vec3, front: Vec3, hw: f32, hh: f32) -> bool {
        let solid_at = |p: Vec3| {
            self.b.solids.iter().any(|s| {
                p.x >= s.min.x
                    && p.x <= s.max.x
                    && p.y >= s.min.y
                    && p.y <= s.max.y
                    && p.z >= s.min.z
                    && p.z <= s.max.z
            })
        };
        let back = c - front * 0.06;
        [
            (-1.0f32, -1.0f32),
            (1.0, -1.0),
            (-1.0, 1.0),
            (1.0, 1.0),
            (0.0, 0.0),
        ]
        .iter()
        .all(|(a, b)| solid_at(back + along * (a * hw) + vec3(0.0, b * hh, 0.0)))
    }

    /// Steht oder haengt vor der Plakatflaeche (mit Rand) nichts?
    fn free_in_front(&self, c: Vec3, along: Vec3, front: Vec3) -> bool {
        let (hw, hh) = POSTER_HALF;
        // von knapp vor der Wand (damit auch flache Plakate und Schilder zaehlen) bis 0,3 m davor
        let region = |depth: f32| {
            let mid = c + front * (depth * 0.5 - 0.01);
            let half = along.abs() * (hw + MARGIN)
                + vec3(0.0, hh + MARGIN, 0.0)
                + front.abs() * (depth * 0.5);
            (mid, half)
        };
        let hits = |i: &GpuInstance, (mid, half): (Vec3, Vec3)| {
            let (ic, ih) = instance_box(i);
            let d = (ic - mid).abs();
            d.x < ih.x + half.x && d.y < ih.y + half.y && d.z < ih.z + half.z
        };
        // Haengeschilder samt Stange verdecken das Plakat auch aus 1 m Abstand
        let hanging = |i: &GpuInstance| {
            i.extra[0] == SIGN || (i.half[0] < 0.02 && i.half[2] < 0.02 && i.center[1] > 2.5)
        };
        // flache Zierleisten direkt auf der Wand (z. B. an der Obstwand) darf das Plakat
        // ueberdecken - nicht aber Schilder, Smiley-Plakate oder Abstehendes
        let wall = c - front * POSTER_OUT;
        let flat_decor = |i: &GpuInstance| {
            let (ic, ih) = instance_box(i);
            i.extra[0] == PLAIN && ih.dot(front.abs()) <= 0.012 && (ic - wall).dot(front) <= 0.02
        };
        let (near, far) = (region(CLEAR_DEPTH), region(SIGN_DEPTH));
        self.b
            .instances
            .iter()
            .all(|i| (flat_decor(i) || !hits(i, near)) && !(hanging(i) && hits(i, far)))
    }
}
