//! Kassen-Minispiel: Sobald die erste Ware auf dem Band liegt, sind Kunden und Mitarbeiter
//! harmlos, solange man an der Kasse bleibt. Dafuer beobachtet einen die Kassiererin.
//! Man muss die Ware richtig aufs Band legen (Getraenke zuerst, nichts Falsches), ihre Fragen
//! beantworten, die Karte ruhig durchziehen und die PIN eintippen. Jeder Fehler fuellt ihr
//! Misstrauen. Ist es voll, kostet das eines von drei Kassen-Leben; beim dritten ist der Run vorbei.

use super::*;

/// Kassen-Leben (getrennt von den Leben im Markt)
pub const TILL_LIVES: u32 = 3;
/// So weit darf man sich waehrend des Kassierens von der Kasse entfernen
pub const TILL_RADIUS: f32 = 3.5;
/// Antwortzeit fuer eine Frage der Kassiererin
pub const QUESTION_TIME: f32 = 7.0;
/// Schneller durchgezogen als das: Karte nicht gelesen
pub const SWIPE_MIN_TIME: f32 = 0.45;
/// Mausbewegung (in Radiant Blickwinkel), die einmal ganz Durchziehen bedeutet
pub const SWIPE_TRAVEL: f32 = 0.6;
/// So lange zeigt das Terminal die PIN an
pub const PIN_SHOW: f32 = 2.5;
/// So lange hat man zum Eintippen
pub const PIN_TIME: f32 = 10.0;

/// Misstrauen je Fehler (1.0 = ein Leben weg)
pub const SUS_ORDER: f32 = 0.35;
pub const SUS_FAKE: f32 = 0.3;
pub const SUS_NEUTRAL: f32 = 0.2;
pub const SUS_BAD: f32 = 0.45;
pub const SUS_SILENT: f32 = 0.35;
pub const SUS_SWIPE: f32 = 0.25;
pub const SUS_PIN: f32 = 0.3;
pub const SUS_PIN_SLOW: f32 = 0.2;
/// pro Sekunde, wenn man waehrend des Kassierens weggeht
pub const SUS_AWAY: f32 = 0.08;

pub struct TillQuestion {
    pub text: &'static str,
    /// [gut, neutral, schlecht]
    pub answers: [&'static str; 3],
}

pub const TILL_QUESTIONS: &[TillQuestion] = &[
    TillQuestion {
        text: "Sammeln Sie Punkte?",
        answers: [
            "Nein, danke.",
            "Ähm ... vielleicht?",
            "Sie haben doch gar keine Augen.",
        ],
    },
    TillQuestion {
        text: "Brauchen Sie eine Tüte?",
        answers: [
            "Nein, ich habe einen Beutel.",
            "Was kostet die denn?",
            "Für Ihren Kopf?",
        ],
    },
    TillQuestion {
        text: "Haben Sie alles gefunden?",
        answers: ["Ja, danke.", "Fast alles.", "Wo ist der Ausgang?!"],
    },
    TillQuestion {
        text: "Möchten Sie den Bon?",
        answers: ["Ja, bitte.", "Egal.", "Ich will nur hier raus."],
    },
    TillQuestion {
        text: "Kommen Sie öfter hierher?",
        answers: ["Jeden Tag.", "Ab und zu.", "Ich war noch nie hier."],
    },
    TillQuestion {
        text: "Zahlen Sie mit Karte?",
        answers: ["Ja, mit Karte.", "Geht auch bar?", "Womit zahlt man hier?"],
    },
    TillQuestion {
        text: "Sind Sie Kunde Nummer 0?",
        answers: ["Nein.", "Vielleicht.", "Ja."],
    },
];

const REPLY_GOOD: &str = "Mhm.";
const REPLY_NEUTRAL: &str = "... Aha.";
const REPLY_BAD: &str = "(Sie hört auf zu scannen und sieht dich an.)";
const REPLY_SILENT: &str = "Hallo? Ich habe Sie etwas gefragt.";

/// Eine laufende Frage der Kassiererin.
pub struct Asked {
    pub question: usize,
    /// order[k] = Index in `answers` fuer die k-te angezeigte Option
    pub order: [usize; 3],
    pub timer: f32,
    pub reply: Option<(&'static str, f32)>,
}

impl Asked {
    pub fn question(&self) -> &'static TillQuestion {
        &TILL_QUESTIONS[self.question]
    }
}

/// Bezahlen am Terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pay {
    Idle,
    /// Karte durchziehen: 0..1, Zeitpunkt, ab dem sie sich bewegt hat
    Swipe {
        progress: f32,
        started: Option<f32>,
    },
    /// Das Terminal zeigt kurz die PIN
    ShowPin(f32),
    /// PIN eintippen (Restzeit)
    Pin(f32),
}

pub struct Till {
    pub lives: u32,
    /// Misstrauen der Kassiererin 0..1
    pub doubt: f32,
    pub pin: [u8; 4],
    pub typed: Vec<u8>,
    pub pay: Pay,
    pub asked: Option<Asked>,
    /// Nach so vielen gescannten Waren stellt sie die naechste Frage
    ask_at: Vec<usize>,
    used: Vec<usize>,
    /// Liegt schon etwas anderes als Getraenke auf dem Band?
    other_on_belt: bool,
    /// Ist man gerade an der Kasse (fuer den Hinweis beim Weggehen)?
    was_near: bool,
}

impl Till {
    fn new(rng: &mut Pcg64) -> Self {
        let first = rng.gen_range(1..=2);
        let second = first + rng.gen_range(2..=3);
        Self {
            lives: TILL_LIVES,
            doubt: 0.0,
            pin: [
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
                rng.gen_range(0..10),
            ],
            typed: Vec::new(),
            pay: Pay::Idle,
            asked: None,
            ask_at: vec![first, second],
            used: Vec::new(),
            other_on_belt: false,
            was_near: true,
        }
    }

    pub fn questions_left(&self) -> bool {
        !self.ask_at.is_empty()
    }
}

impl ItemData {
    /// Getraenke sind schwer und gehoeren zuerst aufs Band.
    pub fn is_drink(&self) -> bool {
        match self.kind {
            PickupKind::Product { product, .. } => {
                let cat = catalog();
                cat.subs[cat.products[product].sub].id == "getraenke"
            }
            PickupKind::Tomato { .. } => true,
        }
    }
}

impl Game {
    /// Laeuft gerade das Kassen-Minispiel (erste Ware liegt auf dem Band, noch nicht bezahlt)?
    pub fn till_active(&self) -> bool {
        self.till.is_some() && !self.paid
    }

    /// Steht man nah genug an der offenen Kasse?
    fn near_till(&self) -> bool {
        self.open_lane()
            .is_some_and(|l| (self.player.pos - l.scan_point).length() < TILL_RADIUS)
    }

    /// An der Kasse sind Kunden und Mitarbeiter harmlos.
    pub fn till_protects(&self) -> bool {
        self.till_active() && self.near_till()
    }

    /// Wird gerade bezahlt (Karte, PIN)? Dann steht man still.
    pub fn till_paying(&self) -> bool {
        self.till_active() && self.till.as_ref().is_some_and(|t| t.pay != Pay::Idle)
    }

    /// Wird die Karte gerade durchgezogen? Dann bewegt die Maus die Karte, nicht den Blick.
    pub fn swiping(&self) -> bool {
        self.till_active() && matches!(self.till.as_ref().map(|t| t.pay), Some(Pay::Swipe { .. }))
    }

    /// Mausbewegung beim Durchziehen (nach unten = positiv).
    pub(super) fn swipe_move(&mut self, dy: f32) {
        let s = BASE_SENSITIVITY * self.sensitivity;
        let now = self.time;
        if let Some(Till {
            pay: Pay::Swipe { progress, started },
            ..
        }) = &mut self.till
        {
            *progress = (*progress + dy * s / SWIPE_TRAVEL).max(0.0);
            if *progress > 0.05 && started.is_none() {
                *started = Some(now);
            }
        }
    }

    /// Kassiererin wird misstrauischer.
    pub(super) fn till_doubt(&mut self, amount: f32, msg: impl Into<String>) {
        let Some(t) = &mut self.till else { return };
        t.doubt += amount;
        self.glitch = self.glitch.max(0.5);
        self.red = self.red.max(0.3);
        self.play(Sfx::Error, 0.6);
        self.say(msg, 3.0);
    }

    /// Ware kommt aufs Band: Minispiel starten und die Band-Regeln pruefen.
    pub(super) fn till_on_place(&mut self, data: &ItemData) {
        if self.paid {
            return;
        }
        if self.till.is_none() {
            self.till = Some(Till::new(&mut self.rng));
            self.announcement = Some((
                "Kasse 0 ist jetzt für Sie geöffnet. Bitte verhalten Sie sich normal.".into(),
                5.0,
            ));
        }
        let drink = data.is_drink();
        let t = self.till.as_mut().unwrap();
        if drink && t.other_on_belt {
            self.till_doubt(
                SUS_ORDER,
                "Kassiererin: \"Schwere Sachen zuerst.\" Sie mustert dich.",
            );
        } else if !drink {
            t.other_on_belt = true;
        }
    }

    /// Nach jedem Scan: eventuell eine Frage stellen.
    pub(super) fn till_after_scan(&mut self) {
        let n = self.scanned.len();
        let due = self
            .till
            .as_ref()
            .is_some_and(|t| t.asked.is_none() && t.ask_at.first().is_some_and(|&a| n >= a));
        if due {
            self.ask_question();
        }
    }

    fn ask_question(&mut self) {
        let Some(t) = &mut self.till else { return };
        if t.ask_at.is_empty() {
            return;
        }
        t.ask_at.remove(0);
        let free: Vec<usize> = (0..TILL_QUESTIONS.len())
            .filter(|q| !t.used.contains(q))
            .collect();
        let question = free[self.rng.gen_range(0..free.len())];
        t.used.push(question);
        let mut order = [0usize, 1, 2];
        order.shuffle(&mut self.rng);
        t.asked = Some(Asked {
            question,
            order,
            timer: QUESTION_TIME,
            reply: None,
        });
        self.play(Sfx::Blip, 0.6);
    }

    /// E am Kartenterminal: Karte durchziehen beginnen.
    pub(super) fn start_swipe(&mut self) {
        if self.till.is_none() {
            self.till = Some(Till::new(&mut self.rng));
        }
        let t = self.till.as_mut().unwrap();
        if t.asked.is_some() || t.questions_left() {
            self.hint("Die Kassiererin will noch etwas von dir wissen.");
            return;
        }
        t.pay = Pay::Swipe {
            progress: 0.0,
            started: None,
        };
        self.player.vel = Vec3::ZERO;
        self.say(
            "Karte durchziehen: Maus ruhig nach unten. Nicht zu schnell.",
            4.0,
        );
        self.play(Sfx::Click, 0.4);
    }

    /// Bezahlen am Terminal (Karte, PIN). Waehrenddessen steht man still.
    pub(super) fn update_till_pay(&mut self, dt: f32, input: &Input) {
        self.player.vel = Vec3::ZERO;
        let now = self.time;
        let Some(t) = &mut self.till else { return };
        match t.pay {
            Pay::Idle => {}
            Pay::Swipe { progress, started } => {
                if input.interact {
                    t.pay = Pay::Idle;
                    self.say("Abgebrochen.", 1.5);
                    return;
                }
                if progress < 1.0 {
                    return;
                }
                let took = started.map_or(0.0, |s| now - s);
                if took < SWIPE_MIN_TIME {
                    t.pay = Pay::Swipe {
                        progress: 0.0,
                        started: None,
                    };
                    self.till_doubt(
                        SUS_SWIPE,
                        "Zu schnell! KARTE NICHT GELESEN. Sie sieht von der Kasse auf.",
                    );
                } else {
                    t.pay = Pay::ShowPin(PIN_SHOW);
                    t.typed.clear();
                    self.play(Sfx::Beep, 0.5);
                    self.say("Karte gelesen. Merk dir die PIN.", 2.5);
                }
            }
            Pay::ShowPin(left) => {
                let left = left - dt;
                t.pay = if left <= 0.0 {
                    Pay::Pin(PIN_TIME)
                } else {
                    Pay::ShowPin(left)
                };
            }
            Pay::Pin(left) => {
                if let Some(d) = input.digit {
                    let k = t.typed.len();
                    if t.pin[k] == d {
                        t.typed.push(d);
                        let done = t.typed.len() == 4;
                        if done {
                            t.pay = Pay::Idle;
                        }
                        self.play(Sfx::Click, 0.4);
                        if done {
                            self.pay();
                        }
                    } else {
                        t.typed.clear();
                        t.pay = Pay::ShowPin(PIN_SHOW);
                        self.till_doubt(SUS_PIN, "PIN FALSCH. Die Kassiererin wartet.");
                    }
                    return;
                }
                let left = left - dt;
                if left <= 0.0 {
                    t.typed.clear();
                    t.pay = Pay::ShowPin(PIN_SHOW);
                    self.till_doubt(SUS_PIN_SLOW, "Zu langsam. \"Wissen Sie Ihre PIN nicht?\"");
                } else {
                    t.pay = Pay::Pin(left);
                }
            }
        }
    }

    /// Fragen, Weggehen, Misstrauen und Leben.
    pub(super) fn update_till(&mut self, dt: f32, input: &Input) {
        if !self.till_active() {
            return;
        }
        let near = self.near_till();
        let all_scanned = self.belt.is_empty() && self.scanned_complete();
        let Some(t) = &mut self.till else { return };

        // Frage beantworten
        if let Some(a) = &mut t.asked {
            if let Some((_, left)) = &mut a.reply {
                *left -= dt;
                if *left <= 0.0 {
                    t.asked = None;
                }
            } else {
                a.timer -= dt;
                let choice = input.choice.filter(|&c| c < 3);
                if choice.is_some() || a.timer <= 0.0 {
                    let (reply, sus) = match choice.map(|k| verdict(a.order[k])) {
                        Some(Verdict::Good) => (REPLY_GOOD, 0.0),
                        Some(Verdict::Neutral) => (REPLY_NEUTRAL, SUS_NEUTRAL),
                        Some(Verdict::Bad) => (REPLY_BAD, SUS_BAD),
                        None => (REPLY_SILENT, SUS_SILENT),
                    };
                    a.reply = Some((reply, 1.8));
                    self.play(Sfx::Blip, 0.6);
                    if sus > 0.0 {
                        let msg = if choice.is_none() {
                            "Du hast geschwiegen. Sie wird misstrauisch."
                        } else {
                            "Falsche Antwort. Sie wird misstrauisch."
                        };
                        self.till_doubt(sus, msg);
                    }
                }
            }
        } else if all_scanned && t.questions_left() {
            // Zu wenig Ware fuer alle Fragen: den Rest stellt sie am Ende
            self.ask_question();
        }

        // Weggehen faellt auf (und an der Kasse ist man nicht mehr sicher)
        let Some(t) = &mut self.till else { return };
        if !near {
            t.doubt += SUS_AWAY * dt;
            if t.was_near {
                self.say(
                    "Sie sieht dir nach. Bleib an der Kasse - hier draußen bist du nicht sicher.",
                    4.0,
                );
            }
        }
        let Some(t) = &mut self.till else { return };
        t.was_near = near;

        // Misstrauen voll: ein Leben weg
        if t.doubt < 1.0 {
            return;
        }
        t.doubt = 0.0;
        t.lives = t.lives.saturating_sub(1);
        let lives = t.lives;
        self.red = 1.0;
        self.glitch = 1.0;
        self.play(Sfx::Sting, 1.0);
        if lives == 0 {
            self.till_caught = true;
            self.outcome = Outcome::Caught;
            self.outcome_timer = 0.0;
            self.catcher = self.open_lane().map(|l| l.seat - vec3(0.0, 0.4, 0.0));
            return;
        }
        self.say(
            format!(
                "Die Kassiererin hebt den Kopf. Sie hat dich bemerkt. (Noch {lives} Leben an der Kasse)"
            ),
            5.0,
        );
    }
}

#[cfg(test)]
mod tests;
