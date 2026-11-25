//! Kiosk: Pfandbons gegen Items. Ein Bon pro Kauf; ist er mehr wert, verfaellt der Rest.
//! Solange das Fenster offen ist, gilt es wie ein Gespraech: Kunden bewegen sich nicht.
//!
//! Layout (480x270-Raster) steht hier, damit Treffer und Zeichnen (hud/kiosk.rs) gleich sind.

use super::super::belt_window::{inside, Rect};
use super::super::*;
use super::items::Item;
use crate::world::Aabb;

/// Ausgabe: so lange rutscht der Gegenstand aus dem Schacht in die Ablage (s)
pub const DISPENSE_SLIDE: f32 = 0.5;
/// Schonfrist nach dem Kauf, wie nach einem Gespraech (s)
const GRACE: f32 = 2.5;
pub const WINDOW: Rect = (40.0, 26.0, 400.0, 196.0);
pub const BONS: Rect = (50.0, 56.0, 110.0, 124.0);
pub const SHELF: Rect = (170.0, 56.0, 260.0, 124.0);
pub const ROW: f32 = 11.0;
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
    /// Zettel des Erbauers liegt obenauf (beim ersten Oeffnen je Run, siehe `NOTE`)
    pub note: bool,
}

/// Der Zettel an der Maschine (beim ersten Oeffnen in jedem Run zu lesen), Zeilen fuers Fenster.
pub const NOTE: [&str; 5] = [
    "Tag 5032.",
    "Ich schaffe es nicht lebendig hier raus.",
    "Aber ich habe diese Maschine geschaffen,",
    "die Gegenstände erzeugt, die anderen",
    "verlorenen Seelen helfen können.",
];

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
            self.hint("Die Maschine rührt sich nicht.");
            return;
        }
        self.player.vel = Vec3::ZERO;
        // hoechster Bon zuerst
        self.pfand.bons.sort_unstable_by(|a, b| b.cmp(a));
        let text = if self.pfand.bons.is_empty() {
            "Ohne Pfandbon erzeugt sie nichts."
        } else {
            kiosk_line(self.tier, self.pfand.bons.len())
        };
        // beim ersten Oeffnen in diesem Run liegt der Zettel des Erbauers obenauf
        let note = !self.pfand.note_read;
        self.pfand.kiosk = Some(KioskWindow {
            column: Column::Bons,
            bon: 0,
            item: 0,
            confirm: false,
            text: text.into(),
            mouse: (0.0, 0.0),
            prev: (true, true, true, true),
            note,
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
        // Zettel lesen: E, Klick oder Q legt ihn weg, erst dann geht es zur Maschine
        if w.note {
            w.prev = held;
            if input.interact || input.ui_pressed || input.cart_toggle {
                w.note = false;
                self.pfand.note_read = true;
                self.play(Sfx::Click, 0.3);
            }
            return;
        }
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
        // der Scooter kommt nicht in die Ablage, er steht neben der Maschine
        if self.pfand.tray.is_some() && item != Item::Scooter {
            w.text = "Erst die Ablage leeren.".into();
            w.confirm = false;
            self.play(Sfx::Error, 0.4);
            return;
        }
        if self.pfand.bons.get(w.bon).is_none() {
            w.text = "Ohne Pfandbon gibt es nichts.".into();
            self.play(Sfx::Error, 0.4);
            return;
        }
        let sel = w.bon;
        let pay = self.bons_for(item, sel);
        let w = self.pfand.kiosk.as_mut().expect("Fenster offen");
        let Some((used, rest)) = pay else {
            w.text = if item == Item::Scooter {
                let all: i64 = self.pfand.bons.iter().sum();
                format!(
                    "Alle Bons zusammen: {}. Reicht nicht.",
                    crate::ui::euro(all)
                )
            } else {
                "Der Bon reicht nicht.".into()
            };
            w.confirm = false;
            self.play(Sfx::Error, 0.4);
            return;
        };
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
            w.confirm = true;
            let several = if used.len() > 1 {
                format!("{} Bons zusammen. ", used.len())
            } else {
                String::new()
            };
            w.text = if rest > 0 {
                format!(
                    "{several}Rest verfällt: {}. Noch mal E.",
                    crate::ui::euro(rest)
                )
            } else {
                format!("{several}Passt genau. Noch mal E.")
            };
            self.play(Sfx::Blip, 0.4);
            return;
        }
        // genau diese Bons sind weg, auch wenn sie mehr wert waren (von hinten entfernen)
        let mut used = used;
        used.sort_unstable_by(|a, b| b.cmp(a));
        for k in used {
            self.pfand.bons.remove(k);
        }
        self.pfand.bought = true;
        self.close_kiosk();
        self.play(Sfx::Rattle, 0.6);
        self.play(Sfx::Thud, 0.35);
        if item == Item::Scooter {
            // steht sofort neben der Maschine (scooter.rs)
            self.receive_item(item);
            return;
        }
        // die Maschine rumpelt und spuckt es in die Ablage (collect_kiosk_draws); das Fenster
        // geht zu, damit man es herausrutschen sieht und mit E nehmen kann
        self.pfand.tray = Some(item);
        self.pfand.dispensed = Some(self.time);
        self.say(
            &format!(
                "Die Maschine spuckt es aus: {}. Nimm es aus der Ablage.",
                item.name()
            ),
            3.5,
        );
    }

    /// Mit welchen Bons `item` bezahlt wird (Index in `pfand.bons`) und wie viel dabei
    /// verfaellt. Sonst ein Bon pro Kauf (der gewaehlte); nur fuer den Scooter (10 €) zaehlen
    /// alle Bons zusammen, die groessten zuerst. None: reicht nicht.
    pub fn bons_for(&self, item: Item, sel: usize) -> Option<(Vec<usize>, i64)> {
        let bons = &self.pfand.bons;
        if item != Item::Scooter {
            let &bon = bons.get(sel)?;
            return (bon >= item.cents()).then(|| (vec![sel], bon - item.cents()));
        }
        let mut order: Vec<usize> = (0..bons.len()).collect();
        order.sort_by(|&a, &b| bons[b].cmp(&bons[a]));
        let mut sum = 0;
        let mut used = Vec::new();
        for k in order {
            if sum >= item.cents() {
                break;
            }
            sum += bons[k];
            used.push(k);
        }
        (sum >= item.cents()).then(|| (used, sum - item.cents()))
    }

    /// Ablage vor dem Schacht: wo das Gekaufte liegt (zum Anschauen etwas grosszuegiger).
    pub(in crate::game) fn tray_box(&self) -> Option<Aabb> {
        let k = self.world.kiosk.as_ref()?;
        let front = k.counter.x + world::MACHINE_HALF.x + 2.0 * world::CHUTE_HALF.x;
        let c = vec3(
            front + 0.08,
            world::CHUTE_Y - world::CHUTE_HALF.y + 0.08,
            k.counter.z,
        );
        let h = vec3(0.14, 0.12, 0.22);
        Some(Aabb {
            min: c - h,
            max: c + h,
        })
    }

    /// Ablage, auf die man schaut, solange etwas darin liegt.
    pub(in crate::game) fn tray_hit(&self, eye: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        self.pfand.tray?;
        self.tray_box()?.ray(eye, dir, max)
    }

    /// E an der Ablage: Gekauftes nehmen.
    pub(in crate::game) fn take_from_tray(&mut self) {
        let Some(item) = self.pfand.tray else {
            return;
        };
        if let Some(why) = self.item_refused(item) {
            self.hint(why);
            self.play(Sfx::Error, 0.3);
            return;
        }
        self.pfand.tray = None;
        self.receive_item(item);
        self.play(Sfx::Pickup, 0.5);
        let how = if item.consumable() {
            "In der Tasche, G benutzt es."
        } else {
            "Wirkt ab jetzt."
        };
        self.say(&format!("{} genommen. {how}", item.name()), 3.0);
    }

    /// Kiosk, auf den man schaut.
    pub(in crate::game) fn kiosk_hit(&self, eye: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        let k = self.world.kiosk.as_ref()?;
        k.body().ray(eye, dir, max)
    }

    /// Rostige Maschine (immer) und daneben das Skelett ihres Erbauers (nur gekippt).
    /// Die Maschine schaut mit der Vorderseite in den Markt (+x); laeuft sie, glimmt sie.
    pub(in crate::game) fn collect_kiosk_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some(k) = self.world.kiosk.as_ref() else {
            return;
        };
        let dim = if self.tipped { 0.8 } else { 1.0 };
        let lift = |p: Vec3| p + vec3(0.0, self.sway_at(p).y, 0.0);
        let running = self.kiosk_has_open();
        out.push(MeshDraw {
            mesh: MeshId::Machine as usize,
            model: Mat4::from_translation(lift(k.counter)),
            tint: Vec4::new(dim, dim * 0.95, dim * 0.9, if running { 0.06 } else { 0.0 }),
            blink: if running { 0.15 } else { 0.0 },
            ..Default::default()
        });
        // Ausgabe: nach dem Kauf rutscht das Gekaufte aus dem Schacht und liegt in der Ablage,
        // bis man es nimmt (leuchtet, wenn man es anschaut)
        if let Some(item) = self.pfand.tray {
            let age = self
                .pfand
                .dispensed
                .map_or(DISPENSE_SLIDE, |at| self.time - at);
            let front = k.counter.x + world::MACHINE_HALF.x + 2.0 * world::CHUTE_HALF.x;
            let s = (age / DISPENSE_SLIDE).clamp(0.0, 1.0);
            let from = vec3(front - 0.08, world::CHUTE_Y + 0.02, k.counter.z);
            let to = vec3(
                front + 0.07,
                world::CHUTE_Y - world::CHUTE_HALF.y + 0.04,
                k.counter.z,
            );
            let p = from.lerp(to, s * s);
            let glow = if self.focus == Some(Focus::Tray) {
                0.45 + 0.3 * (0.5 + 0.5 * (self.time * 5.0).sin())
            } else {
                0.25
            };
            out.push(MeshDraw {
                mesh: MeshId::Can as usize,
                model: Mat4::from_rotation_translation(
                    Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * s),
                    lift(p),
                ),
                tint: item.tint().extend(glow),
                ..Default::default()
            });
        }
        if self.tipped {
            out.push(MeshDraw {
                mesh: MeshId::Skeleton as usize,
                model: Mat4::from_rotation_translation(
                    Quat::from_rotation_y(k.skeleton_yaw),
                    lift(k.skeleton),
                ),
                tint: Vec4::new(0.9, 0.88, 0.8, 0.0),
                ..Default::default()
            });
        }
    }
}

/// Was die Maschine anzeigt, je Stufe.
fn kiosk_line(tier: Tier, n: usize) -> &'static str {
    match tier {
        Tier::Calm => "Die Maschine brummt. Pfandbon einlegen.",
        Tier::Uneasy if n > 1 => "Sie zählt deine Bons. Es sind zu viele.",
        Tier::Uneasy => "Die Maschine kennt dich schon.",
        Tier::Wrong => "Etwas in der Maschine atmet.",
        Tier::Collapse => "Sie flüstert: Nimm mich mit.",
    }
}
