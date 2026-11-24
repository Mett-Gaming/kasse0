//! Kiosk in der Suedwestecke vor den Kassen: Tresen und Rueckwand aus Boxen, dahinter steht der
//! Verkaeufer (Figur, siehe game/pfand/kiosk.rs). Steht in jedem Markt an derselben Stelle.

use super::*;

/// Wo der Kiosk steht.
#[derive(Clone, Copy, Debug)]
pub struct Kiosk {
    /// Mitte des Tresens am Boden
    pub counter: Vec3,
    /// Platz des Verkaeufers
    pub seller: Vec3,
}

const COUNTER_HALF_K: Vec3 = Vec3::new(0.24, 0.52, 0.85);

impl Kiosk {
    /// Tresen mit Verkaeufer, zum Anschauen
    pub fn body(&self) -> Aabb {
        Aabb {
            min: self.counter - vec3(0.8, 0.0, COUNTER_HALF_K.z),
            max: self.counter + vec3(COUNTER_HALF_K.x, 1.9, COUNTER_HALF_K.z),
        }
    }
    /// Wo man als Kunde steht
    pub fn front(&self) -> Vec3 {
        self.counter + vec3(0.75, 0.0, 0.0)
    }
}

impl Gen<'_> {
    /// `signs_x`: x-Stellen der Wegweiser (z = vor den Regalenden).
    pub(super) fn build_kiosk(&mut self, z: f32, signs_x: &[f32]) -> Kiosk {
        let tipped = self.params.tipped;
        let k = Kiosk {
            counter: vec3(1.0, 0.0, z),
            seller: vec3(0.42, 0.0, z),
        };
        let dim = if tipped { 0.55 } else { 1.0 };
        let wood = vec3(0.5, 0.38, 0.24) * dim;
        let blue = vec3(0.14, 0.22, 0.45) * dim;
        let h = COUNTER_HALF_K;
        self.b.add_solid(k.counter + vec3(0.0, h.y, 0.0), h, wood);
        // Platte, Blende
        self.b.add(
            k.counter + vec3(0.0, h.y * 2.0 + 0.02, 0.0),
            vec3(h.x + 0.05, 0.02, h.z + 0.05),
            vec3(0.75, 0.73, 0.68) * dim,
            PLAIN,
        );
        self.b.add(
            k.counter + vec3(h.x + 0.004, 0.5, 0.0),
            vec3(0.006, 0.2, h.z - 0.1),
            blue,
            PLAIN,
        );
        // Seitenwaende mit Pfosten, Dachblende
        for s in [-1.0f32, 1.0] {
            let c = vec3(0.45, 1.1, z + s * (h.z + 0.04));
            self.b
                .add_solid(c, vec3(0.45, 1.1, 0.04), vec3(0.4, 0.4, 0.42) * dim);
            self.b.add(
                vec3(k.counter.x + h.x - 0.03, 1.6, z + s * (h.z + 0.02)),
                vec3(0.03, 0.6, 0.03),
                vec3(0.4, 0.4, 0.42) * dim,
                PLAIN,
            );
        }
        self.b
            .add(vec3(0.66, 2.28, z), vec3(0.62, 0.1, h.z + 0.1), blue, PLAIN);
        self.b.label(
            vec3(1.29, 2.28, z),
            FRAC_PI_2,
            0.75,
            0.08,
            "Kiosk",
            blue,
            0.0,
            if tipped { 0.6 } else { 1.0 },
            if tipped { 0.35 } else { 0.0 },
        );
        self.kiosk_signs(z, signs_x);
        if !tipped {
            self.b.label(
                k.counter + vec3(h.x + 0.014, 0.74, 0.0),
                FRAC_PI_2,
                0.5,
                0.06,
                "Gleich geöffnet",
                vec3(0.92, 0.9, 0.82),
                1.0,
                0.0,
                0.0,
            );
        }
        // Auslage im Ruecken
        for (i, y) in [0.9f32, 1.25, 1.6].iter().enumerate() {
            self.b.add(
                vec3(0.1, *y, z),
                vec3(0.09, 0.012, h.z - 0.05),
                wood * 0.8,
                PLAIN,
            );
            for j in 0..6 {
                let c = [
                    vec3(0.7, 0.2, 0.15),
                    vec3(0.2, 0.5, 0.25),
                    vec3(0.85, 0.7, 0.2),
                    vec3(0.25, 0.35, 0.65),
                ][(i + j) % 4];
                self.b.add(
                    vec3(0.1, y + 0.08, z - 0.65 + j as f32 * 0.26),
                    vec3(0.05, 0.07, 0.07),
                    c * dim,
                    PLAIN,
                );
            }
        }
        k
    }

    /// Grosses Leuchtschild ueber dem Kiosk, warmes Licht und Wegweiser unter der Decke.
    /// Bleibt auch im dunklen Markt ruhig an: Man soll ihn finden.
    fn kiosk_signs(&mut self, z: f32, signs_x: &[f32]) {
        let gold = vec3(0.85, 0.5, 0.08);
        let metal = vec3(0.25, 0.25, 0.27);
        // haengt quer in den Gang, von beiden Seiten und von vorn lesbar
        let c = vec3(1.75, 2.75, z);
        self.b.add(c, vec3(0.34, 0.3, 0.34), metal, PLAIN);
        for (off, yaw) in [
            (vec3(0.36, 0.0, 0.0), FRAC_PI_2),
            (vec3(0.0, 0.0, 0.36), 0.0),
            (vec3(0.0, 0.0, -0.36), PI),
        ] {
            self.b
                .label(c + off, yaw, 0.7, 0.2, "Kiosk", gold, 0.0, 1.6, 0.0);
        }
        self.b.add(
            c + vec3(0.0, 0.65, 0.0),
            vec3(0.02, 0.35, 0.02),
            metal,
            PLAIN,
        );
        // warmes Licht ueber dem Tresen
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
        for &x in signs_x {
            let p = vec3(x, 2.85, z + 0.2);
            self.b.add(p, vec3(0.03, 0.2, 0.03), metal, PLAIN);
            self.b.add(
                p + vec3(0.0, 0.6, 0.0),
                vec3(0.015, 0.45, 0.015),
                metal,
                PLAIN,
            );
            for (off, yaw, text) in [
                (vec3(0.0, 0.0, 0.04), 0.0, "< Kiosk"),
                (vec3(0.0, 0.0, -0.04), PI, "Kiosk >"),
                (vec3(0.04, 0.0, 0.0), FRAC_PI_2, "Kiosk"),
            ] {
                self.b
                    .label(p + off, yaw, 0.6, 0.15, text, gold, 0.0, 1.2, 0.0);
            }
        }
    }
}
