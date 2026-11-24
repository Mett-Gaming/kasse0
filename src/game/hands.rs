//! Alles, was der Spieler mit den Haenden tut: Wagen schieben/abstellen, Waren greifen,
//! in den Wagen legen, fallen lassen, wieder herausnehmen, an der Kasse scannen.

use super::*;

pub(super) const CART_OFFSET: f32 = 0.85;
/// Hoehe des Korbbodens im Wagenmodell (Gitterboden aus assets.rs)
pub(super) const CART_FLOOR: f32 = crate::assets::CART_BASKET_FLOOR;
/// Abstand der Waren zur Gitterwand (Meter)
const BASKET_GAP: f32 = 0.006;
/// Korbboden laengs (Wagenmodell): von z = -0,5 bis +0,24. Plaetze: 3 quer, 4 laengs
pub(super) const CART_CELLS: usize = 12;
const CART_ROW0: f32 = -0.4;
const CART_ROW: f32 = 0.175;
/// So weit sinkt Ware in den Drahtboden ein (sie soll aufliegen, nicht schweben)
const CART_SINK: f32 = 0.01;
/// Kollisionsradius des geschobenen Wagens (breiter als ein Durchgang)
pub(super) const CART_PUSH_RADIUS: f32 = 0.42;
/// Radius des abgestellten Wagens fuer die Spielerkollision
const CART_RADIUS: f32 = 0.5;
/// Wie nah der Wagen fuer Q/F sein muss
const CART_REACH: f32 = 1.9;
/// Quietschen: zufaelliger Abstand in Metern (normal bzw. beim Rennen)
const SQUEAK_EVERY: (f32, f32) = (3.0, 5.5);
const SQUEAK_EVERY_RUN: (f32, f32) = (1.6, 3.0);
/// Ab so viel Drehung (Bogenmass) seit dem letzten Geraeusch klappert der Wagen statt zu quietschen
const RATTLE_TURN: f32 = 0.9;
/// Springen: Absprungtempo (m/s) und Schwerkraft - gut 0,6 m hoch, 0,7 s in der Luft
pub const JUMP_SPEED: f32 = 3.4;
pub const GRAVITY: f32 = 9.8;
/// Sprung mit geschobenem Wagen: nur knapp 0,3 m hoch (der Wagen hopst mit)
pub const CART_JUMP_SPEED: f32 = 2.4;
/// Beim Aufsetzen mit dem Wagen: so oft faellt nichts heraus bzw. genau eine Ware,
/// sonst mehrere (2 bis alle)
pub const SPILL_NONE: f32 = 0.25;
pub const SPILL_ONE: f32 = 0.4;

/// Wie viele von `n` Waren beim Aufsetzen herausfallen (`roll` zufaellig in 0..1).
pub fn spill_count(n: usize, roll: f32, rng: &mut impl Rng) -> usize {
    if n == 0 || roll < SPILL_NONE {
        0
    } else if roll < SPILL_NONE + SPILL_ONE || n == 1 {
        1
    } else {
        rng.gen_range(2..=n)
    }
}
/// Geduckt: Anteil des Renntempos
pub const CROUCH_SPEED: f32 = 0.55;

impl Game {
    pub(super) fn update_movement(&mut self, dt: f32, input: &Input) {
        let (s, c) = self.player.yaw.sin_cos();
        let forward = vec3(s, 0.0, -c);
        let right = vec3(c, 0.0, s);
        let mut wish = Vec3::ZERO;
        if input.forward {
            wish += forward;
        }
        if input.back {
            wish -= forward;
        }
        if input.right {
            wish += right;
        }
        if input.left {
            wish -= right;
        }
        // Stick: jede Richtung, halb gedrueckt langsamer
        let (sx, sy) = input.stick;
        let push = sx.hypot(sy).min(1.0);
        if push > 0.0 {
            wish = forward * sy + right * sx;
        }
        // im Fluchtkorridor: Ausdauer unendlich; dort auch Tempo-Grenzen (Stolpern, Engstellen)
        let endless = self.corridor_endless_stamina();
        let (slow, cap) = self.corridor_speed();
        let pushing = self.trolley.pushing;
        // Ducken (weich) und Springen (nur am Boden, nicht geduckt)
        let p = &mut self.player;
        let want = if input.crouch && p.jump_y <= 0.0 {
            1.0
        } else {
            0.0
        };
        p.crouch += (want - p.crouch) * (dt * 10.0).min(1.0);
        if input.jump && p.jump_y <= 0.0 && p.jump_v <= 0.0 && p.crouch < 0.3 {
            // mit dem Wagen springt man nur ein kleines Stueck (der Wagen hopst mit)
            p.jump_v = if pushing { CART_JUMP_SPEED } else { JUMP_SPEED };
            p.cart_jump = pushing;
        }
        let mut landed = false;
        if p.jump_y > 0.0 || p.jump_v > 0.0 {
            p.jump_y += p.jump_v * dt;
            p.jump_v -= GRAVITY * dt;
            if p.jump_y <= 0.0 {
                p.jump_y = 0.0;
                p.jump_v = 0.0;
                landed = true;
            }
        }
        if endless {
            p.stamina = 1.0;
            p.exhausted = false;
        }
        // Ausdauer: Rennen kostet, Gehen und Stehen erholen (geduckt rennt man nicht)
        p.running = input.run && !p.exhausted && wish.length_squared() > 0.0 && p.crouch < 0.5;
        if p.running && !endless {
            p.stamina -= dt / STAMINA_RUN;
            if p.stamina <= 0.0 {
                p.stamina = 0.0;
                p.exhausted = true;
            }
        } else {
            p.stamina = (p.stamina + dt / STAMINA_REGEN).min(1.0);
            if p.exhausted && p.stamina > 0.35 {
                p.exhausted = false;
            }
        }
        let running = p.running;
        // geduckt: immer gut die Haelfte des Renntempos (geduckt rennen geht nicht)
        let crouch = p.crouch;
        if wish.length_squared() > 0.0 {
            let base = if running { RUN_SPEED } else { WALK_SPEED };
            let v = (base + (RUN_SPEED * CROUCH_SPEED - base) * crouch) * slow;
            let push = if push > 0.0 { push.max(0.25) } else { 1.0 };
            wish = wish.normalize() * v.min(cap) * push;
        }
        if landed {
            self.play(Sfx::Step, 0.6);
            // mit dem Wagen gelandet: es scheppert, Ware fliegt heraus
            if std::mem::take(&mut self.player.cart_jump) && self.trolley.pushing {
                self.spill_cart();
            }
        }
        let old = self.player.pos;
        self.player.vel = self.player.vel.lerp(wish, (dt * 10.0).min(1.0));
        let mut pos = self
            .world
            .collide(old + self.player.vel * dt, PLAYER_RADIUS);
        // Huerden, Tueren, Regale ... im Fluchtkorridor
        pos = self.corridor_collide(pos, PLAYER_RADIUS);

        if self.trolley.pushing {
            // Der geschobene Wagen ist breiter als ein Mensch: schmale Durchgaenge gehen nicht
            let cart = pos + forward * CART_OFFSET;
            let fixed = self.world.collide(cart, CART_PUSH_RADIUS);
            pos = self.world.collide(pos + (fixed - cart), PLAYER_RADIUS);
            let want = pos + forward * CART_OFFSET;
            let t = &mut self.trolley;
            t.pos = t.pos.lerp(want, (dt * 12.0).min(1.0));
            let old_yaw = t.yaw;
            t.yaw = lerp_angle(t.yaw, self.player.yaw, (dt * 12.0).min(1.0));
            let turned = lerp_angle(0.0, t.yaw - old_yaw, 1.0).abs();

            // Feststecken erkennen
            let moved = (pos - old).length();
            if wish.length_squared() > 0.0 && moved < WALK_SPEED * dt * 0.25 {
                self.cart_stuck += dt;
                if self.cart_stuck > 0.5 {
                    self.hint("Der Wagen passt hier nicht durch. [Q] stehen lassen.");
                }
            } else {
                self.cart_stuck = 0.0;
            }
            // Das Rad quietscht - je nach Bewegung anders; Mitarbeiter hoeren das
            self.squeak_acc += moved;
            self.cart_turn += turned;
            if self.squeak_acc > self.squeak_next {
                let (sfx, vol, radius) = if running {
                    (Sfx::Squeak, 0.5, 18.0)
                } else if self.cart_turn > RATTLE_TURN {
                    (Sfx::Rattle, 0.4, 10.0)
                } else {
                    (Sfx::SqueakSoft, 0.3, 8.0)
                };
                let (lo, hi) = if running {
                    SQUEAK_EVERY_RUN
                } else {
                    SQUEAK_EVERY
                };
                self.squeak_acc = 0.0;
                self.cart_turn = 0.0;
                self.squeak_next = self.rng.gen_range(lo..hi);
                self.sfx.push((sfx, vol));
                if self.tipped {
                    self.make_noise(radius);
                    if !self.squeak_hinted {
                        self.squeak_hinted = true;
                        self.say("Das Rad quietscht. Die Mitarbeiter hören das.", 4.0);
                    }
                }
            }
        } else {
            // Abgestellter Wagen ist ein Hindernis
            let d = vec3(pos.x - self.trolley.pos.x, 0.0, pos.z - self.trolley.pos.z);
            let min = CART_RADIUS + PLAYER_RADIUS;
            if d.length() < min && d.length() > 1e-4 {
                pos = self
                    .world
                    .collide(self.trolley.pos + d.normalize() * min, PLAYER_RADIUS);
            }
        }

        // Kassenbereich erst mit vollstaendigem Zettel
        pos = self.keep_out_of_checkout(old, pos);

        let moved = (pos - old).length();
        let p = &mut self.player;
        p.pos = pos;
        p.bob_phase += moved * 5.5;
        p.step_acc += moved;
        if p.step_acc > 0.7 {
            p.step_acc = 0.0;
            self.sfx.push((Sfx::Step, if running { 0.5 } else { 0.25 }));
            // Schritte hoert man: Rennen weit, Gehen kaum
            let r = if running {
                super::senses::NOISE_RUN
            } else {
                super::senses::NOISE_WALK
            };
            self.noise(pos, r);
        }
        if let Some(t) = &mut self.tutorial {
            t.walked += moved;
        }
    }

    fn cart_box(&self) -> world::Aabb {
        world::Aabb::from_center(
            self.trolley.pos + vec3(0.0, 0.45, 0.0),
            vec3(0.45, 0.45, 0.45),
        )
    }

    /// So hoch ist der Wagen gerade (er hopst beim Sprung mit, wenn man ihn schiebt).
    pub fn cart_lift(&self) -> f32 {
        if self.trolley.pushing {
            self.player.jump_y
        } else {
            0.0
        }
    }

    /// Mit dem Wagen gelandet: zufaellig fliegt mal nichts, mal eine, mal mehrere Waren heraus
    /// und liegt dann am Boden (mit E wieder aufheben).
    pub(super) fn spill_cart(&mut self) {
        self.play(Sfx::Rattle, 1.0);
        self.play(Sfx::Thud, 0.6);
        self.make_noise(super::senses::NOISE_RUN);
        let n = spill_count(self.cart.len(), self.rng.gen_range(0.0..1.0), &mut self.rng);
        if n == 0 {
            return;
        }
        let mut idx: Vec<usize> = (0..self.cart.len()).collect();
        idx.shuffle(&mut self.rng);
        let mut out: Vec<usize> = idx[..n].to_vec();
        // von hinten entfernen, damit die Indizes stimmen
        out.sort_unstable_by(|a, b| b.cmp(a));
        let (s, c) = self.trolley.yaw.sin_cos();
        let fwd = vec3(s, 0.0, -c);
        let side = vec3(c, 0.0, s);
        for i in out {
            let from = self.cart_item_model(i).w_axis.truncate() + vec3(0.0, 0.1, 0.0);
            let data = self.cart.remove(i);
            let push = fwd * self.rng.gen_range(0.6..1.6) + side * self.rng.gen_range(-1.2..1.2);
            self.loose.push(Loose {
                data,
                pos: from,
                vel: push + vec3(0.0, self.rng.gen_range(1.2..2.2), 0.0),
                yaw: self.rng.gen_range(0.0..std::f32::consts::TAU),
            });
        }
        self.cart_sel = None;
        self.hint("Es scheppert: Ware ist aus dem Wagen gefallen.");
    }

    /// Lage der i-ten Ware im Wagenkorb (Weltkoordinaten).
    pub(super) fn cart_item_model(&self, i: usize) -> Mat4 {
        let base = self.trolley.pos
            + vec3(
                0.0,
                self.sway_at(self.trolley.pos).y + self.cart_lift(),
                0.0,
            );
        let m = Mat4::from_rotation_translation(Quat::from_rotation_y(-self.trolley.yaw), base);
        m * self.cart_item_local(i)
    }

    /// Lage der i-ten Ware im Korb (Koordinaten des Wagenmodells): Sie liegt wie
    /// hineingelegt auf dem Korbboden oder auf der Ware darunter - hohe Waren (Flaschen,
    /// Kartons) umgekippt, alles leicht schief und verdreht.
    pub(super) fn cart_item_local(&self, i: usize) -> Mat4 {
        // bisherige Hoehe der Stapel je Platz (3 quer x 4 laengs)
        let mut tops = [CART_FLOOR; CART_CELLS];
        let mut out = Mat4::IDENTITY;
        for (k, item) in self.cart.iter().enumerate().take(i + 1) {
            let h = item.half();
            // hohe, schmale Ware liegt auf der Seite (laengs in den Korb gekippt)
            let lying = h.y > h.x.max(h.z) * 1.15;
            let height = if lying { 2.0 * h.z } else { 2.0 * h.y };
            let cell = k % CART_CELLS;
            let (col, row) = ((cell % 3) as f32 - 1.0, (cell / 3) as f32);
            // kleine, feste Unordnung je Ware
            let j = |s: f32| ((k as f32 * 12.9898 + s * 78.233).sin() * 43_758.547).fract();
            let x = col * 0.135 + (j(1.0) - 0.5) * 0.03;
            let z = CART_ROW0 + row * CART_ROW + (j(2.0) - 0.5) * 0.03;
            // Grundflaeche (beliebig gedreht: Umkreis) bleibt innerhalb der Korbwaende
            let foot = if lying {
                h.x.hypot(h.y)
            } else {
                h.x.hypot(h.z)
            } + BASKET_GAP;
            let xr = (crate::assets::CART_BASKET_X - foot).max(0.0);
            let x = x.clamp(-xr, xr);
            let (z0, z1) = (
                crate::assets::CART_BASKET_Z0 + foot,
                crate::assets::CART_BASKET_Z1 - foot,
            );
            let z = if z0 < z1 {
                z.clamp(z0, z1)
            } else {
                (z0 + z1) * 0.5
            };
            let y = tops[cell] - CART_SINK;
            // naechste Ware liegt obenauf (etwas ineinander gerutscht)
            tops[cell] += height * 0.9;
            if k == i {
                let spin = Quat::from_rotation_y(k as f32 * 1.7);
                let tilt = Quat::from_rotation_x((j(3.0) - 0.5) * 0.12)
                    * Quat::from_rotation_z((j(4.0) - 0.5) * 0.12);
                out = if lying {
                    // um die Querachse gekippt: liegt mit der Seite auf, Mitte ueber dem Platz
                    let tip = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                    let rot = spin * tilt * tip;
                    let shift = spin * tilt * vec3(0.0, h.z, -h.y);
                    Mat4::from_rotation_translation(rot, vec3(x, y, z) + shift)
                } else {
                    Mat4::from_rotation_translation(spin * tilt, vec3(x, y, z))
                };
            }
        }
        out
    }

    /// Ware, die F herausnehmen wuerde (nur wenn man den Wagen anschaut und die Hand frei ist).
    pub fn cart_selected(&self) -> Option<usize> {
        if !matches!(self.focus, Some(Focus::Cart | Focus::Belt))
            || self.held.is_some()
            || self.cart.is_empty()
        {
            return None;
        }
        Some(
            self.cart_sel
                .filter(|&i| i < self.cart.len())
                .unwrap_or(self.cart.len() - 1),
        )
    }

    /// Mausrad am Wagen: andere Ware anwaehlen.
    pub(super) fn cycle_cart_selection(&mut self, steps: i32) {
        let n = self.cart.len() as i32;
        if n == 0 || steps == 0 {
            return;
        }
        let cur = self.cart_selected().unwrap_or(n as usize - 1) as i32;
        self.cart_sel = Some((cur + steps).rem_euclid(n) as usize);
    }

    pub fn cart_distance(&self) -> f32 {
        vec3(
            self.trolley.pos.x - self.player.pos.x,
            0.0,
            self.trolley.pos.z - self.player.pos.z,
        )
        .length()
    }

    pub(super) fn update_focus(&mut self) {
        self.focus = None;
        self.aim_cart = false;
        self.at_register = false;
        if self.dialog.is_some() {
            return;
        }
        let eye = self.eye();
        let dir = self.view_dir();
        let mut best = REACH;

        // Tueren im Fluchtkorridor (auch mit etwas in der Hand)
        for (i, b) in self.corridor_doors() {
            if let Some(t) = b.ray(eye, dir, best) {
                best = t;
                self.focus = Some(Focus::Door(i));
            }
        }

        // Der Wagen selbst
        if let Some(t) = self.cart_box().ray(eye, dir, REACH + 0.4) {
            best = t;
            self.focus = Some(Focus::Cart);
            self.aim_cart = true;
        }
        // Gezielt eine Ware im Wagen anvisieren (zum Herausnehmen)
        if self.held.is_none() && self.cart_distance() < CART_REACH + 0.5 {
            let mut hit: Option<(usize, f32)> = None;
            for i in 0..self.cart.len() {
                let half = self.cart[i].half().max(Vec3::splat(0.05));
                let c = self
                    .cart_item_model(i)
                    .transform_point3(vec3(0.0, half.y, 0.0));
                if let Some(t) = world::Aabb::from_center(c, half).ray(eye, dir, REACH + 0.6) {
                    if hit.is_none_or(|(_, b)| t < b) {
                        hit = Some((i, t));
                    }
                }
            }
            // Zielen waehlt nur beim Wechsel auf eine andere Ware; dazwischen gilt das Mausrad
            let aimed = hit.map(|(i, _)| i);
            if aimed.is_some() && aimed != self.cart_aim {
                self.cart_sel = aimed;
            }
            self.cart_aim = aimed;
            if let Some((_, t)) = hit {
                best = best.min(t);
                self.focus = Some(Focus::Cart);
                self.aim_cart = true;
            }
        }
        // Beim Schieben reicht ein Blick nach unten in den Korb
        if self.trolley.pushing && self.player.pitch < -0.35 {
            self.aim_cart = true;
        }
        // Mitarbeiter, der gerade den eigenen Wagen wegschiebt: ansprechbar
        for (i, e) in self.employees.iter().enumerate() {
            if !e.carrying || e.state == EmpState::Talking {
                continue;
            }
            let bx = world::Aabb::from_center(e.pos + vec3(0.0, 0.9, 0.0), vec3(0.4, 0.9, 0.4));
            if let Some(t) = bx.ray(eye, dir, best.max(REACH + 0.5)) {
                if t < best + 0.5 {
                    best = t;
                    self.focus = Some(Focus::Employee(i));
                    self.aim_cart = false;
                }
            }
        }
        if self.held.is_none() {
            if let Some((i, t)) = self.world.item_ray(eye, dir, best) {
                if t < best {
                    best = t;
                    self.focus = Some(Focus::Shelf(i));
                }
            }
            for (i, p) in self.pickups.iter().enumerate() {
                if p.taken {
                    continue;
                }
                let half = p.data.half() + Vec3::splat(0.05);
                let bx = world::Aabb::from_center(p.pos + vec3(0.0, half.y - 0.05, 0.0), half);
                if let Some(t) = bx.ray(eye, dir, best) {
                    if t < best {
                        best = t;
                        self.focus = Some(Focus::Pickup(i));
                    }
                }
            }
            for (i, l) in self.loose.iter().enumerate() {
                let half = l.data.half() + Vec3::splat(0.08);
                let bx = world::Aabb::from_center(l.pos + vec3(0.0, half.y - 0.08, 0.0), half);
                if let Some(t) = bx.ray(eye, dir, best) {
                    if t < best {
                        best = t;
                        self.focus = Some(Focus::Loose(i));
                    }
                }
            }
        }
        if let Some(lane) = self.open_lane() {
            if (lane.scan_point - self.player.pos).length() < 1.3 {
                self.at_register = true;
            }
        }
        // Kassenband und Kartenterminal der offenen Kasse
        if !self.paid {
            if let Some(t) = self
                .terminal_box()
                .and_then(|b| b.ray(eye, dir, best + 0.3))
            {
                if t < best + 0.3 {
                    best = t;
                    self.focus = Some(Focus::Terminal);
                    self.aim_cart = false;
                }
            }
            if let Some(t) = self.belt_box().and_then(|b| b.ray(eye, dir, best)) {
                if t < best {
                    self.focus = Some(Focus::Belt);
                    self.aim_cart = false;
                }
            }
        }
    }

    pub fn open_lane(&self) -> Option<world::Lane> {
        let lanes = &self.world.lanes;
        if lanes.is_empty() {
            return None;
        }
        Some(lanes[self.params.open_lane % lanes.len()])
    }

    /// E / Linksklick
    pub(super) fn interact(&mut self) {
        if let Some(Focus::Employee(i)) = self.focus {
            // "Hey, das ist mein Wagen!"
            self.open_dialog(i, 0, true);
            return;
        }
        match self.focus {
            // E am Band oeffnet das Kassenfenster (belt_window.rs)
            Some(Focus::Belt) => return self.open_belt_window(),
            Some(Focus::Terminal) => return self.use_terminal(),
            // E an einer Tuer im Fluchtkorridor: aufdruecken (game/corridor/obstacles.rs)
            Some(Focus::Door(i)) => return self.push_door(i),
            _ => {}
        }
        if self.held.is_some() {
            if self.aim_cart {
                self.place_in_cart();
            } else {
                self.drop_held();
            }
            return;
        }
        match self.focus {
            Some(Focus::Shelf(i)) => self.grab_shelf(i),
            Some(Focus::Pickup(i)) => self.grab_pickup(i),
            Some(Focus::Loose(i)) => {
                let l = self.loose.remove(i);
                self.held = Some(l.data);
                self.play(Sfx::Pickup, 0.4);
            }
            Some(Focus::Cart) => self.toggle_cart(),
            _ => {}
        }
    }

    /// Q: Wagen stehen lassen / wieder schieben
    pub(super) fn toggle_cart(&mut self) {
        if self.employees.iter().any(|e| e.carrying) && !self.trolley.pushing {
            self.hint("Ein Mitarbeiter schiebt ihn gerade weg. Sprich ihn an [E].");
            return;
        }
        if self.trolley.pushing {
            self.trolley.pushing = false;
            self.play(Sfx::Click, 0.4);
        } else if self.cart_distance() < CART_REACH {
            self.trolley.pushing = true;
            self.play(Sfx::Click, 0.4);
        } else {
            self.hint("Dein Wagen ist zu weit weg.");
            return;
        }
        if let Some(t) = &mut self.tutorial {
            t.toggles += 1;
        }
    }

    /// Lautes Geraeusch an der Spielerposition: wer es hoert, kommt nachsehen (senses.rs).
    pub(super) fn make_noise(&mut self, radius: f32) {
        let p = self.player.pos;
        self.noise(p, radius);
    }

    /// Mitarbeiter schnappt sich den herrenlosen Wagen und bringt ihn zur Wagenbox.
    pub(super) fn start_carrying(&mut self, i: usize) {
        let corral = self.world.corral;
        let field = npc::bfs(&self.world.nav, corral);
        let e = &mut self.employees[i];
        e.carrying = true;
        e.target = corral;
        e.field = field;
        e.state = EmpState::Patrol;
        self.trolley.pushing = false;
        if self.sees(self.trolley.pos + vec3(0.0, 0.5, 0.0)) {
            self.say("Ein Mitarbeiter schiebt deinen Wagen weg!", 3.5);
        }
        self.sfx.push((Sfx::Squeak, 0.5));
    }

    pub(super) fn carry_step(&mut self, i: usize, dt: f32) {
        let corral = self.world.corral;
        let e = &mut self.employees[i];
        let old = e.pos;
        e.pos = npc::step_along(&self.world, &e.field, e.pos, corral, 0.8, dt);
        e.moving = (e.pos - old).length_squared() > 1e-8;
        if e.moving {
            e.yaw = lerp_angle(e.yaw, npc::yaw_towards(old, e.pos), (dt * 6.0).min(1.0));
        }
        self.trolley.pos = self.world.collide(e.cart_front(), 0.3);
        self.trolley.yaw = std::f32::consts::PI - e.yaw;
        // Quietschen, leiser je weiter weg
        self.squeak_acc += (e.pos - old).length();
        if self.squeak_acc > self.squeak_next {
            self.squeak_acc = 0.0;
            self.squeak_next = self.rng.gen_range(SQUEAK_EVERY.0..SQUEAK_EVERY.1);
            let d = (e.pos - self.player.pos).length();
            let vol = (1.0 - d / 25.0).max(0.0) * 0.35;
            if vol > 0.02 {
                self.sfx.push((Sfx::SqueakSoft, vol));
            }
        }
        let arrived = (e.pos - corral).length() < 1.3 || (self.trolley.pos - corral).length() < 0.8;
        if arrived {
            let e = &mut self.employees[i];
            e.carrying = false;
            e.cart_cd = 40.0;
            e.retarget = 0.0;
            self.trolley.pos = corral;
            self.trolley.yaw = 0.0;
            self.announcement = Some((
                "Ein herrenloser Einkaufswagen wurde zur Wagenbox am Eingang gebracht.".into(),
                6.0,
            ));
            self.play(Sfx::Chime, 0.6);
        }
    }

    /// F: letzte Ware wieder aus dem Wagen nehmen
    pub(super) fn take_out(&mut self) {
        if self.held.is_some() {
            self.hint("Du hast schon etwas in der Hand.");
            return;
        }
        if self.cart_distance() > CART_REACH {
            self.hint("Dein Wagen ist zu weit weg.");
            return;
        }
        if self.cart.is_empty() {
            return;
        }
        // gezielt angewaehlte Ware, sonst die zuletzt eingelegte
        let idx = self
            .cart_sel
            .filter(|&i| i < self.cart.len())
            .unwrap_or(self.cart.len() - 1);
        self.cart_sel = None;
        {
            let item = self.cart.remove(idx);
            self.held = Some(item);
            self.play(Sfx::Pickup, 0.4);
            if let Some(t) = &mut self.tutorial {
                t.took_out = true;
            }
        }
    }

    /// Ware aus dem Regal nehmen: sie verschwindet aus der statischen Welt.
    pub(super) fn grab_shelf(&mut self, i: usize) {
        let it = &mut self.world.items[i];
        if it.taken {
            return;
        }
        self.player_reached = true;
        it.taken = true;
        let item_pos = it.pos;
        let data = ItemData::from_shelf(it);
        self.maybe_shelf_face(item_pos);
        self.statics_dirty = true;
        self.play(Sfx::Pickup, 0.5);
        self.held = Some(data);
    }

    fn grab_pickup(&mut self, i: usize) {
        let p = &mut self.pickups[i];
        p.taken = true;
        let data = p.data.clone();
        if let PickupKind::Tomato { customer } = data.kind {
            self.suspicion += 0.35;
            self.glitch = self.glitch.max(0.7);
            self.red = self.red.max(0.4);
            if let Some(c) = self.customers.get_mut(customer) {
                c.angry = 1.0;
                c.stalker = true;
            }
            self.say("Das war IHR Tomatensaft. Sie hat es gesehen.", 3.5);
            self.play(Sfx::Sting, 0.3);
            // abgelaufener Tomatensaft landet nicht in der Hand: er wird sofort getrunken
            // (trip.rs: Dose zum Mund, Rausch, Kotzen)
            self.drink_tomato();
            return;
        }
        self.play(Sfx::Pickup, 0.5);
        self.held = Some(data);
    }

    fn place_in_cart(&mut self) {
        let Some(item) = self.held.take() else { return };
        match item.kind {
            PickupKind::Product { fake: false, .. } => {
                if item.list_entry().is_some_and(|id| self.list_done(id)) {
                    self.hint("Das hast du schon.");
                }
            }
            PickupKind::Product { fake: true, .. } => {
                self.suspicion += 0.25;
                self.glitch = self.glitch.max(0.5);
                self.say("Das stand so nicht auf dem Zettel...", 3.0);
            }
            PickupKind::Tomato { .. } => {}
        }
        self.cart.push(item);
        self.play(Sfx::Pickup, 0.35);
        self.check_tier();
    }

    fn drop_held(&mut self) {
        let Some(data) = self.held.take() else { return };
        let fwd = self.view_dir();
        let flat = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let pos = self.eye() + fwd * 0.45 - vec3(0.0, 0.25, 0.0);
        self.loose.push(Loose {
            data,
            pos,
            vel: flat * 1.2 + vec3(0.0, 0.6, 0.0),
            yaw: self.player.yaw,
        });
        self.play(Sfx::Click, 0.3);
    }

    /// Fuer Tests/Debug: Regalware direkt in den Wagen.
    pub fn take(&mut self, i: usize) {
        self.held = None;
        self.grab_shelf(i);
        self.place_in_cart();
    }

    /// Fuer Tests/Debug: Sonderware (Tomatensaft) direkt in den Wagen.
    pub fn take_pickup(&mut self, i: usize) {
        self.held = None;
        self.grab_pickup(i);
        self.place_in_cart();
    }

    /// Fallen gelassene Waren: Schwerkraft, landen auf Boden oder Hindernissen.
    pub(super) fn update_loose(&mut self, dt: f32) {
        let mut landed = Vec::new();
        for l in &mut self.loose {
            if l.vel == Vec3::ZERO {
                continue;
            }
            let fast = l.vel.length() > 1.5;
            let prev = l.pos;
            l.vel.y -= 9.81 * dt;
            l.pos += l.vel * dt;
            for s in &self.world.solids {
                let inside = l.pos.x > s.min.x
                    && l.pos.x < s.max.x
                    && l.pos.z > s.min.z
                    && l.pos.z < s.max.z;
                if inside && prev.y >= s.max.y - 0.01 && l.pos.y < s.max.y {
                    l.pos.y = s.max.y;
                    l.vel = Vec3::ZERO;
                }
            }
            if l.pos.y < 1.5 {
                let flat = vec3(l.pos.x, 0.0, l.pos.z);
                let fixed = self.world.collide(flat, 0.06);
                if (fixed - flat).length_squared() > 1e-8 {
                    l.pos.x = fixed.x;
                    l.pos.z = fixed.z;
                    l.vel.x = 0.0;
                    l.vel.z = 0.0;
                }
            }
            if l.pos.y <= 0.0 {
                l.pos.y = 0.0;
                l.vel = Vec3::ZERO;
            }
            // laut aufgeschlagen: das hoert man
            if l.vel == Vec3::ZERO && fast {
                landed.push(l.pos);
            }
        }
        for p in landed {
            self.noise(p, super::senses::NOISE_DROP);
        }
    }

    /// Wagen, Wageninhalt, Hand und fallen gelassene Waren zeichnen.
    pub(super) fn collect_hand_draws(&self, out: &mut Vec<MeshDraw>) {
        let white = Vec4::new(1.0, 1.0, 1.0, 0.0);
        let rot = |yaw: f32| Quat::from_rotation_y(yaw);

        // Wagenbox: zwei abgestellte Wagen
        for dx in [0.75f32, 1.35] {
            let p = self.world.corral + vec3(dx, 0.0, 0.0);
            out.push(MeshDraw {
                mesh: MeshId::Cart as usize,
                model: Mat4::from_translation(p),
                tint: Vec4::new(0.85, 0.85, 0.85, 0.0),
                ..Default::default()
            });
        }

        // Wagen + Inhalt
        let cart_pos = self.trolley.pos
            + vec3(
                0.0,
                self.sway_at(self.trolley.pos).y + self.cart_lift(),
                0.0,
            );
        let m = Mat4::from_rotation_translation(rot(-self.trolley.yaw), cart_pos);
        out.push(MeshDraw {
            mesh: MeshId::Cart as usize,
            model: m,
            tint: white,
            ..Default::default()
        });
        let selected = self.cart_selected();
        for (i, item) in self.cart.iter().enumerate() {
            let mut model = self.cart_item_model(i);
            let mut tint = item.tint.extend(0.0);
            if selected == Some(i) {
                // angewaehlte Ware: leicht angehoben und hell
                model = Mat4::from_translation(vec3(0.0, 0.03, 0.0)) * model;
                tint = (item.tint * 1.15).extend(0.45);
            }
            out.push(MeshDraw {
                mesh: item.mesh as usize,
                model,
                tint,
                ..Default::default()
            });
        }

        // Ware in der Hand (vor der Kamera, unten rechts)
        if let Some(item) = &self.held {
            let fwd = self.view_dir();
            let right = vec3(self.player.yaw.cos(), 0.0, self.player.yaw.sin());
            let up = right.cross(fwd);
            let h = item.half();
            let pos = self.eye() + fwd * 0.5 + right * 0.2 - up * (0.2 + h.y);
            let model = Mat4::from_rotation_translation(
                rot(-self.player.yaw) * Quat::from_rotation_x(self.player.pitch * 0.5),
                pos,
            );
            out.push(MeshDraw {
                mesh: item.mesh as usize,
                model,
                tint: item.tint.extend(0.05),
                ..Default::default()
            });
        }

        for l in &self.loose {
            out.push(MeshDraw {
                mesh: l.data.mesh as usize,
                model: Mat4::from_rotation_translation(
                    rot(-l.yaw),
                    l.pos + vec3(0.0, self.sway_at(l.pos).y, 0.0),
                ),
                tint: l.data.tint.extend(0.0),
                ..Default::default()
            });
        }

        // Tomatensaft der Kunden (Regalware ist statisch, siehe World::item_draws)
        for p in self.pickups.iter().filter(|p| !p.taken) {
            for k in 0..3 {
                let off = vec3((k as f32 - 1.0) * 0.08, 0.0, 0.0);
                out.push(MeshDraw {
                    mesh: p.data.mesh,
                    model: Mat4::from_translation(p.pos + off + self.sway_at(p.pos)),
                    tint: white,
                    ..Default::default()
                });
            }
        }
    }
}
