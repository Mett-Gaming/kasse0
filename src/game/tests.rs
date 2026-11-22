use super::*;

pub(super) fn step(g: &mut Game, secs: f32, input: Input) {
    let mut inp = input;
    for _ in 0..(secs * 60.0) as usize {
        g.update(1.0 / 60.0, &mut inp);
    }
}

/// Eine echte (nicht falsche, nicht genommene) Regalware fuer einen Zettel-Eintrag.
pub(super) fn shelf_item(g: &Game, entry: usize, pricey: bool) -> Option<usize> {
    let cat = catalog();
    g.world.items.iter().position(|it| {
        let p = &cat.products[it.product as usize];
        !it.fake && !it.taken && p.list == Some(entry) && p.pricey == pricey
    })
}

/// Spieler vor einen Punkt stellen und ihn anschauen.
fn look_at_point(g: &mut Game, t: Vec3) {
    for d in [1.0f32, 0.8, 1.2] {
        for (dx, dz) in [(d, 0.0f32), (-d, 0.0), (0.0, d), (0.0, -d)] {
            let stand = vec3(t.x + dx, 0.0, t.z + dz);
            if g.world.collide(stand, 0.3).distance(stand) < 1e-3 {
                g.player.pos = stand;
                let to = t - (stand + vec3(0.0, EYE_HEIGHT, 0.0));
                g.player.yaw = to.x.atan2(-to.z);
                g.player.pitch = (to.y / to.length()).asin();
                return;
            }
        }
    }
    panic!("kein Standplatz vor {t:?}");
}

#[test]
fn list_is_random_and_water_waits_in_drinks_room() {
    let mut lists = std::collections::HashSet::new();
    for seed in 0..30 {
        let g = Game::new(seed);
        assert_eq!(g.list[0], g.water_entry());
        assert!(g.list.len() >= 5 && g.list.len() <= 7);
        let mut uniq = g.list.clone();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), g.list.len(), "doppelter Zettel-Eintrag");
        for &e in &g.list {
            assert!(
                shelf_item(&g, e, false).is_some(),
                "billige Ware fuer {e} fehlt"
            );
        }
        let w = shelf_item(&g, g.water_entry(), false).unwrap();
        let _ = w;
        assert!(
            g.world
                .items
                .iter()
                .any(
                    |it| catalog().products[it.product as usize].list == Some(g.water_entry())
                        && g.world.drinks_zone.contains_xz(it.pos)
                ),
            "Wasser nicht im Getraenkeraum"
        );
        lists.insert(g.list.clone());
    }
    assert!(lists.len() > 20, "Zettel kaum zufaellig");
}

#[test]
fn leaving_drinks_room_tips_the_market_and_spawns_npcs() {
    let mut g = Game::new(42);
    let zone = g.world.drinks_zone;
    let door_x = zone.min.x + 1.7;
    g.player.pos = vec3(door_x, 0.0, zone.max.z - 0.8);
    step(&mut g, 0.1, Input::default());
    assert!(!g.tipped);
    g.player.pos = vec3(door_x, 0.0, zone.max.z + 1.0);
    step(&mut g, 0.1, Input::default());
    assert!(g.tipped);
    assert_eq!(g.customers.len(), Tier::Calm.customers());
    assert_eq!(g.employees.len(), Tier::Calm.employees());
    assert!(g.shoppers.is_empty());
}

#[test]
fn cart_value_drives_escalation() {
    let mut g = Game::new(7);
    g.tip();
    assert_eq!(g.tier, Tier::Calm);
    let len_before = g.world.aisle_z1 - g.world.aisle_z0;
    // teure Waren nehmen, bis die Stufe steigt
    let entries: Vec<usize> = (0..catalog().list.len()).collect();
    for e in entries {
        if g.cart_value() >= 1000 {
            break;
        }
        if let Some(i) = shelf_item(&g, e, true) {
            g.take(i);
        }
    }
    assert!(g.cart_value() >= 1000);
    assert!(g.tier >= Tier::Uneasy);
    assert!(
        g.world.aisle_z1 - g.world.aisle_z0 > len_before,
        "Gaenge nicht verlaengert"
    );
    assert!(g.customers.len() >= Tier::Uneasy.customers());
}

#[test]
fn stalker_acts_normal_when_watched_and_creeps_when_not() {
    let mut g = Game::new(99);
    g.tip();
    g.customers.truncate(1);
    g.employees.clear();
    // Verfolger 6 m vor den Spieler in den freien Gang stellen, "zu Hause" am Regal
    let p = g.player.pos;
    let c = &mut g.customers[0];
    c.pos = vec3(p.x, 0.0, p.z - 6.0);
    c.home = c.pos;
    c.stalker = true;
    c.field.clear();
    c.wait = 100.0;
    g.player.yaw = 0.0; // Blick nach Norden
    g.player.pitch = 0.0;
    let dist0 = (g.customers[0].pos - g.player.pos).length();
    step(&mut g, 2.0, Input::default());
    assert!(
        (g.customers[0].pos - g.player.pos).length() > dist0 - 0.05,
        "kommt trotz Blickkontakt naeher"
    );
    assert_eq!(
        g.customers[0].pose,
        npc::Pose::Reach,
        "kauft nicht normal ein"
    );
    // wegsehen
    g.player.yaw = std::f32::consts::PI;
    step(&mut g, 2.0, Input::default());
    assert!(
        (g.customers[0].pos - g.player.pos).length() < dist0 - 0.3,
        "kommt nicht naeher"
    );
}

#[test]
fn caught_when_stalker_reaches_player() {
    // Die ersten beiden Male ueberlebt man, beim dritten Mal ist Schluss
    let mut g = Game::new(5);
    g.tip();
    g.employees.clear();
    g.customers.truncate(1);
    g.customers[0].stalker = true;
    g.trolley.pushing = false;
    for hit in 1..=3 {
        let p = g.player.pos;
        g.customers[0].pos = vec3(p.x, 0.0, p.z - 1.5);
        g.customers[0].home = g.customers[0].pos + vec3(0.0, 0.0, -30.0);
        g.player.yaw = std::f32::consts::PI; // wegsehen
        g.grace = 0.0;
        let cart = g.trolley.pos;
        step(&mut g, 6.0, Input::default());
        if hit < 3 {
            assert_eq!(
                g.outcome,
                Outcome::Running,
                "Treffer {hit}: Run schon vorbei"
            );
            assert_eq!(g.deaths, hit);
            assert!(g.dying.is_none(), "Todesanimation laeuft noch");
            assert!(
                (g.player.pos - g.world.spawn).length() < 0.5,
                "nicht am Eingang aufgewacht"
            );
            assert_eq!(g.trolley.pos, cart, "Wagen hat sich bewegt");
            assert!(g.death_vignette() > 0.0);
            // Schonfrist abwarten
            step(&mut g, 5.5, Input::default());
        } else {
            assert_eq!(g.outcome, Outcome::Caught);
        }
    }
}

#[test]
fn ordinary_customers_only_shop_and_never_catch() {
    let mut g = Game::new(5);
    g.tip();
    g.employees.clear();
    g.customers.truncate(1);
    let p = g.player.pos;
    g.customers[0].pos = vec3(p.x, 0.0, p.z - 1.5);
    g.customers[0].stalker = false;
    g.player.yaw = std::f32::consts::PI; // wegsehen
    let start = g.customers[0].pos;
    step(&mut g, 8.0, Input::default());
    assert_eq!(g.outcome, Outcome::Running);
    // unbeobachtet: bleibt stehen und starrt herueber
    let c = &g.customers[0];
    assert!((c.pos - start).length() < 0.05, "kommt naeher");
    let to_player = npc::yaw_towards(c.pos, g.player.pos);
    assert!(
        lerp_angle(0.0, c.yaw - to_player, 1.0).abs() < 0.3,
        "starrt nicht herueber"
    );
    // hinsehen: kauft wieder ein
    g.player.yaw = 0.0;
    step(&mut g, 8.0, Input::default());
    assert_eq!(g.outcome, Outcome::Running);
    assert!(
        (g.customers[0].pos - start).length() > 0.3,
        "bummelt nicht herum"
    );
}

#[test]
fn escalation_sets_the_number_of_stalkers() {
    let mut g = Game::new(21);
    g.tip();
    let stalkers = g.customers.iter().filter(|c| c.stalker).count();
    assert_eq!(stalkers, Tier::Calm.stalkers().min(g.customers.len()));
    assert!(g.customers.len() > stalkers, "alle verfolgen einen");
}

/// Von `stand` aus auf `target` schauen.
fn look_from(g: &mut Game, stand: Vec3, target: Vec3) {
    g.player.pos = stand;
    let to = target - (stand + vec3(0.0, EYE_HEIGHT, 0.0));
    g.player.yaw = to.x.atan2(-to.z);
    g.player.pitch = (to.y / to.length()).asin();
}

pub(super) fn press_e(g: &mut Game) {
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
}

/// Spieler an die offene Kasse, Blick aufs Band.
pub(super) fn look_at_belt(g: &mut Game) {
    let lane = g.open_lane().unwrap();
    let mid = (lane.belt_start + lane.belt_end) * 0.5;
    look_from(g, lane.scan_point, mid);
    g.update(1.0 / 60.0, &mut Input::default());
}

/// Den ganzen Wagen richtig aufs Band legen: Getraenke zuerst (Wagen steht an der Kasse).
pub(super) fn put_all_on_belt(g: &mut Game) {
    look_at_belt(g);
    while !g.cart.is_empty() {
        let drink = g.cart.iter().position(|c| c.is_drink());
        g.cart_sel = Some(drink.unwrap_or(g.cart.len() - 1));
        press_e(g);
    }
}

/// Eine Frage der Kassiererin gut beantworten, falls eine offen ist.
pub(super) fn answer_well(g: &mut Game) {
    let k = g
        .till
        .as_ref()
        .and_then(|t| t.asked.as_ref())
        .filter(|a| a.reply.is_none())
        .map(|a| a.order.iter().position(|&o| o == 0).unwrap());
    let mut inp = Input {
        choice: k,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
}

/// Scannen lassen und alle Fragen gut beantworten, bis nichts mehr offen ist.
pub(super) fn scan_and_answer(g: &mut Game) {
    for _ in 0..60 * 90 {
        answer_well(g);
        let t = g.till.as_ref().unwrap();
        if g.belt.is_empty() && t.asked.is_none() && !t.questions_left() {
            break;
        }
    }
}

/// Karte aus dem Portemonnaie nehmen (falls noch nicht) und gleichmaessig nach rechts
/// durchziehen; `secs` = wie lange das Durchziehen dauert.
pub(super) fn swipe(g: &mut Game, secs: f32) {
    let in_wallet = matches!(
        g.till.as_ref().map(|t| t.pay),
        Some(till::Pay::Swipe { ready: false, .. })
    );
    if in_wallet {
        press_e(g);
    }
    let frames = (secs * 60.0) as usize;
    let per_rad = 1.0 / (BASE_SENSITIVITY * g.sensitivity) * till::SWIPE_TRAVEL;
    for _ in 0..frames + 2 {
        g.look(per_rad / frames as f32, 0.0);
        g.update(1.0 / 60.0, &mut Input::default());
    }
}

/// Angezeigte PIN abwarten und eintippen.
pub(super) fn type_pin(g: &mut Game) {
    step(g, till::PIN_SHOW + 0.1, Input::default());
    let pin = g.till.as_ref().unwrap().pin;
    for d in pin {
        let mut inp = Input {
            digit: Some(d),
            ..Default::default()
        };
        g.update(1.0 / 60.0, &mut inp);
    }
}

pub(super) fn look_at_terminal(g: &mut Game) {
    let lane = g.open_lane().unwrap();
    look_from(g, lane.scan_point, lane.terminal + vec3(0.0, 0.08, 0.0));
    g.update(1.0 / 60.0, &mut Input::default());
}

#[test]
fn full_run_checkout_and_exit_wins() {
    let mut g = Game::new(12);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    for e in g.list.clone() {
        let i = shelf_item(&g, e, false).expect("billige Ware fehlt");
        g.take(i);
    }
    g.customers.clear();
    g.employees.clear();
    assert!(g.list_complete());
    let lane = g.open_lane().unwrap();
    // Wagen steht noch weit weg: Ausraeumen geht nicht
    g.trolley.pushing = false;
    g.trolley.pos = g.world.spawn;
    look_at_belt(&mut g);
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Belt));
    press_e(&mut g);
    assert!(
        g.belt.is_empty() && g.till.is_none(),
        "Ausraeumen ohne Wagen"
    );
    // Wagen zur Kasse, Stueck fuer Stueck aufs Band (Getraenke zuerst)
    let n = g.cart.len();
    g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
    put_all_on_belt(&mut g);
    assert!(g.cart.is_empty(), "Wagen nicht ausgeraeumt");
    assert!(g.till_active(), "Kassen-Minispiel startet nicht");
    g.till.as_mut().unwrap().cash = false; // Kartenzahlung (bar: till/tests.rs)
    scan_and_answer(&mut g);
    assert!(g.belt.is_empty(), "Band nicht leer");
    assert_eq!(g.scanned.len(), n);
    assert!(g.scanned_complete());
    assert!(!g.paid, "ohne Terminal bezahlt");
    // am Terminal: Karte ruhig durchziehen, PIN eintippen
    look_at_terminal(&mut g);
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Terminal));
    press_e(&mut g);
    swipe(&mut g, 1.0);
    type_pin(&mut g);
    assert!(g.paid, "nicht bezahlt");
    assert_eq!(g.till.as_ref().unwrap().lives, till::TILL_LIVES);
    // durch die Tuer in den Fluchtkorridor ...
    g.customers.clear();
    g.employees.clear();
    g.player.pos = g.world.door_center - vec3(0.0, 0.0, 0.8);
    g.player.yaw = std::f32::consts::PI; // nach Sueden
    g.player.pitch = 0.0;
    step(
        &mut g,
        2.0,
        Input {
            forward: true,
            ..Default::default()
        },
    );
    assert!(g.in_corridor(), "nicht im Korridor angekommen");
    assert_eq!(g.outcome, Outcome::Running);
    // ... bis kurz vor den Ausgang am Ende, dann links hinaus
    let exit = g.world.exit_zone;
    let c = g.world.corridor.unwrap();
    g.player.pos = vec3(c.x1 - 0.5, 0.0, (exit.min.z + exit.max.z) * 0.5);
    g.player.yaw = std::f32::consts::FRAC_PI_2; // nach +x
    step(
        &mut g,
        3.0,
        Input {
            forward: true,
            ..Default::default()
        },
    );
    assert_eq!(g.outcome, Outcome::Won);
}

#[test]
fn grab_carry_and_place_into_parked_cart() {
    let mut g = Game::new(31);
    let water = shelf_item(&g, g.water_entry(), false).unwrap();
    // Wagen stehen lassen
    let mut inp = Input {
        cart_toggle: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(!g.trolley.pushing);
    // zum Wasser gehen und greifen
    let it = g.world.items[water];
    look_at_point(&mut g, it.pos + vec3(0.0, it.half().y, 0.0));
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Shelf(water)));
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.held.is_some(), "nicht gegriffen");
    assert!(g.world.items[water].taken, "Ware steht noch im Regal");
    assert!(g.take_statics_dirty());
    assert!(g.cart.is_empty());
    // zurueck zum Wagen, ihn anschauen, ablegen
    let cart = g.trolley.pos;
    g.player.pos = cart + vec3(0.0, 0.0, 1.2);
    let to = cart + vec3(0.0, 0.5, 0.0) - (g.player.pos + vec3(0.0, EYE_HEIGHT, 0.0));
    g.player.yaw = to.x.atan2(-to.z);
    g.player.pitch = (to.y / to.length()).asin();
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.held.is_none());
    assert_eq!(g.cart.len(), 1);
    assert!(g.list_done(g.water_entry()));
    // wieder herausnehmen
    let mut inp = Input {
        take_out: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.held.is_some() && g.cart.is_empty());
}

#[test]
fn dropped_item_falls_and_can_be_picked_up() {
    let mut g = Game::new(4);
    g.trolley.pushing = false;
    g.trolley.pos = g.world.spawn + vec3(3.0, 0.0, 0.0);
    g.held = Some(ItemData::from_shelf(&g.world.items[0]));
    g.player.pitch = 0.0;
    g.player.yaw = 0.0;
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.held.is_none());
    assert_eq!(g.loose.len(), 1);
    step(&mut g, 2.0, Input::default());
    assert!(
        g.loose[0].pos.y.abs() < 1e-3,
        "liegt nicht am Boden: {:?}",
        g.loose[0].pos
    );
    assert_eq!(g.loose[0].vel, Vec3::ZERO);
    // anschauen und aufheben
    let target = g.loose[0].pos + vec3(0.0, 0.05, 0.0);
    let to = target - g.eye();
    g.player.pitch = (to.y / to.length()).asin();
    g.player.yaw = to.x.atan2(-to.z);
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.held.is_some() && g.loose.is_empty());
}

#[test]
fn employee_tidies_up_unattended_cart_to_corral() {
    let mut g = Game::new(14);
    g.tip();
    g.scare_cd = 1e6; // kein Geisterwagen dazwischen
    g.customers.clear();
    g.employees.clear();
    // Wagen irgendwo im Markt stehen lassen, Spieler weit weg
    g.trolley.pushing = false;
    g.trolley.pos = g.world.lanes[0].scan_point + vec3(0.0, 0.0, -2.5);
    g.player.pos = vec3(
        g.world.drinks_zone.min.x + 1.7,
        0.0,
        g.world.drinks_zone.max.z - 0.8,
    );
    let mut e = npc::Employee::new(g.world.collide(g.trolley.pos + vec3(1.0, 0.0, 0.0), 0.3));
    e.ignore = 100.0; // interessiert sich nicht fuer den Spieler
    g.employees.push(e);
    step(&mut g, 0.2, Input::default());
    assert!(g.employees[0].carrying, "nimmt den Wagen nicht");
    step(&mut g, 60.0, Input::default());
    assert!(!g.employees[0].carrying, "kommt nicht an der Wagenbox an");
    assert!((g.trolley.pos - g.world.corral).length() < 0.01);
}

#[test]
fn talking_to_carrier_releases_cart() {
    let mut g = Game::new(15);
    g.cheat(FIRST_SCARE_CHEAT - 1); // Mitarbeiter nimmt den Wagen mit
    g.customers.clear();
    assert!(g.employees.last().unwrap().carrying);
    let i = g.employees.len() - 1;
    g.employees.truncate(i + 1);
    // anschauen und ansprechen
    let ep = g.employees[i].pos;
    g.player.pos = g.world.collide(ep + vec3(0.0, 0.0, 1.6), 0.3);
    let to = ep + vec3(0.0, 1.0, 0.0) - g.eye();
    g.player.yaw = to.x.atan2(-to.z);
    g.player.pitch = (to.y / to.length()).asin();
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Employee(i)));
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    let d = g.dialog.as_ref().expect("kein Gespraech");
    assert!(d.cart_talk);
    let good = d.order.iter().position(|&a| a == 0).unwrap();
    let mut inp = Input {
        choice: Some(good),
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    step(&mut g, 2.0, Input::default());
    assert!(g.dialog.is_none());
    assert!(!g.employees[i].carrying, "laesst den Wagen nicht los");
}

#[test]
fn safe_during_dialog_and_shortly_after() {
    let mut g = Game::new(16);
    g.tip();
    g.customers.clear();
    g.employees.truncate(1);
    g.employees[0].ignore = 0.0;
    g.employees[0].pos = g.player.pos + vec3(0.0, 0.0, -1.4);
    step(&mut g, 0.2, Input::default());
    assert!(g.dialog.is_some());
    // Kunde direkt hinter dem Spieler
    g.spawn_customer_at(g.player.pos + vec3(0.0, 0.0, 0.5));
    step(&mut g, 1.0, Input::default());
    assert_eq!(g.outcome, Outcome::Running, "im Gespraech erwischt");
}

#[test]
fn sprinting_past_employee_avoids_dialog_but_raises_suspicion() {
    let mut g = Game::new(18);
    g.tip();
    g.customers.clear();
    g.employees.truncate(1);
    g.employees[0].ignore = 0.0;
    g.employees[0].pos = g.player.pos + vec3(0.0, 0.0, -1.4);
    g.player.vel = vec3(0.0, 0.0, -3.0);
    let before = g.suspicion;
    let mut inp = Input {
        forward: true,
        run: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.dialog.is_none(), "Gespraech trotz Sprint");
    assert!(g.suspicion > before + 0.15);
}

#[test]
fn pushed_cart_does_not_fit_through_passage_but_player_does() {
    // Durchgang suchen
    for seed in 0..30 {
        let mut g = Game::new(seed);
        g.tip();
        g.params.warp = 1.0;
        g.rebuild_world();
        g.customers.clear();
        g.employees.clear();
        let Some(&(c, n)) = g.world.passages.first() else {
            continue;
        };
        // quer durch den Durchgang laufen: Start 1,5 m davor im Gang
        let start = c - n * 1.5;
        if g.world.collide(start, 0.45).distance(start) > 1e-3 {
            continue;
        }
        let walk = |g: &mut Game, push: bool| {
            g.player.pos = start;
            g.player.yaw = n.x.atan2(-n.z);
            g.player.pitch = 0.0;
            g.player.vel = Vec3::ZERO;
            g.trolley.pushing = push;
            g.trolley.pos = if push {
                start + n * hands::CART_OFFSET
            } else {
                start - n * 3.0
            };
            let mut inp = Input {
                forward: true,
                ..Default::default()
            };
            for _ in 0..240 {
                g.update(1.0 / 60.0, &mut inp);
            }
            (g.player.pos - c).dot(n)
        };
        let foot = walk(&mut g, false);
        let cart = walk(&mut g, true);
        assert!(
            foot > 0.8,
            "zu Fuss nicht durchgekommen (seed {seed}): {foot}"
        );
        assert!(cart < -0.3, "Wagen passt durch (seed {seed}): {cart}");
        return;
    }
    panic!("kein Durchgang gefunden");
}

#[test]
fn tutorial_ends_when_market_tips() {
    let mut g = Game::new(2);
    g.tutorial = Some(tutorial::Tutorial::default());
    g.tip();
    assert!(g.tutorial.is_none());
}

#[test]
fn incomplete_list_blocks_payment() {
    let mut g = Game::new(3);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    // nur das Wasser einpacken (Zettel unvollstaendig)
    let w = shelf_item(&g, g.water_entry(), false).unwrap();
    g.take(w);
    let lane = g.open_lane().unwrap();
    g.trolley.pushing = false;
    g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
    look_at_belt(&mut g);
    press_e(&mut g);
    step(&mut g, 8.0, Input::default());
    assert_eq!(g.scanned.len(), 1);
    look_at_terminal(&mut g);
    press_e(&mut g);
    assert!(!g.paid);
}

#[test]
fn held_item_goes_on_belt_and_fakes_are_rejected() {
    let mut g = Game::new(8);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    let i = shelf_item(&g, g.water_entry(), false).unwrap();
    let mut fake = ItemData::from_shelf(&g.world.items[i]);
    fake.kind = PickupKind::Product {
        product: g.world.items[i].product as usize,
        fake: true,
    };
    g.trolley.pushing = false;
    g.trolley.pos = g.world.corral;
    g.held = Some(fake);
    look_at_belt(&mut g);
    press_e(&mut g);
    assert!(g.held.is_none());
    assert_eq!(g.belt.len(), 1, "nicht aufs Band gelegt");
    let before = g.suspicion;
    step(&mut g, 7.0, Input::default());
    assert!(g.belt.is_empty());
    assert!(g.scanned.is_empty(), "falsche Ware wurde gescannt");
    // An der Kasse merkt sich das die Kassiererin (nicht die Kundschaft)
    assert_eq!(g.till.as_ref().unwrap().lives, till::TILL_LIVES - 1);
    assert!(g.suspicion <= before);
    // Wertstufe zaehlt Band und Gescanntes mit
    g.held = Some(ItemData::from_shelf(&g.world.items[i]));
    press_e(&mut g);
    assert!(g.cart_value() > 0);
}

#[test]
fn bad_dialog_answer_raises_suspicion() {
    let mut g = Game::new(8);
    g.tip();
    g.customers.clear();
    let e = &mut g.employees[0];
    e.ignore = 0.0;
    e.pos = g.player.pos + vec3(0.0, 0.0, -1.4);
    step(&mut g, 0.2, Input::default());
    let d = g.dialog.as_ref().expect("Mitarbeiter spricht nicht an");
    let bad_k = d.order.iter().position(|&a| a == 2).unwrap();
    let before = g.suspicion;
    let mut inp = Input {
        choice: Some(bad_k),
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert!(g.suspicion > before + 0.2);
    step(&mut g, 2.0, Input::default());
    assert!(g.dialog.is_none());
    assert!(
        g.employees[0].follow > 0.0,
        "folgt nach schlechter Antwort nicht"
    );
}

#[test]
fn taking_tomato_juice_angers_its_customer() {
    let mut g = Game::new(17);
    g.tip();
    let i = g
        .pickups
        .iter()
        .position(|p| matches!(p.data.kind, PickupKind::Tomato { .. }))
        .expect("kein Tomatensaft");
    let PickupKind::Tomato { customer } = g.pickups[i].data.kind else {
        unreachable!()
    };
    g.take_pickup(i);
    assert!(g.customers[customer].angry > 0.0);
    assert!(g.suspicion >= 0.35);
}

#[test]
fn chosen_item_comes_out_of_the_cart() {
    let mut g = Game::new(31);
    g.customers.clear();
    g.employees.clear();
    // drei verschiedene Waren in den Wagen
    let cat = catalog();
    for p in [0usize, 5, 9] {
        let prod = &cat.products[p];
        g.cart.push(ItemData {
            name: prod.name.clone(),
            cents: prod.cents as i64,
            kind: PickupKind::Product {
                product: p,
                fake: false,
            },
            mesh: prod.mesh,
            tint: prod.tint,
        });
    }
    g.trolley.pushing = false;
    let cart = g.trolley.pos;
    // die erste Ware im Korb anvisieren
    let half = g.cart[0].half().max(Vec3::splat(0.05));
    let target = g
        .cart_item_model(0)
        .transform_point3(vec3(0.0, half.y, 0.0));
    let dir = vec3(target.x - cart.x, 0.0, target.z - cart.z).normalize_or_zero();
    let stand = g.world.collide(vec3(cart.x, 0.0, cart.z) + dir * 0.9, 0.3);
    look_from(&mut g, stand, target);
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Cart));
    assert_eq!(g.cart_selected(), Some(0), "angezielte Ware nicht gewaehlt");
    // Mausrad: eine weiter
    let mut inp = Input {
        wheel: -1,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    let chosen = g.cart_selected().unwrap();
    let name = g.cart[chosen].name.clone();
    let mut inp = Input {
        take_out: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
    assert_eq!(g.held.as_ref().map(|h| h.name.clone()), Some(name));
    assert_eq!(g.cart.len(), 2);
}

#[test]
fn npcs_walk_through_narrow_passages() {
    let mut checked = 0;
    for seed in 0..20 {
        let mut p = world::WorldParams::normal(seed);
        p.tipped = true;
        p.stretch = 2.0;
        p.warp = 1.0;
        let w = world::generate(&p);
        for &(c, n) in &w.passages {
            let a = w.collide(c + n * 1.1, 0.25);
            let b = w.collide(c - n * 1.1, 0.25);
            if (a - b).length() < 1.8 {
                continue; // Seite blockiert (z. B. Wand), nicht aussagekraeftig
            }
            let field = npc::bfs(&w.nav, b);
            let mut pos = a;
            // 2,2 m bei 1 m/s: mit Durchgang in 5 s, aussen herum nie
            for _ in 0..(5.0 * 60.0) as usize {
                pos = npc::step_along(&w, &field, pos, b, 1.0, 1.0 / 60.0);
            }
            assert!(
                (pos - b).length() < 0.6,
                "seed {seed}: NPC kommt nicht durch den Durchgang bei {c:?} (steht bei {pos:?})"
            );
            checked += 1;
        }
    }
    assert!(checked > 3, "kaum Durchgaenge geprueft: {checked}");
}

#[test]
fn stalkers_catch_up_faster_when_you_run() {
    let travelled = |run: bool| {
        let mut g = Game::new(99);
        g.tip();
        g.customers.truncate(1);
        g.employees.clear();
        let p = g.player.pos;
        let c = &mut g.customers[0];
        c.pos = vec3(p.x, 0.0, p.z - 8.0);
        c.stalker = true;
        g.player.yaw = std::f32::consts::PI; // wegsehen
        if run {
            g.player.vel = vec3(0.0, 0.0, 3.0);
        }
        let d0 = (g.customers[0].pos - g.player.pos).length();
        for _ in 0..60 {
            if run {
                g.player.vel = vec3(0.0, 0.0, 3.0); // rennt auf der Stelle (Geschwindigkeit zaehlt)
            }
            g.player.pos = p;
            g.update(1.0 / 60.0, &mut Input::default());
        }
        d0 - (g.customers[0].pos - g.player.pos).length()
    };
    let (walk, run) = (travelled(false), travelled(true));
    assert!(
        run > walk * 1.3,
        "beim Rennen nicht schneller: {walk} vs {run}"
    );
    assert!(run < walk * 1.8, "beim Rennen zu schnell: {walk} vs {run}");
}

#[test]
fn checkout_is_closed_until_the_list_is_complete() {
    let mut g = Game::new(12);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    g.trolley.pushing = false;
    g.trolley.pos = g.world.corral;
    let lane = g.open_lane().unwrap();
    let (_, _, z0, _) = g.checkout_barrier().expect("keine Absperrung");
    // von Norden in die Kassengasse laufen
    let walk_in = |g: &mut Game| {
        g.player.pos = g.world.collide(vec3(lane.scan_point.x, 0.0, z0 - 1.5), 0.3);
        g.player.yaw = std::f32::consts::PI; // nach Sueden
        g.player.pitch = 0.0;
        step(
            g,
            3.0,
            Input {
                forward: true,
                ..Default::default()
            },
        );
    };
    walk_in(&mut g);
    assert!(
        g.player.pos.z < z0,
        "trotz Absperrung in der Kasse: {:?}",
        g.player.pos
    );
    // Zettel komplett: Absperrung weg
    g.fill_list_cheap();
    assert!(g.checkout_barrier().is_none());
    walk_in(&mut g);
    assert!(
        g.player.pos.z > z0 + 0.5,
        "kommt nicht in die Kasse: {:?}",
        g.player.pos
    );
}

#[test]
fn customers_whisper_when_they_meet() {
    let mut g = Game::new(7);
    g.tip();
    g.employees.clear();
    g.customers.truncate(2);
    let p = g.player.pos;
    for (k, c) in g.customers.iter_mut().enumerate() {
        c.pos = vec3(p.x + k as f32 * 0.8, 0.0, p.z - 3.0);
        c.stalker = false;
        c.whisper_cd = 0.0;
    }
    g.sfx.clear();
    g.update(1.0 / 60.0, &mut Input::default());
    assert!(
        g.sfx.iter().any(|(s, _)| matches!(s, Sfx::Whisper)),
        "kein Fluestern"
    );
    // danach eine Weile Ruhe
    g.sfx.clear();
    step(&mut g, 2.0, Input::default());
    assert!(!g.sfx.iter().any(|(s, _)| matches!(s, Sfx::Whisper)));
}

#[test]
fn cheap_shopping_and_time_still_reach_the_late_stages() {
    // Nur billige Ware: der Zettel-Fortschritt treibt die Stufe bis zum Kollaps
    let mut g = Game::new(12);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    let mut seen = vec![g.tier];
    for e in g.list.clone() {
        if g.list_done(e) {
            continue;
        }
        let i = shelf_item(&g, e, false).expect("billige Ware fehlt");
        g.take(i);
        seen.push(g.tier);
    }
    assert!(
        g.cart_value() < 2500,
        "Test braucht billigen Einkauf: {}",
        g.cart_value()
    );
    assert_eq!(g.tier, Tier::Collapse, "Zettel komplett, aber kein Kollaps");
    assert!(
        seen.contains(&Tier::Uneasy) || seen.contains(&Tier::Wrong),
        "keine Zwischenstufe"
    );
    // Ohne Einkauf: die Zeit allein hebt die Stufe an
    let mut g = Game::new(13);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    g.tipped_time_add(400.0);
    g.update(1.0 / 60.0, &mut Input::default());
    assert!(
        g.tier >= Tier::Wrong,
        "Zeit hebt die Stufe nicht an: {:?}",
        g.tier
    );
}

#[test]
fn escalation_tables_are_monotonic() {
    let mut last = Tier::Calm;
    for done in 0..=7 {
        let t = escalation::tier_for_progress(done, 7);
        assert!(t >= last);
        last = t;
    }
    assert_eq!(escalation::tier_for_progress(7, 7), Tier::Collapse);
    assert_eq!(escalation::tier_for_time(0.0), Tier::Calm);
    assert_eq!(escalation::tier_for_time(600.0), Tier::Collapse);
}

/// Gekippter Markt auf Stufe `tier`, ohne Abklingzeit fuer Schrecken.
fn scary_game(seed: u64, tier: Tier) -> Game {
    let mut g = Game::new(seed);
    g.tip();
    g.employees.clear();
    g.tier = tier;
    g.scare_cd = 0.0;
    g
}

fn scared(g: &Game) -> bool {
    g.sfx.iter().any(|(s, _)| matches!(s, Sfx::Scare))
}

#[test]
fn stalker_appears_behind_you_when_distracted() {
    let mut g = scary_game(42, Tier::Wrong);
    g.customers.truncate(1);
    let p = g.player.pos;
    g.customers[0].stalker = true;
    // weit weg und ausser Sicht (Spieler schaut nach Norden, Verfolger steht im Sueden)
    g.customers[0].pos = g.world.collide(vec3(p.x, 0.0, p.z + 12.0), 0.25);
    g.player.yaw = 0.0;
    g.player.pitch = 0.0;
    let inp = Input {
        list_open: true,
        ..Default::default()
    };
    for _ in 0..(40 * 60) {
        g.player.pos = p;
        g.player.yaw = 0.0;
        let mut i = inp;
        g.update(1.0 / 60.0, &mut i);
        if g.behind.is_some() {
            break;
        }
    }
    assert!(g.behind.is_some(), "kein Verfolger hinter dem Spieler");
    let c = &g.customers[0];
    assert!(
        (c.pos - g.player.pos).length() < 2.0,
        "steht nicht dicht dahinter"
    );
    assert!(g.hush() > 0.0, "keine Stille davor");
    // umdrehen: Schreck, aber kein Fang
    g.sfx.clear();
    g.player.yaw = std::f32::consts::PI;
    step(&mut g, 0.5, Input::default());
    assert!(scared(&g), "kein Schreck beim Umdrehen");
    assert_eq!(g.outcome, Outcome::Running);
    assert_eq!(g.deaths, 0, "beim Umdrehen gleich erwischt");
}

#[test]
fn cashier_stands_up_after_the_last_item() {
    let mut g = Game::new(12);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    for e in g.list.clone() {
        if !g.list_done(e) {
            let i = shelf_item(&g, e, false).unwrap();
            g.take(i);
        }
    }
    g.customers.clear();
    g.employees.clear();
    let lane = g.open_lane().unwrap();
    g.trolley.pushing = false;
    g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
    put_all_on_belt(&mut g);
    g.sfx.clear();
    // scannen lassen, Fragen der Kassiererin gut beantworten
    for _ in 0..60 * 90 {
        if g.cashier_standing() {
            break;
        }
        answer_well(&mut g);
    }
    assert!(g.cashier_standing(), "Kassiererin steht nicht auf");
    assert!(scared(&g));
    let mut people = Vec::new();
    g.collect_scare_people(&mut people);
    assert!(
        !people.is_empty(),
        "stehende Kassiererin wird nicht gezeichnet"
    );
}

#[test]
fn parked_cart_vanishes_and_rolls_back_towards_you() {
    let mut g = scary_game(31, Tier::Uneasy);
    g.customers.clear();
    g.trolley.pushing = false;
    let p = g.player.pos;
    // Wagen weit weg und ausser Sicht (hinter dem Spieler)
    g.trolley.pos = g.world.collide(vec3(p.x, 0.0, p.z - 9.0), 0.4);
    g.player.yaw = std::f32::consts::PI;
    let mut moved = false;
    for _ in 0..(20 * 60) {
        g.player.pos = p;
        g.update(1.0 / 60.0, &mut Input::default());
        if g.cart_ghost != scares::CartGhost::Idle {
            moved = true;
            break;
        }
    }
    assert!(moved, "Wagen verschwindet nicht");
    let far = (g.trolley.pos - p).length();
    assert!(far > 10.0, "Wagen nicht weit weg: {far}");
    // rollt heran und bleibt in Reichweite stehen
    for _ in 0..(90 * 60) {
        g.player.pos = p;
        g.update(1.0 / 60.0, &mut Input::default());
        if g.cart_ghost == scares::CartGhost::Idle {
            break;
        }
    }
    let d = (g.trolley.pos - p).length();
    assert!(d < 3.5, "Wagen rollt nicht heran: {d}");
}

#[test]
fn ceiling_hole_drops_cables_when_you_pass_under_it() {
    let mut g = scary_game(5, Tier::Wrong);
    g.customers.clear();
    let mut p = WorldParams::normal(5);
    p.tipped = true;
    p.stretch = 2.0;
    p.ceiling_holes = 4;
    p.decay = 2;
    g.world = world::generate(&p);
    let h = *g.world.ceiling_holes.first().expect("keine Deckenloecher");
    g.player.pos = g.world.collide(vec3(h.x, 0.0, h.z), 0.3);
    g.sfx.clear();
    g.update(1.0 / 60.0, &mut Input::default());
    assert!(!g.cable_drops.is_empty(), "keine Kabel gefallen");
    assert!(scared(&g));
    let mut draws = Vec::new();
    g.collect_scare_draws(&mut draws);
    assert!(!draws.is_empty());
}

#[test]
fn death_shows_the_face_up_close() {
    let mut g = scary_game(5, Tier::Calm);
    g.customers.truncate(1);
    let p = g.player.pos;
    g.customers[0].pos = vec3(p.x, 0.0, p.z - 1.5);
    g.customers[0].stalker = true;
    g.player.yaw = std::f32::consts::PI;
    for _ in 0..(6 * 60) {
        g.update(1.0 / 60.0, &mut Input::default());
        if g.dying.is_some() {
            break;
        }
    }
    assert!(g.dying.is_some(), "nicht erwischt");
    let mut people = Vec::new();
    g.collect_scare_people(&mut people);
    assert_eq!(people.len(), 1, "kein Gesicht in Grossaufnahme");
    assert!((people[0].pos - g.eye()).length() < 2.0);
}

/// Spieler in den Kassengang stellen, Blick auf den Ueberwachungsmonitor.
fn watch_monitor(g: &mut Game) -> world::Lane {
    let lane = g.open_lane().unwrap();
    g.player.pos = vec3(lane.scan_point.x, 0.0, lane.monitor.z - 2.5);
    let to = lane.monitor - (g.player.pos + vec3(0.0, EYE_HEIGHT, 0.0));
    g.player.yaw = to.x.atan2(-to.z);
    g.player.pitch = (to.y / to.length()).asin();
    lane
}

#[test]
fn surveillance_camera_shows_the_player() {
    let mut g = scary_game(5, Tier::Uneasy);
    g.customers.clear();
    watch_monitor(&mut g);
    let (vp, cam) = g.monitor_view(16.0 / 9.0).expect("kein Monitor");
    let clip = vp * (g.player.pos + vec3(0.0, 1.0, 0.0)).extend(1.0);
    assert!(clip.w > 0.0, "Spieler hinter der Kamera");
    let ndc = clip.truncate() / clip.w;
    assert!(
        ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0,
        "Spieler nicht im Bild: {ndc}"
    );
    assert!(cam.y > 3.0, "Kamera haengt nicht an der Decke");
    // auf dem Monitor sieht man sich selbst, in der Welt nicht
    let mut mon = Vec::new();
    g.collect_monitor_people(&mut mon);
    assert_eq!(mon.len(), 1);
    assert!((mon[0].pos - g.player.pos).length() < 1e-3);
}

#[test]
fn figure_behind_you_only_on_the_monitor() {
    let mut g = scary_game(5, Tier::Uneasy);
    g.customers.clear();
    watch_monitor(&mut g);
    g.sfx.clear();
    for _ in 0..90 {
        if g.monitor_ghost_visible() {
            break;
        }
        step(&mut g, 1.0 / 30.0, Input::default());
        watch_monitor(&mut g);
    }
    assert!(
        g.monitor_ghost_visible(),
        "niemand erscheint auf dem Monitor"
    );
    let mut mon = Vec::new();
    g.collect_monitor_people(&mut mon);
    assert_eq!(mon.len(), 2, "Gestalt fehlt auf dem Monitor");
    let behind = mon[1].pos - g.player.pos;
    assert!(behind.length() < 1.3, "Gestalt steht nicht hinter einem");
    assert!(
        behind.dot(g.view_dir()) < 0.0,
        "Gestalt steht nicht hinter einem"
    );
    // in der echten Welt ist da nichts
    let mut world_people = Vec::new();
    g.collect_people(&mut world_people);
    assert!(world_people
        .iter()
        .all(|p| (p.pos - g.player.pos).length() > 1.5));
    // weiter hinschauen: Schreck
    for _ in 0..60 {
        step(&mut g, 1.0 / 30.0, Input::default());
        watch_monitor(&mut g);
        if scared(&g) {
            break;
        }
    }
    assert!(scared(&g), "kein Schreck auf dem Monitor");
    // umdrehen: niemand da, und auch vom Monitor verschwunden
    g.player.yaw += std::f32::consts::PI;
    step(&mut g, 1.0 / 30.0, Input::default());
    assert!(!g.monitor_ghost_visible());
}

#[test]
fn monitor_stays_calm_before_the_market_tips() {
    let mut g = Game::new(5);
    g.customers.clear();
    g.employees.clear();
    watch_monitor(&mut g);
    for _ in 0..120 {
        step(&mut g, 1.0 / 30.0, Input::default());
        watch_monitor(&mut g);
    }
    assert!(!g.monitor_ghost_visible());
    assert!(g.monitor_view(16.0 / 9.0).is_some());
}

#[test]
fn blackout_puts_someone_in_front_of_you() {
    let mut g = scary_game(42, Tier::Uneasy);
    g.customers.truncate(3);
    for c in &mut g.customers {
        c.stalker = false;
    }
    // freie Sicht nach vorn: im Gang stehen
    let it = shelf_item(&g, g.list[0], false).unwrap();
    let target = g.world.items[it].pos;
    look_at_point(&mut g, target);
    g.player.yaw += std::f32::consts::PI; // weg vom Regal, in den Gang
    g.player.pitch = 0.0;
    g.sfx.clear();
    g.blackout = Some((0.0, None));
    let mut dark = false;
    for _ in 0..120 {
        g.update(1.0 / 60.0, &mut Input::default());
        dark |= g.blackout() > 0.9;
        if g.blackout.is_none() {
            break;
        }
    }
    assert!(dark, "es wird nicht dunkel");
    assert!(g.blackout.is_none(), "Licht kommt nicht wieder");
    let near = g
        .customers
        .iter()
        .any(|c| (c.pos - g.player.pos).length() < 3.2 && c.stare > 0.0);
    if near {
        assert!(scared(&g), "Gestalt steht da, aber kein Schreck");
    }
}

#[test]
fn jumpscares_come_regularly() {
    // gekippter Markt, Stufe "unruhig", Spieler liest Zettel und dreht sich ab und zu um:
    // in 4 Minuten mehrere Schrecken
    let mut g = scary_game(7, Tier::Uneasy);
    g.scare_cd = 15.0;
    let mut scares = 0;
    for k in 0..(240 * 10) {
        if k % 70 == 69 {
            g.player.yaw += std::f32::consts::PI;
        }
        let mut inp = Input {
            list_open: true,
            ..Default::default()
        };
        for _ in 0..6 {
            g.update(1.0 / 60.0, &mut inp);
        }
        scares += g
            .sfx
            .iter()
            .filter(|(s, _)| matches!(s, Sfx::Scare))
            .count();
        g.sfx.clear();
        if g.outcome != Outcome::Running {
            break;
        }
        g.tier = Tier::Uneasy;
    }
    assert!(scares >= 3, "nur {scares} Schrecken in 4 Minuten");
}

#[test]
fn scare_inverts_colors_flashes_and_zooms() {
    let mut g = scary_game(3, Tier::Uneasy);
    g.customers.clear();
    g.scare_cd = 1e6;
    g.fire_scare(1.0);
    let (mut inverted, mut normal, mut flash, mut zoom) = (false, false, 0.0f32, 1.0f32);
    for _ in 0..20 {
        g.update(1.0 / 60.0, &mut Input::default());
        inverted |= g.scare_invert() > 0.5;
        normal |= g.scare_invert() < 0.5;
        flash = flash.max(g.scare_flash());
        zoom = zoom.min(g.scare_zoom());
    }
    assert!(inverted && normal, "kein Farbumkehr-Stroboskop");
    assert!(flash > 0.3, "kein Lichtblitz");
    assert!(zoom < 0.85, "kein Zoom-Ruck");
    step(&mut g, 1.0, Input::default());
    assert_eq!(g.scare_flash(), 0.0);
    assert_eq!(g.scare_zoom(), 1.0);
}

#[test]
fn subliminal_flashes_only_late() {
    let mut g = scary_game(4, Tier::Uneasy);
    g.customers.clear();
    g.scare_cd = 1e6;
    let mut seen = false;
    for _ in 0..(40 * 60) {
        g.update(1.0 / 60.0, &mut Input::default());
        seen |= g.scare_invert() > 0.5;
        g.tier = Tier::Uneasy;
    }
    assert!(!seen, "Blitze schon auf Stufe unruhig");
    g.tier = Tier::Wrong;
    for _ in 0..(40 * 60) {
        g.update(1.0 / 60.0, &mut Input::default());
        seen |= g.scare_invert() > 0.5;
        g.tier = Tier::Wrong;
    }
    assert!(seen, "keine unterschwelligen Blitze ab falsch");
}
