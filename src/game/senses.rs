//! Wahrnehmung der NPCs: Sehen (Reichweite, beim Personal ein Sichtkegel), Hoeren
//! (Geraeusche des Spielers: Schritte, Rennen, quietschender Wagen, fallende Ware) und ein
//! Gedaechtnis, wo der Spieler zuletzt war. Verfolger jagen die vermutete Position, nicht
//! die echte: Wer sich ungesehen und leise davonmacht, haengt sie ab. Dann suchen sie dort,
//! wo sie einen zuletzt bemerkt haben, und geben irgendwann auf.

use super::*;

/// Hoerweite der Schritte (Meter): Rennen ist laut, Gehen leise
pub const NOISE_RUN: f32 = 15.0;
pub const NOISE_WALK: f32 = 3.5;
/// Hinfallende Ware
pub const NOISE_DROP: f32 = 8.0;
/// So oft schaut ein NPC nach dem Spieler (Sekunden; Sichtpruefungen sind teuer)
const SENSE_EVERY: f32 = 0.25;
/// Sichtweite der Verfolger (rundum: sie "spueren" einen); ganz nah merken sie einen immer
const STALKER_SIGHT: f32 = 22.0;
const STALKER_NEAR: f32 = 3.0;
/// Personal: Sichtweite, Sichtkegel (Kosinus des halben Winkels), Naehe, in der man immer
/// bemerkt wird
const STAFF_SIGHT: f32 = 9.0;
const STAFF_CONE: f32 = 0.45;
const STAFF_NEAR: f32 = 2.5;
/// So alt darf das Wissen sein, damit es als "sieht dich gerade" gilt
pub const MEMORY_FRESH: f32 = 0.6;
/// So lange suchen Verfolger, bevor sie aufgeben
pub const MEMORY_FORGET: f32 = 25.0;

/// Was ein NPC weiss, und wohin er gerade unterwegs ist (Wahrnehmung und Ausweichen).
#[derive(Clone, Copy, Debug)]
pub struct Brain {
    /// zuletzt bemerkte Spielerposition (gesehen oder gehoert)
    pub memory: Option<Vec3>,
    /// Sekunden seit dieser Wahrnehmung
    pub mem_age: f32,
    /// bis zur naechsten Sichtpruefung
    pub sense_cd: f32,
    /// Personal: sieht den Spieler gerade
    pub sees_player: bool,
    /// Personal: geht einem Geraeusch nach
    pub investigate: Option<Vec3>,
    /// Personal: steht am Geraeuschort und schaut sich um (Sekunden)
    pub look_around: f32,
    /// Feststecken erkennen
    pub motion: npc::Motion,
    /// Ausweichschritt nach dem Feststecken: (Ziel, Restzeit)
    pub detour: Option<(Vec3, f32)>,
    /// Flankieren: bis zur naechsten Neuberechnung des Weges
    pub flank_cd: f32,
    /// war im letzten Bild auf der Jagd
    pub hunting: bool,
}

impl Default for Brain {
    fn default() -> Self {
        Self {
            memory: None,
            mem_age: f32::MAX,
            sense_cd: 0.0,
            sees_player: false,
            investigate: None,
            look_around: 0.0,
            motion: Default::default(),
            detour: None,
            flank_cd: 0.0,
            hunting: false,
        }
    }
}

impl Brain {
    /// Weiss der NPC gerade sicher, wo der Spieler ist?
    pub fn fresh(&self) -> bool {
        self.memory.is_some() && self.mem_age < MEMORY_FRESH
    }

    /// Spieler bemerkt (gesehen oder gehoert) an `pos`.
    pub fn notice(&mut self, pos: Vec3, seen: bool) {
        self.memory = Some(pos);
        // gehoert ist nicht gesehen: man weiss nur ungefaehr, wo
        self.mem_age = if seen { 0.0 } else { MEMORY_FRESH };
    }
}

impl Game {
    /// Ein Geraeusch an `pos`, hoerbar im Umkreis `radius`.
    pub(super) fn noise(&mut self, pos: Vec3, radius: f32) {
        self.noises.push((pos, radius));
    }

    /// Wahrnehmung aller NPCs (vor ihrer Bewegung).
    pub(super) fn update_senses(&mut self, dt: f32) {
        let eye = self.eye();
        let player = self.player.pos;
        let noises = std::mem::take(&mut self.noises);
        // Kunden: nur Verfolger achten auf einen
        for i in 0..self.customers.len() {
            let (pos, stalker) = (self.customers[i].pos, self.customers[i].stalker);
            let b = &mut self.customers[i].brain;
            b.mem_age += dt;
            b.sense_cd -= dt;
            if b.mem_age > MEMORY_FORGET {
                b.memory = None;
            }
            if !stalker {
                continue;
            }
            for &(p, r) in &noises {
                if (pos - p).length() < r && !b.fresh() {
                    b.notice(p, false);
                }
            }
            if b.sense_cd > 0.0 {
                continue;
            }
            b.sense_cd = SENSE_EVERY;
            let head = pos + vec3(0.0, 1.6, 0.0);
            let dist = (head - eye).length();
            let sees = dist < STALKER_NEAR
                || (dist < STALKER_SIGHT && self.world.line_of_sight(head, eye));
            if sees {
                self.customers[i].brain.notice(player, true);
            }
        }
        // Personal: Sichtkegel nach vorne, hoert Laerm
        for i in 0..self.employees.len() {
            let e = &self.employees[i];
            let (pos, yaw, busy) = (e.pos, e.yaw, e.carrying || e.ignore > 0.0);
            let e = &mut self.employees[i];
            e.brain.mem_age += dt;
            e.brain.sense_cd -= dt;
            if !busy {
                for &(p, r) in &noises {
                    if (pos - p).length() < r {
                        e.brain.investigate = Some(p);
                        e.brain.look_around = 0.0;
                        e.heard = 6.0;
                    }
                }
            }
            let b = &mut self.employees[i].brain;
            if b.sense_cd > 0.0 {
                continue;
            }
            b.sense_cd = SENSE_EVERY;
            let head = pos + vec3(0.0, 1.7, 0.0);
            let to = eye - head;
            let dist = vec3(to.x, 0.0, to.z).length();
            let facing = vec3(yaw.sin(), 0.0, yaw.cos());
            let in_cone = facing.dot(vec3(to.x, 0.0, to.z) / dist.max(1e-3)) > STAFF_CONE;
            let sees = dist < STAFF_NEAR
                || (dist < STAFF_SIGHT && in_cone && self.world.line_of_sight(head, eye));
            let b = &mut self.employees[i].brain;
            b.sees_player = sees;
            if sees {
                b.notice(player, true);
                b.investigate = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_game(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        g.scare_cd = 1e6;
        g
    }

    /// Mitarbeiter `d` Meter hinter dem Spieler, mit dem Ruecken zu ihm (sieht ihn nicht).
    fn staff_behind(g: &mut Game, d: f32) -> usize {
        let p = g.player.pos;
        let mut best = None;
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let want = p + vec3(a.sin(), 0.0, a.cos()) * d;
            let q = g.world.collide(want, 0.3);
            if (q - want).length() < 0.05 {
                best = Some(q);
                break;
            }
        }
        let pos = best.expect("kein freier Platz");
        let mut e = npc::Employee::new(pos);
        e.ignore = 0.0;
        e.yaw = npc::yaw_towards(p, pos); // schaut vom Spieler weg
        g.employees.push(e);
        g.employees.len() - 1
    }

    #[test]
    fn staff_hears_running_from_far_but_not_walking() {
        let mut g = quiet_game(3);
        let i = staff_behind(&mut g, 12.0);
        let p = g.player.pos;
        g.noise(p, NOISE_WALK);
        g.update_senses(0.016);
        assert!(
            g.employees[i].brain.investigate.is_none(),
            "hoert leises Gehen auf 12 m"
        );
        g.noise(p, NOISE_RUN);
        g.update_senses(0.016);
        let heard = g.employees[i]
            .brain
            .investigate
            .expect("hoert Rennen nicht");
        assert!((heard - p).length() < 0.1);
    }

    #[test]
    fn staff_only_sees_what_is_in_front() {
        let mut g = quiet_game(4);
        let i = staff_behind(&mut g, 5.0);
        g.update_senses(0.016);
        assert!(
            !g.employees[i].brain.sees_player,
            "sieht durch den Hinterkopf"
        );
        // umdrehen
        let e = &mut g.employees[i];
        e.yaw = npc::yaw_towards(e.pos, g.player.pos);
        e.brain.sense_cd = 0.0;
        g.update_senses(0.016);
        let seen = g.employees[i].brain.sees_player;
        let los = g
            .world
            .line_of_sight(g.employees[i].pos + vec3(0.0, 1.7, 0.0), g.eye());
        assert_eq!(seen, los, "Sichtkegel nach vorne wirkt nicht");
    }

    #[test]
    fn stalker_loses_you_and_searches_where_you_were() {
        let mut g = quiet_game(7);
        let p = g.player.pos;
        g.spawn_customer_at(g.world.collide(p + vec3(0.0, 0.0, 3.0), 0.25));
        let c = &mut g.customers[0];
        c.stalker = true;
        c.angry = 0.0;
        g.update_senses(0.016);
        assert!(
            g.customers[0].brain.fresh()
                || !g
                    .world
                    .line_of_sight(g.customers[0].pos + vec3(0.0, 1.6, 0.0), g.eye())
        );
        // Spieler ist ploetzlich weit weg und ausser Sicht: das Wissen veraltet
        g.customers[0].brain.notice(p, true);
        for _ in 0..10 {
            g.customers[0].brain.sense_cd = 1.0; // keine neue Sicht
            g.update_senses(0.1);
        }
        let b = g.customers[0].brain;
        assert!(!b.fresh(), "weiss nach 1 s ohne Sicht noch genau Bescheid");
        assert_eq!(b.memory, Some(p), "vergisst den letzten Ort sofort");
        for _ in 0..300 {
            g.customers[0].brain.sense_cd = 1.0;
            g.update_senses(0.1);
        }
        assert!(g.customers[0].brain.memory.is_none(), "gibt nie auf");
    }
}
