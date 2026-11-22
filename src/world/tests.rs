use super::*;
use crate::catalog::FixtureKind;

fn gen(seed: u64, tipped: bool) -> World {
    let mut p = WorldParams::normal(seed);
    p.tipped = tipped;
    p.stretch = if tipped { 1.6 } else { 1.0 };
    generate(&p)
}

fn escalated(seed: u64, mix: u8, warp: f32, walls: u32) -> World {
    let mut p = WorldParams::normal(seed);
    p.tipped = true;
    p.stretch = 2.0;
    p.warp = warp;
    p.walls = walls;
    p.mix = mix;
    generate(&p)
}

#[test]
fn same_seed_same_market() {
    let a = gen(1994, false);
    let b = gen(1994, false);
    assert_eq!(a.instances, b.instances);
    assert_eq!(a.lights, b.lights);
    assert_eq!(a.items.len(), b.items.len());
    assert!(a
        .items
        .iter()
        .zip(&b.items)
        .all(|(x, y)| x.product == y.product && x.pos == y.pos));
}

#[test]
fn different_seed_different_market() {
    assert_ne!(gen(1, false).instances, gen(2, false).instances);
}

#[test]
fn shelves_are_full_of_real_products_within_budget() {
    for seed in 0..5 {
        let w = escalated(seed, 0, 1.0, 0);
        assert!(w.items.len() > 3000, "zu leer: {}", w.items.len());
        assert!(w.items.len() < 40_000, "zu voll: {}", w.items.len());
        let n = gen(seed, false);
        assert!(n.items.len() > 2000);
    }
}

#[test]
fn every_list_item_is_reachable_at_every_mix_level() {
    for mix in 0..=3 {
        for seed in 0..12 {
            let w = escalated(
                seed,
                mix,
                if mix > 0 { 1.0 } else { 0.0 },
                if mix >= 2 { 3 } else { 0 },
            );
            assert!(
                w.all_reachable(),
                "mix {mix}, seed {seed}: Zettel-Ware oder Bereich unerreichbar"
            );
        }
    }
    for seed in 0..12 {
        assert!(gen(seed, false).all_reachable(), "normal, seed {seed}");
    }
}

#[test]
fn water_lies_in_the_drinks_room() {
    let cat = catalog();
    let water = cat.list_index("Mineralwasser").unwrap();
    for seed in 0..10 {
        let w = gen(seed, false);
        let ok = w.items.iter().any(|it| {
            cat.products[it.product as usize].list == Some(water)
                && w.drinks_zone.contains_xz(it.pos)
        });
        assert!(ok, "kein Wasser im Getraenkeraum (seed {seed})");
    }
}

#[test]
fn logical_layout_puts_departments_in_their_fixtures() {
    let cat = catalog();
    let w = gen(4, false);
    // Kuehlwand an der Nordwand: nur Kuehlware
    let fridge: Vec<&ShelfItem> = w
        .items
        .iter()
        .filter(|it| it.pos.z < 1.1 && !w.drinks_zone.contains_xz(it.pos))
        .collect();
    assert!(!fridge.is_empty());
    assert!(fridge
        .iter()
        .all(|it| cat.subs[cat.products[it.product as usize].sub].fixture == FixtureKind::Cooler));
    // Kuehltruhen (niedrig, Ware auf 0,55 m) enthalten Tiefkuehlware
    let freezer: Vec<&ShelfItem> = w
        .items
        .iter()
        .filter(|it| {
            (it.pos.y - 0.55).abs() < 0.01
                && it.pos.x < 7.0
                && it.pos.z > w.aisle_z0
                && it.pos.z < w.aisle_z1
        })
        .collect();
    assert!(!freezer.is_empty());
    assert!(freezer
        .iter()
        .all(|it| cat.subs[cat.products[it.product as usize].sub].fixture == FixtureKind::Freezer));
}

#[test]
fn chaos_mixes_departments_and_signs_lie() {
    let w = escalated(6, 3, 1.0, 0);
    assert!(w.layout.signs_lie);
    let cat = catalog();
    let fridge_foreign = w
        .items
        .iter()
        .filter(|it| it.pos.z < 1.1 && !w.drinks_zone.contains_xz(it.pos))
        .any(|it| cat.subs[cat.products[it.product as usize].sub].fixture != FixtureKind::Cooler);
    assert!(
        fridge_foreign,
        "im Chaos steht nichts Fremdes in der Kuehlung"
    );
}

#[test]
fn item_ray_finds_products() {
    let w = gen(2, false);
    let it = w.items[w.items.len() / 2];
    let target = it.pos + vec3(0.0, it.half().y, 0.0);
    let eye = target + vec3(0.0, 0.3, 1.2);
    let d = (target - eye).normalize();
    let hit = w.item_ray(eye, d, 2.5);
    assert!(hit.is_some(), "Ware nicht getroffen");
}

#[test]
fn spawn_drinks_door_and_lanes_are_free() {
    for seed in 0..30 {
        for tipped in [false, true] {
            let w = gen(seed, tipped);
            assert_eq!(
                w.collide(w.spawn, 0.3),
                w.spawn,
                "Spawn blockiert (seed {seed})"
            );
            let door = vec3(w.drinks_zone.min.x + 1.7, 0.0, w.drinks_zone.max.z + 1.0);
            assert_eq!(
                w.collide(door, 0.3),
                door,
                "Getraenketuer blockiert (seed {seed})"
            );
            let inside = vec3(w.drinks_zone.min.x + 1.7, 0.0, w.drinks_zone.max.z - 0.8);
            assert!(w.drinks_zone.contains_xz(inside));
            assert_eq!(
                w.collide(inside, 0.3),
                inside,
                "Getraenkeraum-Eingang blockiert (seed {seed})"
            );
            for lane in &w.lanes {
                assert_eq!(
                    w.collide(lane.scan_point, 0.3),
                    lane.scan_point,
                    "Kasse unerreichbar (seed {seed})"
                );
            }
            assert_eq!(
                w.collide(w.corral, 0.4),
                w.corral,
                "Wagenbox blockiert (seed {seed})"
            );
        }
    }
}

#[test]
fn tipped_market_is_longer_and_darker() {
    for seed in 0..10 {
        let normal = gen(seed, false);
        let tipped = gen(seed, true);
        assert!(tipped.size.z > normal.size.z);
        assert!(tipped.fog_density > normal.fog_density);
        assert_eq!(tipped.drinks_zone, normal.drinks_zone);
    }
}

#[test]
fn stretching_remaps_positions() {
    let a = escalated(9, 0, 0.0, 0);
    let mut p = WorldParams::normal(9);
    p.tipped = true;
    p.stretch = 1.6;
    let b = generate(&p);
    let mid = vec3(3.0, 0.0, (a.aisle_z0 + a.aisle_z1) * 0.5);
    let m = b.remap_from(&a, mid);
    assert!((m.z - (b.aisle_z0 + b.aisle_z1) * 0.5).abs() < 1e-3);
}

#[test]
fn curved_aisles_have_rotated_parts_and_passages() {
    let straight = escalated(3, 0, 0.0, 0);
    let crooked = escalated(3, 0, 1.0, 0);
    // gerade Gondeln: keine gedrehten Teile
    assert!(straight.obbs.is_empty());
    assert!(crooked.obbs.len() > 50);
    let max_angle = crooked
        .obbs
        .iter()
        .map(|o| o.yaw.abs())
        .fold(0.0f32, f32::max);
    assert!(max_angle > 0.1, "kaum gebogen: {max_angle}");
    assert!(
        (0..10).any(|s| !escalated(s, 0, 1.0, 0).passages.is_empty()),
        "keine Durchgaenge erzeugt"
    );
}

#[test]
fn rotated_obstacles_collide_and_block_sight() {
    let w = escalated(5, 0, 1.0, 0);
    let o = w
        .obbs
        .iter()
        .copied()
        .find(|o| o.max_y() > 2.0)
        .expect("keine hohe gedrehte Gondel");
    let p = w.collide(vec3(o.center.x, 0.0, o.center.z), 0.3);
    let l = o.to_local(vec3(p.x, o.center.y, p.z));
    assert!(
        l.x.abs() >= o.half.x + 0.29 || l.z.abs() >= o.half.z + 0.29,
        "{l:?}"
    );
    let n = o.to_world_dir(vec3(1.0, 0.0, 0.0));
    let a = o.center + n * 1.5 + vec3(0.0, 0.4, 0.0);
    let b = o.center - n * 1.5 + vec3(0.0, 0.4, 0.0);
    assert!(!w.line_of_sight(a, b));
}

#[test]
fn freezers_are_low_enough_to_look_over() {
    let w = gen(8, false);
    // ueber eine Kuehltruhe (Hoehe 0,9 m) hinweg sieht man in Augenhoehe
    let x = 2.5 + 3.4 * 0.5;
    let z = (w.aisle_z0 + w.aisle_z1) * 0.5;
    assert!(w.line_of_sight(vec3(x - 2.0, 1.6, z), vec3(x + 2.0, 1.6, z)));
    assert!(!w.line_of_sight(vec3(x - 2.0, 0.4, z), vec3(x + 2.0, 0.4, z)));
}

#[test]
#[ignore]
fn zz_bench_generate() {
    let _ = catalog();
    let _ = crate::signs::atlas();
    for (tipped, mix, warp, walls) in [
        (false, 0u8, 0.0f32, 0u32),
        (true, 2, 1.0, 2),
        (true, 3, 1.0, 3),
    ] {
        let t = std::time::Instant::now();
        let w = escalated(7, mix, warp, walls);
        let _ = tipped;
        println!(
            "generate mix {mix}: {:.1} ms, {} Waren, {} Boxen",
            t.elapsed().as_secs_f32() * 1000.0,
            w.items.len(),
            w.instances.len()
        );
        let t = std::time::Instant::now();
        let ok = w.all_reachable();
        println!(
            "  all_reachable: {:.1} ms ({ok})",
            t.elapsed().as_secs_f32() * 1000.0
        );
        let t = std::time::Instant::now();
        let mut v = Vec::new();
        w.item_draws(&mut v);
        println!("  item_draws: {:.1} ms", t.elapsed().as_secs_f32() * 1000.0);
    }
}

fn collapsed(seed: u64) -> World {
    let mut p = WorldParams::normal(seed);
    p.tipped = true;
    p.stretch = 2.0;
    p.warp = 1.0;
    p.walls = 3;
    p.mix = 3;
    p.holes = 5;
    p.ceiling_holes = 7;
    p.rot = 0.15;
    p.giants = 3;
    generate(&p)
}

#[test]
fn collapse_has_holes_rot_and_giants_but_stays_reachable() {
    let mut holes = 0;
    let mut giants = 0;
    for seed in 0..10 {
        let w = collapsed(seed);
        assert!(w.all_reachable(), "seed {seed}: unerreichbar");
        assert!(!w.rotten.is_empty(), "seed {seed}: keine verdorbene Ware");
        assert!(
            w.items.iter().any(|it| it.rot == 2),
            "seed {seed}: nichts Kurioses"
        );
        giants += w.giants.len();
        // Loecher: schwarze, flache Leere am Boden
        holes += w
            .instances
            .iter()
            .filter(|i| i.center[1] < 0.01 && i.extra[0] == material::EMISSIVE && i.color[0] < 0.01)
            .count();
        // Spawn und Kassen bleiben frei
        assert_eq!(w.collide(w.spawn, 0.3), w.spawn);
        for lane in &w.lanes {
            assert_eq!(w.collide(lane.scan_point, 0.3), lane.scan_point);
        }
    }
    assert!(holes > 10, "kaum Loecher: {holes}");
    assert!(giants > 3, "kaum Riesenwaren: {giants}");
    // im normalen Markt nichts davon
    let n = gen(3, false);
    assert!(n.rotten.is_empty() && n.giants.is_empty());
}

#[test]
fn sway_bends_only_the_aisles_and_keeps_the_floor_connected() {
    let w = collapsed(4);
    let s = Vec4::new(0.25, 0.55, w.aisle_z1, w.aisle_x1);
    // ausserhalb des Gangbereichs: fest (Waende, Kassen, Getraenke-Nordwand)
    for p in [
        vec3(0.2, 0.0, 10.0),
        vec3(5.0, 0.0, 1.0),
        vec3(5.0, 0.0, w.aisle_z1 + 2.0),
    ] {
        assert!(sway(s, p, 3.0).length() < 1e-4, "{p:?} bewegt sich");
    }
    // im Gang: Steigung bis zur halben Amplitude, aber stetig (keine Stufen)
    let mut max_up: f32 = 0.0;
    let x = (AISLES_X0 + w.aisle_x1) * 0.5;
    let mut z = w.aisle_z0;
    while z < w.aisle_z1 {
        let a = sway(s, vec3(x, 0.0, z), 7.0);
        let b = sway(s, vec3(x, 0.0, z + 0.05), 7.0);
        assert!((a - b).length() < 0.05, "Sprung bei z = {z}");
        max_up = max_up.max(a.y.abs());
        z += 0.05;
    }
    assert!(max_up > 0.25, "keine Steigung: {max_up}");
}
