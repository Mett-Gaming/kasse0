//! Ueberwachungsmonitor an der Kasse: zeigt live das Bild der Deckenkamera (zweite Kamera,
//! render-to-texture). Darauf sieht man sich selbst von hinten. Schaut man lange genug hin,
//! steht auf dem Bildschirm jemand direkt hinter einem. Dreht man sich um: niemand da.
//! Kann sich nach der Abklingzeit wiederholen.

use super::*;
use crate::people::{Clip, PersonDraw};

/// So lange muss man auf den Monitor schauen, bis sich etwas tut
const WATCH_TIME: f32 = 0.6;
/// Ab hier gilt "auf den Monitor schauen" (Abstand, Blickwinkel als Kosinus)
const WATCH_DIST: f32 = 6.0;
const WATCH_COS: f32 = 0.93;
/// Schaut man weiter weg als das (Kosinus), verschwindet die Gestalt
const LOOK_AWAY_COS: f32 = 0.6;
/// Ablauf der Gestalt: erst naeherkommen, dann Schreck, dann weg
const GHOST_STRIKE: f32 = 0.8;
const GHOST_END: f32 = 2.0;
/// Abstand der Gestalt hinter dem Spieler: anfangs, beim Schreck
const GHOST_FAR: f32 = 1.1;
const GHOST_NEAR: f32 = 0.4;
/// Sichtfeld der Kamera (Grad)
const CAM_FOV: f32 = 68.0;

impl Game {
    /// Kamera fuer das Monitorbild: (view_proj, Position). None = kein Monitor im Markt.
    pub fn monitor_view(&self, aspect: f32) -> Option<(Mat4, Vec3)> {
        // Im Fluchtkorridor filmt die zweite Kamera das Spiegelbild (corridor.rs)
        if let Some(mirror) = self.mirror_view() {
            return Some(mirror);
        }
        let lane = self.open_lane()?;
        let pos = lane.camera - vec3(0.0, 0.32, 0.0);
        // im gekippten Markt folgt die Kamera dem Spieler (wie das Modell in checkout.rs)
        let dir = if self.tipped {
            (self.player.pos + vec3(0.0, 1.0, 0.0) - pos).normalize_or_zero()
        } else {
            let yaw = std::f32::consts::PI * 0.85;
            vec3(yaw.sin(), -0.55, yaw.cos()).normalize()
        };
        if dir == Vec3::ZERO {
            return None;
        }
        let view = Mat4::look_to_rh(pos, dir, Vec3::Y);
        let proj = Mat4::perspective_rh(CAM_FOV.to_radians(), aspect, 0.05, 45.0);
        Some((proj * view, pos))
    }

    /// Wie genau man gerade auf den Monitor schaut (Kosinus) und wie weit weg er ist.
    fn monitor_gaze(&self) -> Option<(f32, f32)> {
        let lane = self.open_lane()?;
        let to = lane.monitor - self.eye();
        let dist = to.length();
        Some((self.view_dir().dot(to / dist.max(0.001)), dist))
    }

    pub(super) fn update_monitor_scare(&mut self, dt: f32) {
        let Some((cos, dist)) = self.monitor_gaze() else {
            return;
        };
        if let Some(t) = &mut self.monitor_ghost {
            let before = *t;
            *t += dt;
            let now = *t;
            if cos < LOOK_AWAY_COS || now >= GHOST_END {
                // umgedreht (oder vorbei): da ist niemand
                self.monitor_ghost = None;
                self.monitor_watch = 0.0;
                return;
            }
            if before < GHOST_STRIKE && now >= GHOST_STRIKE {
                self.fire_scare(0.8);
            }
            return;
        }
        let watching = dist < WATCH_DIST && cos > WATCH_COS;
        self.monitor_watch = if watching {
            self.monitor_watch + dt
        } else {
            0.0
        };
        if self.monitor_watch > WATCH_TIME && self.can_scare(false) {
            self.monitor_ghost = Some(0.0);
            self.hush_t = HUSH_TIME_MONITOR;
        }
    }

    /// Figuren, die nur auf dem Monitor zu sehen sind: man selbst, und manchmal jemand dahinter.
    pub fn collect_monitor_people(&self, out: &mut Vec<PersonDraw>) {
        if self.open_lane().is_none() || self.dying.is_some() {
            return;
        }
        let fwd = self.view_dir();
        let flat = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let flat = if flat == Vec3::ZERO { Vec3::Z } else { flat };
        let pos = self.player.pos;
        let speed = vec3(self.player.vel.x, 0.0, self.player.vel.z).length();
        let me = crate::people::people_of(false)[0];
        out.push(PersonDraw {
            variant: me,
            clip: if speed > 0.3 { Clip::Walk } else { Clip::Idle },
            time: self.time,
            pos,
            yaw: npc::yaw_towards(pos, pos + flat),
            tint: Vec4::new(0.8, 0.8, 0.8, 0.0),
            ghost: false,
            flip: false,
            look_at: None,
            fx: Default::default(),
        });
        if let Some(t) = self.monitor_ghost {
            let k = (t / GHOST_STRIKE).min(1.0);
            let dist = if t >= GHOST_STRIKE {
                GHOST_NEAR
            } else {
                GHOST_FAR + (GHOST_NEAR + 0.25 - GHOST_FAR) * k
            };
            let at = pos - flat * dist;
            let staff = crate::people::people_of(true);
            out.push(PersonDraw {
                variant: staff[staff.len() - 1],
                clip: Clip::Idle,
                time: 0.0,
                pos: at,
                yaw: npc::yaw_towards(at, pos),
                tint: Vec4::new(0.75, 0.75, 0.75, 0.1),
                ghost: false,
                flip: false,
                look_at: None,
                fx: Default::default(),
            });
        }
    }

    /// Steht gerade jemand auf dem Monitor hinter einem? (Tests)
    pub fn monitor_ghost_visible(&self) -> bool {
        self.monitor_ghost.is_some()
    }
}

/// Stille, waehrend die Gestalt auf dem Monitor naeherkommt
const HUSH_TIME_MONITOR: f32 = GHOST_STRIKE + 0.2;
