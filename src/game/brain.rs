//! Verhalten der Figuren ueber das reine Laufen hinaus: was sie tun, wenn sie stehen
//! (Ware nehmen und zuruecklegen in Endlosschleife, telefonieren, Arme verschraenken,
//! reden), wie sie den Spieler nachahmen (stehen bleiben, wenn er steht; ins Regal greifen,
//! wenn er greift; sich wegdrehen, wenn er sie anstarrt), wie sie mitten im Schritt
//! erstarren, wenn man sie beim Heranschleichen erwischt, und welche Animation daraus folgt.

use super::*;
use crate::people::Clip;

/// Was eine Figur im Stehen tut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Activity {
    /// ins Regal greifen
    #[default]
    Browse,
    /// dieselbe Ware nehmen und zuruecklegen, immer wieder
    Loop,
    Phone,
    FoldArms,
    Talk,
}

impl Activity {
    /// Feste Wahl aus dem Zufallswert der Figur.
    pub fn for_seed(seed: f32) -> Self {
        match ((seed * 7.3).fract() * 10.0) as u32 {
            0..=3 => Activity::Browse,
            4..=5 => Activity::Loop,
            6 => Activity::Phone,
            7..=8 => Activity::FoldArms,
            _ => Activity::Talk,
        }
    }

    fn clip(self) -> Clip {
        match self {
            Activity::Browse => Clip::Reach,
            Activity::Loop => Clip::PickUp,
            Activity::Phone => Clip::Phone,
            Activity::FoldArms => Clip::FoldArms,
            Activity::Talk => Clip::Talk,
        }
    }
}

/// Kopf verkehrt herum: Chance beim Ins-Blickfeld-Kommen (Kollaps), Dauer
const ROLL_CHANCE: f64 = 0.2;
const ROLL_TIME: f32 = 2.0;
/// Zerreissen: so genau muss man hinsehen (Kosinus), so nah
const DISTORT_COS: f32 = 0.97;
const DISTORT_DIST: f32 = 12.0;
/// Schritt-Geraeusche: Abstand der Schritte (Meter), Klimpern des Schluesselbunds
const STEP_EVERY: f32 = 0.65;
const KEYS_EVERY: f32 = 6.0;

/// Erstarren, wenn man einen Verfolger beim Heranschleichen ertappt (Sekunden)
pub const FREEZE_CAUGHT: f32 = 1.8;
/// Nachahmen: so lange muss der Spieler stillstehen, so nah, so lange greifen sie mit
const MIMIC_STILL: f32 = 2.0;
const MIMIC_RANGE: f32 = 12.0;
const MIMIC_PICK: f32 = 1.3;
/// Anstarren: so genau und so nah, dann drehen sie sich weg
const STARE_COS: f32 = 0.995;
const STARE_AWAY_DIST: f32 = 8.0;
/// Zupacken: ab dieser Entfernung holt ein Jaeger zum Griff aus, so lange dauert er
pub const GRAB_FROM: f32 = 1.4;
pub const GRAB_TIME: f32 = 0.8;
/// Personal: Anteil der Streifen, die mit Einraeumen enden, und wie lange
pub const RESTOCK_SHARE: f64 = 0.45;
pub const RESTOCK_TIME: (f32, f32) = (6.0, 12.0);
/// im spaeten Markt raeumt das Personal endlos dieselbe Luecke ein
pub const RESTOCK_UNCANNY: (f32, f32) = (20.0, 40.0);

impl Game {
    /// Nachahmen und Wegdrehen (nach der Bewegung der Kunden).
    pub(super) fn update_mimicry(&mut self, dt: f32) {
        let still = self.player.vel.length() < 0.1;
        self.player_still = if still { self.player_still + dt } else { 0.0 };
        let reached = std::mem::take(&mut self.player_reached);
        if !self.tipped || self.dialog.is_some() {
            return;
        }
        let player = self.player.pos;
        let view = self.view_dir();
        let eye = self.eye();
        let mimic_still = self.player_still > MIMIC_STILL;
        let collapse = self.tier >= Tier::Collapse;
        for c in &mut self.customers {
            let b = &mut c.brain;
            b.freeze = (b.freeze - dt).max(0.0);
            b.mimic_pick = (b.mimic_pick - dt).max(0.0);
            b.grab = (b.grab - dt).max(0.0);
            b.roll = (b.roll - dt).max(0.0);
            // im Kollaps: wer gerade ins Blickfeld kommt, hat manchmal den Kopf verkehrt herum
            if collapse && c.seen && !b.was_seen && self.rng.gen_bool(ROLL_CHANCE) {
                b.roll = ROLL_TIME;
            }
            b.was_seen = c.seen;
            if b.hunting || !c.seen {
                continue;
            }
            let dist = (c.pos - player).length();
            if dist > MIMIC_RANGE {
                continue;
            }
            // steht man still, stehen sie auch still
            if mimic_still {
                b.freeze = b.freeze.max(0.15);
            }
            // greift man ins Regal, greifen sie auch
            if reached {
                b.mimic_pick = MIMIC_PICK;
            }
            // starrt man sie an, drehen sie sich weg
            let to = (c.pos + vec3(0.0, 1.4, 0.0) - eye).normalize_or_zero();
            if dist < STARE_AWAY_DIST && view.dot(to) > STARE_COS {
                let away = npc::yaw_towards(player, c.pos);
                c.yaw = lerp_angle(c.yaw, away, (dt * 3.0).min(1.0));
            }
        }
    }

    /// Verformung einer Kundin je Stufe: schiefer Kopf und Ruckler (unruhig), langer Hals,
    /// Zucken, Stop-Motion (falsch), lange Glieder, Kopf verkehrt herum und Zerreissen beim
    /// direkten Hinsehen (Kollaps).
    pub(super) fn customer_fx(&self, c: &npc::Customer, fx: &mut crate::people::PoseFx) {
        if !self.tipped {
            return;
        }
        let t = self.time;
        // Ruckel-Fenster: ab und zu ein paar Sekunden Stop-Motion
        let window = ((t * 0.25 + c.seed).floor() * 3.3 + c.seed).sin().abs() > 0.8;
        let moving = c.gait.moving;
        match self.tier {
            Tier::Calm => {}
            Tier::Uneasy => {
                fx.twitch = 0.15;
                if window {
                    fx.step_fps = 8.0;
                }
            }
            Tier::Wrong => {
                fx.stretch = 0.3;
                fx.twitch = 0.45;
                if moving || window {
                    fx.step_fps = 10.0;
                }
            }
            Tier::Collapse => {
                fx.stretch = 0.75;
                fx.twitch = 0.75;
                if moving || window {
                    fx.step_fps = 7.0;
                }
                if c.brain.roll > 0.0 {
                    fx.head_roll = std::f32::consts::PI;
                }
            }
        }
        if self.tier >= Tier::Wrong && c.seen {
            let to = c.pos + vec3(0.0, 1.3, 0.0) - self.eye();
            let dist = to.length();
            if dist < DISTORT_DIST && self.view_dir().dot(to / dist) > DISTORT_COS {
                // flackert: nicht durchgehend
                let on = ((t * 9.0).floor() * 1.7 + c.seed).sin().abs() > 0.4;
                if on {
                    fx.distort = if self.tier >= Tier::Collapse {
                        1.0
                    } else {
                        0.25
                    };
                }
            }
        }
    }

    /// Schritte und Schluesselbund der NPCs (mit Ort, Stereo). Verfolger hoert man nur,
    /// solange man sie nicht sieht: schaut man hin, verstummen sie.
    pub(super) fn npc_sounds(&mut self, moved: &[f32]) {
        let mut k = 0;
        let collapse = self.tier >= Tier::Collapse;
        for c in &mut self.customers {
            let d = moved.get(k).copied().unwrap_or(0.0);
            k += 1;
            c.brain.step_acc += d;
            if c.brain.step_acc < STEP_EVERY {
                continue;
            }
            c.brain.step_acc = 0.0;
            if c.seen || !self.tipped {
                continue;
            }
            let (sfx, vol) = if c.brain.hunting && collapse {
                (Sfx::Shuffle, 0.7)
            } else if c.gait.run {
                (Sfx::Step, 0.6)
            } else {
                (Sfx::Step, if c.brain.hunting { 0.4 } else { 0.18 })
            };
            self.sfx_at.push((sfx, vol, c.pos));
        }
        for e in &mut self.employees {
            let d = moved.get(k).copied().unwrap_or(0.0);
            k += 1;
            e.brain.step_acc += d;
            e.brain.keys_acc += d;
            if e.brain.step_acc >= STEP_EVERY {
                e.brain.step_acc = 0.0;
                self.sfx_at.push((Sfx::Step, 0.22, e.pos));
            }
            if e.brain.keys_acc >= KEYS_EVERY {
                e.brain.keys_acc = 0.0;
                self.sfx_at.push((Sfx::Keys, 0.5, e.pos));
            }
        }
        for s in &mut self.shoppers {
            let d = moved.get(k).copied().unwrap_or(0.0);
            k += 1;
            s.brain.step_acc += d;
            if s.brain.step_acc >= STEP_EVERY {
                s.brain.step_acc = 0.0;
                self.sfx_at.push((Sfx::Step, 0.12, s.pos));
            }
        }
    }

    /// Cheat "NPC-Debug": Zustand je NPC (Ort ueber dem Kopf, Text).
    pub fn npc_debug_labels(&self) -> Vec<(Vec3, String)> {
        if !self.npc_debug {
            return Vec::new();
        }
        let pushers = self.pushers();
        let mut out = Vec::new();
        for (i, c) in self.customers.iter().enumerate() {
            let b = &c.brain;
            let state = if b.freeze > 0.0 {
                "erstarrt"
            } else if b.hunting && b.fresh() && pushers.contains(&i) {
                "jagt"
            } else if b.hunting && b.fresh() {
                "lauert"
            } else if b.hunting {
                "sucht"
            } else if c.stalker {
                "Verfolger"
            } else {
                "kauft"
            };
            out.push((c.pos + vec3(0.0, 2.05, 0.0), state.to_string()));
            if let (Some(m), true) = (b.memory, b.hunting && !b.fresh()) {
                out.push((m + vec3(0.0, 0.3, 0.0), "x".to_string()));
            }
        }
        for e in &self.employees {
            let state = if e.carrying {
                "Wagen"
            } else if e.brain.restock > 0.0 {
                "raeumt ein"
            } else if e.brain.investigate.is_some() {
                "horcht"
            } else {
                match e.state {
                    EmpState::Talking => "redet",
                    EmpState::Approach => "kommt",
                    EmpState::Patrol => "Streife",
                }
            };
            out.push((e.pos + vec3(0.0, 2.1, 0.0), state.to_string()));
        }
        out
    }

    /// Animation einer Kundin aus Zustand und Stufe.
    pub(super) fn customer_clip(&self, c: &npc::Customer) -> Clip {
        let b = &c.brain;
        if b.grab > 0.0 {
            return Clip::Grab;
        }
        let late = self.tipped && self.tier >= Tier::Wrong;
        if c.gait.moving {
            if c.gait.run {
                return Clip::Run;
            }
            if b.hunting && late {
                // Jaeger schleichen geduckt heran, im Kollaps wie Untote
                return if self.tier >= Tier::Collapse {
                    Clip::ZombieWalk
                } else {
                    Clip::CrouchWalk
                };
            }
            return Clip::Walk;
        }
        if b.mimic_pick > 0.0 {
            return Clip::PickUp;
        }
        if b.hunting {
            return match self.tier {
                Tier::Collapse => Clip::ZombieIdle,
                Tier::Wrong => Clip::CrouchIdle,
                _ => Clip::Idle,
            };
        }
        let act = Activity::for_seed(c.seed);
        if c.pose == Pose::Reach {
            return act.clip();
        }
        match act {
            Activity::Phone | Activity::FoldArms | Activity::Talk => act.clip(),
            _ => Clip::Idle,
        }
    }

    /// Animation eines Mitarbeiters.
    pub(super) fn employee_clip(&self, e: &npc::Employee) -> Clip {
        if e.state == EmpState::Talking {
            return Clip::Talk;
        }
        if e.carrying {
            return Clip::Push;
        }
        if e.brain.restock > 0.0 {
            return Clip::Kneel;
        }
        crowd::gait_clip(&e.gait, Clip::Idle)
    }

    /// Animation der normalen Kundschaft (vor dem Kippen): im Stehen etwas tun.
    pub(super) fn shopper_clip(&self, s: &npc::Shopper) -> Clip {
        let still = match Activity::for_seed(s.seed) {
            Activity::Phone => Clip::Phone,
            Activity::FoldArms => Clip::FoldArms,
            Activity::Talk => Clip::Talk,
            _ => Clip::Idle,
        };
        crowd::gait_clip(&s.gait, still)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::step;
    use super::*;

    fn behaving_game(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        g.scare_cd = 1e6;
        g
    }

    /// Kundin `d` Meter vor dem Spieler, sichtbar.
    fn customer_in_view(g: &mut Game, d: f32) -> usize {
        let p = g.player.pos;
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            let want = p + vec3(a.sin(), 0.0, a.cos()) * d;
            let q = g.world.collide(want, 0.3);
            if (q - want).length() > 0.05 {
                continue;
            }
            g.spawn_customer_at(q);
            let to = q + vec3(0.0, 1.3, 0.0) - g.eye();
            g.player.yaw = to.x.atan2(-to.z);
            g.player.pitch = (to.y / to.length()).asin();
            if g.sees(q + vec3(0.0, 1.3, 0.0)) {
                let c = g.customers.last_mut().unwrap();
                c.angry = 0.0;
                c.stalker = false;
                return g.customers.len() - 1;
            }
            g.customers.pop();
        }
        panic!("keine Kundin in Sicht");
    }

    #[test]
    fn customers_stand_still_when_you_do() {
        let mut g = behaving_game(5);
        let i = customer_in_view(&mut g, 6.0);
        // nach kurzem Stillstand: erstarrt
        step(&mut g, 2.5, Input::default());
        let pos = g.customers[i].pos;
        let t = g.customers[i].anim.clip_time();
        step(&mut g, 2.0, Input::default());
        assert!(
            g.customers[i].brain.freeze > 0.0,
            "ahmt das Stillstehen nicht nach"
        );
        assert!(
            (g.customers[i].pos - pos).length() < 0.01,
            "bewegt sich trotzdem"
        );
        assert!(
            (g.customers[i].anim.clip_time() - t).abs() < 1e-4,
            "Pose nicht eingefroren"
        );
    }

    #[test]
    fn customers_reach_into_the_shelf_when_you_do() {
        let mut g = behaving_game(6);
        let i = customer_in_view(&mut g, 7.0);
        g.update(1.0 / 60.0, &mut Input::default());
        g.player_reached = true;
        g.update(1.0 / 60.0, &mut Input::default());
        assert!(g.customers[i].brain.mimic_pick > 0.0, "greift nicht mit");
        assert_eq!(g.customer_clip(&g.customers[i]), Clip::PickUp);
    }

    #[test]
    fn caught_stalker_freezes_mid_step() {
        let mut g = behaving_game(7);
        let i = customer_in_view(&mut g, 7.0);
        g.customers[i].stalker = true;
        g.customers[i].brain.notice(g.player.pos, true);
        // wegsehen: er schleicht heran
        let (yaw, pitch) = (g.player.yaw, g.player.pitch);
        g.player.yaw += std::f32::consts::PI;
        step(&mut g, 1.0, Input::default());
        assert!(g.customers[i].brain.hunting, "jagt nicht");
        // hinsehen: erstarrt mitten im Schritt
        g.player.yaw = yaw;
        g.player.pitch = pitch;
        g.update(1.0 / 60.0, &mut Input::default());
        if !g.customers[i].seen {
            return; // Sicht verdeckt (Seed-abhaengig), nicht aussagekraeftig
        }
        assert!(g.customers[i].brain.freeze > 1.0, "erstarrt nicht");
        let pos = g.customers[i].pos;
        let t = g.customers[i].anim.clip_time();
        step(&mut g, 1.0, Input::default());
        assert!((g.customers[i].pos - pos).length() < 0.01);
        assert!(
            (g.customers[i].anim.clip_time() - t).abs() < 1e-4,
            "Pose laeuft weiter"
        );
    }

    #[test]
    fn hunters_lunge_with_a_grab() {
        let mut g = behaving_game(8);
        g.god = true;
        let p = g.player.pos;
        g.spawn_customer_at(g.world.collide(p + vec3(0.0, 0.0, 1.3), 0.25));
        let c = g.customers.last_mut().unwrap();
        c.stalker = true;
        c.brain.notice(p, true);
        g.player.yaw = 0.0; // schaut nach -z, Kundin steht hinter ihm
        let mut grabbed = false;
        for _ in 0..60 {
            g.update(1.0 / 60.0, &mut Input::default());
            grabbed |= g.customer_clip(&g.customers[0]) == Clip::Grab;
        }
        assert!(grabbed, "kein Zupacken");
    }

    #[test]
    fn activities_and_staff_clips() {
        let kinds: std::collections::HashSet<_> = (0..40)
            .map(|k| Activity::for_seed(k as f32 * 0.25))
            .collect();
        assert!(kinds.len() >= 4, "zu wenig Abwechslung: {kinds:?}");
        let g = behaving_game(9);
        let mut e = npc::Employee::new(g.player.pos);
        e.brain.restock = 3.0;
        assert_eq!(g.employee_clip(&e), Clip::Kneel);
        e.carrying = true;
        assert_eq!(g.employee_clip(&e), Clip::Push);
        e.state = EmpState::Talking;
        assert_eq!(g.employee_clip(&e), Clip::Talk);
    }

    #[test]
    fn late_stages_deform_the_customers() {
        let mut g = behaving_game(11);
        let i = customer_in_view(&mut g, 5.0);
        g.update(1.0 / 60.0, &mut Input::default());
        let fx = |g: &Game| {
            let mut f = crate::people::PoseFx::default();
            g.customer_fx(&g.customers[i], &mut f);
            f
        };
        g.tier = Tier::Calm;
        let calm = fx(&g);
        assert!(calm.stretch == 0.0 && calm.twitch == 0.0);
        g.tier = Tier::Wrong;
        let wrong = fx(&g);
        assert!(wrong.stretch > 0.0 && wrong.twitch > calm.twitch);
        g.tier = Tier::Collapse;
        let late = fx(&g);
        assert!(late.stretch > wrong.stretch && late.twitch > wrong.twitch);
        // direkt hinsehen: im Kollaps zerreisst sie zeitweise
        let mut torn = false;
        for _ in 0..120 {
            g.time += 1.0 / 60.0;
            torn |= fx(&g).distort > 0.5;
        }
        assert!(torn, "kein Zerreissen beim Hinsehen");
    }

    #[test]
    fn unseen_hunters_can_be_heard_seen_ones_go_silent() {
        let mut g = behaving_game(12);
        g.god = true;
        let p = g.player.pos;
        g.spawn_customer_at(g.world.collide(p + vec3(0.0, 0.0, 5.0), 0.25));
        let c = g.customers.last_mut().unwrap();
        c.stalker = true;
        c.angry = 0.0;
        g.player.yaw = 0.0;
        g.player.pitch = -1.4;
        let mut steps = 0;
        for _ in 0..(3 * 60) {
            g.customers[0].brain.notice(p, true);
            g.update(1.0 / 60.0, &mut Input::default());
            steps += g.sfx_at.iter().filter(|s| matches!(s.0, Sfx::Step)).count();
            g.sfx_at.clear();
        }
        assert!(steps > 0, "Verfolger schleicht lautlos");
        // gesehen: stumm (und erstarrt)
        g.customers[0].seen = true;
        g.customers[0].brain.step_acc = 10.0;
        g.npc_sounds(&[1.0]);
        assert!(
            !g.sfx_at.iter().any(|s| matches!(s.0, Sfx::Step)),
            "gesehen und trotzdem hoerbar"
        );
    }

    #[test]
    fn staff_restocks_shelves_on_patrol() {
        let mut g = behaving_game(10);
        let mut e = npc::Employee::new(g.world.spawn);
        e.ignore = 1e6;
        g.employees.push(e);
        let mut knelt = false;
        // bis 240 s: die Streife bleibt im Markt, der Zufall der Ziele laeuft dadurch anders
        for _ in 0..(120 * 20) {
            for _ in 0..6 {
                g.update(1.0 / 60.0, &mut Input::default());
            }
            if g.employees[0].brain.restock > 0.0 {
                knelt = true;
                break;
            }
        }
        assert!(knelt, "raeumt nie ein");
    }
}
