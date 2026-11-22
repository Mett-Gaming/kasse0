//! HUD, Menues und End-Screens (alles im 480x270-Raster, unter dem VHS-Effekt).

mod smoke;
mod till;

use crate::game::dialog::CART_OPENER;
use crate::game::tutorial::{Tutorial, STEPS as TUTORIAL_STEPS};
use crate::game::{Focus, Game, Outcome, PickupKind};
use crate::settings::Settings;
use crate::ui::{self, euro, Ui, DIM, GREEN, RED, WHITE, YELLOW};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Title,
    Options { from_pause: bool },
    Playing,
    Paused,
    End,
    Cheats,
}

pub const TITLE_ITEMS: &[&str] = &[
    "Neuer Einkauf",
    "Tutorial überspringen",
    "Optionen",
    "Beenden",
];

/// Hauptmenue-Zeilen; der zweite Punkt ist ein Haken (Tutorial ueberspringen).
pub fn title_rows(tutorial: bool) -> Vec<String> {
    TITLE_ITEMS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            if i == 1 {
                format!("[{}] {s}", if tutorial { " " } else { "x" })
            } else {
                s.to_string()
            }
        })
        .collect()
}
pub const PAUSE_ITEMS: &[&str] = &[
    "Weiter",
    "Optionen",
    "Neu starten (gleicher Seed)",
    "Hauptmenü",
    "Beenden",
];
pub const END_ITEMS: &[&str] = &["Nochmal (gleicher Seed)", "Neuer Einkauf", "Hauptmenü"];
pub const OPTION_COUNT: usize = 8;

pub fn option_rows(s: &Settings) -> Vec<String> {
    vec![
        format!("Mausempfindlichkeit   < {:.1}x >", s.sensitivity),
        format!("Lautstärke            < {:>3.0}% >", s.volume * 100.0),
        format!("VHS-Effekt            < {:>3.0}% >", s.vhs * 100.0),
        format!("Sichtfeld             < {:>3.0}° >", s.fov),
        format!(
            "Vollbild              < {:<4} >",
            if s.fullscreen { "An" } else { "Aus" }
        ),
        format!(
            "Tutorial              < {:<4} >",
            if s.tutorial { "An" } else { "Aus" }
        ),
        // laesst sich nicht abschalten (siehe main.rs: menu_scare)
        "Jumpscares            < An   >".to_string(),
        "Zurück".to_string(),
    ]
}

/// Erste Menuezeile (y) und Zeilenabstand fuer Zeichnen und Maus-Treffer.
pub fn menu_layout(screen: Screen) -> (f32, f32) {
    match screen {
        Screen::Title => (150.0, 18.0),
        Screen::Options { .. } => (82.0, 17.0),
        Screen::Paused => (66.0, 16.0),
        Screen::End => (205.0, 16.0),
        Screen::Cheats => (30.0, 12.5),
        Screen::Playing => (0.0, 0.0),
    }
}

pub fn menu_len(screen: Screen) -> usize {
    match screen {
        Screen::Title => TITLE_ITEMS.len(),
        Screen::Options { .. } => OPTION_COUNT,
        Screen::Paused => PAUSE_ITEMS.len(),
        Screen::End => END_ITEMS.len(),
        Screen::Cheats => crate::game::CHEATS.len() + 1,
        Screen::Playing => 0,
    }
}

/// Welche Menuezeile liegt unter dem Mauszeiger?
pub fn menu_hit(screen: Screen, x: f32, y: f32) -> Option<usize> {
    let (y0, step) = menu_layout(screen);
    if !(60.0..=420.0).contains(&x) {
        return None;
    }
    (0..menu_len(screen)).find(|&i| {
        let top = y0 + i as f32 * step - 4.0;
        y >= top && y < top + step
    })
}

fn draw_menu(u: &mut Ui, screen: Screen, items: &[String], sel: usize, time: f32) {
    let (y0, step) = menu_layout(screen);
    for (i, label) in items.iter().enumerate() {
        let y = y0 + i as f32 * step;
        if i == sel {
            let w = Ui::text_width(label, 1.0) + 24.0;
            u.rect(
                (ui::W - w) * 0.5,
                y - 4.0,
                w,
                16.0,
                [0.95, 0.95, 0.92, 0.12 + 0.05 * (time * 4.0).sin()],
            );
            u.text_center(y, &format!("▶ {label}"), YELLOW, 1.0);
        } else {
            u.text_center(y, label, WHITE, 1.0);
        }
    }
}

fn osd(u: &mut Ui, time: f32, label: &str) {
    if (time * 1.5).fract() < 0.7 {
        u.text_s(12.0, 10.0, &format!("▶ {label}"), WHITE, 1.0);
    }
}

fn timestamp(secs: f32) -> String {
    let s = secs as u32;
    format!("SP {}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

pub fn draw_title(u: &mut Ui, sel: usize, time: f32, seed_hint: u64, tutorial: bool) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.35]);
    osd(u, time, "PLAY");
    u.text_center(52.0, "KASSE 0", WHITE, 4.0);
    u.text_center(92.0, "ein ganz normaler Einkauf", DIM, 1.0);
    draw_menu(u, Screen::Title, &title_rows(tutorial), sel, time);
    u.text_s(12.0, 252.0, &format!("Band #{seed_hint}"), DIM, 1.0);
    u.text_s(
        ui::W - 12.0 - Ui::text_width(&timestamp(time), 1.0),
        252.0,
        &timestamp(time),
        WHITE,
        1.0,
    );
}

pub fn draw_options(u: &mut Ui, sel: usize, time: f32, s: &Settings, from_pause: bool) {
    u.rect(
        0.0,
        0.0,
        ui::W,
        ui::H,
        [0.0, 0.0, 0.0, if from_pause { 0.7 } else { 0.5 }],
    );
    u.text_center(40.0, "OPTIONEN", WHITE, 2.0);
    draw_menu(
        u,
        Screen::Options { from_pause },
        &option_rows(s),
        sel,
        time,
    );
    u.text_center(
        214.0,
        "Pfeiltasten / A D ändern - Enter bestätigt - Esc zurück",
        DIM,
        1.0,
    );
    u.text_center(228.0, "Klick: erhöhen - Rechtsklick: verringern", DIM, 1.0);
    u.text_center(
        250.0,
        "Modelle: Quaternius, Poly Haven, Poly by Google",
        DIM,
        1.0,
    );
}

pub fn draw_pause(u: &mut Ui, sel: usize, time: f32, g: &Game) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.65]);
    osd(u, time, "PAUSE");
    u.text_center(28.0, "PAUSE", WHITE, 3.0);
    let items: Vec<String> = PAUSE_ITEMS.iter().map(|s| s.to_string()).collect();
    draw_menu(u, Screen::Paused, &items, sel, time);
    draw_list(u, g, 16.0, 154.0, false);
    draw_controls(u, 186.0, 154.0);
}

fn draw_list(u: &mut Ui, g: &Game, x: f32, y: f32, panel: bool) {
    let lines = g.list.len() as f32 + 3.0;
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
            u.text(x, y, s, c, 1.0)
        } else {
            u.text_s(x, y, s, c, 1.0)
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
}

pub fn draw_hud(u: &mut Ui, g: &Game, show_list: bool, time: f32) {
    if g.outcome != Outcome::Running {
        if g.outcome == Outcome::Caught && (g.time * 10.0).fract() < 0.5 {
            u.text_center(120.0, "TRACKING", RED, 2.0);
        }
        return;
    }
    osd(u, time, "PLAY");
    let ts = timestamp(g.run_time);
    u.text_s(
        ui::W - 12.0 - Ui::text_width(&ts, 1.0),
        252.0,
        &ts,
        WHITE,
        1.0,
    );

    // Auffaelligkeit (nur im gekippten Markt): Balken unten links, gruen -> gelb -> rot
    // Leisten nur, wenn etwas fehlt bzw. anliegt
    // An der Kasse zaehlt stattdessen das Misstrauen der Kassiererin (till::draw)
    if g.tipped && g.suspicion > 0.01 && !g.till_active() {
        suspicion_meter(u, g.suspicion, time);
    }
    if g.player.stamina < 0.999 {
        stamina_meter(u, g.player.stamina, g.player.exhausted);
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
        u.text_center(
            y,
            &format!("In der Hand: {}  {}", item.name, euro(item.cents)),
            WHITE,
            1.0,
        );
        let action = if g.focus == Some(Focus::Belt) {
            "[E] Aufs Band legen"
        } else if g.aim_cart {
            "[E] In den Wagen legen"
        } else {
            "[E] Fallen lassen  (auf den Wagen zielen)"
        };
        u.text_center(y + 11.0, action, YELLOW, 1.0);
    } else {
        match g.focus {
            Some(Focus::Shelf(i)) => {
                let p = crate::game::ItemData::from_shelf(&g.world.items[i]);
                u.text_center(
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
                u.text_center(y, &label, c, 1.0);
            }
            Some(Focus::Loose(i)) => {
                u.text_center(
                    y,
                    &format!("[E] Aufheben: {}", g.loose[i].data.name),
                    WHITE,
                    1.0,
                );
            }
            Some(Focus::Belt) => match g.belt_pick() {
                Some(i) if g.cart_at_register() => {
                    let s = format!(
                        "[E] Aufs Band: {}   ({}/{}, Mausrad)",
                        g.cart[i].name,
                        i + 1,
                        g.cart.len()
                    );
                    u.text_center(y, &s, YELLOW, 1.0);
                    u.text_center(
                        y + 11.0,
                        "Getränke zuerst. Nichts Falsches aufs Band.",
                        DIM,
                        1.0,
                    );
                }
                Some(_) => u.text_center(y, "Kassenband - bring deinen Wagen her", YELLOW, 1.0),
                None => u.text_center(y, "Kassenband", YELLOW, 1.0),
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
                u.text_center(y, &s, YELLOW, 1.0);
            }
            _ if g.at_register && g.checking_out() => {
                let s = format!("Die Kassiererin scannt ... ({} auf dem Band)", g.belt.len());
                u.text_center(y, &s, YELLOW, 1.0);
            }
            Some(Focus::Employee(_)) => {
                u.text_center(y, "[E] \"Hey, das ist mein Wagen!\"", YELLOW, 1.0);
            }
            Some(Focus::Cart) => {
                let s = if g.trolley.pushing {
                    "[E/Q] Wagen stehen lassen"
                } else {
                    "[E/Q] Wagen schieben"
                };
                u.text_center(y, s, WHITE, 1.0);
                if let Some(i) = g.cart_selected() {
                    u.text_center(
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

    if let Some(t) = &g.tutorial {
        draw_tutorial(u, t);
    }

    // Einkaufszettel (TAB) bzw. kurzer Hinweis
    if show_list {
        draw_list(u, g, 18.0, 34.0, true);
    } else if g.run_time < 20.0 && g.tutorial.is_none() {
        u.text_s(12.0, 24.0, "TAB: Einkaufszettel", DIM, 1.0);
    }

    // Durchsage
    if let Some((text, _)) = &g.announcement {
        u.text_center(26.0, "~ DURCHSAGE ~", YELLOW, 1.0);
        for (k, line) in Ui::wrap(text, 52).iter().enumerate() {
            u.text_center(38.0 + k as f32 * 10.0, line, WHITE, 1.0);
        }
    }

    // Meldung
    if let Some((text, _)) = &g.message {
        for (k, line) in Ui::wrap(text, 52).iter().enumerate() {
            u.text_center(196.0 + k as f32 * 10.0, line, WHITE, 1.0);
        }
    }

    // Dialog mit einem Mitarbeiter
    if let Some(d) = &g.dialog {
        let q = d.question();
        u.rect(40.0, 160.0, 400.0, 86.0, [0.02, 0.02, 0.03, 0.85]);
        u.rect(40.0, 160.0, 400.0, 1.0, [0.9, 0.9, 0.85, 0.6]);
        if d.cart_talk {
            u.text(48.0, 152.0, &format!("DU: {CART_OPENER}"), WHITE, 1.0);
        }
        u.text(48.0, 166.0, "MITARBEITER:", DIM, 1.0);
        match d.reply {
            Some((reply, _)) => u.text(48.0, 180.0, reply, WHITE, 1.0),
            None => {
                u.text(48.0, 180.0, q.text, WHITE, 1.0);
                for k in 0..3 {
                    let a = q.answers[d.order[k]];
                    u.text(
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
    smoke::draw(u, g, time);

    // Kassen-Minispiel: Misstrauen, Leben, Fragen, Karte, PIN, Geldboerse
    till::draw(u, g, time);

    // Flucht nach dem Bezahlen
    if g.chasing() && (time * 2.5).fract() < 0.6 {
        u.text_center(64.0, "ALARM - LAUF ZUM AUSGANG!", RED, 2.0);
    }
}

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
    u.text(x, y, "STEUERUNG", YELLOW, 1.0);
    for (i, (key, what)) in TUTORIAL_STEPS.iter().enumerate() {
        let yy = y + 14.0 + i as f32 * 11.0;
        let (mark, c) = match i.cmp(&t.step) {
            std::cmp::Ordering::Less => ("x", GREEN),
            std::cmp::Ordering::Equal => (">", YELLOW),
            std::cmp::Ordering::Greater => (" ", DIM),
        };
        let line = format!("{mark} {key:<11} {what}");
        u.text(x, yy, &line, c, 1.0);
    }
}

/// Tastenbelegung (Pause-Menue)
fn draw_controls(u: &mut Ui, x: f32, y: f32) {
    u.text_s(x, y, "STEUERUNG", YELLOW, 1.0);
    for (i, (key, what)) in TUTORIAL_STEPS.iter().enumerate() {
        let line = format!("{key:<11} {what}");
        u.text_s(x, y + 12.0 + i as f32 * 11.0, &line, WHITE, 1.0);
    }
}

/// Cheat-Menue (F1): Aktionen + "Zurück"; daneben der aktuelle Zustand.
pub fn draw_cheats(u: &mut Ui, sel: usize, time: f32, g: &Game) {
    u.rect(0.0, 0.0, ui::W, ui::H, [0.0, 0.0, 0.0, 0.7]);
    u.text_center(16.0, "CHEATS (F1)", RED, 2.0);
    let mut items: Vec<String> = crate::game::CHEATS.iter().map(|s| s.to_string()).collect();
    items[0] = format!("Unverwundbar: {}", if g.god { "AN" } else { "AUS" });
    items.push("Zurück".into());
    draw_menu(u, Screen::Cheats, &items, sel, time);
    let state = format!(
        "Stufe: {}   Wagen: {}   Auffällig: {:.0} %   {}",
        g.tier.name(),
        euro(g.cart_value()),
        g.suspicion * 100.0,
        if g.tipped { "gekippt" } else { "normal" }
    );
    u.text_center(252.0, &state, DIM, 1.0);
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
            u.text(x + 46.0, 26.0, "KAUF NULL", ink, 1.0);
            u.text(x + 10.0, 40.0, &format!("Band #{}", g.seed), ink, 1.0);
            for (i, item) in g.scanned.iter().take(lines).enumerate() {
                let name: String = item.name.chars().take(13).collect();
                let price = euro(item.cents);
                let y = 56.0 + i as f32 * 10.0;
                u.text(x + 8.0, y, &name, ink, 1.0);
                u.text(x + 172.0 - Ui::text_width(&price, 1.0), y, &price, ink, 1.0);
            }
            if g.scanned.len() > lines {
                u.text(x + 8.0, 56.0 + lines as f32 * 10.0, "...", ink, 1.0);
            }
            let y = 62.0 + (lines as f32 + 1.0) * 10.0;
            let total = format!("SUMME {}", euro(g.scanned_value()));
            u.text(x + 172.0 - Ui::text_width(&total, 1.0), y, &total, ink, 1.0);
            let t = g.run_time as u32;
            u.text(
                x + 8.0,
                y + 12.0,
                &format!("Zeit {}:{:02}", t / 60, t % 60),
                ink,
                1.0,
            );
            u.text_center(y + 34.0, "Danke für Ihren Einkauf.", WHITE, 1.0);
        }
        _ => {
            osd(u, time, "STOP");
            u.text_center(70.0, "DU BIST AUFGEFALLEN", RED, 2.0);
            let why = if g.till_caught {
                "Die Kassiererin hat dich durchschaut."
            } else if g.chase.is_some() {
                "Sie haben Kunde 0 geschnappt - kurz vor dem Ausgang."
            } else {
                "Sie haben dich erreicht."
            };
            u.text_center(100.0, why, WHITE, 1.0);
            let t = g.run_time as u32;
            u.text_center(
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
            u.text_center(140.0, &format!("Stufe: {}", g.tier.name()), DIM, 1.0);
        }
    }
    let items: Vec<String> = END_ITEMS.iter().map(|s| s.to_string()).collect();
    draw_menu(u, Screen::End, &items, sel, time);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        draw_title(&mut u, 0, 1.0, 1, true);
        assert_eq!(title_rows(false)[1], "[x] Tutorial überspringen");
        assert_eq!(title_rows(true)[1], "[ ] Tutorial überspringen");
        draw_options(&mut u, 2, 1.0, &Settings::default(), true);
        draw_pause(&mut u, 1, 1.0, &g);
        draw_hud(&mut u, &g, true, 1.0);
        g.outcome = Outcome::Won;
        draw_end(&mut u, &g, 0, 1.0);
        g.outcome = Outcome::Caught;
        draw_end(&mut u, &g, 0, 1.0);
        assert!(u.verts.len() > 100);
        assert_eq!(u.verts.len() % 6, 0);
    }

    #[test]
    fn jumpscares_cannot_be_switched_off() {
        let rows = option_rows(&Settings::default());
        assert_eq!(rows.len(), OPTION_COUNT);
        assert!(rows[6].starts_with("Jumpscares") && rows[6].contains("An"));
        // alle Zeilen passen auf den Bildschirm
        let (y0, step) = menu_layout(Screen::Options { from_pause: false });
        assert!(y0 + step * (OPTION_COUNT as f32) < ui::H);
        let mut u = Ui::default();
        for e in [0.0, 0.5, 1.5, 3.0] {
            draw_menu_scare(&mut u, e, 1.0);
        }
        assert!(!u.verts.is_empty());
    }
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
