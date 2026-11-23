//! Drei Leben: zweimal erwischt werden ueberlebt man (kurze Todesanimation, man wacht am
//! Eingang auf, der Wagen bleibt am Todespunkt), beim dritten Mal ist der Run vorbei.
//! Jeder Tod hinterlaesst eine staerkere rote Vignette.

use super::*;

/// So oft darf man erwischt werden, ohne dass der Run endet
pub const SURVIVABLE_HITS: u32 = 2;
/// Dauer der Todesanimation (Sekunden)
const DEATH_TIME: f32 = 2.2;
/// Schonfrist nach dem Aufwachen
const RESPAWN_GRACE: f32 = 5.0;
/// Verfolger in diesem Umkreis um den Todespunkt werden zurueckgesetzt
const RESET_RADIUS: f32 = 8.0;

impl Game {
    /// Ein Verfolger hat einen erreicht: verletzt (Todesanimation) oder Run vorbei.
    pub(super) fn caught_by(&mut self, catcher: Vec3, variant: usize) {
        // Tutorial: nur erklaeren, nicht sterben
        if self.tutorial_run {
            let msg = if self.chase.is_some() {
                "Erwischt! Im echten Spiel wäre die Flucht jetzt vorbei. Weiterlaufen!"
            } else {
                "Erwischt! Im echten Spiel kostet das ein Leben. Halte Abstand."
            };
            self.tutorial_forgive(msg);
            return;
        }
        self.catcher_variant = Some(variant);
        // Bei der Flucht zum Ausgang gibt es kein Aufwachen mehr
        if self.deaths >= SURVIVABLE_HITS || self.chase.is_some() {
            self.outcome = Outcome::Caught;
            self.outcome_timer = 0.0;
            self.catcher = Some(catcher);
            self.red = 1.0;
            self.glitch = 1.0;
            self.sfx.push((Sfx::Sting, 1.0));
            return;
        }
        self.deaths += 1;
        self.dying = Some(0.0);
        // Schreck: das leere Gesicht kurz in Grossaufnahme (scares.rs)
        self.sfx.push((Sfx::Scare, 1.0));
        self.sfx.push((Sfx::Thud, 1.0));
        self.scares.push(super::scare_kinds::ScareKind::Death);
        self.catcher = Some(catcher);
        self.dialog = None;
        self.red = 1.0;
        self.glitch = 0.9;
        self.sfx.push((Sfx::Sting, 1.0));
        // Ware in der Hand faellt hin, der Wagen bleibt, wo er ist
        if let Some(data) = self.held.take() {
            let pos = self.player.pos + vec3(0.0, 0.3, 0.0);
            self.loose.push(Loose {
                data,
                pos,
                vel: vec3(0.0, 0.5, 0.0),
                yaw: self.player.yaw,
            });
        }
        self.trolley.pushing = false;
    }

    /// Wie weit die Todesanimation ist (0..1), None = man lebt.
    pub fn dying_progress(&self) -> Option<f32> {
        self.dying.map(|t| (t / DEATH_TIME).min(1.0))
    }

    /// Abblende am Ende der Todesanimation.
    pub fn death_fade(&self) -> f32 {
        self.dying_progress()
            .map_or(0.0, |k| ((k - 0.55) / 0.35).clamp(0.0, 1.0))
    }

    /// Dauerhafte rote Vignette je Tod.
    pub fn death_vignette(&self) -> f32 {
        self.deaths as f32 * 0.25
    }

    /// Todesanimation abspielen; danach am Eingang aufwachen.
    pub(super) fn update_dying(&mut self, dt: f32) {
        let Some(t) = &mut self.dying else { return };
        *t += dt;
        let k = (*t / DEATH_TIME).min(1.0);
        // erst zum Faenger herumreissen, dann zusammensacken und zu Boden sehen
        if let Some(c) = self.catcher {
            let to = c + vec3(0.0, 1.55, 0.0) - self.eye();
            let yaw = to.x.atan2(-to.z);
            self.player.yaw = lerp_angle(self.player.yaw, yaw, (dt * 10.0).min(1.0));
        }
        let want_pitch = if k < 0.3 { 0.1 } else { -1.1 };
        self.player.pitch += (want_pitch - self.player.pitch) * (dt * 4.0).min(1.0);
        self.player.vel = Vec3::ZERO;
        self.red = self.red.max(1.0 - k * 0.3);
        if k < 1.0 {
            return;
        }
        self.respawn();
    }

    fn respawn(&mut self) {
        // danach Ruhe: niemand rueckt heran (director.rs)
        self.director.calm_down(super::director::CALM_AFTER_DEATH);
        let death_pos = self.player.pos;
        self.dying = None;
        self.catcher = None;
        let w = &self.world;
        self.player.pos = w.spawn;
        self.player.yaw = w.spawn_yaw;
        self.player.pitch = -0.1;
        self.player.vel = Vec3::ZERO;
        self.player.stamina = 1.0;
        self.player.exhausted = false;
        self.grace = RESPAWN_GRACE;
        self.glitch = 0.6;
        // Verfolger am Todespunkt und am Eingang verziehen sich wieder an ihr Regal
        let spawn = self.player.pos;
        for c in &mut self.customers {
            let near = (c.pos - death_pos).length() < RESET_RADIUS
                || (c.pos - spawn).length() < RESET_RADIUS;
            if c.stalker && near {
                c.pos = c.home;
                c.field.clear();
                c.wait = 2.0;
            }
        }
        for e in &mut self.employees {
            e.follow = 0.0;
            e.heard = 0.0;
        }
        self.say(
            "Du wachst am Eingang auf. Dein Wagen steht noch dort, wo es passiert ist.",
            5.0,
        );
    }
}
