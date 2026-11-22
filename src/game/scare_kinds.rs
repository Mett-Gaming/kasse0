//! Jeder Schreck hat seine eigene Handschrift: eigener Klang, eigene Bildwirkung
//! (Negativ-Flackern, Lichtblitz, Zoom hinein oder heraus, Wackeln, Rotstich, Bildriss).
//! Damit fuehlt sich nicht jeder Schreck gleich an. Dazu acht weitere Schrecken und eine
//! Abwechslungs-Regel: dieselbe Art kommt nicht zweimal kurz hintereinander.

use super::*;
use crate::people::{Clip, PersonDraw};

/// Art eines Schrecks (bestimmt Klang und Bildwirkung).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScareKind {
    /// allgemein (z. B. Fluchtkorridor): die alte Wirkung
    Generic,
    Behind,
    Blackout,
    Cashier,
    CartGhost,
    Mirage,
    Cables,
    Hanging,
    Monitor,
    Death,
    Freezer,
    Crawler,
    AllLook,
    DarkSteps,
    ShelfFace,
    CartRider,
    Announcement,
    ShelfShake,
}

/// Bildwirkung und Klang eines Schrecks.
#[derive(Clone, Copy, Debug)]
pub struct ScareStyle {
    pub sounds: [(Sfx, f32); 2],
    /// Negativ-Stroboskop
    pub invert: bool,
    /// Ueberbelichtung 0..1
    pub flash: f32,
    /// Zoom-Ruck: > 0 zieht das Bild heran, < 0 reisst es auseinander (Sichtfeld weiter)
    pub zoom: f32,
    /// Rotstich 0..1
    pub red: f32,
    /// Bildriss 0..1
    pub glitch: f32,
    /// Kamera wackelt 0..1
    pub shake: f32,
}

impl ScareKind {
    pub fn style(self) -> ScareStyle {
        use Sfx::*;
        let s = |sounds, invert, flash, zoom, red, glitch, shake| ScareStyle {
            sounds,
            invert,
            flash,
            zoom,
            red,
            glitch,
            shake,
        };
        match self {
            ScareKind::Generic => s([(Scare, 1.0), (Scare, 0.0)], true, 1.0, 0.3, 0.35, 1.0, 0.0),
            // umdrehen, er steht vor dir: dumpfer Schlag, Bild zieht heran, wackelt
            ScareKind::Behind => s([(Thud, 1.0), (Scare, 0.7)], false, 0.0, 0.4, 0.2, 0.3, 0.8),
            // Licht ist wieder da: greller Blitz, Rauschen
            ScareKind::Blackout => s(
                [(Static, 0.9), (Sting, 0.8)],
                false,
                1.0,
                0.0,
                0.0,
                0.6,
                0.0,
            ),
            ScareKind::Cashier => s(
                [(Screech, 1.0), (Sting, 0.5)],
                false,
                0.0,
                0.35,
                0.5,
                0.2,
                0.3,
            ),
            // Wagen kracht: Metall, starkes Wackeln
            ScareKind::CartGhost => s(
                [(Stomp, 1.0), (Rattle, 1.0)],
                false,
                0.2,
                0.15,
                0.0,
                0.2,
                1.0,
            ),
            // Hologramm: Rauschen, Negativ, Bildriss
            ScareKind::Mirage => s([(Static, 1.0), (Sting, 0.4)], true, 0.0, 0.0, 0.0, 1.0, 0.0),
            ScareKind::Cables => s(
                [(Spark, 1.0), (Static, 0.5)],
                false,
                0.9,
                0.0,
                0.0,
                0.5,
                0.3,
            ),
            // kopfueber im Loch: Kreischen, Bild reisst auseinander (Zoom heraus), rot
            ScareKind::Hanging => s(
                [(Screech, 1.0), (Rumble, 0.8)],
                false,
                0.0,
                -0.35,
                0.6,
                0.3,
                0.2,
            ),
            ScareKind::Monitor => s([(Static, 1.0), (Thud, 0.6)], true, 0.0, 0.2, 0.0, 0.9, 0.0),
            ScareKind::Death => s([(Scare, 1.0), (Thud, 1.0)], false, 0.6, 0.4, 1.0, 0.9, 1.0),
            ScareKind::Freezer => s(
                [(Rumble, 1.0), (Screech, 0.7)],
                false,
                0.0,
                0.45,
                0.4,
                0.2,
                0.4,
            ),
            ScareKind::Crawler => s(
                [(Screech, 0.9), (Shuffle, 1.0)],
                false,
                0.0,
                -0.2,
                0.0,
                0.4,
                0.6,
            ),
            // alle schauen: Fluesterchor, ein einzelner Negativblitz, Bild weicht zurueck
            ScareKind::AllLook => s(
                [(Whisper, 1.0), (Sting, 0.9)],
                true,
                0.0,
                -0.3,
                0.2,
                0.1,
                0.0,
            ),
            ScareKind::DarkSteps => s([(Thud, 1.0), (Exhale, 0.9)], false, 0.8, 0.3, 0.0, 0.2, 0.5),
            // Gesicht im Regal: der haerteste Ruck
            ScareKind::ShelfFace => s(
                [(Scare, 1.0), (Screech, 0.8)],
                false,
                0.3,
                0.55,
                0.3,
                0.4,
                0.7,
            ),
            ScareKind::CartRider => s(
                [(Thud, 1.0), (Rattle, 0.8)],
                false,
                0.0,
                0.35,
                0.3,
                0.2,
                0.5,
            ),
            // Durchsage: Kreischen aus den Lautsprechern, Bildriss
            ScareKind::Announcement => s(
                [(Shriek, 0.55), (Static, 0.8)],
                false,
                0.0,
                0.0,
                0.3,
                1.0,
                0.3,
            ),
            // Regal kracht: schwerer Schlag, viel Wackeln
            ScareKind::ShelfShake => s([(Stomp, 1.0), (Thud, 0.8)], false, 0.0, 0.1, 0.0, 0.3, 1.0),
        }
    }
}

/// So viele zuletzt ausgeloeste Arten kommen nicht sofort wieder dran
const RECENT: usize = 3;
/// Chance pro Sekunde fuer einen der freien Schrecken (Decke, alle schauen, Schritte,
/// Durchsage, Regal wackelt), wenn ein Schreck erlaubt ist
const FREE_SCARE_CHANCE: f32 = 0.03;
/// Decke: so schnell, so lange, so weit voraus
const CRAWL_SPEED: f32 = 4.5;
const CRAWL_TIME: f32 = 1.4;
/// Schritte im Dunkeln: Dauer der Dunkelheit
const DARK_STEPS_TIME: f32 = 2.0;
/// Gesicht im Regal: Chance beim Greifen, Dauer
const SHELF_FACE_CHANCE: f64 = 0.15;
const SHELF_FACE_TIME: f32 = 0.7;
/// Im Wagen: so lange unbeobachtet, bevor jemand drin hockt; Dauer
const RIDER_ARM: f32 = 8.0;
const RIDER_TIME: f32 = 1.6;
/// Kuehltruhe: Dauer, Zeitpunkt des Schrecks
const FREEZER_TIME: f32 = 2.2;
const FREEZER_HIT: f32 = 0.3;
/// Durchsage: erst der Gong, dann das Kreischen
const ANNOUNCE_HIT: f32 = 0.9;

/// Laufende Zusatz-Schrecken.
#[derive(Clone, Debug, Default)]
pub struct ExtraScares {
    /// Gestalt rennt kopfueber an der Decke: (Start, Richtung, Zeit)
    pub crawler: Option<(Vec3, Vec3, f32)>,
    /// Schritte im Dunkeln: Zeit
    pub dark_steps: Option<f32>,
    /// Gesicht im Regal: (Ort, Blickrichtung, Zeit, Figur)
    pub shelf_face: Option<(Vec3, f32, f32, usize)>,
    /// Im Wagen: wie lange unbeobachtet, bzw. sichtbar seit
    pub rider_unseen: f32,
    pub rider: Option<f32>,
    /// Kuehltruhe: (Ort, Laengsrichtung, Zeit, Figur)
    pub freezer: Option<(Vec3, f32, f32, usize)>,
    pub freezer_done: bool,
    /// Durchsage: Zeit
    pub announce: Option<f32>,
}

impl Game {
    /// Schreck einer bestimmten Art: Klang und Bildwirkung aus ihrem Stil.
    pub(super) fn fire_scare_kind(&mut self, kind: ScareKind, strength: f32) {
        let st = kind.style();
        for (s, v) in st.sounds {
            if v > 0.0 {
                self.sfx.push((s, v * (0.6 + 0.4 * strength)));
            }
        }
        self.glitch = self.glitch.max(st.glitch * (0.6 + 0.4 * strength));
        self.red = self.red.max(st.red * strength);
        self.hush_t = 0.0;
        self.scare_cd = self
            .rng
            .gen_range(super::scares::SCARE_COOLDOWN.0..super::scares::SCARE_COOLDOWN.1);
        self.scare_fx = super::scares::SCARE_FX;
        self.scare_fx_strength = strength;
        self.scare_style = st;
        self.scares.push(kind);
        self.recent_scares.push(kind);
        if self.recent_scares.len() > RECENT {
            self.recent_scares.remove(0);
        }
        if strength >= 0.9 {
            self.director.calm_down(super::director::CALM_AFTER_SCARE);
        }
    }

    /// Darf ein Schreck dieser Art kommen? (erlaubt und nicht gerade erst dagewesen)
    pub(super) fn can_scare_kind(&self, big: bool, kind: ScareKind) -> bool {
        self.can_scare(big) && !self.recent_scares.contains(&kind)
    }

    /// Kamera-Wackeln des laufenden Schrecks: Versatz im Bild (x, y) und Kippen.
    pub fn scare_shake(&self) -> (f32, f32, f32) {
        let k = (self.scare_fx / super::scares::SCARE_FX).clamp(0.0, 1.0);
        let a = self.scare_style.shake * k * k;
        if a <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let n =
            |s: f32| (((self.time * 60.0).floor() * 12.9898 + s).sin() * 43_758.547).fract() - 0.5;
        (n(1.0) * 0.12 * a, n(2.0) * 0.12 * a, n(3.0) * 0.12 * a)
    }

    // ------------------------------------------------------------ Zusatz-Schrecken

    pub(super) fn update_extra_scares(&mut self, dt: f32) {
        self.update_crawler(dt);
        self.update_dark_steps(dt);
        self.update_shelf_face(dt);
        self.update_cart_rider(dt);
        self.update_freezer(dt);
        self.update_announcement(dt);
        // freie Schrecken: einer davon, zufaellig, aber nie dieselbe Art kurz hintereinander
        if !self.can_scare(true) || self.checking_out() || self.dialog.is_some() {
            return;
        }
        if !self
            .rng
            .gen_bool((FREE_SCARE_CHANCE * dt).clamp(0.0, 1.0) as f64)
        {
            return;
        }
        let kinds = [
            ScareKind::Crawler,
            ScareKind::AllLook,
            ScareKind::DarkSteps,
            ScareKind::Announcement,
            ScareKind::ShelfShake,
        ];
        let open: Vec<ScareKind> = kinds
            .into_iter()
            .filter(|k| !self.recent_scares.contains(k))
            .collect();
        if open.is_empty() {
            return;
        }
        let k = open[self.rng.gen_range(0..open.len())];
        self.start_free_scare(k);
    }

    /// Einen der freien Schrecken sofort starten (auch fuer die Cheats). false = ging nicht.
    pub(super) fn start_free_scare(&mut self, k: ScareKind) -> bool {
        match k {
            ScareKind::Crawler => self.start_crawler(),
            ScareKind::AllLook => self.start_all_look(),
            ScareKind::DarkSteps => {
                self.extra.dark_steps = Some(0.0);
                self.sfx.push((Sfx::Spark, 1.0));
                true
            }
            ScareKind::Announcement => {
                self.extra.announce = Some(0.0);
                self.sfx.push((Sfx::Chime, 0.9));
                self.announcement = Some((
                    "Durchsage: Kundin an Kasse Null. Kundin an Kasse Null.".into(),
                    3.0,
                ));
                true
            }
            ScareKind::ShelfShake => self.start_shelf_shake(),
            _ => false,
        }
    }

    /// Decke: eine Gestalt rennt kopfueber quer durchs Bild, ein Stueck voraus.
    fn start_crawler(&mut self) -> bool {
        let f = self.view_dir();
        let flat = vec3(f.x, 0.0, f.z).normalize_or_zero();
        let side = vec3(-flat.z, 0.0, flat.x);
        for d in [8.0f32, 6.0, 10.0, 5.0] {
            let mid = self.player.pos + flat * d;
            if !self.sees(vec3(mid.x, crate::world::CEILING_HEIGHT - 0.6, mid.z)) {
                continue;
            }
            let dir = if self.rng.gen_bool(0.5) { side } else { -side };
            let start = mid - dir * (CRAWL_SPEED * CRAWL_TIME * 0.5);
            self.extra.crawler = Some((start, dir, 0.0));
            self.fire_scare_kind(ScareKind::Crawler, 0.7);
            return true;
        }
        false
    }

    fn update_crawler(&mut self, dt: f32) {
        let Some((start, dir, t)) = self.extra.crawler else {
            return;
        };
        let t = t + dt;
        if t > CRAWL_TIME {
            self.extra.crawler = None;
            return;
        }
        // schlurfende Schritte an der Decke, mit Richtung
        if (t * 6.0).floor() != ((t - dt) * 6.0).floor() {
            let p = start + dir * CRAWL_SPEED * t;
            self.sfx_at.push((Sfx::Shuffle, 0.8, vec3(p.x, 3.5, p.z)));
        }
        self.extra.crawler = Some((start, dir, t));
    }

    /// Alle schauen: jede sichtbare Kundin dreht sich gleichzeitig zu dir und erstarrt.
    fn start_all_look(&mut self) -> bool {
        let player = self.player.pos;
        let near: Vec<usize> = (0..self.customers.len())
            .filter(|&i| {
                let c = &self.customers[i];
                c.seen && (c.pos - player).length() < 16.0
            })
            .collect();
        if near.len() < 2 {
            return false;
        }
        for &i in &near {
            let c = &mut self.customers[i];
            c.yaw = npc::yaw_towards(c.pos, player);
            c.brain.freeze = 2.5;
        }
        for &i in near.iter().take(3) {
            let at = self.customers[i].pos + vec3(0.0, 1.5, 0.0);
            self.sfx_at.push((Sfx::Whisper, 0.9, at));
        }
        self.fire_scare_kind(ScareKind::AllLook, 0.8);
        true
    }

    /// Schritte im Dunkeln: Licht aus, jemand rennt von hinten heran, atmet dir in den
    /// Nacken; wenn das Licht wiederkommt, ist niemand da.
    fn update_dark_steps(&mut self, dt: f32) {
        let Some(t) = self.extra.dark_steps else {
            return;
        };
        let t = t + dt;
        let f = self.view_dir();
        let back = -vec3(f.x, 0.0, f.z).normalize_or_zero();
        let p = self.player.pos;
        if t < DARK_STEPS_TIME - 0.4 && (t * 5.5).floor() != ((t - dt) * 5.5).floor() {
            let k = (t / (DARK_STEPS_TIME - 0.4)).min(1.0);
            let at = p + back * (7.0 - 6.2 * k) + vec3(0.0, 0.1, 0.0);
            self.sfx_at.push((Sfx::Stomp, 0.5 + 0.5 * k, at));
        }
        if t >= DARK_STEPS_TIME {
            self.extra.dark_steps = None;
            self.fire_scare_kind(ScareKind::DarkSteps, 0.8);
            return;
        }
        self.extra.dark_steps = Some(t);
    }

    /// Dunkelheit der Schritte im Dunkeln (fuer die Abblende).
    pub fn dark_steps_dark(&self) -> f32 {
        match self.extra.dark_steps {
            Some(t) if t > DARK_STEPS_TIME - 0.25 => 0.4,
            Some(_) => 1.0,
            None => 0.0,
        }
    }

    /// Beim Greifen ins Regal: manchmal schiebt sich ein Gesicht zwischen der Ware hindurch.
    pub(super) fn maybe_shelf_face(&mut self, item_pos: Vec3) {
        if !self.can_scare_kind(false, ScareKind::ShelfFace)
            || !self.rng.gen_bool(SHELF_FACE_CHANCE)
        {
            return;
        }
        self.show_shelf_face(item_pos);
    }

    pub(super) fn show_shelf_face(&mut self, item_pos: Vec3) {
        let to = vec3(
            self.player.pos.x - item_pos.x,
            0.0,
            self.player.pos.z - item_pos.z,
        )
        .normalize_or_zero();
        // hinter der Ware, Gesicht auf Hoehe der Ware
        let variant = self.pick_person(false);
        let h = crate::people::height(variant);
        let pos = vec3(item_pos.x, item_pos.y + 0.1 - h * 0.93, item_pos.z) - to * 0.3;
        self.extra.shelf_face = Some((pos, to.x.atan2(to.z), 0.0, variant));
        self.fire_scare_kind(ScareKind::ShelfFace, 0.9);
    }

    fn update_shelf_face(&mut self, dt: f32) {
        if let Some((p, y, t, v)) = self.extra.shelf_face {
            let t = t + dt;
            self.extra.shelf_face = (t < SHELF_FACE_TIME).then_some((p, y, t, v));
        }
    }

    /// Im Wagen: Laesst man den Wagen lange allein, hockt bei der Rueckkehr jemand darin.
    fn update_cart_rider(&mut self, dt: f32) {
        let cart = self.trolley.pos;
        let dist = (cart - self.player.pos).length();
        if let Some(t) = &mut self.extra.rider {
            *t += dt;
            if *t > RIDER_TIME || dist < 1.3 {
                self.extra.rider = None;
            }
            return;
        }
        let watched = self.sees(cart + vec3(0.0, 0.6, 0.0));
        if self.trolley.pushing || self.cart_ghost != super::scares::CartGhost::Idle {
            self.extra.rider_unseen = 0.0;
            return;
        }
        if !watched && dist > 6.0 {
            self.extra.rider_unseen += dt;
        }
        if self.extra.rider_unseen > RIDER_ARM
            && watched
            && dist < 5.0
            && self.can_scare_kind(false, ScareKind::CartRider)
        {
            self.show_cart_rider();
        }
    }

    pub(super) fn show_cart_rider(&mut self) {
        self.extra.rider_unseen = 0.0;
        self.extra.rider = Some(0.0);
        self.fire_scare_kind(ScareKind::CartRider, 0.8);
    }

    /// Kuehltruhe: Wer hineinschaut, sieht jemanden darin liegen, der sich aufrichtet.
    fn update_freezer(&mut self, dt: f32) {
        if let Some((p, y, t, v)) = self.extra.freezer {
            let t = t + dt;
            if t >= FREEZER_HIT && t - dt < FREEZER_HIT {
                self.fire_scare_kind(ScareKind::Freezer, 0.9);
            }
            self.extra.freezer = (t < FREEZER_TIME).then_some((p, y, t, v));
            return;
        }
        if self.extra.freezer_done
            || self.player.pitch > -0.55
            || !self.can_scare_kind(true, ScareKind::Freezer)
        {
            return;
        }
        let p = self.player.pos;
        let hit = self.world.freezers.iter().find_map(|&(mid, yaw, len)| {
            let along = vec3(yaw.sin(), 0.0, yaw.cos());
            let d = p - mid;
            let a = d.dot(along).clamp(-len * 0.5 + 0.6, len * 0.5 - 0.6);
            let point = mid + along * a;
            let across = vec3(p.x - point.x, 0.0, p.z - point.z).length();
            (across < 1.8).then_some((point, yaw))
        });
        if let Some((point, yaw)) = hit {
            self.show_freezer(point, yaw);
        }
    }

    pub(super) fn show_freezer(&mut self, point: Vec3, yaw: f32) {
        let v = self.pick_person(false);
        self.extra.freezer = Some((vec3(point.x, 0.56, point.z), yaw, 0.0, v));
        self.extra.freezer_done = true;
        self.sfx.push((Sfx::Rumble, 0.5));
    }

    /// Durchsage: Gong, dann Kreischen aus allen Lautsprechern.
    fn update_announcement(&mut self, dt: f32) {
        let Some(t) = self.extra.announce else { return };
        let t = t + dt;
        if t >= ANNOUNCE_HIT && t - dt < ANNOUNCE_HIT {
            self.fire_scare_kind(ScareKind::Announcement, 0.8);
        }
        self.extra.announce = (t < ANNOUNCE_HIT + 1.0).then_some(t);
    }

    /// Regal wackelt: neben dir stuerzt Ware aus dem Regal auf dich zu.
    fn start_shelf_shake(&mut self) -> bool {
        let p = self.player.pos;
        let mut picked: Vec<usize> = (0..self.world.items.len())
            .filter(|&i| {
                let it = &self.world.items[i];
                !it.taken
                    && it.pos.y > 0.6
                    && vec3(it.pos.x - p.x, 0.0, it.pos.z - p.z).length() < 1.9
            })
            .collect();
        if picked.len() < 3 {
            return false;
        }
        picked.truncate(7);
        for i in picked {
            let it = &mut self.world.items[i];
            it.taken = true;
            let pos = it.pos;
            let data = ItemData::from_shelf(it);
            let to = vec3(p.x - pos.x, 0.0, p.z - pos.z).normalize_or_zero();
            let jitter = self.rng.gen_range(-0.5..0.5);
            self.loose.push(Loose {
                data,
                pos,
                vel: to * (1.8 + jitter) + vec3(0.0, 1.2, 0.0),
                yaw: self.rng.gen_range(0.0..std::f32::consts::TAU),
            });
        }
        self.statics_dirty = true;
        self.fire_scare_kind(ScareKind::ShelfShake, 0.8);
        true
    }

    /// Figuren der Zusatz-Schrecken.
    pub(super) fn collect_extra_scare_people(&self, out: &mut Vec<PersonDraw>) {
        let eye = self.eye();
        let mut person = |variant, clip, time, pos, yaw, flip, look: bool| {
            out.push(PersonDraw {
                variant,
                clip,
                time,
                pos,
                yaw,
                tint: Vec4::new(0.8, 0.8, 0.8, 0.12),
                ghost: false,
                flip,
                look_at: look.then_some(eye),
                fx: crate::people::PoseFx {
                    look_limit: std::f32::consts::PI,
                    ..Default::default()
                },
            });
        };
        if let Some((start, dir, t)) = self.extra.crawler {
            let p = start + dir * CRAWL_SPEED * t;
            person(
                self.cashier_variant,
                Clip::ZombieWalk,
                t * 2.5,
                vec3(p.x, crate::world::CEILING_HEIGHT, p.z),
                dir.x.atan2(dir.z),
                true,
                false,
            );
        }
        if let Some((p, y, t, v)) = self.extra.shelf_face {
            person(v, Clip::Idle, t, p, y, false, true);
        }
        if self.extra.rider.is_some() {
            let c = self.trolley.pos;
            person(
                self.pick_rider_variant(),
                Clip::CrouchIdle,
                self.time,
                c + vec3(0.0, 0.3, 0.0),
                npc::yaw_towards(c, self.player.pos),
                false,
                true,
            );
        }
        if let Some((p, y, t, v)) = self.extra.freezer {
            // liegt in der Truhe und richtet sich auf
            person(v, Clip::GetUp, t * 1.4, p, y, false, true);
        }
    }

    /// Feste Figur fuer "im Wagen" (keine Zufallszahl beim Zeichnen).
    fn pick_rider_variant(&self) -> usize {
        let pool = crate::people::people_of(false);
        pool[(self.seed as usize) % pool.len()]
    }
}
