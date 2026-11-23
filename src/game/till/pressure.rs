//! Druck an der Kasse. Je laenger man dort steht und je mehr Kassen-Leben schon weg sind,
//! desto hoeher der Stress (0..1), und mit ihm wird alles schlimmer:
//! - die Kassiererin wird von allein ungeduldig (Misstrauen steigt langsam) und noergelt,
//! - sie schaut ab und zu auf: wer sich dann bewegt oder ihr ins leere Gesicht schaut, faellt auf,
//! - Herzschlag, eigene Durchsagen, flackerndes Licht und Klopfen am Absperrband.

use super::*;

/// Stress: Startwert, Anstieg ueber die Zeit (bis `STRESS_TIME` Sekunden), je verlorenem Leben
const STRESS_START: f32 = 0.2;
const STRESS_FROM_TIME: f32 = 0.5;
const STRESS_TIME: f32 = 90.0;
const STRESS_PER_LIFE: f32 = 0.2;

/// Ungeduld: so viel Misstrauen pro Sekunde (Grundwert + Anteil mal Stress)
pub const IMPATIENCE_BASE: f32 = 0.004;
pub const IMPATIENCE_STRESS: f32 = 0.016;

/// Aufschauen: Abstand (Sekunden, geteilt durch 0,5 + Stress), Vorwarnung, Starrdauer
pub const LOOK_EVERY: (f32, f32) = (6.0, 14.0);
pub const LOOK_WARN: f32 = 0.6;
const LOOK_STARE: (f32, f32) = (1.5, 2.5);
/// So viel Misstrauen, wenn man sich beim Aufschauen bewegt oder sie anstarrt
pub const SUS_STARE: f32 = 0.4;
/// Ab hier gilt "ins Gesicht schauen" (Kosinus) bzw. "bewegt sich" (m/s)
const GAZE_COS: f32 = 0.97;
const MOVE_SPEED: f32 = 0.3;

/// Genoergel: Abstand in Sekunden (wird mit dem Stress kuerzer)
pub const NAG_EVERY: (f32, f32) = (15.0, 25.0);
const NAGS: &[&str] = &[
    "Kassiererin: \"Geht das auch schneller?\"",
    "Kassiererin: \"Hinter Ihnen warten Leute.\"",
    "Kassiererin: \"Ich habe nicht ewig Zeit. Ich habe gar keine Zeit.\"",
    "Kassiererin: \"Tick. Tack.\"",
    "Kassiererin: \"Sind Sie bald fertig, Kunde 0?\"",
];

/// Eigene Durchsagen an der Kasse
const TILL_ANNOUNCEMENTS: &[&str] = &[
    "Kunde 0, bitte zur Kasse 0.",
    "Kasse 0 schließt gleich. Kasse 0 schließt nie.",
    "Bitte verhalten Sie sich normal.",
    "Wir sehen Sie, Kunde 0.",
    "Bitte zahlen Sie zügig. Bitte bleiben Sie.",
    "Kasse 0 freut sich auf Ihre Ankunft. Seit Jahren.",
    "Bitte legen Sie alles aufs Band. Auch das, was Sie mitgebracht haben.",
    "Die Kassiererin hat heute Geburtstag. Seit 1987.",
    "Bitte lächeln Sie. Die Kamera sieht es nicht.",
    "Kartenzahlung ist möglich. Bargeld ist möglich. Gehen ist nicht möglich.",
];

/// Klopfen: so nah am Absperrband muss jemand stehen
const KNOCK_RANGE: f32 = 1.2;

/// Sie schaut auf: erst die Vorwarnung (Kopf ruckt hoch), dann starrt sie.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookUp {
    /// Restzeit der Vorwarnung
    pub warn: f32,
    /// Restzeit des Starrens
    pub stare: f32,
    /// schon einmal erwischt (zaehlt nur einmal pro Aufblick)
    pub caught: bool,
}

impl Game {
    /// Stress an der Kasse 0..1 (Zeit an der Kasse und verlorene Kassen-Leben).
    pub fn till_stress(&self) -> f32 {
        let Some(t) = &self.till else { return 0.0 };
        let time = (t.time / STRESS_TIME).min(1.0) * STRESS_FROM_TIME;
        let lost = (TILL_LIVES - t.lives) as f32 * STRESS_PER_LIFE;
        (STRESS_START + time + lost).clamp(0.0, 1.0)
    }

    /// Schaut die Kassiererin gerade auf (Vorwarnung oder Starren)? Dann scannt sie nicht.
    pub fn cashier_looking(&self) -> bool {
        self.till_active() && self.till.as_ref().is_some_and(|t| t.look.is_some())
    }

    /// Starrt sie gerade (nach der Vorwarnung)?
    pub fn cashier_staring(&self) -> bool {
        self.till_active()
            && self
                .till
                .as_ref()
                .and_then(|t| t.look)
                .is_some_and(|l| l.warn <= 0.0)
    }

    /// Schaut man ihr gerade ins (leere) Gesicht?
    fn gazing_at_cashier(&self) -> bool {
        let Some(lane) = self.open_lane() else {
            return false;
        };
        let head = lane.seat + vec3(0.0, 1.6, 0.0);
        let to = head - self.eye();
        let d = to.length();
        d > 0.01 && self.view_dir().dot(to / d) > GAZE_COS
    }

    /// Steht jemand am Absperrband (knapp ausserhalb des Kassenbereichs)?
    fn someone_at_barrier(&self) -> bool {
        let w = &self.world;
        self.customers
            .iter()
            .map(|c| c.pos)
            .chain(self.employees.iter().map(|e| e.pos))
            .any(|p| w.in_checkout_zone(p, KNOCK_RANGE) && !w.in_checkout_zone(p, 0.0))
    }

    pub(in crate::game) fn update_till_pressure(&mut self, dt: f32) {
        if let Some(t) = &mut self.till {
            t.time += dt;
        }
        if self.calm_till {
            return;
        }
        let s = self.till_stress();
        let paying = self.till_paying();
        // im Kassenfenster achtet sie weder auf die Maus noch auf den Blick
        let window = self.belt_window_open();
        let moving =
            !window && vec3(self.player.vel.x, 0.0, self.player.vel.z).length() > MOVE_SPEED;
        let gazing = !window && self.gazing_at_cashier();
        let knock_here = self.someone_at_barrier();
        let doubt_now = self.till.as_ref().map_or(0.0, |t| t.doubt);
        let Some(t) = &mut self.till else { return };
        let rng = &mut self.rng;

        // 1. Ungeduld: Misstrauen steigt von allein (nicht waehrend man bezahlt)
        if !paying {
            t.doubt += (IMPATIENCE_BASE + IMPATIENCE_STRESS * s) * dt;
        }
        let mut nag = None;
        t.nag_wait -= dt * (0.6 + s);
        if t.nag_wait <= 0.0 {
            t.nag_wait = rng.gen_range(NAG_EVERY.0..NAG_EVERY.1);
            if !paying && t.asked.is_none() {
                nag = Some(NAGS[rng.gen_range(0..NAGS.len())]);
            }
        }

        // 2. Sie schaut auf (nicht beim Bezahlen)
        let mut creak = false;
        let mut stared = false;
        if paying {
            t.look = None;
        } else if let Some(l) = &mut t.look {
            if l.warn > 0.0 {
                l.warn -= dt;
            } else {
                l.stare -= dt;
                if (moving || gazing) && !l.caught {
                    l.caught = true;
                    stared = true;
                }
                if l.stare <= 0.0 {
                    t.look = None;
                    t.look_wait = rng.gen_range(LOOK_EVERY.0..LOOK_EVERY.1) / (0.5 + s);
                }
            }
        } else {
            t.look_wait -= dt;
            if t.look_wait <= 0.0 {
                t.look = Some(LookUp {
                    warn: LOOK_WARN,
                    stare: rng.gen_range(LOOK_STARE.0..LOOK_STARE.1),
                    caught: false,
                });
                creak = true;
            }
        }

        // 3. Atmosphaere: Herzschlag, Durchsagen, Flackern, Klopfen
        let k = (0.5 * s + 0.5 * doubt_now).clamp(0.0, 1.0);
        let mut heart = None;
        t.heart_acc += dt;
        if t.heart_acc >= 1.1 - 0.7 * k {
            t.heart_acc = 0.0;
            heart = Some(0.25 + 0.45 * k);
        }
        let mut announce = None;
        t.announce_wait -= dt * (0.6 + s);
        if t.announce_wait <= 0.0 {
            t.announce_wait = rng.gen_range(12.0..25.0);
            announce = Some(TILL_ANNOUNCEMENTS[rng.gen_range(0..TILL_ANNOUNCEMENTS.len())]);
        }
        let mut flicker = false;
        t.flicker_wait -= dt * (0.5 + s);
        if t.flicker_wait <= 0.0 {
            t.flicker_wait = rng.gen_range(4.0..10.0);
            flicker = true;
        }
        let mut knock = false;
        if knock_here {
            t.knock_wait -= dt;
            if t.knock_wait <= 0.0 {
                t.knock_wait = rng.gen_range(1.5..3.0);
                knock = true;
            }
        }

        if let Some(n) = nag {
            self.say(n, 3.0);
            self.play(Sfx::Blip, 0.5);
        }
        if creak {
            self.play(Sfx::Creak, 0.6);
        }
        if stared {
            let msg = if gazing {
                "Sie hat gesehen, wie du sie ansiehst."
            } else {
                "Sie hat gesehen, wie du dich bewegst."
            };
            self.till_doubt(SUS_STARE, msg);
        }
        if let Some(v) = heart {
            self.play(Sfx::Heartbeat, v);
        }
        if let Some(a) = announce {
            self.announcement = Some((a.to_string(), 5.0));
            self.play(Sfx::Chime, 0.6);
        }
        if flicker {
            self.glitch = self.glitch.max(0.45 + 0.4 * s);
            self.red = self.red.max(0.2 * s);
        }
        if knock {
            self.play(Sfx::Knock, 0.45 + 0.35 * s);
        }
    }
}
