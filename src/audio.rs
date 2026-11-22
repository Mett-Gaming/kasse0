//! Audio: alles wird beim Start synthetisiert (keine Sounddateien).
//! Kaufhaus-Gedudel, Neonbrummen, Durchsage-Gong, Scanner-Piep, Schritte, Schock.

use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink, Source};
use std::f32::consts::TAU;

const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    Beep,
    Chime,
    Step,
    Sting,
    Blip,
    Click,
    Error,
    Pickup,
    /// Einkaufswagen beim Rennen: schrill
    Squeak,
    /// Einkaufswagen beim normalen Schieben: leise, tiefer
    SqueakSoft,
    /// Einkaufswagen in der Kurve: klappernde Raeder
    Rattle,
    /// elektrische Funken an einem Kabelende
    Spark,
    /// Kunden fluestern miteinander
    Whisper,
    /// Alarmsirene nach dem Bezahlen (Kunde 0 hat ein Gesicht)
    Alarm,
    /// Muenze oder Schein auf den Tresen
    Coin,
    /// Feuerzeug: Reibrad und Flamme
    Lighter,
    /// Zug an der Zigarette: Knistern der Glut
    Puff,
    /// Rauch ausatmen
    Exhale,
    /// Jumpscare: greller, schiefer Akkord mit Rauschstoss
    Scare,
    /// Troll-Jumpscare im Menue: verzerrter Schrei, Metallkreischen, Basseinschlag
    Shriek,
    /// schwerer, rennender Schritt eines Verfolgers (Flucht)
    Stomp,
    /// Herzschlag (Flucht): zwei dumpfe Schlaege
    Heartbeat,
    /// Kassiererin hebt ruckartig den Kopf: Knarzen von Stuhl und Nacken
    Creak,
    /// Klopfen am Absperrband
    Knock,
    /// schlurfender Schritt (Untote im Kollaps)
    Shuffle,
    /// hohes, kratzendes Kreischen (wie ein Bogen auf Metall)
    Screech,
    /// tiefer, dumpfer Schlag
    Thud,
    /// Fernsehrauschen
    Static,
    /// anschwellendes tiefes Grollen, das abrupt abbricht
    Rumble,
    /// Schluesselbund des Personals
    Keys,
}

/// Bis zu dieser Entfernung hoert man Geraeusche mit Ort (Meter)
pub const HEAR_RANGE: f32 = 18.0;

/// Was die Musik gerade tun soll.
pub struct Mood {
    pub playing: bool,
    pub tipped: bool,
    /// 0..3 (Eskalationsstufe)
    pub tier: u8,
    pub time: f32,
    pub paused: bool,
    /// 0..1: Stille vor einem Schreck (Musik und Brummen weg)
    pub hush: f32,
}

pub struct Audio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    music: Sink,
    hum: Sink,
    sounds: Vec<(Sfx, Vec<f32>)>,
    master: f32,
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn secs(s: f32) -> usize {
    (s * RATE as f32) as usize
}

fn note(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

/// E-Piano-artiger Ton mit exponentiellem Abklingen.
fn add_tone(buf: &mut [f32], start: f32, dur: f32, freq: f32, vol: f32) {
    let s0 = secs(start);
    for i in 0..secs(dur) {
        let Some(out) = buf.get_mut(s0 + i) else {
            break;
        };
        let t = i as f32 / RATE as f32;
        let env = (t * 60.0).min(1.0) * (-t * 3.0 / dur).exp();
        let w = (TAU * freq * t).sin() + 0.3 * (TAU * freq * 2.0 * t).sin() * (-t * 6.0).exp();
        *out += w * env * vol;
    }
}

pub fn muzak() -> Vec<f32> {
    // I - vi - IV - V in C-Dur, 4 Takte a 2 s, zweimal mit Melodievariation
    let mut buf = vec![0.0; secs(16.0)];
    let chords: [[f32; 3]; 4] = [
        [60.0, 64.0, 67.0],
        [57.0, 60.0, 64.0],
        [53.0, 57.0, 60.0],
        [55.0, 59.0, 62.0],
    ];
    let melody: [f32; 16] = [
        76.0, 74.0, 72.0, 74.0, 72.0, 69.0, 67.0, 69.0, 69.0, 72.0, 74.0, 72.0, 71.0, 67.0, 74.0,
        71.0,
    ];
    for rep in 0..2 {
        for (ci, chord) in chords.iter().enumerate() {
            let bar = (rep * 4 + ci) as f32 * 2.0;
            for beat in 0..4 {
                let t = bar + beat as f32 * 0.5;
                for &n in chord {
                    add_tone(&mut buf, t, 0.45, note(n), 0.05);
                }
            }
            add_tone(&mut buf, bar, 1.0, note(chord[0] - 24.0), 0.12);
            add_tone(&mut buf, bar + 1.0, 1.0, note(chord[0] - 17.0), 0.1);
            for k in 0..4 {
                let m = melody[(ci * 4 + k + rep * 2) % 16];
                add_tone(&mut buf, bar + k as f32 * 0.5, 0.5, note(m), 0.07);
            }
        }
    }
    buf
}

pub fn hum() -> Vec<f32> {
    let mut rng = Rng(0x1234_5678);
    (0..secs(3.0))
        .map(|i| {
            let t = i as f32 / RATE as f32;
            0.30 * (TAU * 100.0 * t).sin()
                + 0.15 * (TAU * 200.0 * t).sin()
                + 0.06 * (TAU * 300.0 * t).sin()
                + 0.03 * rng.next()
        })
        .collect()
}

fn synth(sfx: Sfx) -> Vec<f32> {
    let mut rng = Rng(0x9E37_79B9);
    let gen = |dur: f32, f: &mut dyn FnMut(f32) -> f32| {
        (0..secs(dur)).map(|i| f(i as f32 / RATE as f32)).collect()
    };
    match sfx {
        Sfx::Beep => gen(0.12, &mut |t| {
            ((TAU * 1800.0 * t).sin().signum() * 0.25) * (1.0 - t / 0.12)
        }),
        Sfx::Click => gen(0.03, &mut |t| {
            (TAU * 1200.0 * t).sin() * 0.25 * (1.0 - t / 0.03)
        }),
        Sfx::Blip => gen(0.04, &mut |t| (TAU * 700.0 * t).sin() * 0.15),
        Sfx::Pickup => gen(0.15, &mut |t| {
            (TAU * (500.0 + 900.0 * t) * t).sin() * 0.25 * (1.0 - t / 0.15)
        }),
        Sfx::Error => gen(0.35, &mut |t| {
            ((TAU * 140.0 * t).sin().signum() * 0.2) * (1.0 - t / 0.35)
        }),
        Sfx::Squeak => {
            // quietschendes Einkaufswagen-Rad: wackelnde hohe Frequenz
            let mut phase = 0.0f32;
            gen(0.32, &mut |t| {
                let f = 2100.0 + 500.0 * (t * 38.0).sin() + 250.0 * (t * 7.0).sin();
                phase += TAU * f / RATE as f32;
                let env = (t * 40.0).min(1.0) * (1.0 - t / 0.32);
                (phase.sin() + 0.3 * (phase * 2.0).sin()) * 0.18 * env
            })
        }
        Sfx::Scare => {
            // schiefer Saegezahn-Cluster (Halbtonreibung) plus Rauschstoss, hart einsetzend
            let freqs = [233.0f32, 247.0, 349.0, 370.0, 523.0, 554.0];
            let mut ph = [0.0f32; 6];
            gen(1.2, &mut |t| {
                let mut s = 0.0;
                for (k, f) in freqs.iter().enumerate() {
                    let bend = 1.0 - 0.18 * (t / 1.2);
                    ph[k] = (ph[k] + f * bend / RATE as f32).fract();
                    s += ph[k] * 2.0 - 1.0;
                }
                let noise = rng.next() * (1.0 - t / 0.25).max(0.0);
                let env = (t * 200.0).min(1.0) * (1.0 - t / 1.2).max(0.0).powf(1.6);
                ((s / 6.0) * 0.8 + noise * 0.6) * 0.75 * env
            })
        }
        Sfx::Shriek => {
            // Schrei: Saegezahn mit Vibrato, der hochschnellt, plus Obertoene, hart verzerrt;
            // dazu ein kreischender Metallton, ein Basseinschlag und ein Rauschstoss
            const DUR: f32 = 2.6;
            let (mut p1, mut p2, mut p3, mut pb) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            gen(DUR, &mut |t| {
                let glide = 520.0 + 680.0 * (t / 0.12).min(1.0);
                let vib = 1.0 + 0.07 * (TAU * 9.5 * t).sin() + 0.03 * rng.next();
                let f = glide * vib * (1.0 - 0.25 * (t / DUR));
                p1 = (p1 + f / RATE as f32).fract();
                p2 = (p2 + f * 1.51 / RATE as f32).fract();
                p3 = (p3 + (2900.0 + 400.0 * (TAU * 23.0 * t).sin()) / RATE as f32).fract();
                let scream = (p1 * 2.0 - 1.0) + 0.6 * (p2 * 2.0 - 1.0);
                let metal = if p3 < 0.5 { 0.35 } else { -0.35 };
                let fb = 55.0 * (1.0 - t / 0.8).max(0.4);
                pb = (pb + fb / RATE as f32).fract();
                let boom = (TAU * pb).sin() * (1.0 - t / 0.9).max(0.0).powi(2) * 1.4;
                let noise = rng.next() * (0.25 + 0.6 * (1.0 - t / 0.3).max(0.0));
                let env = (t * 400.0).min(1.0) * (1.0 - ((t - 1.9) / 0.7).max(0.0)).max(0.0);
                let mix = (scream * 0.9 + metal + noise) * env + boom;
                (mix * 2.2).tanh() * 0.95
            })
        }
        Sfx::Screech => {
            // Bogen auf Metall: hoher Saegezahn mit nervoesem Vibrato, verzerrt, kratzend
            let mut ph = 0.0f32;
            let mut ph2 = 0.0f32;
            gen(1.1, &mut |t| {
                let f = 1650.0 + 260.0 * (TAU * 13.0 * t).sin() + 140.0 * rng.next();
                ph = (ph + f / RATE as f32).fract();
                ph2 = (ph2 + f * 1.493 / RATE as f32).fract();
                let x = (ph * 2.0 - 1.0) + 0.5 * (ph2 * 2.0 - 1.0) + rng.next() * 0.35;
                let env = (t * 60.0).min(1.0) * (1.0 - t / 1.1).max(0.0).powf(1.3);
                (x * 1.8).tanh() * 0.55 * env
            })
        }
        Sfx::Thud => {
            // dumpfer Einschlag: tiefer Sinus, der absackt, plus kurzer Klick
            let mut ph = 0.0f32;
            gen(0.9, &mut |t| {
                let f = 70.0 * (1.0 - t / 0.9).max(0.35);
                ph = (ph + f / RATE as f32).fract();
                let body = (TAU * ph).sin() * (-t * 5.0).exp();
                let click = rng.next() * (-t * 120.0).exp();
                ((body * 1.4 + click * 0.5) * 1.5).tanh() * 0.9
            })
        }
        Sfx::Static => {
            // Fernsehrauschen: grob abgetastetes Rauschen in Schueben
            let mut hold = 0.0f32;
            let mut n = 0usize;
            gen(0.7, &mut |t| {
                if n % 6 == 0 {
                    hold = rng.next();
                }
                n += 1;
                let gate = if ((t * 23.0) as i32) % 3 == 0 {
                    0.4
                } else {
                    1.0
                };
                hold * 0.5 * gate * (1.0 - t / 0.7).max(0.0)
            })
        }
        Sfx::Rumble => {
            // tiefes Grollen schwillt an und bricht abrupt ab
            let mut lp = 0.0f32;
            let mut ph = 0.0f32;
            gen(1.4, &mut |t| {
                lp += (rng.next() - lp) * 0.02;
                ph = (ph + 42.0 / RATE as f32).fract();
                let swell = (t / 1.3).min(1.0).powi(2);
                let cut = if t > 1.3 {
                    (1.0 - (t - 1.3) / 0.1).max(0.0)
                } else {
                    1.0
                };
                ((lp * 3.0 + (TAU * ph).sin() * 0.6) * swell * cut * 0.8).clamp(-1.0, 1.0)
            })
        }
        Sfx::Shuffle => {
            // Schleifen ueber den Boden: gefiltertes Rauschen, das an- und abschwillt
            let mut lp = 0.0;
            gen(0.5, &mut |t| {
                let n = rng.next();
                lp += (n - lp) * 0.08;
                let env = (t / 0.08).min(1.0) * (1.0 - t / 0.5).max(0.0).powi(2);
                lp * 0.9 * env
            })
        }
        Sfx::Keys => {
            // Schluessel: kurze, hohe, unharmonische Klingeltoene nacheinander
            gen(0.45, &mut |t| {
                let mut s = 0.0;
                for (k, f) in [3150.0f32, 4210.0, 5370.0, 3720.0].iter().enumerate() {
                    let t0 = k as f32 * 0.08;
                    if t > t0 {
                        let u = t - t0;
                        s += (TAU * f * u).sin() * (-u * 28.0).exp();
                    }
                }
                s * 0.18
            })
        }
        Sfx::Spark => {
            // Knistern: dichte, kurze Knacker mit hellem Rauschen, schnell abklingend
            gen(0.35, &mut |t| {
                // Zufall liegt schon zwischen -1 und 1
                let crack = (rng.next() > 0.64) as i32 as f32;
                let hiss = rng.next() * 0.5;
                (crack * rng.next() * 0.7 + hiss * 0.25) * 0.3 * (1.0 - t / 0.35).powi(2)
            })
        }
        Sfx::Whisper => {
            // Fluestern: gefiltertes Rauschen mit Silbenrhythmus und "s"-Lauten
            let mut lp = 0.0;
            let mut hp_prev = 0.0;
            gen(1.1, &mut |t| {
                let n = rng.next() * 0.5;
                lp += (n - lp) * 0.35;
                let hiss = n - hp_prev;
                hp_prev = n;
                let syll = ((t * 7.5).sin() * 0.5 + 0.5).powi(2) * ((t * 2.3).sin() * 0.4 + 0.6);
                let s_sound = if (t * 3.1).fract() < 0.12 {
                    hiss * 0.6
                } else {
                    0.0
                };
                let env = (t * 6.0).min(1.0) * (1.0 - t / 1.1).max(0.0);
                (lp * syll * 0.9 + s_sound) * 0.22 * env
            })
        }
        Sfx::SqueakSoft => {
            // leises, tieferes Rad-Quietschen mit langsamem Eiern
            let mut phase = 0.0f32;
            gen(0.22, &mut |t| {
                let f = 1250.0 + 180.0 * (t * 17.0).sin() + 90.0 * (t * 5.0).sin();
                phase += TAU * f / RATE as f32;
                let env = (t * 30.0).min(1.0) * (1.0 - t / 0.22).powf(1.5);
                (phase.sin() + 0.2 * (phase * 1.5).sin()) * 0.1 * env
            })
        }
        Sfx::Rattle => {
            // klappernde Lenkrollen: kurze Klicks in unregelmaessigem Abstand
            let mut lp = 0.0;
            gen(0.3, &mut |t| {
                let click = ((t * 46.0).fract() < 0.18) as i32 as f32
                    * (0.6 + 0.4 * (t * 13.0).sin().abs());
                lp += (rng.next() - lp) * 0.5;
                lp * click * 0.35 * (1.0 - t / 0.3)
            })
        }
        Sfx::Step => {
            let mut lp = 0.0;
            gen(0.08, &mut |t| {
                lp += (rng.next() - lp) * 0.15;
                lp * 0.8 * (1.0 - t / 0.08)
            })
        }
        Sfx::Chime => {
            let mut buf = vec![0.0; secs(2.0)];
            for (start, freq) in [(0.0, 659.25), (0.55, 523.25)] {
                let s0 = secs(start);
                for i in 0..secs(1.4) {
                    let Some(o) = buf.get_mut(s0 + i) else { break };
                    let t = i as f32 / RATE as f32;
                    let env = (-t * 2.5).exp();
                    *o +=
                        ((TAU * freq * t).sin() + 0.4 * (TAU * freq * 2.76 * t).sin()) * env * 0.25;
                }
            }
            buf
        }
        Sfx::Alarm => {
            // zweitoenige Sirene mit kratzigem Lautsprecher
            let mut phase = 0.0f32;
            gen(1.3, &mut |t| {
                let f = if (t * 2.0).fract() < 0.5 {
                    880.0
                } else {
                    660.0
                };
                phase += TAU * f / RATE as f32;
                let env = (t * 30.0).min(1.0) * (1.0 - (t - 1.2).max(0.0) * 10.0);
                ((phase.sin() + 0.5 * (phase * 3.0).sin()) * 1.6).tanh() * 0.22 * env
            })
        }
        Sfx::Lighter => {
            // ratschendes Reibrad, dann leises Fauchen der Flamme
            let mut lp = 0.0;
            gen(0.7, &mut |t| {
                let n = rng.next();
                lp += (n - lp) * 0.2;
                let strike = if t < 0.08 {
                    (((t * 180.0).fract() < 0.3) as i32 as f32) * n * 0.6
                } else {
                    0.0
                };
                let flame = if t > 0.1 {
                    lp * 0.35 * (1.0 - (t - 0.1) / 0.6)
                } else {
                    0.0
                };
                (strike + flame) * 0.5
            })
        }
        Sfx::Puff => {
            // Glut knistert beim Ziehen
            gen(0.9, &mut |t| {
                let crack = (rng.next() > 0.93) as i32 as f32 * rng.next();
                let hiss = rng.next() * 0.05;
                (crack * 0.35 + hiss) * (t * 4.0).min(1.0) * (1.0 - t / 0.9)
            })
        }
        Sfx::Stomp => {
            // dumpfer Aufprall der Ferse plus kurzes Schlurfen der Sohle
            let mut lp = 0.0;
            gen(0.2, &mut |t| {
                lp += (rng.next() - lp) * 0.25;
                let f = 85.0 * (-t * 9.0).exp() + 45.0;
                let thud = (TAU * f * t).sin() * (-t * 28.0).exp() * 0.9;
                let scuff = lp * (-t * 40.0).exp() * 0.6;
                ((thud + scuff) * 1.4).tanh() * 0.55
            })
        }
        Sfx::Creak => {
            // langsames, holpriges Knarzen: tiefe Frequenz mit unregelmaessigen Rucken
            let mut phase = 0.0f32;
            gen(0.55, &mut |t| {
                let f = 180.0 + 90.0 * (t * 9.0).sin() + 60.0 * (t * 23.0).sin();
                phase += TAU * f / RATE as f32;
                let rucks = (((t * 31.0).fract() < 0.35) as i32 as f32) * 0.7 + 0.3;
                let env = (t * 20.0).min(1.0) * (1.0 - t / 0.55);
                ((phase.sin() * 0.6 + (phase * 2.7).sin() * 0.4) * rucks * env * 1.5).tanh() * 0.35
            })
        }
        Sfx::Knock => {
            // drei dumpfe Schlaege mit Knoecheln auf Plastik/Metall
            gen(0.6, &mut |t| {
                let hit = |t0: f32| {
                    let x = t - t0;
                    if x < 0.0 {
                        0.0
                    } else {
                        ((TAU * 140.0 * x).sin() + 0.5 * (TAU * 410.0 * x).sin())
                            * (-x * 35.0).exp()
                    }
                };
                ((hit(0.0) + hit(0.18) + hit(0.36)) * 1.3).tanh() * 0.5
            })
        }
        Sfx::Heartbeat => {
            // "bumm-bumm": zwei tiefe Schlaege, der zweite leiser
            gen(0.5, &mut |t| {
                let beat = |t0: f32, a: f32| {
                    let x = t - t0;
                    if x < 0.0 {
                        0.0
                    } else {
                        (TAU * 52.0 * x).sin() * (-x * 22.0).exp() * a
                    }
                };
                ((beat(0.0, 1.0) + beat(0.17, 0.7)) * 1.6).tanh() * 0.6
            })
        }
        Sfx::Exhale => {
            // langes, weiches Ausatmen: tief gefiltertes Rauschen, an- und abschwellend
            let mut lp = 0.0;
            gen(1.4, &mut |t| {
                lp += (rng.next() - lp) * 0.08;
                let env = (t * 5.0).min(1.0) * (1.0 - t / 1.4).powf(1.5);
                lp * 0.9 * env
            })
        }
        Sfx::Coin => gen(0.18, &mut |t| {
            let ring = (TAU * 2600.0 * t).sin() + 0.6 * (TAU * 3900.0 * t).sin();
            ring * 0.12 * (-t * 22.0).exp() + rng.next() * 0.05 * (-t * 90.0).exp()
        }),
        Sfx::Sting => {
            let mut lp = 0.0;
            gen(1.6, &mut |t| {
                lp += (rng.next() - lp) * 0.3;
                let f = 300.0 * (-t * 1.5).exp() + 50.0;
                let tone = (TAU * f * t).sin();
                ((tone * 0.5 + lp * 0.8) * 1.8).tanh() * 0.45 * (1.0 - t / 1.6)
            })
        }
    }
}

impl Audio {
    pub fn new(volume: f32) -> Option<Self> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| crate::crash::log(&format!("Kein Audio: {e}")))
            .ok()?;
        let music = Sink::try_new(&handle).ok()?;
        music.append(SamplesBuffer::new(1, RATE, muzak()).repeat_infinite());
        let hum_sink = Sink::try_new(&handle).ok()?;
        hum_sink.append(SamplesBuffer::new(1, RATE, hum()).repeat_infinite());
        let sounds = [
            Sfx::Beep,
            Sfx::Chime,
            Sfx::Step,
            Sfx::Sting,
            Sfx::Blip,
            Sfx::Click,
            Sfx::Error,
            Sfx::Pickup,
            Sfx::Squeak,
            Sfx::SqueakSoft,
            Sfx::Rattle,
            Sfx::Spark,
            Sfx::Whisper,
            Sfx::Alarm,
            Sfx::Coin,
            Sfx::Lighter,
            Sfx::Puff,
            Sfx::Exhale,
            Sfx::Stomp,
            Sfx::Heartbeat,
            Sfx::Creak,
            Sfx::Knock,
            Sfx::Scare,
            Sfx::Shriek,
            Sfx::Shuffle,
            Sfx::Keys,
            Sfx::Screech,
            Sfx::Thud,
            Sfx::Static,
            Sfx::Rumble,
        ]
        .into_iter()
        .map(|s| (s, synth(s)))
        .collect();
        let mut a = Self {
            _stream: stream,
            handle,
            music,
            hum: hum_sink,
            sounds,
            master: volume,
        };
        a.set_master(volume);
        Some(a)
    }

    pub fn set_master(&mut self, v: f32) {
        self.master = v;
    }

    pub fn play(&self, sfx: Sfx) {
        self.play_vol(sfx, 1.0);
    }

    /// Wie `play_vol`, aber im Stereobild verteilt: `pan` -1 = links, 1 = rechts.
    pub fn play_at(&self, sfx: Sfx, vol: f32, pan: f32) {
        let Some((_, data)) = self
            .sounds
            .iter()
            .find(|(s, _)| std::mem::discriminant(s) == std::mem::discriminant(&sfx))
        else {
            return;
        };
        // gleiche Lautheit in der Mitte (Sinus/Kosinus-Verteilung)
        let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
        let (l, r) = (a.cos(), a.sin());
        let stereo: Vec<f32> = data.iter().flat_map(|&x| [x * l, x * r]).collect();
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(self.master * vol);
            sink.append(SamplesBuffer::new(2, RATE, stereo));
            sink.detach();
        }
    }

    pub fn play_vol(&self, sfx: Sfx, vol: f32) {
        let Some((_, data)) = self
            .sounds
            .iter()
            .find(|(s, _)| std::mem::discriminant(s) == std::mem::discriminant(&sfx))
        else {
            return;
        };
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.set_volume(self.master * vol);
            sink.append(SamplesBuffer::new(1, RATE, data.clone()));
            sink.detach();
        }
    }

    /// Jedes Frame: Musik leiern lassen, Brummen regeln.
    pub fn update(&self, m: &Mood) {
        let tier = m.tier as f32;
        let (speed, music_vol, hum_vol) = if !m.playing {
            (1.0, 0.55, 0.0)
        } else if m.tipped {
            let wobble = (m.time * (0.35 + tier * 0.15)).sin() * (0.015 + tier * 0.012);
            (
                0.93 - tier * 0.035 + wobble,
                0.45 - tier * 0.08,
                0.35 + tier * 0.1,
            )
        } else {
            (1.0, 0.5, 0.06)
        };
        let pause = if m.paused { 0.35 } else { 1.0 };
        self.music.set_speed(speed.max(0.5));
        let quiet = 1.0 - m.hush.clamp(0.0, 1.0);
        self.music
            .set_volume(self.master * music_vol.max(0.1) * pause * quiet);
        self.hum.set_volume(self.master * hum_vol * pause * quiet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesized_sounds_are_sane() {
        for s in [
            Sfx::Beep,
            Sfx::Chime,
            Sfx::Step,
            Sfx::Sting,
            Sfx::Blip,
            Sfx::Click,
            Sfx::Error,
            Sfx::Pickup,
            Sfx::Squeak,
            Sfx::SqueakSoft,
            Sfx::Rattle,
            Sfx::Spark,
            Sfx::Whisper,
            Sfx::Alarm,
            Sfx::Coin,
            Sfx::Lighter,
            Sfx::Puff,
            Sfx::Exhale,
            Sfx::Stomp,
            Sfx::Heartbeat,
            Sfx::Creak,
            Sfx::Knock,
            Sfx::Scare,
            Sfx::Shriek,
            Sfx::Shuffle,
            Sfx::Keys,
            Sfx::Screech,
            Sfx::Thud,
            Sfx::Static,
            Sfx::Rumble,
        ] {
            let d = synth(s);
            assert!(!d.is_empty());
            assert!(
                d.iter().all(|x| x.is_finite() && x.abs() <= 1.5),
                "{s:?} uebersteuert"
            );
        }
        let m = muzak();
        assert_eq!(m.len(), secs(16.0));
        assert!(m.iter().all(|x| x.abs() < 1.0));
    }
}
