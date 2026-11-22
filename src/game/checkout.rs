//! Die Kasse wie im Discounter: Waren aufs Band legen, das Band faehrt sie zum Scanner,
//! die gesichtslose Kassierer:in scannt Stueck fuer Stueck, bezahlt wird am Kartenterminal.

use super::*;

/// Wie nah der Wagen an der Kasse stehen muss, um ihn aufs Band auszuraeumen
pub(super) const CART_AT_REGISTER: f32 = 3.2;
/// Bandgeschwindigkeit (m/s)
const BELT_SPEED: f32 = 0.45;
/// Abstand zwischen Waren auf dem Band
const BELT_GAP: f32 = 0.05;
/// Sekunden zwischen zwei Waren beim Ausraeumen des Wagens
const UNLOAD_STEP: f32 = 0.35;
/// Wie weit man von der Kasse weg sein darf, damit weiter gescannt wird
const SCAN_PRESENCE: f32 = 3.0;
/// Abstand des Absperrbands vor und hinter den Kassengassen
const BARRIER_GAP: f32 = 0.4;

/// Eine Ware auf dem Band; `s` = Strecke ab Bandanfang.
#[derive(Clone, Debug)]
pub struct BeltItem {
    pub data: ItemData,
    pub s: f32,
}

impl Game {
    fn belt_len(lane: &world::Lane) -> f32 {
        (lane.belt_end - lane.belt_start).length()
    }

    /// Kasse, an der gerade ausgecheckt wird (die offene).
    pub(super) fn belt_box(&self) -> Option<world::Aabb> {
        let lane = self.open_lane()?;
        let mid = (lane.belt_start + lane.belt_end) * 0.5;
        let half = (lane.belt_end - lane.belt_start).abs() * 0.5 + vec3(0.3, 0.06, 0.15);
        Some(world::Aabb::from_center(mid + vec3(0.0, 0.04, 0.0), half))
    }

    pub(super) fn terminal_box(&self) -> Option<world::Aabb> {
        let lane = self.open_lane()?;
        Some(world::Aabb::from_center(
            lane.terminal + vec3(0.0, 0.1, 0.0),
            vec3(0.12, 0.14, 0.12),
        ))
    }

    /// Wird gerade kassiert? (Band belegt oder Wagen wird ausgeraeumt)
    pub fn checking_out(&self) -> bool {
        !self.belt.is_empty() || self.unloading
    }

    /// Alles, was der Spieler eingesammelt hat: Wagen, Band und bereits gescannte Ware.
    pub fn bought(&self) -> impl Iterator<Item = &ItemData> {
        self.cart
            .iter()
            .chain(self.belt.iter().map(|b| &b.data))
            .chain(self.scanned.iter())
    }

    /// Ware hinten aufs Band legen.
    pub(super) fn put_on_belt(&mut self, data: ItemData) {
        let s = self
            .belt
            .last()
            .map_or(0.0, |b| (b.s - b.data.half().z * 2.0 - BELT_GAP).min(0.0));
        self.belt.push(BeltItem { data, s });
        self.play(Sfx::Click, 0.3);
    }

    /// E auf das Band: Ware aus der Hand ablegen oder den Wagen ausraeumen.
    pub(super) fn use_belt(&mut self) {
        if self.paid {
            self.hint("Du hast schon bezahlt.");
            return;
        }
        if let Some(item) = self.held.take() {
            self.put_on_belt(item);
            return;
        }
        let Some(lane) = self.open_lane() else { return };
        if self.cart.is_empty() {
            self.hint("Dein Wagen ist leer.");
        } else if (self.trolley.pos - lane.scan_point).length() > CART_AT_REGISTER {
            self.say("Bring deinen Wagen zur Kasse.", 3.0);
            self.play(Sfx::Error, 0.5);
        } else {
            self.unloading = true;
            self.unload_timer = 0.0;
        }
    }

    /// E am Kartenterminal: bezahlen.
    pub(super) fn use_terminal(&mut self) {
        if self.paid {
            return;
        }
        if !self.belt.is_empty() || self.unloading {
            self.hint("Warte, bis alles gescannt ist.");
            return;
        }
        if self.scanned.is_empty() {
            self.hint("Leg deine Ware zuerst aufs Band.");
            return;
        }
        if !self.scanned_complete() {
            self.say("Dein Zettel ist noch nicht vollständig gescannt.", 3.0);
            self.play(Sfx::Error, 0.5);
            return;
        }
        self.pay();
    }

    /// Ist jeder Zettelpunkt unter der gescannten Ware?
    pub fn scanned_complete(&self) -> bool {
        self.list
            .iter()
            .all(|&l| self.scanned.iter().any(|s| s.list_entry() == Some(l)))
    }

    pub fn scanned_value(&self) -> i64 {
        self.scanned.iter().map(|c| c.cents).sum::<i64>().max(0)
    }

    fn pay(&mut self) {
        self.paid = true;
        self.params.door_open = true;
        self.rebuild_world();
        self.play(Sfx::Beep, 0.5);
        self.play(Sfx::Chime, 0.5);
        self.say(
            format!(
                "Bezahlt: {}. Die Tür ist offen.",
                crate::ui::euro(self.scanned_value())
            ),
            5.0,
        );
        self.announcement = Some((
            "Vielen Dank für Ihren Einkauf. Beehren Sie uns bald wieder.".into(),
            5.0,
        ));
        for c in &mut self.customers {
            c.angry += 0.5;
        }
    }

    /// Band bewegen, Wagen ausraeumen, scannen.
    pub(super) fn update_belt(&mut self, dt: f32) {
        let Some(lane) = self.open_lane() else { return };
        // Wagen Stueck fuer Stueck aufs Band
        if self.unloading {
            self.unload_timer -= dt;
            if self.unload_timer <= 0.0 {
                self.unload_timer = UNLOAD_STEP;
                let room = self.belt.last().is_none_or(|b| b.s > b.data.half().z * 2.0);
                if room {
                    match self.cart.pop() {
                        Some(item) => self.put_on_belt(item),
                        None => self.unloading = false,
                    }
                }
            }
            if (self.trolley.pos - lane.scan_point).length() > CART_AT_REGISTER {
                self.unloading = false;
            }
        }
        // Band faehrt; Waren stauen sich vor dem Scanner
        let len = Self::belt_len(&lane);
        let mut front = len;
        for b in &mut self.belt {
            let d = b.data.half().z * 2.0;
            let limit = front - d * 0.5;
            b.s = (b.s + BELT_SPEED * dt).min(limit);
            front = b.s - d * 0.5 - BELT_GAP;
        }
        // Scannen: nur wenn die erste Ware am Scanner liegt und man dabei ist
        let near = (self.player.pos - lane.scan_point).length() < SCAN_PRESENCE;
        let ready = self
            .belt
            .first()
            .is_some_and(|b| b.s >= len - b.data.half().z - 0.02);
        if !(ready && near) {
            self.scan_timer = 0.0;
            return;
        }
        self.scan_timer += dt;
        if self.scan_timer < SCAN_TIME {
            return;
        }
        self.scan_timer = 0.0;
        let item = self.belt.remove(0).data;
        match item.kind {
            PickupKind::Product { fake: true, .. } => {
                // Die Kassierer:in legt die falsche Ware einfach weg
                self.play(Sfx::Error, 0.6);
                self.suspicion += 0.15;
                self.glitch = self.glitch.max(0.4);
                self.say(format!("ARTIKEL UNBEKANNT: {}", item.name), 2.0);
            }
            _ => {
                self.play(Sfx::Beep, 0.6);
                self.scanned.push(item);
            }
        }
        if self.belt.is_empty() && !self.unloading {
            if self.scanned_complete() {
                // Schreck: die Kassiererin steht auf und beugt sich vor ("Kundenkarte?")
                self.start_cashier_scare();
            }
            let s = if self.scanned_complete() {
                "Alles gescannt. Bezahl am Kartenterminal [E]."
            } else {
                "Alles gescannt - aber dein Zettel ist nicht vollständig."
            };
            self.say(s, 3.5);
        }
    }

    /// Kassierer:in, Stuehle, Rollos, Terminals, Kameras, Band und Ablage zeichnen.
    pub(super) fn collect_checkout_draws(&self, out: &mut Vec<MeshDraw>) {
        let white = Vec4::new(1.0, 1.0, 1.0, 0.0);
        let face_x = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
        let open = self.open_lane().map(|l| l.seat);
        let eye = self.eye();
        for lane in &self.world.lanes {
            out.push(MeshDraw {
                mesh: MeshId::Register as usize,
                model: Mat4::from_rotation_translation(face_x, lane.register),
                tint: white,
            });
            out.push(MeshDraw {
                mesh: MeshId::Chair as usize,
                model: Mat4::from_rotation_translation(face_x, lane.seat),
                tint: Vec4::new(0.55, 0.55, 0.6, 0.0),
            });
            // Kassierer:in nur an der offenen Kasse
            if Some(lane.seat) == open && !self.cashier_standing() {
                // Dienstkleidung: Mannequin in Uniform-Blau (sitzende Pose gibt es nur fuer das Mannequin)
                let uniform = vec3(0.3, 0.6, 0.92);
                let tint = if self.tipped {
                    (uniform * 0.75).extend(0.08)
                } else {
                    uniform.extend(0.0)
                };
                out.push(MeshDraw {
                    mesh: MeshId::Cashier as usize,
                    model: Mat4::from_rotation_translation(
                        face_x,
                        lane.seat + vec3(0.02, 0.18, 0.0),
                    ),
                    tint,
                });
            }
            // Zigaretten-Rollo: im gekippten Markt hochgezogen, dahinter Zigaretten
            let rolled = self.tipped;
            let (sy, y) = if rolled { (0.5, 1.68) } else { (2.3, 1.0) };
            out.push(MeshDraw {
                mesh: MeshId::Shutter as usize,
                model: Mat4::from_scale_rotation_translation(
                    vec3(1.0, sy, 1.0),
                    face_x,
                    vec3(lane.shutter.x, y, lane.shutter.z),
                ),
                tint: Vec4::new(0.8, 0.8, 0.82, 0.0),
            });
            if rolled {
                for row in 0..3 {
                    for k in 0..9 {
                        let p = vec3(
                            lane.shutter.x + 0.04,
                            1.0 + row as f32 * 0.17,
                            lane.shutter.z - 0.56 + k as f32 * 0.14,
                        );
                        out.push(MeshDraw {
                            mesh: MeshId::Cigarettes as usize,
                            model: Mat4::from_rotation_translation(face_x, p),
                            tint: white,
                        });
                    }
                }
            }
            out.push(MeshDraw {
                mesh: MeshId::Terminal as usize,
                model: Mat4::from_rotation_translation(face_x, lane.terminal),
                tint: white,
            });
            // Ueberwachungskamera: im gekippten Markt folgt sie dem Spieler
            let yaw = if self.tipped {
                let d = eye - lane.camera;
                d.x.atan2(d.z)
            } else {
                std::f32::consts::PI * 0.85
            };
            out.push(MeshDraw {
                mesh: MeshId::Camera as usize,
                model: Mat4::from_rotation_translation(
                    Quat::from_rotation_y(yaw),
                    lane.camera - vec3(0.0, 0.21, 0.0),
                ),
                tint: white,
            });
        }
        // Band und Ablage der offenen Kasse
        let Some(lane) = self.open_lane() else { return };
        let len = Self::belt_len(&lane);
        let dir = (lane.belt_end - lane.belt_start) / len;
        for b in &self.belt {
            let p = lane.belt_start + dir * b.s.max(0.0);
            out.push(MeshDraw {
                mesh: b.data.mesh,
                model: Mat4::from_translation(p),
                tint: b.data.tint.extend(0.0),
            });
        }
        for (i, item) in self.scanned.iter().enumerate() {
            let p = lane.bag
                + vec3(
                    ((i % 3) as f32 - 1.0) * 0.12,
                    (i / 9) as f32 * 0.12,
                    ((i / 3) % 3) as f32 * 0.09 - 0.09,
                );
            out.push(MeshDraw {
                mesh: item.mesh,
                model: Mat4::from_rotation_translation(Quat::from_rotation_y(i as f32 * 1.3), p),
                tint: item.tint.extend(0.0),
            });
        }
    }
}

impl Game {
    /// Absperrband vor dem Kassenbereich, solange der Zettel nicht komplett ist:
    /// (x von, x bis, z Nordseite, z Suedseite).
    pub fn checkout_barrier(&self) -> Option<(f32, f32, f32, f32)> {
        let lanes = &self.world.lanes;
        if !self.tipped || self.paid || lanes.is_empty() || self.list_complete() {
            return None;
        }
        let xs: Vec<f32> = lanes.iter().map(|l| l.belt_start.x).collect();
        let spacing = if xs.len() > 1 {
            (xs[1] - xs[0]).abs()
        } else {
            4.0
        };
        let lo = xs.iter().copied().fold(f32::MAX, f32::min) - spacing * 0.5;
        let hi = xs.iter().copied().fold(f32::MIN, f32::max) + spacing * 0.5;
        let z0 = lanes[0].belt_start.z - 0.15 - BARRIER_GAP;
        let z1 = z0 + BARRIER_GAP * 2.0 + 3.0;
        Some((lo.max(0.05), hi, z0, z1))
    }

    /// Haelt Spieler (und geschobenen Wagen) aus dem abgesperrten Kassenbereich heraus.
    pub(super) fn keep_out_of_checkout(&mut self, old: Vec3, mut pos: Vec3) -> Vec3 {
        let Some((x0, x1, z0, z1)) = self.checkout_barrier() else {
            return pos;
        };
        // mit Wagen ragt man weiter nach vorn
        let r = PLAYER_RADIUS
            + if self.trolley.pushing {
                super::hands::CART_OFFSET + 0.42
            } else {
                0.0
            };
        let in_x = |x: f32| x > x0 - r && x < x1 + r;
        let in_z = |z: f32| z > z0 - r && z < z1 + r;
        let mut blocked = false;
        if in_x(pos.x) && in_z(pos.z) && !(in_x(old.x) && in_z(old.z)) {
            if old.z <= z0 - r {
                pos.z = z0 - r;
            } else if old.z >= z1 + r {
                pos.z = z1 + r;
            } else if old.x >= x1 + r {
                pos.x = x1 + r;
            } else {
                pos.x = x0 - r;
            }
            blocked = true;
        }
        if blocked {
            self.hint("Kasse abgesperrt: erst den ganzen Zettel einpacken.");
        }
        pos
    }

    /// Absperrband zeichnen: Pfosten und rot-weisses Flatterband auf beiden Seiten.
    pub(super) fn collect_barrier_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some((x0, x1, z0, z1)) = self.checkout_barrier() else {
            return;
        };
        let post = Vec4::new(0.18, 0.18, 0.2, 0.0);
        for z in [z0, z1] {
            let n = ((x1 - x0) / 2.0).ceil().max(1.0) as usize;
            for k in 0..=n {
                let x = x0 + (x1 - x0) * k as f32 / n as f32;
                out.push(MeshDraw {
                    mesh: MeshId::Cube as usize,
                    model: Mat4::from_scale_rotation_translation(
                        vec3(0.05, 1.0, 0.05),
                        Quat::IDENTITY,
                        vec3(x, 0.5, z),
                    ),
                    tint: post,
                });
            }
            let segs = ((x1 - x0) / 0.3).ceil() as usize;
            for k in 0..segs {
                let x = x0 + (k as f32 + 0.5) * (x1 - x0) / segs as f32;
                let sag = ((k as f32 / segs as f32 * n as f32).fract() - 0.5).powi(2) * 0.08;
                let tint = if k % 2 == 0 {
                    Vec4::new(0.9, 0.1, 0.08, 0.15)
                } else {
                    Vec4::new(0.95, 0.95, 0.92, 0.15)
                };
                out.push(MeshDraw {
                    mesh: MeshId::Cube as usize,
                    model: Mat4::from_scale_rotation_translation(
                        vec3((x1 - x0) / segs as f32, 0.06, 0.01),
                        Quat::IDENTITY,
                        vec3(x, 0.92 - 0.08 + sag, z),
                    ),
                    tint,
                });
            }
        }
    }
}
