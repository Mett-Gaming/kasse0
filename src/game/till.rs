//! Kassen-Minispiel: Sobald die erste Ware auf dem Band liegt, sind Kunden und Mitarbeiter
//! harmlos, solange man an der Kasse bleibt. Dafuer beobachtet einen die Kassiererin.
//! Man muss die Ware richtig aufs Band legen (alles ab 1 kg zuerst, nichts Falsches), ihre Fragen
//! beantworten, die Karte ruhig durchziehen und die PIN eintippen. Jeder Fehler fuellt ihr
//! Misstrauen. Ist es voll, kostet das eines von drei Kassen-Leben; beim dritten ist der Run vorbei.

use super::*;

/// Kassen-Leben (getrennt von den Leben im Markt)
pub const TILL_LIVES: u32 = 3;
/// Ab diesem Gewicht (kg) ist eine Ware schwer und muss vor allem Leichten aufs Band.
pub const HEAVY_KG: f32 = 1.0;
/// So weit darf man sich waehrend des Kassierens von der Kasse entfernen
pub const TILL_RADIUS: f32 = 3.5;
/// Antwortzeit fuer eine Frage der Kassiererin
pub const QUESTION_TIME: f32 = 7.0;
/// Schneller durchgezogen als das: Karte nicht gelesen (kostet ein Leben)
pub const SWIPE_MIN_TIME: f32 = 0.45;
/// Langsamer als das, oder unterwegs laenger als SWIPE_STALL stehen geblieben: noch mal
pub const SWIPE_MAX_TIME: f32 = 1.3;
pub const SWIPE_STALL: f32 = 0.35;
/// Mausbewegung (in Radiant Blickwinkel), die einmal ganz Durchziehen bedeutet
pub const SWIPE_TRAVEL: f32 = 0.6;
/// So lange zeigt das Terminal die PIN an
pub const PIN_SHOW: f32 = 2.5;
/// So lange hat man zum Eintippen
pub const PIN_TIME: f32 = 10.0;

/// Direkte Fehler kosten sofort ein Leben (`till_mistake`). Weiche Fehler fuellen nur das
/// Misstrauen (1.0 = ein Leben weg):
pub const SUS_NEUTRAL: f32 = 0.2;
pub const SUS_SILENT: f32 = 0.35;
pub const SUS_PIN_SLOW: f32 = 0.2;
pub const SUS_SWIPE_SLOW: f32 = 0.2;
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
        text: "Sind Sie Kunde Nummer 0?",
        answers: ["Nein.", "Vielleicht.", "Ja."],
    },
    // --- absurd-lustige Fragen (die Ware gibt es im Laden gar nicht) ---
    TillQuestion {
        text: "Wieso Kondome und eine Gurke?",
        answers: [
            "Das ist für einen Salat.",
            "Das geht Sie nichts an.",
            "Wollen Sie es wirklich wissen?",
        ],
    },
    TillQuestion {
        text: "Sie haben 47 Tomatensäfte vergessen.",
        answers: [
            "Heute nicht, danke.",
            "47? Wirklich?",
            "Den trinken nur die anderen.",
        ],
    },
    TillQuestion {
        text: "Ist das Ihr Wagen, oder hat er Sie ausgesucht?",
        answers: [
            "Das ist meiner.",
            "Gute Frage ...",
            "Er folgt mir seit Stunden.",
        ],
    },
    TillQuestion {
        text: "Haben Sie Kleingeld? Ich sammle Zähne.",
        answers: [
            "Leider nicht.",
            "Wie viele brauchen Sie?",
            "Ihre oder meine?",
        ],
    },
    TillQuestion {
        text: "Möchten Sie für 1 Cent eine Seele dazu?",
        answers: [
            "Nein, danke.",
            "Ist die noch frisch?",
            "Ich habe schon zwei.",
        ],
    },
    TillQuestion {
        text: "Wussten Sie, dass Gurken Sie beobachten?",
        answers: ["Ach was.", "Nur die grünen?", "Deshalb esse ich sie."],
    },
    TillQuestion {
        text: "Haben Sie Ihr Gesicht selbst ausgesucht?",
        answers: [
            "Das ist angeboren.",
            "Gefällt es Ihnen?",
            "Wollen Sie es haben?",
        ],
    },
    TillQuestion {
        text: "Das Klopapier reicht für wie viele Jahre?",
        answers: [
            "Für ein paar Wochen.",
            "Kommt drauf an.",
            "Bis alles vorbei ist.",
        ],
    },
    TillQuestion {
        text: "Warum schwitzen Sie so?",
        answers: [
            "Es ist warm hier drin.",
            "Tu ich das?",
            "Weil Sie kein Gesicht haben!",
        ],
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
    /// Karte durchziehen (wie eine Kartenleser-Aufgabe): erst aus dem Portemonnaie nehmen
    /// (`ready`), dann mit der Maus gleichmaessig nach rechts durch den Leser ziehen.
    /// `progress` 0..1, `started` = Beginn des Zugs, `last_move` = letzte Bewegung.
    Swipe {
        progress: f32,
        started: Option<f32>,
        ready: bool,
        last_move: f32,
    },
    /// Das Terminal zeigt kurz die PIN
    ShowPin(f32),
    /// PIN eintippen (Restzeit)
    Pin(f32),
    /// Bar und passend aus der Geldboerse bezahlen (Restzeit), siehe cash.rs
    Cash(f32),
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
    /// Liegt schon etwas Leichtes (unter HEAVY_KG) auf dem Band?
    other_on_belt: bool,
    /// Ist man gerade an der Kasse (fuer den Hinweis beim Weggehen)?
    was_near: bool,
    /// Heute nur Barzahlung ("Kartenzahlung geht nicht")
    pub cash: bool,
    /// Geldboerse: Anzahl je Schein/Muenze (cash::DENOMS)
    pub wallet: [u32; cash::DENOMS.len()],
    /// schon hingelegtes Geld (Indizes in cash::DENOMS)
    pub given: Vec<usize>,
    /// gewaehlter Schein/Muenze
    pub sel: usize,
    /// A/D im letzten Bild gedrueckt (zum Weiterschalten)
    lr_prev: (bool, bool),
    /// Anzeige des Kartenlesers ("ZU SCHNELL" ...) und wie lange noch
    pub reader: Option<(&'static str, f32)>,
    /// Sekunden seit der ersten Ware auf dem Band (treibt den Stress, pressure.rs)
    pub time: f32,
    /// Sie schaut gerade auf (pressure.rs) und Wartezeit bis zum naechsten Mal
    pub look: Option<pressure::LookUp>,
    look_wait: f32,
    /// Takte fuer Genoergel, Durchsagen, Flackern, Klopfen und Herzschlag (pressure.rs)
    nag_wait: f32,
    announce_wait: f32,
    flicker_wait: f32,
    knock_wait: f32,
    heart_acc: f32,
    /// Kassenbon mit eventuell falsch gescannten Zeilen (receipt.rs)
    pub receipt: Vec<receipt::ReceiptLine>,
    /// Zeit fuers Barzahlen (cash.rs, haengt vom Stress beim Start ab)
    pub cash_limit: f32,
}

impl Till {
    fn new(rng: &mut Pcg64) -> Self {
        let first = rng.gen_range(1..=2);
        let second = first + rng.gen_range(2..=3);
        Self {
            cash: rng.gen_bool(cash::CASH_CHANCE),
            wallet: [0; cash::DENOMS.len()],
            given: Vec::new(),
            sel: 0,
            lr_prev: (false, false),
            reader: None,
            time: 0.0,
            look: None,
            look_wait: rng.gen_range(pressure::LOOK_EVERY.0..pressure::LOOK_EVERY.1),
            nag_wait: rng.gen_range(pressure::NAG_EVERY.0..pressure::NAG_EVERY.1),
            announce_wait: rng.gen_range(6.0..12.0),
            flicker_wait: rng.gen_range(3.0..8.0),
            knock_wait: 0.0,
            heart_acc: 0.0,
            receipt: Vec::new(),
            cash_limit: cash::CASH_TIME_MAX,
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
    /// Gewicht in kg (aus dem Katalog; eine geworfene Tomate wiegt 100 g).
    pub fn weight_kg(&self) -> f32 {
        match self.kind {
            PickupKind::Product { product, .. } => catalog().products[product].kg,
            PickupKind::Tomato { .. } => 0.1,
            PickupKind::Deposit(_) => 0.05,
        }
    }

    /// Schwere Waren (ab HEAVY_KG) gehoeren zuerst aufs Band.
    pub fn is_heavy(&self) -> bool {
        self.weight_kg() >= HEAVY_KG
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

    /// Mausbewegung beim Durchziehen (nach rechts = positiv).
    pub(super) fn swipe_move(&mut self, dx: f32) {
        let s = BASE_SENSITIVITY * self.sensitivity;
        let now = self.time;
        if let Some(Till {
            pay:
                Pay::Swipe {
                    progress,
                    started,
                    ready: true,
                    last_move,
                },
            ..
        }) = &mut self.till
        {
            *progress = (*progress + dx * s / SWIPE_TRAVEL).max(0.0);
            if *progress > 0.03 {
                if started.is_none() {
                    *started = Some(now);
                }
                if dx.abs() > 0.0 {
                    *last_move = now;
                }
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
        let heavy = data.is_heavy();
        let t = self.till.as_mut().unwrap();
        if heavy && t.other_on_belt {
            self.till_mistake("Kassiererin: \"Schwere Sachen zuerst.\"");
        } else if !heavy {
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

    /// Cheat/Test: Zettel gescannt, alle Fragen beantwortet, direkt am Terminal bezahlen -
    /// mit Karte (`cash = false`) oder bar.
    pub fn debug_jump_to_pay(&mut self, cash: bool) {
        self.tip();
        self.fill_list_cheap();
        self.scanned.append(&mut self.cart);
        self.belt.clear();
        self.held = None;
        self.dialog = None;
        if self.till.is_none() {
            self.till = Some(Till::new(&mut self.rng));
        }
        let Some(lane) = self.open_lane() else { return };
        let t = self.till.as_mut().unwrap();
        t.ask_at.clear();
        t.asked = None;
        t.cash = cash;
        t.pay = Pay::Idle;
        // an die Kasse, Blick aufs Terminal, Wagen daneben
        self.player.pos = lane.scan_point;
        let to = lane.terminal - (lane.scan_point + vec3(0.0, EYE_HEIGHT, 0.0));
        self.player.yaw = to.x.atan2(-to.z);
        self.player.pitch = (to.y / to.length()).asin();
        self.trolley.pushing = false;
        self.trolley.pos = lane.scan_point + vec3(0.0, 0.0, -1.0);
        self.start_swipe();
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
        // Falsch gescannte Zeilen, die man nicht reklamiert hat, kosten jetzt (receipt.rs)
        if !self.till_charge_unnoticed() {
            return;
        }
        let t = self.till.as_mut().unwrap();
        if t.cash {
            self.start_cash();
            return;
        }
        t.pay = Pay::Swipe {
            progress: 0.0,
            started: None,
            ready: false,
            last_move: 0.0,
        };
        t.reader = None;
        self.player.vel = Vec3::ZERO;
        self.say(
            "Nimm die Karte [E] und zieh sie gleichmäßig nach rechts durch.",
            4.0,
        );
        self.play(Sfx::Click, 0.4);
    }

    /// Bezahlen am Terminal (Karte, PIN). Waehrenddessen steht man still.
    pub(super) fn update_till_pay(&mut self, dt: f32, input: &Input) {
        self.player.vel = Vec3::ZERO;
        let now = self.time;
        let Some(t) = &mut self.till else { return };
        if let Some((_, left)) = &mut t.reader {
            *left -= dt;
            if *left <= 0.0 {
                t.reader = None;
            }
        }
        let again = Pay::Swipe {
            progress: 0.0,
            started: None,
            ready: true,
            last_move: 0.0,
        };
        match t.pay {
            Pay::Idle => {}
            Pay::Cash(left) => self.update_cash(left, dt, input),
            Pay::Swipe {
                progress,
                started,
                ready,
                last_move,
            } => {
                if input.take_out {
                    t.pay = Pay::Idle;
                    self.say("Abgebrochen.", 1.5);
                    return;
                }
                // erst die Karte aus dem Portemonnaie nehmen
                if !ready {
                    if input.interact {
                        t.pay = Pay::Swipe {
                            progress: 0.0,
                            started: None,
                            ready: true,
                            last_move: 0.0,
                        };
                        self.play(Sfx::Pickup, 0.4);
                    }
                    return;
                }
                let Some(s) = started else { return };
                let took = now - s;
                if progress >= 1.0 && took < SWIPE_MIN_TIME {
                    t.pay = again;
                    t.reader = Some(("ZU SCHNELL. NOCHMAL.", 1.6));
                    self.till_mistake("Zu schnell! KARTE NICHT GELESEN.");
                } else if progress >= 1.0 && took <= SWIPE_MAX_TIME {
                    t.pay = Pay::ShowPin(PIN_SHOW);
                    t.typed.clear();
                    t.reader = Some(("ANGENOMMEN", 1.6));
                    self.play(Sfx::Beep, 0.5);
                    self.say("Karte gelesen. Merk dir die PIN.", 2.5);
                } else if took > SWIPE_MAX_TIME || now - last_move > SWIPE_STALL {
                    // zu langsam oder unterwegs stehen geblieben
                    t.pay = again;
                    t.reader = Some(("ZU LANGSAM. NOCHMAL.", 1.6));
                    self.till_doubt(SUS_SWIPE_SLOW, "Zu langsam. Die Kassiererin trommelt.");
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
                        self.till_mistake("PIN FALSCH.");
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
        // Druck: Ungeduld, Aufschauen, Atmosphaere (pressure.rs); R reklamiert (receipt.rs)
        self.update_till_pressure(dt);
        if input.complain {
            self.till_complain();
        }
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
                    let v = choice.map(|k| verdict(a.order[k]));
                    let (reply, sus) = match v {
                        Some(Verdict::Good) => (REPLY_GOOD, 0.0),
                        Some(Verdict::Neutral) => (REPLY_NEUTRAL, SUS_NEUTRAL),
                        Some(Verdict::Bad) => (REPLY_BAD, 0.0),
                        None => (REPLY_SILENT, SUS_SILENT),
                    };
                    a.reply = Some((reply, 1.8));
                    self.play(Sfx::Blip, 0.6);
                    if v == Some(Verdict::Bad) {
                        self.till_mistake("Falsche Antwort.");
                    } else if sus > 0.0 {
                        let msg = if choice.is_none() {
                            "Du hast geschwiegen. Sie wird misstrauisch."
                        } else {
                            "Komische Antwort. Sie wird misstrauisch."
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

        // Misstrauen voll: ein Leben weg, der Balken beginnt von vorn
        if t.doubt < 1.0 {
            return;
        }
        t.doubt = 0.0;
        self.till_lose_life("Ihr Misstrauen ist zu groß.");
    }

    /// Direkter Fehler (Schweres nicht zuerst, falsche Ware, schlechte Antwort, Karte zu
    /// schnell, falsche PIN, zu viel Geld): kostet sofort ein Kassen-Leben.
    pub(super) fn till_mistake(&mut self, msg: &str) {
        self.play(Sfx::Error, 0.6);
        self.till_lose_life(msg);
    }

    /// Ein Kassen-Leben weg; beim letzten ist der Run vorbei.
    fn till_lose_life(&mut self, reason: &str) {
        let tutorial = self.tutorial_run;
        let Some(t) = &mut self.till else { return };
        if t.lives == 0 {
            return;
        }
        // Tutorial: kein Leben weg, Misstrauen zurueck auf null, nur erklaeren
        if tutorial {
            t.doubt = 0.0;
            self.tutorial_forgive(&format!(
                "{reason} Im echten Spiel kostet das ein Kassen-Leben."
            ));
            return;
        }
        t.lives -= 1;
        let lives = t.lives;
        self.red = 1.0;
        self.glitch = 1.0;
        self.play(Sfx::Sting, 1.0);
        if lives == 0 {
            self.till_caught = true;
            self.outcome = Outcome::Caught;
            self.outcome_timer = 0.0;
            self.catcher = self.open_lane().map(|l| l.seat - vec3(0.0, 0.4, 0.0));
            self.say(reason, 5.0);
            return;
        }
        self.say(
            format!(
                "{reason} Die Kassiererin hebt den Kopf - ein Leben weg. (Noch {lives} an der Kasse)"
            ),
            5.0,
        );
    }
}

pub mod cash;
pub mod pressure;
pub mod receipt;
#[cfg(test)]
mod tests;
