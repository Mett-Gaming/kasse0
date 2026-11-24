//! Optionen, gespeichert als key=value in %APPDATA%\.mettlauncher\kasse0\settings.txt.

use mettgine::settings::KeyValues;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub sensitivity: f32,
    pub volume: f32,
    pub vhs: f32,
    pub fov: f32,
    pub fullscreen: bool,
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
            vhs: 1.0,
            fov: 70.0,
            fullscreen: true,
            tutorial_asked: false,
            tutorial_done: false,
        }
    }
}

const APP: &str = "kasse0";
const FILE: &str = "settings.txt";

impl Settings {
    fn from_values(kv: &KeyValues) -> Self {
        let d = Self::default();
        let on = |key| kv.str(key) == Some("true");
        Self {
            sensitivity: kv.f32("sensitivity", d.sensitivity, 0.2, 3.0),
            volume: kv.f32("volume", d.volume, 0.0, 1.0),
            vhs: kv.f32("vhs", d.vhs, 0.0, 1.5),
            fov: kv.f32("fov", d.fov, 55.0, 100.0),
            fullscreen: kv.str("fullscreen") != Some("false"),
            // frueher gab es "tutorial" (an/aus): wird ignoriert
            tutorial_asked: on("tutorial_asked"),
            tutorial_done: on("tutorial_done"),
        }
    }

    fn to_values(&self) -> KeyValues {
        let mut kv = KeyValues::default();
        kv.set("sensitivity", self.sensitivity);
        kv.set("volume", self.volume);
        kv.set("vhs", self.vhs);
        kv.set("fov", self.fov);
        kv.set("fullscreen", self.fullscreen);
        kv.set("tutorial_asked", self.tutorial_asked);
        kv.set("tutorial_done", self.tutorial_done);
        kv
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
    fn roundtrip_and_clamping() {
        let s = Settings {
            sensitivity: 1.5,
            volume: 0.3,
            vhs: 0.5,
            fov: 80.0,
            fullscreen: true,
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
        // aeltere Datei mit der frueheren Tutorial-Option wird ohne Fehler gelesen
        let old = parse("fullscreen=true\ntutorial=false\n");
        assert!(old.fullscreen && !old.tutorial_asked);
    }
}
