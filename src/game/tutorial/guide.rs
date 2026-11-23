//! Wegweiser im Tutorial: Pfeile am Boden zum naechsten Ziel und rot umrandete Waren, die
//! noch auf dem Zettel fehlen. Der Weg kommt aus einem eigenen Distanzfeld ueber das
//! Begehbarkeitsraster (wie bei den NPCs, aber der Kassenbereich ist fuer den Spieler offen).
//! Gezeichnet wird alles aus Boxen (draws.rs: collect_guide_draws).

use super::*;
use crate::world::NavGrid;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Waren in diesem Umkreis werden umrandet, hoechstens so viele
pub const MARK_RADIUS: f32 = 12.0;
pub const MARK_MAX: usize = 40;
/// Abstand der Pfeile entlang des Wegs, Anzahl sichtbarer Pfeile
pub const ARROW_STEP: f32 = 1.5;
pub const ARROWS: usize = 5;
/// So oft wird der Weg vom Spieler aus neu abgelesen (Sekunden)
const PATH_EVERY: f32 = 0.2;
/// Ziel hat sich um mehr als das bewegt: Distanzfeld neu berechnen
const GOAL_MOVED: f32 = 0.3;

const FAR: u16 = u16::MAX;
const NEIGHBORS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

#[derive(Default)]
pub struct Guide {
    field: Vec<u16>,
    goal: Option<Vec3>,
    timer: f32,
    /// Pfeile: (Mitte am Boden, Richtung in der Ebene)
    pub arrows: Vec<(Vec3, Vec3)>,
    /// Indizes in `world.items`, die umrandet werden
    pub marked: Vec<usize>,
}

/// Naechste begehbare Zelle (Spieler, nicht NPC) um `p`.
fn nearest_cell(nav: &NavGrid, p: Vec3) -> Option<(usize, usize)> {
    let (cx, cz) = {
        let x = ((p.x - nav.origin.x) / nav.cell).floor() as i32;
        let z = ((p.z - nav.origin.z) / nav.cell).floor() as i32;
        (x, z)
    };
    for r in 0..16i32 {
        let mut best: Option<((usize, usize), f32)> = None;
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue;
                }
                let (x, z) = (cx + dx, cz + dz);
                if x < 0 || z < 0 || !nav.is_walkable(x as usize, z as usize) {
                    continue;
                }
                let c = nav.center(x as usize, z as usize);
                let d = (c - vec3(p.x, 0.0, p.z)).length_squared();
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some(((x as usize, z as usize), d));
                }
            }
        }
        if let Some((c, _)) = best {
            return Some(c);
        }
    }
    None
}

/// Distanzfeld (gewichtet, 8er-Nachbarschaft) zum Ziel ueber alle fuer den Spieler
/// begehbaren Zellen.
fn distance_field(nav: &NavGrid, goal: Vec3, field: &mut Vec<u16>) {
    field.clear();
    field.resize(nav.w * nav.h, FAR);
    let Some(start) = nearest_cell(nav, goal) else {
        return;
    };
    field[start.1 * nav.w + start.0] = 0;
    let mut q = BinaryHeap::new();
    q.push(Reverse((0u16, start.0, start.1)));
    while let Some(Reverse((d, x, z))) = q.pop() {
        if d > field[z * nav.w + x] {
            continue;
        }
        for (dx, dz) in NEIGHBORS {
            let (nx, nz) = (x as i32 + dx, z as i32 + dz);
            if nx < 0 || nz < 0 || !nav.is_walkable(nx as usize, nz as usize) {
                continue;
            }
            let diag = dx != 0 && dz != 0;
            // diagonal nur, wenn beide Nachbarn frei sind (nicht durch Ecken schneiden)
            if diag && !(nav.is_walkable(nx as usize, z) && nav.is_walkable(x, nz as usize)) {
                continue;
            }
            let nd = d.saturating_add(if diag { 7 } else { 5 });
            let i = nz as usize * nav.w + nx as usize;
            if nd < field[i] {
                field[i] = nd;
                q.push(Reverse((nd, nx as usize, nz as usize)));
            }
        }
    }
}

/// Weg vom Spieler feld-abwaerts bis zum Ziel (Zellmitten).
fn descend(nav: &NavGrid, field: &[u16], from: Vec3) -> Vec<Vec3> {
    let mut path = Vec::new();
    let Some((mut x, mut z)) = nearest_cell(nav, from) else {
        return path;
    };
    if field.len() != nav.w * nav.h || field[z * nav.w + x] == FAR {
        return path;
    }
    path.push(vec3(from.x, 0.0, from.z));
    for _ in 0..600 {
        let here = field[z * nav.w + x];
        if here == 0 {
            break;
        }
        let mut best = (here, x, z);
        for (dx, dz) in NEIGHBORS {
            let (nx, nz) = (x as i32 + dx, z as i32 + dz);
            if nx < 0 || nz < 0 || nx as usize >= nav.w || nz as usize >= nav.h {
                continue;
            }
            let v = field[nz as usize * nav.w + nx as usize];
            if v < best.0 {
                best = (v, nx as usize, nz as usize);
            }
        }
        if best.0 >= here {
            break;
        }
        (x, z) = (best.1, best.2);
        path.push(nav.center(x, z));
    }
    path
}

/// Pfeile alle ARROW_STEP Meter entlang des Wegs (der erste ein Stueck vor dem Spieler).
pub fn arrows_along(path: &[Vec3]) -> Vec<(Vec3, Vec3)> {
    let mut out = Vec::new();
    let mut next = ARROW_STEP;
    let mut walked = 0.0;
    for w in path.windows(2) {
        let seg = w[1] - w[0];
        let len = seg.length();
        if len < 1e-4 {
            continue;
        }
        let dir = seg / len;
        while walked + len >= next && out.len() < ARROWS {
            out.push((w[0] + dir * (next - walked), dir));
            next += ARROW_STEP;
        }
        walked += len;
        if out.len() >= ARROWS {
            break;
        }
    }
    out
}

impl Game {
    /// Echte, noch nicht genommene Waren fuer fehlende Zettel-Eintraege der aktuellen Phase.
    fn guide_candidates(&self, phase: Phase) -> Vec<usize> {
        let cat = catalog();
        let entries: Vec<usize> = match phase {
            // im normalen Markt zaehlt nur das Wasser
            Phase::Controls | Phase::Water => self.list[..1].to_vec(),
            Phase::Shopping => self.list.clone(),
            _ => return Vec::new(),
        };
        let missing: Vec<usize> = entries
            .into_iter()
            .filter(|&e| {
                !self.list_done(e) && self.held.as_ref().and_then(|h| h.list_entry()) != Some(e)
            })
            .collect();
        if missing.is_empty() {
            return Vec::new();
        }
        let all: Vec<usize> = self
            .world
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                !it.fake
                    && !it.taken
                    && cat.products[it.product as usize]
                        .list
                        .is_some_and(|l| missing.contains(&l))
            })
            .map(|(i, _)| i)
            .collect();
        // im normalen Markt: nur das Wasser im Getraenkeraum (nicht irgendeins im Gang),
        // denn dort hindurch kippt der Markt
        if matches!(phase, Phase::Controls | Phase::Water) {
            let zone = &self.world.drinks_zone;
            let inside: Vec<usize> = all
                .iter()
                .copied()
                .filter(|&i| zone.contains_xz(self.world.items[i].pos))
                .collect();
            if !inside.is_empty() {
                return inside;
            }
        }
        all
    }

    /// Wohin die Pfeile gerade zeigen (None = keine Pfeile).
    pub fn guide_goal(&self) -> Option<Vec3> {
        let phase = self.tutorial_phase()?;
        let nearest = |c: &[usize]| {
            c.iter().map(|&i| self.world.items[i].pos).min_by(|a, b| {
                (*a - self.player.pos)
                    .length_squared()
                    .total_cmp(&(*b - self.player.pos).length_squared())
            })
        };
        match phase {
            // schon waehrend der Tasten-Liste: Pfeile zum Wasser
            Phase::Controls | Phase::Water => {
                let c = self.guide_candidates(phase);
                // Wasser im Wagen (oder in der Hand): raus aus dem Getraenkeraum
                nearest(&c).or_else(|| Some(self.world.drinks_outside()))
            }
            Phase::Shopping => nearest(&self.guide_candidates(phase)),
            Phase::ToTill => self.open_lane().map(|l| l.scan_point),
            // durch den Korridor bis kurz vor sein Ende, ohne Korridor zur Ladentuer
            Phase::Escape => Some(match self.world.corridor {
                Some(c) => vec3((c.x0 + c.x1) * 0.5, 0.0, c.z1 - 1.0),
                None => self.world.door_center - vec3(0.0, 0.0, 1.0),
            }),
            _ => None,
        }
    }

    /// Umrandete Waren und Pfeile nachfuehren.
    pub(super) fn update_guide(&mut self, dt: f32) {
        let Some(phase) = self.tutorial_phase() else {
            return;
        };
        // umranden: die naechsten fehlenden Waren im Umkreis
        let mut marked: Vec<usize> = self
            .guide_candidates(phase)
            .into_iter()
            .filter(|&i| (self.world.items[i].pos - self.player.pos).length() < MARK_RADIUS)
            .collect();
        let p = self.player.pos;
        marked.sort_by(|&a, &b| {
            let d = |i: usize| (self.world.items[i].pos - p).length_squared();
            d(a).total_cmp(&d(b))
        });
        marked.truncate(MARK_MAX);

        let goal = self.guide_goal();
        let nav = &self.world.nav;
        let Some(t) = &mut self.tutorial else { return };
        let g = &mut t.guide;
        g.marked = marked;
        let Some(goal) = goal else {
            g.goal = None;
            g.arrows.clear();
            return;
        };
        let moved = g.goal.is_none_or(|old| (old - goal).length() > GOAL_MOVED);
        if moved || g.field.len() != nav.w * nav.h {
            distance_field(nav, goal, &mut g.field);
            g.goal = Some(goal);
            g.timer = 0.0;
        }
        g.timer -= dt;
        if g.timer <= 0.0 {
            g.timer = PATH_EVERY;
            g.arrows = arrows_along(&descend(nav, &g.field, p));
        }
    }
}

/// So hoch schweben die Pfeile ueber dem (ggf. verzerrten) Boden
pub const ARROW_LIFT: f32 = 0.02;

impl Game {
    /// Punkt knapp ueber dem Boden an dieser Stelle - auch wenn der gekippte Markt den Boden
    /// gerade sinken, steigen oder seitlich wandern laesst (gleiche Verschiebung wie im Shader).
    pub fn on_floor(&self, p: Vec3) -> Vec3 {
        let g = vec3(p.x, 0.0, p.z);
        g + self.sway_at(g) + vec3(0.0, ARROW_LIFT, 0.0)
    }

    /// Pfeile am Boden und rote Rahmen um die benoetigten Waren (alles aus Boxen).
    pub(in crate::game) fn collect_guide_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some(g) = self.tutorial.as_ref().map(|t| &t.guide) else {
            return;
        };
        let cube = MeshId::Cube as usize;
        let mut bar = |size: Vec3, rot: Quat, pos: Vec3, tint: Vec4| {
            out.push(MeshDraw {
                mesh: cube,
                model: Mat4::from_scale_rotation_translation(size, rot, pos),
                tint,
            });
        };
        // Pfeile: Schaft und zwei schraege Spitzen, leuchten gelb-gruen und pulsieren nach vorn
        for (k, &(p, dir)) in g.arrows.iter().enumerate() {
            let k = k as f32;
            // im gekippten Markt flackern sie ab und zu weg
            if self.tipped && (self.time * 13.0 + k * 2.3).sin() > 0.96 {
                continue;
            }
            let glow = 0.7 + 0.5 * (self.time * 6.0 - k * 1.2).sin().max(0.0);
            let tint = Vec4::new(0.75, 1.0, 0.2, glow);
            // Balken von a nach b, beide Enden folgen dem verzerrten Boden (sinkt/steigt,
            // verschiebt sich seitlich): der Pfeil liegt schraeg mit auf und steckt nie darin
            let mut seg = |a: Vec3, b: Vec3| {
                let (a, b) = (self.on_floor(a), self.on_floor(b));
                let d = b - a;
                let rot = Quat::from_rotation_arc(Vec3::Z, d.normalize_or(Vec3::Z));
                bar(vec3(0.07, 0.01, d.length()), rot, (a + b) * 0.5, tint);
            };
            seg(p - dir * 0.225, p + dir * 0.225);
            let tip = p + dir * 0.24;
            for s in [-1.0f32, 1.0] {
                let d = Quat::from_rotation_y(s * 2.5) * dir;
                seg(tip, tip + d * 0.23);
            }
        }
        // rote Rahmen: 12 duenne Kanten um die Bounding-Box jeder benoetigten Ware
        let red = Vec4::new(1.0, 0.08, 0.06, 1.3);
        let t = 0.008;
        for &i in &g.marked {
            let it = &self.world.items[i];
            let h = it.half() + Vec3::splat(0.012);
            let rot = Quat::from_rotation_y(it.yaw);
            // das Regal bewegt sich mit dem verzerrten Boden, der Rahmen auch
            let c = it.pos + vec3(0.0, it.half().y, 0.0) + self.sway_at(it.pos);
            for (a, b) in [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
                // Kanten entlang x, y und z
                bar(
                    vec3(h.x * 2.0 + t, t, t),
                    rot,
                    c + rot * vec3(0.0, a * h.y, b * h.z),
                    red,
                );
                bar(
                    vec3(t, h.y * 2.0 + t, t),
                    rot,
                    c + rot * vec3(a * h.x, 0.0, b * h.z),
                    red,
                );
                bar(
                    vec3(t, t, h.z * 2.0 + t),
                    rot,
                    c + rot * vec3(a * h.x, b * h.y, 0.0),
                    red,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests::shelf_item;

    fn tutorial_game(seed: u64) -> Game {
        let mut g = Game::new_tutorial(seed);
        g.customers.clear();
        g.employees.clear();
        g.shoppers.clear();
        g
    }

    /// Den Pfeilen folgen: wird man dem Ziel naeher?
    fn arrows_lead_to_goal(g: &mut Game) {
        g.update_guide(1.0);
        let goal = g.guide_goal().expect("kein Ziel");
        let arrows = &g.tutorial.as_ref().unwrap().guide.arrows;
        assert!(
            !arrows.is_empty(),
            "keine Pfeile: Phase {:?}, Spieler {}, Ziel {goal}, Feld da {}",
            g.tutorial_phase(),
            g.player.pos,
            g.tutorial
                .as_ref()
                .unwrap()
                .guide
                .field
                .iter()
                .any(|&v| v != FAR)
        );
        let field = &g.tutorial.as_ref().unwrap().guide.field;
        let nav = &g.world.nav;
        let at = |p: Vec3| {
            nearest_cell(nav, p)
                .map(|(x, z)| field[z * nav.w + x])
                .unwrap()
        };
        let start = at(g.player.pos);
        let last = at(arrows.last().unwrap().0);
        assert!(last < start, "Pfeile fuehren nicht naeher ans Ziel {goal}");
    }

    #[test]
    fn arrows_point_the_way_in_every_phase_with_a_goal() {
        let mut g = tutorial_game(301);
        // gleich am Anfang (Tasten-Liste) zeigen die Pfeile zum Wasser
        assert_eq!(g.tutorial_phase(), Some(Phase::Controls));
        arrows_lead_to_goal(&mut g);
        // Phase Wasser: Tasten erledigt
        g.tutorial.as_mut().unwrap().step = STEPS.len();
        assert_eq!(g.tutorial_phase(), Some(Phase::Water));
        arrows_lead_to_goal(&mut g);
        // Einkaufen (gekippt)
        g.tip();
        assert_eq!(g.tutorial_phase(), Some(Phase::Shopping));
        arrows_lead_to_goal(&mut g);
        // Zettel komplett: zur Kasse
        for e in g.list.clone() {
            if !g.list_done(e) {
                let i = shelf_item(&g, e, false).unwrap();
                g.take(i);
            }
        }
        assert_eq!(g.tutorial_phase(), Some(Phase::ToTill));
        arrows_lead_to_goal(&mut g);
        assert!(g.tutorial.as_ref().unwrap().guide.marked.is_empty());
        // Flucht: bezahlen (oeffnet den Korridor), dann zum Ausgang
        let lane = g.open_lane().unwrap();
        g.player.pos = lane.scan_point;
        g.scanned.append(&mut g.cart);
        g.pay();
        g.customers.clear();
        g.employees.clear();
        assert_eq!(g.tutorial_phase(), Some(Phase::Escape));
        arrows_lead_to_goal(&mut g);
    }

    #[test]
    fn only_missing_real_list_items_are_marked() {
        let mut g = tutorial_game(302);
        g.tutorial.as_mut().unwrap().step = STEPS.len();
        let water = g.list[0];
        // direkt neben ein Wasser im Getraenkeraum stellen
        let cat = catalog();
        let zone = g.world.drinks_zone;
        let w = (0..g.world.items.len())
            .find(|&i| {
                let it = &g.world.items[i];
                !it.fake
                    && cat.products[it.product as usize].list == Some(water)
                    && zone.contains_xz(it.pos)
            })
            .expect("kein Wasser im Getraenkeraum");
        g.player.pos = g.world.items[w].pos + vec3(0.0, 0.0, 1.0);
        g.update_guide(0.1);
        let marked = g.tutorial.as_ref().unwrap().guide.marked.clone();
        assert!(!marked.is_empty(), "Wasser nicht umrandet");
        let cat = catalog();
        for &i in &marked {
            let it = &g.world.items[i];
            assert!(!it.fake && !it.taken);
            assert_eq!(cat.products[it.product as usize].list, Some(water));
        }
        // eingepackt: kein Wasser mehr umrandet, Pfeile fuehren hinaus
        g.take(w);
        g.update_guide(0.1);
        assert!(g.tutorial.as_ref().unwrap().guide.marked.is_empty());
        assert_eq!(g.guide_goal(), Some(g.world.drinks_outside()));
    }

    #[test]
    fn arrows_lead_to_the_water_in_the_drinks_room() {
        // auch wenn im Gang Wasser steht: die Pfeile zeigen in den Getraenkeraum
        for seed in [304, 305, 306] {
            let mut g = tutorial_game(seed);
            let zone = g.world.drinks_zone;
            let goal = g.guide_goal().expect("kein Ziel am Start");
            assert!(
                zone.contains_xz(goal),
                "Seed {seed}: Ziel {goal} nicht im Getraenkeraum"
            );
            // auch die Rahmen nur dort
            g.player.pos = goal + vec3(0.0, 0.0, 1.0);
            g.update_guide(0.1);
            let marked = &g.tutorial.as_ref().unwrap().guide.marked;
            assert!(!marked.is_empty());
            assert!(marked
                .iter()
                .all(|&i| zone.contains_xz(g.world.items[i].pos)));
        }
    }

    #[test]
    fn normal_run_has_no_guide() {
        let mut g = Game::new(303);
        assert!(g.tutorial.is_none() && !g.tutorial_run);
        g.update_guide(1.0);
        assert_eq!(g.guide_goal(), None);
        assert_eq!(g.tutorial_phase(), None);
    }

    #[test]
    fn arrows_ride_on_the_warped_floor() {
        // Kollaps: der Boden sinkt und steigt bis zu 0,55 m - die Pfeile liegen immer knapp
        // darueber, an beiden Enden (nicht im Boden, nicht in der Luft)
        let mut g = tutorial_game(307);
        g.tip();
        g.force_tier(crate::game::escalation::Tier::Collapse);
        let mut moved = false;
        for k in 0..40 {
            g.time = k as f32 * 0.37;
            let p = vec3(
                g.world.aisle_x1 * 0.5,
                0.0,
                (g.world.aisle_z0 + g.world.aisle_z1) * 0.5,
            ) + vec3(0.0, 0.0, k as f32 * 0.4 - 8.0);
            let floor = g.sway_at(p);
            moved |= floor.y.abs() > 0.1;
            let a = g.on_floor(p);
            assert!(
                (a.y - (floor.y + ARROW_LIFT)).abs() < 1e-5,
                "Pfeil nicht auf dem Boden"
            );
            assert!(
                (a.x - (p.x + floor.x)).abs() < 1e-5,
                "Pfeil wandert nicht mit"
            );
        }
        assert!(moved, "Boden verzerrt sich im Test gar nicht");
        // Pfeile werden gezeichnet (Schaft + 2 Spitzen je Pfeil)
        g.update_guide(1.0);
        let n = g.tutorial.as_ref().unwrap().guide.arrows.len();
        let mut out = Vec::new();
        g.collect_guide_draws(&mut out);
        assert!(out.len() >= n * 3);
    }

    #[test]
    fn arrows_are_spaced_along_the_path() {
        let path = [Vec3::ZERO, vec3(0.0, 0.0, 10.0)];
        let a = arrows_along(&path);
        assert_eq!(a.len(), ARROWS);
        assert!((a[0].0.z - ARROW_STEP).abs() < 1e-4);
        assert!((a[1].0.z - a[0].0.z - ARROW_STEP).abs() < 1e-4);
        assert!(a.iter().all(|(_, d)| (d.z - 1.0).abs() < 1e-4));
    }
}
