//! Was in der Luft liegt: Staub im Lampenlicht, Zigarettenrauch, Atem an der Kuehlung.
//! Nur fuers Auge: Das Spiel selbst weiss nichts davon.

use crate::game::smoke::{Cigarette, PUFF_TIME};
use crate::game::Game;
use glam::{vec3, Vec3, Vec4};
use mettgine::gpu::GpuLight;
use mettgine::particles::{ParticleInst, Particles, Style, StyleId};
use mettgine::random::Random;

/// Hoechstens so viele Partikel
const MAX: usize = 260;
/// Davon hoechstens so viele Staubkoerner
const MAX_DUST: usize = 140;
/// Staub entsteht so weit um den Spieler (m)
const DUST_RADIUS: f32 = 5.0;
/// Sekunden zwischen zwei Atemzuegen
const BREATH: f32 = 3.2;

pub struct Air {
    parts: Particles,
    /// Staub zaehlt fuer sich, sonst verdraengt er den Rauch
    dust: Particles,
    mote: StyleId,
    wisp: StyleId,
    cloud: StyleId,
    breath: StyleId,
    rng: Random,
    smoke_rest: f32,
    dust_rest: f32,
    breath_t: f32,
    last_exhale: f32,
    /// neuer Markt: alter Staub weg
    seed: u64,
}

fn grey(v: f32, a: f32) -> Vec4 {
    Vec4::new(v, v, v * 1.02, a)
}

impl Air {
    pub fn new() -> Self {
        let mut parts = Particles::new(MAX - MAX_DUST, 11);
        let mut dust = Particles::new(MAX_DUST, 23);
        let mote = dust.style(Style {
            life: (3.0, 6.0),
            speed: (0.01, 0.06),
            accel: vec3(0.0, -0.004, 0.0),
            size: (0.009, 0.009),
            color: (grey(0.9, 0.35), grey(0.9, 0.0)),
            additive: 1.0,
            softness: 0.6,
            ..Default::default()
        });
        let wisp = parts.style(Style {
            life: (1.2, 2.0),
            speed: (0.05, 0.1),
            spread: 0.35,
            accel: vec3(0.0, 0.05, 0.0),
            drag: 0.6,
            size: (0.004, 0.035),
            color: (grey(0.75, 0.22), grey(0.7, 0.0)),
            spin: (-1.0, 1.0),
            softness: 1.0,
            ..Default::default()
        });
        let cloud = parts.style(Style {
            life: (1.4, 2.6),
            speed: (0.35, 0.8),
            spread: 0.28,
            accel: vec3(0.0, 0.08, 0.0),
            drag: 1.4,
            size: (0.03, 0.26),
            color: (grey(0.72, 0.16), grey(0.7, 0.0)),
            spin: (-0.6, 0.6),
            softness: 1.0,
            ..Default::default()
        });
        let breath = parts.style(Style {
            life: (0.8, 1.5),
            speed: (0.25, 0.5),
            spread: 0.2,
            accel: vec3(0.0, 0.06, 0.0),
            drag: 1.8,
            size: (0.02, 0.13),
            color: (
                Vec4::new(0.75, 0.8, 0.86, 0.09),
                Vec4::new(0.75, 0.8, 0.86, 0.0),
            ),
            softness: 1.0,
            ..Default::default()
        });
        Self {
            parts,
            dust,
            mote,
            wisp,
            cloud,
            breath,
            rng: Random::new(5),
            smoke_rest: 0.0,
            dust_rest: 0.0,
            breath_t: 0.0,
            last_exhale: 0.0,
            seed: 0,
        }
    }

    /// Einen Schritt weiter (nur waehrend gespielt wird).
    pub fn update(&mut self, g: &Game, dt: f32) {
        if self.seed != g.seed {
            self.seed = g.seed;
            self.parts.clear();
            self.dust.clear();
        }
        self.parts.update(dt);
        self.dust.update(dt);
        self.spawn_dust(g, dt);
        self.spawn_smoke(g, dt);
        self.spawn_breath(g, dt);
    }

    fn spawn_dust(&mut self, g: &Game, dt: f32) {
        self.dust_rest += dt * 30.0;
        while self.dust_rest >= 1.0 {
            self.dust_rest -= 1.0;
            let p = g.player.pos
                + vec3(
                    self.rng.range(-DUST_RADIUS, DUST_RADIUS),
                    self.rng.range(0.4, 2.6),
                    self.rng.range(-DUST_RADIUS, DUST_RADIUS),
                );
            if lit(&g.world.lights, p) && free(g, p) {
                self.dust.burst(self.mote, p, Vec3::ZERO, Vec3::ZERO, 1);
            }
        }
    }

    fn spawn_smoke(&mut self, g: &Game, dt: f32) {
        if matches!(g.cig, Cigarette::Lit { .. } | Cigarette::Puff { .. }) {
            if let Some(p) = g.cigarette_ember() {
                let up = Vec3::Y;
                self.parts
                    .stream(self.wisp, p, p, up, 14.0, dt, &mut self.smoke_rest);
            }
        }
        // ausatmen: der Wert springt am Ende des Zugs nach oben
        if g.exhale > self.last_exhale + PUFF_TIME * 0.5 {
            let f = g.view_dir();
            self.parts.burst(self.cloud, mouth(g), f, Vec3::ZERO, 26);
        }
        self.last_exhale = g.exhale;
    }

    fn spawn_breath(&mut self, g: &Game, dt: f32) {
        self.breath_t += dt;
        if self.breath_t < BREATH {
            return;
        }
        self.breath_t = 0.0;
        if cold(g) && g.dying.is_none() {
            let f = g.view_dir();
            self.parts.burst(self.breath, mouth(g), f, Vec3::ZERO, 9);
        }
    }

    /// Zeichen-Daten, von hinten nach vorn.
    pub fn instances(&mut self, eye: Vec3, out: &mut Vec<ParticleInst>) {
        out.clear();
        out.extend_from_slice(self.dust.instances(eye));
        out.extend_from_slice(self.parts.instances(eye));
    }
}

fn mouth(g: &Game) -> Vec3 {
    g.eye() + g.view_dir() * 0.16 - vec3(0.0, 0.08, 0.0)
}

/// Liegt der Punkt im Lichtkegel unter einer brennenden Deckenlampe?
fn lit(lights: &[GpuLight], p: Vec3) -> bool {
    lights.iter().any(|l| {
        let d = Vec3::new(l.pos[0], l.pos[1], l.pos[2]) - p;
        let side = d.x.hypot(d.z);
        l.color[3] > 0.15 && d.y > 0.2 && side < 0.35 + d.y * 0.45 && d.length() < l.pos[3]
    })
}

/// Nicht in einem Regal oder einer Wand.
fn free(g: &Game, p: Vec3) -> bool {
    let ground = vec3(p.x, 0.0, p.z);
    g.world.collide(ground, 0.05).distance(ground) < 1e-3 && g.world.line_of_sight(g.eye(), p)
}

/// An der Kuehlwand (Nordwand) oder an einer Kuehltruhe?
pub fn cold(g: &Game) -> bool {
    let p = g.player.pos;
    if g.in_corridor() || g.world.in_wing(p) {
        return false;
    }
    let chest = g.world.freezers.iter().any(|&(mid, yaw, len)| {
        let dir = vec3(yaw.sin(), 0.0, yaw.cos());
        let along = (p - mid).dot(dir).clamp(-len * 0.5, len * 0.5);
        let q = mid + dir * along;
        (p.x - q.x).hypot(p.z - q.z) < 2.0
    });
    chest || (p.z > 0.0 && p.z < 3.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(g: &Game, air: &mut Air, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            air.update(g, 1.0 / 60.0);
        }
    }

    #[test]
    fn dust_stays_below_its_limit_and_out_of_walls() {
        let g = Game::new(4);
        let mut air = Air::new();
        run(&g, &mut air, 20.0);
        let mut out = Vec::new();
        air.instances(g.eye(), &mut out);
        assert!(!out.is_empty(), "kein Staub");
        assert!(out.len() <= MAX_DUST);
        for p in &out {
            let p = Vec3::from(p.pos);
            assert!(p.distance(g.eye()) < DUST_RADIUS * 2.0);
            assert!(p.y > 0.0 && p.y < 3.5);
        }
    }

    #[test]
    fn smoke_rises_from_the_ember_and_old_air_is_gone_in_a_new_market() {
        let mut g = Game::new(4);
        let mut air = Air::new();
        g.cig = Cigarette::Lit { len: 1.0, puffs: 0 };
        run(&g, &mut air, 1.0);
        assert!(!air.parts.is_empty());
        let before = air.parts.len();
        g.exhale = 2.2;
        air.update(&g, 1.0 / 60.0);
        assert!(air.parts.len() >= before + 20, "keine Wolke beim Ausatmen");
        let other = Game::new(5);
        air.update(&other, 1.0 / 60.0);
        assert!(air.parts.is_empty());
    }

    #[test]
    fn breath_shows_only_where_it_is_cold() {
        let mut g = Game::new(4);
        g.player.pos = vec3(g.world.spawn.x, 0.0, 2.0);
        assert!(cold(&g));
        g.player.pos = g.world.spawn;
        assert!(!cold(&g));
        let mut air = Air::new();
        run(&g, &mut air, 8.0);
        assert!(air.parts.is_empty());
    }
}
