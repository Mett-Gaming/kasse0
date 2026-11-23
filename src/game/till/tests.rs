use super::super::tests::{
    belt_drop, look_at_belt, look_at_terminal, press_e, put_all_on_belt, scan_and_answer,
    shelf_item, step, swipe, type_pin,
};
use super::*;

/// Gekippter Markt ohne NPCs, Zettel komplett im Wagen, Wagen und Spieler an der Kasse.
fn at_till(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    for e in g.list.clone() {
        let i = shelf_item(&g, e, false).expect("billige Ware fehlt");
        g.take(i);
    }
    g.customers.clear();
    g.employees.clear();
    let lane = g.open_lane().unwrap();
    g.trolley.pushing = false;
    g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
    look_at_belt(&mut g);
    g
}

fn lives(g: &Game) -> u32 {
    g.till.as_ref().map_or(TILL_LIVES, |t| t.lives)
}

fn doubt(g: &Game) -> f32 {
    g.till.as_ref().map_or(0.0, |t| t.doubt)
}

/// Bis zum Terminal: alles richtig aufs Band, scannen, Fragen gut beantworten, E am Terminal.
fn at_terminal(seed: u64, cash: bool) -> Game {
    let mut g = at_till(seed);
    put_all_on_belt(&mut g);
    g.till.as_mut().unwrap().cash = cash;
    scan_and_answer(&mut g);
    assert!(g.scanned_complete());
    look_at_terminal(&mut g);
    press_e(&mut g);
    g
}

fn ready_to_pay(seed: u64) -> Game {
    let g = at_terminal(seed, false);
    assert!(g.swiping(), "Durchziehen startet nicht");
    g
}

/// Ein Bild mit diesen Eingaben.
fn frame(g: &mut Game, inp: Input) {
    let mut inp = inp;
    g.update(1.0 / 60.0, &mut inp);
}

/// Passend bar bezahlen: von gross nach klein hinlegen, was in die Restsumme passt.
fn pay_cash_exactly(g: &mut Game) {
    let total = g.scanned_value();
    for _ in 0..200 {
        let t = g.till.as_ref().unwrap();
        if g.paid {
            return;
        }
        let rest = total - t.given_sum();
        let want = (0..cash::DENOMS.len())
            .find(|&i| t.wallet[i] > 0 && cash::DENOMS[i] <= rest)
            .expect("Geldboerse reicht nicht");
        let steps = want as i32 - t.sel as i32;
        frame(
            g,
            Input {
                wheel: -steps,
                ..Default::default()
            },
        );
        frame(
            g,
            Input {
                interact: true,
                ..Default::default()
            },
        );
    }
}

#[test]
fn sometimes_you_must_pay_cash_exactly() {
    let mut g = at_terminal(45, true);
    assert!(
        matches!(g.till.as_ref().unwrap().pay, Pay::Cash(_)),
        "keine Barzahlung"
    );
    assert!(!g.swiping());
    pay_cash_exactly(&mut g);
    assert!(g.paid, "passend bezahlt, aber nicht fertig");
    assert_eq!(doubt(&g), 0.0);
}

#[test]
fn paying_too_much_cash_costs_a_life_and_is_returned() {
    let mut g = at_terminal(46, true);
    // den groessten Schein, der zu viel ist, hinlegen
    let total = g.scanned_value();
    let t = g.till.as_mut().unwrap();
    t.wallet[0] += 1; // sicher ein 50-Euro-Schein
    t.sel = 0;
    let before: u32 = t.wallet.iter().sum();
    if cash::DENOMS[0] <= total {
        t.given = vec![0; (total / cash::DENOMS[0]) as usize];
    }
    frame(
        &mut g,
        Input {
            interact: true,
            ..Default::default()
        },
    );
    let t = g.till.as_ref().unwrap();
    assert!(!g.paid);
    assert_eq!(t.lives, TILL_LIVES - 1, "zu viel kostet kein Leben");
    assert!(t.given.is_empty(), "Geld nicht zurueckgeschoben");
    assert!(t.wallet.iter().sum::<u32>() >= before, "Geld verschwunden");
}

#[test]
fn paying_cash_too_slowly_is_suspicious() {
    let mut g = at_terminal(47, true);
    step(&mut g, cash::CASH_TIME_MAX + 0.2, Input::default());
    assert!((doubt(&g) - cash::SUS_CASH_SLOW).abs() < 1e-4);
    assert!(!g.paid);
}

#[test]
fn till_questions_fit_on_screen_and_have_distinct_answers() {
    assert!(TILL_QUESTIONS.len() >= 10, "zu wenig Fragen");
    for q in TILL_QUESTIONS {
        assert!(q.text.chars().count() <= 60, "{} zu lang", q.text);
        assert_ne!(q.answers[0], q.answers[1]);
        assert_ne!(q.answers[1], q.answers[2]);
        assert_ne!(q.answers[0], q.answers[2]);
        for a in q.answers {
            assert!(a.chars().count() <= 40, "{a} zu lang");
        }
    }
}

#[test]
fn cheats_jump_straight_to_card_or_cash_payment() {
    let mut g = Game::new(49);
    g.cheat(12);
    assert!(g.swiping(), "Karten-Cheat startet kein Durchziehen");
    assert!(g.scanned_complete());
    swipe(&mut g, 1.0);
    type_pin(&mut g);
    assert!(g.paid, "nach dem Karten-Cheat nicht bezahlbar");

    let mut g = Game::new(50);
    g.cheat(13);
    assert!(
        matches!(g.till.as_ref().unwrap().pay, Pay::Cash(_)),
        "Bar-Cheat oeffnet keine Geldboerse"
    );
    pay_cash_exactly(&mut g);
    assert!(g.paid, "nach dem Bar-Cheat nicht bezahlbar");
}

#[test]
fn first_item_on_belt_starts_the_till_game() {
    let mut g = at_till(31);
    assert!(g.till.is_none());
    put_all_on_belt(&mut g);
    let t = g.till.as_ref().expect("Minispiel startet nicht");
    assert_eq!(t.lives, TILL_LIVES);
    assert_eq!(t.doubt, 0.0, "richtig sortiert und trotzdem misstrauisch");
}

#[test]
fn heavy_not_first_costs_a_life_at_once() {
    let mut g = at_till(32);
    // erst etwas Leichtes, dann das Wasser (1,6 kg)
    let other = g.cart.iter().position(|c| !c.is_heavy()).unwrap();
    g.cart_sel = Some(other);
    belt_drop(&mut g);
    assert_eq!(doubt(&g), 0.0);
    let water = g.cart.iter().position(|c| c.is_heavy()).unwrap();
    g.cart_sel = Some(water);
    belt_drop(&mut g);
    assert_eq!(lives(&g), TILL_LIVES - 1, "Reihenfolge kostet kein Leben");
}

#[test]
fn npcs_are_harmless_at_the_till_but_not_away_from_it() {
    let mut g = at_till(33);
    put_all_on_belt(&mut g);
    // Verfolger direkt hinter dem Spieler, man sieht weg
    let mut c = Game::new(5);
    c.tip();
    g.customers = vec![c.customers.remove(0)];
    g.customers[0].stalker = true;
    let p = g.player.pos;
    g.customers[0].pos = vec3(p.x, 0.0, p.z + 0.5);
    g.player.yaw += std::f32::consts::PI;
    g.grace = 0.0;
    step(&mut g, 2.0, Input::default());
    assert_eq!(g.deaths, 0, "an der Kasse erwischt");
    assert_eq!(g.outcome, Outcome::Running);
    // Weggehen: Misstrauen steigt, und man ist nicht mehr geschuetzt
    let lane = g.open_lane().unwrap();
    g.player.pos = lane.scan_point + vec3(0.0, 0.0, -(TILL_RADIUS + 3.0));
    g.player.pos = g.world.collide(g.player.pos, 0.3);
    assert!(!g.till_protects());
    let before = doubt(&g);
    step(&mut g, 1.0, Input::default());
    assert!(doubt(&g) > before, "Weggehen faellt nicht auf");
}

#[test]
fn employees_do_not_talk_to_you_at_the_till() {
    let mut g = at_till(34);
    put_all_on_belt(&mut g);
    let mut other = Game::new(5);
    other.tip();
    let mut e = other.employees.remove(0);
    e.pos = g.player.pos + vec3(1.0, 0.0, 0.0);
    e.follow = 20.0;
    g.employees = vec![e];
    step(&mut g, 3.0, Input::default());
    assert!(
        g.dialog.is_none(),
        "Mitarbeiter spricht einen an der Kasse an"
    );
}

#[test]
fn silence_and_bad_answers_make_her_suspicious() {
    let mut g = at_till(35);
    put_all_on_belt(&mut g);
    // scannen lassen, bis sie fragt
    for _ in 0..60 * 30 {
        if g.till.as_ref().unwrap().asked.is_some() {
            break;
        }
        step(&mut g, 1.0 / 60.0, Input::default());
    }
    assert!(g.till.as_ref().unwrap().asked.is_some(), "keine Frage");
    // schweigen
    step(&mut g, QUESTION_TIME + 0.2, Input::default());
    assert!((doubt(&g) - SUS_SILENT).abs() < 0.05, "Schweigen egal");
}

#[test]
fn bad_answer_costs_a_life_neutral_only_doubt() {
    let mut g = at_till(48);
    put_all_on_belt(&mut g);
    let ask = |g: &mut Game| {
        for _ in 0..60 * 30 {
            let t = g.till.as_ref().unwrap();
            if t.asked.as_ref().is_some_and(|a| a.reply.is_none()) {
                return;
            }
            step(g, 1.0 / 60.0, Input::default());
        }
        panic!("keine Frage");
    };
    let pick = |g: &Game, answer: usize| {
        let a = g.till.as_ref().unwrap().asked.as_ref().unwrap();
        a.order.iter().position(|&o| o == answer).unwrap()
    };
    ask(&mut g);
    let k = pick(&g, 2); // schlechte Antwort
    step(
        &mut g,
        1.0 / 60.0,
        Input {
            choice: Some(k),
            ..Default::default()
        },
    );
    assert_eq!(
        lives(&g),
        TILL_LIVES - 1,
        "schlechte Antwort kostet kein Leben"
    );
    ask(&mut g);
    let k = pick(&g, 1); // neutrale Antwort
    step(
        &mut g,
        1.0 / 60.0,
        Input {
            choice: Some(k),
            ..Default::default()
        },
    );
    assert_eq!(
        lives(&g),
        TILL_LIVES - 1,
        "neutrale Antwort kostet ein Leben"
    );
    assert!((doubt(&g) - SUS_NEUTRAL).abs() < 0.05);
}

#[test]
fn swiping_too_fast_fails_and_slowly_works() {
    let mut g = ready_to_pay(36);
    swipe(&mut g, 0.15);
    assert_eq!(lives(&g), TILL_LIVES - 1, "zu schnell kostet kein Leben");
    assert!(g.swiping(), "nach dem Fehler nicht erneut durchziehen");
    swipe(&mut g, 1.0);
    assert!(
        matches!(g.till.as_ref().unwrap().pay, Pay::ShowPin(_)),
        "langsam durchgezogen und nicht gelesen"
    );
    assert_eq!(lives(&g), TILL_LIVES - 1);
}

#[test]
fn card_must_be_taken_out_and_swiping_too_slowly_just_retries() {
    let mut g = ready_to_pay(51);
    // ohne die Karte zu nehmen bewegt die Maus nichts
    g.look(500.0, 0.0);
    frame(&mut g, Input::default());
    assert!(matches!(
        g.till.as_ref().unwrap().pay,
        Pay::Swipe {
            ready: false,
            progress: 0.0,
            ..
        }
    ));
    // viel zu langsam: kein Leben weg, nur Misstrauen, und man darf noch mal
    swipe(&mut g, 2.5);
    assert_eq!(lives(&g), TILL_LIVES, "zu langsam kostet ein Leben");
    assert!(doubt(&g) > 0.0, "zu langsam faellt nicht auf");
    assert!(g.swiping());
    assert_eq!(
        g.till.as_ref().unwrap().reader.map(|r| r.0),
        Some("ZU LANGSAM. NOCHMAL.")
    );
    // F bricht ab
    frame(
        &mut g,
        Input {
            take_out: true,
            ..Default::default()
        },
    );
    assert!(!g.swiping());
}

#[test]
fn camera_stays_still_while_swiping() {
    let mut g = ready_to_pay(37);
    let (yaw, pitch) = (g.player.yaw, g.player.pitch);
    g.look(300.0, 80.0);
    assert_eq!((g.player.yaw, g.player.pitch), (yaw, pitch));
}

#[test]
fn wrong_pin_costs_a_life_right_pin_pays() {
    let mut g = ready_to_pay(38);
    swipe(&mut g, 1.0);
    step(&mut g, PIN_SHOW + 0.1, Input::default());
    let pin = g.till.as_ref().unwrap().pin;
    let mut inp = Input {
        digit: Some((pin[0] + 1) % 10),
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert_eq!(lives(&g), TILL_LIVES - 1, "falsche PIN kostet kein Leben");
    assert!(!g.paid);
    type_pin(&mut g);
    assert!(g.paid, "richtige PIN bezahlt nicht");
    assert!(g.params.door_open);
}

#[test]
fn full_doubt_costs_a_life_and_the_third_ends_the_run() {
    let mut g = at_till(39);
    put_all_on_belt(&mut g);
    for left in (0..TILL_LIVES).rev() {
        g.till.as_mut().unwrap().doubt = 1.0;
        step(&mut g, 1.0 / 60.0, Input::default());
        if left > 0 {
            let t = g.till.as_ref().unwrap();
            assert_eq!(t.lives, left);
            assert!(t.doubt < 0.1, "Misstrauen nicht zurueckgesetzt");
            assert_eq!(g.outcome, Outcome::Running);
        } else {
            assert_eq!(g.outcome, Outcome::Caught);
            assert!(g.till_caught);
        }
    }
    // Die Leben im Markt bleiben davon unberuehrt
    assert_eq!(g.deaths, 0);
}

// ------------------------------------------------------------ Druck an der Kasse

/// An der Kasse mit Ware auf dem Band, Druck eingeschaltet, keine Fragen, kein Aufschauen.
fn under_pressure(seed: u64) -> Game {
    let mut g = at_till(seed);
    put_all_on_belt(&mut g);
    g.calm_till = false;
    let t = g.till.as_mut().unwrap();
    t.ask_at.clear();
    t.look_wait = 1e6;
    g
}

/// Blick auf den Kopf der Kassiererin (oder mit `away` genau entgegengesetzt).
fn look_at_cashier(g: &mut Game, away: bool) {
    let head = g.open_lane().unwrap().seat + vec3(0.0, 1.6, 0.0);
    let to = head - g.eye();
    let mut yaw = to.x.atan2(-to.z);
    let mut pitch = (to.y / to.length()).asin();
    if away {
        yaw += std::f32::consts::PI;
        pitch = 0.0;
    }
    g.player.yaw = yaw;
    g.player.pitch = pitch;
}

fn force_look(g: &mut Game) {
    let t = g.till.as_mut().unwrap();
    t.look = Some(pressure::LookUp {
        warn: 0.0,
        stare: 2.0,
        caught: false,
    });
    t.look_wait = 1e6;
}

#[test]
fn stress_rises_with_time_and_lost_lives() {
    let mut g = under_pressure(91);
    let s0 = g.till_stress();
    g.till.as_mut().unwrap().time = 90.0;
    let s1 = g.till_stress();
    g.till.as_mut().unwrap().lives -= 1;
    let s2 = g.till_stress();
    assert!(s0 < s1 && s1 < s2, "Stress {s0} {s1} {s2}");
    assert!(s2 <= 1.0);
}

#[test]
fn the_cashier_gets_impatient_on_her_own() {
    let wait = |time: f32| {
        let mut g = under_pressure(92);
        g.till.as_mut().unwrap().time = time;
        look_at_cashier(&mut g, true);
        let before = doubt(&g);
        step(&mut g, 10.0, Input::default());
        doubt(&g) - before
    };
    let calm = wait(0.0);
    let late = wait(90.0);
    assert!(calm > 0.02, "keine Ungeduld: {calm}");
    assert!(
        late > calm,
        "unter Stress nicht ungeduldiger: {calm} vs {late}"
    );
}

#[test]
fn when_she_looks_up_the_scanner_stops_and_staring_is_noticed() {
    let mut g = under_pressure(93);
    look_at_cashier(&mut g, true);
    force_look(&mut g);
    let scanned = g.scanned.len();
    let before = doubt(&g);
    step(&mut g, 1.0, Input::default());
    assert!(g.cashier_looking());
    assert_eq!(g.scanned.len(), scanned, "sie scannt, obwohl sie aufschaut");
    assert!(
        doubt(&g) - before < 0.05,
        "stillhalten und wegschauen faellt auf"
    );
    // ins leere Gesicht schauen faellt auf
    force_look(&mut g);
    look_at_cashier(&mut g, false);
    let before = doubt(&g);
    step(&mut g, 0.2, Input::default());
    assert!(
        doubt(&g) - before >= pressure::SUS_STARE,
        "Anstarren faellt nicht auf"
    );
}

#[test]
fn moving_while_she_stares_is_noticed() {
    let mut g = under_pressure(94);
    look_at_cashier(&mut g, true);
    force_look(&mut g);
    let before = doubt(&g);
    step(
        &mut g,
        0.6,
        Input {
            back: true,
            ..Default::default()
        },
    );
    assert!(
        doubt(&g) - before >= pressure::SUS_STARE,
        "Bewegen faellt nicht auf"
    );
}

#[test]
fn wrong_scans_must_be_reclaimed_with_r() {
    let mut g = at_till(95);
    put_all_on_belt(&mut g);
    g.till.as_mut().unwrap().cash = false;
    scan_and_answer(&mut g);
    let fair = g.scanned_value();
    // viel zu teuer: die Summe steigt, R korrigiert
    g.debug_force_wrong_scan(false);
    assert!(g.scanned_value() > fair, "Fehlscan aendert die Summe nicht");
    let r = Input {
        complain: true,
        ..Default::default()
    };
    frame(&mut g, r);
    assert_eq!(g.scanned_value(), fair, "Reklamation korrigiert nicht");
    assert_eq!(lives(&g), TILL_LIVES);
    // R ohne Fehler: sie wird misstrauisch
    let before = doubt(&g);
    frame(&mut g, r);
    assert!((doubt(&g) - before - receipt::SUS_FALSE_COMPLAINT).abs() < 1e-3);
    // doppelt gescannt und nicht bemerkt: beim Bezahlen ist ein Leben weg
    g.debug_force_wrong_scan(true);
    assert!(g.scanned_value() > fair);
    look_at_terminal(&mut g);
    press_e(&mut g);
    assert_eq!(
        lives(&g),
        TILL_LIVES - 1,
        "nicht bemerkter Fehlscan kostet nichts"
    );
    assert!(g.swiping(), "Bezahlen startet danach nicht");
    assert_eq!(
        g.scanned_value(),
        fair,
        "Fehlscan nach dem Leben nicht korrigiert"
    );
    // Display und Summe passen immer zusammen
    let t = g.till.as_ref().unwrap();
    let shown: i64 = t
        .receipt
        .iter()
        .filter_map(|l| l.shown())
        .map(|l| l.1)
        .sum();
    assert_eq!(shown, g.scanned_value());
}

#[test]
fn someone_knocks_at_the_barrier() {
    let mut g = under_pressure(96);
    let mut other = Game::new(5);
    other.tip();
    let mut c = other.customers.remove(0);
    let (x0, x1, z0, _) = g.world.checkout_zone().unwrap();
    c.pos = g.world.collide(vec3((x0 + x1) * 0.5, 0.0, z0 - 0.6), 0.25);
    c.stalker = true;
    g.customers = vec![c];
    g.sfx.clear();
    let mut knocks = 0;
    for _ in 0..60 * 4 {
        g.update(1.0 / 60.0, &mut Input::default());
        knocks += g
            .sfx
            .drain(..)
            .filter(|(s, _)| matches!(s, Sfx::Knock))
            .count();
    }
    assert!(knocks > 0, "kein Klopfen am Absperrband");
}

/// Ware aus dem Katalog nach Namen (fuer Tests mit bestimmtem Gewicht).
fn product(name: &str) -> ItemData {
    let cat = crate::catalog::catalog();
    let i = cat.products.iter().position(|p| p.name == name).unwrap();
    let p = &cat.products[i];
    ItemData {
        name: p.name.clone(),
        cents: p.cents as i64,
        kind: PickupKind::Product {
            product: i,
            fake: false,
        },
        mesh: p.mesh,
        tint: p.tint,
    }
}

#[test]
fn weight_decides_what_goes_first_not_being_a_drink() {
    // Die Limo-Dose ist ein Getraenk, aber leicht; die Wassermelone ist schwer
    let can = product("Limo Dose");
    let melon = product("Wassermelone");
    assert!(!can.is_heavy() && melon.is_heavy());
    assert!(product("Mineralwasser 1,5L").is_heavy());
    assert!(!product("Schokoriegel").is_heavy());

    // Dose zuerst, dann Melone: ein Leben weg
    let mut g = at_till(41);
    g.cart = vec![can.clone(), melon.clone()];
    g.cart_sel = Some(0);
    belt_drop(&mut g);
    g.cart_sel = Some(0);
    belt_drop(&mut g);
    assert_eq!(lives(&g), TILL_LIVES - 1, "Melone nach Dose kostet nichts");

    // Melone zuerst, dann Dose: alles gut
    let mut g = at_till(42);
    g.cart = vec![melon, can];
    g.cart_sel = Some(0);
    belt_drop(&mut g);
    g.cart_sel = Some(0);
    belt_drop(&mut g);
    assert_eq!(lives(&g), TILL_LIVES);
}

#[test]
fn every_product_has_a_weight_and_the_window_shows_it() {
    for p in &crate::catalog::catalog().products {
        assert!(p.kg > 0.0 && p.kg < 10.0, "{}: {} kg", p.name, p.kg);
    }
    use crate::game::belt_window::weight_text;
    assert_eq!(weight_text(1.6), "1,6 kg");
    assert_eq!(weight_text(1.0), "1 kg");
    assert_eq!(weight_text(0.35), "350 g");
}

#[test]
fn tutorial_till_mistakes_only_explain() {
    // Tutorial-Modus: falsche Reihenfolge kostet kein Kassen-Leben, Misstrauen bleibt bei 0
    let mut g = at_till(43);
    g.tutorial_run = true;
    g.tutorial = Some(crate::game::tutorial::Tutorial::default());
    let other = g.cart.iter().position(|c| !c.is_heavy()).unwrap();
    g.cart_sel = Some(other);
    belt_drop(&mut g);
    let heavy = g.cart.iter().position(|c| c.is_heavy()).unwrap();
    g.cart_sel = Some(heavy);
    belt_drop(&mut g);
    assert_eq!(lives(&g), TILL_LIVES, "im Tutorial ein Leben weg");
    assert_eq!(doubt(&g), 0.0);
    assert_eq!(g.outcome, Outcome::Running);
    assert!(
        g.message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("Im echten Spiel")),
        "keine Erklaerung"
    );
}
