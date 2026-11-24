//! Optionen: Startseite mit den Bereichen Grafik, Klang und Steuerung.

use super::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Main,
    Graphics,
    Sound,
    Controls,
}

/// Eine Zeile im Optionsmenue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    Open(Page),
    /// laesst sich nicht abschalten (siehe main.rs: menu_scare)
    Jumpscares,
    /// erst, wenn man das Tutorial einmal geschafft hat
    Tutorial,
    Sensitivity,
    Volume,
    Vhs,
    Fov,
    Fullscreen,
    FpsLimit,
    LowLatency,
    Back,
}

impl Row {
    /// Mit links/rechts verstellbar?
    pub fn is_value(self) -> bool {
        !matches!(self, Row::Open(_) | Row::Tutorial | Row::Back)
    }
}

/// Zeilen einer Seite.
pub fn rows(page: Page, tutorial: bool) -> Vec<Row> {
    let mut rows = match page {
        Page::Main => vec![
            Row::Open(Page::Graphics),
            Row::Open(Page::Sound),
            Row::Open(Page::Controls),
            Row::Jumpscares,
        ],
        Page::Graphics => vec![
            Row::Vhs,
            Row::Fov,
            Row::Fullscreen,
            Row::FpsLimit,
            Row::LowLatency,
        ],
        Page::Sound => vec![Row::Volume],
        Page::Controls => vec![Row::Sensitivity],
    };
    if page == Page::Main && tutorial {
        rows.push(Row::Tutorial);
    }
    rows.push(Row::Back);
    rows
}

pub fn title(page: Page) -> &'static str {
    match page {
        Page::Main => "OPTIONEN",
        Page::Graphics => "GRAFIK",
        Page::Sound => "KLANG",
        Page::Controls => "STEUERUNG",
    }
}

fn value(name: &str, v: String) -> String {
    format!("{name:<21} < {v:<5} >")
}

fn on_off(on: bool) -> String {
    if on { "An" } else { "Aus" }.to_string()
}

fn percent(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

pub fn label(row: Row, s: &Settings) -> String {
    match row {
        Row::Open(Page::Graphics) => "Grafik".into(),
        Row::Open(Page::Sound) => "Klang".into(),
        Row::Open(Page::Controls) => "Steuerung".into(),
        Row::Open(Page::Main) => "Optionen".into(),
        Row::Jumpscares => value("Jumpscares", on_off(true)),
        Row::Tutorial => "Tutorial spielen".into(),
        Row::Sensitivity => value("Mausempfindlichkeit", format!("{:.1}x", s.sensitivity)),
        Row::Volume => value("Lautstärke", percent(s.volume)),
        Row::Vhs => value("VHS-Effekt", percent(s.vhs)),
        Row::Fov => value("Sichtfeld", format!("{:.0}°", s.fov)),
        Row::Fullscreen => value("Vollbild", on_off(s.fullscreen)),
        Row::FpsLimit => value(
            "Bildrate höchstens",
            match s.fps_limit {
                0 => "frei".into(),
                n => n.to_string(),
            },
        ),
        Row::LowLatency => value("Weniger Verzögerung", on_off(s.low_latency)),
        Row::Back => "Zurück".into(),
    }
}

pub fn draw(u: &mut Ui, sel: usize, time: f32, s: &Settings, from_pause: bool, page: Page) {
    let shade = if from_pause { 0.7 } else { 0.5 };
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, shade]);
    u.text_center(40.0, title(page), WHITE, 2.0);
    let tutorial = s.tutorial_done;
    let items: Vec<String> = rows(page, tutorial).iter().map(|r| label(*r, s)).collect();
    let screen = Screen::Options {
        from_pause,
        tutorial,
        page,
    };
    draw_menu(u, screen, &items, sel, time);
    u.text_center(
        220.0,
        "Pfeiltasten / A D ändern - Enter bestätigt - Esc zurück",
        DIM,
        1.0,
    );
    u.text_center(233.0, "Klick: erhöhen - Rechtsklick: verringern", DIM, 1.0);
    u.text_center(
        250.0,
        "Modelle: Quaternius, Poly Haven, Poly by Google",
        DIM,
        1.0,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGES: [Page; 4] = [Page::Main, Page::Graphics, Page::Sound, Page::Controls];

    #[test]
    fn every_page_fits_above_the_hints_and_ends_with_back() {
        let s = Settings::default();
        for page in PAGES {
            let rows = rows(page, true);
            assert_eq!(*rows.last().unwrap(), Row::Back);
            let (y, step) = menu_layout(Screen::Options {
                from_pause: true,
                tutorial: true,
                page,
            });
            assert!(y + rows.len() as f32 * step <= 220.0, "{page:?} zu lang");
            for r in rows {
                assert!(Ui::text_width(&label(r, &s), 1.0) < 360.0);
            }
        }
    }

    #[test]
    fn the_tutorial_row_appears_only_after_finishing_it() {
        assert!(!rows(Page::Main, false).contains(&Row::Tutorial));
        assert!(rows(Page::Main, true).contains(&Row::Tutorial));
        assert!(!rows(Page::Sound, true).contains(&Row::Tutorial));
    }
}
