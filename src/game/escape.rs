//! Finale nach dem Bezahlen: Durchsage "Kunde 0 hat ein Gesicht", der Alarm geht los, und
//! alle Kunden und Mitarbeiter rennen auf einen zu. Wer den Ausgang erreicht, gewinnt.
//! Wer jetzt erwischt wird, wacht nicht mehr auf: Der Run ist sofort vorbei.
//! In den Kassenbereich kommen die NPCs auch jetzt nicht - aber dort ist kein Ausgang.

use super::*;

/// So schnell rennen alle auf einen zu (Gehen 1,5 m/s, Rennen 2,56 m/s mit Ausdauer)
pub const CHASE_SPEED: f32 = 2.05;
/// Abstand zwischen zwei Sirenentoenen
const ALARM_EVERY: f32 = 1.3;
/// Wer beim Alarm naeher dran ist, startet weiter weg (sonst kaeme man nie hinaus)
pub const HEAD_START: f32 = 10.0;

pub const CHASE_ANNOUNCEMENT: &str = "ACHTUNG: Kunde 0 hat ein Gesicht. Schnappt ihn!";

impl Game {
    /// Laeuft die Flucht zum Ausgang?
    pub fn chasing(&self) -> bool {
        self.chase.is_some() && self.outcome == Outcome::Running
    }

    /// Direkt nach dem Bezahlen: Alarm, alle hinterher.
    pub(super) fn start_chase(&mut self) {
        if !self.tipped || self.chase.is_some() {
            return;
        }
        self.chase = Some(0.0);
        self.alarm_timer = 0.0;
        self.announcement = Some((CHASE_ANNOUNCEMENT.into(), 7.0));
        self.say("Der Alarm geht los. LAUF ZUM AUSGANG!", 5.0);
        self.glitch = 1.0;
        self.red = 0.6;
        self.play(Sfx::Sting, 0.8);
        self.dialog = None;
        // Fairer Start: wer schon an der Kasse lauert, rennt erst von seinem Regal aus los
        let player = self.player.pos;
        for c in &mut self.customers {
            c.stalker = true;
            if (c.pos - player).length() < HEAD_START {
                c.pos = c.home;
            }
            c.field.clear();
        }
        for i in 0..self.employees.len() {
            if (self.employees[i].pos - player).length() < HEAD_START {
                if let Some(p) = self.far_walkable_cell(HEAD_START) {
                    self.employees[i].pos = p;
                }
            }
            let e = &mut self.employees[i];
            e.carrying = false;
            e.state = EmpState::Approach;
            e.ignore = 0.0;
        }
        self.field_timer = 0.0;
    }

    /// Sirene und rotes Blinken waehrend der Flucht.
    pub(super) fn update_chase(&mut self, dt: f32) {
        if !self.chasing() {
            return;
        }
        if let Some(t) = &mut self.chase {
            *t += dt;
        }
        self.alarm_timer -= dt;
        if self.alarm_timer <= 0.0 {
            self.alarm_timer = ALARM_EVERY;
            self.play(Sfx::Alarm, 0.7);
            self.red = self.red.max(0.35);
            self.glitch = self.glitch.max(0.3);
        }
    }

    /// Mitarbeiter in der Flucht: rennen hinterher und koennen jetzt selbst zupacken.
    pub(super) fn chase_employee(&mut self, i: usize, dt: f32) {
        let player = self.player.pos;
        let e = &mut self.employees[i];
        e.state = EmpState::Approach;
        let old = e.pos;
        e.pos = npc::step_along(
            &self.world,
            &self.player_field,
            e.pos,
            player,
            CHASE_SPEED,
            dt,
        );
        e.moving = (e.pos - old).length_squared() > 1e-8;
        e.yaw = npc::yaw_towards(e.pos, player);
        let d = vec3(e.pos.x - player.x, 0.0, e.pos.z - player.z).length();
        if d < CATCH_DIST && !self.god && !self.paid_and_outside() {
            let at = self.employees[i].pos;
            self.caught_by(at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paid_game(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.fill_list_cheap();
        g.scanned.append(&mut g.cart);
        let lane = g.open_lane().unwrap();
        g.player.pos = lane.scan_point;
        g.pay();
        g
    }

    #[test]
    fn paying_starts_alarm_and_everyone_hunts_you() {
        let g = paid_game(41);
        assert!(g.chasing(), "keine Flucht nach dem Bezahlen");
        let a = g.announcement.as_ref().map(|a| a.0.as_str());
        assert_eq!(a, Some(CHASE_ANNOUNCEMENT));
        assert!(
            g.customers.iter().all(|c| c.stalker),
            "nicht alle verfolgen"
        );
        // niemand lauert direkt an der Kasse (ausser er wohnt dort)
        for c in &g.customers {
            let d = (c.pos - g.player.pos).length();
            assert!(
                d >= HEAD_START || c.pos == c.home,
                "Kunde startet {d} m entfernt"
            );
        }
    }

    #[test]
    fn caught_during_the_chase_ends_the_run_at_once() {
        let mut g = paid_game(42);
        g.employees.clear();
        g.customers.truncate(1);
        // Spieler raus aus dem Kassenbereich, ein Kunde direkt daneben - auch wenn man hinsieht
        g.player.pos = g.world.spawn;
        let p = g.player.pos;
        g.customers[0].pos = vec3(p.x + 0.6, 0.0, p.z);
        g.player.yaw = std::f32::consts::FRAC_PI_2;
        g.grace = 0.0;
        let mut inp = Input::default();
        for _ in 0..30 {
            g.update(1.0 / 60.0, &mut inp);
        }
        assert_eq!(g.outcome, Outcome::Caught);
        assert_eq!(g.deaths, 0, "noch einmal aufgewacht");
    }

    #[test]
    fn employees_chase_and_catch_too() {
        let mut g = paid_game(43);
        g.customers.clear();
        g.employees.truncate(1);
        assert_eq!(g.employees.len(), 1);
        g.player.pos = g.world.spawn;
        let p = g.player.pos;
        g.employees[0].pos = vec3(p.x + 3.0, 0.0, p.z);
        g.employees[0].pos = g.world.collide(g.employees[0].pos, 0.25);
        let mut inp = Input::default();
        for _ in 0..60 * 4 {
            g.update(1.0 / 60.0, &mut inp);
        }
        assert_eq!(g.outcome, Outcome::Caught, "Mitarbeiter holt nicht auf");
        assert!(g.dialog.is_none());
    }

    #[test]
    fn npcs_never_enter_the_checkout_area() {
        let mut g = paid_game(44);
        g.employees.clear();
        let lane = g.open_lane().unwrap();
        g.player.pos = lane.scan_point;
        // alle Kunden rund um die Kasse aufstellen
        let (x0, x1, z0, z1) = g.world.checkout_zone().unwrap();
        for (k, c) in g.customers.iter_mut().enumerate() {
            let x = x0 + (x1 - x0) * (k as f32 + 0.5) / 10.0;
            let z = if k % 2 == 0 { z0 - 1.5 } else { z1 + 1.5 };
            c.pos = g.world.collide(vec3(x, 0.0, z), 0.25);
        }
        let mut inp = Input::default();
        for _ in 0..60 * 8 {
            g.update(1.0 / 60.0, &mut inp);
            for c in &g.customers {
                assert!(
                    !g.world.in_checkout_zone(c.pos, 0.0),
                    "Kunde im Kassenbereich"
                );
            }
        }
        assert_eq!(g.outcome, Outcome::Running, "im Kassenbereich erwischt");
    }
}
