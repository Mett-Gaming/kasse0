//! Pfandautomaten im Seitentrakt: ein Fenster mit kleinem Minispiel. Die Flasche liegt verdreht
//! im Schacht; gedreht wird, bis der Strichcode oben ist, dann bei gruener Lampe einlegen.
//! Je Stufe laufen weniger Automaten und sie werden unangenehmer.
//!
//! Layout (480x270-Raster) steht hier, damit Treffer und Zeichnen (hud/pfand.rs) gleich sind.

use super::super::belt_window::{inside, Rect};
use super::super::*;
use super::Empty;

/// Stellungen einer Flasche; 0 = Strichcode oben
pub const TURNS: u8 = 8;
/// Wo man Bons einloest (nach jedem Bondruck)
const KIOSK_HINT: &str = "Bon einlösen: Kiosk vor den Kassen, an der Westwand.";
/// So lange ist die Lampe nach dem Nachlegen rot (s)
pub const LAMP_TIME: f32 = 0.8;
/// Wagen muss so nah stehen, damit die Tuete erreichbar ist (m)
pub const BAG_REACH: f32 = 3.0;
/// Falsch eingelegt: so weit hoert man es (m)
pub const JAM_NOISE: f32 = 10.0;
/// Kollaps: so lange darf man davorstehen, bis etwas herausgreift (s)
pub const ARM_AFTER: f32 = 12.0;
/// so lange sieht man den Arm (s)
pub const ARM_TIME: f32 = 1.1;
const ARM_NOISE: f32 = 20.0;
/// Ab "unruhig": so oft lehnt er grundlos ab
const REFUSE: f64 = 0.15;

pub const WINDOW: Rect = (90.0, 34.0, 300.0, 176.0);
pub const SLOT: Rect = (106.0, 62.0, 150.0, 84.0);
pub const BTN_LEFT: Rect = (106.0, 152.0, 44.0, 18.0);
pub const BTN_INSERT: Rect = (154.0, 152.0, 54.0, 18.0);
pub const BTN_RIGHT: Rect = (212.0, 152.0, 44.0, 18.0);
pub const BTN_PRINT: Rect = (270.0, 152.0, 106.0, 18.0);

/// Laufendes Fenster.
#[derive(Clone, Debug)]
pub struct MachineWindow {
    /// Automat (Raum im Seitentrakt)
    pub machine: usize,
    /// Flasche im Schacht und ob sie aus der Tuete kam
    pub bottle: Option<(Empty, bool)>,
    /// Drehung 0..TURNS
    pub turn: u8,
    /// Sekunden, seit die Flasche im Schacht liegt
    pub loaded: f32,
    /// Anzeige des Automaten
    pub text: String,
    /// Zeit am Automaten seit dem letzten Arm
    pub t: f32,
    /// der Arm ist draussen (Restzeit)
    pub arm: f32,
    pub mouse: (f32, f32),
    prev: (bool, bool),
}

impl MachineWindow {
    pub fn green(&self) -> bool {
        self.bottle.is_some() && self.loaded >= LAMP_TIME
    }
}

/// Was er sagt, wenn er ablehnt.
fn refusal(tier: Tier, n: usize) -> &'static str {
    let list: &[&str] = match tier {
        Tier::Calm => &["BITTE ERNEUT EINLEGEN"],
        Tier::Uneasy => &["NICHT VON HIER", "DIE WAR NOCH VOLL", "BITTE LÄCHELN"],
        Tier::Wrong => &[
            "DIE GEHÖRT IHR",
            "WER HAT DARAUS GETRUNKEN",
            "NULL NULL NULL",
        ],
        Tier::Collapse => &["NÄHER KOMMEN", "HAND EINLEGEN", "ICH HABE HUNGER"],
    };
    list[n % list.len()]
}

fn thanks(tier: Tier) -> &'static str {
    match tier {
        Tier::Calm => "DANKE",
        Tier::Uneasy => "DANKE. BIS GLEICH",
        Tier::Wrong => "DANKE, KUNDE 0",
        Tier::Collapse => "MEHR",
    }
}

impl Game {
    pub fn machine_window_open(&self) -> bool {
        self.pfand.window.is_some()
    }

    /// Ein eigenes Fenster mit Mauszeiger (Kassenfenster oder Automat).
    pub fn pointer_window_open(&self) -> bool {
        self.belt_window_open() || self.machine_window_open() || self.kiosk_open()
    }

    /// Esc oder B: was gerade offen ist, schliessen.
    pub fn close_pointer_window(&mut self) {
        self.close_belt_window();
        self.close_machine();
        self.close_kiosk();
    }

    /// Laeuft dieser Automat? Ruhig: alle; unruhig: einer nicht; danach nur einer.
    pub fn machine_works(&self, i: usize) -> bool {
        let n = self.world.wing.as_ref().map_or(0, |w| w.machines.len());
        if i >= n {
            return false;
        }
        if self.tutorial_run {
            return true;
        }
        let pick = (self.seed as usize / 3 + self.pfand.shift) % n;
        match self.tier {
            Tier::Calm => true,
            Tier::Uneasy => n == 1 || i != pick,
            Tier::Wrong | Tier::Collapse => i == pick,
        }
    }

    /// Automat, auf den man schaut: (Nummer, Entfernung).
    pub(in crate::game) fn machine_hit(
        &self,
        eye: Vec3,
        dir: Vec3,
        max: f32,
    ) -> Option<(usize, f32)> {
        let wing = self.world.wing.as_ref()?;
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in wing.machines.iter().enumerate() {
            if let Some(t) = m.body().ray(eye, dir, max) {
                if best.is_none_or(|b| t < b.1) {
                    best = Some((i, t));
                }
            }
        }
        best
    }

    /// E am Automaten.
    pub(in crate::game) fn open_machine(&mut self, i: usize) {
        if !self.machine_works(i) {
            self.hint("AUSSER BETRIEB. Vielleicht läuft ein anderer.");
            self.play(Sfx::Error, 0.4);
            return;
        }
        let n = self.world.wing.as_ref().map_or(0, |w| w.machines.len());
        self.pfand.sums.resize(n, 0);
        self.player.vel = Vec3::ZERO;
        self.pfand.window = Some(MachineWindow {
            machine: i,
            bottle: None,
            turn: 0,
            loaded: 0.0,
            text: "FLASCHE EINLEGEN".into(),
            t: 0.0,
            arm: 0.0,
            mouse: (0.0, 0.0),
            // beim Oeffnen gehaltene Tasten zaehlen nicht
            prev: (true, true),
        });
        self.play(Sfx::Machine, 0.4);
        self.load_bottle();
    }

    /// Fenster schliessen; die Flasche im Schacht kommt zurueck.
    pub fn close_machine(&mut self) {
        let Some(w) = self.pfand.window.take() else {
            return;
        };
        if let Some((k, from_bag)) = w.bottle {
            if from_bag && !self.pfand.bag_full() {
                self.pfand.bag.push(k);
            } else if self.held.is_none() {
                self.held = Some(k.item());
            } else {
                let pos = self.player.pos + vec3(0.0, 0.3, 0.0);
                self.loose.push(Loose {
                    data: k.item(),
                    pos,
                    vel: vec3(0.0, 0.2, 0.0),
                    yaw: 0.0,
                });
            }
        }
        self.play(Sfx::Click, 0.3);
    }

    fn bag_reachable(&self) -> bool {
        self.cart_distance() <= BAG_REACH
    }

    /// Naechste Flasche in den Schacht: erst aus der Hand, dann aus der Tuete.
    fn load_bottle(&mut self) {
        if self
            .pfand
            .window
            .as_ref()
            .is_none_or(|w| w.bottle.is_some())
        {
            return;
        }
        let in_hand = self.held.as_ref().and_then(|h| match h.kind {
            PickupKind::Deposit(k) => Some(k),
            _ => None,
        });
        let next = if let Some(k) = in_hand {
            self.held = None;
            Some((k, false))
        } else if self.bag_reachable() {
            self.pfand.bag_changed = self.time;
            self.pfand.bag.pop().map(|k| (k, true))
        } else {
            None
        };
        let turn = self.pfand.rng.gen_range(1..TURNS);
        if let Some(w) = &mut self.pfand.window {
            w.bottle = next;
            w.turn = turn;
            w.loaded = 0.0;
        }
        if next.is_some() {
            self.play(Sfx::Clink, 0.3);
        }
    }

    /// Fenster bedienen. Laeuft statt der normalen Steuerung.
    pub(in crate::game) fn update_machine(&mut self, dt: f32, input: &Input) {
        self.player.vel = Vec3::ZERO;
        let Some(w) = &mut self.pfand.window else {
            return;
        };
        w.loaded += dt;
        w.t += dt;
        w.mouse = input.ui_mouse;
        if w.arm > 0.0 {
            w.arm -= dt;
            if w.arm <= 0.0 {
                self.close_machine();
            }
            return;
        }
        let m = w.mouse;
        let click = |r: Rect| input.ui_pressed && inside(r, m);
        let left = (input.left && !w.prev.0) as i32 + click(BTN_LEFT) as i32;
        let right = (input.right && !w.prev.1) as i32 + click(BTN_RIGHT) as i32;
        w.prev = (input.left, input.right);
        let steps = right - left + input.wheel;
        if steps != 0 && w.bottle.is_some() {
            w.turn = (w.turn as i32 + steps).rem_euclid(TURNS as i32) as u8;
            self.play(Sfx::Click, 0.2);
        }
        if input.cart_toggle {
            self.close_machine();
            return;
        }
        if input.interact || click(BTN_INSERT) {
            self.insert_bottle();
        }
        if input.take_out || click(BTN_PRINT) {
            self.print_bon();
        }
        // Kollaps: wer zu lange davorsteht ...
        let grab = self.tier == Tier::Collapse
            && !self.tutorial_run
            && self.can_scare(true)
            && self.pfand.window.as_ref().is_some_and(|w| w.t > ARM_AFTER);
        if grab {
            self.machine_arm();
        }
    }

    pub(in crate::game) fn insert_bottle(&mut self) {
        let tier = self.tier;
        let Some(w) = &mut self.pfand.window else {
            return;
        };
        let Some((kind, _)) = w.bottle else {
            w.text = "KEINE FLASCHE".into();
            self.load_bottle();
            return;
        };
        let early = !w.green();
        let wrong = w.turn != 0;
        let refuse = tier > Tier::Calm && !self.tutorial_run && self.pfand.rng.gen_bool(REFUSE);
        if early || wrong || refuse {
            let n = self.pfand.rng.gen_range(0..100usize);
            let turn = self.pfand.rng.gen_range(1..TURNS);
            let w = self.pfand.window.as_mut().expect("Fenster offen");
            w.text = if early {
                "ZU FRÜH".into()
            } else if wrong {
                "STRICHCODE NICHT LESBAR".into()
            } else {
                refusal(tier, n).into()
            };
            w.turn = turn;
            w.loaded = 0.0;
            self.play(Sfx::Error, 0.6);
            self.make_noise(JAM_NOISE);
            return;
        }
        // ab "falsch" behaelt er ein paar Cent
        let keep = if tier >= Tier::Wrong && !self.tutorial_run {
            self.pfand.rng.gen_range(0..=3)
        } else {
            0
        };
        let i = w.machine;
        w.bottle = None;
        w.text = thanks(tier).into();
        if self.pfand.sums.len() <= i {
            self.pfand.sums.resize(i + 1, 0);
        }
        self.pfand.sums[i] += (kind.cents() - keep).max(1);
        self.play(Sfx::Machine, 0.6);
        self.play(Sfx::Clink, 0.4);
        self.load_bottle();
    }

    /// Bon drucken: der Betrag des Automaten wird ein Bon.
    pub(in crate::game) fn print_bon(&mut self) {
        let Some(w) = &mut self.pfand.window else {
            return;
        };
        let i = w.machine;
        let sum = self.pfand.sums.get(i).copied().unwrap_or(0);
        if sum <= 0 {
            w.text = "ERST FLASCHEN EINLEGEN".into();
            self.play(Sfx::Error, 0.4);
            return;
        }
        w.text = format!("BON {}", crate::ui::euro(sum));
        self.pfand.sums[i] = 0;
        self.pfand.bons.push(sum);
        self.pfand.printed = true;
        self.say(KIOSK_HINT, 5.0);
        self.play(Sfx::Print, 0.6);
    }

    /// Ein Arm greift aus der Oeffnung: Schreck, Laerm, die Flasche ist weg. Kein Tod.
    pub(in crate::game) fn machine_arm(&mut self) {
        let Some(w) = &mut self.pfand.window else {
            return;
        };
        w.arm = ARM_TIME;
        w.t = 0.0;
        w.bottle = None;
        w.text = "DANKE".into();
        self.fire_scare(0.9);
        self.make_noise(ARM_NOISE);
    }

    /// Anzeige am Automaten: gruen = laeuft, rot = ausser Betrieb, dazu der Strichcode-Leser.
    pub(super) fn collect_machine_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some(wing) = &self.world.wing else { return };
        for (i, m) in wing.machines.iter().enumerate() {
            let works = self.machine_works(i);
            let blink = if works {
                0.8 + 0.2 * (self.time * 2.0).sin()
            } else {
                0.5 + 0.5 * (self.time * 7.0 + i as f32).sin().signum()
            };
            let color = if works {
                vec3(0.3, 1.0, 0.4)
            } else {
                vec3(1.0, 0.12, 0.08)
            };
            let mut lamp = |at: Vec3, h: Vec3, c: Vec3, glow: f32| {
                out.push(MeshDraw {
                    mesh: MeshId::Cube as usize,
                    model: Mat4::from_scale_rotation_translation(
                        h * 2.0,
                        Quat::IDENTITY,
                        m.pos + at,
                    ),
                    tint: c.extend(glow),
                    ..Default::default()
                });
            };
            lamp(
                vec3(0.365, 1.5, -0.14),
                vec3(0.006, 0.1, 0.2),
                color * 0.5,
                blink,
            );
            // Betrag schon drin: kleine gelbe Lampe
            if self.pfand.sums.get(i).is_some_and(|s| *s > 0) {
                lamp(
                    vec3(0.365, 1.3, 0.28),
                    vec3(0.006, 0.02, 0.02),
                    vec3(1.0, 0.85, 0.2),
                    1.2,
                );
            }
        }
    }
}
