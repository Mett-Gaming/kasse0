//! Mobility-Scooter "Leon Mobility INC" vom Kiosk (Item::Scooter, 10 €): ersetzt den
//! Einkaufswagen. Technisch ist er der Wagen (`Game::trolley`) in anderer Gestalt: Der Korb
//! vorn liegt genau wie der Wagenkorb, deshalb liegen die Waren gleich darin, die Pfandtuete
//! haengt daran, Springen und Kasse funktionieren wie mit dem Wagen. "Schieben" heisst hier
//! sitzen und fahren; Q steigt ab und wieder auf.
//!
//! Auf dem Scooter: schneller als rennen, ohne Ausdauer, faellt nicht auf und macht keine
//! Schritte; man sitzt tiefer (`SEAT_DROP`) und kann sich ducken (unter Huerden im
//! Fluchtkorridor durch). Dazu 2 Leben mehr und Licht wie die Taschenlampe.

use super::pfand::items::Item;
use super::*;
use std::f32::consts::FRAC_PI_2;

/// Fahrtempo (m/s): deutlich schneller als rennen (`RUN_SPEED` 2,56)
pub const SCOOTER_SPEED: f32 = 3.6;
/// So viel tiefer sitzt man als man steht (Auge und Kopf, m)
pub const SEAT_DROP: f32 = 0.35;
/// Kollisionsradius vorn (schmaler als der Wagen, passt durch die Engstellen im Korridor)
pub const SCOOTER_RADIUS: f32 = 0.34;
/// So viele Leben mehr
pub const EXTRA_LIVES: u32 = 2;
/// Abgestellt: Radius der beiden Hindernis-Kreise (Korb vorn, Sitz hinten)
pub const PARKED_RADIUS: f32 = 0.42;
/// Mitte des Sitzes hinter der Wagenposition (m, Modell bei +z)
pub const SEAT_BACK: f32 = 0.85;
/// Lenker: groesster Einschlag (Bogenmass) und Einschlag je Drehgeschwindigkeit (rad/s)
pub const MAX_STEER: f32 = 0.5;
pub const STEER_PER_TURN: f32 = 0.35;
/// Lenksaeule im Modell (scooter.py, Spielkoordinaten): Fusspunkt und Spitze; um diese
/// Achse dreht sich der Lenker
pub const STEER_FOOT: Vec3 = Vec3::new(0.0, 0.2, 0.4);
pub const STEER_TOP: Vec3 = Vec3::new(0.0, 1.0, 0.52);

impl Game {
    /// Hat man den Scooter (statt des Wagens)?
    pub fn has_scooter(&self) -> bool {
        self.pfand.owned.has(Item::Scooter)
    }

    /// Sitzt man gerade auf dem Scooter?
    pub fn riding(&self) -> bool {
        self.has_scooter() && self.trolley.pushing
    }

    /// Lautstaerke des Motors (audio.rs): 0 im Stand oder abgestiegen, 1 bei vollem Tempo.
    pub fn scooter_motor(&self) -> f32 {
        if !self.riding() || self.outcome != Outcome::Running {
            return 0.0;
        }
        (self.player.vel.length() / SCOOTER_SPEED).clamp(0.0, 1.0)
    }

    /// Lenkeinschlag (Bogenmass, + = rechts): folgt der Drehgeschwindigkeit, weich.
    pub(super) fn update_steer(&mut self, dt: f32, turn_rate: f32) {
        let want = if self.riding() {
            (turn_rate * STEER_PER_TURN).clamp(-MAX_STEER, MAX_STEER)
        } else {
            0.0
        };
        self.steer += (want - self.steer) * (dt * 8.0).min(1.0);
    }

    /// Lage des Lenkers im Scooter-Modell: um die Lenksaeule gedreht.
    pub(super) fn steer_local(&self) -> Mat4 {
        let axis = (STEER_TOP - STEER_FOOT).normalize();
        Mat4::from_translation(STEER_FOOT)
            * Mat4::from_axis_angle(axis, -self.steer)
            * Mat4::from_translation(-STEER_FOOT)
    }

    /// Herrenloser Wagen, den das Personal zur Wagenbox bringt (crowd.rs). Den Scooter
    /// schiebt keiner weg.
    pub(super) fn cart_unattended_for_staff(&self) -> bool {
        self.tipped
            && !self.paid
            && !self.trolley.pushing
            && !self.has_scooter()
            && self.cart_distance() > 6.0
    }

    /// Wie viele Treffer man ueberlebt (death.rs): mit Scooter 2 mehr.
    pub(super) fn survivable_hits(&self) -> u32 {
        super::death::SURVIVABLE_HITS + if self.has_scooter() { EXTRA_LIVES } else { 0 }
    }

    /// Auf dem Sitz tiefer als im Stehen.
    pub(super) fn seat_drop(&self) -> f32 {
        if self.riding() {
            SEAT_DROP
        } else {
            0.0
        }
    }

    /// Gerade gekauft: Der Wagen verschwindet, an seiner Stelle steht der Scooter neben der
    /// Maschine; was im Wagen lag, liegt jetzt in seinem Korb.
    pub(super) fn spawn_scooter(&mut self) {
        self.trolley.pushing = false;
        for e in &mut self.employees {
            e.carrying = false;
        }
        let r = PARKED_RADIUS;
        let (pos, yaw) = match self.world.kiosk {
            // neben der Maschine, Korb nach Osten in den Raum
            Some(k) => {
                let f = k.front();
                // erster Platz, an dem Korb und Sitz frei stehen
                let spots = [
                    f + vec3(0.9, 0.0, -1.4),
                    f + vec3(0.9, 0.0, 1.4),
                    f + vec3(1.6, 0.0, 0.0),
                ];
                let fits = |p: Vec3| {
                    let seat = p - vec3(SEAT_BACK, 0.0, 0.0);
                    self.world.collide(p, r).distance(p) < 0.01
                        && self.world.collide(seat, r).distance(seat) < 0.01
                };
                let p = spots
                    .into_iter()
                    .find(|&p| fits(p))
                    .unwrap_or_else(|| self.world.collide(spots[2], r));
                // Korb nach Osten (+x), Sitz zur Maschine hin
                (p, FRAC_PI_2)
            }
            // ohne Kiosk (Cheat im normalen Markt): vor einem
            None => {
                let f = self.view_dir();
                let p = self.player.pos + vec3(f.x, 0.0, f.z).normalize_or_zero() * 1.6;
                (self.world.collide(p, r), self.player.yaw)
            }
        };
        self.trolley.pos = pos;
        self.trolley.yaw = yaw;
        self.cart_sel = None;
        self.say(
            "Neben der Maschine steht jetzt ein Scooter. Dein Wagen ist weg, alles liegt im Korb. [Q] aufsteigen.",
            5.0,
        );
    }

    /// Abgestellter Scooter als Hindernis: Kreise um Korb und Sitz.
    pub(super) fn parked_scooter_push(&self, pos: Vec3) -> Vec3 {
        let (s, c) = self.trolley.yaw.sin_cos();
        let back = -vec3(s, 0.0, -c);
        let mut p = pos;
        for centre in [self.trolley.pos, self.trolley.pos + back * SEAT_BACK] {
            let d = vec3(p.x - centre.x, 0.0, p.z - centre.z);
            let min = PARKED_RADIUS + PLAYER_RADIUS;
            if d.length() < min && d.length() > 1e-4 {
                p = self
                    .world
                    .collide(centre + d.normalize() * min, PLAYER_RADIUS);
            }
        }
        p
    }

    /// Q am Scooter: aufsteigen setzt einen auf den Sitz (hinter den Korb).
    pub(super) fn mount_scooter(&mut self) {
        let (s, c) = self.trolley.yaw.sin_cos();
        let back = -vec3(s, 0.0, -c);
        self.player.pos = self
            .world
            .collide(self.trolley.pos + back * SEAT_BACK, PLAYER_RADIUS);
        self.player.yaw = self.trolley.yaw;
        self.player.vel = Vec3::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::super::pfand::items::Item;
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn with_scooter(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        g.receive_item(Item::Scooter);
        g
    }

    fn drive(g: &mut Game, secs: f32, f: impl Fn(&mut Input)) {
        for _ in 0..(secs / DT) as usize {
            g.customers.clear();
            g.employees.clear();
            let mut i = Input::default();
            f(&mut i);
            g.update(DT, &mut i);
        }
    }

    #[test]
    fn the_scooter_replaces_the_cart_and_keeps_its_goods() {
        let mut g = Game::new(70);
        g.tip();
        g.fill_list_cheap();
        let n = g.cart.len();
        assert!(n > 0);
        g.trolley.pushing = true;
        g.receive_item(Item::Scooter);
        assert!(g.has_scooter() && !g.trolley.pushing, "nicht abgestellt");
        assert_eq!(g.cart.len(), n, "Waren aus dem Wagen verloren");
        // steht neben der Maschine
        let k = g.world.kiosk.unwrap();
        assert!(g.trolley.pos.distance(k.front()) < 2.5, "nicht am Kiosk");
        // nur noch der Scooter wird gezeichnet, kein Wagen
        let mut d = Vec::new();
        g.collect_hand_draws(&mut d);
        assert!(d.iter().any(|m| m.mesh == MeshId::Scooter as usize));
        let own_carts = d
            .iter()
            .filter(|m| m.mesh == MeshId::Cart as usize)
            .filter(|m| m.model.w_axis.truncate().distance(g.trolley.pos) < 0.5)
            .count();
        assert_eq!(own_carts, 0, "Wagen ist noch da");
    }

    #[test]
    fn riding_is_fast_quiet_low_and_you_can_duck() {
        let mut g = with_scooter(71);
        // am Eingang, Korb in den Markt (freie Fahrt nach -z)
        g.trolley.pos = g.world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET);
        g.trolley.yaw = 0.0;
        g.player.pos = g.world.spawn;
        g.toggle_cart();
        assert!(g.riding(), "nicht aufgestiegen");
        let stand = EYE_HEIGHT;
        assert!(g.eye().y < stand - SEAT_DROP + 0.05, "sitzt nicht");
        // schnell, ohne Ausdauer, keine Auffaelligkeit beim Fahren, keine Schritte
        g.suspicion = 0.0;
        let from = g.player.pos;
        let mut top = 0.0f32;
        for _ in 0..60 {
            g.customers.clear();
            g.employees.clear();
            g.update(
                DT,
                &mut Input {
                    forward: true,
                    run: true,
                    ..Default::default()
                },
            );
            top = top.max(g.player.vel.length());
        }
        assert!(top > RUN_SPEED * 1.2, "nicht schneller als rennen: {top}");
        assert!(g.player.pos.distance(from) > 1.0, "faehrt nicht");
        assert!(g.player.stamina > 0.99 && !g.player.running);
        assert_eq!(g.suspicion, 0.0);
        // ducken: Kopf unter jeder Duck-Huerde (Unterkante ab 1,05 m)
        drive(&mut g, 0.6, |i| i.crouch = true);
        assert!(g.riding());
        assert!(g.head_top() < 1.05, "Kopf zu hoch: {}", g.head_top());
        // absteigen: wieder stehend
        g.toggle_cart();
        assert!(!g.riding());
        drive(&mut g, 0.5, |_| {});
        assert!(g.eye().y > stand - 0.05);
    }

    #[test]
    fn the_scooter_stands_free_beside_the_machine_in_every_market() {
        for seed in 0..10u64 {
            let mut g = with_scooter(80 + seed);
            let p = g.trolley.pos;
            let k = g.world.kiosk.unwrap();
            assert!(p.distance(k.front()) < 2.5, "{seed}: zu weit weg");
            assert!(!g.world.in_checkout(p), "{seed}: im Kassenbereich");
            // Korb und Sitz stehen frei, man kann hin und aufsteigen
            let (s, c) = g.trolley.yaw.sin_cos();
            let seat = p - vec3(s, 0.0, -c) * SEAT_BACK;
            for q in [p, seat] {
                assert!(
                    g.world.collide(q, 0.3).distance(q) < 0.05,
                    "{seed}: steckt fest"
                );
            }
            let cells = g.world.reachable_centers();
            assert!(cells.iter().any(|c| c.distance(seat) < 1.2), "{seed}");
            g.player.pos = k.front();
            g.toggle_cart();
            assert!(g.riding(), "{seed}: nicht aufgestiegen");
        }
    }

    #[test]
    fn the_scooter_is_bought_with_all_receipts_together() {
        // am Kiosk mit vier Bons, zusammen 11,70 €
        let mut g = Game::new(91);
        g.pfand_cheat(3);
        g.pfand.note_read = true;
        g.pfand.bons = vec![300, 500, 250, 120];
        let need = g.bons_for(Item::Scooter, 0).expect("reicht nicht");
        // groesste zuerst: 5,00 + 3,00 + 2,50 = 10,50, Rest 0,50 verfaellt; 1,20 bleibt
        assert_eq!(need.0.len(), 3);
        assert_eq!(need.1, 50);
        g.open_kiosk();
        let w = g.pfand.kiosk.as_mut().unwrap();
        w.column = super::super::pfand::kiosk::Column::Items;
        w.item = Item::ALL.len() - 1;
        g.kiosk_choose();
        assert!(g.pfand.kiosk.as_ref().unwrap().text.contains("3 Bons"));
        g.kiosk_choose();
        assert!(g.has_scooter(), "nicht gekauft");
        assert_eq!(g.pfand.bons, vec![120]);
        assert_eq!(g.pfand.tray, None, "Scooter in der Ablage");
        // andere Items: weiter ein Bon pro Kauf
        assert!(g.bons_for(Item::Torch, 0).is_none());
        // zu wenig fuer den Scooter
        g.pfand.bons = vec![500, 400];
        assert!(g.bons_for(Item::Scooter, 0).is_none());
    }

    #[test]
    fn bottles_come_back_over_time() {
        let mut g = Game::new(92);
        g.tip();
        let full = g.pfand.bottles.len();
        g.pfand.bottles.truncate(2);
        drive(
            &mut g,
            super::super::pfand::REFILL_EVERY * 3.0 + 1.0,
            |_| {},
        );
        let n = g.pfand.bottles.len();
        assert!((4..=5).contains(&n), "{n} Flaschen (voll {full})");
    }

    #[test]
    fn handlebar_steers_motor_hums_and_the_view_stays_calm() {
        let mut g = with_scooter(73);
        g.trolley.pos = g.world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET);
        g.trolley.yaw = 0.0;
        g.player.pos = g.world.spawn;
        g.toggle_cart();
        assert_eq!(g.scooter_motor(), 0.0, "Motor im Stand");
        // geradeaus: Motor laut, Lenker gerade, Blick wippt nicht
        let mut heights = Vec::new();
        for _ in 0..60 {
            g.customers.clear();
            g.employees.clear();
            g.update(
                DT,
                &mut Input {
                    forward: true,
                    ..Default::default()
                },
            );
            heights.push(g.eye().y);
        }
        assert!(
            g.scooter_motor() > 0.8,
            "Motor zu leise: {}",
            g.scooter_motor()
        );
        assert!(g.steer.abs() < 0.05);
        let (lo, hi) = heights[30..]
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
        assert!(hi - lo < 1e-3, "Blick wippt: {}", hi - lo);
        // rechts herum: Lenker schlaegt nach rechts ein; der Lenker wird gedreht gezeichnet
        for _ in 0..20 {
            g.player.yaw += 1.2 * DT;
            g.customers.clear();
            g.employees.clear();
            g.update(
                DT,
                &mut Input {
                    forward: true,
                    ..Default::default()
                },
            );
        }
        assert!(g.steer > 0.2, "lenkt nicht rechts: {}", g.steer);
        let tip = g.steer_local().transform_point3(vec3(0.27, 1.0, 0.54));
        assert!(tip.z > 0.54 + 0.05, "rechter Griff kommt nicht zum Fahrer");
        let mut d = Vec::new();
        g.collect_hand_draws(&mut d);
        assert!(d.iter().any(|m| m.mesh == MeshId::ScooterBar as usize));
        // links herum
        for _ in 0..40 {
            g.player.yaw -= 1.2 * DT;
            g.update(DT, &mut Input::default());
        }
        assert!(g.steer < -0.2, "lenkt nicht links: {}", g.steer);
        // abgestiegen: Motor aus, Lenker kehrt zurueck
        g.toggle_cart();
        drive(&mut g, 1.0, |_| {});
        assert_eq!(g.scooter_motor(), 0.0);
        assert!(g.steer.abs() < 0.05);
    }

    #[test]
    fn looking_at_the_seat_offers_to_ride() {
        let mut g = with_scooter(74);
        let (s, c) = g.trolley.yaw.sin_cos();
        let seat = g.trolley.pos - vec3(s, 0.0, -c) * SEAT_BACK;
        // seitlich vor den Sitz stellen und hinschauen
        let side = vec3(c, 0.0, s);
        g.player.pos = g.world.collide(seat + side * 1.2, PLAYER_RADIUS);
        let to = seat + vec3(0.0, 0.55, 0.0) - g.eye();
        g.player.yaw = to.x.atan2(-to.z);
        g.player.pitch = (to.y / to.length()).asin();
        g.update_focus();
        assert_eq!(g.focus, Some(Focus::Cart), "Sitz nicht anschaubar");
    }

    #[test]
    fn the_scooter_gives_two_more_lives_and_light() {
        let mut g = Game::new(72);
        assert_eq!(g.survivable_hits(), super::super::death::SURVIVABLE_HITS);
        let dark = g.camera_light();
        g.receive_item(Item::Scooter);
        assert_eq!(
            g.survivable_hits(),
            super::super::death::SURVIVABLE_HITS + 2
        );
        assert!(g.camera_light() > dark * 1.5, "kein Licht");
        // Mitarbeiter schieben den abgestellten Scooter nicht weg
        assert!(!g.cart_unattended_for_staff());
    }
}
