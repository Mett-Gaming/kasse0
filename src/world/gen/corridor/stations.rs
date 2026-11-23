//! Hindernisse und Ereignisse entlang des Wegs: alle 6-9,5 m eine Station, nie direkt an einer
//! Ecke. Statische Teile (Kisten, Rohre, Rahmen, Engstellen) entstehen hier; was sich bewegt
//! (Tueren, Regale, Wagen, Gestalten, Decke, Waende), steuert das Spiel (game/corridor/).

use super::build::{near_corner, Frame};
use super::*;

/// Abstand zwischen zwei Stationen (Meter)
const GAP: (f32, f32) = (6.0, 9.5);
/// Unterkante der Duck-Huerden, Oberkante der Spring-Huerden
pub const DUCK_BOTTOM: (f32, f32) = (1.05, 1.2);
pub const JUMP_TOP: f32 = 0.45;
/// Tiefe (entlang des Gangs) der Huerden
pub const HURDLE_DEPTH: f32 = 0.35;
/// Engstelle: Breite der Luecke und Laenge
pub const SQUEEZE_GAP: f32 = 0.8;
pub const SQUEEZE_LEN: f32 = 3.0;
/// Halbe Laenge der Stuecke, auf denen sich der Gang veraendert (Decke, Waende; game/corridor)
pub const LONG_HALF: f32 = 3.0;

#[derive(Clone, Copy, PartialEq)]
enum Slot {
    Slalom,
    Duck,
    Jump,
    Door,
    Shelf,
    Cart,
    Squeeze,
}

pub(super) fn place(b: &mut Builder, rng: &mut Pcg64, c: &mut Corridor) {
    let hw = CORRIDOR_HALF_W;
    // Stellen sammeln (nicht an Ecken, nicht im letzten Stueck)
    let mut spots = Vec::new();
    let mut s = rng.gen_range(5.0..7.0);
    while s < c.end_s - hw - 3.0 {
        if near_corner(c, s, hw + 1.0) {
            s += 0.5;
            continue;
        }
        spots.push(s);
        s += rng.gen_range(GAP.0..GAP.1);
    }
    // Ereignisse: 1-3 Gestalten, je Lauf eine Veraenderung des Gangs (Decke oder Waende)
    let n_fig = rng.gen_range(1..=3).min(spots.len());
    for k in 0..n_fig {
        // zwischen die Hindernisse (halber Abstand weiter), damit sie frei stehen
        let s = spots[(k * spots.len()) / n_fig.max(1)] + 3.5;
        if !near_corner(c, s, hw + 1.0) && s < c.end_s - hw - 2.0 {
            push(c, s, StationKind::Figure { variant: rng.gen() });
        }
    }
    // Decke/Waende sind 6 m lang: nur dort, wo sie nicht in ein Eckquadrat ragen
    let long_ok = |s: f32| !near_corner(c, s, hw + LONG_HALF + 0.5);
    let fits: Vec<usize> = (1..spots.len()).filter(|&k| long_ok(spots[k])).collect();
    let change = if fits.is_empty() {
        None
    } else {
        Some(spots.remove(fits[rng.gen_range(0..fits.len())]))
    };
    if let Some(s) = change {
        let kind = if rng.gen_bool(0.5) {
            StationKind::Ceiling
        } else {
            StationKind::Walls
        };
        push(c, s, kind);
    }
    // Hindernisse: gewichtet gewuerfelt, nie zweimal dasselbe direkt hintereinander
    let weights = [
        (Slot::Slalom, 2.0),
        (Slot::Duck, 2.0),
        (Slot::Jump, 2.0),
        (Slot::Door, 1.5),
        (Slot::Shelf, 1.2),
        (Slot::Cart, 1.0),
        (Slot::Squeeze, 1.0),
    ];
    let total: f32 = weights.iter().map(|w| w.1).sum();
    let mut prev = None;
    for &s in &spots {
        let slot = loop {
            let mut r = rng.gen_range(0.0..total);
            let mut pick = Slot::Slalom;
            for &(k, w) in &weights {
                if r < w {
                    pick = k;
                    break;
                }
                r -= w;
            }
            // Engstellen sind 3 m lang: nicht so nah an einer Ecke, dass sie hineinragen
            let too_long = pick == Slot::Squeeze && near_corner(c, s, hw + SQUEEZE_LEN * 0.5 + 0.5);
            if Some(pick) != prev && !too_long {
                break pick;
            }
        };
        prev = Some(slot);
        let f = Frame::new(c.point_at(s), c.dir_at_s(s));
        match slot {
            Slot::Slalom => slalom(b, rng, &f),
            Slot::Duck => duck(b, rng, c, &f),
            Slot::Jump => jump(b, rng, c, &f),
            Slot::Door => {
                door_frame(b, &f);
                push(
                    c,
                    s,
                    StationKind::Door {
                        stuck: rng.gen_bool(0.3),
                    },
                );
            }
            Slot::Shelf => push(
                c,
                s,
                StationKind::Shelf {
                    side: if rng.gen_bool(0.5) { 1.0 } else { -1.0 },
                },
            ),
            Slot::Cart => push(
                c,
                s,
                StationKind::Cart {
                    lane: rng.gen_range(-0.7..0.7),
                },
            ),
            Slot::Squeeze => squeeze(b, rng, c, &f),
        }
    }
    // Ende: Ausgangstuer in der linken Wand des letzten Stuecks
    let s = c.len - (EXIT_FROM + EXIT_TO) * 0.5;
    push(c, s, StationKind::ExitDoor);
    c.stations.sort_by(|a, b| a.s.total_cmp(&b.s));
}

fn push(c: &mut Corridor, s: f32, kind: StationKind) {
    let d = c.dir_at_s(s);
    c.stations.push(Station {
        s,
        kind,
        at: c.point_at(s),
        dir: d,
        side: side_of(d),
    });
}

/// Kisten-Reihe quer im Gang mit einer Luecke von 1,25-1,7 m (wie frueher).
fn slalom(b: &mut Builder, rng: &mut Pcg64, f: &Frame) {
    let hw = CORRIDOR_HALF_W;
    let gap_w = rng.gen_range(1.25..1.7);
    let gap_c = rng.gen_range(-hw + gap_w * 0.5..hw - gap_w * 0.5);
    let (ga, gb) = (gap_c - gap_w * 0.5, gap_c + gap_w * 0.5);
    for (v0, v1) in [(-hw, ga), (gb, hw)] {
        if v1 - v0 > 0.3 {
            let u = rng.gen_range(-0.4..0.4);
            let kind = rng.gen_range(0..4);
            obstacle(b, rng, f, v0, v1, u, kind);
        }
    }
}

/// Ein Block zwischen `v0` und `v1` (quer) bei `u`: 0 Kistenstapel, 1 umgekipptes Regal,
/// 2 Palette mit Tomatensaft, 3 Kartonsaeule.
fn obstacle(b: &mut Builder, rng: &mut Pcg64, f: &Frame, v0: f32, v1: f32, u: f32, kind: u32) {
    let vc = (v0 + v1) * 0.5;
    let hv = (v1 - v0) * 0.5;
    match kind {
        0 => {
            b.solid(f.at(u, vc, 0.55), f.half(0.4, hv, 0.55));
            let n = ((v1 - v0) / 0.8).ceil().max(1.0) as usize;
            let w = hv / n as f32;
            for i in 0..n {
                let v = v0 + w * (2 * i + 1) as f32;
                let tall = rng.gen_bool(0.6);
                for level in 0..(1 + tall as usize) {
                    b.add_rot(
                        f.at(
                            u + rng.gen_range(-0.08..0.08),
                            v,
                            0.27 + 0.55 * level as f32,
                        ),
                        vec3(w * 0.95, 0.27, 0.38),
                        f.yaw() + rng.gen_range(-0.18..0.18),
                        vec3(0.45, 0.32, 0.18) * rng.gen_range(0.75..1.1),
                        PLAIN,
                    );
                }
            }
        }
        1 => {
            b.add_solid(
                f.at(u, vc, 0.35),
                f.half(0.28, hv, 0.35),
                vec3(0.3, 0.31, 0.33),
            );
            b.add(
                f.at(u, vc, 0.72),
                f.half(0.3, hv, 0.02),
                vec3(0.95, 0.8, 0.2),
                PLAIN,
            );
        }
        2 => {
            b.solid(f.at(u, vc, 0.45), f.half(0.45, hv, 0.45));
            b.add(
                f.at(u, vc, 0.07),
                f.half(0.45, hv, 0.07),
                vec3(0.5, 0.4, 0.25),
                PLAIN,
            );
            b.add(
                f.at(u, vc, 0.5),
                f.half(0.42, hv * 0.95, 0.36),
                vec3(0.55, 0.06, 0.05),
                PLAIN,
            );
        }
        _ => {
            // Kartonsaeule, an einer zufaelligen Stelle im Block
            let pw = hv.min(0.3);
            let pv = rng.gen_range(v0 + pw..=v1 - pw);
            b.add_solid(
                f.at(u, pv, 0.9),
                f.half(0.3, pw, 0.9),
                vec3(0.42, 0.3, 0.17),
            );
        }
    }
}

/// Huerde ueber die ganze Breite eintragen.
fn hurdle(c: &mut Corridor, f: &Frame, y0: f32, y1: f32, duck: bool) {
    let center = f.at(0.0, 0.0, (y0 + y1) * 0.5);
    let half = f.half(HURDLE_DEPTH * 0.5, CORRIDOR_HALF_W, (y1 - y0) * 0.5);
    c.hurdles.push(Hurdle {
        area: Aabb::from_center(center, half),
        duck,
    });
}

/// Ducken: herabhaengende Rohre und Kabel oder ein halb heruntergelassenes Rollgitter.
fn duck(b: &mut Builder, rng: &mut Pcg64, c: &mut Corridor, f: &Frame) {
    let hw = CORRIDOR_HALF_W;
    let h = CORRIDOR_H;
    let bottom = rng.gen_range(DUCK_BOTTOM.0..DUCK_BOTTOM.1);
    if rng.gen_bool(0.5) {
        // Rohre quer, dazwischen haengende Kabel
        for (k, y) in [bottom + 0.06, bottom + 0.26, bottom + 0.5]
            .iter()
            .enumerate()
        {
            b.add(
                f.at(k as f32 * 0.1 - 0.1, 0.0, *y),
                f.half(0.06, hw, 0.06),
                vec3(0.32, 0.3, 0.27),
                PLAIN,
            );
        }
        for k in 0..5 {
            let v = -hw + 0.3 + k as f32 * 0.6 + rng.gen_range(-0.1..0.1);
            let low = bottom + rng.gen_range(0.0..0.12);
            b.add(
                f.at(0.05, v, (low + h) * 0.5),
                f.half(0.012, 0.012, (h - low) * 0.5),
                vec3(0.05, 0.05, 0.05),
                PLAIN,
            );
        }
        // Halterung an der Decke
        b.add(
            f.at(0.0, 0.0, (bottom + 0.5 + h) * 0.5),
            f.half(0.03, 0.03, (h - bottom - 0.5) * 0.5),
            vec3(0.25, 0.25, 0.25),
            PLAIN,
        );
    } else {
        // Rollgitter: von der Decke bis zur Unterkante, mit Lamellen und Warnstreifen
        b.add(
            f.at(0.0, 0.0, (bottom + h) * 0.5),
            f.half(0.04, hw, (h - bottom) * 0.5),
            vec3(0.32, 0.33, 0.34),
            PLAIN,
        );
        let mut y = bottom + 0.1;
        while y < h - 0.1 {
            b.add(
                f.at(-0.045, 0.0, y),
                f.half(0.006, hw, 0.012),
                vec3(0.18, 0.18, 0.19),
                PLAIN,
            );
            y += 0.16;
        }
        b.add(
            f.at(-0.045, 0.0, bottom + 0.04),
            f.half(0.008, hw, 0.04),
            vec3(0.95, 0.75, 0.1),
            PLAIN,
        );
    }
    hurdle(c, f, bottom, h, true);
}

/// Springen: niedrige Barriere ueber die ganze Breite (Palette, Kartons, Absperrung).
fn jump(b: &mut Builder, rng: &mut Pcg64, c: &mut Corridor, f: &Frame) {
    let hw = CORRIDOR_HALF_W;
    let top = JUMP_TOP;
    match rng.gen_range(0..3) {
        0 => {
            // Palette mit Kartons
            b.add(
                f.at(0.0, 0.0, 0.07),
                f.half(0.17, hw, 0.07),
                vec3(0.5, 0.4, 0.25),
                PLAIN,
            );
            let mut v = -hw + 0.25;
            while v < hw - 0.2 {
                let hh = rng
                    .gen_range(0.12..(top - 0.14) * 0.5 + 0.02)
                    .min((top - 0.14) * 0.5);
                b.add_rot(
                    f.at(0.0, v, 0.14 + hh),
                    vec3(0.22, hh, 0.16),
                    f.yaw() + rng.gen_range(-0.2..0.2),
                    vec3(0.45, 0.32, 0.18) * rng.gen_range(0.75..1.1),
                    PLAIN,
                );
                v += 0.5;
            }
        }
        1 => {
            // umgestuerzte Absperrung mit Warnstreifen
            b.add(
                f.at(0.0, 0.0, top * 0.5),
                f.half(0.12, hw, top * 0.5),
                vec3(0.3, 0.3, 0.32),
                PLAIN,
            );
            for k in 0..6 {
                let v = -hw + 0.25 + k as f32 * 0.5;
                b.add(
                    f.at(-0.125, v, top * 0.5),
                    f.half(0.004, 0.12, top * 0.45),
                    if k % 2 == 0 {
                        vec3(0.95, 0.75, 0.1)
                    } else {
                        vec3(0.08, 0.08, 0.08)
                    },
                    PLAIN,
                );
            }
        }
        _ => {
            // Reihe umgekippter Getraenkekisten
            let mut v = -hw + 0.2;
            while v < hw - 0.15 {
                b.add_rot(
                    f.at(rng.gen_range(-0.05..0.05), v, top * 0.5),
                    vec3(0.18, top * 0.5, 0.16),
                    f.yaw() + rng.gen_range(-0.25..0.25),
                    [
                        vec3(0.6, 0.1, 0.08),
                        vec3(0.1, 0.25, 0.5),
                        vec3(0.15, 0.4, 0.15),
                    ][rng.gen_range(0..3)],
                    PLAIN,
                );
                v += 0.4;
            }
        }
    }
    hurdle(c, f, 0.0, top, false);
}

/// Tuerrahmen fuer eine Schwingtuer (die Fluegel bewegt das Spiel).
fn door_frame(b: &mut Builder, f: &Frame) {
    let hw = CORRIDOR_HALF_W;
    let c = vec3(0.16, 0.17, 0.18);
    for s in [-1.0f32, 1.0] {
        b.add(
            f.at(0.0, s * (hw - 0.06), 1.15),
            f.half(0.1, 0.06, 1.15),
            c,
            PLAIN,
        );
    }
    b.add(
        f.at(0.0, 0.0, (2.3 + CORRIDOR_H) * 0.5),
        f.half(0.1, CORRIDOR_HALF_W, (CORRIDOR_H - 2.3) * 0.5),
        c,
        PLAIN,
    );
}

/// Engstelle: die Waende ruecken fuer 3 m auf 0,8 m zusammen (dort nur langsam).
fn squeeze(b: &mut Builder, rng: &mut Pcg64, c: &mut Corridor, f: &Frame) {
    let hw = CORRIDOR_HALF_W;
    let gc = rng.gen_range(-0.6..0.6);
    let (ga, gb) = (gc - SQUEEZE_GAP * 0.5, gc + SQUEEZE_GAP * 0.5);
    let hu = SQUEEZE_LEN * 0.5;
    for (v0, v1) in [(-hw, ga), (gb, hw)] {
        let vc = (v0 + v1) * 0.5;
        let hv = (v1 - v0) * 0.5;
        b.add_solid(
            f.at(0.0, vc, CORRIDOR_H * 0.5),
            f.half(hu, hv, CORRIDOR_H * 0.5),
            vec3(0.17, 0.2, 0.17),
        );
        // Rohre auf der Gangseite
        let inner = if v0 < 0.0 { v1 } else { v0 };
        for y in [0.6, 1.3, 2.0] {
            b.add(
                f.at(0.0, inner + if v0 < 0.0 { 0.04 } else { -0.04 }, y),
                f.half(hu, 0.04, 0.04),
                vec3(0.3, 0.28, 0.25),
                PLAIN,
            );
        }
    }
    c.squeezes.push(Aabb::from_center(
        f.at(0.0, gc, CORRIDOR_H * 0.5),
        f.half(hu + 0.3, SQUEEZE_GAP * 0.5 + 0.1, CORRIDOR_H * 0.5),
    ));
}
