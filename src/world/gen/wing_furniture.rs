//! Einrichtung der Pfandraeume (zu wing_look.rs): Kistenregal an der Nordwand, drei
//! Sammeltonnen (Glas, Plastik, Dosen), Sitzbank und Anleitungstafel an der Suedwand,
//! Wanduhr, Muelleimer neben dem Automaten, Fussmatte an der Tuer zum Markt, Lueftungsgitter.
//! Mit dem Verfall fehlen Kisten im Regal, eine Tonne liegt um, die Uhr steht.
//!
//! Alles aus Boxen (Einrichtung). Was im Weg steht, bekommt einen festen Koerper; Durchgaenge,
//! Tueren und der Platz vor dem Automaten bleiben frei.

use super::wing_look::RoomShape;
use super::*;

/// Schriften der Einrichtung (stehen im Schild-Atlas, signs.rs)
pub const FURNITURE_TEXTS: [&str; 7] = [
    "So geht's",
    "1 Flasche drehen",
    "2 Bei Grün einlegen",
    "3 Bon drucken",
    "Glas",
    "Plastik",
    "Dosen",
];

/// Fester Zufall je Raum und Stelle (0..1).
fn hash(seed: u64, k: usize, i: usize) -> f32 {
    let x = (seed as f32 * 0.021 + k as f32 * 7.13 + i as f32 * 3.917).sin() * 24_634.634;
    x.fract().abs()
}

impl Gen<'_> {
    /// Einrichtung eines Pfandraums. `d`: Verfall 0..1, `has_door`: Tuer zum Markt (Ostwand).
    pub(super) fn furnish_wing_room(
        &mut self,
        k: usize,
        r: &RoomShape,
        machine_z: f32,
        d: f32,
        has_door: bool,
    ) {
        let seed = self.params.seed;
        let (xc, zc) = ((r.x_w + r.x_e) * 0.5, (r.z0 + r.z1) * 0.5);
        let metal = vec3(0.45, 0.47, 0.5);
        let dark = vec3(0.12, 0.12, 0.13);

        // --- Kistenregal an der Nordwand (westlich vom Durchgang) ---
        let (rx0, rx1) = (r.x_w + 0.65, xc - 0.8);
        let depth = 0.22;
        let rz = r.z1 - depth - 0.02;
        let crate_h = vec3(0.2, 0.13, 0.17);
        for x in [rx0, rx1] {
            for dz in [-depth + 0.02, depth - 0.02] {
                self.b
                    .add(vec3(x, 0.8, rz + dz), vec3(0.02, 0.8, 0.02), metal, PLAIN);
            }
        }
        let crate_cols = [
            vec3(0.15, 0.45, 0.25),
            vec3(0.18, 0.3, 0.6),
            vec3(0.62, 0.2, 0.15),
            vec3(0.75, 0.6, 0.15),
        ];
        let mut t = 0;
        for (level, y) in [0.06f32, 0.62, 1.18].into_iter().enumerate() {
            self.b.add(
                vec3((rx0 + rx1) * 0.5, y, rz),
                vec3((rx1 - rx0) * 0.5 + 0.03, 0.02, depth),
                metal,
                PLAIN,
            );
            let n = ((rx1 - rx0) / (crate_h.x * 2.0 + 0.05)).floor() as usize;
            for c in 0..n {
                t += 1;
                // Verfall: Luecken im Regal
                if hash(seed, k, t) < d * 0.6 {
                    continue;
                }
                let x = rx0 + 0.05 + crate_h.x + c as f32 * (crate_h.x * 2.0 + 0.05);
                let col = crate_cols[(c + level + k) % crate_cols.len()];
                let p = vec3(x, y + 0.02 + crate_h.y, rz);
                self.b.add(p, crate_h, col, PLAIN);
                self.b.add(
                    p + vec3(0.0, crate_h.y, 0.0),
                    vec3(crate_h.x - 0.03, 0.004, crate_h.z - 0.03),
                    col * 0.4,
                    PLAIN,
                );
            }
        }
        self.b.solid(
            vec3((rx0 + rx1) * 0.5, 0.8, rz),
            vec3((rx1 - rx0) * 0.5 + 0.03, 0.8, depth),
        );

        // --- drei Sammeltonnen an der Nordwand (oestlich vom Durchgang) ---
        let bins = [
            (vec3(0.2, 0.5, 0.25), FURNITURE_TEXTS[4]),
            (vec3(0.85, 0.72, 0.15), FURNITURE_TEXTS[5]),
            (vec3(0.45, 0.5, 0.58), FURNITURE_TEXTS[6]),
        ];
        let bin_h = vec3(0.26, 0.45, 0.26);
        for (i, (col, label)) in bins.iter().enumerate() {
            let x = xc + 1.1 + i as f32 * 0.65;
            let z = r.z1 - bin_h.z - 0.06;
            // ab "falsch" liegt die mittlere Tonne um
            if d >= 0.66 && i == 1 {
                let p = vec3(x, bin_h.z, z - 0.35);
                self.b
                    .add_rot(p, vec3(bin_h.y, bin_h.z, bin_h.x), 0.3, *col * 0.8, PLAIN);
                self.b.solid_rot(p, vec3(bin_h.y, bin_h.z, bin_h.x), 0.3);
                continue;
            }
            self.b.add(vec3(x, bin_h.y, z), bin_h, *col, PLAIN);
            // Deckel mit Einwurf
            self.b.add(
                vec3(x, bin_h.y * 2.0 + 0.02, z),
                vec3(bin_h.x + 0.02, 0.02, bin_h.z + 0.02),
                *col * 0.7,
                PLAIN,
            );
            self.b.add(
                vec3(x, bin_h.y * 2.0 + 0.042, z),
                vec3(0.1, 0.003, 0.05),
                dark,
                PLAIN,
            );
            self.b.label(
                vec3(x, 0.62, z - bin_h.z - 0.01),
                PI,
                0.2,
                0.07,
                label,
                *col,
                0.0,
                0.0,
                0.0,
            );
            self.b.solid(vec3(x, bin_h.y, z), bin_h);
        }

        // --- Sitzbank an der Suedwand (unter "Danke fuers Zurueckbringen") ---
        let bx = xc + 1.75;
        let bz = r.z0 + 0.24;
        let wood = vec3(0.55, 0.38, 0.22) * (1.0 - 0.3 * d);
        self.b
            .add(vec3(bx, 0.45, bz), vec3(0.7, 0.03, 0.2), wood, PLAIN);
        self.b.add(
            vec3(bx, 0.75, r.z0 + 0.05),
            vec3(0.7, 0.12, 0.02),
            wood,
            PLAIN,
        );
        for sx in [-0.62f32, 0.62] {
            for sz in [-0.15f32, 0.15] {
                self.b.add(
                    vec3(bx + sx, 0.21, bz + sz),
                    vec3(0.025, 0.21, 0.025),
                    metal,
                    PLAIN,
                );
            }
        }
        self.b.solid(vec3(bx, 0.24, bz), vec3(0.7, 0.24, 0.2));

        // --- Anleitungstafel an der Suedwand (westlich vom Durchgang) ---
        let tx = xc - 1.8;
        let tz = r.z0 + 0.014;
        let board = vec3(0.94, 0.94, 0.9);
        self.b
            .add(vec3(tx, 1.45, tz), vec3(0.62, 0.42, 0.012), board, PLAIN);
        self.b.add(
            vec3(tx, 1.82, tz + 0.004),
            vec3(0.62, 0.06, 0.012),
            vec3(0.16, 0.34, 0.6),
            PLAIN,
        );
        self.b.label(
            vec3(tx, 1.82, tz + 0.02),
            0.0,
            0.5,
            0.05,
            FURNITURE_TEXTS[0],
            vec3(0.16, 0.34, 0.6),
            0.0,
            0.0,
            0.0,
        );
        for (i, line) in FURNITURE_TEXTS[1..4].iter().enumerate() {
            self.b.label(
                vec3(tx - 0.02, 1.6 - i as f32 * 0.2, tz + 0.02),
                0.0,
                0.5,
                0.055,
                line,
                board,
                2.0,
                0.0,
                0.0,
            );
        }

        // --- Wanduhr an der Westwand zwischen Automat und aufgemalter Tuer ---
        let cz = zc - 0.5;
        let cx = r.x_w + 0.02;
        self.b
            .add(vec3(cx, 2.05, cz), vec3(0.015, 0.17, 0.17), dark, PLAIN);
        self.b.add(
            vec3(cx + 0.01, 2.05, cz),
            vec3(0.012, 0.15, 0.15),
            vec3(0.95, 0.95, 0.92),
            PLAIN,
        );
        // Zeiger: gepflegt 10 nach 10, im Verfall steht sie auf kurz vor zwoelf
        let (a_h, a_m) = if d >= 0.66 {
            (0.0f32, -0.3f32)
        } else {
            (-1.0, 1.0)
        };
        for (ang, len) in [(a_h, 0.07f32), (a_m, 0.11)] {
            let (s, c) = ang.sin_cos();
            self.b.add(
                vec3(cx + 0.025, 2.05 + c * len * 0.5, cz - s * len * 0.5),
                vec3(0.004, len * 0.5, 0.006),
                dark,
                PLAIN,
            );
        }

        // --- Muelleimer neben dem Automaten ---
        let tb = vec3(r.x_w + 0.24, 0.0, machine_z + 0.85);
        self.b.add(
            tb + vec3(0.0, 0.3, 0.0),
            vec3(0.17, 0.3, 0.17),
            metal * 0.8,
            PLAIN,
        );
        self.b.add(
            tb + vec3(0.0, 0.61, 0.0),
            vec3(0.18, 0.015, 0.18),
            dark,
            PLAIN,
        );
        self.b
            .solid(tb + vec3(0.0, 0.3, 0.0), vec3(0.17, 0.3, 0.17));

        // --- Fussmatte an der Tuer zum Markt ---
        if has_door {
            let mat = vec3(0.14, 0.2, 0.32) * (1.0 - 0.4 * d);
            self.b.add_floor(
                vec3(r.x_e - 0.65, 0.006, zc),
                vec3(0.55, 0.004, 0.85),
                0.0,
                mat,
                FLOOR,
            );
        }

        // --- Lueftungsgitter an der Decke ---
        let vent = vec3(xc - 1.4, WING_H_LOOK - 0.012, zc);
        self.b.add(vent, vec3(0.3, 0.012, 0.3), metal * 1.1, PLAIN);
        for i in 0..5 {
            self.b.add(
                vent - vec3(0.0, 0.014, -0.24 + i as f32 * 0.12),
                vec3(0.27, 0.004, 0.015),
                dark,
                PLAIN,
            );
        }
    }
}

/// Deckenhoehe des Trakts (wie wing.rs `WING_H`)
const WING_H_LOOK: f32 = super::wing::WING_H;
