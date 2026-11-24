//! Seitentrakt des gekippten Markts: hinter Tueroeffnungen in der Westwand liegt eine Kette
//! identischer, leerer Raeume (niedrige Decke, eine flackernde Roehre, ein einsamer Gegenstand,
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

/// Was im Trakt liegt (fuer das Spiel: Klang, Gestalt, Cheat, Umbau).
#[derive(Clone, Debug)]
pub struct Wing {
    /// Raeume (Innenmasse), aufsteigend in z
    pub rooms: Vec<Aabb>,
    /// Mitten der Tueroeffnungen in der Westwand des Markts (x = 0)
    pub doors: Vec<Vec3>,
    /// Endlos-Flur (innen), nur im Kollaps
    pub hall: Option<Aabb>,
    /// Durchgang ins Nichts (Mitte der Oeffnung), ab "falsch"
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
    /// Trakt bauen. Liefert ihn (mit den Tueroeffnungen fuer die Westwand des Markts) und
    /// die Warnschilder darin.
    pub(super) fn build_wing(
        &mut self,
        aisle_z0: f32,
        aisle_z1: f32,
        wall_color: Vec3,
    ) -> Option<(Wing, Vec<(Vec3, f32)>)> {
        let want = rooms_for(self.params.wing);
        if !self.params.tipped || want == 0 {
            return None;
        }
        let slots = door_slots(aisle_z0, aisle_z1);
        let n = want.min(slots.len());
        if n == 0 {
            return None;
        }
        // Startplatz fest je Markt (nicht je Umbau): der Trakt bleibt, wo er ist
        let start = (self.params.seed as usize / 7) % (slots.len() - n + 1);
        let zs: Vec<f32> = slots[start..start + n].to_vec();
        let x_e = -WALL_T; // Innenkante Westwand des Markts (Wandmitte bei -WALL_T)
        let x_w = x_e - ROOM_W; // Innenkante der Trakt-Westwand
        let wall_c = wall_color * vec3(1.05, 1.0, 0.82); // vergilbt
        let floor_c = vec3(0.47, 0.45, 0.36);
        let ceil_c = vec3(0.78, 0.76, 0.62);
        let tube_c = vec3(1.0, 0.94, 0.7);
        // Gegenstand im Raum: fest je Markt
        let shelf_obj = self.params.seed % 2 == 0;
        let mut rooms = Vec::new();
        let mut void = None;
        let void_room = (self.params.wing >= 3 && n >= 3).then_some(1);
        let hall_room = (self.params.wing >= 4).then_some(n - 1);
        let mut signs = Vec::new();
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
            // eine Roehre, alle im Gleichtakt flackernd
            self.b.add(
                vec3(xc, WING_H - 0.03, zc),
                vec3(0.09, 0.03, 0.9),
                vec3(0.6, 0.6, 0.58),
                PLAIN,
            );
            self.b.add_ex(
                vec3(xc, WING_H - 0.065, zc),
                vec3(0.06, 0.008, 0.87),
                tube_c,
                EMISSIVE,
                TUBE_SEED,
                1.6,
                0.6,
            );
            self.b
                .light(vec3(xc, WING_H - 0.4, zc), 6.0, tube_c, 0.5, TUBE_SEED, 0.6);
            // aufgemalte Tuer an der Westwand, die nirgends hinfuehrt
            if gap.is_none() {
                self.fake_door(vec3(x_w + 0.012, 0.0, zc + 1.6), wall_c);
            }
            // der einsame Gegenstand, immer an derselben Stelle (im letzten Raum gedreht)
            let last = k == n - 1 && n > 1;
            let spot = vec3(x_w + 1.2, 0.0, zc - 2.0);
            if shelf_obj {
                self.empty_rack(spot, if last { FRAC_PI_2 } else { 0.0 });
            } else {
                signs.push((vec3(xc + 0.8, 0.0, zc - 1.2), if last { PI } else { 0.4 }));
            }
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
        if self.params.wing >= 2 && n >= 2 {
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
        Some((
            Wing {
                rooms,
                doors,
                hall,
                void,
                hall_end,
                x_min,
            },
            signs,
        ))
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

    /// Leeres Regalgestell aus Boxen (zwei Seitenteile, drei Boeden), steht im Weg.
    fn empty_rack(&mut self, c: Vec3, yaw: f32) {
        let f = Frame::new(c, yaw);
        let metal = vec3(0.45, 0.46, 0.44);
        for s in [-0.45f32, 0.45] {
            self.b
                .add_rot(f.pos(s, 0.0, 0.9), vec3(0.2, 0.9, 0.02), yaw, metal, PLAIN);
        }
        for y in [0.25f32, 0.85, 1.45] {
            self.b.add_rot(
                f.pos(0.0, 0.0, y),
                vec3(0.2, 0.015, 0.45),
                yaw,
                metal,
                PLAIN,
            );
        }
        self.b
            .solid_rot(f.pos(0.0, 0.0, 0.9), vec3(0.22, 0.9, 0.47), yaw);
    }
}
