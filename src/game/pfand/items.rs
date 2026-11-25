//! Was es am Kiosk gibt und was es bewirkt. Gilt nur im laufenden Run.

use super::super::*;

/// So viel Verbrauchbares passt in die Tasche
pub const POCKET_MAX: usize = 4;
/// Energy-Drink: so lange kostet Rennen keine Ausdauer (s)
pub const ENERGY_TIME: f32 = 20.0;
/// Wunderkerze brennt so lange (s)
pub const SPARKLE_TIME: f32 = 15.0;
/// Tee: Auffaelligkeit mal so viel
const TEA: f32 = 0.5;
/// Filzsohlen: Schritte so weit hoerbar (Anteil)
pub const SOLES: f32 = 0.5;
/// Gloeckchen: Verfolger so nah hinter einem (m)
pub const BELL_RANGE: f32 = 6.0;
/// Sekunden zwischen zwei Klingeln
const BELL_EVERY: f32 = 1.4;
/// Mettbroetchen: Ausdauer reicht so viel mal laenger
pub const METT_STAMINA: f32 = 2.0;
/// Taschenlampe: Licht an der Kamera mal so viel
pub const TORCH: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Sticker,
    Energy,
    Sparkler,
    Tea,
    Oil,
    Mett,
    Soles,
    Bell,
    Torch,
    /// Mobility-Scooter (scooter.rs): ersetzt den Wagen, schneller, 2 Leben mehr, Licht
    Scooter,
}

impl Item {
    /// Nach Preis geordnet
    pub const ALL: [Item; 10] = [
        Item::Sticker,
        Item::Energy,
        Item::Sparkler,
        Item::Tea,
        Item::Oil,
        Item::Mett,
        Item::Soles,
        Item::Bell,
        Item::Torch,
        Item::Scooter,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Item::Sticker => "Sticker Kunde 0",
            Item::Energy => "Energy-Drink",
            Item::Sparkler => "Wunderkerze",
            Item::Tea => "Beruhigungstee",
            Item::Oil => "Öl",
            Item::Mett => "Mettbrötchen",
            Item::Soles => "Filzsohlen",
            Item::Bell => "Glöckchen",
            Item::Torch => "Taschenlampe",
            Item::Scooter => "Mobility Scooter",
        }
    }

    /// Preis in Cent
    pub fn cents(self) -> i64 {
        match self {
            Item::Sticker => 15,
            Item::Energy => 40,
            Item::Sparkler => 50,
            Item::Tea => 75,
            Item::Oil => 100,
            Item::Mett => 120,
            Item::Soles => 125,
            Item::Bell => 150,
            Item::Torch => 250,
            Item::Scooter => 1000,
        }
    }

    pub fn about(self) -> &'static str {
        match self {
            Item::Sticker => "Klebt am Wagen. Sonst nichts.",
            Item::Energy => "20 Sekunden rennen ohne Ausdauer.",
            Item::Sparkler => "Funkelt 15 Sekunden. Völlig nutzlos.",
            Item::Tea => "Halbiert deine Auffälligkeit.",
            Item::Oil => "Der Wagen quietscht nicht mehr.",
            Item::Mett => "Doppelte Ausdauer bis Einkaufsende.",
            Item::Soles => "Schritte nur halb so weit hörbar.",
            Item::Bell => "Klingelt, wenn hinter dir einer kommt.",
            Item::Torch => "Doppelt so hell im dunklen Markt.",
            Item::Scooter => "Du bist jetzt fett.",
        }
    }

    /// Farbe der Dose, in der es aus der Maschine kommt (kiosk.rs)
    pub fn tint(self) -> Vec3 {
        match self {
            Item::Sticker => vec3(0.95, 0.85, 0.2),
            Item::Energy => vec3(0.3, 0.9, 0.35),
            Item::Sparkler => vec3(0.95, 0.6, 0.2),
            Item::Tea => vec3(0.7, 0.85, 0.6),
            Item::Oil => vec3(0.55, 0.45, 0.2),
            Item::Mett => vec3(0.9, 0.45, 0.4),
            Item::Soles => vec3(0.5, 0.45, 0.55),
            Item::Bell => vec3(0.95, 0.8, 0.35),
            Item::Torch => vec3(0.85, 0.85, 0.95),
            Item::Scooter => vec3(0.62, 0.12, 0.12),
        }
    }

    /// Kurzer Name fuer die Anzeige, was man dabei hat (hud.rs)
    pub fn short(self) -> &'static str {
        match self {
            Item::Sticker => "Sticker",
            Item::Oil => "Öl",
            Item::Soles => "Sohlen",
            Item::Bell => "Glöckchen",
            Item::Torch => "Lampe",
            Item::Scooter => "Scooter",
            other => other.name(),
        }
    }

    /// Kommt in die Tasche und wird mit G benutzt?
    pub fn consumable(self) -> bool {
        matches!(self, Item::Energy | Item::Sparkler | Item::Tea | Item::Mett)
    }
}

/// Gekauftes in diesem Run.
#[derive(Clone, Debug, Default)]
pub struct Owned {
    pub pocket: Vec<Item>,
    pub sel: usize,
    /// dauerhaft Wirkendes
    pub kept: Vec<Item>,
    /// Restzeiten (s)
    pub energy: f32,
    pub sparkle: f32,
    /// Mettbroetchen gegessen: Ausdauer reicht doppelt so lange (bis Einkaufsende)
    pub mett: bool,
    bell: f32,
}

impl Owned {
    pub fn has(&self, i: Item) -> bool {
        self.kept.contains(&i)
    }

    /// Was G benutzen wuerde.
    pub fn selected(&self) -> Option<Item> {
        self.pocket
            .get(self.sel.min(self.pocket.len().saturating_sub(1)))
            .copied()
    }
}

impl Game {
    /// Passt dieses Item noch? Sonst der Grund.
    pub(in crate::game) fn item_refused(&self, i: Item) -> Option<&'static str> {
        let o = &self.pfand.owned;
        if i.consumable() {
            (o.pocket.len() >= POCKET_MAX).then_some("Deine Taschen sind voll.")
        } else {
            o.has(i).then_some("Das hast du schon.")
        }
    }

    pub(in crate::game) fn receive_item(&mut self, i: Item) {
        let o = &mut self.pfand.owned;
        if i.consumable() {
            o.pocket.push(i);
            o.sel = o.pocket.len() - 1;
        } else {
            o.kept.push(i);
        }
        // der Scooter kommt nicht aus dem Schacht, er steht neben der Maschine (scooter.rs)
        if i == Item::Scooter {
            self.spawn_scooter();
        }
    }

    /// G: gewaehltes Item benutzen.
    pub(in crate::game) fn use_item(&mut self) {
        let Some(i) = self.pfand.owned.selected() else {
            self.hint("Du hast nichts in der Tasche.");
            return;
        };
        let o = &mut self.pfand.owned;
        let k = o.sel.min(o.pocket.len() - 1);
        o.pocket.remove(k);
        o.sel = k.min(o.pocket.len().saturating_sub(1));
        match i {
            Item::Energy => {
                o.energy = ENERGY_TIME;
                self.player.exhausted = false;
                self.play(Sfx::Gulp, 0.6);
                self.say("Herzklopfen. Du könntest ewig rennen.", 3.0);
            }
            Item::Sparkler => {
                o.sparkle = SPARKLE_TIME;
                self.play(Sfx::Lighter, 0.6);
                self.say("Sie funkelt. Schön.", 2.5);
            }
            Item::Tea => {
                self.suspicion *= TEA;
                self.play(Sfx::Gulp, 0.6);
                self.say("Kamille. Du wirkst wieder wie ein Kunde.", 3.0);
            }
            Item::Mett => {
                // wirkt einmal: ein zweites Broetchen macht nicht noch ausdauernder
                let again = o.mett;
                o.mett = true;
                self.play(Sfx::Munch, 0.7);
                if again {
                    self.say("Noch mehr Mett. Satter wirst du nicht.", 3.0);
                } else {
                    self.say("Mett mit Zwiebeln. Du hast doppelt so lange Puste.", 3.5);
                }
            }
            _ => {}
        }
    }

    /// Mausrad: anderes Item waehlen.
    pub(in crate::game) fn cycle_item(&mut self, steps: i32) {
        let o = &mut self.pfand.owned;
        let n = o.pocket.len() as i32;
        if n > 0 {
            o.sel = (o.sel as i32 + steps).rem_euclid(n) as usize;
        }
    }

    /// Zeiten ablaufen lassen, Gloeckchen.
    pub(in crate::game) fn update_items(&mut self, dt: f32) {
        // nachkommende Flaschen (eine je REFILL_EVERY, nie in der Naehe des Spielers)
        self.pfand.refill -= dt;
        if self.pfand.refill <= 0.0 {
            self.pfand.refill = super::REFILL_EVERY;
            let n = self.pfand.bottles.len();
            self.scatter_bottles_upto(n + 1);
        }
        let o = &mut self.pfand.owned;
        o.energy = (o.energy - dt).max(0.0);
        o.sparkle = (o.sparkle - dt).max(0.0);
        o.bell = (o.bell - dt).max(0.0);
        if !o.has(Item::Bell) || o.bell > 0.0 || self.in_talk() {
            return;
        }
        if self.stalker_behind() {
            self.pfand.owned.bell = BELL_EVERY;
            self.sfx_at.push((Sfx::Bell, 0.8, self.bag_center()));
        }
    }

    /// Ist ein Verfolger nah hinter einem?
    fn stalker_behind(&self) -> bool {
        let f = self.view_dir();
        self.customers.iter().any(|c| {
            let d = c.pos - self.player.pos;
            let flat = vec3(d.x, 0.0, d.z);
            c.stalker && flat.length() < BELL_RANGE && flat.dot(vec3(f.x, 0.0, f.z)) < 0.0
        })
    }

    /// Laeuft der Energy-Drink noch?
    pub(in crate::game) fn energized(&self) -> bool {
        self.pfand.owned.energy > 0.0
    }

    /// So viel mal laenger reicht die Ausdauer beim Rennen (Mettbroetchen).
    pub(in crate::game) fn stamina_factor(&self) -> f32 {
        if self.pfand.owned.mett {
            METT_STAMINA
        } else {
            1.0
        }
    }

    /// Licht an der Kamera im dunklen Markt (1 = ueblich).
    pub fn camera_light(&self) -> f32 {
        // der Scooter hat einen Scheinwerfer: so hell wie die Taschenlampe
        if self.pfand.owned.has(Item::Torch) || self.pfand.owned.has(Item::Scooter) {
            TORCH
        } else {
            1.0
        }
    }

    /// Spitze der brennenden Wunderkerze.
    pub fn sparkler_tip(&self) -> Option<Vec3> {
        if self.pfand.owned.sparkle <= 0.0 {
            return None;
        }
        let f = self.view_dir();
        let r = f.cross(Vec3::Y).normalize_or_zero();
        let u = r.cross(f);
        let burned = 1.0 - self.pfand.owned.sparkle / SPARKLE_TIME;
        Some(self.eye() + f * 0.42 - r * 0.17 - u * (0.04 + 0.12 * burned))
    }

    /// Wunderkerze in der Hand, Sticker und Gloeckchen am Wagen.
    pub(super) fn collect_item_draws(&self, out: &mut Vec<MeshDraw>) {
        let cube = MeshId::Cube as usize;
        if let Some(tip) = self.sparkler_tip() {
            let f = self.view_dir();
            let u = f.cross(Vec3::Y).normalize_or_zero().cross(f);
            let foot = tip - u * 0.16;
            let rot = Quat::from_rotation_arc(Vec3::Y, u.normalize_or(Vec3::Y));
            out.push(MeshDraw {
                mesh: cube,
                model: Mat4::from_scale_rotation_translation(
                    vec3(0.004, 0.16, 0.004),
                    rot,
                    (tip + foot) * 0.5,
                ),
                tint: Vec4::new(0.4, 0.4, 0.42, 0.1),
                ..Default::default()
            });
            out.push(MeshDraw {
                mesh: cube,
                model: Mat4::from_scale_rotation_translation(Vec3::splat(0.012), rot, tip),
                tint: Vec4::new(1.0, 0.9, 0.6, 3.0),
                ..Default::default()
            });
        }
        let m = Mat4::from_rotation_translation(
            Quat::from_rotation_y(-self.trolley.yaw),
            self.trolley.pos
                + vec3(
                    0.0,
                    self.sway_at(self.trolley.pos).y + self.cart_lift(),
                    0.0,
                ),
        );
        let mut part = |c: Vec3, size: Vec3, tint: Vec4| {
            out.push(MeshDraw {
                mesh: cube,
                model: m * Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, c),
                tint,
                ..Default::default()
            });
        };
        if self.pfand.owned.has(Item::Sticker) {
            let c = vec3(0.0, 0.72, 0.36);
            part(c, vec3(0.12, 0.07, 0.004), Vec4::new(0.95, 0.85, 0.2, 0.1));
            // die Null
            part(
                c + vec3(0.0, 0.0, 0.003),
                vec3(0.03, 0.045, 0.004),
                Vec4::new(0.1, 0.1, 0.12, 0.0),
            );
            part(
                c + vec3(0.0, 0.0, 0.005),
                vec3(0.014, 0.026, 0.004),
                Vec4::new(0.95, 0.85, 0.2, 0.1),
            );
        }
        if self.pfand.owned.has(Item::Bell) {
            let swing = (self.time * 9.0).sin() * 0.01;
            part(
                vec3(-0.22 + swing, 0.86, 0.36),
                vec3(0.035, 0.04, 0.035),
                Vec4::new(0.95, 0.75, 0.25, 0.15),
            );
        }
    }
}
