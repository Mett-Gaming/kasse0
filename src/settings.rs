//! Optionen, gespeichert als key=value in %APPDATA%\.mettlauncher\kasse0\settings.txt.

use std::path::PathBuf;

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

fn path() -> Option<PathBuf> {
    crate::crash::data_dir("kasse0").map(|d| d.join("settings.txt"))
}

impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let v = v.trim();
            let f = v.parse::<f32>().ok();
            match k.trim() {
                "sensitivity" => s.sensitivity = f.unwrap_or(s.sensitivity).clamp(0.2, 3.0),
                "volume" => s.volume = f.unwrap_or(s.volume).clamp(0.0, 1.0),
                "vhs" => s.vhs = f.unwrap_or(s.vhs).clamp(0.0, 1.5),
                "fov" => s.fov = f.unwrap_or(s.fov).clamp(55.0, 100.0),
                "fullscreen" => s.fullscreen = v != "false",
                "tutorial_asked" => s.tutorial_asked = v == "true",
                "tutorial_done" => s.tutorial_done = v == "true",
                // frueher gab es "tutorial" (an/aus): wird ignoriert
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        format!(
            "sensitivity={}\nvolume={}\nvhs={}\nfov={}\nfullscreen={}\ntutorial_asked={}\n\
             tutorial_done={}\n",
            self.sensitivity,
            self.volume,
            self.vhs,
            self.fov,
            self.fullscreen,
            self.tutorial_asked,
            self.tutorial_done
        )
    }

    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&p, self.to_text()) {
                crate::crash::log(&format!(
                    "Einstellungen konnten nicht gespeichert werden: {e}"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_fullscreen_unless_switched_off() {
        assert!(Settings::default().fullscreen);
        assert!(Settings::parse("volume=0.5").fullscreen);
        assert!(!Settings::parse("fullscreen=false").fullscreen);
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
        assert_eq!(Settings::parse(&s.to_text()), s);
        // neue Spieler: noch nicht gefragt, Tutorial noch nicht geschafft
        assert!(!Settings::default().tutorial_asked);
        assert!(!Settings::default().tutorial_done);
        let bad = Settings::parse("volume=7\nfov=abc\nunbekannt=1");
        assert_eq!(bad.volume, 1.0);
        assert_eq!(bad.fov, 70.0);
        // aeltere Datei mit der frueheren Tutorial-Option wird ohne Fehler gelesen
        let old = Settings::parse("fullscreen=true\ntutorial=false\n");
        assert!(old.fullscreen && !old.tutorial_asked);
    }
}
