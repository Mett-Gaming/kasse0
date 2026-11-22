use super::*;

fn step(g: &mut Game, secs: f32, input: Input) {
    let mut inp = input;
    for _ in 0..(secs * 60.0) as usize {
        g.update(1.0 / 60.0, &mut inp);
    }
}

/// Eine echte (nicht falsche, nicht genommene) Regalware fuer einen Zettel-Eintrag.
fn shelf_item(g: &Game, entry: usize, pricey: bool) -> Option<usize> {
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
    let mut g = Game::new(5);
    g.tip();
    g.employees.clear();
    g.customers.truncate(1);
    let p = g.player.pos;
    g.customers[0].pos = vec3(p.x, 0.0, p.z - 1.5);
    g.customers[0].stalker = true;
    g.player.yaw = std::f32::consts::PI; // wegsehen
    step(&mut g, 6.0, Input::default());
    assert_eq!(g.outcome, Outcome::Caught);
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

fn press_e(g: &mut Game) {
    let mut inp = Input {
        interact: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &mut inp);
}

/// Spieler an die offene Kasse, Blick aufs Band.
fn look_at_belt(g: &mut Game) {
    let lane = g.open_lane().unwrap();
    let mid = (lane.belt_start + lane.belt_end) * 0.5;
    look_from(g, lane.scan_point, mid);
    g.update(1.0 / 60.0, &mut Input::default());
}

fn look_at_terminal(g: &mut Game) {
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
    assert!(!g.unloading && g.belt.is_empty(), "Ausraeumen ohne Wagen");
    // Wagen zur Kasse, alles aufs Band
    let n = g.cart.len();
    g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
    press_e(&mut g);
    assert!(g.unloading, "Ausraeumen startet nicht");
    for _ in 0..60 {
        if g.cart.is_empty() && g.belt.is_empty() && !g.unloading {
            break;
        }
        step(&mut g, 1.0, Input::default());
    }
    assert!(g.cart.is_empty() && g.belt.is_empty(), "Band nicht leer");
    assert_eq!(g.scanned.len(), n);
    assert!(g.scanned_complete());
    assert!(!g.paid, "ohne Terminal bezahlt");
    // am Terminal bezahlen
    look_at_terminal(&mut g);
    g.update(1.0 / 60.0, &mut Input::default());
    assert_eq!(g.focus, Some(Focus::Terminal));
    press_e(&mut g);
    assert!(g.paid, "nicht bezahlt");
    // zur Tuer und raus
    g.player.pos = g.world.door_center - vec3(0.0, 0.0, 0.8);
    g.player.yaw = std::f32::consts::PI; // nach Sueden
    g.player.pitch = 0.0;
    step(
        &mut g,
        4.0,
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
    g.cheat(CHEATS.len() - 1); // Mitarbeiter nimmt den Wagen mit
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
    assert!(g.suspicion > before);
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
