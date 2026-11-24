use super::obstacles::Block;
use super::*;
use crate::game::tests::{press_e, step};
use crate::world::Station;
use std::f32::consts::{FRAC_PI_2, PI};

const DT: f32 = 1.0 / 60.0;

/// Bezahlt, Alarm laeuft, Spieler steht im Gang (Tuer hinter ihm zu) und schaut nach vorn.
/// `len`: Ganglaenge erzwingen (None = aus der Run-Zeit, also 40 m).
fn in_corridor(seed: u64, len: Option<f32>) -> Game {
    let mut g = Game::new(seed);
    g.tip();
    g.fill_list_cheap();
    g.scanned.append(&mut g.cart);
    g.pay();
    // im Test reproduzierbar (im Spiel kommt der Zufall des Gangs von der Uhrzeit)
    g.params.corridor_seed = seed.wrapping_mul(7919);
    if let Some(len) = len {
        g.params.corridor_len = len;
    }
    g.corridor_run = None;
    g.rebuild_world();
    g.customers.clear();
    g.employees.clear();
    let c = g
        .world
        .corridor
        .clone()
        .expect("kein Korridor nach dem Bezahlen");
    g.player.pos = c.point_at(ENTER_AT + 1.5);
    g.player.yaw = PI; // Blick nach +z (erstes Stueck)
    g.player.pitch = 0.0;
    g.update(DT, &mut Input::default());
    g
}

/// Blickwinkel (yaw) in Richtung `d`.
fn yaw_to(d: Vec3) -> f32 {
    // view_dir = (sin yaw, 0, -cos yaw)
    d.x.atan2(-d.z)
}

fn corridor(g: &Game) -> Corridor {
    g.world.corridor.clone().unwrap()
}

/// Einen Gang suchen, in dem es eine Station dieser Art gibt.
fn with_station(kind: fn(&StationKind) -> bool) -> (Game, Station, usize) {
    for seed in 0..80 {
        let g = in_corridor(400 + seed, Some(140.0));
        let c = corridor(&g);
        if let Some((i, st)) = c.stations.iter().enumerate().find(|(_, s)| kind(&s.kind)) {
            return (g, *st, i);
        }
    }
    panic!("keine passende Station gefunden");
}

/// Vor eine Stelle stellen (so weit davor), Blick nach vorn.
fn stand_before(g: &mut Game, s: f32, before: f32) {
    let c = corridor(g);
    g.player.pos = c.point_at(s - before);
    g.player.yaw = yaw_to(c.dir_at_s(s - before));
    g.player.vel = Vec3::ZERO;
    g.god = true; // die Front soll hier nicht stoeren
    if let Some(r) = &mut g.corridor_run {
        r.front = -100.0;
    }
}

#[test]
fn paying_opens_a_winding_corridor_and_you_are_alone_in_it() {
    let g = in_corridor(81, None);
    let c = corridor(&g);
    assert!(c.len >= 39.0, "Gang zu kurz: {}", c.len);
    assert!(g.in_corridor());
    assert!(g.chasing());
    assert!(g.corridor_entered(), "Tuer hinter einem nicht zu");
    assert!(
        g.customers.is_empty() && g.employees.is_empty(),
        "NPCs im Gang"
    );
    // Spiegel am Ende, Ausgangstuer kurz davor, aber nicht gleich hinter der Ladentuer
    let exit = c
        .stations
        .iter()
        .find(|s| s.kind == StationKind::ExitDoor)
        .unwrap();
    assert!(exit.s > 30.0 && exit.s > c.len - 3.0);
    assert!(!g.world.exit_zone.contains_xz(g.player.pos));
    // zurueck zur Tuer geht nicht (sie ist zu)
    let mut g = g;
    g.god = true;
    g.player.yaw = 0.0;
    step(
        &mut g,
        2.0,
        Input {
            forward: true,
            ..Default::default()
        },
    );
    assert!(
        g.corridor_progress() > 0.1,
        "durch die zugeknallte Tuer gelaufen"
    );
}

#[test]
fn corridor_length_depends_on_the_time_in_the_run() {
    let mut g = Game::new(90);
    g.tip();
    g.fill_list_cheap();
    g.scanned.append(&mut g.cart);
    g.run_time = 600.0; // 10 Minuten: 40 + 5 * 8 = 80 m
    g.pay();
    assert!((g.params.corridor_len - 80.0).abs() < 1e-3);
    let c = corridor(&g);
    assert!((c.len - 80.0).abs() < 6.5, "{} m", c.len);
    // ein Umbau des Markts (Stufe) baut denselben Gang
    let pts = c.pts.clone();
    g.rebuild_world();
    assert_eq!(corridor(&g).pts, pts);
}

#[test]
fn corridor_is_different_every_run() {
    // gleicher Seed, mehrmals bezahlt (echter Zufall aus der Uhrzeit): jedes Mal ein anderer Weg
    let mut shapes = std::collections::HashSet::new();
    for _ in 0..12 {
        let mut g = Game::new(82);
        g.tip();
        g.fill_list_cheap();
        g.scanned.append(&mut g.cart);
        g.pay();
        let key: Vec<i32> = corridor(&g)
            .pts
            .iter()
            .map(|p| (p.x * 10.0) as i32)
            .collect();
        shapes.insert(key);
    }
    assert!(shapes.len() > 8, "Gang kaum zufaellig: {}", shapes.len());
}

#[test]
fn tutorial_turning_around_only_warns() {
    let mut g = in_corridor(84, None);
    g.tutorial_run = true;
    g.tutorial = Some(crate::game::tutorial::Tutorial::default());
    g.player.yaw = 0.0; // Blick zurueck
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Running, "im Tutorial Game Over");
    assert!(!g.turned_back);
    assert!(g.tutorial.as_ref().unwrap().turn_warned);
    // wieder nach vorn: die Warnung kann erneut kommen
    g.player.yaw = PI;
    g.update(DT, &mut Input::default());
    assert!(!g.tutorial.as_ref().unwrap().turn_warned);
}

#[test]
fn turning_around_in_the_corridor_is_instant_game_over() {
    let mut g = in_corridor(83, None);
    // seitlich schauen ist erlaubt (Warnung), umdrehen nicht
    g.player.yaw = PI * 0.5;
    g.update(DT, &mut Input::default());
    assert_eq!(
        g.outcome,
        Outcome::Running,
        "zur Seite schauen ist schon tot"
    );
    assert!(g.turn_warning() > 0.0, "keine Warnung beim Seitenblick");
    g.player.yaw = 0.0; // Blick zurueck (-z)
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Caught);
    assert!(g.turned_back);
}

#[test]
fn after_a_corner_forward_is_the_new_direction() {
    let mut g = in_corridor(88, Some(90.0));
    let c = corridor(&g);
    // mitten im zweiten Stueck (seitlich)
    let (_, d, l, cum) = c.segment(1);
    assert!(d.x.abs() > 0.99, "zweites Stueck nicht seitlich");
    let s = cum + l * 0.5;
    g.god = false;
    g.player.pos = c.point_at(s);
    if let Some(r) = &mut g.corridor_run {
        r.front = -100.0;
    }
    g.player.yaw = yaw_to(d);
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Running, "nach vorn schauen ist tot");
    assert!(g.corridor_facing() > 0.9);
    // in die alte Richtung (+z) zu schauen ist hier nur seitlich
    g.player.yaw = PI;
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Running);
    g.player.yaw = yaw_to(-d);
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Caught, "zurueck geschaut, aber lebt");
}

#[test]
fn turning_around_triggers_a_jumpscare_face() {
    let mut g = in_corridor(87, None);
    g.player.yaw = 0.0;
    g.update(DT, &mut Input::default());
    assert!(g.turned_back);
    assert!(
        g.scare_flash() > 0.0 && g.scare_zoom() < 1.0,
        "kein Schreck-Effekt"
    );
    let far = g.turn_scare_face().expect("kein Gesicht").1;
    for _ in 0..20 {
        g.update(DT, &mut Input::default());
    }
    let near = g.turn_scare_face().expect("Gesicht zu frueh weg").1;
    assert!(
        near < far && near < 0.5,
        "Gesicht kommt nicht heran: {far} -> {near}"
    );
    let mut people = Vec::new();
    g.collect_people(&mut people);
    let eye = g.eye();
    assert!(
        people.iter().any(|p| (p.pos - eye).length() < 2.0),
        "Gesicht wird nicht gezeichnet"
    );
    for _ in 0..(TURN_FACE_TIME * 60.0) as usize {
        g.update(DT, &mut Input::default());
    }
    assert!(g.turn_scare_face().is_none());
}

#[test]
fn cheats_jump_into_the_corridor() {
    let mut g = Game::new(86);
    let i = CHEATS
        .iter()
        .position(|c| *c == "Flucht: Korridor")
        .unwrap();
    g.cheat(i);
    g.update(DT, &mut Input::default());
    assert!(g.paid && g.chasing());
    assert!(g.in_corridor(), "Cheat setzt nicht in den Korridor");
    assert_eq!(g.outcome, Outcome::Running);
    let mut g = Game::new(86);
    g.cheat(crate::game::cheats::LONG_CORRIDOR_CHEAT);
    g.update(DT, &mut Input::default());
    assert!(g.in_corridor());
    assert!(corridor(&g).len > 130.0, "langer Korridor ist kurz");
}

#[test]
fn reaching_the_exit_wins() {
    let mut g = in_corridor(84, None);
    let exit = g.world.exit_zone;
    g.player.pos = vec3(exit.min.x + 0.3, 0.0, (exit.min.z + exit.max.z) * 0.5);
    g.update(DT, &mut Input::default());
    assert_eq!(g.outcome, Outcome::Won);
}

#[test]
fn mirror_camera_sits_behind_the_mirror_and_looks_back() {
    let mut g = in_corridor(85, None);
    let c = corridor(&g);
    // im letzten Stueck, vor dem Spiegel
    g.player.pos = c.point_at(c.len - 4.0);
    g.player.yaw = PI;
    let (vp, cam) = g.mirror_view().expect("kein Spiegelbild im Gang");
    assert!(cam.z > c.mirror_center.z, "Kamera nicht hinter dem Spiegel");
    for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
        let p = c.mirror_center + vec3(sx * c.mirror_half.x, sy * c.mirror_half.y, 0.0);
        let q = vp * p.extend(1.0);
        let (x, y) = (q.x / q.w, q.y / q.w);
        assert!(
            (x.abs() - 1.0).abs() < 1e-3 && (y.abs() - 1.0).abs() < 1e-3,
            "{x} {y}"
        );
    }
    assert!(Game::new(85).mirror_view().is_none());
    let _ = &mut g;
}

#[test]
fn endless_stamina_in_the_corridor() {
    let mut g = in_corridor(89, None);
    g.god = true;
    g.player.stamina = 0.2;
    step(
        &mut g,
        3.0,
        Input {
            forward: true,
            run: true,
            ..Default::default()
        },
    );
    // voll und bleibt voll (der Balken erscheint erst unter 99,9 %), im Gang ohnehin ausgeblendet
    assert!(
        g.player.stamina >= 0.999,
        "Ausdauer sinkt im Gang: {}",
        g.player.stamina
    );
    assert!(
        g.corridor_endless_stamina(),
        "Balken wuerde im Gang erscheinen"
    );
    assert!(g.player.running);
}

#[test]
fn duck_hurdle_needs_crouching() {
    let (mut g, _, _) = with_station(|_| true);
    let c = corridor(&g);
    let h = *c
        .hurdles
        .iter()
        .find(|h| h.duck)
        .expect("keine Duck-Huerde");
    let s = c.progress((h.area.min + h.area.max) * 0.5);
    let fwd = Input {
        forward: true,
        ..Default::default()
    };
    stand_before(&mut g, s, 1.2);
    step(&mut g, 2.5, fwd);
    assert!(g.corridor_progress() < s, "stehend unter der Huerde durch");
    stand_before(&mut g, s, 1.2);
    step(
        &mut g,
        3.0,
        Input {
            crouch: true,
            ..fwd
        },
    );
    assert!(
        g.corridor_progress() > s + 0.5,
        "geduckt nicht durchgekommen"
    );
    // geduckt ist man langsamer und tiefer
    assert!(g.player.crouch > 0.9 && g.eye().y < 1.0);
}

#[test]
fn jump_hurdle_needs_jumping() {
    let (mut g, _, _) = with_station(|_| true);
    let c = corridor(&g);
    let h = *c
        .hurdles
        .iter()
        .find(|h| !h.duck)
        .expect("keine Spring-Huerde");
    let s = c.progress((h.area.min + h.area.max) * 0.5);
    let run = Input {
        forward: true,
        run: true,
        ..Default::default()
    };
    stand_before(&mut g, s, 2.0);
    step(&mut g, 2.0, run);
    assert!(g.corridor_progress() < s, "ohne Sprung ueber die Huerde");
    // anlaufen und kurz davor springen
    stand_before(&mut g, s, 2.2);
    for _ in 0..240 {
        let gap = s - g.corridor_progress();
        let jump = gap < 0.75 && gap > 0.3 && g.player.jump_y <= 0.0;
        let mut inp = Input { jump, ..run };
        g.update(DT, &mut inp);
    }
    assert!(g.corridor_progress() > s + 0.5, "mit Sprung nicht drueber");
    // geduckt springt man nicht
    g.player.crouch = 1.0;
    let mut inp = Input {
        jump: true,
        crouch: true,
        ..Default::default()
    };
    g.update(DT, &mut inp);
    assert_eq!(g.player.jump_y, 0.0);
}

#[test]
fn doors_open_with_e_and_stuck_ones_need_three_tries() {
    let (mut g, st, i) = with_station(|k| matches!(k, StationKind::Door { .. }));
    stand_before(&mut g, st.s, 0.8);
    g.update(DT, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Door(i)), "Tuer nicht anvisiert");
    // zu: man kommt nicht durch
    let fwd = Input {
        forward: true,
        ..Default::default()
    };
    step(&mut g, 1.0, fwd);
    assert!(g.corridor_progress() < st.s, "durch die geschlossene Tuer");
    let need = if matches!(st.kind, StationKind::Door { stuck: true }) {
        obstacles::STUCK_PRESSES
    } else {
        1
    };
    for k in 0..need {
        g.player.yaw = yaw_to(st.dir);
        g.update(DT, &mut Input::default());
        press_e(&mut g);
        let opening = g.corridor_run.as_ref().unwrap().stations[i].t.is_some();
        assert_eq!(opening, k + 1 == need, "Druck {k} von {need}");
    }
    step(&mut g, 2.0, fwd);
    assert!(
        g.corridor_progress() > st.s + 0.5,
        "nach dem Oeffnen nicht durch"
    );
}

#[test]
fn tipping_shelf_falls_and_stumbles_you_but_does_not_kill() {
    let (mut g, st, i) = with_station(|k| matches!(k, StationKind::Shelf { .. }));
    stand_before(&mut g, st.s, obstacles::SHELF_TRIGGER - 0.5);
    g.god = false;
    g.update(DT, &mut Input::default());
    assert!(
        g.corridor_run.as_ref().unwrap().stations[i].t.is_some(),
        "kippt nicht"
    );
    // direkt drunter stellen, bis es aufschlaegt
    g.player.pos = st.at;
    step(&mut g, obstacles::SHELF_FALL + 0.2, Input::default());
    let r = g.corridor_run.as_ref().unwrap();
    assert!(r.stations[i].done, "liegt nicht");
    assert!(r.stumble > 0.0, "kein Stolpern");
    assert_eq!(g.outcome, Outcome::Running);
    // jetzt liegt es quer: Spring-Huerde
    assert!(g
        .corridor_blockers()
        .iter()
        .any(|(_, b)| matches!(b, Block::Jump(_))));
}

#[test]
fn rolling_cart_comes_at_you_and_a_hit_only_stumbles() {
    let (mut g, st, i) = with_station(|k| matches!(k, StationKind::Cart { .. }));
    let lane = match st.kind {
        StationKind::Cart { lane } => lane,
        _ => unreachable!(),
    };
    stand_before(&mut g, st.s, obstacles::CART_TRIGGER - 1.0);
    // in die Spur des Wagens stellen
    let c = corridor(&g);
    let s0 = c.progress(g.player.pos);
    g.player.pos = c.point_at(s0) + crate::world::side_of(c.dir_at_s(s0)) * lane;
    // unverwundbar: nur der Wagen soll wirken (die Front holt beim Stehen sonst auf)
    g.god = true;
    let start = g.corridor_run.as_ref().unwrap().stations[i].s;
    step(&mut g, 4.0, Input::default());
    let r = g.corridor_run.as_ref().unwrap();
    assert!(r.stations[i].s < start - 2.0, "Wagen rollt nicht");
    assert!(r.stations[i].done, "Wagen trifft nicht");
    assert!(r.stumble > 0.0 || r.stations[i].t.is_some());
    assert_eq!(g.outcome, Outcome::Running, "Wagen toetet");
}

#[test]
fn standing_still_gets_you_caught_by_the_front() {
    let mut g = in_corridor(91, None);
    let mut face = false;
    for _ in 0..360 {
        g.update(DT, &mut Input::default());
        face |= g.turn_scare_face().is_some();
    }
    assert_eq!(g.outcome, Outcome::Caught, "Front holt nicht auf");
    assert!(g.front_caught && !g.turned_back);
    assert!(face, "kein Schreck");
    // im Tutorial bleibt sie hinter einem
    let mut g = in_corridor(91, None);
    g.tutorial_run = true;
    g.tutorial = Some(crate::game::tutorial::Tutorial::default());
    step(&mut g, 10.0, Input::default());
    assert_eq!(g.outcome, Outcome::Running, "Front toetet im Tutorial");
    assert!(g.front_gap().unwrap() >= 2.9);
}

#[test]
fn figures_appear_ahead_and_vanish() {
    let (mut g, st, i) = with_station(|k| matches!(k, StationKind::Figure { .. }));
    stand_before(&mut g, st.s, events::FIGURE_SHOW - 0.5);
    g.update(DT, &mut Input::default());
    let mut people = Vec::new();
    g.collect_people(&mut people);
    assert!(
        people.iter().any(|p| (p.pos - st.at).length() < 0.5),
        "Gestalt nicht zu sehen"
    );
    stand_before(&mut g, st.s, events::FIGURE_GONE - 0.5);
    g.update(DT, &mut Input::default());
    assert!(
        g.corridor_run.as_ref().unwrap().stations[i].done,
        "Gestalt bleibt"
    );
}

#[test]
fn ceiling_comes_down_and_you_have_to_duck() {
    let (mut g, st, _) = with_station(|k| *k == StationKind::Ceiling);
    stand_before(&mut g, st.s, events::CHANGE_TRIGGER - 0.5);
    step(&mut g, 3.0, Input::default());
    let duck = g.corridor_blockers().iter().find_map(|(_, b)| match b {
        Block::Duck(y) => Some(*y),
        _ => None,
    });
    assert!(duck.is_some_and(|y| y < 1.3), "Decke kommt nicht herunter");
}

#[test]
fn walls_close_in_but_you_can_still_sprint() {
    let (mut g, st, _) = with_station(|k| *k == StationKind::Walls);
    stand_before(&mut g, st.s, events::CHANGE_TRIGGER - 0.5);
    step(&mut g, 3.0, Input::default());
    // die Waende sind zusammengerueckt ...
    let walls = g
        .corridor_blockers()
        .iter()
        .filter(|(b, rule)| {
            *rule == Block::Solid
                && (b.max.y - b.min.y) > 2.0
                && st.at.distance((b.min + b.max) * 0.5) < 3.5
        })
        .count();
    assert!(walls >= 2, "Waende ruecken nicht zusammen");
    // ... aber dazwischen sprintet man mit vollem Tempo
    g.player.pos = st.at - st.dir * 2.0;
    g.player.yaw = yaw_to(st.dir);
    assert!(g.between_walls(g.player.pos));
    let (_, cap) = g.corridor_speed();
    assert!(
        cap > crate::game::RUN_SPEED,
        "zwischen den Waenden gedrosselt"
    );
    let mut top = 0.0f32;
    for _ in 0..60 {
        let mut inp = Input {
            forward: true,
            run: true,
            ..Default::default()
        };
        g.update(DT, &mut inp);
        top = top.max(g.player.vel.length());
    }
    assert!(g.player.running, "rennt nicht");
    assert!(top > crate::game::RUN_SPEED * 0.95, "nur {top} m/s");
}

#[test]
fn at_the_end_someone_stands_behind_you_in_the_mirror() {
    let mut g = in_corridor(92, None);
    let c = corridor(&g);
    let st = *c
        .stations
        .iter()
        .find(|s| s.kind == StationKind::ExitDoor)
        .unwrap();
    g.player.pos = st.at;
    g.player.yaw = PI;
    g.update(DT, &mut Input::default());
    let fig = g.ending_figure().expect("niemand hinter einem");
    assert!((fig - g.player.pos).length() < 1.6);
    // im Spiegelbild zu sehen
    let (vp, _) = g.mirror_view().unwrap();
    let q = vp * (fig + vec3(0.0, 1.0, 0.0)).extend(1.0);
    assert!(q.w > 0.0 && (q.x / q.w).abs() < 1.0 && (q.y / q.w).abs() < 1.0);
    // an der klemmenden Tuer zu lange zoegern: eingeholt
    g.player.yaw = FRAC_PI_2;
    step(&mut g, 7.0, Input::default());
    assert_eq!(
        g.outcome,
        Outcome::Caught,
        "zoegern an der Tuer kostet nichts"
    );
}

#[test]
fn last_scare_at_the_exit_does_not_kill() {
    let mut g = in_corridor(93, None);
    let c = corridor(&g);
    let st = *c
        .stations
        .iter()
        .find(|s| s.kind == StationKind::ExitDoor)
        .unwrap();
    g.player.pos = st.at + st.side * (CORRIDOR_HALF_W - 0.6);
    g.player.yaw = FRAC_PI_2;
    g.update(DT, &mut Input::default());
    for _ in 0..obstacles::STUCK_PRESSES {
        press_e(&mut g);
        step(&mut g, 0.1, Input::default());
    }
    g.sfx.clear();
    step(
        &mut g,
        3.0,
        Input {
            forward: true,
            ..Default::default()
        },
    );
    assert!(
        g.corridor_run.as_ref().unwrap().last_scare,
        "kein letzter Schreck"
    );
    assert_eq!(g.outcome, Outcome::Won);
    // geschafft: der letzte Schreck ist nur zu sehen, nicht zu hoeren
    assert!(
        !g.sfx
            .iter()
            .any(|(s, _)| matches!(s, Sfx::Scare | Sfx::Shriek)),
        "Schreck-Ton beim Entkommen"
    );
}

/// Ein Testlaeufer: rennt die Mittellinie entlang, weicht Bloecken aus, duckt, springt und
/// drueckt Tueren auf. Liefert (gewonnen, kleinster Abstand zur Front).
fn run_bot(seed: u64, len: f32) -> (bool, f32) {
    let mut g = in_corridor(seed, Some(len));
    let c = corridor(&g);
    let exit_s = c
        .stations
        .iter()
        .find(|s| s.kind == StationKind::ExitDoor)
        .unwrap()
        .s;
    let mut v = 0.0f32;
    let mut min_gap = f32::MAX;
    let mut last_gap = 0.0f32;
    for _ in 0..(len / 1.2 * 60.0) as usize {
        if g.outcome != Outcome::Running {
            break;
        }
        let p = g.player.pos;
        let s = c.progress(p);
        let ending = g.corridor_run.as_ref().is_some_and(|r| r.ending);
        if let Some(gap) = g.front_gap().filter(|_| !ending) {
            last_gap = gap;
            min_gap = min_gap.min(gap);
        }
        let mut inp = Input {
            forward: true,
            run: true,
            ..Default::default()
        };
        if s > exit_s - 0.3 || !c.contains(p) {
            // am Ende: zur Ausgangstuer (links, +x), aufdruecken, hindurch
            g.player.yaw = FRAC_PI_2;
            g.player.pitch = 0.0;
            inp.interact = g.corridor_doors().iter().any(|(_, b)| b.max.x > p.x);
            g.update(DT, &mut inp);
            continue;
        }
        // was liegt direkt voraus?
        let ahead = |b: &world::Aabb| {
            let m = (b.min + b.max) * 0.5;
            let d = c.progress(m) - s;
            (d, c.contains(m))
        };
        let mut duck = false;
        let mut jump = false;
        for (b, rule) in g.corridor_blockers() {
            // Abstand zum Kasten (nicht zu seiner Mitte: die Decke ist 6 m lang)
            let cx = p.x.clamp(b.min.x, b.max.x);
            let cz = p.z.clamp(b.min.z, b.max.z);
            let near = vec3(p.x - cx, 0.0, p.z - cz).length();
            let (d, inside) = ahead(&b);
            match rule {
                Block::Duck(y) if inside && near < 1.3 && y < 1.8 => duck = true,
                Block::Jump(_) if inside && d > 0.0 && near > 0.3 && near < 0.75 => jump = true,
                _ => {}
            }
        }
        inp.crouch = duck;
        inp.jump = jump && g.player.jump_y <= 0.0;
        // Tuer voraus: aufdruecken
        let door = g
            .corridor_doors()
            .iter()
            .any(|(_, b)| ahead(b).0 > 0.0 && ahead(b).0 < 1.2);
        inp.interact = door;
        // Ziel 1,6 m voraus, seitlich dort, wo frei ist (Luecken in den Reihen)
        let la = (s + 1.6).min(exit_s);
        let d = c.dir_at_s(la);
        let side = crate::world::side_of(d);
        let base = c.point_at(la);
        let free = |q: Vec3| {
            (0..=6).all(|k| {
                let t = p.lerp(q, k as f32 / 6.0);
                let t = vec3(t.x, 0.0, t.z);
                g.world.collide(t, 0.32).distance(t) < 1e-3
            })
        };
        let mut best: Option<(f32, f32)> = None;
        for k in -6..=6 {
            let cand = k as f32 * 0.2;
            if free(base + side * cand) {
                let cost = (cand - v).abs();
                if best.is_none_or(|b| cost < b.1) {
                    best = Some((cand, cost));
                }
            }
        }
        v = best.map_or(v * 0.9, |b| b.0);
        let target = base + side * v;
        g.player.yaw = yaw_to(target - p);
        g.player.pitch = 0.0;
        g.update(DT, &mut inp);
    }
    if g.outcome != Outcome::Won {
        let s = c.progress(g.player.pos);
        let near: Vec<_> = c
            .stations
            .iter()
            .filter(|st| (st.s - s).abs() < 3.0)
            .map(|st| (st.s, st.kind))
            .collect();
        let hurdles: Vec<_> = c
            .hurdles
            .iter()
            .map(|h| (c.progress((h.area.min + h.area.max) * 0.5), h.duck))
            .filter(|h| (h.0 - s).abs() < 3.0)
            .collect();
        eprintln!(
            "Laeufer Seed {seed}, {len} m: steckt bei {s:.1} von {:.1} m, {:?}, Stationen {near:?}, Huerden {hurdles:?}, Stolpern {}",
            c.len,
            g.outcome,
            g.corridor_run.as_ref().map_or(0.0, |r| r.stumble)
        );
    }
    LAST_GAP.with(|l| l.set(last_gap));
    (g.outcome == Outcome::Won, min_gap)
}

#[test]
fn a_clean_run_escapes_the_front() {
    for len in [40.0, 140.0] {
        for seed in [501, 502, 503, 504, 505, 506] {
            let (won, gap) = run_bot(seed, len);
            let end = LAST_GAP.with(|l| l.get());
            assert!(
                end <= front::FRONT_MAX_GAP + 0.1,
                "Front faellt zu weit zurueck: {end}"
            );
            assert!(
                won,
                "{len} m, Seed {seed}: nicht geschafft (Abstand zuletzt {gap})"
            );
            assert!(gap > 0.5, "{len} m, Seed {seed}: knapp ({gap} m)");
        }
    }
}

thread_local! {
    /// Abstand zur Front, als der Laeufer das letzte Stueck erreichte (nur fuer die Ausgabe)
    static LAST_GAP: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}
