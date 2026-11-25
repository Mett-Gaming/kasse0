//! HUD, Menues und End-Screens (alles im 480x270-Raster, unter dem VHS-Effekt).

mod belt_window;
pub mod keys;
mod kiosk;
pub mod options;
mod pfand;
mod till;

use crate::game::dialog::CART_OPENER;
use crate::game::tutorial::{Phase, Tutorial, PHASES as TUTORIAL_PHASES, STEPS as TUTORIAL_STEPS};
use crate::game::{Focus, Game, Outcome, PickupKind};
use crate::settings::Settings;
use crate::ui::{self, euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

/// Text mit Tastenhinweisen: zeigt die Tasten, die gerade gelten (controls.rs).
pub(crate) trait KeyHints {
    fn hint(&mut self, x: f32, y: f32, s: &str, color: ui::Color, scale: f32);
    fn hint_s(&mut self, x: f32, y: f32, s: &str, color: ui::Color, scale: f32);
    fn hint_center(&mut self, y: f32, s: &str, color: ui::Color, scale: f32);
}

impl KeyHints for Ui {
    fn hint(&mut self, x: f32, y: f32, s: &str, color: ui::Color, scale: f32) {
        self.text(x, y, &crate::controls::hint(s), color, scale);
    }
    fn hint_s(&mut self, x: f32, y: f32, s: &str, color: ui::Color, scale: f32) {
        self.text_s(x, y, &crate::controls::hint(s), color, scale);
    }
    fn hint_center(&mut self, y: f32, s: &str, color: ui::Color, scale: f32) {
        self.text_center(y, &crate::controls::hint(s), color, scale);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Options {
        from_pause: bool,
        /// Zeile "Tutorial spielen" (erst, wenn man das Tutorial einmal geschafft hat)
        tutorial: bool,
        page: options::Page,
    },
    /// Tasten belegen (aus den Optionen)
    Keys {
        from_pause: bool,
    },
    Playing,
    Paused,
    End,
    /// Cheat-Menue (F1): erst die Kategorien (`group: None`), dann die Cheats einer Kategorie
    Cheats {
        group: Option<usize>,
    },
    /// Beim allerersten Einkauf: Tutorial spielen? (Ja/Nein)
    AskTutorial,
    /// Nach "Nein": "OK Champ ..." (OK startet den normalen Einkauf)
    Champ,
    /// Tutorial geschafft, auf dem Weg ins Hauptmenue: "Du bist jetzt schlau genug ..."
    TutorialDone,
}

pub const TITLE_ITEMS: &[&str] = &["Neuer Einkauf", "Optionen", "Beenden"];
pub const ASK_ITEMS: &[&str] = &["Ja", "Nein"];
pub const CHAMP_ITEMS: &[&str] = &["OK"];

/// Hauptmenue-Zeilen: Einkauf, Optionen, Beenden. (Das Tutorial wird beim ersten Einkauf
/// angeboten, siehe AskTutorial.)
pub fn title_rows() -> Vec<String> {
    TITLE_ITEMS.iter().map(|s| s.to_string()).collect()
}

/// Bildschirme ueber dem Kameraschwenk des Hauptmenues (kein laufendes Spiel).
pub fn on_title(screen: Screen) -> bool {
    matches!(screen, Screen::Title | Screen::AskTutorial | Screen::Champ)
}
pub const PAUSE_ITEMS: &[&str] = &[
    "Weiter",
    "Optionen",
    "Neu starten (gleicher Seed)",
    "Hauptmenü",
    "Beenden",
];
pub const END_ITEMS: &[&str] = &["Nochmal (gleicher Seed)", "Neuer Einkauf", "Hauptmenü"];
/// Erste Menuezeile (y) und Zeilenabstand fuer Zeichnen und Maus-Treffer.
pub fn menu_layout(screen: Screen) -> (f32, f32) {
    match screen {
        Screen::Title => (150.0, 18.0),
        Screen::Paused => (66.0, 16.0),
        Screen::End => (205.0, 16.0),
        Screen::Cheats { group: None } => (62.0, 18.0),
        Screen::Cheats { group: Some(_) } => (56.0, 15.0),
        Screen::AskTutorial => (150.0, 18.0),
        Screen::Champ | Screen::TutorialDone => (158.0, 18.0),
        // Optionen ordnet hud/options.rs selbst an
        Screen::Playing | Screen::Options { .. } | Screen::Keys { .. } => (0.0, 0.0),
    }
}

pub fn menu_len(screen: Screen) -> usize {
    match screen {
        Screen::Title => TITLE_ITEMS.len(),
        Screen::Options { tutorial, page, .. } => options::rows(page, tutorial).len(),
        Screen::Keys { .. } => keys::count(),
        Screen::Paused => PAUSE_ITEMS.len(),
        Screen::End => END_ITEMS.len(),
        Screen::Cheats { group: None } => crate::game::cheat_groups().len() + 1,
        Screen::Cheats { group: Some(g) } => {
            crate::game::cheat_groups().get(g).map_or(0, |c| c.2.len()) + 1
        }
        Screen::AskTutorial => ASK_ITEMS.len(),
        Screen::Champ | Screen::TutorialDone => CHAMP_ITEMS.len(),
        Screen::Playing => 0,
    }
}

/// Welche Menuezeile liegt unter dem Mauszeiger?
/// Mitte (x) und Oberkante (y) des Menuepunkts `i`, dazu die Spaltenbreite.
pub fn menu_cell(screen: Screen, i: usize) -> (f32, f32, f32) {
    let (y0, step) = menu_layout(screen);
    if let Some(rows) = two_column_rows(screen) {
        // lange Cheat-Kategorie: zwei Spalten
        let half = ui::W * 0.5;
        let col = i / rows;
        return (
            half * (col as f32 + 0.5),
            y0 + (i % rows) as f32 * step,
            half,
        );
    }
    (ui::W * 0.5, y0 + i as f32 * step, 360.0)
}

/// Zeilen pro Spalte, wenn ein Menue zweispaltig ist (lange Cheat-Kategorien), sonst None.
pub fn two_column_rows(screen: Screen) -> Option<usize> {
    let n = menu_len(screen);
    (matches!(screen, Screen::Cheats { group: Some(_) }) && n > 12).then(|| n.div_ceil(2))
}

pub fn menu_hit(screen: Screen, x: f32, y: f32) -> Option<usize> {
    if matches!(screen, Screen::Options { .. }) {
        return options::hit(menu_len(screen), x, y);
    }
    if matches!(screen, Screen::Keys { .. }) {
        return keys::hit(x, y);
    }
    let (_, step) = menu_layout(screen);
    (0..menu_len(screen)).find(|&i| {
        let (cx, top, w) = menu_cell(screen, i);
        let top = top - 4.0;
        (x - cx).abs() <= w * 0.5 && y >= top && y < top + step
    })
}

fn draw_menu(u: &mut Ui, screen: Screen, items: &[String], sel: usize, time: f32) {
    for (i, label) in items.iter().enumerate() {
        let (cx, y, _) = menu_cell(screen, i);
        let shown = if i == sel {
            format!("▶ {label}")
        } else {
            label.clone()
        };
        let x = cx - Ui::text_width(&shown, 1.0) * 0.5;
        if i == sel {
            let w = Ui::text_width(label, 1.0) + 24.0;
            u.rect(
                cx - w * 0.5,
                y - 4.0,
                w,
                16.0,
                [0.95, 0.95, 0.92, 0.12 + 0.05 * (time * 4.0).sin()],
            );
            u.hint_s(x, y, &shown, YELLOW, 1.0);
        } else {
            u.hint_s(x, y, &shown, WHITE, 1.0);
        }
    }
}

fn osd(u: &mut Ui, time: f32, label: &str) {
    if (time * 1.5).fract() < 0.7 {
        u.hint_s(12.0, 10.0, &format!("▶ {label}"), WHITE, 1.0);
    }
}

fn timestamp(secs: f32) -> String {
    let s = secs as u32;
    format!("SP {}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

pub fn draw_title(u: &mut Ui, sel: usize, time: f32, seed_hint: u64) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.35]);
    osd(u, time, "PLAY");
    u.hint_center(52.0, "KASSE 0", WHITE, 4.0);
    u.hint_center(92.0, "ein ganz normaler Einkauf", DIM, 1.0);
    draw_menu(u, Screen::Title, &title_rows(), sel, time);
    u.hint_s(12.0, 252.0, &format!("Band #{seed_hint}"), DIM, 1.0);
    u.hint_s(
        ui::W - 12.0 - Ui::text_width(&timestamp(time), 1.0),
        252.0,
        &timestamp(time),
        WHITE,
        1.0,
    );
}

/// Erster Einkauf: Tutorial ja/nein, bzw. danach die Antwort auf "Nein".
pub fn draw_ask_tutorial(u: &mut Ui, screen: Screen, sel: usize, time: f32) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.55]);
    osd(u, time, "PAUSE");
    let (x, y, w, h) = (60.0, 70.0, 360.0, 120.0);
    u.rect(x, y, w, h, [0.03, 0.03, 0.04, 0.95]);
    u.rect(x, y, w, 1.0, [0.95, 0.85, 0.3, 0.7]);
    if screen == Screen::TutorialDone {
        u.hint_center(y + 12.0, "TUTORIAL GESCHAFFT", DIM, 1.0);
        u.hint_center(y + 28.0, "Du bist jetzt schlau genug,", WHITE, 1.0);
        u.hint_center(y + 40.0, "um einkaufen zu können.", WHITE, 1.0);
        u.hint_center(y + 56.0, "WOW.", YELLOW, 2.0);
        let items: Vec<String> = CHAMP_ITEMS.iter().map(|s| s.to_string()).collect();
        draw_menu(u, screen, &items, sel, time);
    } else if screen == Screen::Champ {
        u.hint_center(y + 18.0, "OK Champ,", YELLOW, 2.0);
        u.hint_center(
            y + 48.0,
            "hab nicht gedacht, dass du so schlau bist.",
            WHITE,
            1.0,
        );
        let items: Vec<String> = CHAMP_ITEMS.iter().map(|s| s.to_string()).collect();
        draw_menu(u, screen, &items, sel, time);
    } else {
        u.hint_center(y + 14.0, "DEIN ERSTER EINKAUF", YELLOW, 1.0);
        u.hint_center(y + 34.0, "Willst du das Tutorial spielen?", WHITE, 1.0);
        u.hint_center(
            y + 48.0,
            "Mit Pfeilen am Boden. Verlieren geht nicht.",
            DIM,
            1.0,
        );
        let items: Vec<String> = ASK_ITEMS.iter().map(|s| s.to_string()).collect();
        draw_menu(u, screen, &items, sel, time);
    }
}

pub fn draw_pause(u: &mut Ui, sel: usize, time: f32, g: &Game) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.65]);
    osd(u, time, "PAUSE");
    u.hint_center(28.0, "PAUSE", WHITE, 3.0);
    let items: Vec<String> = PAUSE_ITEMS.iter().map(|s| s.to_string()).collect();
    draw_menu(u, Screen::Paused, &items, sel, time);
    draw_list(u, g, 16.0, 154.0, false);
    draw_controls(u);
}

fn draw_list(u: &mut Ui, g: &Game, x: f32, y: f32, panel: bool) {
    let lines = g.list.len() as f32 + 3.0 + (!g.pfand.bons.is_empty()) as u8 as f32;
    if panel {
        u.rect(
            x - 6.0,
            y - 6.0,
            176.0,
            lines * 11.0 + 8.0,
            [0.92, 0.9, 0.82, 0.88],
        );
    }
    let (ink, done_c, head) = if panel {
        (
            [0.12, 0.12, 0.15, 1.0],
            [0.15, 0.45, 0.2, 1.0],
            [0.1, 0.1, 0.12, 1.0],
        )
    } else {
        (WHITE, GREEN, YELLOW)
    };
    let text = |u: &mut Ui, x: f32, y: f32, s: &str, c| {
        if panel {
            u.hint(x, y, s, c, 1.0)
        } else {
            u.hint_s(x, y, s, c, 1.0)
        }
    };
    text(u, x, y, "EINKAUFSZETTEL", head);
    for (i, &item) in g.list.iter().enumerate() {
        let done = g.list_done(item);
        let mark = if done { "[x]" } else { "[ ]" };
        let name = &crate::catalog::catalog().list[item].name;
        text(
            u,
            x,
            y + 12.0 + i as f32 * 11.0,
            &format!("{mark} {name}"),
            if done { done_c } else { ink },
        );
    }
    let yv = y + 16.0 + g.list.len() as f32 * 11.0;
    text(
        u,
        x,
        yv,
        &format!("Im Wagen: {}", euro(g.cart_value())),
        ink,
    );
    if !g.pfand.bons.is_empty() {
        // die zwei hoechsten, mehr passt nicht
        let mut bons = g.pfand.bons.clone();
        bons.sort_unstable_by(|a, b| b.cmp(a));
        let shown: Vec<String> = bons
            .iter()
            .take(2)
            .map(|b| format!("{},{:02}", b / 100, b % 100))
            .collect();
        let more = if bons.len() > 2 { " ..." } else { "" };
        let s = format!("Bons: {}{more}", shown.join(" "));
        text(u, x, yv + 11.0, &s, ink);
    }
}

pub fn draw_hud(u: &mut Ui, g: &Game, show_list: bool, time: f32) {
    // nach dem Ende (Tod oder Gewinn) kein HUD mehr, nur das Bild
    if g.outcome != Outcome::Running {
        return;
    }
    osd(u, time, "PLAY");
    let ts = timestamp(g.run_time);
    u.hint_s(
        ui::W - 12.0 - Ui::text_width(&ts, 1.0),
        252.0,
        &ts,
        WHITE,
        1.0,
    );

    let (show_sus, show_stamina) = meters_visible(g);
    if show_sus {
        suspicion_meter(u, g.suspicion, time);
    }
    if show_stamina {
        stamina_meter(u, g.player.stamina, g.player.exhausted);
    }

    // Dauerhaft Wirkendes vom Kiosk
    if let Some(s) = kept_line(g) {
        u.hint_s(ui::W - 12.0 - Ui::text_width(&s, 1.0), 226.0, &s, DIM, 1.0);
    }
    // Tasche: was G benutzen wuerde
    if let Some(i) = g.pfand.owned.selected() {
        let n = g.pfand.owned.pocket.len();
        let more = if n > 1 {
            format!(" ({n})")
        } else {
            String::new()
        };
        let s = format!("[G] {}{more}", i.name());
        u.hint_s(
            ui::W - 12.0 - Ui::text_width(&s, 1.0),
            238.0,
            &s,
            WHITE,
            1.0,
        );
    }

    // Fadenkreuz
    u.rect(
        ui::W * 0.5 - 1.0,
        ui::H * 0.5 - 1.0,
        2.0,
        2.0,
        [1.0, 1.0, 1.0, 0.6],
    );

    // Interaktion
    let y = ui::H * 0.5 + 14.0;
    if let Some(item) = &g.held {
        u.hint_center(
            y,
            &format!("In der Hand: {}  {}", item.name, euro(item.cents)),
            WHITE,
            1.0,
        );
        let deposit = matches!(item.kind, PickupKind::Deposit(_));
        let bag = format!(
            "[E] In die Tüte ({}/{})",
            g.pfand.bag.len(),
            crate::game::pfand::BAG_MAX
        );
        let action = if matches!(g.focus, Some(Focus::Machine(_))) {
            "[E] Pfandautomat"
        } else if deposit && g.aim_cart {
            &bag
        } else if g.focus == Some(Focus::Belt) {
            "[E] Kassenfenster öffnen"
        } else if g.aim_cart {
            "[E] In den Wagen legen"
        } else {
            "[E] Fallen lassen  (auf den Wagen zielen)"
        };
        u.hint_center(y + 11.0, action, YELLOW, 1.0);
    } else {
        match g.focus {
            Some(Focus::Bottle(i)) => {
                if let Some(b) = g.pfand.bottles.get(i) {
                    let s = format!("[E] {}   Pfand {}", b.kind.name(), euro(b.kind.cents()));
                    u.hint_center(y, &s, WHITE, 1.0);
                }
            }
            Some(Focus::Tray) => {
                if let Some(i) = g.pfand.tray {
                    u.hint_center(y, &format!("[E] {} nehmen", i.name()), YELLOW, 1.0);
                }
            }
            Some(Focus::Kiosk) => {
                let s = if g.kiosk_has_open() {
                    "[E] Kiosk"
                } else {
                    "Kiosk - gleich geöffnet"
                };
                u.hint_center(y, s, YELLOW, 1.0);
            }
            Some(Focus::Machine(i)) => {
                if g.machine_works(i) {
                    u.hint_center(y, "[E] Pfandautomat", YELLOW, 1.0);
                } else {
                    u.hint_center(y, "AUSSER BETRIEB", RED, 1.0);
                }
            }
            Some(Focus::Door(_)) => {
                u.hint_center(y, "[E] Tür aufdrücken", YELLOW, 1.0);
            }
            Some(Focus::Shelf(i)) => {
                let p = crate::game::ItemData::from_shelf(&g.world.items[i]);
                u.hint_center(
                    y,
                    &format!("[E] {}   {}", p.name, euro(p.cents)),
                    WHITE,
                    1.0,
                );
            }
            Some(Focus::Pickup(i)) => {
                let p = &g.pickups[i].data;
                let label = format!("[E] {}   {}", p.name, euro(p.cents));
                let c = if matches!(p.kind, PickupKind::Tomato { .. }) {
                    RED
                } else {
                    WHITE
                };
                u.hint_center(y, &label, c, 1.0);
            }
            Some(Focus::Loose(i)) => {
                u.hint_center(
                    y,
                    &format!("[E] Aufheben: {}", g.loose[i].data.name),
                    WHITE,
                    1.0,
                );
            }
            Some(Focus::Belt) => match g.belt_pick() {
                Some(_) if g.cart_at_register() => {
                    let s = format!("[E] Kassenfenster öffnen ({} im Wagen)", g.cart.len());
                    u.hint_center(y, &s, YELLOW, 1.0);
                    u.hint_center(
                        y + 11.0,
                        "Dort ziehst du die Ware mit der Maus aufs Band.",
                        DIM,
                        1.0,
                    );
                }
                Some(_) => u.hint_center(y, "Kassenband - bring deinen Wagen her", YELLOW, 1.0),
                None => u.hint_center(y, "Kassenband", YELLOW, 1.0),
            },
            Some(Focus::Terminal) => {
                let asking = g
                    .till
                    .as_ref()
                    .is_some_and(|t| t.asked.is_some() || t.questions_left());
                let s = if g.checking_out() {
                    "Die Kassiererin scannt noch ...".to_string()
                } else if g.scanned.is_empty() {
                    "Kartenterminal - erst aufs Band legen".to_string()
                } else if g.scanned_complete() && asking {
                    "Die Kassiererin will noch etwas wissen ...".to_string()
                } else if g.scanned_complete() {
                    format!("[E] Bezahlen   {}", euro(g.scanned_value()))
                } else {
                    "Zettel nicht vollständig gescannt".to_string()
                };
                u.hint_center(y, &s, YELLOW, 1.0);
            }
            _ if g.at_register && g.checking_out() => {
                let s = format!("Die Kassiererin scannt ... ({} auf dem Band)", g.belt.len());
                u.hint_center(y, &s, YELLOW, 1.0);
            }
            Some(Focus::Employee(_)) => {
                u.hint_center(y, "[E] \"Hey, das ist mein Wagen!\"", YELLOW, 1.0);
            }
            Some(Focus::Cart) => {
                let s = if g.trolley.pushing {
                    "[E/Q] Wagen stehen lassen"
                } else {
                    "[E/Q] Wagen schieben"
                };
                u.hint_center(y, s, WHITE, 1.0);
                let n = g.pfand.bag.len();
                if n > 0 && (g.pfand.aim_bag || g.cart.is_empty()) {
                    let s = format!(
                        "[F] Flasche aus der Tüte   ({n}/{})",
                        crate::game::pfand::BAG_MAX
                    );
                    u.hint_center(y + 11.0, &s, DIM, 1.0);
                } else if let Some(i) = g.cart_selected() {
                    u.hint_center(
                        y + 11.0,
                        &format!(
                            "[F] Herausnehmen: {}   ({}/{}, Mausrad oder draufzielen)",
                            g.cart[i].name,
                            i + 1,
                            g.cart.len()
                        ),
                        DIM,
                        1.0,
                    );
                }
            }
            None => {}
        }
    }

    // Tutorial-Modus: erst die Tasten-Liste, dann ein Kasten je Phase
    if let (Some(t), Some(phase)) = (&g.tutorial, g.tutorial_phase()) {
        if phase == Phase::Controls {
            draw_tutorial(u, t);
        } else if !g.pointer_window_open() && !g.till_paying() && g.dialog.is_none() {
            draw_tutorial_phase(u, phase, g.run_time);
        }
    }

    // Einkaufszettel (TAB) bzw. kurzer Hinweis
    if show_list {
        draw_list(u, g, 18.0, 34.0, true);
    } else if g.run_time < 20.0 && g.tutorial.is_none() {
        u.hint_s(12.0, 24.0, "TAB: Einkaufszettel", DIM, 1.0);
    }

    // Durchsage
    if let Some((text, _)) = &g.announcement {
        u.hint_center(26.0, "~ DURCHSAGE ~", YELLOW, 1.0);
        for (k, line) in Ui::wrap(text, 52).iter().enumerate() {
            u.hint_center(38.0 + k as f32 * 10.0, line, WHITE, 1.0);
        }
    }

    // Meldung
    if let Some((text, _)) = &g.message {
        for (k, line) in Ui::wrap(text, 52).iter().enumerate() {
            u.hint_center(196.0 + k as f32 * 10.0, line, WHITE, 1.0);
        }
    }

    // Dialog mit einem Mitarbeiter
    if let Some(d) = &g.dialog {
        let q = d.question();
        u.rect(40.0, 160.0, 400.0, 86.0, [0.02, 0.02, 0.03, 0.85]);
        u.rect(40.0, 160.0, 400.0, 1.0, [0.9, 0.9, 0.85, 0.6]);
        if d.cart_talk {
            u.hint(48.0, 152.0, &format!("DU: {CART_OPENER}"), WHITE, 1.0);
        }
        u.hint(48.0, 166.0, "MITARBEITER:", DIM, 1.0);
        match d.reply {
            Some((reply, _)) => u.hint(48.0, 180.0, reply, WHITE, 1.0),
            None => {
                u.hint(48.0, 180.0, q.text, WHITE, 1.0);
                for k in 0..3 {
                    let a = q.answers[d.order[k]];
                    u.hint(
                        56.0,
                        198.0 + k as f32 * 12.0,
                        &format!("{}) {}", k + 1, a),
                        YELLOW,
                        1.0,
                    );
                }
                let frac = (d.timer / crate::game::dialog::ANSWER_TIME).clamp(0.0, 1.0);
                u.rect(48.0, 238.0, 384.0 * frac, 2.0, [0.9, 0.3, 0.2, 0.8]);
            }
        }
    }

    // die nutzlose Zigarette (1/2/3)

    // Kassenfenster (Waren mit der Maus aufs Band)
    belt_window::draw(u, g, time);
    // Pfandautomat
    if let Some(w) = &g.pfand.window {
        pfand::draw(u, g, w, time);
    }
    if let Some(w) = &g.pfand.kiosk {
        kiosk::draw(u, g, w, time);
    }

    // Kassen-Minispiel: Misstrauen, Leben, Fragen, Karte, PIN, Geldboerse
    till::draw(u, g, time);

    // Tutorial: Erklaerleiste oben links ueber Kassenfenster, Karte/PIN und Bargeld
    if let Some(hints) = crate::game::tutorial::ui_hints(g) {
        draw_tutorial_bar(u, hints, time);
    }

    // Flucht nach dem Bezahlen; im Korridor: nicht umdrehen
    let warn = g.turn_warning();
    if warn > 0.0 {
        // je weiter man sich dreht, desto roter und schneller
        u.rect(0.0, 0.0, ui::W, ui::H, [0.6, 0.0, 0.0, 0.25 * warn]);
        if (time * (3.0 + 6.0 * warn)).fract() < 0.65 {
            u.hint_center(110.0, "NICHT UMDREHEN", RED, 3.0);
        }
    } else if g.chasing() && g.in_corridor() {
        // die Front holt auf: SCHNELLER (blinkt schneller, je naeher)
        let near = g.front_warning();
        if near > 0.0 && (time * (2.0 + 6.0 * near)).fract() < 0.6 {
            u.hint_center(64.0, "SCHNELLER", RED, 2.0);
        } else if near <= 0.0 && (time * 1.5).fract() < 0.7 {
            u.hint_center(64.0, "NICHT UMDREHEN. LAUF.", RED, 1.0);
        }
    } else if g.chasing() && (time * 2.5).fract() < 0.6 {
        u.hint_center(64.0, "ALARM - LAUF ZUM AUSGANG!", RED, 2.0);
    }
}

/// Tutorial ab Phase 2: kurzer Kasten oben links der Mitte (unter der Durchsage, neben dem
/// Kassendisplay rechts oben).
fn draw_tutorial_phase(u: &mut Ui, phase: Phase, time: f32) {
    let (page, lines) = phase.page(time);
    let pages = phase.pages().len();
    let (x, y, w) = (46.0, 66.0, 300.0);
    let h = 20.0 + lines.len() as f32 * 10.0;
    u.rect(x, y, w, h, [0.02, 0.02, 0.03, 0.78]);
    u.rect(x, y, w, 1.0, [0.95, 0.85, 0.3, 0.7]);
    let title = format!(
        "TUTORIAL {}/{}: {}",
        phase.number(),
        TUTORIAL_PHASES,
        phase.title()
    );
    u.hint(x + 6.0, y + 5.0, &title, YELLOW, 1.0);
    // mehrere Seiten (z. B. Einkaufen, Kunden, Mitarbeiter): Seitenzahl rechts oben
    if pages > 1 {
        let n = format!("{}/{}", page + 1, pages);
        u.hint(x + w - 6.0 - Ui::text_width(&n, 1.0), y + 5.0, &n, DIM, 1.0);
    }
    for (k, l) in lines.iter().enumerate() {
        u.hint(x + 6.0, y + 17.0 + k as f32 * 10.0, l, WHITE, 1.0);
    }
}

/// Schmale Erklaerleiste ganz oben links (ueber den Kassen-Fenstern, die erst bei y = 14
/// beginnen): ein Hinweis nach dem anderen, Seitenzahl rechts.
fn draw_tutorial_bar(u: &mut Ui, hints: &[&str], time: f32) {
    use crate::game::tutorial::HINT_TIME;
    if hints.is_empty() {
        return;
    }
    let k = (time / HINT_TIME) as usize % hints.len();
    let n = format!("{}/{}", k + 1, hints.len());
    let w = 72.0 + Ui::text_width(hints[k], 1.0) + 8.0 + Ui::text_width(&n, 1.0) + 6.0;
    // mittig, klein auf der Oberkante des Fensters (ganz oben am Bildrand schneidet der
    // VHS-Effekt ab); gelber Rahmen, damit es sich vom Fenster abhebt
    let (x, y, h) = ((ui::W - w) * 0.5, TUTORIAL_BAR_Y, 14.0);
    u.rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, [0.95, 0.85, 0.3, 0.9]);
    u.rect(x, y, w, h, [0.02, 0.02, 0.03, 0.97]);
    u.hint(x + 4.0, y + 3.0, "TUTORIAL", YELLOW, 1.0);
    u.hint(x + 72.0, y + 3.0, hints[k], WHITE, 1.0);
    u.hint(x + w - 6.0 - Ui::text_width(&n, 1.0), y + 3.0, &n, DIM, 1.0);
}

/// Hoehe der Tutorial-Leiste ueber den Kassen-Fenstern (Kassenfenster ab y = 14,
/// Kartenleser ab 20, Geldboerse ab 28)
pub const TUTORIAL_BAR_Y: f32 = 16.0;

/// Alle Tasten auf einen Blick; erledigte abgehakt, aktuelle hervorgehoben.
fn draw_tutorial(u: &mut Ui, t: &Tutorial) {
    let x = 166.0;
    let y = 14.0;
    let n = TUTORIAL_STEPS.len() as f32;
    u.rect(
        x - 6.0,
        y - 6.0,
        316.0,
        18.0 + n * 11.0,
        [0.02, 0.02, 0.03, 0.75],
    );
    u.hint(x, y, "STEUERUNG", YELLOW, 1.0);
    for (i, (key, what)) in TUTORIAL_STEPS.iter().enumerate() {
        let yy = y + 14.0 + i as f32 * 11.0;
        let (mark, c) = match i.cmp(&t.step) {
            std::cmp::Ordering::Less => ("x", GREEN),
            std::cmp::Ordering::Equal => (">", YELLOW),
            std::cmp::Ordering::Greater => (" ", DIM),
        };
        let line = format!("{mark} {key:<11} {what}");
        u.hint(x, yy, &line, c, 1.0);
    }
}

/// Tastenbelegung (Pause-Menue)
/// Steuerung im Pausenmenue: zwei Spalten mit Ueberschrift, Tasten als kleine Tastenkappen.
pub const CONTROLS_MOVE: &[(&str, &str)] = &[
    ("WASD", "Gehen"),
    ("Maus", "Umsehen"),
    ("Shift", "Rennen"),
    ("Leer", "Springen"),
    ("C/Strg", "Ducken"),
    ("Esc", "Pause"),
    ("G", "Item"),
];
pub const CONTROLS_SHOP: &[(&str, &str)] = &[
    ("E", "Benutzen"),
    ("Q", "Wagen"),
    ("F", "Ware raus"),
    ("Tab", "Zettel"),
    ("Rad", "Auswählen"),
    ("1 2 3", "Antworten"),
    ("R", "Reklamieren"),
    ("0-9", "PIN"),
];
/// Kasten der Steuerung (x, y, Breite, Hoehe) und Spalten (x, Breite der Tastenkappen)
const CONTROLS_BOX: (f32, f32, f32, f32) = (184.0, 144.0, 290.0, 122.0);
const CONTROLS_COLS: [(f32, f32); 2] = [(190.0, 54.0), (330.0, 44.0)];
const CONTROLS_ROW: f32 = 11.5;

fn draw_controls(u: &mut Ui) {
    let (bx, by, bw, bh) = CONTROLS_BOX;
    u.rect(bx, by, bw, bh, [0.03, 0.03, 0.04, 0.85]);
    u.rect(bx, by, bw, 1.0, [0.95, 0.85, 0.3, 0.7]);
    u.hint(bx + 6.0, by + 4.0, "STEUERUNG", YELLOW, 1.0);
    let sections = [
        ("BEWEGEN", CONTROLS_MOVE),
        ("EINKAUF & KASSE", CONTROLS_SHOP),
    ];
    for ((title, list), (cx, cap)) in sections.iter().zip(CONTROLS_COLS) {
        u.hint(cx, by + 17.0, title, DIM, 1.0);
        for (k, (key, what)) in list.iter().enumerate() {
            let y = by + 30.0 + k as f32 * CONTROLS_ROW;
            // Tastenkappe: heller Rahmen, Taste mittig
            u.rect(cx, y - 2.0, cap, 11.0, [0.75, 0.75, 0.7, 1.0]);
            u.rect(cx + 1.0, y - 1.0, cap - 2.0, 9.0, [0.16, 0.16, 0.18, 1.0]);
            // lange Knopfnamen gekuerzt, damit sie in die Kappe passen
            let key: String = crate::controls::hint(key)
                .chars()
                .take((cap / 8.0) as usize)
                .collect();
            let kx = cx + (cap - Ui::text_width(&key, 1.0)) * 0.5;
            u.text(kx, y, &key, WHITE, 1.0);
            u.hint(cx + cap + 5.0, y, what, WHITE, 1.0);
        }
    }
}

/// Cheat-Menue (F1): Aktionen + "Zurück"; daneben der aktuelle Zustand.
/// Cheat-Menue: Kategorien mit kurzer Beschreibung, darin die einzelnen Cheats.
pub fn draw_cheats(u: &mut Ui, sel: usize, time: f32, g: &Game, group: Option<usize>) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.75]);
    let groups = crate::game::cheat_groups();
    let screen = Screen::Cheats { group };
    match group.and_then(|i| groups.get(i).map(|c| (i, c))) {
        None => {
            u.hint_center(14.0, "CHEATS (F1)", RED, 2.0);
            u.hint_center(
                36.0,
                "Ein Cheat sperrt die Erfolge für diesen Einkauf",
                DIM,
                1.0,
            );
            let mut items: Vec<String> = groups
                .iter()
                .map(|(name, _, list)| format!("{name}  ({})", list.len()))
                .collect();
            items.push("Zurück zum Spiel".into());
            draw_menu(u, screen, &items, sel, time);
            // Beschreibung der gewaehlten Kategorie
            if let Some((_, about, _)) = groups.get(sel) {
                let (_, y, _) = menu_cell(screen, groups.len() + 1);
                u.hint_center(y + 4.0, about, YELLOW, 1.0);
            }
        }
        Some((_, (name, _, list))) => {
            u.hint_center(14.0, &format!("CHEATS > {}", name.to_uppercase()), RED, 1.0);
            u.hint_center(30.0, "Enter: auslösen   Esc: zurück", DIM, 1.0);
            let mut items: Vec<String> = list
                .iter()
                .map(|&i| {
                    if i == 0 {
                        format!("Unverwundbar: {}", if g.god { "AN" } else { "AUS" })
                    } else if i == crate::game::STAMINA_CHEAT {
                        let on = if g.endless_stamina { "AN" } else { "AUS" };
                        format!("Unendlich Ausdauer: {on}")
                    } else {
                        crate::game::CHEATS[i].to_string()
                    }
                })
                .collect();
            items.push("< Zurück".into());
            draw_menu(u, screen, &items, sel, time);
        }
    }
    let state = format!(
        "Stufe: {}   Wagen: {}   Auffällig: {:.0} %   {}",
        g.tier.name(),
        euro(g.cart_value()),
        g.suspicion * 100.0,
        if g.tipped { "gekippt" } else { "normal" }
    );
    u.hint_center(252.0, &state, DIM, 1.0);
}

/// Gruselige Schriftzuege beim Tod (statt "DU BIST AUFGEFALLEN"), einer je Tod.
pub const DEATH_TITLES: &[&str] = &[
    "DU BLEIBST JETZT HIER",
    "WILLKOMMEN IM SORTIMENT",
    "SIE HABEN DICH",
    "DEIN GESICHT GEHÖRT UNS",
    "DER MARKT SCHLIESST NIE",
    "ZURÜCK INS REGAL",
    "MINDESTHALTBAR BIS: JETZT",
    "WIR SEHEN DICH",
    "KUNDE 0 IST AUSVERKAUFT",
    "DU WARST NIE HIER",
    "NIEMAND VERLÄSST KAUF NULL",
    "SIE HABEN DICH GEFUNDEN",
];

/// Welcher Schriftzug fuer diesen Tod: haengt an Seed und Zeitpunkt, bleibt also stehen,
/// solange der Endbildschirm zu sehen ist.
pub fn death_title(g: &Game) -> &'static str {
    let h = g
        .seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((g.run_time * 10.0) as u64)
        .wrapping_mul(0xBF58_476D_1CE4_E5B9);
    DEATH_TITLES[((h >> 33) % DEATH_TITLES.len() as u64) as usize]
}

pub fn draw_end(u: &mut Ui, g: &Game, sel: usize, time: f32) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.8]);
    match g.outcome {
        Outcome::Won => {
            osd(u, time, "STOP");
            // Kassenbon
            let x = 150.0;
            let lines = g.scanned.len().min(10);
            u.rect(
                x,
                18.0,
                180.0,
                64.0 + lines as f32 * 10.0 + 34.0,
                [0.94, 0.93, 0.88, 1.0],
            );
            let ink = [0.1, 0.1, 0.12, 1.0];
            u.hint(x + 46.0, 26.0, "KAUF NULL", ink, 1.0);
            u.hint(x + 10.0, 40.0, &format!("Band #{}", g.seed), ink, 1.0);
            for (i, item) in g.scanned.iter().take(lines).enumerate() {
                let name: String = item.name.chars().take(13).collect();
                let price = euro(item.cents);
                let y = 56.0 + i as f32 * 10.0;
                u.hint(x + 8.0, y, &name, ink, 1.0);
                u.hint(x + 172.0 - Ui::text_width(&price, 1.0), y, &price, ink, 1.0);
            }
            if g.scanned.len() > lines {
                u.hint(x + 8.0, 56.0 + lines as f32 * 10.0, "...", ink, 1.0);
            }
            let y = 62.0 + (lines as f32 + 1.0) * 10.0;
            let total = format!("SUMME {}", euro(g.scanned_value()));
            u.hint(x + 172.0 - Ui::text_width(&total, 1.0), y, &total, ink, 1.0);
            let t = g.run_time as u32;
            u.hint(
                x + 8.0,
                y + 12.0,
                &format!("Zeit {}:{:02}", t / 60, t % 60),
                ink,
                1.0,
            );
            // (das Lob nach dem Tutorial kommt erst auf dem Weg ins Hauptmenue)
            u.hint_center(y + 34.0, "Danke für Ihren Einkauf.", WHITE, 1.0);
        }
        _ => {
            osd(u, time, "STOP");
            // gruseliger Schriftzug (zufaellig je Tod), zittert und flackert ab und zu
            let title = death_title(g);
            let glitch = ((time * 7.3).sin() * (time * 3.1).cos()) > 0.82;
            let jx = if glitch {
                (time * 91.0).sin() * 3.0
            } else {
                0.0
            };
            let x = (ui::W - Ui::text_width(title, 2.0)) * 0.5 + jx;
            if glitch {
                // kurzer Bildriss: weisser Schatten versetzt dahinter
                u.hint(x + 2.0, 69.0, title, [1.0, 1.0, 1.0, 0.5], 2.0);
            }
            u.hint(x, 70.0, title, RED, 2.0);
            let why = if g.till_caught {
                "Die Kassiererin hat dich durchschaut."
            } else if g.turned_back {
                "Du hast dich umgedreht."
            } else if g.front_caught {
                "Sie haben dich eingeholt. Du warst zu langsam."
            } else if g.chase.is_some() {
                "Sie haben Kunde 0 geschnappt - kurz vor dem Ausgang."
            } else {
                "Sie haben dich erreicht."
            };
            u.hint_center(100.0, why, WHITE, 1.0);
            let t = g.run_time as u32;
            u.hint_center(
                124.0,
                &format!(
                    "Im Wagen: {}   Zeit {}:{:02}",
                    euro(g.cart_value()),
                    t / 60,
                    t % 60
                ),
                DIM,
                1.0,
            );
            u.hint_center(140.0, &format!("Stufe: {}", g.tier.name()), DIM, 1.0);
        }
    }
    let items: Vec<String> = END_ITEMS.iter().map(|s| s.to_string()).collect();
    draw_menu(u, Screen::End, &items, sel, time);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheat_menu_has_categories_that_fit_on_screen() {
        let g = Game::new(3);
        let groups = crate::game::cheat_groups();
        let top = Screen::Cheats { group: None };
        assert_eq!(menu_len(top), groups.len() + 1);
        let mut u = Ui::default();
        draw_cheats(&mut u, 0, 1.0, &g, None);
        for (i, (_, about, list)) in groups.iter().enumerate() {
            let screen = Screen::Cheats { group: Some(i) };
            assert_eq!(menu_len(screen), list.len() + 1);
            // letzte Zeile ("Zurück") noch ueber der Statuszeile unten
            let (_, y, _) = menu_cell(screen, list.len());
            assert!(y + 12.0 < 248.0, "Kategorie {i} zu lang");
            assert!(Ui::text_width(about, 1.0) < ui::W - 20.0);
            draw_cheats(&mut u, 0, 1.0, &g, Some(i));
            // Mausklick trifft die Zeilen
            assert_eq!(
                menu_hit(screen, 240.0, menu_cell(screen, 0).1 + 2.0),
                Some(0)
            );
        }
        let (_, y, _) = menu_cell(top, groups.len() + 1);
        assert!(y + 12.0 < 248.0, "Kategorien zu lang");
        assert_eq!(u.verts.len() % 6, 0);
    }

    #[test]
    fn menu_hit_matches_layout() {
        let (y0, step) = menu_layout(Screen::Title);
        assert_eq!(menu_hit(Screen::Title, 240.0, y0 + 2.0), Some(0));
        assert_eq!(menu_hit(Screen::Title, 240.0, y0 + step + 2.0), Some(1));
        assert_eq!(menu_hit(Screen::Title, 10.0, y0), None);
        assert_eq!(menu_hit(Screen::Paused, 240.0, 0.0), None);
    }

    #[test]
    fn hud_draws_for_all_screens() {
        let mut g = Game::new(1);
        let mut u = Ui::default();
        draw_title(&mut u, 0, 1.0, 1);
        // kein Tutorial-Knopf mehr: das Tutorial wird beim ersten Einkauf angeboten
        assert!(!title_rows().iter().any(|r| r.contains("Tutorial")));
        draw_ask_tutorial(&mut u, Screen::AskTutorial, 0, 1.0);
        draw_ask_tutorial(&mut u, Screen::Champ, 0, 1.0);
        // Ja/Nein und OK liegen im Kasten und sind per Maus treffbar
        draw_ask_tutorial(&mut u, Screen::TutorialDone, 0, 1.0);
        for s in [Screen::AskTutorial, Screen::Champ, Screen::TutorialDone] {
            let (y0, _) = menu_layout(s);
            assert!(y0 > 70.0 && y0 + menu_len(s) as f32 * 18.0 < 190.0);
            assert_eq!(menu_hit(s, 240.0, y0 + 2.0), Some(0));
        }
        options::draw(
            &mut u,
            2,
            1.0,
            &Settings::default(),
            true,
            options::Page::Display,
        );
        draw_pause(&mut u, 1, 1.0, &g);
        draw_hud(&mut u, &g, true, 1.0);
        g.outcome = Outcome::Won;
        draw_end(&mut u, &g, 0, 1.0);
        g.outcome = Outcome::Caught;
        draw_end(&mut u, &g, 0, 1.0);
        assert!(u.verts.len() > 100);
        // gruselige Schriftzuege: passen in die Breite, wechseln von Tod zu Tod, bleiben
        // aber stehen, solange der Endbildschirm zu sehen ist
        for t in DEATH_TITLES {
            assert!(Ui::text_width(t, 2.0) < ui::W - 12.0, "zu breit: {t}");
        }
        assert!(!DEATH_TITLES.contains(&"DU BIST AUFGEFALLEN"));
        let first = death_title(&g);
        g.time += 5.0;
        assert_eq!(death_title(&g), first, "Schriftzug springt");
        let mut seen = std::collections::HashSet::new();
        for k in 0..60 {
            g.run_time = 30.0 + k as f32 * 7.3;
            seen.insert(death_title(&g));
        }
        assert!(seen.len() >= 6, "kaum Abwechslung: {}", seen.len());
        assert_eq!(u.verts.len() % 6, 0);
    }

    #[test]
    fn npc_debug_overlay_draws() {
        let g = Game::new(2);
        let mut u = Ui::default();
        draw_npc_debug(&mut u, &[(100.0, 50.0, "jagt".into())], &g);
        assert!(u.verts.len() > 12);
    }

    #[test]
    fn jumpscares_cannot_be_switched_off() {
        let shown = options::value(options::Row::Jumpscares, &Settings::default());
        assert_eq!(shown, "An");
        let mut u = Ui::default();
        for e in [0.0, 0.5, 1.5, 3.0] {
            draw_menu_scare(&mut u, e, 1.0);
        }
        assert!(!u.verts.is_empty());
    }
}

/// Welche Balken unten links zu sehen sind: (Auffaelligkeit, Ausdauer).
/// - Auffaelligkeit nur im gekippten Markt und nur, wenn etwas anliegt; an der Kasse zaehlt
///   stattdessen das Misstrauen der Kassiererin (till::draw)
/// - Ausdauer nur, wenn sie nicht voll ist
/// - im Fluchtkorridor keiner von beiden: dort ist man allein (Auffaelligkeit zaehlt nicht
///   mehr) und die Ausdauer ist unendlich
pub fn meters_visible(g: &Game) -> (bool, bool) {
    let corridor = g.corridor_entered();
    let sus = g.tipped && g.suspicion > 0.01 && !g.till_active() && !corridor;
    let stamina = g.player.stamina < 0.999 && !corridor && !g.corridor_endless_stamina();
    (sus, stamina)
}

/// Auffaelligkeits-Meter: schmaler Balken unten links, blinkt bei 100 %.
fn suspicion_meter(u: &mut Ui, s: f32, time: f32) {
    meter(u, "AUFFÄLLIG", s, time);
}

/// Balken unten links mit Beschriftung (gruen -> gelb -> rot).
fn meter(u: &mut Ui, label: &str, s: f32, time: f32) {
    let s = s.clamp(0.0, 1.0);
    let (x, y, w, h) = (12.0, 250.0, 72.0, 5.0);
    let color = if s < 0.5 {
        let k = s / 0.5;
        [0.3 + 0.65 * k, 0.85, 0.3 - 0.1 * k, 0.9]
    } else {
        let k = (s - 0.5) / 0.5;
        [0.95, 0.85 - 0.7 * k, 0.2 - 0.1 * k, 0.9]
    };
    let blink = s >= 1.0 && (time * 4.0).fract() < 0.5;
    u.text_s(x, y - 11.0, label, if blink { RED } else { DIM }, 1.0);
    u.rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, [0.0, 0.0, 0.0, 0.6]);
    u.rect(x, y, w * s, h, if blink { RED } else { color });
}

/// Ausdauer: schmaler Balken ueber der Auffaelligkeit, nur sichtbar, wenn sie nicht voll ist.
/// "Dabei: Öl, Lampe" - was vom Kiosk dauerhaft wirkt (Mettbroetchen gegessen zaehlt mit).
pub(crate) fn kept_line(g: &Game) -> Option<String> {
    let o = &g.pfand.owned;
    let mut names: Vec<&str> = o.kept.iter().map(|i| i.short()).collect();
    if o.mett {
        names.push("Mett-Puste");
    }
    (!names.is_empty()).then(|| format!("Dabei: {}", names.join(", ")))
}

fn stamina_meter(u: &mut Ui, s: f32, exhausted: bool) {
    let (x, y, w, h) = (12.0, 226.0, 72.0, 4.0);
    let color = if exhausted {
        [0.85, 0.25, 0.2, 0.9]
    } else {
        [0.85, 0.85, 0.8, 0.85]
    };
    u.text_s(x, y - 11.0, "AUSDAUER", DIM, 1.0);
    u.rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, [0.0, 0.0, 0.0, 0.6]);
    u.rect(x, y, w * s.clamp(0.0, 1.0), h, color);
}

/// Wer versucht, Jumpscares abzuschalten, bekommt einen.
/// `e` = Sekunden seit dem Ausloesen. Erst weisser Blitz und Stroboskop (das Gesicht macht der
/// Renderer), dann knallt die Nachricht Zeile fuer Zeile rein und zittert.
pub fn draw_menu_scare(u: &mut Ui, e: f32, clock: f32) {
    let shake = |k: f32, amp: f32| {
        let n = ((clock * 60.0).floor() * 12.9898 + k * 78.233).sin() * 43758.547;
        (n.fract() - 0.5) * 2.0 * amp
    };
    // weisser Blitz zum Einschlag, danach rotes Pulsieren
    if e < 0.07 {
        u.rect(0.0, 0.0, ui::W, ui::H, [1.0, 1.0, 1.0, 1.0]);
        return;
    }
    if e < MENU_SCARE_FACE {
        let strobe = ((e * 14.0) as i32) % 2 == 0;
        if !strobe {
            u.rect(0.0, 0.0, ui::W, ui::H, [0.6, 0.0, 0.0, 0.35]);
        }
        return;
    }
    let t = e - MENU_SCARE_FACE;
    let pulse = 0.55 + 0.45 * (clock * 18.0).sin().abs();
    u.rect(0.0, 0.0, ui::W, ui::H, [0.05, 0.0, 0.0, 0.82]);
    u.rect(0.0, 0.0, ui::W, ui::H, [0.5, 0.0, 0.0, 0.18 * pulse]);
    let red = [1.0, 0.08 * pulse, 0.05, 1.0];
    let lines: [(&str, f32, f32, f32); 3] = [
        ("PEASANT,", 4.0, 62.0, 0.0),
        ("did you really think...", 2.0, 128.0, 0.45),
        ("...you could turn us off?", 2.0, 168.0, 1.0),
    ];
    for (k, (text, scale, y, at)) in lines.into_iter().enumerate() {
        if t < at {
            continue;
        }
        // jede Zeile knallt gross rein und schrumpft auf ihre Groesse
        let slam = (1.0 - (t - at) / 0.12).max(0.0);
        let sc = scale * (1.0 + slam * 0.8);
        let amp = 1.5 + slam * 6.0;
        let w = Ui::text_width(text, sc);
        let x = (ui::W - w) * 0.5 + shake(k as f32, amp);
        let yy = y - (sc - scale) * 4.0 + shake(k as f32 + 9.0, amp);
        // Doppelbild versetzt, wie ein kaputter Fernseher
        u.text(x + 3.0, yy, text, [0.2, 0.9, 1.0, 0.35], sc);
        u.text_s(x, yy, text, red, sc);
    }
}

/// Dauer des Troll-Jumpscares und wie lange davon das Gesicht zu sehen ist
pub const MENU_SCARE_TIME: f32 = 3.4;
pub const MENU_SCARE_FACE: f32 = 1.1;

/// Cheat "NPC-Zustände": Text ueber den Koepfen und die Phase des Directors oben.
pub fn draw_npc_debug(u: &mut Ui, labels: &[(f32, f32, String)], g: &Game) {
    for (x, y, text) in labels {
        let w = Ui::text_width(text, 1.0);
        u.rect(
            x - w * 0.5 - 2.0,
            y - 2.0,
            w + 4.0,
            12.0,
            [0.0, 0.0, 0.0, 0.55],
        );
        u.text(x - w * 0.5, *y, text, YELLOW, 1.0);
    }
    let d = &g.director;
    let phase = match d.phase {
        crate::game::director::Phase::Build => "Aufbau",
        crate::game::director::Phase::Peak => "Höhepunkt",
        crate::game::director::Phase::Relax => "Ruhe",
    };
    u.text_s(
        8.0,
        24.0,
        &format!(
            "Director: {phase} {:.0} s  Druck {:.0} %",
            d.t,
            d.pressure * 100.0
        ),
        YELLOW,
        1.0,
    );
}

#[cfg(test)]
mod controls_tests {
    use super::*;

    #[test]
    fn controls_overview_is_tidy_and_fits() {
        let (bx, by, bw, bh) = CONTROLS_BOX;
        assert!(
            bx + bw <= ui::W && by + bh <= ui::H,
            "Kasten ragt aus dem Bild"
        );
        // unter dem Pausenmenue (letzter Eintrag samt Markierung)
        let (y0, step) = menu_layout(Screen::Paused);
        assert!(y0 + step * (PAUSE_ITEMS.len() as f32 - 1.0) + 12.0 <= by);
        let lists = [CONTROLS_MOVE, CONTROLS_SHOP];
        for (k, (list, (cx, cap))) in lists.iter().zip(CONTROLS_COLS).enumerate() {
            let right = if k == 0 {
                CONTROLS_COLS[1].0 - 4.0
            } else {
                bx + bw - 3.0
            };
            for (key, what) in list.iter() {
                assert!(
                    Ui::text_width(key, 1.0) <= cap - 2.0,
                    "Taste zu breit: {key}"
                );
                let end = cx + cap + 5.0 + Ui::text_width(what, 1.0);
                assert!(end <= right, "{what} ragt hinaus");
            }
            let last = by + 30.0 + (list.len() as f32 - 1.0) * CONTROLS_ROW + 9.0;
            assert!(last <= by + bh, "Spalte {k} zu lang");
        }
        // die neuen Tasten stehen drin
        assert!(CONTROLS_MOVE.iter().any(|c| c.1 == "Springen"));
        assert!(CONTROLS_MOVE.iter().any(|c| c.1 == "Ducken"));
        let mut u = Ui::default();
        draw_controls(&mut u);
        assert!(!u.verts.is_empty());
    }
}

#[cfg(test)]
mod meter_tests {
    use super::*;

    #[test]
    fn no_meters_in_the_corridor() {
        // im gekippten Markt: auffaellig und ausser Atem -> beide Balken
        let mut g = Game::new(64);
        g.tip();
        g.suspicion = 0.6;
        g.player.stamina = 0.4;
        assert_eq!(meters_visible(&g), (true, true));
        // im Fluchtkorridor (Tuer zu): keiner von beiden, auch wenn die Werte gleich bleiben
        g.debug_jump_to_corridor(None);
        g.update(1.0 / 60.0, &mut crate::game::Input::default());
        assert!(g.corridor_entered());
        g.suspicion = 0.6;
        g.player.stamina = 0.4;
        assert_eq!(meters_visible(&g), (false, false));
    }
}
