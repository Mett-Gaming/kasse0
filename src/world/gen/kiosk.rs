//! Kiosk in der Suedwestecke vor den Kassen: eine rostige Maschine, die gegen Pfandbons
//! Gegenstaende erzeugt (siehe game/pfand/kiosk.rs). Steht in jedem Markt an derselben Stelle.

use super::*;

/// Wo der Kiosk steht: eine rostige Maschine an der Westwand (Modell `MeshId::Machine`),
/// im gekippten Markt liegt daneben ein Skelett (`MeshId::Skeleton`). Gezeichnet werden beide
/// vom Spiel (game/pfand/kiosk.rs); hier stehen nur Lage und feste Koerper.
#[derive(Clone, Copy, Debug)]
pub struct Kiosk {
    /// Mitte der Maschine am Boden
    pub counter: Vec3,
    /// Mitte des liegenden Skeletts am Boden und seine Drehung (nur gekippt sichtbar)
    pub skeleton: Vec3,
    pub skeleton_yaw: f32,
}

/// Halbe Groesse der Maschine (gemessen am Modell: 0,74 x 1,45 x 2,08 m, lang in z)
pub const MACHINE_HALF: Vec3 = Vec3::new(0.37, 0.725, 1.04);
/// Kopf der Maschine (aus Boxen) ueber dem Kompressor: Unterkante und halbe Groesse (m)
pub const HEAD_Y: f32 = 1.5;
pub const HEAD_HALF: Vec3 = Vec3::new(0.3, 0.36, 0.5);
/// Ausgabeschacht vorn: Mitte der Hoehe und halbe Groesse (m)
pub const CHUTE_Y: f32 = 0.62;
pub const CHUTE_HALF: Vec3 = Vec3::new(0.12, 0.24, 0.22);
/// Anzeigefenster: links, rechts (normal bzw. im gekippten Markt)
pub const DISPLAY_NORMAL: [&str; 2] = ["0000", "0000"];
pub const DISPLAY_TIPPED: [&str; 2] = ["TAG", "5032"];
/// Halbe Groesse des liegenden Skeletts (Modell 1,75 x 0,42 x 0,42 m, etwas Luft fuer die Drehung)
const SKELETON_HALF: Vec3 = Vec3::new(0.88, 0.21, 0.3);
/// Wegweiser-Tafel: halbe Breite (m)
const ARROW_HALF: f32 = 0.5;

impl Kiosk {
    /// Die Maschine, zum Anschauen
    pub fn body(&self) -> Aabb {
        Aabb {
            min: self.counter - vec3(MACHINE_HALF.x, 0.0, MACHINE_HALF.z),
            max: self.counter
                + vec3(
                    MACHINE_HALF.x + 2.0 * CHUTE_HALF.x,
                    HEAD_Y + 2.0 * HEAD_HALF.y,
                    MACHINE_HALF.z,
                ),
        }
    }
    /// Wo man vor der Maschine steht
    pub fn front(&self) -> Vec3 {
        self.counter + vec3(MACHINE_HALF.x + 2.0 * CHUTE_HALF.x + 0.5, 0.0, 0.0)
    }
}

impl Gen<'_> {
    /// `signs_x`: x-Stellen der Wegweiser (z = vor den Regalenden).
    pub(super) fn build_kiosk(&mut self, z: f32, signs_x: &[f32]) -> Kiosk {
        let tipped = self.params.tipped;
        // Maschine mit dem Ruecken zur Westwand, Skelett liegt am suedlichen Ende davor
        let k = Kiosk {
            counter: vec3(0.2 + MACHINE_HALF.x, 0.0, z),
            // Kopf an der Wand, Fuesse in den Raum (Modell 1,75 m lang, Kopf nach -x)
            skeleton: vec3(0.25 + SKELETON_HALF.x, 0.0, z + MACHINE_HALF.z + 0.4),
            skeleton_yaw: 0.12,
        };
        self.b
            .solid(k.counter + vec3(0.0, MACHINE_HALF.y, 0.0), MACHINE_HALF);
        if tipped {
            // liegt quer am Boden: niedriger Koerper, ueber den man nicht hinweglaeuft
            self.b
                .solid(k.skeleton + vec3(0.0, SKELETON_HALF.y, 0.0), SKELETON_HALF);
        }
        self.machine_parts(&k);
        self.kiosk_signs(z, signs_x);
        k
    }

    /// Aufbau der Maschine aus Boxen (aehnlich einer alten Zapfsaeule, ohne Marke): Kopf mit
    /// Anzeigefenstern auf zwei Stuetzen ueber dem Kompressor, Schlaeuche mit Duesen an den
    /// Seiten, vorn der Ausgabeschacht mit Ablage, aus dem die Gegenstaende kommen.
    fn machine_parts(&mut self, k: &Kiosk) {
        let tipped = self.params.tipped;
        let dim = if tipped { 0.6 } else { 1.0 };
        let c = k.counter;
        let front = c.x + MACHINE_HALF.x;
        let cream = vec3(0.74, 0.7, 0.6) * dim;
        let rust = vec3(0.45, 0.24, 0.11) * dim;
        let green = vec3(0.45, 0.62, 0.38) * dim;
        let iron = vec3(0.22, 0.2, 0.18) * dim;
        let hose = vec3(0.06, 0.06, 0.06);
        // Stuetzen und Kopf
        for s in [-1.0f32, 1.0] {
            self.b.add(
                vec3(c.x - 0.2, HEAD_Y * 0.5, c.z + s * 0.42),
                vec3(0.035, HEAD_Y * 0.5, 0.035),
                iron,
                PLAIN,
            );
        }
        let head = vec3(c.x, HEAD_Y + HEAD_HALF.y, c.z);
        self.b.add(head, HEAD_HALF, cream, PLAIN);
        self.b.add(
            head + vec3(0.0, HEAD_HALF.y + 0.015, 0.0),
            vec3(HEAD_HALF.x + 0.02, 0.015, HEAD_HALF.z + 0.02),
            iron,
            PLAIN,
        );
        // Rost- und Gruenflecken auf der Vorderseite des Kopfs
        let face = front - MACHINE_HALF.x + HEAD_HALF.x + 0.003;
        for (dy, dz, hy, hz, col) in [
            (-0.25f32, -0.3f32, 0.08f32, 0.12f32, rust),
            (-0.2, 0.25, 0.1, 0.08, rust),
            (0.3, 0.38, 0.04, 0.06, green),
            (-0.28, 0.05, 0.05, 0.1, green),
        ] {
            self.b.add(
                vec3(face, head.y + dy, c.z + dz),
                vec3(0.002, hy, hz),
                col,
                PLAIN,
            );
        }
        // Anzeigefenster: gekippt leuchten Tag und Zahl, sonst nur Nullen
        let (texts, glow) = if tipped {
            (DISPLAY_TIPPED, 1.4)
        } else {
            (DISPLAY_NORMAL, 0.0)
        };
        for (k, dz) in [-0.2f32, 0.2].into_iter().enumerate() {
            let p = vec3(face + 0.004, head.y + 0.06, c.z + dz);
            self.b
                .add(p, vec3(0.003, 0.11, 0.16), vec3(0.03, 0.04, 0.03), PLAIN);
            self.b.label(
                p + vec3(0.006, 0.0, 0.0),
                FRAC_PI_2,
                0.13,
                0.07,
                texts[k],
                vec3(0.03, 0.04, 0.03),
                3.0,
                glow,
                if tipped { 0.25 } else { 0.0 },
            );
        }
        // Schlaeuche: Duese in der Halterung am Kopf, Schlauch haengt herab und liegt in
        // Schlaufen am Boden
        for s in [-1.0f32, 1.0] {
            let z = c.z + s * (HEAD_HALF.z + 0.06);
            self.b.add(
                vec3(c.x + 0.12, HEAD_Y + 0.25, z),
                vec3(0.06, 0.1, 0.04),
                iron,
                PLAIN,
            );
            self.b.add(
                vec3(c.x + 0.2, HEAD_Y + 0.18, z),
                vec3(0.08, 0.025, 0.025),
                hose,
                PLAIN,
            );
            self.b.add(
                vec3(c.x + 0.3, HEAD_Y * 0.5 + 0.05, z),
                vec3(0.02, HEAD_Y * 0.5, 0.02),
                hose,
                PLAIN,
            );
            for (dx, dz, hx, hz) in [
                (0.55f32, 0.0f32, 0.25f32, 0.02f32),
                (0.78, s * 0.15, 0.02, 0.15),
                (0.6, s * 0.3, 0.2, 0.02),
            ] {
                self.b.add_floor(
                    vec3(c.x + dx, 0.02, z + dz),
                    vec3(hx, 0.02, hz),
                    0.0,
                    hose,
                    PLAIN,
                );
            }
        }
        // Ausgabeschacht vorn: Kasten, dunkle Oeffnung, Klappe darueber, Ablage davor
        let chute = vec3(front + CHUTE_HALF.x, CHUTE_Y, c.z);
        self.b.add_solid(chute, CHUTE_HALF, rust);
        self.b.add(
            chute + vec3(CHUTE_HALF.x + 0.003, 0.02, 0.0),
            vec3(0.003, 0.11, 0.15),
            vec3(0.01, 0.01, 0.01),
            PLAIN,
        );
        self.b.add(
            chute + vec3(CHUTE_HALF.x + 0.01, 0.15, 0.0),
            vec3(0.01, 0.03, 0.17),
            iron,
            PLAIN,
        );
        self.b.add(
            vec3(
                front + 2.0 * CHUTE_HALF.x + 0.08,
                CHUTE_Y - CHUTE_HALF.y + 0.02,
                c.z,
            ),
            vec3(0.08, 0.02, 0.18),
            iron,
            PLAIN,
        );
    }

    /// Warmes Licht ueber der Maschine und Wegweiser unter der Decke.
    /// Bleibt auch im dunklen Markt ruhig an: Man soll ihn finden.
    fn kiosk_signs(&mut self, z: f32, signs_x: &[f32]) {
        let gold = vec3(0.85, 0.5, 0.08);
        let metal = vec3(0.25, 0.25, 0.27);
        // warmes Licht ueber der Maschine
        self.b.add_ex(
            vec3(1.0, 2.15, z),
            vec3(0.1, 0.02, 0.6),
            vec3(1.0, 0.75, 0.4),
            EMISSIVE,
            3.0,
            1.3,
            0.0,
        );
        self.b.light(
            vec3(1.15, 2.0, z),
            5.0,
            vec3(1.0, 0.72, 0.42),
            0.7,
            3.0,
            0.0,
        );
        // Wegweiser: nach Westen geht es zum Kiosk
        // Wegweiser: flache Tafel quer zum Gang, vorn "< Kiosk", hinten "Kiosk >" (von hinten
        // gesehen zeigt der Pfeil richtig nach Westen); Schrift schmaler als die Tafel
        for &x in signs_x {
            let p = vec3(x, 2.85, z + 0.2);
            self.b.add(p, vec3(ARROW_HALF, 0.17, 0.025), metal, PLAIN);
            self.b.add(
                p + vec3(0.0, 0.6, 0.0),
                vec3(0.015, 0.45, 0.015),
                metal,
                PLAIN,
            );
            for (off, yaw, text) in [
                (vec3(0.0, 0.0, 0.035), 0.0, "< Kiosk"),
                (vec3(0.0, 0.0, -0.035), PI, "Kiosk >"),
            ] {
                self.b.label(
                    p + off,
                    yaw,
                    ARROW_HALF - 0.04,
                    0.13,
                    text,
                    gold,
                    0.0,
                    1.2,
                    0.0,
                );
            }
        }
    }
}
