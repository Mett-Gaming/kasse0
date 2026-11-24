//! Aktionen ausfuehren, Controller abfragen, Tasten umbelegen.

use super::*;
use controls::Action;
use game::till::{Pay, SWIPE_TRAVEL};
use mettgine::input::{Bind, Pad, Stick};

/// Blickdrehung bei vollem Ausschlag (Radiant/s, waagerecht und senkrecht)
const LOOK_YAW: f32 = 2.8;
const LOOK_PITCH: f32 = 1.7;
/// Karte durchziehen: voller Ausschlag braucht so lange (zwischen zu schnell und zu langsam)
const SWIPE_SECONDS: f32 = 0.75;
/// Zeiger im Kassenfenster (Rasterpunkte/s)
const CURSOR_SPEED: f32 = 210.0;
/// Menue: Wiederholung bei gehaltener Richtung
const REPEAT_FIRST: f32 = 0.4;
const REPEAT_NEXT: f32 = 0.13;

/// Was der Controller zuletzt tat.
pub struct PadState {
    /// Aktion wird gerade am Controller gehalten
    held: [bool; controls::ACTIONS],
    repeat: f32,
    /// Ziffer, die fuers Kartenterminal gewaehlt ist
    digit: u8,
    pub focused: bool,
    /// letzte Vibration klingt noch (Sekunden)
    rumble: f32,
    was_dying: bool,
}

impl Default for PadState {
    fn default() -> Self {
        Self {
            held: Default::default(),
            repeat: 0.0,
            digit: 5,
            focused: true,
            rumble: 0.0,
            was_dying: false,
        }
    }
}

impl App {
    /// Eine Aktion im Spiel: gedrueckt oder losgelassen.
    pub fn action(&mut self, a: Action, down: bool) {
        let i = &mut self.input;
        match a {
            Action::Forward => i.forward = down,
            Action::Back => i.back = down,
            Action::Left => i.left = down,
            Action::Right => i.right = down,
            Action::Run => i.run = down,
            Action::Crouch => i.crouch = down,
            Action::List => {
                self.show_list = down;
                i.list_open = down;
            }
            Action::Cart if down => i.cart_toggle = true,
            Action::TakeOut if down => i.take_out = true,
            Action::Complain if down => i.complain = true,
            Action::Interact if down => i.interact = true,
            Action::Jump if down => i.jump = true,
            Action::Cart | Action::TakeOut | Action::Complain | Action::Interact | Action::Jump => {
            }
        }
    }

    /// Aktion, die auf dieser Taste liegt.
    pub fn action_of(&self, b: Bind) -> Option<Action> {
        controls::all().find(|a| self.keys.of(*a).contains(&b))
    }

    /// Ziffer getippt: Antworten 1-3, PIN, Zigarette.
    pub fn digit(&mut self, d: u8) {
        if (1..=3).contains(&d) {
            self.input.choice = Some(d as usize - 1);
        }
        self.input.digit = Some(d);
    }

    fn typing_pin(&self) -> bool {
        let pay = self.game.till.as_ref().map(|t| t.pay);
        self.game.till_paying() && matches!(pay, Some(Pay::Pin(_)))
    }

    /// Einmal pro Bild: Controller auswerten.
    pub fn poll_pad(&mut self, stage: &mut Stage, dt: f32) {
        let playing = self.screen == Screen::Playing;
        // eigene Belegung; gehaltene Knoepfe aus dem Menue zaehlen im Spiel nicht
        let mut edges = Vec::new();
        for (i, a) in controls::all().enumerate() {
            let binds: Vec<Bind> = self.keys.of(a).to_vec();
            let pads = binds
                .into_iter()
                .filter(|b| matches!(b, Bind::Pad(_) | Bind::Stick(_)))
                .collect::<Vec<_>>();
            let now = stage.input.held(&pads);
            if now != self.pad.held[i] {
                self.pad.held[i] = now;
                edges.push((a, now));
            }
        }
        let pad_used = stage.input.using_pad;
        controls::set_hints_if_changed(&mut self.hints_for, &self.keys, pad_used);
        self.input.stick = (0.0, 0.0);
        if stage.splash_active() || !self.pad.focused {
            return;
        }
        let pressed = |p: Pad| stage.input.pressed(&[Bind::Pad(p)]);
        if let Some(a) = self.rebinding {
            let any = stage
                .input
                .any_pressed()
                .filter(|b| matches!(b, Bind::Pad(_)));
            if let Some(b) = any {
                self.finish_rebind(a, b);
            }
            return;
        }
        if !playing {
            let (up, down, left, right) = (
                self.pad_dir(stage, Pad::Up, Stick::Up),
                self.pad_dir(stage, Pad::Down, Stick::Down),
                self.pad_dir(stage, Pad::Left, Stick::Left),
                self.pad_dir(stage, Pad::Right, Stick::Right),
            );
            let mut keys = Vec::new();
            let held = up.1 || down.1 || left.1 || right.1;
            let fresh = up.0 || down.0 || left.0 || right.0;
            self.pad.repeat -= dt;
            if fresh || (held && self.pad.repeat <= 0.0) {
                self.pad.repeat = if fresh { REPEAT_FIRST } else { REPEAT_NEXT };
                for ((_, on), k) in [
                    (up, KeyCode::ArrowUp),
                    (down, KeyCode::ArrowDown),
                    (left, KeyCode::ArrowLeft),
                    (right, KeyCode::ArrowRight),
                ] {
                    if on {
                        keys.push(k);
                        break;
                    }
                }
            }
            if pressed(Pad::South) {
                keys.push(KeyCode::Enter);
            }
            if pressed(Pad::East) || (pressed(Pad::Start) && self.screen == Screen::Paused) {
                keys.push(KeyCode::Escape);
            }
            if matches!(self.screen, Screen::Options { .. }) {
                if pressed(Pad::LeftBumper) {
                    keys.push(KeyCode::KeyQ);
                }
                if pressed(Pad::RightBumper) {
                    keys.push(KeyCode::Tab);
                }
            }
            for k in keys {
                self.key(k, true, false);
            }
            return;
        }

        if pressed(Pad::Start) {
            self.key(KeyCode::Escape, true, false);
            return;
        }
        let window = self.game.pointer_window_open();
        let pin = self.typing_pin();
        for (p, d) in [(Pad::Left, 1), (Pad::Up, 2), (Pad::Right, 3)] {
            if !pin && pressed(p) {
                self.digit(d);
            }
        }
        if pin {
            let step = pressed(Pad::Up) as u8 + 9 * pressed(Pad::Down) as u8;
            let side = pressed(Pad::Right) as u8 + 9 * pressed(Pad::Left) as u8;
            self.pad.digit = (self.pad.digit + step + side) % 10;
            if pressed(Pad::South) {
                self.digit(self.pad.digit);
            }
        }
        if pressed(Pad::RightBumper) {
            self.input.wheel += 1;
        }
        if pressed(Pad::LeftBumper) {
            self.input.wheel -= 1;
        }
        let (a_down, a_held) = (
            pressed(Pad::South),
            stage.input.held(&[Bind::Pad(Pad::South)]),
        );
        if window {
            if a_down {
                self.input.ui_pressed = true;
                self.input.ui_down = true;
            }
            if pad_used && !a_held && self.input.ui_down {
                self.input.ui_released = true;
                self.input.ui_down = false;
            }
            if pressed(Pad::East) {
                self.game.close_pointer_window();
            }
        }
        for (a, down) in edges {
            // Kassenfenster: A greift, B schliesst; beim PIN-Tippen gehoeren A und das Kreuz
            // der Ziffer (egal, welche Aktion darauf liegt)
            let on = |p: Pad| self.keys.of(a).contains(&Bind::Pad(p));
            let taken = (window && (on(Pad::South) || on(Pad::East)))
                || (pin && (on(Pad::South) || on(Pad::Down)))
                // am Automaten bewegt der Stick den Zeiger, gedreht wird mit LB/RB
                || (window
                    && matches!(
                        a,
                        Action::Left | Action::Right | Action::Forward | Action::Back
                    ));
            if !down || !taken {
                self.action(a, down);
            }
        }

        let sticks = *stage.input.sticks();
        let (wx, wy) = sticks.walk();
        let (lx, ly) = sticks.look();
        if window {
            let k = CURSOR_SPEED * dt;
            self.cursor.0 = (self.cursor.0 + wx * k).clamp(0.0, ui::W);
            self.cursor.1 = (self.cursor.1 - wy * k).clamp(0.0, ui::H);
            return;
        }
        self.input.stick = (wx, wy);
        if self.game.swiping() {
            // rechts schieben, gleich welcher Stick
            let x = if lx.abs() > wx.abs() { lx } else { wx };
            self.game.turn(x * SWIPE_TRAVEL / SWIPE_SECONDS * dt, 0.0);
        } else if lx != 0.0 || ly != 0.0 {
            // feiner in der Mitte
            let curve = |v: f32| v * v.abs();
            let k = self.settings.pad_sensitivity * dt;
            self.game
                .turn(curve(lx) * LOOK_YAW * k, -curve(ly) * LOOK_PITCH * k);
        }
    }

    /// (neu gedrueckt, gehalten)
    fn pad_dir(&self, stage: &Stage, p: Pad, s: Stick) -> (bool, bool) {
        let b = [Bind::Pad(p), Bind::Stick(s)];
        (stage.input.pressed(&b), stage.input.held(&b))
    }

    /// Vibration: Schrecken, Fangen, Herzschlag. `loud` = lautestes Geraeusch je Art.
    pub fn rumble(&mut self, stage: &mut Stage, dt: f32, sounds: &[(Sfx, f32)]) {
        self.pad.rumble = (self.pad.rumble - dt).max(0.0);
        let dying = self.game.dying.is_some();
        let caught = dying && !self.pad.was_dying;
        self.pad.was_dying = dying;
        let mut want = if caught { (1.0, 700) } else { (0.0, 0) };
        for (s, vol) in sounds {
            let r = match s {
                Sfx::Scare | Sfx::Shriek | Sfx::Sting if *vol > 0.3 => (0.8, 350),
                Sfx::Thud | Sfx::Rumble if *vol > 0.3 => (0.45, 250),
                Sfx::Heartbeat => (0.2, 90),
                _ => continue,
            };
            if r.0 > want.0 {
                want = r;
            }
        }
        let on = self.settings.rumble && stage.input.using_pad && self.pad.focused;
        if on && want.0 > 0.0 && (want.0 > 0.5 || self.pad.rumble <= 0.0) {
            self.pad.rumble = want.1 as f32 / 1000.0;
            stage.input.rumble(want.0, want.1);
        }
    }

    /// Hinweis fuer die PIN am Controller.
    pub fn draw_pin_digit(&mut self, pad: bool) {
        if pad && self.screen == Screen::Playing && self.typing_pin() {
            let s = format!("Kreuz: < {} >   A: tippen", self.pad.digit);
            self.ui.text_center(250.0, &s, ui::YELLOW, 1.0);
        }
    }

    // --- Umbelegen ---

    /// Belegung uebernehmen oder verwerfen (Esc, feste Tasten).
    pub fn finish_rebind(&mut self, a: Action, b: Bind) {
        self.rebinding = None;
        if controls::allowed(b) {
            controls::rebind(&mut self.keys, a, b);
            controls::save(&self.keys);
            self.hints_for = None;
            self.sfx(Sfx::Pickup);
        } else {
            self.sfx(Sfx::Error);
        }
    }

    pub fn activate_key_row(&mut self) {
        match hud::keys::row(self.sel) {
            hud::keys::KeyRow::Action(a) => self.rebinding = Some(a),
            hud::keys::KeyRow::Reset => {
                self.keys = controls::defaults();
                controls::save(&self.keys);
                self.hints_for = None;
            }
            hud::keys::KeyRow::Back => self.back(),
        }
    }
}
