use super::*;

fn corridor(seed: u64, len: f32) -> (Corridor, Aabb) {
    let mut b = Builder::default();
    build(&mut b, seed, len, 10.0, 50.0)
}

#[test]
fn length_grows_with_the_time_in_the_run() {
    assert_eq!(corridor_len_for(0.0), 40.0);
    assert_eq!(corridor_len_for(300.0), 40.0);
    assert!((corridor_len_for(420.0) - 56.0).abs() < 1e-3);
    assert_eq!(corridor_len_for(3600.0), 140.0);
    // und der gebaute Gang ist auch so lang
    for len in [40.0, 56.0, 100.0, 140.0] {
        for seed in 0..6 {
            let (c, _) = corridor(seed, len);
            assert!(
                (c.len - len).abs() < 6.5,
                "Seed {seed}: {} statt {len} m",
                c.len
            );
        }
    }
}

#[test]
fn path_turns_left_and_right_but_never_goes_back() {
    let (mut left, mut right) = (false, false);
    for seed in 0..20 {
        let (c, _) = corridor(seed, 90.0);
        assert!(c.segments() >= 3, "keine Kurven");
        for i in 0..c.segments() {
            let (_, d, l, _) = c.segment(i);
            assert!(d.z > -0.5, "Gang laeuft zurueck");
            // achsparallel, Stuecke lang genug fuer Ecken
            assert!(d.x.abs() > 0.99 || d.z.abs() > 0.99);
            assert!(l >= 6.9, "Stueck {i} nur {l} m");
            left |= d.x > 0.5;
            right |= d.x < -0.5;
        }
        // letztes Stueck vorwaerts, Spiegel am Ende schaut zurueck
        let (_, d, _, _) = c.segment(c.segments() - 1);
        assert!(d.z > 0.99);
        assert!((c.mirror_center.z - c.pts.last().unwrap().z).abs() < 0.1);
        // nie vor der Ladentuer (z waechst nur)
        assert!(c.pts.windows(2).all(|w| w[1].z >= w[0].z - 1e-4));
    }
    assert!(left && right, "Kurven gehen nur in eine Richtung");
}

#[test]
fn progress_point_and_direction_fit_together() {
    let (c, _) = corridor(7, 120.0);
    let mut s = 0.5;
    while s < c.len - 0.5 {
        let p = c.point_at(s);
        assert!(c.contains(p), "Punkt bei {s} m nicht im Gang");
        assert!((c.progress(p) - s).abs() < 0.05, "Fortschritt bei {s} m");
        // nach vorn schauen ist "vorn", zurueck ist "zurueck"
        let d = c.dir_at_s(s);
        assert!(c.facing(p, d) > 0.99);
        assert!(c.facing(p, -d) < -0.5 || c.facing(p, -d) < 0.1);
        s += 0.7;
    }
    // neben dem Gang ist nicht im Gang
    let p = c.point_at(10.0) + side_of(c.dir_at_s(10.0)) * (CORRIDOR_HALF_W + 1.0);
    assert!(!c.contains(p));
}

#[test]
fn stations_and_hurdles_lie_in_the_corridor() {
    for seed in 0..10 {
        let (c, exit) = corridor(seed, 140.0);
        let things = c.stations.len() + c.hurdles.len() + c.squeezes.len();
        assert!(things >= 10, "zu wenig los: {things}");
        for st in &c.stations {
            assert!(c.contains(st.at), "Station ausserhalb");
            assert!((c.progress(st.at) - st.s).abs() < 0.1);
        }
        // genau eine Ausgangstuer, am Ende
        let exits: Vec<_> = c
            .stations
            .iter()
            .filter(|s| s.kind == StationKind::ExitDoor)
            .collect();
        assert_eq!(exits.len(), 1);
        assert!(exits[0].s > c.end_s);
        // Huerden ueber die ganze Breite: man kommt nicht seitlich vorbei
        for h in &c.hurdles {
            let w = (h.area.max.x - h.area.min.x).max(h.area.max.z - h.area.min.z);
            assert!(w >= 2.0 * CORRIDOR_HALF_W - 1e-3);
            if h.duck {
                assert!(h.area.min.y > 1.0 && h.area.min.y < 1.25);
            } else {
                assert!(h.area.max.y <= 0.5);
            }
        }
        // Ausgangsraum neben dem letzten Stueck
        assert!(exit.min.z > c.pts[c.segments() - 1].z);
    }
}

#[test]
fn every_run_gets_a_different_corridor() {
    let mut shapes = std::collections::HashSet::new();
    for seed in 0..12 {
        let (c, _) = corridor(seed * 7919, 80.0);
        let key: Vec<i32> = c.pts.iter().map(|p| (p.x * 10.0) as i32).collect();
        shapes.insert(key);
    }
    assert!(shapes.len() > 9, "Gaenge kaum zufaellig: {}", shapes.len());
}
