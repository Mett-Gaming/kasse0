//! Seitentrakt des gekippten Markts: hinter Tueroeffnungen in der Westwand liegt eine Kette
//! identischer Raeume (niedrige Decke, eine flackernde Roehre, ein einsamer Pfandautomat,
//! eine aufgemalte Tuer, die nirgends hinfuehrt). Ab Stufe "falsch" ein Durchgang ins Nichts,
//! im Kollaps ein langer Flur mit einer Tuer, die sich nicht oeffnet.
//!
//! Alles aus Boxen. Der Trakt liegt bei x < 0 (das Navigationsraster waechst nach links mit).

use super::*;

/// Raumgroesse innen: quer (x) und laengs (z, = Abstand der Poster an der Westwand)
pub const ROOM_W: f32 = 6.0;
pub const ROOM_D: f32 = 7.0;
/// niedrige Decke im Trakt
pub const WING_H: f32 = 2.6;
/// Tueroeffnung in der Westwand des Markts: halbe Breite, Hoehe
const DOOR_HALF: f32 = 1.0;
const DOOR_H: f32 = 2.2;
/// Durchgang zwischen den Raeumen: halbe Breite
const INNER_HALF: f32 = 0.6;
/// Endlos-Flur: Laenge, halbe Breite
pub const HALL_LEN: f32 = 18.0;
const HALL_HALF: f32 = 1.5;
/// Alle Roehren im Trakt flackern im Gleichtakt (gleicher Zufallswert)
const TUBE_SEED: f32 = 4.2;

/// Ein Pfandautomat: steht mit dem Ruecken an der Westwand des Raums, vorne ist +x.
#[derive(Clone, Copy, Debug)]
pub struct DepositMachine {
    /// Mitte am Boden
    pub pos: Vec3,
}

/// Halbe Groesse eines Automaten
const MACHINE_HALF: Vec3 = Vec3::new(0.35, 0.95, 0.45);

impl DepositMachine {
    pub fn body(&self) -> Aabb {
        Aabb::from_center(self.pos + vec3(0.0, MACHINE_HALF.y, 0.0), MACHINE_HALF)
    }
    /// Wo man davorsteht
    pub fn front(&self) -> Vec3 {
        self.pos + vec3(1.1, 0.0, 0.0)
    }
}

/// Was im Trakt liegt (fuer das Spiel: Klang, Gestalt, Cheat, Umbau).
#[derive(Clone, Debug)]
pub struct Wing {
    /// Raeume (Innenmasse), aufsteigend in z
    pub rooms: Vec<Aabb>,
    /// Pfandautomaten, je Raum einer an derselben Stelle
    pub machines: Vec<DepositMachine>,
    /// Mitten der Tueroeffnungen in der Westwand des Markts (x = 0)
    pub doors: Vec<Vec3>,
    /// Endlos-Flur (innen), nur im Kollaps
    pub hall: Option<Aabb>,
    /// Durchgang ins Nichts (Mitte der Oeffnung), ab "falsch" (nur von Tests gelesen)
    #[cfg_attr(not(test), allow(dead_code))]
    pub void: Option<Vec3>,
    /// Tuer am Ende des Flurs (Mitte)
    pub hall_end: Option<Vec3>,
    /// westlichste Stelle (fuer das Navigationsraster)
    pub x_min: f32,
}

impl Wing {
    /// Liegt der Punkt im Trakt (Raum oder Flur)?
    pub fn contains(&self, p: Vec3) -> bool {
        self.rooms
            .iter()
            .chain(self.hall.iter())
            .any(|a| a.contains_xz(p))
    }
    /// Index des Raums, in dem der Punkt liegt.
    pub fn room_of(&self, p: Vec3) -> Option<usize> {
        self.rooms.iter().position(|a| a.contains_xz(p))
    }
}

/// Wie viele Raeume je Stufe des Trakts (1..4).
fn rooms_for(wing: u8) -> usize {
    match wing {
        0 => 0,
        1 => 2,
        2 => 3,
        _ => 4,
    }
}

/// Moegliche Lagen (z) der Tueroeffnungen: mittig zwischen den Plakaten an der Westwand.
pub(super) fn door_slots(aisle_z0: f32, aisle_z1: f32) -> Vec<f32> {
    let mut v = Vec::new();
    let mut z = aisle_z0 + 1.5 + ROOM_D * 0.5;
    while z + ROOM_D * 0.5 <= aisle_z1 - 0.3 {
        v.push(z);
        z += ROOM_D;
    }
    v
}

impl Gen<'_> {
    /// Trakt bauen. Liefert ihn mit den Tueroeffnungen fuer die Westwand des Markts.
    pub(super) fn build_wing(
        &mut self,
        aisle_z0: f32,
        aisle_z1: f32,
        wall_color: Vec3,
    ) -> Option<Wing> {
        // die Pfandraeume gibt es immer: im normalen Markt wie auf der ersten Stufe (gepflegt)
        let level = self.wing_level();
        let want = rooms_for(level);
        if want == 0 {
            return None;
        }
        let slots = door_slots(aisle_z0, aisle_z1);
        let n = want.min(slots.len());
        if n == 0 {
            return None;
        }
        // immer ab dem ersten Platz: der liegt im normalen und im (laengeren) gekippten Markt an
        // derselben Stelle, der Trakt bleibt also beim Kippen, wo er ist
        let zs: Vec<f32> = slots[..n].to_vec();
        let x_e = -WALL_T; // Innenkante Westwand des Markts (Wandmitte bei -WALL_T)
        let x_w = x_e - ROOM_W; // Innenkante der Trakt-Westwand
                                // gepflegt hell, mit dem Verfall vergilbt (wing_look.rs)
        let decay = self.wing_decay();
        let wall_c = self.wing_wall_color(wall_color);
        let floor_c = vec3(0.33, 0.33, 0.32); // Fugen zwischen den Bodenfliesen
        let ceil_c = vec3(0.88, 0.88, 0.86).lerp(vec3(0.78, 0.76, 0.62), decay);
        let tube_c = vec3(1.0, 0.94, 0.7);
        let mut rooms = Vec::new();
        let mut void = None;
        let void_room = (level >= 3 && n >= 3).then_some(1);
        let hall_room = (level >= 4).then_some(n - 1);
        let mut machines = Vec::new();
        for (k, &zc) in zs.iter().enumerate() {
            let (z0, z1) = (zc - ROOM_D * 0.5, zc + ROOM_D * 0.5);
            let xc = (x_e + x_w) * 0.5;
            rooms.push(Aabb {
                min: vec3(x_w, 0.0, z0),
                max: vec3(x_e, WING_H, z1),
            });
            // Boden, Decke
            self.b.add(
                vec3(xc, -0.05, zc),
                vec3(ROOM_W * 0.5 + WALL_T, 0.05, ROOM_D * 0.5),
                floor_c,
                FLOOR,
            );
            self.b.add(
                vec3(xc, WING_H + 0.05, zc),
                vec3(ROOM_W * 0.5 + WALL_T, 0.05, ROOM_D * 0.5 + WALL_T),
                ceil_c,
                CEILING,
            );
            // Westwand des Raums: ganz, mit Oeffnung (Flur) oder mit Durchgang ins Nichts
            let xw = x_w - WALL_T;
            let gap = if Some(k) == hall_room {
                Some(HALL_HALF)
            } else if Some(k) == void_room {
                Some(INNER_HALF)
            } else {
                None
            };
            match gap {
                Some(g) => {
                    self.b.wall(xw, z0, xw, zc - g, 0.0, WING_H, wall_c);
                    self.b.wall(xw, zc + g, xw, z1, 0.0, WING_H, wall_c);
                    self.b.wall(xw, zc - g, xw, zc + g, DOOR_H, WING_H, wall_c);
                }
                None => self.b.wall(xw, z0, xw, z1, 0.0, WING_H, wall_c),
            }
            // Trennwand nach Sueden (erster Raum: geschlossen), mit Durchgang zum Vorraum
            let zw = z0 - WALL_T;
            if k == 0 {
                self.b.wall(x_w, zw, x_e, zw, 0.0, WING_H, wall_c);
            } else {
                self.b
                    .wall(x_w, zw, xc - INNER_HALF, zw, 0.0, WING_H, wall_c);
                self.b
                    .wall(xc + INNER_HALF, zw, x_e, zw, 0.0, WING_H, wall_c);
                self.b.wall(
                    xc - INNER_HALF,
                    zw,
                    xc + INNER_HALF,
                    zw,
                    DOOR_H,
                    WING_H,
                    wall_c,
                );
            }
            if k == n - 1 {
                let zn = z1 + WALL_T;
                self.b.wall(x_w, zn, x_e, zn, 0.0, WING_H, wall_c);
            }
            // zwei Deckenleuchten: gepflegt ruhig, mit dem Verfall flackernd (alle im Gleichtakt)
            let flicker = 0.6 * decay;
            for dz in [-1.6f32, 1.6] {
                let p = vec3(xc, WING_H, zc + dz);
                self.b.add(
                    p - vec3(0.0, 0.03, 0.0),
                    vec3(0.32, 0.03, 0.62),
                    vec3(0.7, 0.7, 0.68),
                    PLAIN,
                );
                self.b.add_ex(
                    p - vec3(0.0, 0.065, 0.0),
                    vec3(0.28, 0.008, 0.58),
                    tube_c,
                    EMISSIVE,
                    TUBE_SEED,
                    2.2 - 0.7 * decay,
                    flicker,
                );
                self.b.light(
                    p - vec3(0.0, 0.4, 0.0),
                    6.5,
                    tube_c,
                    0.95 - 0.4 * decay,
                    TUBE_SEED,
                    flicker,
                );
            }
            // aufgemalte Tuer an der Westwand, die nirgends hinfuehrt
            if gap.is_none() {
                self.fake_door(vec3(x_w + 0.012, 0.0, zc + 1.6), wall_c);
            }
            // der einsame Gegenstand, immer an derselben Stelle: ein Pfandautomat
            let spot = vec3(x_w + MACHINE_HALF.x + 0.02, 0.0, zc - 2.3);
            machines.push(self.deposit_machine(spot));
            // Einrichtung (wing_look.rs): Fliesen, Schilder, Markierung, Kisten
            let has_door = k == 0 || (level >= 2 && n >= 2 && k == n - 1);
            let inner = (xc - INNER_HALF, xc + INNER_HALF);
            let shape = super::wing_look::RoomShape {
                x_w,
                x_e,
                z0,
                z1,
                gaps: [
                    match gap {
                        Some(g) => vec![(zc - g, zc + g)],
                        None => vec![(zc + 1.6 - 0.5, zc + 1.6 + 0.5)], // aufgemalte Tuer
                    },
                    if has_door {
                        vec![(zc - DOOR_HALF - 0.08, zc + DOOR_HALF + 0.08)]
                    } else {
                        vec![]
                    },
                    if k > 0 { vec![inner] } else { vec![] },
                    if k + 1 < n { vec![inner] } else { vec![] },
                ],
            };
            self.dress_wing_room(k, &shape, spot.z);
            if Some(k) == void_room {
                // Durchgang ins Nichts: Rahmen, dahinter tiefes Schwarz; nicht begehbar
                let c = vec3(xw - 0.6, DOOR_H * 0.5, zc);
                self.b.add_ex(
                    c,
                    vec3(0.02, DOOR_H * 0.5, INNER_HALF),
                    vec3(0.0, 0.0, 0.0),
                    EMISSIVE,
                    0.0,
                    0.0,
                    0.0,
                );
                for s in [-1.0f32, 1.0] {
                    self.b.add(
                        vec3(xw - 0.3, DOOR_H * 0.5, zc + s * INNER_HALF),
                        vec3(0.3, DOOR_H * 0.5, 0.02),
                        vec3(0.0, 0.0, 0.0),
                        PLAIN,
                    );
                }
                self.b.add(
                    vec3(xw - 0.3, DOOR_H + 0.01, zc),
                    vec3(0.3, 0.01, INNER_HALF),
                    vec3(0.0, 0.0, 0.0),
                    PLAIN,
                );
                self.b
                    .solid(vec3(xw - 0.3, 1.0, zc), vec3(0.35, 1.0, INNER_HALF + 0.05));
                void = Some(vec3(xw, 0.0, zc));
            }
        }
        // Endlos-Flur hinter dem letzten Raum (nur Kollaps)
        let (mut hall, mut hall_end, mut x_min) = (None, None, x_w - WALL_T * 2.0);
        if let Some(k) = hall_room {
            let zc = zs[k];
            let x1 = x_w - WALL_T * 2.0;
            let x0 = x1 - HALL_LEN;
            let xc = (x0 + x1) * 0.5;
            self.b.add(
                vec3(xc, -0.05, zc),
                vec3(HALL_LEN * 0.5, 0.05, HALL_HALF + WALL_T),
                floor_c,
                FLOOR,
            );
            self.b.add(
                vec3(xc, WING_H + 0.05, zc),
                vec3(HALL_LEN * 0.5, 0.05, HALL_HALF + WALL_T),
                ceil_c,
                CEILING,
            );
            for s in [-1.0f32, 1.0] {
                let zw = zc + s * (HALL_HALF + WALL_T);
                self.b.wall(x0, zw, x1, zw, 0.0, WING_H, wall_c);
            }
            let xe = x0 - WALL_T;
            self.b
                .wall(xe, zc - HALL_HALF, xe, zc + HALL_HALF, 0.0, WING_H, wall_c);
            self.fake_door(vec3(x0 + 0.012, 0.0, zc), wall_c);
            // Lichter werden nach hinten schwaecher
            for i in 0..4 {
                let x = x1 - 2.5 - i as f32 * 4.5;
                let k = 1.0 - i as f32 * 0.22;
                self.b.add_ex(
                    vec3(x, WING_H - 0.065, zc),
                    vec3(0.87, 0.008, 0.06),
                    tube_c * k,
                    EMISSIVE,
                    TUBE_SEED,
                    1.6 * k,
                    0.6,
                );
                self.b.light(
                    vec3(x, WING_H - 0.4, zc),
                    5.5,
                    tube_c,
                    0.45 * k,
                    TUBE_SEED,
                    0.6,
                );
            }
            hall = Some(Aabb {
                min: vec3(x0, 0.0, zc - HALL_HALF),
                max: vec3(x1 + WALL_T * 2.0, WING_H, zc + HALL_HALF),
            });
            hall_end = Some(vec3(x0, 0.0, zc));
            x_min = xe - WALL_T;
        }
        // Tueroeffnungen in die Westwand des Markts: erster und (ab 2) letzter Raum
        let mut doors = vec![vec3(0.0, 0.0, zs[0])];
        if level >= 2 && n >= 2 {
            doors.push(vec3(0.0, 0.0, zs[n - 1]));
        }
        for d in &doors {
            // Tuerrahmen beidseitig
            for s in [-1.0f32, 1.0] {
                self.b.add(
                    vec3(-WALL_T, DOOR_H * 0.5, d.z + s * (DOOR_HALF + 0.04)),
                    vec3(WALL_T + 0.04, DOOR_H * 0.5, 0.04),
                    vec3(0.3, 0.29, 0.25),
                    PLAIN,
                );
            }
            self.b.add(
                vec3(-WALL_T, DOOR_H + 0.04, d.z),
                vec3(WALL_T + 0.04, 0.04, DOOR_HALF + 0.08),
                vec3(0.3, 0.29, 0.25),
                PLAIN,
            );
        }
        Some(Wing {
            rooms,
            machines,
            doors,
            hall,
            void,
            hall_end,
            x_min,
        })
    }

    /// Pfandautomat aus Boxen: Korpus, Oeffnung, Anzeige, Bonschlitz, Schriftzug.
    fn deposit_machine(&mut self, pos: Vec3) -> DepositMachine {
        let m = DepositMachine { pos };
        let h = MACHINE_HALF;
        let body = vec3(0.52, 0.54, 0.5);
        let dark = vec3(0.03, 0.03, 0.03);
        let front = pos.x + h.x;
        self.b.add_solid(pos + vec3(0.0, h.y, 0.0), h, body);
        // Blende und Oeffnung
        self.b.add(
            vec3(front + 0.005, 1.0, pos.z + 0.18),
            vec3(0.006, 0.2, 0.2),
            vec3(0.2, 0.33, 0.55),
            PLAIN,
        );
        self.b.add(
            vec3(front + 0.012, 1.02, pos.z + 0.18),
            vec3(0.006, 0.12, 0.12),
            dark,
            PLAIN,
        );
        // Anzeigerahmen, Bonschlitz, Sockel
        self.b.add(
            vec3(front + 0.004, 1.5, pos.z - 0.14),
            vec3(0.006, 0.13, 0.24),
            dark,
            PLAIN,
        );
        self.b.add(
            vec3(front + 0.006, 1.12, pos.z - 0.2),
            vec3(0.006, 0.012, 0.07),
            dark,
            PLAIN,
        );
        self.b.add(
            vec3(front + 0.004, 0.06, pos.z),
            vec3(0.006, 0.06, h.z),
            dark,
            PLAIN,
        );
        self.b.label(
            vec3(front + 0.012, 1.78, pos.z),
            FRAC_PI_2,
            0.4,
            0.09,
            "Pfand",
            vec3(0.2, 0.33, 0.55),
            0.0,
            0.5,
            0.4,
        );
        m
    }

    /// Aufgemalte Tuer (flach an der Wand, nicht zu oeffnen), Vorderseite nach +x.
    fn fake_door(&mut self, base: Vec3, wall_c: Vec3) {
        let frame = vec3(0.32, 0.31, 0.27);
        self.b.add(
            base + vec3(0.0, 1.05, 0.0),
            vec3(0.012, 1.05, 0.48),
            frame,
            PLAIN,
        );
        self.b.add(
            base + vec3(0.01, 1.02, 0.0),
            vec3(0.012, 1.0, 0.43),
            wall_c * 0.8,
            PLAIN,
        );
        // Klinke
        self.b.add(
            base + vec3(0.03, 1.0, 0.33),
            vec3(0.02, 0.012, 0.06),
            vec3(0.55, 0.55, 0.5),
            PLAIN,
        );
    }
}
