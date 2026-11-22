//! KI-Director: steuert die Spannungskurve im gekippten Markt. Druck baut sich auf
//! (immer mehr Verfolger ruecken wirklich heran), erreicht einen Hoehepunkt (alle), dann
//! folgt eine Ruhephase (niemand drueckt, die Verfolger lauern nur). Nach einem Tod und
//! nach grossen Schrecken gibt es Ruhe. Waehrend eines Gespraechs steht die Kurve still,
//! und auf dem Hoehepunkt gibt es keine Scares: Druck und Schrecken wechseln sich ab.

/// Aufbau bis zum Hoehepunkt, Dauer des Hoehepunkts, Ruhe danach (Sekunden)
const BUILD_TIME: f32 = 55.0;
const PEAK_TIME: f32 = 20.0;
const RELAX_TIME: f32 = 15.0;
/// Druck zu Beginn bzw. nach einer Ruhephase
const START_PRESSURE: f32 = 0.35;
const AFTER_RELAX: f32 = 0.2;
/// Ruhe nach einem Tod bzw. nach einem grossen Schrecken
pub const CALM_AFTER_DEATH: f32 = 20.0;
pub const CALM_AFTER_SCARE: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Build,
    Peak,
    Relax,
}

#[derive(Clone, Copy, Debug)]
pub struct Director {
    pub phase: Phase,
    /// Sekunden in der aktuellen Phase
    pub t: f32,
    /// 0..1, wie viele Verfolger gerade heranruecken duerfen
    pub pressure: f32,
    relax_len: f32,
}

impl Default for Director {
    fn default() -> Self {
        Self {
            phase: Phase::Build,
            t: 0.0,
            pressure: START_PRESSURE,
            relax_len: RELAX_TIME,
        }
    }
}

impl Director {
    /// Weiterlaufen lassen; `paused` (Gespraech, Markt noch normal) haelt die Kurve an.
    pub fn update(&mut self, dt: f32, paused: bool) {
        if paused {
            return;
        }
        self.t += dt;
        match self.phase {
            Phase::Build => {
                self.pressure = (self.pressure + dt / BUILD_TIME).min(1.0);
                if self.pressure >= 1.0 {
                    self.enter(Phase::Peak);
                }
            }
            Phase::Peak => {
                self.pressure = 1.0;
                if self.t > PEAK_TIME {
                    self.relax_len = RELAX_TIME;
                    self.enter(Phase::Relax);
                }
            }
            Phase::Relax => {
                self.pressure = 0.0;
                if self.t > self.relax_len {
                    self.enter(Phase::Build);
                    self.pressure = AFTER_RELAX;
                }
            }
        }
    }

    fn enter(&mut self, p: Phase) {
        self.phase = p;
        self.t = 0.0;
    }

    /// Ruhe fuer mindestens `secs` Sekunden (nach Tod oder Schrecken). Eine laengere
    /// laufende Ruhe wird nicht verkuerzt.
    pub fn calm_down(&mut self, secs: f32) {
        if self.phase == Phase::Relax && self.relax_len - self.t >= secs {
            return;
        }
        self.enter(Phase::Relax);
        self.relax_len = secs;
        self.pressure = 0.0;
    }

    /// Wie viele von `n` jagenden Verfolgern gerade wirklich heranruecken.
    pub fn pushers(&self, n: usize) -> usize {
        match self.phase {
            Phase::Relax => 0,
            Phase::Peak => n,
            Phase::Build => ((self.pressure * n as f32).ceil() as usize).clamp(1.min(n), n),
        }
    }

    /// Duerfen gerade Scares kommen? (nicht auf dem Hoehepunkt)
    pub fn allows_scares(&self) -> bool {
        self.phase != Phase::Peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tension_builds_peaks_and_relaxes() {
        let mut d = Director::default();
        assert_eq!(d.phase, Phase::Build);
        assert!(d.pushers(4) >= 1 && d.pushers(4) < 4);
        let mut seen = vec![d.phase];
        for _ in 0..(200 * 10) {
            d.update(0.1, false);
            if *seen.last().unwrap() != d.phase {
                seen.push(d.phase);
            }
        }
        assert!(seen.starts_with(&[Phase::Build, Phase::Peak, Phase::Relax, Phase::Build]));
        // Pause haelt alles an
        let before = (d.phase, d.t);
        d.update(5.0, true);
        assert_eq!((d.phase, d.t), before);
    }

    #[test]
    fn calm_after_death_holds_everyone_back() {
        let mut d = Director::default();
        d.calm_down(CALM_AFTER_DEATH);
        // ein kurzer Schrecken verkuerzt die lange Ruhe nicht
        d.calm_down(CALM_AFTER_SCARE);
        assert!(d.relax_len >= CALM_AFTER_DEATH);
        for _ in 0..190 {
            d.update(0.1, false);
            assert_eq!(d.pushers(5), 0, "drueckt schon nach {} s", d.t);
        }
        for _ in 0..30 {
            d.update(0.1, false);
        }
        assert_eq!(d.phase, Phase::Build);
        assert!(d.pushers(5) >= 1);
        assert!(!Director {
            phase: Phase::Peak,
            ..Default::default()
        }
        .allows_scares());
    }
}
