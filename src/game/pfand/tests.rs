//! Tests zum Pfand.

use super::machine::{ARM_AFTER, JAM_NOISE, LAMP_TIME};
use super::*;

const DT: f32 = 1.0 / 60.0;

fn market(seed: u64) -> Game {
    let mut g = Game::new(seed);
    g.tip();
    g.customers.clear();
    g.employees.clear();
    g
}

fn step(g: &mut Game, mut input: Input) {
    g.customers.clear();
    g.employees.clear();
    g.update(DT, &mut input);
}

fn wait(g: &mut Game, secs: f32) {
    for _ in 0..(secs / DT) as usize {
        step(g, Input::default());
    }
}

fn press(f: impl Fn(&mut Input)) -> Input {
    let mut i = Input::default();
    f(&mut i);
    i
}

/// Vor einem laufenden Automaten, Wagen daneben, Fenster offen.
fn at_machine(seed: u64, bag: &[Empty]) -> Game {
    let mut g = market(seed);
    g.pfand_cheat(0);
    g.pfand.bag = bag.to_vec();
    let i = (0..9).find(|&i| g.machine_works(i)).unwrap();
    g.open_machine(i);
    g
}

/// Richtig drehen, auf Gruen warten, einlegen.
fn insert_right(g: &mut Game) {
    wait(g, LAMP_TIME + 0.1);
    g.pfand.window.as_mut().unwrap().turn = 0;
    step(g, press(|i| i.interact = true));
}

#[test]
fn bottles_lie_reachable_in_the_tipped_market_only() {
    for seed in 0..12u64 {
        let mut g = Game::new(seed);
        assert!(g.pfand.bottles.is_empty(), "Flaschen im normalen Markt");
        g.tip();
        assert!(g.pfand.bottles.len() >= BOTTLES - 2, "{seed}: zu wenige");
        let check = |g: &Game| {
            let cells = g.world.reachable_centers();
            for b in &g.pfand.bottles {
                let d = cells
                    .iter()
                    .map(|c| c.distance(b.pos))
                    .fold(f32::MAX, f32::min);
                assert!(d < 0.7, "{seed}: Flasche unerreichbar ({d} m)");
                assert!(!g.world.in_checkout(b.pos));
            }
            let wing = g
                .pfand
                .bottles
                .iter()
                .filter(|b| g.world.in_wing(b.pos))
                .count();
            assert!(wing <= IN_WING + 2);
            assert!(g.pfand.bottles.len() <= BOTTLES);
        };
        check(&g);
        for tier in [Tier::Uneasy, Tier::Wrong, Tier::Collapse] {
            g.force_tier(tier);
            g.finish_umbau();
            check(&g);
        }
    }
}

#[test]
fn every_room_has_a_machine_one_can_stand_in_front_of() {
    for seed in 0..12u64 {
        let mut g = market(seed);
        for (tier, works) in [
            (Tier::Calm, None),
            (Tier::Uneasy, None),
            (Tier::Wrong, Some(1)),
            (Tier::Collapse, Some(1)),
        ] {
            if tier > Tier::Calm {
                g.force_tier(tier);
                g.finish_umbau();
            }
            let wing = g.world.wing.as_ref().unwrap();
            assert_eq!(wing.machines.len(), wing.rooms.len());
            let cells = g.world.reachable_centers();
            for m in &wing.machines {
                let f = m.front();
                assert!(wing.contains(f));
                // auch mit dem Wagen
                assert!(
                    g.world.collide(f, 0.42).distance(f) < 1e-3,
                    "{seed}/{tier:?}: kein Platz davor"
                );
                assert!(cells.iter().any(|c| c.distance(f) < 0.7));
            }
            let n = wing.machines.len();
            let on = (0..n).filter(|&i| g.machine_works(i)).count();
            match works {
                Some(k) => assert_eq!(on, k),
                None if tier == Tier::Calm => assert_eq!(on, n),
                None => assert_eq!(on, if n == 1 { 1 } else { n - 1 }),
            }
        }
    }
}

#[test]
fn the_bag_holds_five_and_deposit_is_no_ware() {
    let mut g = market(3);
    let (value, tier) = (g.cart_value(), g.tier);
    let cart = g.cart.len();
    for k in 0..7 {
        g.held = Some(Empty::ALL[k % 4].item());
        g.aim_cart = true;
        g.interact_for_test();
        assert!(g.pfand.bag.len() <= BAG_MAX);
    }
    assert_eq!(g.pfand.bag.len(), BAG_MAX);
    assert!(g.held.is_some(), "sechste Flasche verschwunden");
    assert_eq!(g.cart.len(), cart, "Leergut im Korb");
    assert_eq!(g.cart_value(), value);
    g.check_tier();
    assert_eq!(g.tier, tier);
    // nicht aufs Band
    g.window_drop(belt_window::DragSrc::Hand);
    assert!(g.belt.is_empty() && g.held.is_some());
    // heraus geht es wieder, zuletzt Eingelegtes zuerst
    g.held = None;
    let last = *g.pfand.bag.last().unwrap();
    g.take_from_bag();
    assert_eq!(g.held.as_ref().unwrap().kind, PickupKind::Deposit(last));
}

#[test]
fn bottles_can_be_picked_up_by_looking_at_them() {
    let mut g = market(5);
    g.trolley.pos = g.world.corral;
    let spot = g.world.spawn - vec3(0.0, 0.0, 2.0);
    g.player.pos = spot;
    g.pfand.bottles = vec![Bottle {
        pos: spot + vec3(0.0, 0.0, -1.0),
        yaw: 0.3,
        kind: Empty::Can,
    }];
    g.player.yaw = 0.0;
    g.player.pitch = -1.0;
    step(&mut g, Input::default());
    assert_eq!(g.focus, Some(Focus::Bottle(0)));
    step(&mut g, press(|i| i.interact = true));
    assert!(g.pfand.bottles.is_empty());
    assert!(matches!(
        g.held.as_ref().unwrap().kind,
        PickupKind::Deposit(Empty::Can)
    ));
}

#[test]
fn machines_are_seen_and_open_with_e() {
    let mut g = market(6);
    g.pfand_cheat(0);
    step(&mut g, Input::default());
    assert!(matches!(g.focus, Some(Focus::Machine(_))), "{:?}", g.focus);
    step(&mut g, press(|i| i.interact = true));
    assert!(g.machine_window_open());
    // Esc bzw. Q schliesst
    step(&mut g, press(|i| i.cart_toggle = true));
    assert!(!g.machine_window_open());
}

#[test]
fn right_side_up_and_green_gives_money_wrong_or_early_makes_noise() {
    let mut g = at_machine(7, &[Empty::Can, Empty::Plastic]);
    let i = g.pfand.window.as_ref().unwrap().machine;
    // zu frueh
    g.pfand.window.as_mut().unwrap().turn = 0;
    g.noises.clear();
    g.insert_bottle();
    assert_eq!(g.pfand.sums[i], 0);
    assert!(g.noises.iter().any(|n| n.1 >= JAM_NOISE), "kein Laerm");
    assert!(g.pfand.window.as_ref().unwrap().bottle.is_some());
    // falsch herum
    wait(&mut g, LAMP_TIME + 0.1);
    g.pfand.window.as_mut().unwrap().turn = 3;
    step(&mut g, press(|i| i.interact = true));
    assert_eq!(g.pfand.sums[i], 0);
    // richtig: zaehlt, und die naechste Flasche rutscht nach
    insert_right(&mut g);
    assert_eq!(g.pfand.sums[i], 25);
    assert!(g.pfand.window.as_ref().unwrap().bottle.is_some());
    insert_right(&mut g);
    assert_eq!(g.pfand.sums[i], 50);
    assert!(g.pfand.bag.is_empty());
    // Bon
    step(&mut g, press(|i| i.take_out = true));
    assert_eq!(g.pfand.bons, vec![50]);
    assert_eq!(g.pfand.sums[i], 0);
    step(&mut g, press(|i| i.take_out = true));
    assert_eq!(g.pfand.bons.len(), 1, "leerer Bon");
}

#[test]
fn turning_works_with_keys_wheel_and_mouse() {
    let mut g = at_machine(8, &[Empty::Glass]);
    step(&mut g, Input::default()); // gehaltene Tasten loslassen
    let turn = |g: &Game| g.pfand.window.as_ref().unwrap().turn;
    let t0 = turn(&g);
    step(&mut g, press(|i| i.right = true));
    assert_eq!(turn(&g), (t0 + 1) % 8);
    step(&mut g, press(|i| i.right = true));
    assert_eq!(turn(&g), (t0 + 1) % 8, "gehalten zaehlt nur einmal");
    step(&mut g, press(|i| i.wheel = -2));
    assert_eq!(turn(&g), (t0 + 7) % 8);
    let r = machine::BTN_RIGHT;
    step(
        &mut g,
        press(|i| {
            i.ui_mouse = (r.0 + 3.0, r.1 + 3.0);
            i.ui_pressed = true;
        }),
    );
    assert_eq!(turn(&g), t0);
    // der Blick dreht sich nicht mit
    let yaw = g.player.yaw;
    g.look(200.0, 0.0);
    assert_eq!(g.player.yaw, yaw);
}

#[test]
fn closing_gives_the_bottle_back() {
    let mut g = at_machine(9, &[Empty::Can]);
    assert!(g.pfand.bag.is_empty());
    g.close_pointer_window();
    assert_eq!(g.pfand.bag, vec![Empty::Can]);
    // ohne Wagen in der Naehe: nur aus der Hand
    g.trolley.pos = g.world.corral;
    let i = (0..9).find(|&i| g.machine_works(i)).unwrap();
    g.open_machine(i);
    assert!(g.pfand.window.as_ref().unwrap().bottle.is_none());
    g.close_machine();
    g.held = Some(Empty::Glass.item());
    g.open_machine(i);
    assert!(g.held.is_none());
    g.close_machine();
    assert!(g.held.is_some());
}

#[test]
fn rebuilding_forgets_unprinted_money_but_keeps_receipts() {
    let mut g = at_machine(10, &[Empty::Can, Empty::Can]);
    insert_right(&mut g);
    g.print_bon();
    insert_right(&mut g);
    assert!(g.pfand.sums.iter().sum::<i64>() > 0);
    g.params.shuffle += 1;
    g.rebuild_world();
    assert_eq!(g.pfand.sums.iter().sum::<i64>(), 0);
    assert_eq!(g.pfand.bons, vec![25]);
    assert!(!g.machine_window_open());
    // im Tutorial verfaellt nichts
    let mut t = Game::new_tutorial(10);
    t.tip();
    t.pfand.sums = vec![75; 2];
    t.rebuild_world();
    assert_eq!(t.pfand.sums[0], 75);
}

#[test]
fn something_grabs_you_in_the_collapse_but_does_not_kill() {
    let mut g = at_machine(11, &[Empty::Can]);
    wait(&mut g, ARM_AFTER + 1.0);
    assert!(g.machine_window_open(), "Arm schon in ruhig");
    g.close_machine();
    g.force_tier(Tier::Collapse);
    g.finish_umbau();
    g.pfand_cheat(0);
    let i = (0..9).find(|&i| g.machine_works(i)).unwrap();
    g.open_machine(i);
    g.god = true;
    g.machine_arm();
    assert!(g.pfand.window.as_ref().unwrap().arm > 0.0);
    wait(&mut g, machine::ARM_TIME + 0.2);
    assert!(!g.machine_window_open());
    assert_eq!(g.outcome, Outcome::Running);
    assert_eq!(g.deaths, 0);
    // im Tutorial nie
    let mut t = Game::new_tutorial(11);
    t.tip();
    t.pfand_cheat(0);
    t.tier = Tier::Collapse;
    t.open_machine(0);
    wait(&mut t, ARM_AFTER + 2.0);
    assert!(t.pfand.window.as_ref().is_none_or(|w| w.arm <= 0.0));
}

#[test]
fn bottles_clink_and_fall_out_when_jumping() {
    let mut g = market(12);
    g.pfand.bag = vec![Empty::Can; 5];
    let loose = g.loose.len();
    g.spill_cart();
    let out = 5 - g.pfand.bag.len();
    assert!((1..=2).contains(&out));
    assert!(g.loose.len() >= loose + out);
    // am Boden liegendes Leergut laesst sich wieder einsammeln
    let i = g
        .loose
        .iter()
        .position(|l| matches!(l.data.kind, PickupKind::Deposit(_)))
        .unwrap();
    g.held = Some(g.loose.remove(i).data);
    g.aim_cart = true;
    g.interact_for_test();
    assert_eq!(g.pfand.bag.len(), 5 - out + 1);
}

#[test]
fn a_new_run_starts_without_deposit() {
    let mut g = market(13);
    g.pfand.bons.push(100);
    g.pfand.bag.push(Empty::Can);
    let n = Game::new(13);
    assert!(n.pfand.bons.is_empty() && n.pfand.bag.is_empty());
}

#[test]
fn the_tutorial_bottles_are_near_and_worth_enough() {
    for seed in 60..66 {
        let mut g = Game::new_tutorial(seed);
        g.tip();
        let near = g
            .pfand
            .bottles
            .iter()
            .filter(|b| b.kind == Empty::Plastic)
            .filter(|b| {
                let d = g.world.wing.as_ref().unwrap().doors[0];
                b.pos.distance(d) < 8.0
            })
            .count();
        assert!(near >= 2, "{seed}: {near}");
    }
}

// --- Kiosk und Items ---

use super::items::{Item, ENERGY_TIME, POCKET_MAX, SPARKLE_TIME};
use super::kiosk::Column;

fn at_kiosk(seed: u64, bons: &[i64]) -> Game {
    let mut g = market(seed);
    g.pfand_cheat(3);
    g.pfand.bons = bons.to_vec();
    step(&mut g, Input::default());
    assert_eq!(g.focus, Some(Focus::Kiosk));
    step(&mut g, press(|i| i.interact = true));
    assert!(g.kiosk_open());
    g
}

fn choose(g: &mut Game, item: Item) {
    let w = g.pfand.kiosk.as_mut().unwrap();
    w.column = Column::Items;
    w.item = Item::ALL.iter().position(|i| *i == item).unwrap();
    w.confirm = false;
}

#[test]
fn the_kiosk_is_reachable_everywhere_and_blocks_nothing() {
    for seed in 0..12u64 {
        let mut g = Game::new(seed);
        assert!(!g.kiosk_has_open(), "offen im normalen Markt");
        assert!(g.world.all_reachable(), "{seed}: normaler Markt versperrt");
        g.tip();
        for tier in [Tier::Calm, Tier::Uneasy, Tier::Wrong, Tier::Collapse] {
            if tier > Tier::Calm {
                g.force_tier(tier);
                g.finish_umbau();
            }
            let k = g.world.kiosk.unwrap();
            let f = k.front();
            assert!(g.world.all_reachable(), "{seed}/{tier:?}: versperrt");
            assert!(
                g.world.collide(f, 0.42).distance(f) < 1e-3,
                "{seed}/{tier:?}: kein Platz vor dem Kiosk"
            );
            let cells = g.world.reachable_centers();
            assert!(cells.iter().any(|c| c.distance(f) < 0.7), "{seed}/{tier:?}");
            assert!(g.kiosk_seller().is_some());
        }
    }
}

#[test]
fn one_receipt_per_purchase_and_the_rest_is_lost() {
    let mut g = at_kiosk(20, &[200, 90]);
    choose(&mut g, Item::Energy);
    step(&mut g, press(|i| i.interact = true));
    // erst nachfragen, noch nichts verkauft
    assert!(g.pfand.kiosk.as_ref().unwrap().confirm);
    assert!(g.pfand.kiosk.as_ref().unwrap().text.contains("verfällt"));
    assert_eq!(g.pfand.bons.len(), 2);
    step(&mut g, press(|i| i.interact = true));
    // der Bon ueber 2,00 ist ganz weg, der andere unberuehrt
    assert_eq!(g.pfand.bons, vec![90]);
    assert_eq!(g.pfand.owned.pocket, vec![Item::Energy]);
    assert!(g.pfand.bought);
}

#[test]
fn small_receipts_do_not_add_up() {
    let mut g = at_kiosk(21, &[60, 60, 60]);
    choose(&mut g, Item::Oil); // 1,00
    for _ in 0..3 {
        step(&mut g, press(|i| i.interact = true));
    }
    assert_eq!(g.pfand.bons.len(), 3);
    assert!(!g.pfand.owned.has(Item::Oil));
    // ohne Bon gibt es nichts
    let mut g = at_kiosk(21, &[]);
    choose(&mut g, Item::Sticker);
    for _ in 0..3 {
        step(&mut g, press(|i| i.interact = true));
    }
    assert!(!g.pfand.owned.has(Item::Sticker));
}

#[test]
fn refused_purchases_cost_nothing() {
    let mut g = at_kiosk(22, &[250, 250]);
    g.pfand.owned.kept.push(Item::Torch);
    choose(&mut g, Item::Torch);
    for _ in 0..3 {
        step(&mut g, press(|i| i.interact = true));
    }
    g.pfand.owned.pocket = vec![Item::Tea; POCKET_MAX];
    choose(&mut g, Item::Mett);
    for _ in 0..3 {
        step(&mut g, press(|i| i.interact = true));
    }
    assert_eq!(g.pfand.bons.len(), 2);
    assert_eq!(g.pfand.owned.pocket.len(), POCKET_MAX);
}

#[test]
fn mouse_and_keys_operate_the_kiosk() {
    let mut g = at_kiosk(23, &[150, 40]);
    step(&mut g, Input::default());
    // zweiten Bon anklicken, dann das zweite Item
    let click = |x: f32, y: f32| {
        press(move |i| {
            i.ui_mouse = (x, y);
            i.ui_pressed = true;
        })
    };
    let (b, s) = (kiosk::BONS, kiosk::SHELF);
    step(&mut g, click(b.0 + 20.0, b.1 + 14.0 + kiosk::ROW * 1.5));
    assert_eq!(g.pfand.kiosk.as_ref().unwrap().bon, 1);
    step(&mut g, click(s.0 + 20.0, s.1 + 14.0 + kiosk::ROW * 1.5));
    assert!(g.pfand.kiosk.as_ref().unwrap().confirm);
    step(&mut g, click(s.0 + 20.0, s.1 + 14.0 + kiosk::ROW * 1.5));
    assert_eq!(g.pfand.bons, vec![150]);
    assert_eq!(g.pfand.owned.pocket, vec![Item::Energy]);
    // Tasten: Spalte wechseln, Zeile weiter
    let item = g.pfand.kiosk.as_ref().unwrap().item;
    step(&mut g, press(|i| i.back = true));
    assert_eq!(g.pfand.kiosk.as_ref().unwrap().item, item + 1);
    step(&mut g, Input::default());
    step(&mut g, press(|i| i.left = true));
    assert_eq!(g.pfand.kiosk.as_ref().unwrap().column, Column::Bons);
    step(&mut g, press(|i| i.cart_toggle = true));
    assert!(!g.kiosk_open());
    assert!(g.grace > 1.0, "keine Schonfrist");
}

#[test]
fn customers_stand_still_and_nobody_is_caught_at_the_kiosk() {
    let mut g = at_kiosk(24, &[100]);
    g.customers.clear();
    let p = g.player.pos + vec3(1.2, 0.0, 0.0);
    g.spawn_customer_at(p);
    g.customers[0].stalker = true;
    g.player.yaw = -FRAC_PI_2; // Blick zum Kiosk, Kunde im Ruecken
    let from = g.customers[0].pos;
    for _ in 0..540 {
        g.employees.clear();
        g.update(DT, &mut Input::default());
    }
    assert!(
        g.customers[0].pos.distance(from) < 0.01,
        "Kunde bewegt sich"
    );
    assert!(g.dying.is_none() && g.outcome == Outcome::Running);
}

#[test]
fn items_work_and_wear_off() {
    let mut g = market(25);
    // Tee
    g.suspicion = 0.8;
    g.receive_item(Item::Tea);
    g.use_item();
    assert!((g.suspicion - 0.4).abs() < 1e-5);
    assert!(g.pfand.owned.pocket.is_empty());
    // Energy
    g.receive_item(Item::Energy);
    step(&mut g, press(|i| i.use_item = true));
    assert!(g.energized());
    // Wunderkerze und Mett
    g.receive_item(Item::Sparkler);
    g.receive_item(Item::Mett);
    assert_eq!(g.pfand.owned.selected(), Some(Item::Mett));
    g.cycle_item(1);
    assert_eq!(g.pfand.owned.selected(), Some(Item::Sparkler));
    g.use_item();
    assert!(g.sparkler_tip().is_some());
    g.use_item();
    wait(&mut g, ENERGY_TIME.max(SPARKLE_TIME) + 0.5);
    assert!(!g.energized() && g.sparkler_tip().is_none());
    g.use_item(); // leer: nur ein Hinweis
                  // Taschenlampe
    assert_eq!(g.camera_light(), 1.0);
    g.receive_item(Item::Torch);
    assert!(g.camera_light() > 1.5);
}

#[test]
fn energy_keeps_stamina_full() {
    let run = |drink: bool| {
        let mut g = market(26);
        g.trolley.pushing = false;
        g.trolley.pos = g.world.corral;
        if drink {
            g.receive_item(Item::Energy);
            g.use_item();
        }
        for _ in 0..120 {
            step(
                &mut g,
                press(|i| {
                    i.forward = true;
                    i.run = true;
                }),
            );
        }
        g.player.stamina
    };
    assert!(run(false) < 0.9);
    assert_eq!(run(true), 1.0);
}

#[test]
fn oil_and_soles_make_you_quieter() {
    let loudest = |items: &[Item], push: bool| {
        let mut g = market(27);
        for i in items {
            g.receive_item(*i);
        }
        g.trolley.pushing = push;
        if !push {
            g.trolley.pos = g.world.corral;
        }
        let mut worst = 0.0f32;
        for k in 0..240 {
            let turn = k as f32 * 0.03;
            g.player.yaw = turn;
            g.noises.clear();
            let input = press(|i| i.forward = true);
            g.update_movement(DT, &input);
            worst = g.noises.iter().map(|n| n.1).fold(worst, f32::max);
        }
        worst
    };
    assert!(loudest(&[Item::Oil], true) < loudest(&[], true));
    assert!(loudest(&[Item::Soles], false) < loudest(&[], false));
}

#[test]
fn the_bell_rings_only_for_stalkers_behind() {
    let mut g = market(28);
    g.receive_item(Item::Bell);
    let p = g.player.pos;
    g.player.yaw = 0.0; // Blick nach -z
    let mut ring = |behind: bool, stalker: bool| {
        g.customers.clear();
        let z = if behind { 3.0 } else { -3.0 };
        g.spawn_customer_at(p);
        g.customers[0].pos = p + vec3(0.0, 0.0, z);
        g.customers[0].stalker = stalker;
        g.sfx_at.clear();
        g.pfand.owned.kept = vec![Item::Bell];
        for _ in 0..100 {
            g.update_items(DT);
        }
        g.sfx_at.iter().any(|s| matches!(s.0, Sfx::Bell))
    };
    assert!(ring(true, true));
    assert!(!ring(false, true));
    assert!(!ring(true, false));
}

#[test]
fn bought_things_are_gone_in_the_next_run() {
    let n = Game::new(29);
    assert!(n.pfand.owned.pocket.is_empty() && n.pfand.owned.kept.is_empty());
    assert_eq!(n.camera_light(), 1.0);
}
