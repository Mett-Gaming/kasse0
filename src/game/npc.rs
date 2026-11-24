//! NPCs: uncanny Kunden, gesichtslose Mitarbeiter, normale Kundschaft vor dem Kippen.
//! Navigation ueber Distanzfelder auf dem Begehbarkeitsraster der Welt: gewichtet (Diagonale
//! kostet ~sqrt(2)), Wege werden beim Gehen geglaettet (Blick entlang des Feldes voraus, die
//! am weitesten frei sichtbare Zelle wird direkt angesteuert: kein Zickzack mehr).

use crate::world::{NavGrid, World};
use glam::{vec3, Vec3};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub const UNREACHABLE: u16 = u16::MAX;
/// Kosten eines geraden bzw. diagonalen Schritts im Distanzfeld
pub const STRAIGHT: u16 = 5;
pub const DIAGONAL: u16 = 7;
/// So viele Zellen schaut ein NPC entlang des Feldes voraus (Wegglaettung)
const LOOKAHEAD: usize = 12;

/// Distanzfeld (gewichtet, 8er-Nachbarschaft) von `goal` aus.
pub fn bfs(nav: &NavGrid, goal: Vec3) -> Vec<u16> {
    let mut field = Vec::new();
    bfs_into(nav, goal, &mut field);
    field
}

/// Wie `bfs`, schreibt aber in einen vorhandenen Puffer (spart die Neuanlage).
pub fn bfs_into(nav: &NavGrid, goal: Vec3, field: &mut Vec<u16>) {
    bfs_blocked(nav, goal, field, |_, _| false);
}

/// Distanzfeld mit zusaetzlich gesperrten Zellen (z. B. Durchgaenge, durch die der Wagen
/// nicht passt).
pub fn bfs_blocked(
    nav: &NavGrid,
    goal: Vec3,
    field: &mut Vec<u16>,
    blocked: impl Fn(usize, usize) -> bool,
) {
    field.clear();
    field.resize(nav.w * nav.h, UNREACHABLE);
    let Some(start) = nearest_walkable(nav, goal) else {
        return;
    };
    field[start.1 * nav.w + start.0] = 0;
    let mut q = BinaryHeap::new();
    q.push(Reverse((0u16, start.0 as u32, start.1 as u32)));
    while let Some(Reverse((d, x, z))) = q.pop() {
        let (x, z) = (x as usize, z as usize);
        if d > field[z * nav.w + x] {
            continue;
        }
        for (dx, dz) in NEIGHBORS {
            let (nx, nz) = (x as i32 + dx, z as i32 + dz);
            if nx < 0 || nz < 0 {
                continue;
            }
            let (nx, nz) = (nx as usize, nz as usize);
            if !nav.npc_walkable(nx, nz) || blocked(nx, nz) {
                continue;
            }
            let diag = dx != 0 && dz != 0;
            // keine Ecken schneiden
            if diag && !(nav.npc_walkable(x, nz) && nav.npc_walkable(nx, z)) {
                continue;
            }
            let nd = d.saturating_add(if diag { DIAGONAL } else { STRAIGHT });
            let slot = &mut field[nz * nav.w + nx];
            if nd < *slot {
                *slot = nd;
                q.push(Reverse((nd, nx as u32, nz as u32)));
            }
        }
    }
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

/// Frei begehbarer gerader Streifen zwischen zwei Punkten auf dem Raster? Der Streifen ist
/// eine Zelle breiter als die Linie selbst, damit Abkuerzungen nicht an Ecken haengen bleiben.
fn grid_line_free(nav: &NavGrid, a: Vec3, b: Vec3) -> bool {
    let d = vec3(b.x - a.x, 0.0, b.z - a.z);
    let len = d.length();
    if len < 1e-4 {
        return true;
    }
    let side = vec3(-d.z, 0.0, d.x) / len * nav.cell;
    let steps = (len / (nav.cell * 0.5)).ceil().max(1.0) as usize;
    let free = |p: Vec3| nav.cell_of(p).is_some_and(|(x, z)| nav.npc_walkable(x, z));
    (0..=steps).all(|k| {
        let p = a + d * (k as f32 / steps as f32);
        free(p) && free(p + side) && free(p - side)
    })
}

/// Bester Nachbar im Feld (kleinster Wert), None = Ziel erreicht oder Sackgasse.
fn downhill(nav: &NavGrid, field: &[u16], (cx, cz): (usize, usize)) -> Option<(usize, usize)> {
    // Feld vom alten Markt (vor einem Umbau, anderes Raster): nicht benutzbar
    if field.len() != nav.w * nav.h {
        return None;
    }
    let here = field[cz * nav.w + cx];
    let mut best = (here, None);
    for (dx, dz) in NEIGHBORS {
        let (nx, nz) = (cx as i32 + dx, cz as i32 + dz);
        if nx < 0 || nz < 0 || nx as usize >= nav.w || nz as usize >= nav.h {
            continue;
        }
        let (nx, nz) = (nx as usize, nz as usize);
        let f = field[nz * nav.w + nx];
        if f < best.0 {
            best = (f, Some((nx, nz)));
        }
    }
    best.1
}

/// Startzelle eines NPCs im Feld: die eigene, wenn begehbar; sonst unter den Nachbarzellen
/// die, die dem Ziel am naechsten ist (nicht einfach die naechste: die koennte hinter einem
/// duennen Regal liegen und einen weiten Umweg bedeuten).
fn start_cell(nav: &NavGrid, field: &[u16], pos: Vec3) -> Option<(usize, usize)> {
    let (cx, cz) = nav.cell_of(pos)?;
    if nav.npc_walkable(cx, cz) || field.len() != nav.w * nav.h {
        return nearest_walkable(nav, pos);
    }
    let mut best: Option<(u16, (usize, usize))> = None;
    for dz in -2i32..=2 {
        for dx in -2i32..=2 {
            let (x, z) = (cx as i32 + dx, cz as i32 + dz);
            if x < 0 || z < 0 || x as usize >= nav.w || z as usize >= nav.h {
                continue;
            }
            let (x, z) = (x as usize, z as usize);
            let f = field[z * nav.w + x];
            if nav.npc_walkable(x, z) && f != UNREACHABLE && best.is_none_or(|b| f < b.0) {
                best = Some((f, (x, z)));
            }
        }
    }
    best.map(|b| b.1).or_else(|| nearest_walkable(nav, pos))
}

/// Wohin ein NPC an `pos` auf dem Feld gerade zusteuert: die am weitesten voraus liegende
/// Zelle, die noch in gerader Linie frei erreichbar ist (oder das Ziel selbst).
pub fn steer_point(world: &World, field: &[u16], pos: Vec3, goal: Vec3) -> Vec3 {
    let nav = &world.nav;
    // kein (oder ein veraltetes) Feld nach einem Umbau des Markts: direkt aufs Ziel zu,
    // bis das Feld neu berechnet ist
    if field.len() != nav.w * nav.h {
        return goal;
    }
    let Some(start) = start_cell(nav, field, pos) else {
        return goal;
    };
    let Some(&here) = field.get(start.1 * nav.w + start.0) else {
        return goal; // noch kein Feld berechnet: direkt aufs Ziel zu
    };
    if here <= STRAIGHT {
        return goal;
    }
    let mut aim = None;
    let mut cell = start;
    for _ in 0..LOOKAHEAD {
        let Some(next) = downhill(nav, field, cell) else {
            break;
        };
        cell = next;
        let p = nav.center(cell.0, cell.1);
        if field[cell.1 * nav.w + cell.0] <= STRAIGHT && grid_line_free(nav, pos, goal) {
            return goal;
        }
        if grid_line_free(nav, pos, p) {
            aim = Some(p);
        } else if aim.is_some() {
            break;
        }
    }
    aim.unwrap_or_else(|| {
        // nichts frei sichtbar (z. B. an einer Ecke): zum direkten Nachbarn
        downhill(nav, field, start).map_or(goal, |(x, z)| nav.center(x, z))
    })
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
    step_along_r(world, field, pos, goal, speed, dt, 0.25)
}

/// Wie `step_along`, mit eigenem Kollisionsradius (z. B. der rollende Wagen).
pub fn step_along_r(
    world: &World,
    field: &[u16],
    pos: Vec3,
    goal: Vec3,
    speed: f32,
    dt: f32,
    radius: f32,
) -> Vec3 {
    let go = |target: Vec3| {
        let d = vec3(target.x - pos.x, 0.0, target.z - pos.z);
        let len = d.length();
        if len < 1e-4 {
            return (pos, 0.0);
        }
        let step = (speed * dt).min(len);
        let new = world.collide(pos + d / len * step, radius);
        // Nicht in den Kassenbereich hinein (wer schon drin ist, darf hinaus)
        // (gleiche Regel wie das Raster: die Koerpermitte bleibt draussen)
        if world.in_checkout_zone(new, 0.0) && !world.in_checkout_zone(pos, 0.0) {
            return (pos, step);
        }
        (new, step)
    };
    let (new, want) = go(steer_point(world, field, pos, goal));
    if want <= 0.0 || (new - pos).length() > want * 0.3 {
        return new;
    }
    // an einer Kante haengen geblieben: diesen Schritt Zelle fuer Zelle gehen
    let nav = &world.nav;
    let near = start_cell(nav, field, pos)
        .filter(|&(x, z)| field.get(z * nav.w + x).is_some_and(|&f| f > STRAIGHT))
        .and_then(|c| downhill(nav, field, c))
        .map_or(goal, |(x, z)| nav.center(x, z));
    go(near).0
}

/// Ist `goal` von `from` aus ueberhaupt zu Fuss erreichbar (laut Feld zu `goal`)?
pub fn reachable(nav: &NavGrid, field: &[u16], from: Vec3) -> bool {
    nearest_walkable(nav, from)
        .is_some_and(|(x, z)| field.get(z * nav.w + x).is_some_and(|&f| f != UNREACHABLE))
}

/// Erkennt, ob ein NPC feststeckt: will gehen, kommt aber in 1,5 s keine 20 cm voran.
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    t: f32,
    anchor: Vec3,
}

impl Motion {
    pub const WINDOW: f32 = 1.5;

    /// Pro Bild aufrufen. `wants` = der NPC moechte sich gerade bewegen.
    pub fn stuck(&mut self, pos: Vec3, wants: bool, dt: f32) -> bool {
        if !wants {
            self.t = 0.0;
            self.anchor = pos;
            return false;
        }
        self.t += dt;
        if self.t < Self::WINDOW {
            return false;
        }
        let moved = vec3(pos.x - self.anchor.x, 0.0, pos.z - self.anchor.z).length();
        self.t = 0.0;
        self.anchor = pos;
        moved < 0.2
    }
}

/// Fester Zufallswert 0..10 aus einer Startposition (Animationsversatz einer Figur).
pub fn seed_of(p: Vec3) -> f32 {
    ((p.x * 12.9898 + p.z * 78.233 + p.y * 37.719).sin() * 43_758.547)
        .fract()
        .abs()
        * 10.0
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
    /// steht still und starrt den Spieler an (nach "hinter dir"), solange > 0
    pub stare: f32,
    /// Animationszeit, an die Laufstrecke gekoppelt
    pub gait: crate::people::Gait,
    /// Clip-Wechsel mit Ueberblenden
    pub anim: crate::people::Animator,
    /// fester Zufallswert der Figur (Animationsversatz, Zucken)
    pub seed: f32,
    /// Wahrnehmung, Gedaechtnis, Feststecken (senses.rs)
    pub brain: super::senses::Brain,
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
    /// Clip-Wechsel mit Ueberblenden
    pub anim: crate::people::Animator,
    /// fester Zufallswert der Figur (Animationsversatz, Zucken)
    pub seed: f32,
    /// Wahrnehmung, Gedaechtnis, Feststecken (senses.rs)
    pub brain: super::senses::Brain,
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
    /// schon gewuerfelt, ob er auf den Spieler zukommt (70 %, siehe crowd.rs)
    pub approach_rolled: bool,
}

impl Employee {
    pub fn new(pos: Vec3) -> Self {
        Self {
            pos,
            gait: Default::default(),
            anim: Default::default(),
            seed: seed_of(pos),
            brain: Default::default(),
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
            approach_rolled: false,
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
    /// Clip-Wechsel mit Ueberblenden
    pub anim: crate::people::Animator,
    /// fester Zufallswert der Figur (Animationsversatz, Zucken)
    pub seed: f32,
    /// Wahrnehmung, Gedaechtnis, Feststecken (senses.rs)
    pub brain: super::senses::Brain,
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
