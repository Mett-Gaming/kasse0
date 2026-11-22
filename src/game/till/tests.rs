use super::super::tests::{
    look_at_belt, look_at_terminal, press_e, put_all_on_belt, scan_and_answer, shelf_item, step,
    swipe, type_pin,
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

fn doubt(g: &Game) -> f32 {
    g.till.as_ref().map_or(0.0, |t| t.doubt)
}

/// Bis zum Terminal: alles richtig aufs Band, scannen, Fragen gut beantworten.
fn ready_to_pay(seed: u64) -> Game {
    let mut g = at_till(seed);
    put_all_on_belt(&mut g);
    scan_and_answer(&mut g);
    assert!(g.scanned_complete());
    look_at_terminal(&mut g);
    press_e(&mut g);
    assert!(g.swiping(), "Durchziehen startet nicht");
    g
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
fn drinks_must_go_on_the_belt_first() {
    let mut g = at_till(32);
    // erst etwas anderes, dann das Wasser
    let other = g.cart.iter().position(|c| !c.is_drink()).unwrap();
    g.cart_sel = Some(other);
    press_e(&mut g);
    assert_eq!(doubt(&g), 0.0);
    let water = g.cart.iter().position(|c| c.is_drink()).unwrap();
    g.cart_sel = Some(water);
    press_e(&mut g);
    assert!((doubt(&g) - SUS_ORDER).abs() < 1e-4, "Reihenfolge egal");
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
fn swiping_too_fast_fails_and_slowly_works() {
    let mut g = ready_to_pay(36);
    swipe(&mut g, 0.15);
    assert!(
        (doubt(&g) - SUS_SWIPE).abs() < 1e-4,
        "zu schnell geht durch"
    );
    assert!(g.swiping(), "nach dem Fehler nicht erneut durchziehen");
    swipe(&mut g, 1.0);
    assert!(
        matches!(g.till.as_ref().unwrap().pay, Pay::ShowPin(_)),
        "langsam durchgezogen und nicht gelesen"
    );
    assert!((doubt(&g) - SUS_SWIPE).abs() < 1e-4);
}

#[test]
fn camera_stays_still_while_swiping() {
    let mut g = ready_to_pay(37);
    let (yaw, pitch) = (g.player.yaw, g.player.pitch);
    g.look(300.0, 80.0);
    assert_eq!((g.player.yaw, g.player.pitch), (yaw, pitch));
}

#[test]
fn wrong_pin_costs_doubt_right_pin_pays() {
    let mut g = ready_to_pay(38);
    swipe(&mut g, 1.0);
    step(&mut g, PIN_SHOW + 0.1, Input::default());
    let pin = g.till.as_ref().unwrap().pin;
    let mut inp = Input {
        digit: Some((pin[0] + 1) % 10),
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!((doubt(&g) - SUS_PIN).abs() < 1e-4, "falsche PIN egal");
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
