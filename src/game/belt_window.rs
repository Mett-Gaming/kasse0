//! Kassenfenster: E am Kassenband oeffnet ein eigenes Fenster (Markt dahinter abgedunkelt).
//! Links liegt der Inhalt des Wagens (und was man in der Hand hat), in der Mitte das Band,
//! rechts der Kassenbon. Waren zieht man mit gedrueckter Maustaste vom Wagen aufs Band; dabei
//! gelten dieselben Regeln wie sonst (Getraenke zuerst ...). E oder Esc schliesst das Fenster.
//! Schaut die Kassiererin auf, muss die Maus still bleiben (till/pressure.rs).
//!
//! Das Layout (im 480x270-Raster des HUD) steht hier, damit Treffer-Tests und Zeichnen
//! (hud/belt_window.rs) dieselben Rechtecke benutzen.

use super::*;

/// Rechteck im HUD-Raster: x, y, Breite, Hoehe
pub type Rect = (f32, f32, f32, f32);

pub const WINDOW: Rect = (16.0, 14.0, 448.0, 142.0);
/// Wagen links, Band in der Mitte, Kasse/Bon rechts
pub const CART_PANEL: Rect = (24.0, 32.0, 150.0, 118.0);
pub const BELT_PANEL: Rect = (182.0, 32.0, 150.0, 118.0);
pub const TILL_PANEL: Rect = (340.0, 32.0, 116.0, 118.0);
/// Kacheln im Wagen: zwei Spalten
const TILE_W: f32 = 71.0;
const TILE_H: f32 = 15.0;
const TILE_GAP: f32 = 3.0;
pub const TILE_ROWS: usize = 5;
/// "In der Hand" unten im Wagen-Feld
pub const HAND_TILE: Rect = (28.0, 132.0, 142.0, 14.0);
/// So viel Mausbewegung pro Bild (HUD-Pixel) zaehlt beim Aufschauen als "bewegt"
pub const MOUSE_STILL: f32 = 1.0;

/// Was man gerade zieht.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragSrc {
    Cart(usize),
    Hand,
}

#[derive(Clone, Debug, Default)]
pub struct BeltWindow {
    pub drag: Option<DragSrc>,
    /// Mauszeiger (HUD-Raster) und wie weit er sich im letzten Bild bewegt hat
    pub mouse: (f32, f32),
    pub moved: f32,
}

/// Kachel `i` des Wagens (oder None, wenn sie nicht mehr ins Feld passt).
pub fn cart_tile(i: usize) -> Option<Rect> {
    if i >= TILE_ROWS * 2 {
        return None;
    }
    let (px, py, _, _) = CART_PANEL;
    let col = (i % 2) as f32;
    let row = (i / 2) as f32;
    Some((
        px + 4.0 + col * (TILE_W + TILE_GAP),
        py + 14.0 + row * (TILE_H + TILE_GAP),
        TILE_W,
        TILE_H,
    ))
}

pub fn inside(r: Rect, p: (f32, f32)) -> bool {
    p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3
}

impl Game {
    pub fn belt_window_open(&self) -> bool {
        self.belt_window.is_some()
    }

    /// E am Band: Fenster oeffnen, wenn es etwas aufzulegen gibt.
    pub(super) fn open_belt_window(&mut self) {
        if self.paid {
            self.hint("Du hast schon bezahlt.");
            return;
        }
        if self.held.is_none() && self.cart.is_empty() {
            self.hint("Dein Wagen ist leer.");
            return;
        }
        if self.held.is_none() && !self.cart_at_register() {
            self.say("Bring deinen Wagen zur Kasse.", 3.0);
            self.play(Sfx::Error, 0.5);
            return;
        }
        self.belt_window = Some(BeltWindow::default());
        self.player.vel = Vec3::ZERO;
        self.play(Sfx::Click, 0.4);
    }

    pub fn close_belt_window(&mut self) {
        if self.belt_window.take().is_some() {
            self.play(Sfx::Click, 0.3);
        }
    }

    /// Fenster bedienen: ziehen, ablegen, schliessen. Laeuft statt der normalen Steuerung.
    pub(super) fn update_belt_window(&mut self, input: &Input) {
        self.player.vel = Vec3::ZERO;
        // beim Bezahlen oder wenn alles vorbei ist, schliesst es sich
        if input.interact || self.paid || self.till_paying() || self.outcome != Outcome::Running {
            self.close_belt_window();
            return;
        }
        let Some(w) = &mut self.belt_window else {
            return;
        };
        let m = input.ui_mouse;
        w.moved = ((m.0 - w.mouse.0).powi(2) + (m.1 - w.mouse.1).powi(2)).sqrt();
        w.mouse = m;
        if input.ui_pressed && w.drag.is_none() {
            w.drag = if inside(HAND_TILE, m) && self.held.is_some() {
                Some(DragSrc::Hand)
            } else {
                (0..self.cart.len())
                    .find(|&i| cart_tile(i).is_some_and(|r| inside(r, m)))
                    .map(DragSrc::Cart)
            };
            if w.drag.is_some() {
                self.play(Sfx::Pickup, 0.3);
            }
        }
        let Some(w) = &mut self.belt_window else {
            return;
        };
        if input.ui_released || !input.ui_down {
            if let Some(src) = w.drag.take() {
                if inside(BELT_PANEL, m) {
                    self.window_drop(src);
                }
            }
        }
    }

    /// Ware aus dem Wagen (bzw. der Hand) aufs Band legen. Regeln wie beim Auflegen per E.
    pub(super) fn window_drop(&mut self, src: DragSrc) {
        let item = match src {
            DragSrc::Hand => self.held.take(),
            DragSrc::Cart(i) if i < self.cart.len() => {
                if !self.cart_at_register() {
                    self.say("Bring deinen Wagen zur Kasse.", 3.0);
                    return;
                }
                self.cart_sel = None;
                Some(self.cart.remove(i))
            }
            DragSrc::Cart(_) => None,
        };
        if let Some(item) = item {
            self.put_on_belt(item);
        }
        // nichts mehr aufzulegen: Fenster zu
        if self.cart.is_empty() && self.held.is_none() {
            self.close_belt_window();
        }
    }

    /// Hat man waehrend des Aufschauens die Maus bewegt oder gezogen? (pressure.rs)
    pub fn window_mouse_busy(&self) -> bool {
        self.belt_window
            .as_ref()
            .is_some_and(|w| w.moved > MOUSE_STILL || w.drag.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::tests::{look_at_belt, press_e, shelf_item};

    fn at_belt(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        for e in g.list.clone() {
            let i = shelf_item(&g, e, false).unwrap();
            g.take(i);
        }
        g.customers.clear();
        g.employees.clear();
        let lane = g.open_lane().unwrap();
        g.trolley.pushing = false;
        g.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
        look_at_belt(&mut g);
        g
    }

    fn mouse(g: &mut Game, p: (f32, f32), down: bool, pressed: bool, released: bool) {
        let mut inp = Input {
            ui_mouse: p,
            ui_down: down,
            ui_pressed: pressed,
            ui_released: released,
            ..Default::default()
        };
        g.update(1.0 / 60.0, &mut inp);
    }

    fn center(r: Rect) -> (f32, f32) {
        (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5)
    }

    #[test]
    fn e_at_the_belt_opens_the_window_and_e_closes_it() {
        let mut g = at_belt(101);
        assert_eq!(g.focus, Some(Focus::Belt));
        press_e(&mut g);
        assert!(g.belt_window_open(), "E am Band oeffnet kein Fenster");
        assert!(g.belt.is_empty(), "E allein legt schon etwas aufs Band");
        // im Fenster bewegt man sich nicht
        let p = g.player.pos;
        let mut inp = Input {
            forward: true,
            ..Default::default()
        };
        for _ in 0..30 {
            g.update(1.0 / 60.0, &mut inp);
        }
        assert!(
            (g.player.pos - p).length() < 1e-3,
            "man laeuft im Fenster weg"
        );
        press_e(&mut g);
        assert!(!g.belt_window_open());
    }

    #[test]
    fn dragging_a_cart_item_onto_the_belt_puts_it_there() {
        let mut g = at_belt(102);
        press_e(&mut g);
        let n = g.cart.len();
        let drink = g.cart.iter().position(|c| c.is_drink()).unwrap();
        let from = center(cart_tile(drink).unwrap());
        mouse(&mut g, from, true, true, false);
        assert_eq!(
            g.belt_window.as_ref().unwrap().drag,
            Some(DragSrc::Cart(drink))
        );
        mouse(&mut g, center(BELT_PANEL), true, false, false);
        mouse(&mut g, center(BELT_PANEL), false, false, true);
        assert_eq!(g.cart.len(), n - 1);
        assert_eq!(g.belt.len(), 1, "Ware liegt nicht auf dem Band");
        assert!(g.belt[0].data.is_drink());
        // daneben loslassen: nichts passiert
        let from = center(cart_tile(0).unwrap());
        mouse(&mut g, from, true, true, false);
        mouse(&mut g, (5.0, 260.0), false, false, true);
        assert_eq!(g.cart.len(), n - 1);
        assert_eq!(g.belt.len(), 1);
    }

    #[test]
    fn rules_still_apply_in_the_window() {
        let mut g = at_belt(103);
        press_e(&mut g);
        // erst etwas anderes, dann ein Getraenk: ein Leben weg
        let other = g.cart.iter().position(|c| !c.is_drink()).unwrap();
        g.window_drop(DragSrc::Cart(other));
        let drink = g.cart.iter().position(|c| c.is_drink()).unwrap();
        g.window_drop(DragSrc::Cart(drink));
        assert_eq!(g.till.as_ref().unwrap().lives, till::TILL_LIVES - 1);
    }

    #[test]
    fn moving_the_mouse_while_she_stares_is_noticed() {
        let mut g = at_belt(104);
        press_e(&mut g);
        // eine Ware auflegen, damit das Minispiel laeuft; Druck an, sie starrt
        let drink = g.cart.iter().position(|c| c.is_drink()).unwrap();
        g.window_drop(DragSrc::Cart(drink));
        g.calm_till = false;
        // Zeiger steht schon an seiner Stelle (sonst zaehlt der erste Sprung als Bewegung)
        g.belt_window.as_mut().unwrap().mouse = (100.0, 100.0);
        let t = g.till.as_mut().unwrap();
        t.look = Some(till::pressure::LookUp {
            warn: 0.0,
            stare: 2.0,
            caught: false,
        });
        mouse(&mut g, (100.0, 100.0), false, false, false);
        let before = g.till.as_ref().unwrap().doubt;
        // stillhalten: nichts
        for _ in 0..20 {
            mouse(&mut g, (100.0, 100.0), false, false, false);
        }
        let still = g.till.as_ref().unwrap().doubt - before;
        assert!(still < 0.05, "Stillhalten faellt auf: {still}");
        // Maus bewegen: faellt auf
        mouse(&mut g, (140.0, 120.0), false, false, false);
        let moved = g.till.as_ref().unwrap().doubt - before;
        assert!(
            moved >= till::pressure::SUS_STARE,
            "Mausbewegung faellt nicht auf"
        );
    }
}
