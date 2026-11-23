//! Das Ende des Gangs: Im letzten Stueck steht eine Gestalt dicht hinter einem - sehen kann man
//! sie nur im Spiegel (umdrehen ist toedlich). Die Ausgangstuer seitlich vor dem Spiegel klemmt
//! immer (3x E); wer zu lange braucht, wird eingeholt. Hinter der Tuer: ein letzter, stummer
//! Schreck (nur Bild, kein Ton), dann ist man draussen.

use super::*;
use crate::people::{Clip, PersonDraw};

/// So dicht steht die Gestalt hinter einem
pub const BEHIND: f32 = 1.3;
/// So schnell kriecht sie heran, wenn man stehen bleibt (1,3 m in 6 s)
pub const CREEP: f32 = BEHIND / 6.0;

impl Game {
    pub(super) fn update_ending(&mut self, _dt: f32) {
        let Some(c) = &self.world.corridor else {
            return;
        };
        let p = self.player.pos;
        let progress = c.progress(p);
        let inside = c.contains(p);
        // hinter der Ausgangstuer: ueber die Wandlinie getreten
        let end = *c.pts.last().unwrap();
        let through = p.x > end.x + CORRIDOR_HALF_W + 0.2 && p.z > end.z - EXIT_FROM - 0.5;
        let Some(r) = &mut self.corridor_run else {
            return;
        };
        if !r.ending && inside && progress > c.end_s + 0.5 {
            r.ending = true;
            r.front = r.front.max(progress - BEHIND);
            self.say("Da steht jemand hinter dir. Sieh in den Spiegel.", 4.0);
            self.play(Sfx::Whisper, 1.0);
            self.play(Sfx::Sting, 0.5);
            return;
        }
        if r.ending && through && !r.last_scare {
            // der letzte Schreck: Gesicht, Schrei, Lichtblitz - aber man ist durch
            r.last_scare = true;
            // nur fuers Auge (Blitz, Rot, Rauschen) - geschafft ist geschafft: kein Schreck-Ton
            let quiet = self.sfx.len();
            self.fire_scare(0.9);
            self.sfx.truncate(quiet);
            self.red = self.red.max(0.6);
            self.glitch = 1.0;
        }
    }

    /// Ist die Gestalt dicht hinter einem gerade da (letztes Stueck)?
    pub fn ending_figure(&self) -> Option<Vec3> {
        let (c, r) = (self.world.corridor.as_ref()?, self.corridor_run.as_ref()?);
        if !r.ending || self.outcome != Outcome::Running || r.last_scare {
            return None;
        }
        Some(c.point_at(r.front.max(0.0)))
    }

    pub(super) fn collect_ending_people(&self, out: &mut Vec<PersonDraw>) {
        let Some(pos) = self.ending_figure() else {
            return;
        };
        let pool = crate::people::people_of(false);
        let v = pool[(self.seed as usize ^ 7) % pool.len()];
        out.push(PersonDraw {
            variant: v,
            clip: Clip::Idle,
            time: self.time,
            pos: pos + vec3(0.0, self.sway_at(pos).y, 0.0),
            yaw: npc::yaw_towards(pos, self.player.pos),
            tint: Vec4::new(0.75, 0.78, 0.82, 0.25),
            ghost: false,
            flip: false,
            look_at: Some(self.eye()),
            fx: Default::default(),
        });
    }
}
