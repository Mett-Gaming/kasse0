//! Alles, was der Spieler mit den Haenden tut: Wagen schieben/abstellen, Waren greifen,
//! in den Wagen legen, fallen lassen, wieder herausnehmen, an der Kasse scannen.

use super::*;

pub(super) const CART_OFFSET: f32 = 0.85;
/// Kollisionsradius des geschobenen Wagens (breiter als ein Durchgang)
const CART_PUSH_RADIUS: f32 = 0.42;
/// Radius des abgestellten Wagens fuer die Spielerkollision
const CART_RADIUS: f32 = 0.5;
/// Wie nah der Wagen fuer Q/F sein muss
const CART_REACH: f32 = 1.9;

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
        if wish.length_squared() > 0.0 {
            wish = wish.normalize() * if input.run { RUN_SPEED } else { WALK_SPEED };
        }
        let old = self.player.pos;
        self.player.vel = self.player.vel.lerp(wish, (dt * 10.0).min(1.0));
        let mut pos = self
            .world
            .collide(old + self.player.vel * dt, PLAYER_RADIUS);

        if self.trolley.pushing {
            // Der geschobene Wagen ist breiter als ein Mensch: schmale Durchgaenge gehen nicht
            let cart = pos + forward * CART_OFFSET;
            let fixed = self.world.collide(cart, CART_PUSH_RADIUS);
            pos = self.world.collide(pos + (fixed - cart), PLAYER_RADIUS);
            let want = pos + forward * CART_OFFSET;
            let t = &mut self.trolley;
            t.pos = t.pos.lerp(want, (dt * 12.0).min(1.0));
            t.yaw = lerp_angle(t.yaw, self.player.yaw, (dt * 12.0).min(1.0));

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
            // Das Rad quietscht - Mitarbeiter hoeren das
            self.squeak_acc += moved;
            if self.squeak_acc > 1.3 {
                self.squeak_acc = 0.0;
                self.sfx
                    .push((Sfx::Squeak, if input.run { 0.6 } else { 0.35 }));
                if self.tipped {
                    self.make_noise(if input.run { 20.0 } else { 14.0 });
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

        let moved = (pos - old).length();
        let p = &mut self.player;
        p.pos = pos;
        p.bob_phase += moved * 5.5;
        p.step_acc += moved;
        if p.step_acc > 0.7 {
            p.step_acc = 0.0;
            self.sfx
                .push((Sfx::Step, if input.run { 0.5 } else { 0.25 }));
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

        // Der Wagen selbst
        if let Some(t) = self.cart_box().ray(eye, dir, REACH + 0.4) {
            best = t;
            self.focus = Some(Focus::Cart);
            self.aim_cart = true;
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
            Some(Focus::Belt) => return self.use_belt(),
            Some(Focus::Terminal) => return self.use_terminal(),
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

    /// Lautes Geraeusch an der Spielerposition: Mitarbeiter im Umkreis kommen nachsehen.
    pub(super) fn make_noise(&mut self, radius: f32) {
        let p = self.player.pos;
        for e in &mut self.employees {
            if !e.carrying && e.ignore <= 0.0 && (e.pos - p).length() < radius {
                e.heard = 6.0;
            }
        }
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
        if self.squeak_acc > 1.3 {
            self.squeak_acc = 0.0;
            let d = (e.pos - self.player.pos).length();
            let vol = (1.0 - d / 25.0).max(0.0) * 0.4;
            if vol > 0.02 {
                self.sfx.push((Sfx::Squeak, vol));
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
        if let Some(item) = self.cart.pop() {
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
        it.taken = true;
        let data = ItemData::from_shelf(it);
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
            }
            self.say("Das war IHR Tomatensaft. Sie hat es gesehen.", 3.5);
            self.play(Sfx::Sting, 0.3);
        } else {
            self.play(Sfx::Pickup, 0.5);
        }
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
        for l in &mut self.loose {
            if l.vel == Vec3::ZERO {
                continue;
            }
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
            });
        }

        // Wagen + Inhalt
        let m = Mat4::from_rotation_translation(rot(-self.trolley.yaw), self.trolley.pos);
        out.push(MeshDraw {
            mesh: MeshId::Cart as usize,
            model: m,
            tint: white,
        });
        for (i, item) in self.cart.iter().enumerate() {
            let col = (i % 3) as f32 - 1.0;
            let row = ((i / 3) % 4) as f32 - 1.5;
            let layer = (i / 12) as f32;
            let local = vec3(col * 0.13, 0.42 + layer * 0.15, row * 0.17);
            let spin = Quat::from_rotation_y(i as f32 * 1.7);
            let model = m * Mat4::from_rotation_translation(spin, local);
            out.push(MeshDraw {
                mesh: item.mesh as usize,
                model,
                tint: item.tint.extend(0.0),
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
            });
        }

        for l in &self.loose {
            out.push(MeshDraw {
                mesh: l.data.mesh as usize,
                model: Mat4::from_rotation_translation(rot(-l.yaw), l.pos),
                tint: l.data.tint.extend(0.0),
            });
        }

        // Tomatensaft der Kunden (Regalware ist statisch, siehe World::item_draws)
        for p in self.pickups.iter().filter(|p| !p.taken) {
            for k in 0..3 {
                let off = vec3((k as f32 - 1.0) * 0.08, 0.0, 0.0);
                out.push(MeshDraw {
                    mesh: p.data.mesh,
                    model: Mat4::from_translation(p.pos + off),
                    tint: white,
                });
            }
        }
    }
}
