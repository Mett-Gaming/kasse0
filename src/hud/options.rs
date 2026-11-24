//! Optionen: oben waagerechte Reiter (Anzeige, Gameplay, Steuerung), darunter die Werte.
//! Zeile 0 jeder Seite sind die Reiter selbst.

use super::*;

/// Ein Reiter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Display,
    Gameplay,
    Controls,
}

pub const PAGES: [Page; 3] = [Page::Display, Page::Gameplay, Page::Controls];

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Display => "ANZEIGE",
            Page::Gameplay => "GAMEPLAY",
            Page::Controls => "STEUERUNG",
        }
    }

    /// Nachbar-Reiter; am Ende geht es vorne weiter.
    pub fn next(self, dir: i32) -> Page {
        let i = PAGES.iter().position(|p| *p == self).unwrap_or(0) as i32;
        PAGES[(i + dir).rem_euclid(PAGES.len() as i32) as usize]
    }
}

/// Eine Zeile im Optionsmenue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Row {
    /// die Reiter
    Tabs,
    /// laesst sich nicht abschalten (siehe main.rs: menu_scare)
    Jumpscares,
    /// erst, wenn man das Tutorial einmal geschafft hat
    Tutorial,
    Sensitivity,
    Volume,
    Music,
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
        !matches!(self, Row::Tutorial | Row::Back)
    }

    fn name(self) -> &'static str {
        match self {
            Row::Tabs => "",
            Row::Jumpscares => "Jumpscares",
            Row::Tutorial => "Tutorial spielen",
            Row::Sensitivity => "Mausempfindlichkeit",
            Row::Volume => "Lautstärke",
            Row::Music => "Musik",
            Row::Vhs => "VHS-Effekt",
            Row::Fov => "Sichtfeld",
            Row::Fullscreen => "Vollbild",
            Row::FpsLimit => "Höchste Bildrate",
            Row::LowLatency => "Weniger Verzögerung",
            Row::Back => "Zurück",
        }
    }

    /// Erklaerung der gewaehlten Zeile.
    fn help(self) -> &'static str {
        match self {
            Row::Tabs => "Links / Rechts wechselt den Reiter",
            Row::Jumpscares => "Schreckmomente im Markt",
            Row::Tutorial => "Neuer Einkauf mit Anleitung",
            Row::Sensitivity => "Wie schnell sich der Blick mit der Maus dreht",
            Row::Volume => "Gesamte Lautstärke",
            Row::Music => "Kaufhaus-Gedudel",
            Row::Vhs => "Bildrauschen und Verzerrung der Videokassette",
            Row::Fov => "Wie viel du nach links und rechts siehst",
            Row::Fullscreen => "Randloses Vollbild oder Fenster",
            Row::FpsLimit => "Begrenzt die Bilder pro Sekunde",
            Row::LowLatency => "NVIDIA Reflex bzw. AMD Anti-Lag",
            Row::Back => "Speichert und schließt die Optionen",
        }
    }

    /// Grenzen der Werte mit Schieberegler.
    pub fn range(self) -> Option<(f32, f32)> {
        Some(match self {
            Row::Sensitivity => (0.2, 3.0),
            Row::Volume | Row::Music => (0.0, 1.0),
            Row::Vhs => (0.0, 1.5),
            Row::Fov => (55.0, 100.0),
            _ => return None,
        })
    }
}

/// Zeilen einer Seite.
pub fn rows(page: Page, tutorial: bool) -> Vec<Row> {
    let mut rows = vec![Row::Tabs];
    match page {
        Page::Display => rows.extend([
            Row::Vhs,
            Row::Fov,
            Row::Fullscreen,
            Row::FpsLimit,
            Row::LowLatency,
        ]),
        Page::Gameplay => {
            rows.extend([Row::Volume, Row::Music, Row::Jumpscares]);
            if tutorial {
                rows.push(Row::Tutorial);
            }
        }
        Page::Controls => rows.push(Row::Sensitivity),
    }
    rows.push(Row::Back);
    rows
}

/// Wert als Text.
pub fn value(row: Row, s: &Settings) -> String {
    let on_off = |on: bool| if on { "An" } else { "Aus" }.to_string();
    let percent = |v: f32| format!("{:.0}%", v * 100.0);
    match row {
        Row::Jumpscares => on_off(true),
        Row::Sensitivity => format!("{:.1}x", s.sensitivity),
        Row::Volume => percent(s.volume),
        Row::Music => percent(s.music),
        Row::Vhs => percent(s.vhs),
        Row::Fov => format!("{:.0}°", s.fov),
        Row::Fullscreen => on_off(s.fullscreen),
        Row::FpsLimit if s.fps_limit == 0 => "frei".into(),
        Row::FpsLimit => s.fps_limit.to_string(),
        Row::LowLatency => on_off(s.low_latency),
        Row::Tabs | Row::Tutorial | Row::Back => String::new(),
    }
}

// --- Anordnung (480x270) ---

const LEFT: f32 = 60.0;
const WIDTH: f32 = 360.0;
const TABS_Y: f32 = 50.0;
const TAB_H: f32 = 18.0;
const TAB_W: f32 = WIDTH / PAGES.len() as f32;
const ROWS_Y: f32 = 82.0;
const STEP: f32 = 17.0;
const BAR_X: f32 = 268.0;
const BAR_W: f32 = 72.0;
const VALUE_X: f32 = 358.0;
const HINT_Y: f32 = 222.0;

/// Mitte (x), Oberkante (y) und Hoehe einer Zeile; Zeile 0 sind die Reiter.
fn row_top(i: usize) -> f32 {
    match i {
        0 => TABS_Y,
        i => ROWS_Y + (i - 1) as f32 * STEP,
    }
}

/// Welche Zeile liegt unter dem Mauszeiger?
pub fn hit(n: usize, x: f32, y: f32) -> Option<usize> {
    if !(LEFT..LEFT + WIDTH).contains(&x) {
        return None;
    }
    (0..n).find(|&i| {
        let h = if i == 0 { TAB_H } else { STEP };
        let top = row_top(i) - if i == 0 { 0.0 } else { 4.0 };
        (top..top + h).contains(&y)
    })
}

/// Reiter unter dem Mauszeiger.
pub fn tab_at(x: f32) -> Option<Page> {
    let i = ((x - LEFT) / TAB_W).floor();
    (i >= 0.0).then(|| PAGES.get(i as usize).copied()).flatten()
}

/// Klick auf einen Schieberegler: gewuenschter Wert.
pub fn slider_value(row: Row, x: f32) -> Option<f32> {
    let (min, max) = row.range()?;
    let k = (x - BAR_X) / BAR_W;
    (-0.06..=1.06)
        .contains(&k)
        .then(|| min + (max - min) * k.clamp(0.0, 1.0))
}

fn draw_tabs(u: &mut Ui, page: Page, selected: bool, time: f32) {
    for (i, p) in PAGES.iter().enumerate() {
        let x = LEFT + i as f32 * TAB_W;
        let active = *p == page;
        let back = if active {
            [0.16, 0.16, 0.15, 0.95]
        } else {
            [0.04, 0.04, 0.05, 0.8]
        };
        u.rect(x + 1.0, TABS_Y, TAB_W - 2.0, TAB_H, back);
        if active {
            let glow = if selected {
                0.75 + 0.25 * (time * 4.0).sin()
            } else {
                0.6
            };
            let [r, g, b, _] = YELLOW;
            u.rect(
                x + 1.0,
                TABS_Y + TAB_H - 2.0,
                TAB_W - 2.0,
                2.0,
                [r, g, b, glow],
            );
        }
        let t = p.title();
        let tx = x + (TAB_W - Ui::text_width(t, 1.0)) * 0.5;
        let color = match (active, selected) {
            (true, true) => YELLOW,
            (true, false) => WHITE,
            _ => DIM,
        };
        u.text_s(tx, TABS_Y + 5.0, t, color, 1.0);
    }
}

fn draw_slider(u: &mut Ui, y: f32, k: f32, color: ui::Color) {
    u.rect(BAR_X, y + 3.0, BAR_W, 2.0, [1.0, 1.0, 1.0, 0.22]);
    u.rect(BAR_X, y + 3.0, BAR_W * k, 2.0, color);
    u.rect(BAR_X + BAR_W * k - 1.5, y, 3.0, 8.0, color);
}

pub fn draw(u: &mut Ui, sel: usize, time: f32, s: &Settings, from_pause: bool, page: Page) {
    let shade = if from_pause { 0.7 } else { 0.5 };
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, shade]);
    u.text_center(22.0, "OPTIONEN", WHITE, 2.0);
    u.rect(
        LEFT,
        TABS_Y + TAB_H,
        WIDTH,
        HINT_Y - 4.0 - TABS_Y - TAB_H,
        [0.03, 0.03, 0.04, 0.8],
    );
    draw_tabs(u, page, sel == 0, time);
    let rows = rows(page, s.tutorial_done);
    for (i, row) in rows.iter().enumerate().skip(1) {
        let y = row_top(i);
        let on = i == sel;
        let color = if on { YELLOW } else { WHITE };
        if on {
            let pulse = 0.12 + 0.05 * (time * 4.0).sin();
            u.rect(
                LEFT + 4.0,
                y - 4.0,
                WIDTH - 8.0,
                16.0,
                [0.95, 0.95, 0.92, pulse],
            );
            u.text_s(LEFT + 8.0, y, "▶", YELLOW, 1.0);
        }
        u.text_s(LEFT + 24.0, y, row.name(), color, 1.0);
        if let Some((min, max)) = row.range() {
            let v = s.number(*row);
            draw_slider(u, y, ((v - min) / (max - min)).clamp(0.0, 1.0), color);
        }
        let v = value(*row, s);
        if !v.is_empty() {
            let shown = if row.range().is_none() && on {
                format!("< {v} >")
            } else {
                v
            };
            // rechtsbuendig
            let x = LEFT + WIDTH - 12.0 - Ui::text_width(&shown, 1.0);
            u.text_s(x.max(BAR_X.min(VALUE_X)), y, &shown, color, 1.0);
        }
    }
    if let Some(row) = rows.get(sel) {
        u.text_center(HINT_Y - 18.0, row.help(), DIM, 1.0);
    }
    u.text_center(
        HINT_Y + 4.0,
        "Tab: nächster Reiter - Links/Rechts: ändern - Esc: zurück",
        DIM,
        1.0,
    );
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

    #[test]
    fn every_page_fits_and_ends_with_back() {
        let s = Settings::default();
        for page in PAGES {
            let rows = rows(page, true);
            assert_eq!(rows[0], Row::Tabs);
            assert_eq!(*rows.last().unwrap(), Row::Back);
            assert!(
                row_top(rows.len() - 1) + 8.0 <= HINT_Y - 20.0,
                "{page:?} zu lang"
            );
            for r in rows {
                assert!(LEFT + 24.0 + Ui::text_width(r.name(), 1.0) <= BAR_X);
                assert!(Ui::text_width(r.help(), 1.0) <= ui::W);
                let v = format!("< {} >", value(r, &s));
                assert!(BAR_X + Ui::text_width(&v, 1.0) <= LEFT + WIDTH);
            }
        }
    }

    #[test]
    fn tabs_are_side_by_side_and_wrap_around() {
        assert_eq!(
            PAGES.map(|p| tab_at(
                LEFT + 10.0 + TAB_W * PAGES.iter().position(|q| *q == p).unwrap() as f32
            )),
            PAGES.map(Some)
        );
        assert_eq!(tab_at(LEFT - 5.0), None);
        assert_eq!(tab_at(LEFT + WIDTH + 5.0), None);
        assert_eq!(Page::Display.next(-1), Page::Controls);
        assert_eq!(Page::Controls.next(1), Page::Display);
        assert_eq!(hit(4, LEFT + 5.0, TABS_Y + 2.0), Some(0));
        assert_eq!(hit(4, LEFT + 5.0, ROWS_Y), Some(1));
        assert_eq!(hit(4, LEFT + 5.0, HINT_Y), None);
    }

    #[test]
    fn sliders_reach_both_ends() {
        assert_eq!(slider_value(Row::Volume, BAR_X - 2.0), Some(0.0));
        assert_eq!(slider_value(Row::Volume, BAR_X + BAR_W + 2.0), Some(1.0));
        assert_eq!(slider_value(Row::Volume, BAR_X - 40.0), None);
        assert_eq!(slider_value(Row::Fullscreen, BAR_X + 5.0), None);
    }

    #[test]
    fn the_tutorial_row_appears_only_after_finishing_it() {
        assert!(!rows(Page::Gameplay, false).contains(&Row::Tutorial));
        assert!(rows(Page::Gameplay, true).contains(&Row::Tutorial));
    }
}
