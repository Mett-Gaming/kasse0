//! Pfand: leere Flaschen und Dosen liegen im gekippten Markt. Sie kommen in eine Tuete, die am
//! Wagen haengt (hoechstens BAG_MAX), und werden an den Automaten im Seitentrakt abgegeben
//! (machine.rs). Dafuer gibt es Bons, jeder fuer sich einzuloesen.
//!
//! Pfand ist keine Ware: zaehlt nicht zum Warenwert, nicht zum Zettel und kommt nicht aufs Band.

pub mod items;
pub mod kiosk;
pub mod machine;
#[cfg(test)]
mod tests;

use super::*;
use crate::assets::{FIXED_COUNT, PRODUCT_MESH_IDS};
pub use machine::MachineWindow;

/// So viele Flaschen passen in die Tuete am Wagen
pub const BAG_MAX: usize = 5;
/// So viele Flaschen liegen im Markt (nach jedem Umbau wieder aufgefuellt)
pub const BOTTLES: usize = 14;
/// So oft (s) kommt eine neue Flasche dazu, solange weniger als `BOTTLES` herumliegen
/// (damit sich ueber den Einkauf genug fuer den Scooter, 10 €, sammeln laesst)
pub const REFILL_EVERY: f32 = 15.0;
/// davon hoechstens so viele im Seitentrakt
const IN_WING: usize = 3;
/// Mindestabstand zweier Flaschen (m)
const APART: f32 = 2.5;
/// Je Flasche in der Tuete hoert man den Wagen so viel weiter (m)
pub const CLINK_RADIUS: f32 = 1.0;

/// Sorte einer leeren Flasche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Empty {
    Glass,
    Reusable,
    Plastic,
    Can,
}

impl Empty {
    pub const ALL: [Empty; 4] = [Empty::Glass, Empty::Reusable, Empty::Plastic, Empty::Can];

    /// Pfand in Cent
    pub fn cents(self) -> i64 {
        match self {
            Empty::Glass => 8,
            Empty::Reusable => 15,
            Empty::Plastic | Empty::Can => 25,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Empty::Glass => "Leere Glasflasche",
            Empty::Reusable => "Leere Mehrwegflasche",
            Empty::Plastic => "Leere Plastikflasche",
            Empty::Can => "Leere Dose",
        }
    }

    fn mesh(self) -> usize {
        let id = match self {
            Empty::Glass => "bottle_q",
            Empty::Reusable => "water",
            Empty::Plastic => "soda_bottle",
            Empty::Can => "soda_can",
        };
        // steht im Katalog (Getraenke); sonst ein vorhandenes Modell
        let i = PRODUCT_MESH_IDS.iter().position(|m| *m == id).unwrap_or(0);
        FIXED_COUNT + i
    }

    pub fn item(self) -> ItemData {
        ItemData {
            name: self.name().into(),
            cents: self.cents(),
            kind: PickupKind::Deposit(self),
            mesh: self.mesh(),
            // ausgetrunken: blass
            tint: vec3(0.78, 0.82, 0.8),
        }
    }
}

/// Eine Flasche, die im Markt liegt.
#[derive(Clone, Copy, Debug)]
pub struct Bottle {
    pub pos: Vec3,
    pub yaw: f32,
    pub kind: Empty,
}

/// Alles zum Pfand in diesem Run.
pub struct Pfand {
    pub bottles: Vec<Bottle>,
    /// Tuete am Wagen
    pub bag: Vec<Empty>,
    /// gedruckte Bons (Cent)
    pub bons: Vec<i64>,
    /// noch nicht gedruckt: Betrag je Automat (Raum)
    pub sums: Vec<i64>,
    pub window: Option<MachineWindow>,
    pub kiosk: Option<kiosk::KioskWindow>,
    pub owned: items::Owned,
    /// schon einmal etwas gekauft (Tutorial)
    pub bought: bool,
    /// Blick auf die Tuete (statt auf den Wagen)
    pub aim_bag: bool,
    /// wie oft umgebaut wurde: andere Automaten laufen
    pub shift: usize,
    /// schon einmal einen Bon gedruckt (Tutorial)
    pub printed: bool,
    /// Spielzeit des letzten Kaufs: dann rutscht ein Gegenstand aus dem Schacht (kiosk.rs)
    pub dispensed: Option<f32>,
    /// Gekauftes, das in der Ablage der Maschine liegt, bis man es mit E nimmt (kiosk.rs)
    pub tray: Option<items::Item>,
    /// Zeit bis zur naechsten nachkommenden Flasche (s)
    pub refill: f32,
    /// Zettel an der Maschine in diesem Run schon gelesen (kiosk.rs)
    pub note_read: bool,
    /// Spielzeit, zu der zuletzt eine Flasche in die Tuete kam oder heraus (Nachwippen)
    pub bag_changed: f32,
    /// eigener Zufall, damit sich sonst nichts verschiebt
    pub(super) rng: Pcg64,
    hinted: bool,
}

impl Pfand {
    pub fn new(seed: u64) -> Self {
        Self {
            bottles: Vec::new(),
            bag: Vec::new(),
            bons: Vec::new(),
            sums: Vec::new(),
            window: None,
            kiosk: None,
            owned: Default::default(),
            bought: false,
            aim_bag: false,
            shift: 0,
            printed: false,
            note_read: false,
            bag_changed: -10.0,
            dispensed: None,
            tray: None,
            refill: REFILL_EVERY,
            rng: Pcg64::seed_from_u64(seed ^ 0x9FA2_D000),
            hinted: false,
        }
    }

    pub fn bag_full(&self) -> bool {
        self.bag.len() >= BAG_MAX
    }
}

impl Game {
    /// Flaschen bis zur vollen Zahl im Markt verteilen (erreichbare Zellen).
    pub(super) fn scatter_bottles(&mut self) {
        self.scatter_bottles_upto(BOTTLES);
    }

    /// Flaschen verteilen, bis `target` im Markt liegen (hoechstens `BOTTLES`).
    pub(super) fn scatter_bottles_upto(&mut self, target: usize) {
        let target = target.min(BOTTLES);
        if !self.tipped || self.paid {
            return;
        }
        let cells = self.world.reachable_centers();
        if cells.is_empty() {
            return;
        }
        // zwei gut erreichbare fuers Tutorial: nahe der Tuer zum Seitentrakt
        if self.tutorial_run && self.pfand.bottles.is_empty() {
            let door = self.world.wing.as_ref().and_then(|w| w.doors.first());
            if let Some(&d) = door {
                let mut near: Vec<Vec3> = cells
                    .iter()
                    .copied()
                    .filter(|c| c.x > 0.8 && (1.5..7.0).contains(&c.distance(d)))
                    .collect();
                near.sort_by(|a, b| a.distance(d).total_cmp(&b.distance(d)));
                for k in [0, near.len() / 2] {
                    if let Some(&pos) = near.get(k) {
                        self.pfand.bottles.push(Bottle {
                            pos,
                            yaw: k as f32,
                            kind: Empty::Plastic,
                        });
                    }
                }
            }
        }
        let mut tries = 0;
        while self.pfand.bottles.len() < target && tries < 600 {
            tries += 1;
            let p = cells[self.pfand.rng.gen_range(0..cells.len())];
            let in_wing = |q: Vec3| self.world.in_wing(q);
            let wing_count = self.pfand.bottles.iter().filter(|b| in_wing(b.pos)).count();
            // Trakt: hoechstens IN_WING; nicht in Kassenbereich, Getraenkeraum, vor der Nase
            let bad = (in_wing(p) && wing_count >= IN_WING)
                || self.world.drinks_zone.contains_xz(p)
                || self.world.in_checkout(p)
                || self.pfand.bottles.iter().any(|b| b.pos.distance(p) < APART)
                || p.distance(self.player.pos) < 4.0;
            if bad {
                continue;
            }
            let kind = Empty::ALL[self.pfand.rng.gen_range(0..Empty::ALL.len())];
            let yaw = self.pfand.rng.gen_range(0.0..std::f32::consts::TAU);
            self.pfand.bottles.push(Bottle { pos: p, yaw, kind });
        }
    }

    /// Nach einem Umbau: Flaschen mitnehmen, Ungedrucktes verfaellt, auffuellen.
    pub(super) fn pfand_after_rebuild(&mut self, old: &World) {
        self.close_machine();
        self.close_kiosk();
        self.pfand.shift += 1;
        let w = &self.world;
        let cells = w.reachable_centers();
        self.pfand.bottles.retain_mut(|b| {
            let p = w.collide(w.remap_from(old, b.pos), 0.25);
            let near = cells
                .iter()
                .copied()
                .min_by(|a, c| a.distance_squared(p).total_cmp(&c.distance_squared(p)));
            // in eine Wand geraten: zur naechsten freien Stelle, sonst weg
            match near {
                Some(n) if n.distance(p) < 0.6 => {
                    b.pos = p;
                    true
                }
                Some(n) if n.distance(p) < 6.0 => {
                    b.pos = n;
                    true
                }
                _ => false,
            }
        });
        if !self.tutorial_run && self.pfand.sums.iter().any(|s| *s > 0) {
            self.pfand.sums.clear();
            self.say("Der Automat hat dein Pfand vergessen.", 3.5);
        }
        let n = self.world.wing.as_ref().map_or(0, |w| w.machines.len());
        self.pfand.sums.resize(n, 0);
        self.scatter_bottles();
    }

    /// Flasche aufheben.
    pub(super) fn grab_bottle(&mut self, i: usize) {
        if i >= self.pfand.bottles.len() {
            return;
        }
        let b = self.pfand.bottles.remove(i);
        self.held = Some(b.kind.item());
        self.play(Sfx::Clink, 0.35);
        if !self.pfand.hinted {
            self.pfand.hinted = true;
            self.say("Leergut. In die Tüte am Wagen damit.", 4.0);
        }
    }

    /// Leergut aus der Hand in die Tuete am Wagen.
    pub(super) fn place_in_bag(&mut self, kind: Empty) {
        if self.pfand.bag_full() {
            self.hint("Die Tüte ist voll. Mehr als fünf passen nicht.");
            self.play(Sfx::Error, 0.3);
            return;
        }
        self.held = None;
        self.pfand.bag.push(kind);
        self.pfand.bag_changed = self.time;
        self.play(Sfx::Clink, 0.45);
    }

    /// Letzte Flasche wieder aus der Tuete nehmen.
    pub(super) fn take_from_bag(&mut self) {
        if let Some(k) = self.pfand.bag.pop() {
            self.pfand.bag_changed = self.time;
            self.held = Some(k.item());
            self.play(Sfx::Clink, 0.35);
        }
    }

    /// Mit dem Wagen gelandet: eine oder zwei Flaschen springen heraus.
    pub(super) fn spill_bag(&mut self) {
        let n = self.pfand.bag.len().min(2);
        if n == 0 {
            return;
        }
        let n = self.pfand.rng.gen_range(1..=n);
        let (s, c) = self.trolley.yaw.sin_cos();
        let side = vec3(c, 0.0, s);
        for _ in 0..n {
            let Some(k) = self.pfand.bag.pop() else { break };
            let push = self.pfand.rng.gen_range(0.5..1.2);
            self.loose.push(Loose {
                data: k.item(),
                pos: self.trolley.pos + side * 0.4 + vec3(0.0, 0.6, 0.0),
                vel: side * push + vec3(0.0, 1.2, 0.0),
                yaw: push * 5.0,
            });
        }
        self.play(Sfx::Clink, 0.8);
    }

    /// Lage der Tuete: haengt rechts am Wagen.
    pub(super) fn bag_center(&self) -> Vec3 {
        let m = Mat4::from_rotation_translation(
            Quat::from_rotation_y(-self.trolley.yaw),
            self.trolley.pos + vec3(0.0, self.cart_lift(), 0.0),
        );
        m.transform_point3(BAG_AT)
    }

    pub(super) fn bag_box(&self) -> world::Aabb {
        world::Aabb::from_center(self.bag_center(), vec3(0.2, 0.2, 0.2))
    }

    /// E ohne Blickstrahl (Tests).
    #[cfg(test)]
    fn interact_for_test(&mut self) {
        self.focus = Some(Focus::Cart);
        self.interact();
    }

    pub(super) fn pfand_cheat(&mut self, k: usize) -> String {
        self.tip();
        match k {
            0 => {
                let n = self.world.wing.as_ref().map_or(0, |w| w.machines.len());
                let Some(i) = (0..n).find(|&i| self.machine_works(i)) else {
                    return "Kein Automat.".into();
                };
                let p = self.world.wing.as_ref().map(|w| w.machines[i].front());
                self.player.pos = self.world.collide(p.unwrap_or_default(), PLAYER_RADIUS);
                self.player.yaw = -FRAC_PI_2;
                self.player.pitch = -0.2;
                let beside = self.player.pos + vec3(0.6, 0.0, 1.6);
                self.trolley.pushing = false;
                self.trolley.pos = self.world.collide(beside, 0.42);
                "Vor dem Automaten.".into()
            }
            1 => {
                while !self.pfand.bag_full() {
                    self.pfand.bag.push(Empty::ALL[self.pfand.bag.len() % 4]);
                }
                "Tüte voll.".into()
            }
            3 => {
                let Some(k) = self.world.kiosk else {
                    return "Kein Kiosk.".into();
                };
                self.player.pos = self.world.collide(k.front(), PLAYER_RADIUS);
                self.player.yaw = -FRAC_PI_2;
                self.player.pitch = -0.1;
                "Am Kiosk.".into()
            }
            _ => {
                self.pfand.bons.push(250);
                self.pfand.printed = true;
                "Bon über 2,50 €.".into()
            }
        }
    }

    /// Flaschen am Boden, Tuete am Wagen, Anzeigen der Automaten.
    pub(super) fn collect_pfand_draws(&self, out: &mut Vec<MeshDraw>) {
        if !self.tipped {
            return;
        }
        for (i, b) in self.pfand.bottles.iter().enumerate() {
            let item = b.kind.item();
            let h = item.half();
            // liegt auf der Seite
            let tip = Quat::from_rotation_y(b.yaw) * Quat::from_rotation_x(FRAC_PI_2);
            let p = b.pos + vec3(0.0, h.x.max(h.z), 0.0) + self.sway_at(b.pos);
            let glow = if self.focus == Some(Focus::Bottle(i)) || self.bottle_marked(i) {
                0.35 + 0.35 * (0.5 + 0.5 * (self.time * 5.0).sin())
            } else {
                0.06
            };
            out.push(MeshDraw {
                mesh: item.mesh,
                model: Mat4::from_rotation_translation(tip, p),
                tint: item.tint.extend(glow),
                ..Default::default()
            });
        }
        self.collect_bag_draws(out);
        self.collect_machine_draws(out);
        self.collect_item_draws(out);
    }

    /// Groesse der Tuete (1 = voll): leer haengt sie schlaff, jede Flasche blaeht sie auf.
    /// Kurz nach dem Hineinlegen wippt sie nach (`bag_changed`).
    pub(super) fn bag_size(&self) -> Vec3 {
        let fill = self.pfand.bag.len() as f32 / BAG_MAX as f32;
        let wide = BAG_EMPTY + (1.0 - BAG_EMPTY) * fill;
        let tall = 0.75 + 0.25 * fill;
        let age = self.time - self.pfand.bag_changed;
        let wobble = if (0.0..0.6).contains(&age) {
            0.08 * (1.0 - age / 0.6) * (age * 30.0).sin()
        } else {
            0.0
        };
        vec3(wide + wobble, tall - wobble * 0.5, wide + wobble)
    }

    /// Pfandtuete am Wagen (Modell, haengt am Haken); die Flaschen sind darin verschwunden.
    fn collect_bag_draws(&self, out: &mut Vec<MeshDraw>) {
        let base = self.trolley.pos
            + vec3(
                0.0,
                self.sway_at(self.trolley.pos).y + self.cart_lift(),
                0.0,
            );
        let m = Mat4::from_rotation_translation(Quat::from_rotation_y(-self.trolley.yaw), base);
        let glow = if self.pfand.aim_bag { 0.25 } else { 0.0 };
        let s = self.bag_size();
        // haengt am Knoten: oben fest, waechst nach unten und nach aussen (weg vom Korb)
        let top = BAG_AT.y + BAG_HANG;
        let foot = vec3(
            BAG_AT.x + (BAG_HALF_W * s.x - BAG_HALF_W * BAG_EMPTY).max(0.0),
            top - BAG_HEIGHT * s.y,
            BAG_AT.z,
        );
        out.push(MeshDraw {
            mesh: MeshId::PfandBag as usize,
            model: m * Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, foot),
            tint: Vec4::new(0.95, 0.95, 0.92, glow),
            ..Default::default()
        });
        // Henkel vom Knoten zum Griff
        out.push(MeshDraw {
            mesh: MeshId::Cube as usize,
            model: m * Mat4::from_scale_rotation_translation(
                vec3(0.012, 0.08, 0.012),
                Quat::IDENTITY,
                vec3(foot.x, top + 0.03, BAG_AT.z),
            ),
            tint: Vec4::new(0.8, 0.78, 0.7, 0.0),
            ..Default::default()
        });
    }
}

/// Aufhaengung der Tuete in Wagen-Koordinaten (rechts neben dem Korb, hinten am Griff)
const BAG_AT: Vec3 = Vec3::new(0.34, 0.6, 0.2);
/// Modell der Tuete (assets.rs `PfandBag`): Hoehe und halbe Breite in voller Groesse (m)
const BAG_HEIGHT: f32 = 0.36;
const BAG_HALF_W: f32 = 0.16;
/// Knoten der Tuete so weit ueber `BAG_AT` (m)
const BAG_HANG: f32 = 0.16;
/// Breite der leeren Tuete (Anteil der vollen)
pub const BAG_EMPTY: f32 = 0.5;
use std::f32::consts::FRAC_PI_2;
