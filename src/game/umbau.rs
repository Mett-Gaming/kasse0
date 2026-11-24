//! Umbau im Dunkeln: Wenn sich der gekippte Markt live umbaut (neue Stufe, neu gemischte
//! Gaenge), flackert erst das Licht, dann ist es kurz ganz dunkel - und genau dann wird die
//! Welt ausgetauscht. Danach flackert das Licht wieder an. So sieht man den Wechsel nicht
//! (frueher sprang die ganze Welt in einem Bild um und es wirkte wie ein Teleport).
//!
//! Ein Umbau kann ein vorheriges Wiederanflackern zuruecksetzen; der Text des letzten
//! Ausloesers gewinnt.

use super::*;

/// So lange flackert das Licht, bevor es ausgeht (s)
pub const FLICKER_OUT: f32 = 0.55;
/// Ab hier wieder Licht (s)
pub const DARK_END: f32 = 0.95;
/// In der Mitte der Dunkelheit wird umgebaut (s)
pub const SWAP_AT: f32 = 0.65;
/// Ganzer Ablauf (s)
pub const UMBAU_TIME: f32 = 1.4;

/// Laufender Umbau.
#[derive(Clone, Debug, Default)]
pub struct Umbau {
    /// Zeit seit Beginn (s)
    pub t: f32,
    /// Welt schon ausgetauscht?
    pub swapped: bool,
    /// Text, der beim Wiederanflackern erscheint
    pub msg: Option<&'static str>,
}

impl Game {
    /// Umbau im Dunkeln starten. Laeuft schon einer, baut er einfach den neuesten Stand
    /// (die Parameter sind dann schon gesetzt); ein neuer Text ersetzt den alten.
    pub(super) fn start_umbau(&mut self, msg: Option<&'static str>) {
        if let Some(u) = &mut self.umbau {
            if !u.swapped {
                if msg.is_some() {
                    u.msg = msg;
                }
                return;
            }
            // schon umgebaut, Licht kommt gerade wieder: zurueck ins Dunkle, noch einmal umbauen
            u.t = FLICKER_OUT;
            u.swapped = false;
            if msg.is_some() {
                u.msg = msg;
            }
            return;
        }
        self.umbau = Some(Umbau {
            msg,
            ..Default::default()
        });
        self.play(Sfx::Spark, 0.5);
    }

    /// Laeuft gerade ein Umbau?
    pub fn umbau_running(&self) -> bool {
        self.umbau.is_some()
    }

    /// Umbau sofort abschliessen (Run vorbei, bezahlt, Tod): nie halb umgebaut lassen.
    pub(super) fn finish_umbau(&mut self) {
        if let Some(u) = self.umbau.take() {
            if !u.swapped {
                self.rebuild_world();
            }
        }
    }

    pub(super) fn update_umbau(&mut self, dt: f32) {
        let Some(u) = &self.umbau else { return };
        if self.outcome != Outcome::Running || self.dying.is_some() || self.paid {
            self.finish_umbau();
            return;
        }
        let (t, now) = (u.t, u.t + dt);
        if !u.swapped && now >= SWAP_AT {
            self.rebuild_world();
            self.play(Sfx::Rumble, 0.35);
            if let Some(u) = &mut self.umbau {
                u.swapped = true;
            }
        }
        if t < DARK_END && now >= DARK_END {
            self.play(Sfx::Spark, 0.4);
            if let Some(msg) = self.umbau.as_ref().and_then(|u| u.msg) {
                self.say(msg, 4.0);
            }
        }
        if now >= UMBAU_TIME {
            self.umbau = None;
        } else if let Some(u) = &mut self.umbau {
            u.t = now;
        }
    }

    /// Wie dunkel der Umbau das Bild macht (0 = normal, 1 = schwarz): unruhiges Flackern,
    /// dann ganz dunkel, dann flackert es wieder an.
    pub fn umbau_dark(&self) -> f32 {
        let Some(u) = &self.umbau else { return 0.0 };
        let t = u.t;
        // fest aus der Zeit gewuerfeltes Flackermuster (alle 1/20 s ein neuer Wert)
        let flick = |seed: f32| {
            let k = (t * 20.0).floor() + seed;
            let r = ((k * 12.9898).sin() * 43_758.547).fract().abs();
            if r < 0.35 {
                0.0
            } else if r < 0.65 {
                0.55
            } else {
                0.92
            }
        };
        if t < FLICKER_OUT {
            // wird immer unruhiger und dunkler
            let k = t / FLICKER_OUT;
            (flick(0.0) * (0.4 + 0.6 * k)).max(0.35 * k)
        } else if t < DARK_END {
            1.0
        } else {
            let k = ((t - DARK_END) / (UMBAU_TIME - DARK_END)).clamp(0.0, 1.0);
            (flick(7.0) * (1.0 - k)).max(0.0)
        }
    }
}
