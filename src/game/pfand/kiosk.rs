//! Kiosk: Pfandbons gegen Items. Ein Bon pro Kauf; ist er mehr wert, verfaellt der Rest.
//! Solange das Fenster offen ist, gilt es wie ein Gespraech: Kunden bewegen sich nicht.
//!
//! Layout (480x270-Raster) steht hier, damit Treffer und Zeichnen (hud/kiosk.rs) gleich sind.

use super::super::belt_window::{inside, Rect};
use super::super::*;
use super::items::Item;

/// Schonfrist nach dem Kauf, wie nach einem Gespraech (s)
const GRACE: f32 = 2.5;
pub const WINDOW: Rect = (40.0, 26.0, 400.0, 196.0);
pub const BONS: Rect = (50.0, 56.0, 110.0, 124.0);
pub const SHELF: Rect = (170.0, 56.0, 260.0, 124.0);
pub const ROW: f32 = 12.0;
pub const BUY: Rect = (330.0, 198.0, 100.0, 16.0);
/// So viele Bons sind zu sehen (die Auswahl blaettert mit)
pub const BON_ROWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Bons,
    Items,
}

#[derive(Clone, Debug)]
pub struct KioskWindow {
    pub column: Column,
    pub bon: usize,
    pub item: usize,
    /// noch einmal E kauft
    pub confirm: bool,
    pub text: String,
    pub mouse: (f32, f32),
    prev: (bool, bool, bool, bool),
}

/// Erste sichtbare Zeile der Bons.
pub fn first_bon(sel: usize) -> usize {
    sel.saturating_sub(BON_ROWS - 1)
}

impl KioskWindow {
    /// Zeile unter dem Zeiger.
    fn hit(&self, bons: usize) -> Option<(Column, usize)> {
        let m = self.mouse;
        let row = |r: Rect| ((m.1 - r.1 - 14.0) / ROW).floor();
        if inside(BONS, m) {
            let k = row(BONS);
            let i = first_bon(self.bon) as f32 + k;
            return (k >= 0.0 && k < BON_ROWS as f32 && i < bons as f32)
                .then_some((Column::Bons, i as usize));
        }
        if inside(SHELF, m) {
            let k = row(SHELF);
            return (k >= 0.0 && k < Item::ALL.len() as f32).then_some((Column::Items, k as usize));
        }
        None
    }
}

impl Game {
    pub fn kiosk_open(&self) -> bool {
        self.pfand.kiosk.is_some()
    }

    /// Gespraech mit einem Mitarbeiter oder am Kiosk: Kunden stehen, niemand wird gefangen.
    pub fn in_talk(&self) -> bool {
        self.dialog.is_some() || self.kiosk_open()
    }

    /// Hat der Kiosk gerade auf? (nur im gekippten Markt, nicht auf der Flucht)
    pub fn kiosk_has_open(&self) -> bool {
        self.tipped && !self.paid && self.world.kiosk.is_some()
    }

    pub(in crate::game) fn open_kiosk(&mut self) {
        if !self.kiosk_has_open() {
            self.hint("Gleich geöffnet.");
            return;
        }
        self.player.vel = Vec3::ZERO;
        // hoechster Bon zuerst
        self.pfand.bons.sort_unstable_by(|a, b| b.cmp(a));
        let text = if self.pfand.bons.is_empty() {
            "Ohne Pfandbon gibt es nichts."
        } else {
            kiosk_line(self.tier, self.pfand.bons.len())
        };
        self.pfand.kiosk = Some(KioskWindow {
            column: Column::Bons,
            bon: 0,
            item: 0,
            confirm: false,
            text: text.into(),
            mouse: (0.0, 0.0),
            prev: (true, true, true, true),
        });
        self.play(Sfx::Blip, 0.4);
    }

    pub fn close_kiosk(&mut self) {
        if self.pfand.kiosk.take().is_some() {
            self.grace = self.grace.max(GRACE);
            self.play(Sfx::Click, 0.3);
        }
    }

    pub(in crate::game) fn update_kiosk(&mut self, input: &Input) {
        self.player.vel = Vec3::ZERO;
        let bons = self.pfand.bons.len();
        let Some(w) = &mut self.pfand.kiosk else {
            return;
        };
        w.mouse = input.ui_mouse;
        let held = (input.left, input.right, input.forward, input.back);
        let edge = |now: bool, before: bool| now && !before;
        let side = edge(held.0, w.prev.0) || edge(held.1, w.prev.1);
        let step = edge(held.3, w.prev.3) as i32 - edge(held.2, w.prev.2) as i32 - input.wheel;
        w.prev = held;
        if side {
            w.column = match w.column {
                Column::Bons => Column::Items,
                Column::Items => Column::Bons,
            };
            w.confirm = false;
        }
        if step != 0 {
            let (sel, n) = match w.column {
                Column::Bons => (&mut w.bon, bons),
                Column::Items => (&mut w.item, Item::ALL.len()),
            };
            if n > 0 {
                *sel = (*sel as i32 + step).rem_euclid(n as i32) as usize;
            }
            w.confirm = false;
        }
        let mut go = input.interact;
        if input.ui_pressed {
            if inside(BUY, w.mouse) {
                w.column = Column::Items;
                go = true;
            } else if let Some((c, i)) = w.hit(bons) {
                w.column = c;
                match c {
                    Column::Bons => {
                        w.bon = i;
                        w.confirm = false;
                    }
                    Column::Items => {
                        if w.item != i {
                            w.confirm = false;
                        }
                        w.item = i;
                        go = true;
                    }
                }
            }
        }
        if input.cart_toggle {
            self.close_kiosk();
            return;
        }
        if go {
            self.kiosk_choose();
        }
    }

    /// E: Bon gewaehlt -> zu den Items; Item gewaehlt -> nachfragen, dann kaufen.
    pub(in crate::game) fn kiosk_choose(&mut self) {
        let Some(w) = &mut self.pfand.kiosk else {
            return;
        };
        if w.column == Column::Bons {
            w.column = Column::Items;
            w.confirm = false;
            return;
        }
        let item = Item::ALL[w.item.min(Item::ALL.len() - 1)];
        let Some(&bon) = self.pfand.bons.get(w.bon) else {
            w.text = "Ohne Pfandbon gibt es nichts.".into();
            self.play(Sfx::Error, 0.4);
            return;
        };
        if bon < item.cents() {
            w.text = "Der Bon reicht nicht.".into();
            w.confirm = false;
            self.play(Sfx::Error, 0.4);
            return;
        }
        if let Some(why) = self.item_refused(item) {
            if let Some(w) = &mut self.pfand.kiosk {
                w.text = why.into();
                w.confirm = false;
            }
            self.play(Sfx::Error, 0.4);
            return;
        }
        let w = self.pfand.kiosk.as_mut().expect("Fenster offen");
        if !w.confirm {
            let rest = bon - item.cents();
            w.confirm = true;
            w.text = if rest > 0 {
                format!("Rest verfällt: {}. Noch mal E.", crate::ui::euro(rest))
            } else {
                "Passt genau. Noch mal E.".into()
            };
            self.play(Sfx::Blip, 0.4);
            return;
        }
        // genau dieser eine Bon ist weg, auch wenn er mehr wert war
        let k = w.bon;
        w.confirm = false;
        w.text = format!("{}. Bitte sehr.", item.name());
        w.bon = 0;
        self.pfand.bons.remove(k);
        self.pfand.bought = true;
        self.receive_item(item);
        self.play(Sfx::Beep, 0.5);
        self.play(Sfx::Pickup, 0.4);
    }

    /// Kiosk, auf den man schaut.
    pub(in crate::game) fn kiosk_hit(&self, eye: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        let k = self.world.kiosk.as_ref()?;
        k.body().ray(eye, dir, max)
    }

    /// Verkaeufer hinter dem Tresen.
    pub(in crate::game) fn kiosk_seller(&self) -> Option<(usize, Vec3, f32)> {
        if !self.kiosk_has_open() {
            return None;
        }
        let k = self.world.kiosk.as_ref()?;
        let staff = crate::people::people_of(true);
        let v = staff[(self.seed as usize / 5 + 1) % staff.len()];
        Some((v, k.seller, FRAC_PI_2))
    }
}

use std::f32::consts::FRAC_PI_2;

/// Begruessung, je Stufe.
fn kiosk_line(tier: Tier, n: usize) -> &'static str {
    match tier {
        Tier::Calm => "Guten Tag. Pfandbon oder gehen Sie.",
        Tier::Uneasy if n > 1 => "So viele Bons. Was haben Sie getan?",
        Tier::Uneasy => "Sie waren schon hier.",
        Tier::Wrong => "Ich habe keinen Feierabend.",
        Tier::Collapse => "Nehmen Sie mich mit.",
    }
}
