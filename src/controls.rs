//! Steuerung: Aktionen, ihre (umstellbare) Belegung auf Tastatur, Maus und Controller, und die
//! passenden Tastennamen fuer Hinweise im Bild.
//!
//! Fest belegt sind Esc (Pause), F1, F12, die Ziffern, sowie am Controller Start, LB/RB und
//! links/oben/rechts auf dem Kreuz (Antworten 1-3, PIN).

use mettgine::input::{Bind, Bindings, Key, MouseButton, Pad, Stick};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Run,
    Crouch,
    Jump,
    Interact,
    Cart,
    TakeOut,
    Complain,
    List,
}

/// Aktion, Speichername, Vorgabe
const DEFAULTS: &[(Action, &str, &[Bind])] = {
    use Action::*;
    &[
        (
            Forward,
            "vor",
            &[
                Bind::Key(Key::KeyW),
                Bind::Key(Key::ArrowUp),
                Bind::Stick(Stick::Up),
            ],
        ),
        (
            Back,
            "zurueck",
            &[
                Bind::Key(Key::KeyS),
                Bind::Key(Key::ArrowDown),
                Bind::Stick(Stick::Down),
            ],
        ),
        (
            Left,
            "links",
            &[
                Bind::Key(Key::KeyA),
                Bind::Key(Key::ArrowLeft),
                Bind::Stick(Stick::Left),
            ],
        ),
        (
            Right,
            "rechts",
            &[
                Bind::Key(Key::KeyD),
                Bind::Key(Key::ArrowRight),
                Bind::Stick(Stick::Right),
            ],
        ),
        (
            Run,
            "rennen",
            &[
                Bind::Key(Key::ShiftLeft),
                Bind::Key(Key::ShiftRight),
                Bind::Pad(Pad::LeftTrigger),
            ],
        ),
        (
            Crouch,
            "ducken",
            &[
                Bind::Key(Key::KeyC),
                Bind::Key(Key::ControlLeft),
                Bind::Key(Key::ControlRight),
                Bind::Pad(Pad::East),
            ],
        ),
        (
            Jump,
            "springen",
            &[Bind::Key(Key::Space), Bind::Pad(Pad::South)],
        ),
        (
            Interact,
            "benutzen",
            &[
                Bind::Key(Key::KeyE),
                Bind::Mouse(MouseButton::Left),
                Bind::Pad(Pad::West),
            ],
        ),
        (
            Cart,
            "wagen",
            &[Bind::Key(Key::KeyQ), Bind::Pad(Pad::North)],
        ),
        (
            TakeOut,
            "herausnehmen",
            &[Bind::Key(Key::KeyF), Bind::Pad(Pad::RightTrigger)],
        ),
        (
            Complain,
            "reklamieren",
            &[Bind::Key(Key::KeyR), Bind::Pad(Pad::Down)],
        ),
        (
            List,
            "zettel",
            &[Bind::Key(Key::Tab), Bind::Pad(Pad::Select)],
        ),
    ]
};

pub const ACTIONS: usize = DEFAULTS.len();

pub fn all() -> impl Iterator<Item = Action> {
    DEFAULTS.iter().map(|d| d.0)
}

pub fn name(a: Action) -> &'static str {
    match a {
        Action::Forward => "Vorwärts",
        Action::Back => "Rückwärts",
        Action::Left => "Links",
        Action::Right => "Rechts",
        Action::Run => "Rennen",
        Action::Crouch => "Ducken",
        Action::Jump => "Springen",
        Action::Interact => "Benutzen",
        Action::Cart => "Wagen",
        Action::TakeOut => "Ware herausnehmen",
        Action::Complain => "Reklamieren",
        Action::List => "Einkaufszettel",
    }
}

pub type Keys = Bindings<Action>;

pub fn defaults() -> Keys {
    Bindings::new(DEFAULTS)
}

const FILE: &str = "tasten.txt";

pub fn load() -> Keys {
    let mut k = defaults();
    k.load(&mettgine::settings::load("kasse0", FILE));
    k
}

pub fn save(k: &Keys) {
    let mut kv = Default::default();
    k.save(&mut kv);
    mettgine::settings::save("kasse0", FILE, &kv);
}

/// Tastatur und Maus gehoeren zusammen, Knoepfe und Sticks ebenso.
fn is_pad(b: &Bind) -> bool {
    matches!(b, Bind::Pad(_) | Bind::Stick(_))
}

/// Laesst sich diese Taste fuer eine Aktion benutzen?
pub fn allowed(b: Bind) -> bool {
    match b {
        Bind::Key(k) => !matches!(
            k,
            Key::Escape
                | Key::F1
                | Key::F12
                | Key::Digit0
                | Key::Digit1
                | Key::Digit2
                | Key::Digit3
                | Key::Digit4
                | Key::Digit5
                | Key::Digit6
                | Key::Digit7
                | Key::Digit8
                | Key::Digit9
        ),
        Bind::Pad(p) => !matches!(
            p,
            Pad::Start | Pad::LeftBumper | Pad::RightBumper | Pad::Up | Pad::Left | Pad::Right
        ),
        // der linke Stick laeuft immer
        Bind::Stick(_) => false,
        Bind::Mouse(_) => true,
    }
}

/// Neue Taste fuer eine Aktion. Wer sie bisher hatte, bekommt dafuer die alte: So bleibt jede
/// Aktion erreichbar.
pub fn rebind(k: &mut Keys, action: Action, new: Bind) {
    let pad = is_pad(&new);
    let mine: Vec<Bind> = k.of(action).to_vec();
    if mine.contains(&new) && mine.iter().filter(|b| is_pad(b) == pad).count() == 1 {
        return;
    }
    let old = mine.iter().copied().find(|b| is_pad(b) == pad);
    for other in all().filter(|o| *o != action) {
        let mut binds = k.of(other).to_vec();
        if !binds.contains(&new) {
            continue;
        }
        binds.retain(|b| *b != new);
        // hatte sie sonst nichts auf diesem Geraet: alte Taste uebernehmen
        if !binds.iter().any(|b| is_pad(b) == pad) {
            binds.extend(old);
        }
        k.set(other, &binds);
    }
    let mut binds = vec![new];
    binds.extend(
        mine.into_iter()
            .filter(|b| is_pad(b) != pad || matches!(b, Bind::Stick(_))),
    );
    k.set(action, &binds);
}

/// Name der ersten Belegung auf einem Geraet.
pub fn label(k: &Keys, action: Action, pad: bool) -> String {
    let binds = k.of(action);
    let found = binds.iter().find(|b| is_pad(b) == pad);
    found.map_or("-".into(), |b| bind_label(*b))
}

fn bind_label(b: Bind) -> String {
    match b {
        Bind::Pad(p) => match p {
            Pad::South => "A",
            Pad::East => "B",
            Pad::West => "X",
            Pad::North => "Y",
            Pad::LeftBumper => "LB",
            Pad::RightBumper => "RB",
            Pad::LeftTrigger => "LT",
            Pad::RightTrigger => "RT",
            Pad::Start => "Start",
            Pad::Select => "Select",
            Pad::Up => "Kreuz oben",
            Pad::Down => "Kreuz unten",
            Pad::Left => "Kreuz links",
            Pad::Right => "Kreuz rechts",
        }
        .into(),
        Bind::Stick(_) => "Stick".into(),
        Bind::Mouse(MouseButton::Left) => "Klick".into(),
        Bind::Mouse(MouseButton::Right) => "Rechtsklick".into(),
        Bind::Mouse(_) => "Maustaste".into(),
        Bind::Key(k) => key_label(k),
    }
}

fn key_label(k: Key) -> String {
    let full = format!("{k:?}");
    match k {
        // deutsche Tastatur: die Lage zaehlt, beschriftet sind sie andersherum
        Key::KeyY => "Z".into(),
        Key::KeyZ => "Y".into(),
        Key::ShiftLeft => "Shift".into(),
        Key::ShiftRight => "Shift R".into(),
        Key::ControlLeft => "Strg".into(),
        Key::ControlRight => "Strg R".into(),
        Key::AltLeft => "Alt".into(),
        Key::Space => "Leertaste".into(),
        Key::Enter => "Enter".into(),
        Key::ArrowUp => "Pfeil hoch".into(),
        Key::ArrowDown => "Pfeil runter".into(),
        Key::ArrowLeft => "Pfeil links".into(),
        Key::ArrowRight => "Pfeil rechts".into(),
        _ => full.strip_prefix("Key").unwrap_or(&full).to_string(),
    }
}

// --- Hinweise im Bild ---

/// Ersetzungen fuer Hinweise; leer = Vorgabe auf der Tastatur
static HINTS: RwLock<Vec<(String, String)>> = RwLock::new(Vec::new());

/// Zuletzt ein Controller benutzt?
static PAD: AtomicBool = AtomicBool::new(false);

pub fn pad_active() -> bool {
    PAD.load(Ordering::Relaxed)
}

/// Hinweise an Belegung und Geraet anpassen.
pub fn set_hints(k: &Keys, pad: bool) {
    PAD.store(pad, Ordering::Relaxed);
    let new = hints(k, pad);
    if let Ok(mut h) = HINTS.write() {
        *h = new;
    }
}

/// Nur neu rechnen, wenn sich Geraet oder Belegung geaendert haben (`last` = None: geaendert).
pub fn set_hints_if_changed(last: &mut Option<bool>, k: &Keys, pad: bool) {
    if *last != Some(pad) {
        *last = Some(pad);
        set_hints(k, pad);
    }
}

fn hints(k: &Keys, pad: bool) -> Vec<(String, String)> {
    let d = defaults();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut add = |from: &str, to: String| out.push((from.into(), to));
    if pad {
        // laengere Wendungen zuerst
        for (from, to) in [
            ("A/D oder Mausrad", "Stick oder LB/RB"),
            ("A/D", "Stick"),
            ("W A S D", "Stick"),
            ("WASD", "Stick"),
            ("Mausrad", "LB/RB"),
            ("Zifferntasten 0-9", "Kreuz wählt, A tippt"),
            ("Zifferntasten", "Kreuz"),
            ("0-9", "Kreuz"),
            ("1/2/3", "Kreuz"),
            ("1 2 3", "Kreuz"),
            ("mit gedrückter Maus", "mit gehaltenem A"),
            ("mit der Maus", "mit A und Stick"),
            ("die Maus", "den Stick"),
            ("Maus", "Stick"),
            ("Rad", "LB/RB"),
            ("Esc", "Start"),
            ("Enter", "A"),
        ] {
            add(from, to.into());
        }
    }
    for (a, words) in [
        (Action::Crouch, &["C oder Strg", "C/Strg"] as &[&str]),
        (Action::Jump, &["Leertaste", "Leer"]),
        (Action::List, &["TAB", "Tab"]),
        (Action::Run, &["Shift"]),
        (Action::Interact, &["E / Klick", "E"]),
        (Action::Cart, &["Q"]),
        (Action::TakeOut, &["F"]),
        (Action::Complain, &["R"]),
    ] {
        if pad || label(k, a, false) != label(&d, a, false) {
            for w in words {
                add(w, label(k, a, pad));
            }
        }
    }
    out
}

/// Hinweis mit den Tasten, die gerade gelten.
pub fn hint(s: &str) -> Cow<'_, str> {
    match HINTS.read() {
        Ok(h) if !h.is_empty() => Cow::Owned(replace_words(s, &h)),
        _ => Cow::Borrowed(s),
    }
}

/// Ersetzt ganze Woerter; Ersetztes wird nicht noch einmal angefasst.
fn replace_words(s: &str, list: &[(String, String)]) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    let mut before: Option<char> = None;
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        let start = !before.is_some_and(char::is_alphanumeric);
        let found = list.iter().find(|(from, _)| {
            start
                && rest.starts_with(from.as_str())
                && !rest[from.len()..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric)
        });
        if let Some((from, to)) = found {
            out.push_str(to);
            before = from.chars().last();
            rest = &rest[from.len()..];
        } else {
            out.push(c);
            before = Some(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_action_has_a_key_and_a_button_and_none_is_shared() {
        let k = defaults();
        let mut seen = HashSet::new();
        for a in all() {
            assert!(k.of(a).iter().any(is_pad), "{a:?} ohne Knopf");
            assert!(k.of(a).iter().any(|b| !is_pad(b)), "{a:?} ohne Taste");
            for b in k.of(a) {
                assert!(seen.insert(*b), "{b:?} doppelt");
                assert!(allowed(*b) || matches!(b, Bind::Stick(_)), "{b:?}");
            }
        }
    }

    #[test]
    fn rebinding_swaps_so_nothing_gets_lost() {
        let mut k = defaults();
        // Q fuer Benutzen: der Wagen bekommt E
        rebind(&mut k, Action::Interact, Bind::Key(Key::KeyQ));
        assert_eq!(label(&k, Action::Interact, false), "Q");
        assert_eq!(label(&k, Action::Cart, false), "E");
        // Controller bleibt, wie er war
        assert_eq!(label(&k, Action::Interact, true), "X");
        rebind(&mut k, Action::Jump, Bind::Pad(Pad::West));
        assert_eq!(label(&k, Action::Interact, true), "A");
        for a in all() {
            assert!(k.of(a).iter().any(is_pad) && k.of(a).iter().any(|b| !is_pad(b)));
        }
        // uebersteht das Speichern
        let mut kv = Default::default();
        k.save(&mut kv);
        let mut back = defaults();
        back.load(&kv);
        assert!(all().all(|a| back.of(a) == k.of(a)));
    }

    #[test]
    fn walking_keeps_the_stick_when_a_key_changes() {
        let mut k = defaults();
        rebind(&mut k, Action::Forward, Bind::Key(Key::KeyI));
        assert!(k.of(Action::Forward).contains(&Bind::Stick(Stick::Up)));
    }

    #[test]
    fn hints_follow_the_device_and_the_bindings() {
        let k = defaults();
        assert!(hints(&k, false).is_empty(), "Tastatur-Vorgabe bleibt");
        let pad = hints(&k, true);
        let show = |s: &str| replace_words(s, &pad);
        assert_eq!(show("[E] Aufheben: Eier"), "[X] Aufheben: Eier");
        assert_eq!(show("[E/Q] Wagen schieben"), "[X/Y] Wagen schieben");
        assert_eq!(show("TAB: Einkaufszettel"), "Select: Einkaufszettel");
        assert_eq!(show("Leertaste springt, C oder"), "A springt, C oder");
        assert_eq!(show("A/D oder Mausrad wählt"), "Stick oder LB/RB wählt");
        let mut k = k;
        rebind(&mut k, Action::Interact, Bind::Key(Key::KeyG));
        let keys = hints(&k, false);
        assert_eq!(
            replace_words("[E] Tür aufdrücken", &keys),
            "[G] Tür aufdrücken"
        );
    }
}
