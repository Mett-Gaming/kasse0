//! Hindernisse im Gang: Huerden (Ducken/Springen), Schwingtueren (E), Kipp-Regale, rollende
//! Wagen, Engstellen, die zugeknallte Ladentuer. Hier stehen Bewegung, Kollision mit dem Spieler
//! und das Zeichnen (alles aus Boxen bzw. dem Wagen-Modell).

use super::*;
use crate::world::Aabb;

/// Hoechsttempo in Engstellen (seitlich quetschen)
pub const SQUEEZE_SPEED: f32 = 1.3;
/// Tuer: so lange schwingt sie auf; so oft muss man bei einer klemmenden druecken
pub const DOOR_OPEN_TIME: f32 = 0.35;
pub const STUCK_PRESSES: u32 = 3;
/// Kipp-Regal: loest so weit davor aus, faellt so lange
pub const SHELF_TRIGGER: f32 = 5.0;
pub const SHELF_FALL: f32 = 0.7;
const SHELF_H: f32 = 2.3;
const SHELF_DEEP: f32 = 0.45;
const SHELF_ALONG: f32 = 0.5;
/// Wagen: rollt los, wenn man so nah ist; Tempo; Treffer-Abstand
pub const CART_TRIGGER: f32 = 12.0;
pub const CART_SPEED: f32 = 3.0;
pub const CART_HIT: f32 = 0.7;
/// Stolpern: so lange langsamer
pub const STUMBLE: f32 = 1.0;
/// Kopf (stehend) und Spielraum ueber einer Spring-Huerde
const HEAD: f32 = 1.75;
const JUMP_SLACK: f32 = 0.15;

/// Wie ein Kasten den Spieler aufhaelt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Block {
    /// immer
    Solid,
    /// nur wer nicht tief genug geduckt ist (Unterkante)
    Duck(f32),
    /// nur wer nicht hoch genug springt (Oberkante)
    Jump(f32),
}

/// Halbe Groesse einer achsparallelen Box: `hu` entlang `dir`, `hv` quer, `hy` hoch.
pub(super) fn half_of(dir: Vec3, hu: f32, hv: f32, hy: f32) -> Vec3 {
    if dir.x.abs() > 0.5 {
        vec3(hu, hy, hv)
    } else {
        vec3(hv, hy, hu)
    }
}

/// Kasten der (geschlossenen) Tuer an Station `st`.
pub(super) fn door_box(st: &crate::world::Station) -> Aabb {
    match st.kind {
        StationKind::ExitDoor => {
            let half = (EXIT_FROM - EXIT_TO) * 0.5;
            Aabb::from_center(
                st.at + st.side * (CORRIDOR_HALF_W + 0.1) + vec3(0.0, 1.1, 0.0),
                half_of(st.dir, half, 0.06, 1.1),
            )
        }
        _ => Aabb::from_center(
            st.at + vec3(0.0, 1.1, 0.0),
            half_of(st.dir, 0.06, CORRIDOR_HALF_W, 1.1),
        ),
    }
}

impl Game {
    /// Oberkante des Kopfs (geduckt tiefer, im Sprung hoeher).
    pub fn head_top(&self) -> f32 {
        HEAD * (1.0 - 0.45 * self.player.crouch) + self.player.jump_y
    }

    /// Alle Kaesten im Gang, die den Spieler aufhalten koennen.
    pub(super) fn corridor_blockers(&self) -> Vec<(Aabb, Block)> {
        let mut out = Vec::new();
        let Some(c) = &self.world.corridor else {
            return out;
        };
        for h in &c.hurdles {
            let rule = if h.duck {
                Block::Duck(h.area.min.y)
            } else {
                Block::Jump(h.area.max.y)
            };
            out.push((h.area, rule));
        }
        for z in &c.squeezes {
            let _ = z; // Engstellen sind normale Waende (Solids); hier nur das Tempo
        }
        let Some(r) = &self.corridor_run else {
            return out;
        };
        if r.entered {
            // die zugeknallte Ladentuer
            out.push((
                Aabb::from_center(
                    c.door_center + vec3(0.0, 1.3, -0.06),
                    vec3(CORRIDOR_HALF_W + 0.1, 1.3, 0.06),
                ),
                Block::Solid,
            ));
        }
        for (st, s) in c.stations.iter().zip(&r.stations) {
            match st.kind {
                StationKind::Door { .. } | StationKind::ExitDoor => {
                    if s.open < 0.6 {
                        out.push((door_box(st), Block::Solid));
                    }
                }
                StationKind::Shelf { side } => {
                    if s.t.is_none() {
                        out.push((shelf_standing_box(st, side), Block::Solid));
                    } else if s.done {
                        out.push((
                            Aabb::from_center(
                                st.at + vec3(0.0, SHELF_DEEP * 0.5, 0.0),
                                half_of(
                                    st.dir,
                                    SHELF_ALONG * 0.5,
                                    CORRIDOR_HALF_W,
                                    SHELF_DEEP * 0.5,
                                ),
                            ),
                            Block::Jump(SHELF_DEEP + 0.05),
                        ));
                    }
                }
                StationKind::Cart { lane } => {
                    if s.done {
                        let p = cart_pos(c, s.s, lane);
                        out.push((
                            Aabb::from_center(p + vec3(0.0, 0.5, 0.0), vec3(0.35, 0.5, 0.35)),
                            Block::Solid,
                        ));
                    }
                }
                _ => {}
            }
        }
        self.event_blockers(&mut out);
        out
    }

    /// Spieler gegen die Kaesten im Gang schieben (nach `World::collide`).
    pub(in crate::game) fn corridor_collide(&self, mut p: Vec3, radius: f32) -> Vec3 {
        if self.world.corridor.is_none() {
            return p;
        }
        let head = self.head_top();
        let feet = self.player.jump_y;
        for _ in 0..2 {
            for (b, rule) in self.corridor_blockers() {
                let blocks = match rule {
                    Block::Solid => true,
                    Block::Duck(bottom) => head > bottom,
                    Block::Jump(top) => feet < top - JUMP_SLACK,
                };
                if blocks {
                    p = push_out(p, radius, &b);
                }
            }
        }
        p
    }

    /// Tueren, Regale, Wagen nachfuehren.
    pub(super) fn update_obstacles(&mut self, dt: f32) {
        let Some(c) = self.world.corridor.clone() else {
            return;
        };
        let progress = c.progress(self.player.pos);
        let player = self.player.pos;
        let mut stumble = false;
        let mut sounds: Vec<(Sfx, f32, Vec3)> = Vec::new();
        let Some(r) = &mut self.corridor_run else {
            return;
        };
        for (st, s) in c.stations.iter().zip(r.stations.iter_mut()) {
            match st.kind {
                StationKind::Door { .. } | StationKind::ExitDoor => {
                    if let Some(t) = &mut s.t {
                        *t += dt;
                        s.open = (*t / DOOR_OPEN_TIME).min(1.0);
                    }
                }
                StationKind::Shelf { .. } => {
                    if s.t.is_none() && progress > st.s - SHELF_TRIGGER && progress < st.s + 1.0 {
                        s.t = Some(0.0);
                        sounds.push((Sfx::Creak, 1.0, st.at + vec3(0.0, 2.0, 0.0)));
                        sounds.push((Sfx::Rumble, 0.6, st.at));
                    }
                    if let Some(t) = &mut s.t {
                        let before = *t;
                        *t += dt;
                        if before < SHELF_FALL && *t >= SHELF_FALL {
                            s.done = true;
                            sounds.push((Sfx::Thud, 1.0, st.at));
                            sounds.push((Sfx::Rattle, 0.8, st.at));
                            if (progress - st.s).abs() < SHELF_ALONG * 0.5 + 0.45 {
                                stumble = true;
                            }
                        }
                    }
                }
                StationKind::Cart { lane } => {
                    if s.done {
                        continue;
                    }
                    if s.t.is_none() && progress > st.s - CART_TRIGGER {
                        s.t = Some(0.0);
                        sounds.push((Sfx::SqueakSoft, 0.9, st.at));
                    }
                    if let Some(t) = &mut s.t {
                        *t += dt;
                        s.s -= CART_SPEED * dt;
                        let p = cart_pos(&c, s.s, lane);
                        if (*t * 3.0).fract() < dt * 3.0 {
                            sounds.push((Sfx::SqueakSoft, 0.5, p));
                        }
                        let d = vec3(p.x - player.x, 0.0, p.z - player.z).length();
                        if d < CART_HIT && self.player.jump_y < 0.3 {
                            s.done = true;
                            stumble = true;
                            sounds.push((Sfx::Thud, 1.0, p));
                            sounds.push((Sfx::Rattle, 1.0, p));
                        } else if s.s < progress - 3.0 || s.s < 0.5 {
                            // vorbeigerollt: bleibt hinten liegen
                            s.done = true;
                        }
                    }
                }
                _ => {}
            }
        }
        self.sfx_at.extend(sounds);
        if stumble {
            self.stumble();
        }
    }

    /// Gestolpert: kurz langsamer, Aufprall, Bild wackelt.
    pub(super) fn stumble(&mut self) {
        if let Some(r) = &mut self.corridor_run {
            r.stumble = STUMBLE;
        }
        self.play(Sfx::Thud, 0.9);
        self.glitch = self.glitch.max(0.5);
        self.red = self.red.max(0.3);
        self.player.vel *= 0.3;
    }

    /// Geschlossene Tueren in Reichweite (fuer den Fokus): (Index, Kasten).
    pub fn corridor_doors(&self) -> Vec<(usize, Aabb)> {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return Vec::new();
        };
        c.stations
            .iter()
            .zip(&r.stations)
            .enumerate()
            .filter(|(_, (st, s))| {
                matches!(st.kind, StationKind::Door { .. } | StationKind::ExitDoor) && s.t.is_none()
            })
            .map(|(i, (st, _))| (i, door_box(st)))
            .collect()
    }

    /// E an einer Tuer: aufdruecken (klemmende brauchen mehrere Anlaeufe).
    pub(in crate::game) fn push_door(&mut self, i: usize) {
        let Some(c) = &self.world.corridor else {
            return;
        };
        let Some(st) = c.stations.get(i).copied() else {
            return;
        };
        let need = match st.kind {
            StationKind::Door { stuck: true } | StationKind::ExitDoor => STUCK_PRESSES,
            StationKind::Door { stuck: false } => 1,
            _ => return,
        };
        let Some(s) = self
            .corridor_run
            .as_mut()
            .and_then(|r| r.stations.get_mut(i))
        else {
            return;
        };
        if s.t.is_some() {
            return;
        }
        s.presses += 1;
        let at = st.at + vec3(0.0, 1.1, 0.0);
        if s.presses < need {
            // klemmt: ruettelt, knarzt
            self.sfx_at.push((Sfx::Knock, 1.0, at));
            self.sfx_at.push((Sfx::Creak, 0.6, at));
            self.glitch = self.glitch.max(0.2);
            self.hint("Klemmt! Nochmal E.");
        } else {
            s.t = Some(0.0);
            self.sfx_at.push((Sfx::Creak, 1.0, at));
            self.sfx_at.push((Sfx::Thud, 0.5, at));
        }
    }

    /// Tueren, Regale, Wagen, zugeknallte Ladentuer zeichnen.
    pub(in crate::game) fn collect_obstacle_draws(&self, out: &mut Vec<MeshDraw>) {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return;
        };
        let cube = MeshId::Cube as usize;
        let mut boxed = |size: Vec3, rot: Quat, pos: Vec3, color: Vec3| {
            out.push(MeshDraw {
                mesh: cube,
                model: Mat4::from_scale_rotation_translation(size, rot, pos),
                tint: color.extend(0.0),
                ..Default::default()
            });
        };
        if r.entered {
            // Ladentuer: schlaegt in 0,2 s zu (von der Seite herein)
            let k = (r.slam / 0.2).min(1.0);
            let w = 2.0 * (CORRIDOR_HALF_W + 0.1);
            boxed(
                vec3(w * k, 2.6, 0.1),
                Quat::IDENTITY,
                c.door_center + vec3(-w * 0.5 * (1.0 - k), 1.3, -0.06),
                vec3(0.12, 0.1, 0.09),
            );
        }
        for (st, s) in c.stations.iter().zip(&r.stations) {
            match st.kind {
                StationKind::Door { .. } => {
                    // zwei Fluegel, schwingen in Laufrichtung auf
                    for side in [-1.0f32, 1.0] {
                        let hinge = st.at + st.side * side * (CORRIDOR_HALF_W - 0.06);
                        let closed = -st.side * side;
                        wing(
                            &mut boxed,
                            hinge,
                            closed,
                            st.dir,
                            s.open,
                            CORRIDOR_HALF_W - 0.08,
                        );
                    }
                }
                StationKind::ExitDoor => {
                    // ein Fluegel in der Seitenwand, schwingt nach aussen (+x) auf
                    let half = (EXIT_FROM - EXIT_TO) * 0.5;
                    let center = st.at + st.side * (CORRIDOR_HALF_W + 0.1);
                    let hinge = center - st.dir * half;
                    wing(
                        &mut boxed,
                        hinge,
                        st.dir,
                        st.side,
                        s.open,
                        half * 2.0 - 0.04,
                    );
                }
                StationKind::Shelf { side } => {
                    let a = match s.t {
                        None => 0.0,
                        Some(t) => (t / SHELF_FALL).min(1.0).powi(2) * FRAC_PI_2 * 0.97,
                    };
                    shelf(&mut boxed, st, side, a);
                }
                _ => {}
            }
        }
        // Wagen
        for (st, s) in c.stations.iter().zip(&r.stations) {
            if let StationKind::Cart { lane } = st.kind {
                let p = cart_pos(c, s.s, lane);
                let d = c.dir_at_s(s.s);
                let tip = if s.done && s.t.is_some() { 0.35 } else { 0.0 };
                out.push(MeshDraw {
                    mesh: MeshId::Cart as usize,
                    model: Mat4::from_rotation_translation(
                        Quat::from_rotation_y(std::f32::consts::PI - npc::yaw_towards(p, p - d))
                            * Quat::from_rotation_z(tip),
                        p + vec3(0.0, self.sway_at(p).y, 0.0),
                    ),
                    tint: Vec4::new(0.7, 0.7, 0.72, 0.0),
                    ..Default::default()
                });
            }
        }
        self.collect_event_draws(out);
    }
}

/// Wo der Wagen bei `s` Metern steht (seitlich versetzt).
pub(super) fn cart_pos(c: &Corridor, s: f32, lane: f32) -> Vec3 {
    c.point_at(s.max(0.0)) + crate::world::side_of(c.dir_at_s(s.max(0.0))) * lane
}

/// Ein Tuerfluegel: Scharnier, Richtung im geschlossenen Zustand, Richtung beim Oeffnen.
fn wing(
    boxed: &mut impl FnMut(Vec3, Quat, Vec3, Vec3),
    hinge: Vec3,
    closed: Vec3,
    opens: Vec3,
    open: f32,
    len: f32,
) {
    let a = open * FRAC_PI_2 * 0.95;
    let w = (closed * a.cos() + opens * a.sin()).normalize_or_zero();
    let rot = Quat::from_rotation_arc(Vec3::X, w);
    let center = hinge + w * len * 0.5 + vec3(0.0, 1.1, 0.0);
    boxed(vec3(len, 2.2, 0.05), rot, center, vec3(0.32, 0.34, 0.33));
    // Sichtfenster (dunkel) und Trittblech
    boxed(
        vec3(len * 0.35, 0.35, 0.06),
        rot,
        hinge + w * len * 0.55 + vec3(0.0, 1.5, 0.0),
        vec3(0.03, 0.04, 0.05),
    );
    boxed(
        vec3(len * 0.9, 0.25, 0.06),
        rot,
        hinge + w * len * 0.5 + vec3(0.0, 0.15, 0.0),
        vec3(0.5, 0.5, 0.48),
    );
}

/// Kasten des noch stehenden Regals an der Wand.
fn shelf_standing_box(st: &crate::world::Station, side: f32) -> Aabb {
    let v = side * (CORRIDOR_HALF_W - SHELF_DEEP * 0.5);
    Aabb::from_center(
        st.at + st.side * v + vec3(0.0, SHELF_H * 0.5, 0.0),
        half_of(st.dir, SHELF_ALONG * 0.5, SHELF_DEEP * 0.5, SHELF_H * 0.5),
    )
}

/// Regal zeichnen, um `a` (Bogenmass) quer in den Gang gekippt.
fn shelf(
    boxed: &mut impl FnMut(Vec3, Quat, Vec3, Vec3),
    st: &crate::world::Station,
    side: f32,
    a: f32,
) {
    // Kippkante: Unterkante zur Gangseite
    let pivot = st.at + st.side * side * (CORRIDOR_HALF_W - SHELF_DEEP);
    // um die Gangrichtung drehen, Oberkante kippt zur anderen Seite
    let q = Quat::from_axis_angle(st.dir, side * a);
    let size = |across: f32, up: f32, along: f32| {
        if st.dir.x.abs() > 0.5 {
            vec3(along, up, across)
        } else {
            vec3(across, up, along)
        }
    };
    let local = |v: f32, y: f32| pivot + q * (st.side * side * v + vec3(0.0, y, 0.0));
    boxed(
        size(SHELF_DEEP, SHELF_H, SHELF_ALONG),
        q,
        local(SHELF_DEEP * 0.5, SHELF_H * 0.5),
        vec3(0.36, 0.37, 0.4),
    );
    // Regalboeden mit Ware (bunte Kisten)
    for (k, y) in [0.35f32, 0.95, 1.55, 2.1].iter().enumerate() {
        boxed(
            size(SHELF_DEEP * 1.02, 0.04, SHELF_ALONG * 1.04),
            q,
            local(SHELF_DEEP * 0.5, *y),
            vec3(0.2, 0.2, 0.22),
        );
        boxed(
            size(SHELF_DEEP * 0.7, 0.22, SHELF_ALONG * 0.8),
            q,
            local(SHELF_DEEP * 0.5, *y + 0.14),
            [
                vec3(0.6, 0.1, 0.08),
                vec3(0.1, 0.3, 0.55),
                vec3(0.55, 0.45, 0.15),
                vec3(0.2, 0.45, 0.2),
            ][k],
        );
    }
}
