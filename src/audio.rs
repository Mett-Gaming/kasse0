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
    /// Jumpscare: greller, schiefer Akkord mit Rauschstoss
    Scare,
    /// Troll-Jumpscare im Menue: verzerrter Schrei, Metallkreischen, Basseinschlag
    Shriek,
}

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
            .map_err(|e| eprintln!("Kein Audio: {e}"))
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
            Sfx::Scare,
            Sfx::Shriek,
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
            Sfx::Scare,
            Sfx::Shriek,
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
