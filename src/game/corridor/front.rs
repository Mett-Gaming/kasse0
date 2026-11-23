//! Die Front: unsichtbar, rueckt entlang des Gangs nach. Man hoert sie (Schritte im Abstand der
//! Front, Atmen, Herzschlag) und sieht sie am roten Rand. Holt sie einen ein: Schreck, Game Over.
//! Im letzten Stueck ist sie die Gestalt direkt hinter einem (ending.rs).

use super::*;

/// So schnell rueckt die Front nach (Rennen: 2,56 m/s)
pub const FRONT_SPEED: f32 = 2.0;
/// Weiter als das faellt sie nie zurueck (sie bleibt einem auf den Fersen)
pub const FRONT_MAX_GAP: f32 = 8.0;
/// So weit hinter einem startet sie beim Betreten des Gangs
pub const FRONT_START: f32 = 5.0;
/// Naeher als das: eingeholt
pub const FRONT_CATCH: f32 = 0.3;
/// Ab diesem Abstand wird es eng (roter Rand, "SCHNELLER")
pub const FRONT_CLOSE: f32 = 5.0;
/// Im Tutorial bleibt sie so weit hinter einem
const TUTORIAL_GAP: f32 = 3.0;
/// Schritte der Front: alle so viele Meter, hoerbar bis so weit
const STEP_EVERY: f32 = 0.75;
const HEAR: f32 = 16.0;

impl Game {
    /// Abstand der Front hinter einem (Meter), None = keine Front.
    pub fn front_gap(&self) -> Option<f32> {
        let r = self.corridor_run.as_ref().filter(|r| r.entered)?;
        Some(self.corridor_progress() - r.front)
    }

    pub(super) fn update_front(&mut self, dt: f32) {
        let Some(c) = &self.world.corridor else {
            return;
        };
        // im Ausgangsraum ist man ihr entkommen
        if !c.contains(self.player.pos) || self.outcome != Outcome::Running {
            return;
        }
        let progress = c.progress(self.player.pos);
        let front_pos = |s: f32| c.point_at(s);
        let tutorial = self.tutorial_run;
        let Some(r) = &mut self.corridor_run else {
            return;
        };
        if r.ending {
            // die Gestalt bleibt dicht hinter einem und kriecht heran, wenn man stehen bleibt
            r.front = (r.front + super::ending::CREEP * dt).max(progress - super::ending::BEHIND);
        } else {
            r.front = (r.front + FRONT_SPEED * dt).max(progress - FRONT_MAX_GAP);
        }
        if tutorial {
            r.front = r.front.min(progress - TUTORIAL_GAP);
        }
        let gap = progress - r.front;
        // Schritte im Abstand der Front (hinter einem), lauter, je naeher
        r.step_acc += FRONT_SPEED * dt;
        let step = r.step_acc >= STEP_EVERY && gap < HEAR && !r.ending;
        if step {
            r.step_acc = 0.0;
        }
        // Herzschlag: schneller, je naeher
        let close = (1.0 - gap / 12.0).clamp(0.0, 1.0);
        r.heart_acc += dt;
        let heart = r.heart_acc >= 1.0 - 0.65 * close;
        if heart {
            r.heart_acc = 0.0;
        }
        let at = front_pos(r.front.max(0.0)) + vec3(0.0, 1.0, 0.0);
        if step {
            let vol = (1.0 - gap / HEAR).clamp(0.1, 1.0) * self.rng.gen_range(0.75..1.0);
            self.sfx_at.push((Sfx::Stomp, vol, at));
        }
        if heart {
            self.play(Sfx::Heartbeat, 0.3 + 0.55 * close);
        }
        if gap < FRONT_CLOSE {
            self.red = self.red.max(0.45 * (1.0 - gap / FRONT_CLOSE));
        }
        if gap <= FRONT_CATCH && !tutorial && !self.god {
            self.turn_scare(true);
        }
    }

    /// Eng: "SCHNELLER" im HUD (0..1, 0 = genug Abstand).
    pub fn front_warning(&self) -> f32 {
        match self.front_gap() {
            Some(g) if self.in_corridor() => (1.0 - g / FRONT_CLOSE).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}
