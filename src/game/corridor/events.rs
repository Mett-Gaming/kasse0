//! Ereignisse im Gang: Gestalten, die vor einem stehen und verschwinden, Lautsprecher-Durchsagen
//! und Fluestern, eine Decke, die sich senkt (ducken!), Waende, die zusammenruecken.

use super::obstacles::{half_of, Block};
use super::*;
use crate::people::{Clip, PersonDraw};
use crate::world::{Aabb, CORRIDOR_H};

/// Gestalt: erscheint so weit voraus, verschwindet so nah bzw. nach so langer Zeit
pub const FIGURE_SHOW: f32 = 9.0;
pub const FIGURE_GONE: f32 = 4.0;
pub const FIGURE_TIME: f32 = 2.5;
/// Decke/Waende: loesen so weit davor aus, Laenge des Stuecks, Dauer
pub const CHANGE_TRIGGER: f32 = 7.0;
pub const CHANGE_HALF: f32 = 3.0;
const CEILING_LOW: f32 = 1.2;
const CEILING_TIME: f32 = 2.0;
const WALLS_GAP: f32 = 1.2;
const WALLS_TIME: f32 = 2.5;
/// Durchsagen im Gang (alle 8-14 s)
const VOICES: &[&str] = &[
    "Kunde 0, bitte umdrehen.",
    "Sie sind gleich da.",
    "Dreh dich um. Nur kurz.",
    "Kunde 0, Sie haben etwas vergessen.",
    "Bitte bleiben Sie stehen.",
    "Der Ausgang ist hinter Ihnen.",
    "Kunde 0, Sie haben Ihren Schatten vergessen.",
    "Bitte nicht so schnell. Wir kommen nicht hinterher.",
    "Rechts. Nein, links. Nein. Bleiben Sie einfach stehen.",
    "Hinter Ihnen ist alles in Ordnung. Sehen Sie ruhig nach.",
    "Dieser Gang ist länger, als er aussieht. Er weiß, wie lange Sie hier waren.",
];

impl Game {
    pub(super) fn update_corridor_events(&mut self, dt: f32) {
        let Some(c) = self.world.corridor.clone() else {
            return;
        };
        let progress = c.progress(self.player.pos);
        let ending = self.corridor_run.as_ref().is_some_and(|r| r.ending);
        let mut sounds: Vec<(Sfx, f32, Vec3)> = Vec::new();
        let mut glitch = 0.0f32;
        let mut voice = None;
        let Some(r) = &mut self.corridor_run else {
            return;
        };
        for (st, s) in c.stations.iter().zip(r.stations.iter_mut()) {
            match st.kind {
                StationKind::Figure { .. } => {
                    if s.done {
                        continue;
                    }
                    if s.t.is_none() && progress > st.s - FIGURE_SHOW {
                        s.t = Some(0.0);
                        sounds.push((Sfx::Whisper, 0.5, st.at + vec3(0.0, 1.6, 0.0)));
                    }
                    if let Some(t) = &mut s.t {
                        *t += dt;
                        if *t > FIGURE_TIME || progress > st.s - FIGURE_GONE {
                            s.done = true;
                            sounds.push((Sfx::Static, 0.8, st.at + vec3(0.0, 1.2, 0.0)));
                            glitch = glitch.max(0.7);
                        }
                    }
                }
                StationKind::Ceiling | StationKind::Walls => {
                    if s.t.is_none() && progress > st.s - CHANGE_TRIGGER {
                        s.t = Some(0.0);
                        sounds.push((Sfx::Rumble, 1.0, st.at + vec3(0.0, 2.0, 0.0)));
                        sounds.push((Sfx::Creak, 0.8, st.at));
                    }
                    if let Some(t) = &mut s.t {
                        *t += dt;
                    }
                }
                _ => {}
            }
        }
        // Stimmen: Durchsagen, Fluestern von hinten, Rufe
        r.voice_t -= dt;
        if r.voice_t <= 0.0 && !ending {
            r.voice_t = self.rng.gen_range(8.0..14.0);
            voice = Some(VOICES[self.rng.gen_range(0..VOICES.len())]);
        }
        r.whisper_t -= dt;
        if r.whisper_t <= 0.0 {
            r.whisper_t = self.rng.gen_range(3.0..6.0);
            let back = c.point_at((progress - self.rng.gen_range(1.5..3.0)).max(0.0));
            sounds.push((Sfx::Whisper, 0.7, back + vec3(0.0, 1.6, 0.0)));
        }
        self.sfx_at.extend(sounds);
        self.glitch = self.glitch.max(glitch);
        if let Some(v) = voice {
            self.announcement = Some((v.into(), 4.0));
            self.play(Sfx::Chime, 0.4);
        }
    }

    /// Wie weit die Decke an Station `st` heruntergekommen ist (Unterkante, Meter).
    fn ceiling_height(t: Option<f32>) -> f32 {
        let k = t.map_or(0.0, |t| (t / CEILING_TIME).clamp(0.0, 1.0));
        CORRIDOR_H - (CORRIDOR_H - CEILING_LOW) * k * k * (3.0 - 2.0 * k)
    }

    /// Wie weit die Waende zusammengerueckt sind (halbe Luecke, Meter).
    fn walls_half_gap(t: Option<f32>) -> f32 {
        let k = t.map_or(0.0, |t| (t / WALLS_TIME).clamp(0.0, 1.0));
        CORRIDOR_HALF_W - (CORRIDOR_HALF_W - WALLS_GAP * 0.5) * k * k * (3.0 - 2.0 * k)
    }

    /// Kaesten der Ereignisse (gesenkte Decke = Duck-Zone, Waende = Solid).
    pub(super) fn event_blockers(&self, out: &mut Vec<(Aabb, Block)>) {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return;
        };
        for (st, s) in c.stations.iter().zip(&r.stations) {
            match st.kind {
                StationKind::Ceiling if s.t.is_some() => {
                    let y = Self::ceiling_height(s.t);
                    out.push((
                        Aabb::from_center(
                            st.at + vec3(0.0, (y + CORRIDOR_H) * 0.5, 0.0),
                            half_of(st.dir, CHANGE_HALF, CORRIDOR_HALF_W, (CORRIDOR_H - y) * 0.5),
                        ),
                        Block::Duck(y),
                    ));
                }
                StationKind::Walls if s.t.is_some() => {
                    let g = Self::walls_half_gap(s.t);
                    let th = (CORRIDOR_HALF_W - g) * 0.5;
                    for side in [-1.0f32, 1.0] {
                        out.push((
                            Aabb::from_center(
                                st.at
                                    + st.side * side * (g + th)
                                    + vec3(0.0, CORRIDOR_H * 0.5, 0.0),
                                half_of(st.dir, CHANGE_HALF, th.max(0.01), CORRIDOR_H * 0.5),
                            ),
                            Block::Solid,
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    /// Ist man zwischen zusammengerueckten Waenden? (Sprinten geht dort weiter.)
    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn between_walls(&self, p: Vec3) -> bool {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return false;
        };
        c.stations.iter().zip(&r.stations).any(|(st, s)| {
            st.kind == StationKind::Walls
                && s.t.is_some()
                && Self::walls_half_gap(s.t) < 1.0
                && (c.progress(p) - st.s).abs() < CHANGE_HALF
        })
    }

    /// Decke und Waende zeichnen (nur sobald sie sich bewegen).
    pub(super) fn collect_event_draws(&self, out: &mut Vec<MeshDraw>) {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return;
        };
        let cube = MeshId::Cube as usize;
        let mut push = |center: Vec3, half: Vec3, color: Vec3| {
            out.push(MeshDraw {
                mesh: cube,
                model: Mat4::from_scale_rotation_translation(half * 2.0, Quat::IDENTITY, center),
                tint: color.extend(0.0),
                ..Default::default()
            });
        };
        for (st, s) in c.stations.iter().zip(&r.stations) {
            match st.kind {
                StationKind::Ceiling if s.t.is_some() => {
                    let y = Self::ceiling_height(s.t);
                    push(
                        st.at + vec3(0.0, y + 0.06, 0.0),
                        half_of(st.dir, CHANGE_HALF, CORRIDOR_HALF_W, 0.06),
                        vec3(0.2, 0.2, 0.19),
                    );
                    // Rohre unter der Decke
                    for v in [-0.9f32, 0.0, 0.9] {
                        push(
                            st.at + st.side * v + vec3(0.0, y - 0.05, 0.0),
                            half_of(st.dir, CHANGE_HALF, 0.05, 0.05),
                            vec3(0.3, 0.28, 0.25),
                        );
                    }
                }
                StationKind::Walls if s.t.is_some() => {
                    let g = Self::walls_half_gap(s.t);
                    for side in [-1.0f32, 1.0] {
                        push(
                            st.at + st.side * side * (g + 0.05) + vec3(0.0, CORRIDOR_H * 0.5, 0.0),
                            half_of(st.dir, CHANGE_HALF, 0.05, CORRIDOR_H * 0.5),
                            vec3(0.19, 0.22, 0.19),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    /// Gestalten im Gang (und die Gestalt hinter einem am Ende, ending.rs).
    pub(in crate::game) fn collect_corridor_people(&self, out: &mut Vec<PersonDraw>) {
        let (Some(c), Some(r)) = (&self.world.corridor, &self.corridor_run) else {
            return;
        };
        let pool = crate::people::people_of(false);
        for (st, s) in c.stations.iter().zip(&r.stations) {
            if let StationKind::Figure { variant } = st.kind {
                if s.t.is_some() && !s.done {
                    out.push(PersonDraw {
                        variant: pool[variant as usize % pool.len()],
                        clip: Clip::Idle,
                        time: s.t.unwrap_or(0.0),
                        pos: st.at + vec3(0.0, self.sway_at(st.at).y, 0.0),
                        yaw: npc::yaw_towards(st.at, self.player.pos),
                        tint: Vec4::new(0.8, 0.82, 0.86, 0.2),
                        ghost: false,
                        flip: false,
                        look_at: Some(self.eye()),
                        fx: Default::default(),
                    });
                }
            }
        }
        self.collect_ending_people(out);
    }
}
