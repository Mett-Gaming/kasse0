//! Der eigene Koerper: In der Ich-Sicht sieht man beim Runterschauen Koerper und Beine, die
//! Haende liegen am Wagengriff (Clip `Push`) bzw. am Lenker des Scooters (`Sit`). Dieselbe
//! Figur (der Mann im Hoodie) steht auf dem Ueberwachungsmonitor und im Spiegel des
//! Fluchtkorridors, dort mit Kopf (monitor.rs). In der Ich-Sicht blendet main.rs den Kopf aus
//! (`people::hide_head`).

use super::*;
use crate::people::{Clip, PersonDraw};

/// So weit sitzt der Koerper auf dem Scooter in der Ich-Sicht hinter dem Auge (m): man schaut
/// nicht in Schultern und Brust hinein. Stehend steht er genau unter dem Auge (Kopf ist weg).
pub const BODY_BACK: f32 = 0.15;
/// Sitzflaeche des Scooters ueber dem Stuhl der Kassiererin (fuer den Sitz-Clip, m)
pub const SCOOTER_SEAT_UP: f32 = 0.12;
/// Stehend so weit hinter dem Auge (m): man schaut auf Brust und Bauch, nicht in den Kragen
pub const STAND_BACK: f32 = 0.1;
/// Geduckt so weit hinter dem Auge (m): die Figur beugt sich vor, sonst schaute man von innen
/// gegen die eigene Schulter
pub const CROUCH_BACK: f32 = 0.38;

/// Die Figur des Spielers (der Mann im Hoodie, wie bisher auf dem Monitor).
pub fn player_variant() -> usize {
    crate::people::people_of(false)[0]
}

impl Game {
    /// Gangart des eigenen Koerpers fortschreiben (jedes Bild, nach der Bewegung).
    pub(super) fn update_body(&mut self, dt: f32) {
        let female = crate::people::PEOPLE[player_variant()].1;
        let before = self.player_gait.t;
        self.player_gait.advance(self.player.pos, dt, female);
        // mit dem Wagen im Stand: Haende ruhig am Griff, nicht auf der Stelle laufen
        if self.trolley.pushing && !self.player_gait.moving {
            self.player_gait.t = before;
        }
    }

    /// Welcher Clip gerade passt.
    pub fn body_clip(&self) -> Clip {
        let p = &self.player;
        let moving = vec3(p.vel.x, 0.0, p.vel.z).length() > 0.3;
        if self.riding() {
            Clip::Sit
        } else if self.trolley.pushing {
            // Haende am Griff, auch im Stand
            Clip::Push
        } else if p.crouch > 0.5 {
            if moving {
                Clip::CrouchWalk
            } else {
                Clip::CrouchIdle
            }
        } else if moving && p.running {
            Clip::Run
        } else if moving {
            Clip::Walk
        } else {
            Clip::Idle
        }
    }

    /// Der eigene Koerper als Figur. `first_person`: fuer die eigene Kamera (etwas hinter dem
    /// Auge; der Kopf wird in main.rs ausgeblendet), sonst fuer Monitor und Spiegel.
    /// None: kein Koerper (Sterben, Run vorbei).
    pub fn player_person(&self, first_person: bool) -> Option<PersonDraw> {
        if self.dying.is_some() || self.outcome != Outcome::Running {
            return None;
        }
        let p = &self.player;
        let (s, c) = p.yaw.sin_cos();
        let forward = vec3(s, 0.0, -c);
        // Figuren schauen nach +z ihres Modells: so dreht man sie in Blickrichtung
        let yaw = npc::yaw_towards(Vec3::ZERO, forward);
        let facing = vec3(yaw.sin(), 0.0, yaw.cos());
        let clip = self.body_clip();
        let back = match (first_person, clip == Clip::Sit) {
            (true, true) => BODY_BACK,
            (true, false) if clip == Clip::CrouchIdle || clip == Clip::CrouchWalk => CROUCH_BACK,
            (true, false) => STAND_BACK,
            _ => 0.0,
        };
        let pos = if clip == Clip::Sit {
            // auf dem Sitz des Scooters (wie die Kassiererin auf ihrem Stuhl)
            let (ts, tc) = self.trolley.yaw.sin_cos();
            let seat = self.trolley.pos - vec3(ts, 0.0, -tc) * super::scooter::SEAT_BACK;
            seat + facing * crate::people::SIT_BACK
                + vec3(
                    0.0,
                    crate::people::SIT_UP + SCOOTER_SEAT_UP + self.cart_lift(),
                    0.0,
                )
                - forward * back
        } else {
            p.pos + vec3(0.0, p.jump_y, 0.0) - forward * back
        };
        let time = if clip.locomotion() {
            self.player_gait.t
        } else {
            self.time
        };
        Some(PersonDraw {
            variant: player_variant(),
            clip,
            time,
            pos,
            // im Sitz schaut man, wohin der Scooter zeigt
            yaw: if clip == Clip::Sit {
                let (ts, tc) = self.trolley.yaw.sin_cos();
                npc::yaw_towards(Vec3::ZERO, vec3(ts, 0.0, -tc))
            } else {
                yaw
            },
            tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ghost: false,
            flip: false,
            look_at: None,
            fx: Default::default(),
        })
    }

    /// Modell-Matrix der Figur: Lage und Drehung.
    pub fn body_model(&self, me: &PersonDraw) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(me.yaw), me.pos)
    }

    /// Lage des Wagens bzw. Scooters (wie er gezeichnet wird).
    fn trolley_matrix(&self) -> Mat4 {
        let t = &self.trolley;
        let base = t.pos + vec3(0.0, self.sway_at(t.pos).y + self.cart_lift(), 0.0);
        Mat4::from_rotation_translation(Quat::from_rotation_y(-t.yaw), base)
    }

    /// Wohin die Haende greifen (Welt, links / rechts): Wagenstange, Scooter-Lenker (lenkt
    /// mit), rechts was man in der Hand hat (Ware, Zigarette, Dose beim Trinken).
    pub fn hand_targets(&self) -> [Option<Vec3>; 2] {
        let mut hands = [None, None];
        if self.riding() {
            let m = self.trolley_matrix() * self.steer_local();
            for (k, s) in [-1.0f32, 1.0].into_iter().enumerate() {
                hands[k] = Some(m.transform_point3(vec3(s * GRIP_X, BAR_Y, BAR_Z)));
            }
        } else if self.trolley.pushing {
            let m = self.trolley_matrix();
            for (k, s) in [-1.0f32, 1.0].into_iter().enumerate() {
                hands[k] = Some(m.transform_point3(vec3(s * HANDLE_X, HANDLE_Y, HANDLE_Z)));
            }
        }
        // rechte Hand: was man haelt, hat Vorrang
        let held = self
            .drink_can()
            .map(|(base, up, _)| base + up * 0.05)
            .or_else(|| match self.cig {
                super::smoke::Cigarette::Lit { .. } | super::smoke::Cigarette::Puff { .. } => {
                    self.cigarette_pose().map(|(filter, _, _)| filter)
                }
                _ => None,
            })
            .or_else(|| self.held_item_pos());
        if held.is_some() {
            hands[1] = held;
        }
        hands
    }

    /// Skinning-Matrizen des eigenen Koerpers an `out` anhaengen, mit den Haenden an ihren
    /// Zielen (Zwei-Knochen-IK der Engine). Liefert die Modell-Matrix.
    pub fn player_bones(&self, me: &PersonDraw, out: &mut Vec<Mat4>) -> Mat4 {
        use mettgine::anim::{ClipId, Pose};
        let set = crate::people::anims(crate::people::PEOPLE[me.variant].1);
        let model = self.body_model(me);
        let first = out.len();
        let mut pose = Pose::default();
        pose.sample(set, me.clip.index(), me.time);
        let to_local = model.inverse();
        for (k, side) in ["L", "R"].into_iter().enumerate() {
            let Some(target) = self.hand_targets()[k] else {
                continue;
            };
            let bones = [
                set.bone(&format!("UpperArm.{side}")),
                set.bone(&format!("LowerArm.{side}")),
                set.bone(&format!("Wrist.{side}")),
            ];
            let [Some(a), Some(b), Some(c)] = bones else {
                continue;
            };
            // Handgelenk etwas vor dem Griff: die Finger liegen darum
            let t = to_local.transform_point3(target);
            let shoulder = pose.globals(set)[a].w_axis.truncate();
            let wrist = t - (t - shoulder).normalize_or_zero() * PALM;
            // Ellbogen zeigt nach aussen, unten, hinten
            let s = if side == "L" { 1.0 } else { -1.0 };
            let pole = shoulder + vec3(s * 0.6, -0.6, -0.5);
            pose.two_bone_ik(set, [a, b, c], wrist, pole, 1.0);
        }
        pose.skin(set, out);
        // mit Scooter dick: Bauch, Brust, Huefte breiter (Arme und Haende nicht)
        if self.has_scooter() {
            crate::people::fatten(me.variant, &mut out[first..], FAT);
        }
        model
    }
}

/// Mit Scooter ist man dick: so viel breiter (x und z) wird die Figur
pub const FAT: f32 = 1.4;
/// ... und zu Fuss so viel langsamer (30 % weniger; der Scooter faehrt gleich schnell)
pub const FAT_WALK: f32 = 0.7;
/// Haendchen: vom Handgelenk bis zur Mitte der Finger (m)
const PALM: f32 = 0.07;
/// Griffstange des Wagens (Modell `Cart`, gemessen): Griffe bei +-x, Hoehe, hinten (z)
const HANDLE_X: f32 = 0.15;
const HANDLE_Y: f32 = 0.81;
const HANDLE_Z: f32 = 0.49;
/// Lenker des Scooters (Modell `ScooterBar`, tools/models/scooter.py): Griffe bei +-x
const GRIP_X: f32 = 0.24;
const BAR_Y: f32 = 1.0;
const BAR_Z: f32 = 0.54;

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn game() -> Game {
        let mut g = Game::new(120);
        g.tutorial = None;
        g.customers.clear();
        g.employees.clear();
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
    fn the_body_picks_a_clip_for_what_you_do() {
        let mut g = game();
        g.trolley.pushing = false;
        g.trolley.pos = g.world.corral;
        drive(&mut g, 0.5, |_| {});
        assert_eq!(g.body_clip(), Clip::Idle);
        drive(&mut g, 0.5, |i| i.forward = true);
        assert_eq!(g.body_clip(), Clip::Walk);
        drive(&mut g, 0.5, |i| {
            i.forward = true;
            i.run = true;
        });
        assert_eq!(g.body_clip(), Clip::Run);
        drive(&mut g, 0.5, |i| i.crouch = true);
        assert_eq!(g.body_clip(), Clip::CrouchIdle);
        drive(&mut g, 0.5, |i| {
            i.crouch = true;
            i.forward = true;
        });
        assert_eq!(g.body_clip(), Clip::CrouchWalk);
        // Wagen schieben
        let mut g = game();
        assert!(g.trolley.pushing, "Start mit Wagen");
        drive(&mut g, 0.5, |i| i.forward = true);
        assert_eq!(g.body_clip(), Clip::Push);
        // Scooter: sitzen
        g.receive_item(super::super::pfand::items::Item::Scooter);
        g.trolley.pos = g.world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET);
        g.trolley.yaw = 0.0;
        g.player.pos = g.world.spawn;
        g.toggle_cart();
        assert!(g.riding());
        assert_eq!(g.body_clip(), Clip::Sit);
    }

    #[test]
    fn the_own_body_stands_just_behind_the_eye_and_on_the_floor() {
        let mut g = game();
        g.player.yaw = 0.7;
        let me = g.player_person(true).expect("kein Koerper");
        assert_eq!(me.variant, player_variant());
        let eye = g.eye();
        let fwd = g.view_dir();
        // stehend knapp hinter dem Auge (der Kopf ist ausgeblendet)
        let to_body = vec3(me.pos.x - eye.x, 0.0, me.pos.z - eye.z);
        let behind = -to_body.dot(vec3(fwd.x, 0.0, fwd.z).normalize());
        assert!(
            (behind - STAND_BACK).abs() < 1e-3,
            "Koerper verschoben: {to_body}"
        );
        assert!(me.pos.y.abs() < 1e-4, "schwebt");
        // die Figur schaut in Blickrichtung (vorne = +z des Modells)
        let model_fwd = vec3(me.yaw.sin(), 0.0, me.yaw.cos());
        assert!(model_fwd.dot(vec3(fwd.x, 0.0, fwd.z).normalize()) > 0.99);
        // im Sprung mit hoch
        g.player.jump_y = 0.4;
        assert!((g.player_person(true).unwrap().pos.y - 0.4).abs() < 1e-4);
        // Monitor und Spiegel: dieselbe Figur, am Spielerort
        g.player.jump_y = 0.0;
        let mon = g.player_person(false).unwrap();
        assert_eq!(mon.variant, me.variant);
        assert!(mon.pos.distance(g.player.pos) < 1e-4);
        // beim Sterben kein Koerper
        g.dying = Some(Default::default());
        assert!(g.player_person(true).is_none());
    }

    #[test]
    fn on_the_scooter_you_sit_on_the_seat() {
        let mut g = game();
        g.receive_item(super::super::pfand::items::Item::Scooter);
        g.trolley.pos = g.world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET);
        g.trolley.yaw = 0.0;
        g.player.pos = g.world.spawn;
        g.toggle_cart();
        let me = g.player_person(false).unwrap();
        assert_eq!(me.clip, Clip::Sit);
        // ueber dem Sitz (nicht im Boden), Blick nach vorn wie der Scooter
        assert!(me.pos.y > 0.1, "sitzt im Boden");
        let model_fwd = vec3(me.yaw.sin(), 0.0, me.yaw.cos());
        assert!(
            model_fwd.dot(vec3(0.0, 0.0, -1.0)) > 0.99,
            "falsch herum: {model_fwd}"
        );
    }

    #[test]
    fn the_head_folds_away_only_when_asked() {
        let v = player_variant();
        let a = crate::people::anims(crate::people::PEOPLE[v].1);
        let hidden: Vec<usize> = ["Head", "Neck"]
            .iter()
            .map(|n| a.bone(n).expect("Bone fehlt"))
            .collect();
        let mut bones = Vec::new();
        a.sample(Clip::Idle, 0.3, &mut bones);
        let keep = bones.clone();
        crate::people::hide_head(v, &mut bones);
        // Kopf und Hals: alles auf je einen Punkt (Groesse 0), oben in der Figur
        for &h in &hidden {
            let m = bones[h];
            let p0 = m.transform_point3(Vec3::ZERO);
            let p1 = m.transform_point3(Vec3::splat(1.0));
            assert!(p0.distance(p1) < 1e-5, "Knochen {h} nicht gefaltet");
            assert!(p0.y > 1.3 && p0.y < 2.0, "Punkt nicht oben: {p0}");
        }
        // alle anderen Knochen unveraendert
        for (i, (a, b)) in bones.iter().zip(&keep).enumerate() {
            if !hidden.contains(&i) {
                assert_eq!(a, b, "Knochen {i} veraendert");
            }
        }
    }

    /// Wo die Hand (Mitte der Handflaeche) nach der Pose liegt (Welt).
    fn palm(g: &Game, me: &PersonDraw, side: &str) -> Vec3 {
        let mut bones = Vec::new();
        let model = g.player_bones(me, &mut bones);
        let set = crate::people::anims(crate::people::PEOPLE[me.variant].1);
        let w = set.bone(&format!("Wrist.{side}")).unwrap();
        let c = crate::people::bone_centers(me.variant)[w];
        model.transform_point3(bones[w].transform_point3(c))
    }

    fn on_scooter(seed: u64) -> Game {
        let mut g = game();
        let _ = seed;
        g.receive_item(super::super::pfand::items::Item::Scooter);
        g.trolley.pos = g.world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET);
        g.trolley.yaw = 0.0;
        g.player.pos = g.world.spawn;
        g.toggle_cart();
        g
    }

    #[test]
    fn hands_hold_the_cart_handle_and_the_scooter_bar() {
        // Wagen: beide Haende an der Stange (nicht in der Luft)
        let g = game();
        assert!(g.trolley.pushing);
        let me = g.player_person(true).unwrap();
        let [l, r] = g.hand_targets();
        let (l, r) = (l.expect("links frei"), r.expect("rechts frei"));
        assert!((l.y - (HANDLE_Y + g.trolley.pos.y)).abs() < 0.05);
        for (side, t) in [("L", l), ("R", r)] {
            let p = palm(&g, &me, side);
            assert!(p.distance(t) < 0.12, "{side}: Hand {p} statt am Griff {t}");
        }
        // Scooter: Haende am Lenker, beim Lenken wandern die Griffe (und Haende) mit
        let mut g = on_scooter(1);
        let me = g.player_person(true).unwrap();
        let straight = g.hand_targets();
        for (k, side) in ["L", "R"].into_iter().enumerate() {
            let t = straight[k].unwrap();
            assert!(
                palm(&g, &me, side).distance(t) < 0.12,
                "{side}: nicht am Lenker"
            );
        }
        g.steer = 0.4;
        let turned = g.hand_targets();
        let moved = turned[1].unwrap().distance(straight[1].unwrap());
        assert!(moved > 0.05, "Griff bewegt sich beim Lenken nicht");
        let me = g.player_person(true).unwrap();
        assert!(palm(&g, &me, "R").distance(turned[1].unwrap()) < 0.12);
    }

    #[test]
    fn what_you_hold_is_in_your_right_hand() {
        let mut g = game();
        g.trolley.pushing = false;
        g.trolley.pos = g.world.corral;
        g.fill_list_cheap();
        g.held = g.cart.pop();
        let held = g.held_item_pos().unwrap();
        assert_eq!(g.hand_targets()[1], Some(held));
        let me = g.player_person(true).unwrap();
        assert!(
            palm(&g, &me, "R").distance(held) < 0.15,
            "Ware nicht in der Hand"
        );
        // trinken: die Hand fuehrt die Dose
        g.held = None;
        g.drink_tomato();
        drive(&mut g, 0.5, |_| {});
        let (base, up, _) = g.drink_can().expect("keine Dose");
        let t = g.hand_targets()[1].unwrap();
        assert!(t.distance(base + up * 0.05) < 1e-4);
    }

    #[test]
    fn with_the_scooter_you_are_fat_and_slow_on_foot() {
        let mut g = game();
        let me = g.player_person(false).unwrap();
        let set = crate::people::anims(crate::people::PEOPLE[me.variant].1);
        let belly = set.bone("Torso").or(set.bone("Chest")).unwrap();
        let arm = set.bone("LowerArm.L").unwrap();
        let mut thin = Vec::new();
        g.player_bones(&me, &mut thin);
        g.receive_item(super::super::pfand::items::Item::Scooter);
        g.trolley.pushing = false;
        let mut fat = Vec::new();
        g.player_bones(&me, &mut fat);
        let width = |m: Mat4| m.transform_vector3(Vec3::X).length();
        assert!(width(fat[belly]) > width(thin[belly]) * 1.3, "nicht dicker");
        assert!(
            (width(fat[arm]) - width(thin[arm])).abs() < 1e-4,
            "Arme mit dick"
        );
        // zu Fuss 30 % langsamer (der Scooter selbst bleibt schnell, scooter.rs)
        let speed = |g: &mut Game| {
            g.trolley.pushing = false;
            g.trolley.pos = g.world.corral;
            g.player.pos = g.world.spawn;
            g.player.yaw = 0.0;
            drive(g, 1.0, |i| i.forward = true);
            g.player.vel.length()
        };
        let mut slim = game();
        let (normal, slow) = (speed(&mut slim), speed(&mut g));
        assert!((slow / normal - FAT_WALK).abs() < 0.05, "{slow} / {normal}");
    }
}
