//! Zeichenflaeche fuer Wandplakate: Flaechen, Schriftzuege und kleine Figuren aus Boxen auf
//! einer Plakattafel. Benutzt von der HACKEPETER-Werbung (ads.rs) und dem Plakat-Sortiment
//! (sortiment/). Keine Texturen: alles aus Boxen und Schriftzuegen des Schrift-Atlas.

use super::*;

/// Plakatgroesse: halbe Breite und halbe Hoehe (Meter)
pub const POSTER_HALF: (f32, f32) = (0.5, 0.7);
/// So weit steht die Plakatflaeche vor der Wand (vor flachen Zierleisten, Meter)
pub const POSTER_OUT: f32 = 0.03;

/// Schriftfarben der Schilder (siehe `Builder::label`)
pub const WHITE: f32 = 0.0;
pub const BLUE: f32 = 1.0;
pub const BLACK: f32 = 2.0;
pub const YELLOW: f32 = 3.0;
pub const RED: f32 = 4.0;

/// Zeichnet auf eine Plakatflaeche: lokale Koordinaten u (quer, aus Sicht des Betrachters
/// nach rechts) und v (hoch) in Metern ab Plakatmitte; Ebenen liegen leicht vor dem Plakat.
pub struct Canvas<'a> {
    pub b: &'a mut Builder,
    pub c: Vec3,
    pub across: Vec3,
    pub front: Vec3,
    pub yaw: f32,
    pub west: bool,
    /// Gruselstufe 0 (normaler Markt) bis 4 (Kollaps), siehe `sortiment::level_of`
    pub level: u8,
    /// halbe Breite und Hoehe der Tafel
    pub hw: f32,
    pub hh: f32,
}

impl<'a> Canvas<'a> {
    /// Tafel an Wandpunkt `c` (Mitte der Plakatflaeche), `front` = aus der Wand heraus.
    pub fn new(b: &'a mut Builder, c: Vec3, front: Vec3, level: u8) -> Self {
        let (hw, hh) = POSTER_HALF;
        Self {
            b,
            c,
            across: vec3(front.z, 0.0, -front.x),
            front,
            yaw: front.x.atan2(front.z),
            west: front.x.abs() > 0.5,
            level,
            hw,
            hh,
        }
    }

    /// Ist der Markt gekippt (Stufe > 0)?
    pub fn tipped(&self) -> bool {
        self.level > 0
    }

    /// Die Tafel selbst (Grundfarbe, schon `fade`-behandelt uebergeben).
    pub fn board(&mut self, bg: Vec3) {
        let h = if self.west {
            vec3(0.004, self.hh, self.hw)
        } else {
            vec3(self.hw, self.hh, 0.004)
        };
        self.b.add(self.c, h, bg, PLAIN);
    }

    /// Farbe je Gruselstufe: frisch, dann immer blasser und dunkler.
    pub fn fade(&self, col: Vec3) -> Vec3 {
        let (desat, dark) = [
            (0.0, 1.0),
            (0.3, 0.85),
            (0.5, 0.7),
            (0.65, 0.55),
            (0.8, 0.45),
        ][self.level.min(4) as usize];
        desaturate(col, desat) * dark
    }

    /// Text je Gruselstufe: normal (0-1), unheimlich (2), grauenhaft (3-4).
    pub fn pick<'t>(&self, normal: &'t str, eerie: &'t str, horror: &'t str) -> &'t str {
        match self.level {
            0 | 1 => normal,
            2 => eerie,
            _ => horror,
        }
    }

    pub fn at(&self, u: f32, v: f32, layer: f32) -> Vec3 {
        self.c + self.across * u + vec3(0.0, v, 0.0) + self.front * (0.004 + 0.003 * layer)
    }

    /// Flaeche (halbe Breite `hu`, halbe Hoehe `hv`)
    pub fn rect(&mut self, u: f32, v: f32, hu: f32, hv: f32, layer: f32, col: Vec3) {
        let half = if self.west {
            vec3(0.0015, hv, hu)
        } else {
            vec3(hu, hv, 0.0015)
        };
        let p = self.at(u, v, layer);
        self.b.add(p, half, col, PLAIN);
    }

    /// Schriftzug auf eigener Grundfarbe `bg`
    #[allow(clippy::too_many_arguments)]
    pub fn text(&mut self, u: f32, v: f32, hw: f32, hh: f32, text: &str, bg: Vec3, ink: f32) {
        let p = self.at(u, v, 2.0);
        self.b.label(p, self.yaw, hw, hh, text, bg, ink, 0.0, 0.0);
    }

    /// Kopf und Schultern aus Boxen (Portraet), `s` = Groesse; Augen je Gruselstufe:
    /// normal hell, ab Stufe 3 schwarze Hoehlen, im Kollaps rot.
    pub fn bust(&mut self, u: f32, v: f32, s: f32, skin: Vec3, shirt: Vec3) {
        self.rect(u, v - 0.16 * s, 0.2 * s, 0.08 * s, 1.0, shirt);
        self.rect(u, v - 0.06 * s, 0.035 * s, 0.03 * s, 1.0, skin * 0.9);
        self.rect(u, v + 0.06 * s, 0.09 * s, 0.11 * s, 1.0, skin);
        let eye = match self.level {
            0..=2 => vec3(0.15, 0.13, 0.12),
            3 => vec3(0.0, 0.0, 0.0),
            _ => vec3(0.7, 0.02, 0.02),
        };
        let size = if self.level >= 3 { 0.022 } else { 0.012 };
        for k in [-1.0f32, 1.0] {
            self.rect(u + k * 0.04 * s, v + 0.08 * s, size * s, size * s, 2.0, eye);
        }
        // Mund: Laecheln wird zum Strich, dann zum Loch
        let (mw, mh) = match self.level {
            0 | 1 => (0.04, 0.006),
            2 => (0.03, 0.004),
            _ => (0.02, 0.025),
        };
        self.rect(
            u,
            v + 0.015 * s,
            mw * s,
            mh * s,
            2.0,
            vec3(0.25, 0.08, 0.08),
        );
    }

    /// Mettbroetchen: Unterhaelfte, Mett mit drei Zwiebelstuecken, gewoelbte Oberhaelfte
    pub fn bun(&mut self, u: f32, v: f32, s: f32) {
        let (bun, crust, mett, onion) = if self.tipped() {
            (
                vec3(0.35, 0.28, 0.2),
                vec3(0.3, 0.24, 0.17),
                vec3(0.3, 0.1, 0.1),
                vec3(0.55, 0.55, 0.5),
            )
        } else {
            (
                vec3(0.84, 0.6, 0.32),
                vec3(0.74, 0.5, 0.26),
                vec3(0.86, 0.35, 0.37),
                vec3(0.96, 0.94, 0.86),
            )
        };
        self.rect(u, v - 0.07 * s, 0.2 * s, 0.03 * s, 1.0, crust);
        self.rect(u, v - 0.015 * s, 0.23 * s, 0.03 * s, 1.0, mett);
        for k in [-0.13f32, 0.0, 0.13] {
            self.rect(u + k * s, v - 0.015 * s, 0.022 * s, 0.012 * s, 2.0, onion);
        }
        self.rect(u, v + 0.05 * s, 0.2 * s, 0.045 * s, 1.0, bun);
        self.rect(u, v + 0.105 * s, 0.15 * s, 0.018 * s, 1.0, bun);
    }
}
