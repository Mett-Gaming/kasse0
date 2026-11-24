//! Optionen, gespeichert als key=value in %APPDATA%\.mettlauncher\kasse0\settings.txt.

use crate::hud::options::Row;
use mettgine::settings::KeyValues;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub sensitivity: f32,
    pub volume: f32,
    /// Musik, zusaetzlich zur Gesamtlautstaerke
    pub music: f32,
    pub vhs: f32,
    pub fov: f32,
    pub fullscreen: bool,
    /// Bilder pro Sekunde hoechstens, 0 = unbegrenzt (einer aus FPS_LIMITS)
    pub fps_limit: u32,
    /// Reflex bzw. Anti-Lag
    pub low_latency: bool,
    /// Beim ersten Einkauf schon gefragt, ob man das Tutorial spielen will?
    pub tutorial_asked: bool,
    /// Tutorial einmal geschafft? Dann gibt es "Tutorial spielen" in den Optionen.
    pub tutorial_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            volume: 0.8,
            music: 1.0,
            vhs: 1.0,
            fov: 70.0,
            fullscreen: true,
            fps_limit: 0,
            low_latency: true,
            tutorial_asked: false,
            tutorial_done: false,
        }
    }
}

/// Waehlbare Bildraten
const FPS_LIMITS: [u32; 5] = [0, 60, 120, 144, 240];

const APP: &str = "kasse0";
const FILE: &str = "settings.txt";

impl Settings {
    fn from_values(kv: &KeyValues) -> Self {
        let d = Self::default();
        let on = |key| kv.str(key) == Some("true");
        Self {
            sensitivity: kv.f32("sensitivity", d.sensitivity, 0.2, 3.0),
            volume: kv.f32("volume", d.volume, 0.0, 1.0),
            music: kv.f32("music", d.music, 0.0, 1.0),
            vhs: kv.f32("vhs", d.vhs, 0.0, 1.5),
            fov: kv.f32("fov", d.fov, 55.0, 100.0),
            fullscreen: kv.str("fullscreen") != Some("false"),
            fps_limit: Some(kv.u32("fps_limit", d.fps_limit, 0, 1000))
                .filter(|v| FPS_LIMITS.contains(v))
                .unwrap_or(d.fps_limit),
            low_latency: kv.bool("low_latency", d.low_latency),
            // frueher gab es "tutorial" (an/aus): wird ignoriert
            tutorial_asked: on("tutorial_asked"),
            tutorial_done: on("tutorial_done"),
        }
    }

    fn to_values(&self) -> KeyValues {
        let mut kv = KeyValues::default();
        kv.set("sensitivity", self.sensitivity);
        kv.set("volume", self.volume);
        kv.set("music", self.music);
        kv.set("vhs", self.vhs);
        kv.set("fov", self.fov);
        kv.set("fullscreen", self.fullscreen);
        kv.set("fps_limit", self.fps_limit);
        kv.set("low_latency", self.low_latency);
        kv.set("tutorial_asked", self.tutorial_asked);
        kv.set("tutorial_done", self.tutorial_done);
        kv
    }

    /// Zahl hinter einer Zeile mit Schieberegler.
    pub fn number(&self, row: Row) -> f32 {
        match row {
            Row::Sensitivity => self.sensitivity,
            Row::Volume => self.volume,
            Row::Music => self.music,
            Row::Vhs => self.vhs,
            Row::Fov => self.fov,
            _ => 0.0,
        }
    }

    /// Schieberegler setzen (in seinen Grenzen).
    pub fn set_number(&mut self, row: Row, v: f32) {
        let Some((min, max)) = row.range() else {
            return;
        };
        // gerundet, sonst sammeln sich Fehler der Schritte an
        let v = (v.clamp(min, max) * 100.0).round() / 100.0;
        match row {
            Row::Sensitivity => self.sensitivity = v,
            Row::Volume => self.volume = v,
            Row::Music => self.music = v,
            Row::Vhs => self.vhs = v,
            Row::Fov => self.fov = v.round(),
            _ => {}
        }
    }

    /// Wert einer Optionszeile verstellen; `dir` = +1/-1.
    pub fn change(&mut self, row: Row, dir: i32) {
        match row {
            Row::Sensitivity | Row::Volume | Row::Music | Row::Vhs | Row::Fov => {
                let step = match row {
                    Row::Vhs => 0.25,
                    Row::Fov => 5.0,
                    _ => 0.1,
                };
                self.set_number(row, self.number(row) + step * dir as f32);
            }
            Row::Fullscreen => self.fullscreen = !self.fullscreen,
            Row::FpsLimit => {
                let n = FPS_LIMITS.len() as i32;
                let i = FPS_LIMITS.iter().position(|v| *v == self.fps_limit);
                let next = (i.unwrap_or(0) as i32 + dir).rem_euclid(n);
                self.fps_limit = FPS_LIMITS[next as usize];
            }
            Row::LowLatency => self.low_latency = !self.low_latency,
            Row::Tabs | Row::Jumpscares | Row::Tutorial | Row::Back => {}
        }
    }

    pub fn load() -> Self {
        Self::from_values(&mettgine::settings::load(APP, FILE))
    }

    pub fn save(&self) {
        mettgine::settings::save(APP, FILE, &self.to_values());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Settings {
        Settings::from_values(&KeyValues::parse(text))
    }

    #[test]
    fn starts_fullscreen_unless_switched_off() {
        assert!(Settings::default().fullscreen);
        assert!(parse("volume=0.5").fullscreen);
        assert!(!parse("fullscreen=false").fullscreen);
    }

    #[test]
    fn every_value_row_changes_something_and_the_rate_wraps_around() {
        use crate::hud::options::{rows, PAGES};
        for page in PAGES {
            for row in rows(page, true)
                .into_iter()
                .filter(|r| r.is_value() && !matches!(r, Row::Tabs | Row::Jumpscares))
            {
                let mut s = Settings::default();
                s.change(row, -1);
                assert_ne!(s, Settings::default(), "{row:?} tut nichts");
            }
        }
        let mut s = Settings::default();
        for _ in 0..FPS_LIMITS.len() {
            s.change(Row::FpsLimit, 1);
        }
        assert_eq!(s.fps_limit, 0);
    }

    #[test]
    fn roundtrip_and_clamping() {
        let s = Settings {
            sensitivity: 1.5,
            volume: 0.3,
            music: 0.4,
            vhs: 0.5,
            fov: 80.0,
            fullscreen: true,
            fps_limit: 144,
            low_latency: false,
            tutorial_asked: true,
            tutorial_done: true,
        };
        assert_eq!(parse(&s.to_values().to_text()), s);
        // neue Spieler: noch nicht gefragt, Tutorial noch nicht geschafft
        assert!(!Settings::default().tutorial_asked);
        assert!(!Settings::default().tutorial_done);
        let bad = parse("volume=7\nfov=abc\nunbekannt=1");
        assert_eq!(bad.volume, 1.0);
        assert_eq!(bad.fov, 70.0);
        assert_eq!(parse("fps_limit=77").fps_limit, 0);
        // aeltere Datei mit der frueheren Tutorial-Option wird ohne Fehler gelesen
        let old = parse("fullscreen=true\ntutorial=false\n");
        assert!(old.fullscreen && !old.tutorial_asked);
    }
}
