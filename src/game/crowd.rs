//! Menschen im Markt: Spawnen und Verhalten von Kund:innen, Mitarbeitern und normalen Einkaufenden.

use super::*;
use crate::people::Clip;

/// Einkaufsbummel der Kunden: Umkreis um den aktuellen Platz und Tempo
const BROWSE_RADIUS: f32 = 7.0;
const BROWSE_SPEED: f32 = 0.75;
/// Ab dieser Entfernung starren unbeobachtete Kunden herueber
const STARE_RANGE: f32 = 16.0;
/// So viel schneller kommen Verfolger, wenn man rennt
const RUN_CHASE: f32 = 1.6;
/// Flankieren: ab dieser Entfernung, so weit vor dem Spieler wird der Weg abgeschnitten
const FLANK_FROM: f32 = 6.0;
/// So breit ist der Streifen um den Weg des ersten Verfolgers, den der Flankierer meidet
const FLANK_WIDTH: f32 = 1.2;
/// Verfolger, die laut Director gerade nicht heranruecken, lauern auf diesem Abstand
const STALK_DIST: f32 = 9.0;
/// Mindestabstand zwischen Menschen, zum Spieler und zum abgestellten Wagen
const PERSON_GAP: f32 = 0.55;
const PLAYER_GAP: f32 = 0.45;
const CART_GAP: f32 = 0.55;
/// So lange sieht sich ein Mitarbeiter am Ort eines Geraeuschs um
const LOOK_AROUND: f32 = 2.5;

impl Game {
    // --------------------------------------------------------------- Spawns

    pub(super) fn far_walkable_cell(&mut self, min_dist: f32) -> Option<Vec3> {
        let nav = &self.world.nav;
        for _ in 0..400 {
            let x = self.rng.gen_range(0..nav.w);
            let z = self.rng.gen_range(0..nav.h);
            if !nav.npc_walkable(x, z) {
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
                whisper_cd: 0.0,
                gait: Default::default(),
                anim: Default::default(),
                seed: npc::seed_of(pos),
                brain: Default::default(),
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
                whisper_cd: 0.0,
                gait: Default::default(),
                anim: Default::default(),
                seed: npc::seed_of(s.pos),
                brain: Default::default(),
                stare: 0.0,
                stalker: false,
                target: Vec3::ZERO,
                field: Vec::new(),
                wait: 0.0,
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
                whisper_cd: 0.0,
                gait: Default::default(),
                anim: Default::default(),
                seed: npc::seed_of(pos),
                brain: Default::default(),
                stare: 0.0,
                stalker: false,
                target: Vec3::ZERO,
                field: Vec::new(),
                wait: 0.0,
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
            whisper_cd: 0.0,
            gait: Default::default(),
            anim: Default::default(),
            seed: npc::seed_of(pos),
            brain: Default::default(),
            stare: 0.0,
            stalker: true,
            target: pos,
            field: Vec::new(),
            wait: 1.0e6,
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
        // Wo alle vorher standen: wer in den Kassenbereich geraten ist (auch durch Schubsen),
        // muss zurueck
        let before: Vec<Vec3> = self
            .customers
            .iter()
            .map(|c| c.pos)
            .chain(self.employees.iter().map(|e| e.pos))
            .chain(self.shoppers.iter().map(|s| s.pos))
            .collect();
        self.update_senses(dt);
        let paused = self.dialog.is_some() || !self.tipped;
        self.director.update(dt, paused);
        self.update_shoppers(dt);
        self.update_customers(dt);
        self.update_employees(dt, input);
        self.update_mimicry(dt);
        self.separate_people();
        self.keep_npcs_out_of_checkout(&before);
        // Schritte und Schluesselbund (Strecke je NPC in diesem Bild)
        let moved: Vec<f32> = self
            .customers
            .iter()
            .map(|c| c.pos)
            .chain(self.employees.iter().map(|e| e.pos))
            .chain(self.shoppers.iter().map(|s| s.pos))
            .zip(&before)
            .map(|(a, b)| vec3(a.x - b.x, 0.0, a.z - b.z).length().min(0.3))
            .collect();
        self.npc_sounds(&moved);
        self.update_whispers(dt);
        // Gangart: Animationszeit an die zurueckgelegte Strecke koppeln (Fuesse rutschen nicht)
        // im Gespraech (oder erstarrt): Kunden-Posen eingefroren (Animationszeit steht)
        let talking = self.dialog.is_some();
        for i in 0..self.customers.len() {
            if talking || self.customers[i].brain.freeze > 0.0 {
                continue;
            }
            let female = crate::people::PEOPLE[self.customers[i].variant].1;
            let c = &mut self.customers[i];
            c.gait.advance(c.pos, dt, female);
            let clip = self.customer_clip(&self.customers[i]);
            let c = &mut self.customers[i];
            c.anim.update(clip, c.gait.t + c.seed, dt, BLEND);
            // dieselbe Ware immer wieder nehmen
            if clip == Clip::PickUp && c.anim.finished(crate::people::anims(female).duration(clip))
            {
                c.anim.restart();
            }
        }
        for i in 0..self.employees.len() {
            let female = crate::people::PEOPLE[self.employees[i].variant].1;
            let e = &mut self.employees[i];
            e.gait.advance(e.pos, dt, female);
            let clip = self.employee_clip(&self.employees[i]);
            let e = &mut self.employees[i];
            e.anim.update(clip, e.gait.t + e.seed, dt, BLEND);
        }
        for i in 0..self.shoppers.len() {
            let female = crate::people::PEOPLE[self.shoppers[i].variant].1;
            let s = &mut self.shoppers[i];
            s.gait.advance(s.pos, dt, female);
            let clip = self.shopper_clip(&self.shoppers[i]);
            let s = &mut self.shoppers[i];
            s.anim.update(clip, s.gait.t + s.seed, dt, BLEND);
        }
    }

    /// NPCs, die in diesem Bild in den Kassenbereich geraten sind, zurueck an ihren alten Platz.
    fn keep_npcs_out_of_checkout(&mut self, before: &[Vec3]) {
        let w = &self.world;
        let mut old = before.iter();
        let positions = self
            .customers
            .iter_mut()
            .map(|c| &mut c.pos)
            .chain(self.employees.iter_mut().map(|e| &mut e.pos))
            .chain(self.shoppers.iter_mut().map(|s| &mut s.pos));
        for pos in positions {
            let Some(&prev) = old.next() else { break };
            if w.in_checkout_zone(*pos, 0.0) && !w.in_checkout_zone(prev, 0.0) {
                *pos = prev;
            }
        }
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
                let from = self.shoppers[i].pos;
                if let Some((t, field)) = self.reachable_target(from, |g| g.far_walkable_cell(0.0))
                {
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
            if s.brain.motion.stuck(s.pos, true, dt) {
                s.field.clear(); // kommt nicht weiter: neues Ziel
            }
        }
    }

    pub(super) fn update_customers(&mut self, dt: f32) {
        let player = self.player.pos;
        let base = self.tier.customer_speed();
        let running = self.player.vel.length() > WALK_SPEED * 1.3;
        let noticed = self.suspicion >= 1.0;
        let chase = self.chasing();
        // Im Gespraech mit einem Mitarbeiter steht die Welt still: kein Kunde rueckt naeher
        let talking = self.dialog.is_some();
        let flankers = self.flankers();
        let pushers = self.pushers();
        for i in 0..self.customers.len() {
            let chest = self.customers[i].pos + vec3(0.0, 1.25, 0.0);
            let head = self.customers[i].pos + vec3(0.0, 1.6, 0.0);
            let seen = self.sees(chest) || self.sees(head);
            let c = &mut self.customers[i];
            c.seen = seen;
            c.moving = false;
            c.bump_cd -= dt;
            if talking {
                // eingefroren; nur der Blick wandert langsam zu einem herueber
                c.yaw = lerp_angle(c.yaw, npc::yaw_towards(c.pos, player), (dt * 1.5).min(1.0));
                continue;
            }
            // erstarrt (ertappt oder ahmt den Stillstand nach): keine Bewegung, Pose steht
            if c.brain.freeze > 0.0 {
                continue;
            }
            // starrt (nach "hinter dir"): steht nur da, schaut her, tut nichts
            if c.stare > 0.0 {
                c.stare -= dt;
                c.pose = Pose::Idle;
                c.yaw = lerp_angle(c.yaw, npc::yaw_towards(c.pos, player), (dt * 8.0).min(1.0));
                continue;
            }
            // Verfolger kommen naeher, sobald man wegsieht (oder immer, wenn man aufgefallen ist)
            // Nach dem Alarm rennen alle, ob man hinsieht oder nicht. Sonst jagen sie nur,
            // was sie bemerkt haben (senses.rs): den Spieler selbst oder den Ort, an dem sie
            // ihn zuletzt gesehen oder gehoert haben.
            let omniscient = noticed || chase;
            let knows = omniscient || c.brain.memory.is_some();
            let hunting = c.stalker && (!seen || omniscient) && knows;
            if c.brain.hunting && !hunting {
                // Jagd vorbei: nicht zum alten Ziel zurueck, sondern hier weiterbummeln
                c.field.clear();
                c.wait = 0.0;
                // beim Heranschleichen ertappt: erstarrt mitten im Schritt
                if seen {
                    c.brain.freeze = super::brain::FREEZE_CAUGHT;
                    c.brain.hunting = false;
                    continue;
                }
            }
            c.brain.hunting = hunting;
            if !hunting {
                // Wer nicht verfolgt: kauft ein, solange man hinsieht. Sieht man weg, bleibt er
                // stehen und starrt herueber - dreht man sich um, schauen einen alle an.
                let near = (c.pos - player).length() < STARE_RANGE;
                if !seen && near {
                    c.pose = Pose::Idle;
                    c.yaw = lerp_angle(c.yaw, npc::yaw_towards(c.pos, player), (dt * 3.0).min(1.0));
                    continue;
                }
                self.browse_customer(i, dt);
                continue;
            }
            let speed = base
                * (1.0 + self.suspicion * 1.2)
                * (1.0 + c.angry)
                * if self.paid { 1.4 } else { 1.0 };
            // Rennen faellt auf: dann holen sie deutlich schneller auf
            let s = if chase {
                // im engen Fluchtkorridor etwas langsamer
                if self.world.corridor.is_some_and(|k| k.contains(c.pos)) {
                    super::corridor::CORRIDOR_CHASE_SPEED
                } else {
                    super::escape::CHASE_SPEED
                }
            } else if seen {
                speed * 0.4
            } else if running {
                speed * RUN_CHASE
            } else {
                speed
            };
            let dist_now = vec3(c.pos.x - player.x, 0.0, c.pos.z - player.z).length();
            // Director: wer nicht heranruecken darf, lauert auf Abstand und beobachtet
            let held = !omniscient && c.brain.fresh() && !pushers.contains(&i);
            if held && dist_now < STALK_DIST {
                c.moving = false;
                c.yaw = lerp_angle(c.yaw, npc::yaw_towards(c.pos, player), (dt * 4.0).min(1.0));
                continue;
            }
            // Zupacken: auf den letzten Metern ein Ausfallschritt
            if dist_now < super::brain::GRAB_FROM && c.brain.grab <= 0.0 && !held {
                c.brain.grab = super::brain::GRAB_TIME;
            }
            let s = if c.brain.grab > 0.0 { s * 1.3 } else { s };
            let old = c.pos;
            let fresh = omniscient || c.brain.fresh();
            let flank_of = flankers
                .iter()
                .find(|f| f.0 == i)
                .map(|f| f.1)
                .filter(|_| fresh && !omniscient);
            let flank = flank_of.is_some();
            if let Some((to, left)) = c.brain.detour {
                // Ausweichschritt nach dem Feststecken
                let d = vec3(to.x - c.pos.x, 0.0, to.z - c.pos.z);
                if d.length() > 0.05 {
                    c.pos = self
                        .world
                        .collide(c.pos + d.normalize() * (s * dt).min(d.length()), 0.25);
                }
                c.brain.detour = (left > dt).then_some((to, left - dt));
            } else if fresh && !flank {
                // weiss genau, wo man ist: auf kuerzestem Weg
                c.pos = npc::step_along(&self.world, &self.player_field, c.pos, player, s, dt);
            } else {
                // Flankieren (Zange: anderer Weg als der erste Verfolger) oder zur zuletzt
                // bemerkten Stelle
                let goal = if flank {
                    player
                } else {
                    c.brain.memory.unwrap_or(player)
                };
                let c = &mut self.customers[i];
                c.brain.flank_cd -= dt;
                if c.field.is_empty() || (goal - c.target).length() > 1.0 || c.brain.flank_cd <= 0.0
                {
                    c.brain.flank_cd = 0.7;
                    c.target = goal;
                    let from = c.pos;
                    let mut field = std::mem::take(&mut c.field);
                    let nav = &self.world.nav;
                    match flank_of {
                        // der Weg des ersten Verfolgers ist fuer den Flankierer gesperrt
                        Some(lead) => {
                            npc::bfs_blocked(nav, goal, &mut field, |x, z| {
                                let q = nav.center(x, z);
                                (q - goal).length() > 2.5
                                    && (q - from).length() > 1.5
                                    && seg_dist(q, lead, goal) < FLANK_WIDTH
                            });
                            if !npc::reachable(nav, &field, from) {
                                npc::bfs_into(nav, goal, &mut field);
                            }
                        }
                        None => npc::bfs_into(nav, goal, &mut field),
                    }
                    self.customers[i].field = field;
                }
                let c = &mut self.customers[i];
                if !fresh && (c.pos - goal).length() < 0.8 {
                    // dort ist niemand mehr: in der Naehe suchen
                    let around = c.pos;
                    if let Some(q) = self.near_walkable_cell(around, 4.0) {
                        self.customers[i].brain.memory = Some(q);
                    }
                } else {
                    c.pos = npc::step_along(&self.world, &c.field, c.pos, goal, s, dt);
                }
            }
            let c = &mut self.customers[i];
            c.moving = (c.pos - old).length_squared() > 1e-8;
            if c.moving {
                c.yaw = lerp_angle(c.yaw, npc::yaw_towards(old, c.pos), (dt * 10.0).min(1.0));
            }
            c.pose = Pose::Idle;
            c.wait = 0.0;
            if c.brain.motion.stuck(c.pos, true, dt) && c.brain.detour.is_none() {
                let around = c.pos;
                if let Some(q) = self.near_walkable_cell(around, 1.5) {
                    self.customers[i].brain.detour = Some((q, 0.8));
                }
            }
            let c = &mut self.customers[i];
            let dist = vec3(c.pos.x - player.x, 0.0, c.pos.z - player.z).length();
            // Waehrend eines Gespraechs (und kurz danach) ist man sicher
            // ... und an der Kasse, solange das Kassen-Minispiel laeuft
            let safe =
                self.god || self.dialog.is_some() || self.grace > 0.0 || self.till_protects();
            if dist < CATCH_DIST && !self.paid_and_outside() && !safe {
                let (at, variant) = (self.customers[i].pos, self.customers[i].variant);
                self.caught_by(at, variant);
                return;
            }
        }
    }

    /// Ganz normal einkaufen: zum eigenen Regal, hineingreifen, in der Naehe herumschlendern.
    fn browse_customer(&mut self, i: usize, dt: f32) {
        let arrived = {
            let c = &self.customers[i];
            c.field.is_empty() || (c.pos - c.target).length() < 0.5
        };
        if arrived {
            let c = &mut self.customers[i];
            c.wait -= dt;
            let at_home = (c.pos - c.home).length() < 0.8;
            if at_home {
                // am Regal: Ware begutachten
                c.pose = Pose::Reach;
                let want = c.home_facing.x.atan2(c.home_facing.z);
                c.yaw = lerp_angle(c.yaw, want, (dt * 4.0).min(1.0));
            } else {
                c.pose = Pose::Idle;
            }
            if c.wait > 0.0 {
                return;
            }
            let (pos, home) = (c.pos, c.home);
            let go_home = !at_home && self.rng.gen_bool(0.6);
            let picked = if go_home {
                Some(home)
            } else {
                self.near_walkable_cell(pos, BROWSE_RADIUS)
            };
            let (target, field) = match picked {
                Some(t) => self
                    .reachable_target(pos, |_| Some(t))
                    .or_else(|| {
                        self.reachable_target(pos, |g| g.near_walkable_cell(pos, BROWSE_RADIUS))
                    })
                    .unwrap_or((pos, Vec::new())),
                None => (pos, Vec::new()),
            };
            let wait = if go_home {
                self.rng.gen_range(4.0..9.0)
            } else {
                self.rng.gen_range(1.5..4.0)
            };
            let c = &mut self.customers[i];
            c.target = target;
            c.field = field;
            c.wait = wait;
            return;
        }
        let c = &mut self.customers[i];
        let old = c.pos;
        c.pos = npc::step_along(&self.world, &c.field, c.pos, c.target, BROWSE_SPEED, dt);
        c.moving = (c.pos - old).length_squared() > 1e-8;
        if c.moving {
            c.yaw = lerp_angle(c.yaw, npc::yaw_towards(old, c.pos), (dt * 6.0).min(1.0));
        }
        if !c.moving || c.brain.motion.stuck(c.pos, true, dt) {
            // kommt nicht weiter: neues Ziel suchen
            c.field.clear();
        }
        c.pose = Pose::Idle;
    }

    /// Ziel suchen, das von `from` aus wirklich zu Fuss erreichbar ist (bis zu 5 Versuche).
    /// Liefert Ziel und Wegfeld dorthin.
    fn reachable_target(
        &mut self,
        from: Vec3,
        mut pick: impl FnMut(&mut Self) -> Option<Vec3>,
    ) -> Option<(Vec3, Vec<u16>)> {
        for _ in 0..5 {
            let t = pick(self)?;
            let field = npc::bfs(&self.world.nav, t);
            if npc::reachable(&self.world.nav, &field, from) {
                return Some((t, field));
            }
        }
        None
    }

    /// Welche Verfolger gerade wirklich heranruecken duerfen (Director): die naechsten.
    pub(super) fn pushers(&self) -> Vec<usize> {
        let player = self.player.pos;
        let mut hunters: Vec<(f32, usize)> = self
            .customers
            .iter()
            .enumerate()
            .filter(|(_, c)| c.stalker && c.brain.fresh())
            .map(|(i, c)| ((c.pos - player).length(), i))
            .collect();
        hunters.sort_by(|a, b| a.0.total_cmp(&b.0));
        let k = self.director.pushers(hunters.len());
        hunters.iter().take(k).map(|h| h.1).collect()
    }

    /// Welche jagenden Verfolger flankieren, und wer vor ihnen jagt: nach Entfernung sortiert
    /// jeder zweite (nie der naechste), und nur, wer weiter als 6 m weg ist.
    pub(super) fn flankers(&self) -> Vec<(usize, Vec3)> {
        let player = self.player.pos;
        let mut hunters: Vec<(f32, usize)> = self
            .customers
            .iter()
            .enumerate()
            .filter(|(_, c)| c.stalker && c.brain.fresh())
            .map(|(i, c)| ((c.pos - player).length(), i))
            .collect();
        hunters.sort_by(|a, b| a.0.total_cmp(&b.0));
        hunters
            .iter()
            .enumerate()
            .filter(|(rank, (d, _))| rank % 2 == 1 && *d > FLANK_FROM)
            .map(|(rank, (_, i))| (*i, self.customers[hunters[rank - 1].1].pos))
            .collect()
    }

    /// Niemand laeuft durch andere hindurch: Kunden, Personal und Kundschaft halten Abstand
    /// voneinander, zum Spieler und zum abgestellten Wagen. Im Gespraech bleiben Kunden stehen.
    fn separate_people(&mut self) {
        let talking = self.dialog.is_some();
        let player = self.player.pos;
        let cart = (!self.trolley.pushing).then_some(self.trolley.pos);
        // (Position, darf geschoben werden); Jaeger draengen sich am Wagen vorbei
        let mut ps: Vec<(Vec3, bool)> = Vec::new();
        let mut cart_shy: Vec<bool> = Vec::new();
        for c in &self.customers {
            ps.push((c.pos, !talking && c.stare <= 0.0));
            cart_shy.push(!c.brain.hunting);
        }
        for e in &self.employees {
            ps.push((e.pos, e.state != EmpState::Talking && !e.carrying));
            cart_shy.push(true);
        }
        for s in &self.shoppers {
            ps.push((s.pos, true));
            cart_shy.push(true);
        }
        let before: Vec<Vec3> = ps.iter().map(|p| p.0).collect();
        for i in 0..ps.len() {
            for j in (i + 1)..ps.len() {
                let d = vec3(ps[j].0.x - ps[i].0.x, 0.0, ps[j].0.z - ps[i].0.z);
                let l = d.length();
                if l >= PERSON_GAP || l < 1e-4 {
                    continue;
                }
                let push = d / l * (PERSON_GAP - l);
                match (ps[i].1, ps[j].1) {
                    (true, true) => {
                        ps[i].0 -= push * 0.5;
                        ps[j].0 += push * 0.5;
                    }
                    (true, false) => ps[i].0 -= push,
                    (false, true) => ps[j].0 += push,
                    _ => {}
                }
            }
        }
        for (p, shy) in ps.iter_mut().zip(&cart_shy).filter(|(p, _)| p.1) {
            for (o, gap) in [
                (Some(player), PLAYER_GAP),
                (cart.filter(|_| *shy), CART_GAP),
            ] {
                let Some(o) = o else { continue };
                let d = vec3(p.0.x - o.x, 0.0, p.0.z - o.z);
                let l = d.length();
                if l < gap && l > 1e-4 {
                    p.0 += d / l * (gap - l);
                }
            }
        }
        let mut k = 0;
        let world = &self.world;
        let mut write = |pos: &mut Vec3| {
            let (new, movable) = ps[k];
            if movable && (new - before[k]).length_squared() > 1e-10 {
                *pos = world.collide(new, 0.25);
            }
            k += 1;
        };
        for c in &mut self.customers {
            write(&mut c.pos);
        }
        for e in &mut self.employees {
            write(&mut e.pos);
        }
        for s in &mut self.shoppers {
            write(&mut s.pos);
        }
    }

    /// Zufaelliger begehbarer Platz im Umkreis (fuer den Einkaufsbummel).
    fn near_walkable_cell(&mut self, center: Vec3, radius: f32) -> Option<Vec3> {
        let nav = &self.world.nav;
        for _ in 0..60 {
            let p = center
                + vec3(
                    self.rng.gen_range(-radius..radius),
                    0.0,
                    self.rng.gen_range(-radius..radius),
                );
            let Some((x, z)) = nav.cell_of(p) else {
                continue;
            };
            if !nav.npc_walkable(x, z) {
                continue;
            }
            let c = nav.center(x, z);
            if self.world.drinks_zone.contains_xz(c) || c.z > self.world.size.z - 0.5 {
                continue;
            }
            return Some(c);
        }
        None
    }

    /// Sorgt dafuer, dass so viele Kunden Verfolger sind, wie die Stufe vorgibt
    /// (bevorzugt solche, die gerade weit weg sind).
    pub(super) fn assign_stalkers(&mut self) {
        let want = self.tier.stalkers().min(self.customers.len());
        let mut have = self.customers.iter().filter(|c| c.stalker).count();
        let player = self.player.pos;
        let mut order: Vec<usize> = (0..self.customers.len()).collect();
        order.sort_by(|&a, &b| {
            let da = (self.customers[a].pos - player).length();
            let db = (self.customers[b].pos - player).length();
            db.total_cmp(&da)
        });
        for i in order {
            if have >= want {
                break;
            }
            if !self.customers[i].stalker {
                self.customers[i].stalker = true;
                have += 1;
            }
        }
    }

    pub(super) fn paid_and_outside(&self) -> bool {
        self.paid && self.world.exit_zone.contains_xz(self.player.pos)
    }

    pub(super) fn update_employees(&mut self, dt: f32, _input: &Input) {
        let player = self.player.pos;
        let eye = self.eye();
        let sprinting = self.player.vel.length() > 2.2;
        // Ein unbeaufsichtigter Wagen im gekippten Markt wird "aufgeraeumt"
        let unattended =
            self.tipped && !self.paid && !self.trolley.pushing && self.cart_distance() > 6.0;
        for i in 0..self.employees.len() {
            let head = self.employees[i].pos + vec3(0.0, 1.7, 0.0);
            let los = self.world.line_of_sight(head, eye);
            let sees = self.employees[i].brain.sees_player;
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
                    e.yaw =
                        lerp_angle(e.yaw, npc::yaw_towards(e.pos, player), (dt * 10.0).min(1.0));
                    continue;
                }
            }
            if self.chasing() {
                self.chase_employee(i, dt);
                if self.outcome != Outcome::Running {
                    return;
                }
                continue;
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
            // An der Kasse lassen die Mitarbeiter einen in Ruhe
            // bemerkt ihn (Sichtkegel, senses.rs) oder folgt ihm nach einer schlechten Antwort
            let interested = (e.follow > 0.0 || sees) && e.ignore <= 0.0 && !self.till_protects();
            // hat etwas gehoert: geht nachsehen, wo es war (nicht direkt zum Spieler)
            let investigating = !interested
                && e.ignore <= 0.0
                && !self.till_protects()
                && e.brain.investigate.is_some();
            if investigating {
                let spot = e.brain.investigate.unwrap();
                let e = &mut self.employees[i];
                e.state = EmpState::Approach;
                if (e.pos - spot).length() < 1.0 || e.brain.look_around > 0.0 {
                    // angekommen: umsehen, dann zurueck zur Streife
                    e.moving = false;
                    e.brain.look_around += dt;
                    e.yaw += dt * 1.4 * (e.brain.look_around * 1.3).sin().signum();
                    if e.brain.look_around > LOOK_AROUND {
                        e.brain.investigate = None;
                        e.brain.look_around = 0.0;
                        e.field.clear();
                    }
                    continue;
                }
                if e.field.is_empty() || (e.target - spot).length() > 1.0 {
                    e.target = spot;
                    let mut field = std::mem::take(&mut e.field);
                    npc::bfs_into(&self.world.nav, spot, &mut field);
                    self.employees[i].field = field;
                }
                let e = &mut self.employees[i];
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &e.field, e.pos, spot, 1.3, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                if e.moving {
                    e.yaw = lerp_angle(e.yaw, npc::yaw_towards(old, e.pos), (dt * 8.0).min(1.0));
                }
                if e.brain.motion.stuck(e.pos, true, dt) {
                    e.brain.investigate = None;
                }
                continue;
            }
            if interested {
                let busy = self.checking_out();
                let e = &mut self.employees[i];
                e.state = EmpState::Approach;
                let speed = if e.follow > 0.0 { 1.7 } else { 1.2 };
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &self.player_field, e.pos, player, speed, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                e.yaw = lerp_angle(e.yaw, npc::yaw_towards(e.pos, player), (dt * 8.0).min(1.0));
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
                // kniet am Regal und raeumt ein
                let e = &mut self.employees[i];
                if e.brain.restock > 0.0 {
                    e.brain.restock -= dt;
                    e.moving = false;
                    if let Some(f) = e.brain.restock_face {
                        e.yaw = lerp_angle(e.yaw, f.x.atan2(f.z), (dt * 4.0).min(1.0));
                    }
                    continue;
                }
                let e = &self.employees[i];
                let arrived = !e.field.is_empty() && (e.pos - e.target).length() < 0.7;
                if arrived && e.brain.restock_face.is_some() {
                    // am Regal angekommen: einraeumen (im spaeten Markt endlos)
                    let (lo, hi) = if self.tipped && self.tier >= Tier::Wrong {
                        super::brain::RESTOCK_UNCANNY
                    } else {
                        super::brain::RESTOCK_TIME
                    };
                    let t = self.rng.gen_range(lo..hi);
                    let e = &mut self.employees[i];
                    e.brain.restock = t;
                    e.field.clear();
                    e.retarget = 0.0;
                    continue;
                }
                let e = &self.employees[i];
                if e.retarget <= 0.0 || e.field.is_empty() || (e.pos - e.target).length() < 0.7 {
                    // Manchmal gezielt zum herrenlosen Wagen
                    let mut face = None;
                    let target = if unattended
                        && !someone_carrying
                        && e.cart_cd <= 0.0
                        && self.rng.gen_bool(0.5)
                    {
                        self.trolley.pos
                    } else if !self.world.spots.is_empty()
                        && self.rng.gen_bool(super::brain::RESTOCK_SHARE)
                    {
                        // zu einem Regal, dort einraeumen
                        let k = self.rng.gen_range(0..self.world.spots.len());
                        let spot = &self.world.spots[k];
                        face = Some(spot.facing);
                        spot.pos
                    } else {
                        let mut target = e.pos;
                        for _ in 0..60 {
                            let nav = &self.world.nav;
                            let x = self.rng.gen_range(0..nav.w);
                            let z = self.rng.gen_range(0..nav.h);
                            if nav.npc_walkable(x, z) {
                                target = nav.center(x, z);
                                break;
                            }
                        }
                        target
                    };
                    let from = self.employees[i].pos;
                    let (target, field) = self
                        .reachable_target(from, |_| Some(target))
                        .unwrap_or((target, npc::bfs(&self.world.nav, target)));
                    let e = &mut self.employees[i];
                    e.retarget = 12.0;
                    e.target = target;
                    e.field = field;
                    e.brain.restock_face = face;
                    continue;
                }
                let e = &mut self.employees[i];
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &e.field, e.pos, e.target, 0.9, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                if e.moving {
                    e.yaw = lerp_angle(e.yaw, npc::yaw_towards(old, e.pos), (dt * 6.0).min(1.0));
                }
                if e.brain.motion.stuck(e.pos, true, dt) {
                    e.retarget = 0.0;
                }
            }
        }
    }
}

/// Fluestern: so nah muessen sich zwei begegnen, so lange danach Ruhe, so weit hoert man es
const WHISPER_DIST: f32 = 1.4;
const WHISPER_PAUSE: (f32, f32) = (9.0, 18.0);
const WHISPER_HEAR: f32 = 14.0;

impl Game {
    /// Kunden (bzw. normale Einkaufende) fluestern miteinander, wenn sie sich begegnen.
    pub(super) fn update_whispers(&mut self, dt: f32) {
        let mut people: Vec<(Vec3, f32)> = if self.tipped {
            self.customers
                .iter()
                .map(|c| (c.pos, c.whisper_cd))
                .collect()
        } else {
            self.shoppers
                .iter()
                .map(|s| (s.pos, s.whisper_cd))
                .collect()
        };
        for p in &mut people {
            p.1 -= dt;
        }
        let player = self.player.pos;
        for i in 0..people.len() {
            for j in (i + 1)..people.len() {
                if people[i].1 > 0.0 || people[j].1 > 0.0 {
                    continue;
                }
                let d = people[i].0 - people[j].0;
                if vec3(d.x, 0.0, d.z).length() > WHISPER_DIST {
                    continue;
                }
                let mid = (people[i].0 + people[j].0) * 0.5;
                if (mid - player).length() < WHISPER_HEAR {
                    // Fluestern kommt aus ihrer Richtung
                    self.sfx_at
                        .push((Sfx::Whisper, 0.55, mid + vec3(0.0, 1.5, 0.0)));
                }
                people[i].1 = self.rng.gen_range(WHISPER_PAUSE.0..WHISPER_PAUSE.1);
                people[j].1 = self.rng.gen_range(WHISPER_PAUSE.0..WHISPER_PAUSE.1);
            }
        }
        if self.tipped {
            for (c, p) in self.customers.iter_mut().zip(&people) {
                c.whisper_cd = p.1;
            }
        } else {
            for (s, p) in self.shoppers.iter_mut().zip(&people) {
                s.whisper_cd = p.1;
            }
        }
    }
}

/// Ueberblendzeit zwischen zwei Clips (Sekunden)
const BLEND: f32 = 0.25;

/// Gangart -> Clip: Gehen/Rennen laufen mit der Strecke mit, sonst Stehen (oder Greifen)
pub(super) fn gait_clip(g: &crate::people::Gait, still: Clip) -> Clip {
    if !g.moving {
        still
    } else if g.run {
        Clip::Run
    } else {
        Clip::Walk
    }
}

/// Abstand eines Punkts zur Strecke a-b (in der Ebene).
fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = vec3(b.x - a.x, 0.0, b.z - a.z);
    let ap = vec3(p.x - a.x, 0.0, p.z - a.z);
    let t = (ap.dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (ap - ab * t).length()
}
