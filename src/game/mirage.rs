//! Fata Morgana: Hologramme eines normalen Supermarkts, die im gekippten Markt kurz
//! auftauchen - eine froehliche Kundin, ein winkender Kassierer, ein voller Wagen,
//! eine riesige Flasche, ein gruenes Ausgangsschild. Kommt man naeher, sind sie weg.

use super::*;

pub struct Hologram {
    pub pos: Vec3,
    pub yaw: f32,
    pub mesh: usize,
    /// Figur (animiert) statt Mesh
    pub person: Option<usize>,
    pub tint: Vec3,
    pub scale: Vec3,
    pub life: f32,
    /// So sieht es aus, solange man es fuer echt haelt (rgb, w = Eigenleuchten)
    pub real_tint: Vec4,
    /// < 0: noch nicht entlarvt; sonst Sekunden seit dem Entlarven
    pub revealed: f32,
    /// selten: dreht sich ruckartig zum Spieler, bevor sie zerfaellt (Schreck)
    pub scary: bool,
}

/// Erst so nah entpuppt sich eine Fata Morgana, dann flackert sie kurz und ist weg
const REVEAL_DIST: f32 = 3.0;
const REVEAL_TIME: f32 = 0.8;
/// So lange bleibt eine "unheimliche" Fata Morgana nach dem Umdrehen noch scheinbar echt
pub(super) const SCARY_HOLD: f32 = 0.2;

impl Game {
    pub(super) fn update_mirages(&mut self, dt: f32) {
        if !self.tipped {
            self.holograms.clear();
            return;
        }
        self.tipped_time += dt;
        // Aus der Naehe entpuppt sie sich: flackert kurz als Hologramm und ist weg
        let player = self.player.pos;
        let mut revealed_now = false;
        let scary_now =
            self.mirage_force_scary || (self.tier >= Tier::Uneasy && self.rng.gen_bool(0.6));
        let mut scared = false;
        self.holograms.retain_mut(|h| {
            h.life -= dt;
            let d = vec3(h.pos.x - player.x, 0.0, h.pos.z - player.z).length();
            if h.revealed < 0.0 && d < REVEAL_DIST {
                h.revealed = 0.0;
                revealed_now = true;
                // selten: eine Figur dreht sich ruckartig um, bevor sie zerfaellt
                if h.person.is_some() && scary_now {
                    h.scary = true;
                    h.yaw = npc::yaw_towards(h.pos, player);
                    scared = true;
                }
            } else if h.revealed >= 0.0 {
                h.revealed += dt;
            }
            h.life > 0.0 && h.revealed < REVEAL_TIME
        });
        if revealed_now {
            self.glitch = self.glitch.max(0.35);
            self.sfx.push((Sfx::Blip, 0.3));
        }
        if scared {
            self.mirage_force_scary = false;
        }
        if scared && self.can_scare_kind(false, super::scare_kinds::ScareKind::Mirage) {
            self.fire_scare_kind(super::scare_kinds::ScareKind::Mirage, 0.7);
        }

        self.mirage_timer -= dt;
        if self.mirage_timer > 0.0 || self.holograms.len() >= 2 {
            return;
        }
        self.mirage_timer = self.rng.gen_range(14.0..28.0) - self.tier.level() as f32 * 2.0;
        // Ein Platz in 9-20 m Entfernung, am liebsten gerade im Blick
        let mut spot = None;
        for attempt in 0..200 {
            let Some(p) = self.far_walkable_cell(0.0) else {
                continue;
            };
            let d = (p - player).length();
            if !(9.0..=20.0).contains(&d) {
                continue;
            }
            if attempt < 150 && !self.sees(p + vec3(0.0, 1.0, 0.0)) {
                continue;
            }
            spot = Some(p);
            break;
        }
        let Some(pos) = spot else { return };
        let facing = npc::yaw_towards(pos, player);
        let shopper = self.pick_person(false);
        let clerk = self.pick_person(true);
        let kind = self.rng.gen_range(0..5);
        let h = match kind {
            0 => Hologram {
                pos,
                yaw: facing,
                mesh: 0,
                person: Some(shopper),
                tint: vec3(1.0, 0.55, 0.5),
                scale: Vec3::ONE,
                life: 18.0,
                real_tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
                revealed: -1.0,
                scary: false,
            },
            1 => Hologram {
                pos,
                yaw: facing,
                mesh: 0,
                person: Some(clerk),
                tint: vec3(0.5, 0.9, 1.0),
                scale: Vec3::ONE,
                life: 18.0,
                real_tint: Vec4::new(1.0, 1.0, 1.0, 0.22),
                revealed: -1.0,
                scary: false,
            },
            2 => Hologram {
                pos,
                yaw: facing + 1.2,
                mesh: MeshId::Cart as usize,
                person: None,
                tint: vec3(0.6, 1.0, 0.7),
                scale: Vec3::ONE,
                life: 18.0,
                real_tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
                revealed: -1.0,
                scary: false,
            },
            3 => Hologram {
                pos,
                yaw: 0.0,
                mesh: catalog().products[catalog().list[self.water_entry()].cheap[0]].mesh,
                person: None,
                tint: vec3(0.5, 0.8, 1.0),
                scale: Vec3::splat(7.0),
                life: 14.0,
                real_tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
                revealed: -1.0,
                scary: false,
            },
            _ => Hologram {
                pos: pos + vec3(0.0, 2.4, 0.0),
                yaw: facing,
                mesh: MeshId::Cube as usize,
                person: None,
                tint: vec3(0.2, 1.0, 0.35),
                scale: vec3(1.3, 0.4, 0.12),
                life: 16.0,
                real_tint: Vec4::new(0.2, 1.0, 0.35, 1.2),
                revealed: -1.0,
                scary: false,
            },
        };
        self.holograms.push(h);
    }

    /// Fuer Tests/Screenshots: sofort eine Fata Morgana erzeugen.
    pub fn force_mirage(&mut self) {
        self.mirage_timer = 0.0;
        self.update_mirages(0.0);
    }

    /// Fuer Tests/Screenshots: Zeit im gekippten Markt vorspulen (Poster verrotten).
    pub fn tipped_time_add(&mut self, secs: f32) {
        self.tipped_time += secs;
    }

    /// Verfall der Werbeposter: frisch im normalen Markt, verrottet mit Stufe und Zeit.
    pub fn poster_decay(&self) -> f32 {
        if !self.tipped {
            return 0.0;
        }
        (0.25 + self.tier.level() as f32 * 0.25 + self.tipped_time / 300.0 * 0.25).min(1.0)
    }

    /// Hologramme flackern: einzelne Frames fehlen.
    /// Flackern und Ausblenden der Hologramme.
    pub(super) fn hologram_visible(&self, i: usize) -> bool {
        let h = &self.holograms[i];
        let flick = hash(self.time * 13.0 + i as f32 * 7.3);
        let fading = h.life < 1.5 && hash(self.time * 31.0 + i as f32) > h.life / 1.5;
        !(flick > 0.88 || fading)
    }

    /// Noch nicht entlarvte Fata Morganen sehen aus wie echte Dinge (undurchsichtig).
    pub(super) fn collect_mirage_draws(&self, out: &mut Vec<MeshDraw>) {
        for h in self
            .holograms
            .iter()
            .filter(|h| h.person.is_none() && h.revealed < 0.0)
        {
            out.push(MeshDraw {
                mesh: h.mesh,
                model: Mat4::from_scale_rotation_translation(
                    h.scale,
                    Quat::from_rotation_y(h.yaw),
                    h.pos,
                ),
                tint: h.real_tint,
                ..Default::default()
            });
        }
    }

    pub fn collect_ghosts(&self, out: &mut Vec<MeshDraw>) {
        for (i, h) in self.holograms.iter().enumerate() {
            if h.person.is_some() || h.revealed < 0.0 || !self.hologram_visible(i) {
                continue;
            }
            let model =
                Mat4::from_scale_rotation_translation(h.scale, Quat::from_rotation_y(h.yaw), h.pos);
            out.push(MeshDraw {
                mesh: h.mesh,
                model,
                tint: h.tint.extend(2.2),
                ..Default::default()
            });
        }
    }
}

fn hash(x: f32) -> f32 {
    ((x.floor() * 127.1 + 311.7).sin() * 43_758.547)
        .fract()
        .abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirages_appear_and_vanish_when_approached() {
        let mut g = Game::new(21);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        g.mirage_timer = 0.0;
        let mut inp = Input::default();
        g.update(1.0 / 60.0, &mut inp);
        assert_eq!(g.holograms.len(), 1, "keine Fata Morgana");
        let h = &g.holograms[0];
        let d = (h.pos - g.player.pos).length();
        assert!((9.0..=21.0).contains(&d), "Abstand {d}");
        assert!(
            g.holograms[0].revealed < 0.0,
            "schon von weitem als Hologramm erkennbar"
        );
        // hinlaufen (teleportieren) -> entpuppt sich, flackert kurz, ist weg
        g.player.pos = g
            .world
            .collide(vec3(h.pos.x, 0.0, h.pos.z) + vec3(1.0, 0.0, 0.0), 0.3);
        g.mirage_timer = 100.0;
        g.update(1.0 / 60.0, &mut inp);
        assert!(g.holograms[0].revealed >= 0.0, "nicht entlarvt");
        for _ in 0..60 {
            g.player.pos = g.world.collide(g.player.pos, 0.3);
            g.update(1.0 / 60.0, &mut inp);
        }
        assert!(g.holograms.is_empty());
    }

    #[test]
    fn posters_rot_with_escalation() {
        let mut g = Game::new(22);
        assert_eq!(g.poster_decay(), 0.0);
        g.tip();
        let a = g.poster_decay();
        g.tier = Tier::Collapse;
        assert!(g.poster_decay() > a);
        assert!(g.poster_decay() <= 1.0);
    }
}
