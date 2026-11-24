//! Der Fluchtkorridor hinter der Ladentuer (Geometrie: world/gen/corridor/).
//! Nach dem Bezahlen rennt man durch den Markt zur Tuer. Im Gang selbst ist man allein - es wirkt
//! nur so, als waere einem die Meute auf den Fersen: Hinter einem knallt die Tuer zu, eine
//! unsichtbare "Front" rueckt nach (Schritte, Atmen, Herzschlag). Wer stehen bleibt oder zu
//! langsam ist, wird eingeholt; wer sich umdreht, ist sofort tot. Ausdauer ist unendlich.
//!
//! - `front.rs`: die Front und wie man sie spuert
//! - `obstacles.rs`: Tueren, Kipp-Regale, rollende Wagen, Huerden, Engstellen (Kollision)
//! - `events.rs`: Gestalten, Durchsagen, Decke senkt sich, Waende ruecken zusammen
//! - `ending.rs`: letztes Stueck mit Spiegel, Gestalt hinter einem, klemmende Ausgangstuer
//!
//! Was hinter einem passiert, sieht man nur im Spiegel am Ende - dafuer filmt die zweite Kamera
//! (sonst der Ueberwachungsmonitor) vom gespiegelten Auge aus.

use super::*;
use crate::world::{Corridor, StationKind, CORRIDOR_HALF_W, EXIT_FROM, EXIT_TO};
use std::f32::consts::FRAC_PI_2;

mod ending;
mod events;
mod front;
mod obstacles;

/// Ab hier (Kosinus zwischen Blick und Gangrichtung) warnt das Spiel: "NICHT UMDREHEN"
pub const TURN_WARN: f32 = 0.2;
/// Ab hier hat man sich umgedreht (gut 110 Grad): sofort Game Over
pub const TURN_DEAD: f32 = -0.35;
/// Umdreh-Schreck: so lange ist das Gesicht im Bild (danach Abblende zum Game Over)
pub const TURN_FACE_TIME: f32 = 1.4;
/// So weit (Meter) muss man im Gang sein, damit hinter einem die Tuer zuknallt
const ENTER_AT: f32 = 1.0;

/// Zustand einer Station im Gang (Index wie `Corridor::stations`).
#[derive(Clone, Copy, Debug, Default)]
pub struct StationState {
    /// ausgeloest seit (Sekunden), None = noch nicht
    pub t: Option<f32>,
    /// Tuer: 0 zu .. 1 offen
    pub open: f32,
    /// Tuer: wie oft schon gedrueckt
    pub presses: u32,
    /// Wagen: wo er gerade ist (Meter entlang des Wegs)
    pub s: f32,
    /// erledigt (Gestalt weg, Wagen hat getroffen/ist vorbei, Regal liegt)
    pub done: bool,
}

/// Was im Gang gerade los ist (entsteht beim ersten Betreten).
#[derive(Clone, Debug, Default)]
pub struct Run {
    /// Tuer zum Markt zu (man ist drin, NPCs sind weg)
    pub entered: bool,
    /// seit dem Zuknallen (Sekunden)
    pub slam: f32,
    /// wie weit die Front ist (Meter entlang des Wegs)
    pub front: f32,
    pub stations: Vec<StationState>,
    /// gestolpert: noch so lange langsamer (Sekunden)
    pub stumble: f32,
    /// Ton-Takte der Front
    pub step_acc: f32,
    pub heart_acc: f32,
    pub voice_t: f32,
    pub whisper_t: f32,
    /// im letzten Stueck (Gestalt direkt hinter einem)
    pub ending: bool,
    /// letzter Schreck an der Ausgangstuer schon passiert
    pub last_scare: bool,
}

impl Run {
    fn new(c: &Corridor) -> Self {
        Self {
            stations: c
                .stations
                .iter()
                .map(|st| StationState {
                    s: st.s,
                    ..Default::default()
                })
                .collect(),
            voice_t: 5.0,
            whisper_t: 3.0,
            ..Default::default()
        }
    }
}

impl Game {
    /// Steht man im Gang (ein Stueck hinter der Tuer)?
    pub fn in_corridor(&self) -> bool {
        self.world
            .corridor
            .as_ref()
            .is_some_and(|c| c.contains(self.player.pos) && c.progress(self.player.pos) > 0.6)
    }

    /// Ist man im Gang und die Tuer hinter einem schon zu?
    pub fn corridor_entered(&self) -> bool {
        self.corridor_run.as_ref().is_some_and(|r| r.entered)
    }

    /// Wie weit ist man im Gang (Meter ab der Ladentuer)?
    pub fn corridor_progress(&self) -> f32 {
        self.world
            .corridor
            .as_ref()
            .map_or(0.0, |c| c.progress(self.player.pos))
    }

    /// Wie weit schaut man gerade nach vorn (1) bzw. zurueck (-1)? Massgeblich ist die
    /// Richtung des Gangs an dieser Stelle (in Ecken die bessere von beiden), an der
    /// Ausgangstuer zaehlt auch der Blick zur Tuer.
    pub fn corridor_facing(&self) -> f32 {
        let Some(c) = &self.world.corridor else {
            return 1.0;
        };
        let f = self.view_dir();
        let p = self.player.pos;
        let mut facing = c.facing(p, f);
        if c.progress(p) > c.len - EXIT_FROM - 1.5 {
            let flat = vec3(f.x, 0.0, f.z).normalize_or_zero();
            facing = facing.max(flat.dot(Vec3::X));
        }
        facing
    }

    /// Umdreh-Warnung 0..1 fuer das HUD (0 = schaut nach vorn).
    pub fn turn_warning(&self) -> f32 {
        if !(self.chasing() && self.in_corridor()) {
            return 0.0;
        }
        ((TURN_WARN - self.corridor_facing()) / (TURN_WARN - TURN_DEAD)).clamp(0.0, 1.0)
    }

    /// Im Gang: beim Betreten Tuer zu und NPCs weg, Front, Hindernisse, Ereignisse, Ende,
    /// Umdrehen = sofort tot.
    pub(super) fn update_corridor(&mut self, dt: f32) {
        if !self.chasing() || self.outcome != Outcome::Running {
            return;
        }
        let Some(c) = &self.world.corridor else {
            return;
        };
        if self.corridor_run.is_none() {
            self.corridor_run = Some(Run::new(c));
        }
        let progress = c.progress(self.player.pos);
        let inside = c.contains(self.player.pos);
        // Betreten: hinter einem knallt die Tuer zu, im Gang ist man allein
        if inside && progress > ENTER_AT && !self.corridor_entered() {
            self.enter_corridor(progress);
        }
        if !self.corridor_entered() {
            return;
        }
        if let Some(r) = &mut self.corridor_run {
            r.slam += dt;
            r.stumble = (r.stumble - dt).max(0.0);
        }
        self.update_obstacles(dt);
        self.update_corridor_events(dt);
        self.update_ending(dt);
        self.update_front(dt);
        if self.outcome != Outcome::Running
            || !self.in_corridor()
            || self.world.exit_zone.contains_xz(self.player.pos)
        {
            return;
        }
        // Umdrehen
        let facing = self.corridor_facing();
        let turned = facing < TURN_DEAD;
        if self.tutorial_run {
            // Tutorial: warnen statt Game Over, einmal je Umdrehen
            let warn = match &mut self.tutorial {
                Some(t) if turned && !t.turn_warned => {
                    t.turn_warned = true;
                    true
                }
                Some(t) => {
                    if facing > 0.0 {
                        t.turn_warned = false;
                    }
                    false
                }
                None => false,
            };
            if warn {
                self.tutorial_forgive(
                    "Du hast dich umgedreht! Im echten Spiel ist das Game Over. Weiter!",
                );
            }
            return;
        }
        if turned && !self.god {
            self.turn_scare(false);
        }
    }

    /// Tuer zu, NPCs weg, die Front startet hinter einem.
    fn enter_corridor(&mut self, progress: f32) {
        self.customers.clear();
        self.employees.clear();
        self.shoppers.clear();
        self.dialog = None;
        // der Wagen bleibt im Markt (hinter der zugeknallten Tuer)
        if let Some(c) = &self.world.corridor {
            if self.trolley.pushing || c.contains(self.trolley.pos) {
                self.trolley.pushing = false;
                self.trolley.pos = c.door_center - vec3(0.0, 0.0, 1.6);
            }
        }
        self.player.cart_jump = false;
        if let Some(r) = &mut self.corridor_run {
            r.entered = true;
            r.slam = 0.0;
            r.front = progress - front::FRONT_START;
        }
        self.play(Sfx::Thud, 1.0);
        self.play(Sfx::Rumble, 0.6);
        self.glitch = self.glitch.max(0.4);
        self.say("Lauf. Und dreh dich NICHT um.", 4.0);
        self.play(Sfx::Sting, 0.4);
    }

    /// Umgedreht (oder von der Front eingeholt): Jumpscare wie die anderen Schrecken
    /// (scares.rs) - Schrei, Farbumkehr-Stroboskop, Lichtblitz, Zoom-Ruck - und ein leeres
    /// Gesicht rast zitternd auf die Kamera zu (collect_scare_people). Danach Game Over.
    fn turn_scare(&mut self, by_front: bool) {
        self.turned_back = !by_front;
        self.front_caught = by_front;
        self.outcome = Outcome::Caught;
        self.outcome_timer = 0.0;
        self.red = 1.0;
        self.glitch = 1.0;
        self.fire_scare(1.0);
        self.play(Sfx::Shriek, 1.3);
        self.play(Sfx::Sting, 1.0);
        // im Gang ist niemand - eine der Gestalten aus dem Markt
        let pool = crate::people::people_of(false);
        let k = (self.seed as usize + self.params.corridor_seed as usize) % pool.len();
        self.catcher_variant = Some(pool[k]);
        // die Kamera bleibt auf sie gerichtet
        let f = self.view_dir();
        self.catcher = Some(self.player.pos + vec3(f.x, 0.0, f.z).normalize_or_zero() * 0.6);
    }

    /// Schreck-Gesicht vor der Kamera (Abstand, Zittern), None = vorbei/kein Schreck.
    pub fn turn_scare_face(&self) -> Option<(usize, f32, Vec3)> {
        if !(self.turned_back || self.front_caught) || self.outcome != Outcome::Caught {
            return None;
        }
        let t = self.outcome_timer;
        if t >= TURN_FACE_TIME {
            return None;
        }
        let v = self.catcher_variant?;
        // in 0,3 s von 1,4 m auf 0,32 m heran, dann haelt es zitternd still
        let k = (t / 0.3).min(1.0);
        let dist = 1.4 - (1.4 - 0.32) * k * k;
        let s = self.time;
        let shake = vec3((s * 61.0).sin(), (s * 47.0).sin(), 0.0) * (0.012 + 0.02 * (1.0 - k));
        Some((v, dist, shake))
    }

    /// Cheat/Test: bezahlt, Alarm laeuft, man steht kurz hinter der Tuer im Gang und schaut
    /// nach vorn. `len`: Ganglaenge erzwingen (None = aus der Run-Zeit).
    pub fn debug_jump_to_corridor(&mut self, len: Option<f32>) {
        if !self.paid {
            self.tip();
            self.fill_list_cheap();
            self.scanned.append(&mut self.cart);
            self.belt.clear();
            self.held = None;
            self.dialog = None;
            self.pay();
        }
        if let Some(len) = len {
            self.params.corridor_len = len;
            self.corridor_run = None;
            self.rebuild_world();
        }
        let Some(c) = &self.world.corridor else {
            return;
        };
        self.player.pos = c.point_at(ENTER_AT + 0.6);
        self.player.yaw = std::f32::consts::PI;
        self.player.pitch = 0.0;
        self.player.vel = Vec3::ZERO;
        self.trolley.pushing = false;
        self.field_timer = 0.0;
    }

    /// Kamera fuers Spiegelbild: vom am Spiegel gespiegelten Auge aus, Bildausschnitt genau
    /// die Spiegelflaeche (schiefes Frustum). None = nicht im Gang.
    pub fn mirror_view(&self) -> Option<(Mat4, Vec3)> {
        let c = self.world.corridor.as_ref()?;
        if !c.contains(self.player.pos) {
            return None;
        }
        let eye = self.eye();
        let zm = c.mirror_center.z;
        let n = (zm - eye.z).max(0.05);
        let cam = vec3(eye.x, eye.y, zm + n);
        let view = Mat4::look_to_rh(cam, -Vec3::Z, Vec3::Y);
        let (mn, mx) = (
            c.mirror_center - c.mirror_half,
            c.mirror_center + c.mirror_half,
        );
        let (l, r) = (mn.x - cam.x, mx.x - cam.x);
        let (b, t) = (mn.y - cam.y, mx.y - cam.y);
        // weit genug, dass der ganze Gang (doppelter Abstand) ins Bild passt
        let far = 2.0 * n + 50.0;
        let proj = Mat4::from_cols(
            Vec4::new(2.0 * n / (r - l), 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 * n / (t - b), 0.0, 0.0),
            Vec4::new((r + l) / (r - l), (t + b) / (t - b), far / (n - far), -1.0),
            Vec4::new(0.0, 0.0, n * far / (n - far), 0.0),
        );
        Some((proj * view, cam))
    }

    /// Im Gang: Ausdauer unendlich.
    pub fn corridor_endless_stamina(&self) -> bool {
        self.corridor_entered() && self.in_corridor()
    }

    /// Tempo-Faktor im Gang: gestolpert, in einer Engstelle. Zwischen zusammengerueckten
    /// Waenden darf man weiter sprinten. Liefert (Faktor, Hoechsttempo).
    pub(super) fn corridor_speed(&self) -> (f32, f32) {
        let Some(r) = &self.corridor_run else {
            return (1.0, f32::MAX);
        };
        let Some(c) = &self.world.corridor else {
            return (1.0, f32::MAX);
        };
        let f = if r.stumble > 0.0 { 0.4 } else { 1.0 };
        let p = self.player.pos;
        let squeezed = c.squeezes.iter().any(|z| z.contains_xz(p));
        (
            f,
            if squeezed {
                obstacles::SQUEEZE_SPEED
            } else {
                f32::MAX
            },
        )
    }
}

/// Kreis (xz) aus einem Kasten herausschieben.
pub(super) fn push_out(p: Vec3, r: f32, b: &world::Aabb) -> Vec3 {
    let cx = p.x.clamp(b.min.x, b.max.x);
    let cz = p.z.clamp(b.min.z, b.max.z);
    let (dx, dz) = (p.x - cx, p.z - cz);
    let len = (dx * dx + dz * dz).sqrt();
    if len >= r {
        return p;
    }
    if len > 1e-5 {
        let k = (r - len) / len;
        return vec3(p.x + dx * k, p.y, p.z + dz * k);
    }
    // Mittelpunkt im Kasten: auf kuerzestem Weg hinaus
    let opts = [
        (p.x - b.min.x + r, vec3(-1.0, 0.0, 0.0)),
        (b.max.x - p.x + r, vec3(1.0, 0.0, 0.0)),
        (p.z - b.min.z + r, vec3(0.0, 0.0, -1.0)),
        (b.max.z - p.z + r, vec3(0.0, 0.0, 1.0)),
    ];
    let (d, n) = opts
        .iter()
        .copied()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    p + n * d
}

#[cfg(test)]
mod tests;
