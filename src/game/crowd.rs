//! Menschen im Markt: Spawnen und Verhalten von Kund:innen, Mitarbeitern und normalen Einkaufenden.

use super::*;

impl Game {
    // --------------------------------------------------------------- Spawns

    pub(super) fn far_walkable_cell(&mut self, min_dist: f32) -> Option<Vec3> {
        let nav = &self.world.nav;
        for _ in 0..400 {
            let x = self.rng.gen_range(0..nav.w);
            let z = self.rng.gen_range(0..nav.h);
            if !nav.is_walkable(x, z) {
                continue;
            }
            let p = nav.center(x, z);
            if self.world.drinks_zone.contains_xz(p) || p.z > self.world.size.z - 0.5 {
                continue;
            }
            if (p - self.player.pos).length() >= min_dist && !self.sees(p + vec3(0.0, 1.2, 0.0)) {
                return Some(p);
            }
        }
        None
    }

    /// Zufaellige Figur: Kundschaft (Alltagskleidung) oder Personal (Dienstkleidung, gesichtslos).
    pub(super) fn pick_person(&mut self, staff: bool) -> usize {
        let pool = crate::assets::people_of(staff);
        pool[self.rng.gen_range(0..pool.len())]
    }

    pub(super) fn spawn_shoppers(&mut self) {
        for i in 0..5 {
            let Some(pos) = self.far_walkable_cell(4.0) else {
                continue;
            };
            let tint = SHOPPER_TINTS[i % SHOPPER_TINTS.len()];
            let variant = self.pick_person(false);
            self.shoppers.push(Shopper {
                pos,
                yaw: 0.0,
                target: pos,
                field: Vec::new(),
                tint,
                variant,
                wait: 0.0,
            });
        }
    }

    pub(super) fn spawn_customers(&mut self, n: usize) {
        let mut spots = self.world.spots.clone();
        spots.shuffle(&mut self.rng);
        let player = self.player.pos;
        spots.sort_by(|a, b| {
            (b.pos - player)
                .length()
                .total_cmp(&(a.pos - player).length())
        });
        let used: Vec<Vec3> = self.customers.iter().map(|c| c.home).collect();
        let mut added = 0;
        for s in spots {
            if added >= n {
                break;
            }
            if used.iter().any(|u| (*u - s.pos).length() < 2.0)
                || (s.pos - player).length() < 12.0
                || self.sees(s.pos + vec3(0.0, 1.2, 0.0))
            {
                continue;
            }
            let idx = self.customers.len();
            let tint = CUSTOMER_TINTS[self.rng.gen_range(0..CUSTOMER_TINTS.len())];
            let variant = self.pick_person(false);
            self.customers.push(Customer {
                pos: s.pos,
                yaw: s.facing.x.atan2(s.facing.z),
                home: s.pos,
                home_facing: s.facing,
                variant,
                tint,
                seen: false,
                moving: false,
                pose: Pose::Reach,
                angry: 0.0,
                bump_cd: 0.0,
            });
            // "ihr" Tomatensaft
            self.pickups.push(Pickup {
                data: ItemData {
                    name: "Tomatensaft (abgelaufen)".into(),
                    cents: 89,
                    kind: PickupKind::Tomato { customer: idx },
                    mesh: MeshId::Can as usize,
                    tint: Vec3::ONE,
                },
                pos: s.shelf,
                taken: false,
            });
            added += 1;
        }
        // nicht genug Regalplaetze: irgendwo in die Ferne stellen
        while added < n {
            let Some(pos) = self.far_walkable_cell(14.0) else {
                break;
            };
            let tint = CUSTOMER_TINTS[self.rng.gen_range(0..CUSTOMER_TINTS.len())];
            let variant = self.pick_person(false);
            self.customers.push(Customer {
                pos,
                yaw: 0.0,
                home: pos,
                home_facing: vec3(0.0, 0.0, 1.0),
                variant,
                tint,
                seen: false,
                moving: false,
                pose: Pose::Idle,
                angry: 0.0,
                bump_cd: 0.0,
            });
            added += 1;
        }
    }

    pub(super) fn spawn_employees(&mut self, n: usize) {
        for _ in 0..n {
            let Some(pos) = self.far_walkable_cell(15.0) else {
                break;
            };
            let mut e = Employee::new(pos);
            e.variant = self.pick_person(true);
            self.employees.push(e);
        }
    }

    /// Fuer Tests/Screenshots: einen Kunden an eine Stelle setzen.
    pub fn spawn_customer_at(&mut self, pos: Vec3) {
        self.customers.push(Customer {
            pos,
            yaw: 0.0,
            home: pos,
            home_facing: vec3(0.0, 0.0, -1.0),
            variant: 0,
            tint: CUSTOMER_TINTS[0],
            seen: false,
            moving: false,
            pose: Pose::Idle,
            angry: 1.0,
            bump_cd: 0.0,
        });
    }

    // ----------------------------------------------------------------- NPCs

    pub(super) fn update_npcs(&mut self, dt: f32, input: &Input) {
        // Distanzfeld zum Spieler regelmaessig neu berechnen
        self.field_timer -= dt;
        if self.field_timer <= 0.0 || self.player_field.len() != self.world.nav.w * self.world.nav.h
        {
            self.player_field = npc::bfs(&self.world.nav, self.player.pos);
            self.field_timer = 0.3;
        }
        self.update_shoppers(dt);
        self.update_customers(dt);
        self.update_employees(dt, input);
    }

    pub(super) fn update_shoppers(&mut self, dt: f32) {
        for i in 0..self.shoppers.len() {
            let needs_target = {
                let s = &self.shoppers[i];
                s.field.is_empty() || (s.pos - s.target).length() < 0.6
            };
            if needs_target {
                self.shoppers[i].wait -= dt;
                if self.shoppers[i].wait > 0.0 {
                    continue;
                }
                if let Some(t) = self.far_walkable_cell(0.0) {
                    let field = npc::bfs(&self.world.nav, t);
                    let s = &mut self.shoppers[i];
                    s.target = t;
                    s.field = field;
                    s.wait = self.rng.gen_range(1.0..4.0);
                }
                continue;
            }
            let s = &mut self.shoppers[i];
            let old = s.pos;
            s.pos = npc::step_along(&self.world, &s.field, s.pos, s.target, 0.9, dt);
            if (s.pos - old).length_squared() > 1e-6 {
                s.yaw = lerp_angle(s.yaw, npc::yaw_towards(old, s.pos), (dt * 6.0).min(1.0));
            }
        }
    }

    pub(super) fn update_customers(&mut self, dt: f32) {
        let player = self.player.pos;
        let base = self.tier.customer_speed();
        let noticed = self.suspicion >= 1.0;
        for i in 0..self.customers.len() {
            let chest = self.customers[i].pos + vec3(0.0, 1.25, 0.0);
            let head = self.customers[i].pos + vec3(0.0, 1.6, 0.0);
            let seen = self.sees(chest) || self.sees(head);
            let c = &mut self.customers[i];
            c.seen = seen;
            c.moving = false;
            c.bump_cd -= dt;
            let speed = base
                * (1.0 + self.suspicion * 1.2)
                * (1.0 + c.angry)
                * if self.paid { 1.4 } else { 1.0 };
            let may_move = !seen || noticed;
            if may_move {
                let s = if seen { speed * 0.4 } else { speed };
                let old = c.pos;
                c.pos = npc::step_along(&self.world, &self.player_field, c.pos, player, s, dt);
                c.moving = (c.pos - old).length_squared() > 1e-8;
                if (c.pos - old).length_squared() > 1e-8 {
                    c.yaw = npc::yaw_towards(old, c.pos);
                }
                c.pose = Pose::Idle;
            } else {
                // beim Hinsehen: Rollenspiel am Regal oder starr zum Spieler
                let at_home = (c.pos - c.home).length() < 0.5;
                if at_home {
                    c.pose = Pose::Reach;
                    c.yaw = c.home_facing.x.atan2(c.home_facing.z);
                } else {
                    c.pose = Pose::Idle;
                    c.yaw = npc::yaw_towards(c.pos, player);
                }
            }
            let dist = vec3(c.pos.x - player.x, 0.0, c.pos.z - player.z).length();
            // Waehrend eines Gespraechs (und kurz danach) ist man sicher
            let safe = self.god || self.dialog.is_some() || self.grace > 0.0;
            if dist < CATCH_DIST && !self.paid_and_outside() && !safe {
                self.outcome = Outcome::Caught;
                self.outcome_timer = 0.0;
                self.catcher = Some(self.customers[i].pos);
                self.red = 1.0;
                self.glitch = 1.0;
                self.sfx.push((Sfx::Sting, 1.0));
                return;
            }
        }
        // Kunden nicht ineinander laufen lassen
        for i in 0..self.customers.len() {
            for j in (i + 1)..self.customers.len() {
                let d = self.customers[j].pos - self.customers[i].pos;
                let l = vec3(d.x, 0.0, d.z).length();
                if l < 0.6 && l > 1e-4 {
                    let push = vec3(d.x, 0.0, d.z) / l * (0.6 - l) * 0.5;
                    self.customers[i].pos -= push;
                    self.customers[j].pos += push;
                }
            }
        }
    }

    pub(super) fn paid_and_outside(&self) -> bool {
        self.paid && self.player.pos.z > self.world.exit_z - 0.5
    }

    pub(super) fn update_employees(&mut self, dt: f32, _input: &Input) {
        let player = self.player.pos;
        let eye = self.eye();
        let sprinting = self.player.vel.length() > 2.4;
        // Ein unbeaufsichtigter Wagen im gekippten Markt wird "aufgeraeumt"
        let unattended =
            self.tipped && !self.paid && !self.trolley.pushing && self.cart_distance() > 6.0;
        for i in 0..self.employees.len() {
            let head = self.employees[i].pos + vec3(0.0, 1.7, 0.0);
            let los = self.world.line_of_sight(head, eye);
            {
                let e = &mut self.employees[i];
                e.ignore -= dt;
                e.follow -= dt;
                e.retarget -= dt;
                e.heard -= dt;
                e.cart_cd -= dt;
                e.sprint_cd -= dt;
                if e.state == EmpState::Talking {
                    e.moving = false;
                    e.yaw = npc::yaw_towards(e.pos, player);
                    continue;
                }
            }
            if self.employees[i].carrying {
                self.carry_step(i, dt);
                continue;
            }
            let someone_carrying = self.employees.iter().any(|e| e.carrying);
            let e = &self.employees[i];
            if unattended
                && !someone_carrying
                && e.cart_cd <= 0.0
                && (e.pos - self.trolley.pos).length() < 1.6
            {
                self.start_carrying(i);
                continue;
            }
            let dist = (e.pos - player).length();
            let interested =
                (e.follow > 0.0 || e.heard > 0.0 || (dist < 7.0 && los)) && e.ignore <= 0.0;
            if interested {
                let busy = self.checking_out();
                let e = &mut self.employees[i];
                e.state = EmpState::Approach;
                let speed = if e.follow > 0.0 { 1.7 } else { 1.2 };
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &self.player_field, e.pos, player, speed, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                e.yaw = npc::yaw_towards(e.pos, player);
                if dist < 1.7
                    && los
                    && self.dialog.is_none()
                    && !busy
                    && self.outcome == Outcome::Running
                {
                    if sprinting {
                        // Vorbeigerannt: kein Gespraech, aber das faellt auf
                        if e.sprint_cd <= 0.0 {
                            e.sprint_cd = 8.0;
                            e.follow = 12.0;
                            self.suspicion += 0.2;
                            self.glitch = self.glitch.max(0.5);
                            self.message =
                                Some(("Du bist an ihm vorbeigerannt. Das fiel auf.".into(), 3.0));
                        }
                    } else {
                        e.state = EmpState::Talking;
                        let q = self.rng.gen_range(0..QUESTIONS.len());
                        self.open_dialog(i, q, false);
                    }
                }
            } else {
                self.employees[i].state = EmpState::Patrol;
                let e = &self.employees[i];
                if e.retarget <= 0.0 || e.field.is_empty() || (e.pos - e.target).length() < 0.7 {
                    // Manchmal gezielt zum herrenlosen Wagen
                    let target = if unattended
                        && !someone_carrying
                        && e.cart_cd <= 0.0
                        && self.rng.gen_bool(0.5)
                    {
                        self.trolley.pos
                    } else {
                        let mut target = e.pos;
                        for _ in 0..60 {
                            let nav = &self.world.nav;
                            let x = self.rng.gen_range(0..nav.w);
                            let z = self.rng.gen_range(0..nav.h);
                            if nav.is_walkable(x, z) {
                                target = nav.center(x, z);
                                break;
                            }
                        }
                        target
                    };
                    let field = npc::bfs(&self.world.nav, target);
                    let e = &mut self.employees[i];
                    e.retarget = 12.0;
                    e.target = target;
                    e.field = field;
                    continue;
                }
                let e = &mut self.employees[i];
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &e.field, e.pos, e.target, 0.9, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                if e.moving {
                    e.yaw = npc::yaw_towards(old, e.pos);
                }
            }
        }
    }
}
