//! Weg des Korridors wuerfeln: gerade Stuecke "vorwaerts" (+z) und "seitlich" (+x oder -x,
//! zufaellig) im Wechsel, verbunden durch 90-Grad-Ecken. Er laeuft nie zurueck (-z), bleibt also
//! hinter dem Markt und kreuzt sich nie. Das letzte Stueck laeuft vorwaerts (Spiegel nach -z).

use super::*;

/// Kuerzeste und laengste Ganglaenge (Meter)
pub const LEN_MIN: f32 = 40.0;
pub const LEN_MAX: f32 = 140.0;
/// Bis zu dieser Run-Zeit (Sekunden) bleibt es bei LEN_MIN, danach je Minute LEN_PER_MIN mehr
const LEN_FROM: f32 = 300.0;
const LEN_PER_MIN: f32 = 8.0;
/// Laenge der Stuecke: erstes, vorwaerts, seitlich, letztes
const FIRST: (f32, f32) = (8.0, 14.0);
const FORWARD: (f32, f32) = (7.0, 14.0);
const SIDEWAYS: (f32, f32) = (7.0, 12.0);
pub const LAST: f32 = 12.0;

/// Ganglaenge aus der Zeit im Run (beim Bezahlen): je laenger der Run, desto laenger der Gang.
pub fn corridor_len_for(run_time: f32) -> f32 {
    let extra = ((run_time - LEN_FROM) / 60.0).max(0.0) * LEN_PER_MIN;
    (LEN_MIN + extra).clamp(LEN_MIN, LEN_MAX)
}

/// Eckpunkte der Mittellinie ab `start` (Ladentuer), Gesamtlaenge etwa `len`.
pub(super) fn path(rng: &mut Pcg64, len: f32, start: Vec3) -> Vec<Vec3> {
    let len = len.clamp(LEN_MIN, LEN_MAX);
    let mut pts = vec![start];
    let mut p = start;
    let first = rng.gen_range(FIRST.0..FIRST.1);
    p.z += first;
    pts.push(p);
    // was zwischen erstem und letztem Stueck uebrig ist: abwechselnd seitlich und vorwaerts,
    // vor dem letzten Stueck immer seitlich (sonst waere es ein langes gerades Ende)
    let mut rem = len - first - LAST;
    loop {
        let side = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        // reicht es noch fuer seitlich + vorwaerts + seitlich?
        if rem >= SIDEWAYS.0 + FORWARD.0 + SIDEWAYS.0 {
            let s = rng
                .gen_range(SIDEWAYS.0..SIDEWAYS.1)
                .min(rem - FORWARD.0 - SIDEWAYS.0);
            p.x += side * s;
            pts.push(p);
            let f = rng
                .gen_range(FORWARD.0..FORWARD.1)
                .min(rem - s - SIDEWAYS.0);
            p.z += f;
            pts.push(p);
            rem -= s + f;
        } else {
            // letztes Seitenstueck nimmt den Rest (mindestens SIDEWAYS.0)
            p.x += side * rem.max(SIDEWAYS.0);
            pts.push(p);
            break;
        }
    }
    p.z += LAST;
    pts.push(p);
    pts
}
