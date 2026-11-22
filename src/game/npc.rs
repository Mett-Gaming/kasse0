//! NPCs: uncanny Kunden, gesichtslose Mitarbeiter, normale Kundschaft vor dem Kippen.
//! Navigation ueber Distanzfelder (Breitensuche) auf dem Begehbarkeitsraster der Welt.

use crate::world::{NavGrid, World};
use glam::{vec3, Vec3};
use std::collections::VecDeque;

pub const UNREACHABLE: u16 = u16::MAX;

/// Distanzfeld (in Zellen, 8er-Nachbarschaft) von `goal` aus.
pub fn bfs(nav: &NavGrid, goal: Vec3) -> Vec<u16> {
    let mut field = vec![UNREACHABLE; nav.w * nav.h];
    let Some(start) = nearest_walkable(nav, goal) else {
        return field;
    };
    field[start.1 * nav.w + start.0] = 0;
    let mut q = VecDeque::from([start]);
    while let Some((x, z)) = q.pop_front() {
        let d = field[z * nav.w + x];
        for (dx, dz) in NEIGHBORS {
            let (nx, nz) = (x as i32 + dx, z as i32 + dz);
            if nx < 0 || nz < 0 {
                continue;
            }
            let (nx, nz) = (nx as usize, nz as usize);
            if !nav.npc_walkable(nx, nz) || field[nz * nav.w + nx] != UNREACHABLE {
                continue;
            }
            // keine Ecken schneiden
            if dx != 0 && dz != 0 && !(nav.npc_walkable(x, nz) && nav.npc_walkable(nx, z)) {
                continue;
            }
            field[nz * nav.w + nx] = d + 1;
            q.push_back((nx, nz));
        }
    }
    field
}

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

pub fn nearest_walkable(nav: &NavGrid, p: Vec3) -> Option<(usize, usize)> {
    let (cx, cz) = nav.cell_of(p)?;
    if nav.npc_walkable(cx, cz) {
        return Some((cx, cz));
    }
    // Umkreis absuchen (unabhaengig von der Zellgroesse); 3 m reichen auch aus der Mitte
    // des fuer NPCs gesperrten Kassenbereichs bis zu seinem Rand
    let max_r = (3.0 / nav.cell).ceil() as i32;
    for r in 1..=max_r {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dz.abs() != r {
                    continue; // nur der Ring, das Innere wurde schon abgesucht
                }
                let (x, z) = (cx as i32 + dx, cz as i32 + dz);
                if x >= 0 && z >= 0 && nav.npc_walkable(x as usize, z as usize) {
                    return Some((x as usize, z as usize));
                }
            }
        }
    }
    None
}

#[cfg(test)]
pub fn field_at(nav: &NavGrid, field: &[u16], p: Vec3) -> u16 {
    nearest_walkable(nav, p).map_or(UNREACHABLE, |(x, z)| field[z * nav.w + x])
}

/// Bewegt `pos` entlang des Feldes Richtung Ziel. Gibt die neue Position zurueck.
pub fn step_along(
    world: &World,
    field: &[u16],
    pos: Vec3,
    goal: Vec3,
    speed: f32,
    dt: f32,
) -> Vec3 {
    let nav = &world.nav;
    let Some((cx, cz)) = nearest_walkable(nav, pos) else {
        return pos;
    };
    let here = field[cz * nav.w + cx];
    let target = if here <= 1 {
        goal
    } else {
        let mut best = (here, nav.center(cx, cz));
        for (dx, dz) in NEIGHBORS {
            let (nx, nz) = (cx as i32 + dx, cz as i32 + dz);
            if nx < 0 || nz < 0 {
                continue;
            }
            let (nx, nz) = (nx as usize, nz as usize);
            if nx >= nav.w || nz >= nav.h {
                continue;
            }
            let f = field[nz * nav.w + nx];
            if f < best.0 {
                best = (f, nav.center(nx, nz));
            }
        }
        best.1
    };
    let d = vec3(target.x - pos.x, 0.0, target.z - pos.z);
    let len = d.length();
    if len < 1e-4 {
        return pos;
    }
    let step = (speed * dt).min(len);
    let new = world.collide(pos + d / len * step, 0.25);
    // Nicht in den Kassenbereich hinein (wer schon drin ist, darf hinaus)
    if world.in_checkout_zone(new, 0.25) && !world.in_checkout_zone(pos, 0.25) {
        return pos;
    }
    new
}

pub fn yaw_towards(from: Vec3, to: Vec3) -> f32 {
    let d = to - from;
    d.x.atan2(d.z)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose {
    Idle,
    Reach,
}

pub struct Customer {
    pub pos: Vec3,
    pub yaw: f32,
    pub home: Vec3,
    pub home_facing: Vec3,
    /// Figur (Index in assets::PEOPLE)
    pub variant: usize,
    /// bewegt sich gerade (Gehen-Animation)
    pub moving: bool,
    pub tint: Vec3,
    pub seen: bool,
    pub pose: Pose,
    /// Zusatzgeschwindigkeit, wenn man ihren Tomatensaft genommen hat
    pub angry: f32,
    pub bump_cd: f32,
    /// Pause bis zum naechsten Fluestern
    pub whisper_cd: f32,
    /// Animationszeit, an die Laufstrecke gekoppelt
    pub gait: crate::people::Gait,
    /// Verfolger: schleicht sich an, sobald man wegsieht. Alle anderen kaufen nur ein.
    pub stalker: bool,
    /// Einkaufsbummel: Ziel, Wegfeld dorthin, Wartezeit am Ziel
    pub target: Vec3,
    pub field: Vec<u16>,
    pub wait: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EmpState {
    Patrol,
    Approach,
    Talking,
}

pub struct Employee {
    pub pos: Vec3,
    pub gait: crate::people::Gait,
    /// Figur in Dienstkleidung (Index in assets::PEOPLE)
    pub variant: usize,
    pub yaw: f32,
    pub state: EmpState,
    pub target: Vec3,
    pub field: Vec<u16>,
    /// Zeit, in der er den Spieler in Ruhe laesst
    pub ignore: f32,
    /// Zeit, in der er den Spieler verfolgt (nach schlechter Antwort)
    pub follow: f32,
    pub moving: bool,
    pub retarget: f32,
    /// Schiebt gerade den Wagen des Spielers zur Wagenbox
    pub carrying: bool,
    /// Hat den quietschenden Wagen gehoert und sieht nach
    pub heard: f32,
    /// Abklingzeit, bevor er wieder einen Wagen wegraeumt
    pub cart_cd: f32,
    /// Abklingzeit fuer "an ihm vorbeigerannt"
    pub sprint_cd: f32,
}

impl Employee {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            gait: Default::default(),
            variant: 0,
            yaw: 0.0,
            state: EmpState::Patrol,
            target: pos,
            field: Vec::new(),
            ignore: 8.0,
            follow: 0.0,
            moving: false,
            retarget: 0.0,
            carrying: false,
            heard: 0.0,
            cart_cd: 0.0,
            sprint_cd: 0.0,
        }
    }

    /// Wo der geschobene Wagen vor ihm steht
    pub fn cart_front(&self) -> Vec3 {
        self.pos + vec3(self.yaw.sin(), 0.0, self.yaw.cos()) * 0.85
    }
}

/// Normale Kundschaft im noch normalen Markt.
pub struct Shopper {
    pub pos: Vec3,
    pub yaw: f32,
    pub target: Vec3,
    pub field: Vec<u16>,
    pub tint: Vec3,
    /// Figur (Index in assets::PEOPLE)
    pub variant: usize,
    pub wait: f32,
    pub whisper_cd: f32,
    pub gait: crate::people::Gait,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{generate, WorldParams};

    #[test]
    fn npc_walks_to_goal_around_shelves() {
        let mut p = WorldParams::normal(21);
        p.tipped = true;
        p.stretch = 1.6;
        let w = generate(&p);
        let goal = w.spawn;
        let field = bfs(&w.nav, goal);
        // Start im Getraenkeraum
        let mut pos = vec3(w.drinks_zone.min.x + 1.7, 0.0, w.drinks_zone.max.z - 0.8);
        assert_ne!(field_at(&w.nav, &field, pos), UNREACHABLE);
        for _ in 0..(60 * 120) {
            pos = step_along(&w, &field, pos, goal, 1.5, 1.0 / 60.0);
        }
        assert!(
            (pos - goal).length() < 1.0,
            "nicht angekommen: {pos:?} vs {goal:?}"
        );
    }
}
