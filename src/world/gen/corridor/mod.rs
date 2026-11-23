//! Der Fluchtkorridor hinter der Ladentuer. Es gibt ihn nur im gekippten Markt, wenn nach dem
//! Bezahlen die Tuer offen ist: ein langer, niedriger Gang mit rot flackernden Roehren, der bei
//! jedem Lauf neu gewuerfelt wird - gerade Stuecke, mal links, mal rechts um die Ecke, nie zurueck.
//! Unterwegs: Hindernisse (Slalom, Ducken, Springen, Tueren, kippende Regale, rollende Wagen,
//! Engstellen) und Ereignisse (Gestalten, veraendernder Gang). Am Ende ein grosser Spiegel und
//! seitlich davor die (klemmende) Ausgangstuer.
//!
//! - `layout.rs`: Weg wuerfeln (Eckpunkte), Laenge aus der Run-Zeit
//! - `build.rs`: Waende, Boden, Decke, Licht, Poster, Spiegel, Ausgangsraum
//! - `stations.rs`: Hindernisse und Ereignisse entlang des Wegs platzieren
//!
//! Das Spiel (game/corridor/) bewegt Tueren, Regale, Wagen usw. und prueft die Huerden.

use super::*;

mod build;
mod layout;
mod stations;

pub use layout::{corridor_len_for, LEN_MAX as CORRIDOR_LEN_MAX};

/// Halbe Breite des Gangs (Innenkante der Waende)
pub const CORRIDOR_HALF_W: f32 = 1.5;
/// Deckenhoehe im Gang (niedriger als im Markt)
pub const CORRIDOR_H: f32 = 2.7;
/// Spiegel am Ende: halbe Groesse (16:9 wie das Kamerabild) und Hoehe der Mitte
pub const MIRROR_HALF: Vec3 = Vec3::new(1.1, 0.62, 0.01);
pub const MIRROR_Y: f32 = 1.2;
/// Ausgang: Oeffnung in der linken Wand (+x) des letzten Stuecks, so weit vor dem Ende
pub const EXIT_FROM: f32 = 2.6;
pub const EXIT_TO: f32 = 0.5;
/// Tiefe des Ausgangsraums
const EXIT_DEPTH: f32 = 3.4;

/// Texte der Poster an den Waenden (Reihenfolge = Abstand zur Tuer)
pub const POSTER_TEXTS: &[&str] = &[
    "NICHT UMDREHEN",
    "SIE SIND DIREKT HINTER DIR",
    "DREH DICH NICHT UM",
    "NUR NACH VORNE",
    "SCHAU NICHT ZURÜCK",
    "KUNDE 0 HAT EIN GESICHT",
    "NICHT UMDREHEN",
    "SIEH IN DEN SPIEGEL",
];

/// Huerde quer im Gang: Ducken (blockiert, wer steht; `min.y` = Unterkante) oder Springen
/// (blockiert, wer nicht hoch genug ist; `max.y` = Oberkante). Keine normalen Solids.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hurdle {
    pub area: Aabb,
    pub duck: bool,
}

/// Was an einer Stelle des Wegs passiert (bewegt wird es im Spiel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StationKind {
    /// Schwingtuer quer im Gang, E drueckt sie auf; `stuck` = klemmt (3x E)
    Door { stuck: bool },
    /// hohes Regal an der Wand (`side` = Seite: +1 oder -1), kippt quer, wenn man naht
    Shelf { side: f32 },
    /// Einkaufswagen, rollt einem entgegen (`lane` = seitlicher Versatz)
    Cart { lane: f32 },
    /// Gestalt steht im Gang und verschwindet, wenn man naht
    Figure { variant: u32 },
    /// Decke senkt sich auf einem Stueck (dann ducken)
    Ceiling,
    /// Waende ruecken auf einem Stueck zusammen
    Walls,
    /// Ausgangstuer in der Seitenwand am Ende (klemmt immer)
    ExitDoor,
}

/// Eine Stelle entlang des Wegs: `s` = Meter ab der Ladentuer, `at` = Punkt auf der Mittellinie
/// (Boden), `dir` = Gangrichtung dort, `side` = quer dazu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Station {
    pub s: f32,
    pub kind: StationKind,
    pub at: Vec3,
    pub dir: Vec3,
    pub side: Vec3,
}

/// Was das Spiel ueber den Gang wissen muss.
#[derive(Clone, Debug)]
pub struct Corridor {
    /// Eckpunkte der Mittellinie (Boden): Ladentuer, Ecken, Ende an der Spiegelwand
    pub pts: Vec<Vec3>,
    /// Laenge der Mittellinie
    pub len: f32,
    /// Spiegelflaeche (zeigt nach -z, zum Spieler)
    pub mirror_center: Vec3,
    pub mirror_half: Vec3,
    pub hurdles: Vec<Hurdle>,
    pub stations: Vec<Station>,
    /// Engstellen: hier geht es nur langsam (seitlich quetschen)
    pub squeezes: Vec<Aabb>,
    /// Ab hier beginnt das letzte Stueck (Spiegel, Ausgangstuer)
    pub end_s: f32,
    /// Oeffnung der Ladentuer (knallt zu, sobald man drin ist): Mitte und halbe Breite (x)
    pub door_center: Vec3,
}

/// Seitenrichtung zu einer Gangrichtung (fuer +z: +x).
pub fn side_of(d: Vec3) -> Vec3 {
    vec3(d.z, 0.0, -d.x)
}

impl Corridor {
    /// Anzahl der geraden Stuecke
    pub fn segments(&self) -> usize {
        self.pts.len() - 1
    }

    /// Stueck `i`: Anfang, Richtung, Laenge, Weg bis zum Anfang.
    pub fn segment(&self, i: usize) -> (Vec3, Vec3, f32, f32) {
        let mut cum = 0.0;
        for k in 0..i {
            cum += (self.pts[k + 1] - self.pts[k]).length();
        }
        let a = self.pts[i];
        let d = self.pts[i + 1] - a;
        let l = d.length();
        (a, d / l.max(1e-4), l, cum)
    }

    /// Naechstes Stueck zu `p`: (Index, u entlang, v quer, Abstand ausserhalb des Gangs).
    fn locate(&self, p: Vec3) -> (usize, f32, f32, f32) {
        let hw = CORRIDOR_HALF_W;
        let mut best = (0, 0.0, 0.0, f32::MAX);
        let mut best_score = f32::MAX;
        for i in 0..self.segments() {
            let (a, d, l, _) = self.segment(i);
            let q = vec3(p.x - a.x, 0.0, p.z - a.z);
            let u = q.dot(d);
            let v = q.dot(side_of(d));
            // Rechteck des Stuecks (an Ecken um die halbe Breite verlaengert)
            let lo = if i == 0 { 0.0 } else { -hw };
            let hi = if i + 1 == self.segments() { l } else { l + hw };
            let du = (lo - u).max(u - hi).max(0.0);
            let dv = (v.abs() - hw).max(0.0);
            let out = (du * du + dv * dv).sqrt();
            // in einer Ecke liegt man in zwei Rechtecken: dann das Stueck, zu dessen
            // Mittellinie man gehoert (nicht in der Verlaengerung, nah an der Mitte)
            let ext = (-u).max(u - l).max(0.0);
            let score = out * 100.0 + ext + v.abs();
            if score < best_score {
                best_score = score;
                best = (i, u, v, out);
            }
        }
        best
    }

    /// Ist der Punkt im Gang (zwischen Ladentuer und Spiegelwand, ohne Ausgangsraum)?
    pub fn contains(&self, p: Vec3) -> bool {
        self.locate(p).3 <= 0.2
    }

    /// Wie weit ist der Punkt entlang des Wegs (Meter ab der Ladentuer)?
    pub fn progress(&self, p: Vec3) -> f32 {
        let (i, u, _, _) = self.locate(p);
        let (_, _, l, cum) = self.segment(i);
        cum + u.clamp(0.0, l)
    }

    /// Punkt auf der Mittellinie nach `s` Metern.
    pub fn point_at(&self, s: f32) -> Vec3 {
        let mut s = s.max(0.0);
        for i in 0..self.segments() {
            let (a, d, l, _) = self.segment(i);
            if s <= l || i + 1 == self.segments() {
                return a + d * s.min(l);
            }
            s -= l;
        }
        self.pts[0]
    }

    /// Gangrichtung nach `s` Metern.
    pub fn dir_at_s(&self, s: f32) -> Vec3 {
        let mut s = s.max(0.0);
        for i in 0..self.segments() {
            let (_, d, l, _) = self.segment(i);
            if s < l || i + 1 == self.segments() {
                return d;
            }
            s -= l;
        }
        Vec3::Z
    }

    /// Gangrichtung an diesem Punkt (fuer Tests und kuenftige Nutzer).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn dir_at(&self, p: Vec3) -> Vec3 {
        let (i, _, _, _) = self.locate(p);
        self.segment(i).1
    }

    /// Wie weit schaut `f` nach vorn (1) bzw. zurueck (-1)? In einer Ecke zaehlt die bessere
    /// der beiden Richtungen (man biegt ja gerade ab).
    pub fn facing(&self, p: Vec3, f: Vec3) -> f32 {
        let f = vec3(f.x, 0.0, f.z).normalize_or_zero();
        let hw = CORRIDOR_HALF_W;
        let (i, u, _, _) = self.locate(p);
        let (_, d, l, _) = self.segment(i);
        let mut best = f.dot(d);
        if u < hw && i > 0 {
            best = best.max(f.dot(self.segment(i - 1).1));
        }
        if u > l - hw && i + 1 < self.segments() {
            best = best.max(f.dot(self.segment(i + 1).1));
        }
        best
    }
}

/// Baut Gang, Hindernisse, Licht, Poster, Spiegel und Ausgangsraum. Liefert den Gang und den
/// Bereich, in dem man draussen ist (gewonnen).
/// `seed`: Zufall fuer Weg und Hindernisse (bei jedem Bezahlen neu, siehe WorldParams),
/// `len`: gewuenschte Laenge (aus der Run-Zeit, siehe `corridor_len_for`).
pub(super) fn build(
    b: &mut Builder,
    seed: u64,
    len: f32,
    door_cx: f32,
    z0: f32,
) -> (Corridor, Aabb) {
    let mut rng = Pcg64::seed_from_u64(seed ^ 0xC0_221D_0);
    let pts = layout::path(&mut rng, len, vec3(door_cx, 0.0, z0));
    let mut c = Corridor {
        len: pts.windows(2).map(|w| (w[1] - w[0]).length()).sum(),
        pts,
        mirror_center: Vec3::ZERO,
        mirror_half: MIRROR_HALF,
        hurdles: Vec::new(),
        stations: Vec::new(),
        squeezes: Vec::new(),
        end_s: 0.0,
        door_center: vec3(door_cx, 0.0, z0),
    };
    let last = c.segments() - 1;
    c.end_s = c.segment(last).3;
    let exit = build::shell(b, &mut rng, &mut c);
    stations::place(b, &mut rng, &mut c);
    (c, exit)
}

#[cfg(test)]
mod tests;
