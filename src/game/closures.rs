//! Mettarbeiter sperren Gaenge: Im gekippten Markt geht ab und zu ein Mitarbeiter an ein
//! Gangende, kniet kurz und spannt an beiden Enden Absperrband ("Wird befuellt"). Er bleibt
//! als Wache stehen; nach Ablauf baut er wieder ab. Spieler, geschobener Wagen und NPCs
//! kommen nicht durch das Band (Durchgaenge in gebogenen Regalen bleiben offen).
//! Ab Stufe "falsch" sperren sie bevorzugt den Gang mit der naechsten noch fehlenden Ware.
//! Nie wird der Gang gesperrt, in dem der Spieler gerade steht.

use super::*;

/// Abstand zwischen Sperren (s): bis "unruhig", ab "falsch"
const GAP_CALM: (f32, f32) = (60.0, 90.0);
const GAP_WRONG: (f32, f32) = (35.0, 60.0);
/// So lange bleibt ein Gang gesperrt (s), im Kollaps laenger
pub const LIFE: f32 = 40.0;
pub const LIFE_COLLAPSE: f32 = 60.0;
/// Aufbauen und Abbauen: so lange kniet die Wache (s)
pub const SETUP: f32 = 2.0;
const TEARDOWN: f32 = 1.5;
/// So lange darf der Weg zum Gang dauern, sonst faellt die Sperre aus (s)
const WALK_LIMIT: f32 = 30.0;
/// Gehgeschwindigkeit zum Gang (m/s)
const WALK_SPEED: f32 = 1.1;
/// Halbe Dicke des Bands fuer die Kollision (m)
const TAPE_HALF: f32 = 0.06;
/// Nach einer guten Antwort: so schnell ist der Gang wieder frei (s)
const OPEN_SOON: f32 = 3.0;

/// Was die Sperre gerade tut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Wache geht zum Gangende (Zeit unterwegs)
    Walk(f32),
    /// Wache kniet und spannt das Band
    Setup(f32),
    /// gesperrt (Zeit seit Beginn)
    Active(f32),
    /// Wache baut ab (Band haengt noch)
    Teardown(f32),
}

#[derive(Clone, Debug)]
pub struct Closure {
    /// Gangende aus `world.aisle_gates`
    pub gate: world::AisleGate,
    pub phase: Phase,
    /// So lange bleibt der Gang gesperrt (s)
    pub life: f32,
    /// Index der Wache in `employees`
    pub guard: Option<usize>,
    /// Navigationszellen, die fuer NPCs gesperrt wurden (werden beim Abbau freigegeben)
    pub blocked: Vec<usize>,
}

impl Closure {
    /// Haengt das Band gerade (Spieler und NPCs kommen nicht durch)?
    pub fn taped(&self) -> bool {
        matches!(self.phase, Phase::Active(_) | Phase::Teardown(_))
    }
}

impl Game {
    fn closure_gap(&mut self) -> f32 {
        let (a, b) = if self.tier >= Tier::Wrong {
            GAP_WRONG
        } else {
            GAP_CALM
        };
        self.eerie_rng.gen_range(a..b)
    }

    /// Steht `p` im Laufweg des Gangs (zwischen den beiden Enden)?
    pub(super) fn in_gate(gate: &world::AisleGate, p: Vec3, margin: f32) -> bool {
        let [n, s] = gate.ends;
        if p.z < n.z || p.z > s.z {
            return false;
        }
        let t = (p.z - n.z) / (s.z - n.z).max(0.01);
        let mid = n.x + (s.x - n.x) * t;
        (p.x - mid).abs() < gate.half + margin
    }

    /// Welcher Gang gesperrt wird: nie der, in dem der Spieler steht; ab "falsch" (oder wenn
    /// `target` verlangt) der Gang der naechsten noch fehlenden Ware vom Zettel.
    pub(super) fn pick_closure_gate(&mut self, target: bool) -> Option<world::AisleGate> {
        let player = self.player.pos;
        let free: Vec<world::AisleGate> = self
            .world
            .aisle_gates
            .iter()
            .filter(|g| !Self::in_gate(g, player, 0.4))
            .copied()
            .collect();
        if free.is_empty() {
            return None;
        }
        if target || self.tier >= Tier::Wrong {
            // naechste Ware, die noch auf dem Zettel fehlt (echte Ausgabe im Regal)
            let cat = catalog();
            let mut best: Option<(f32, world::AisleGate)> = None;
            for it in self.world.items.iter().filter(|it| !it.taken && !it.fake) {
                let needed = cat.products[it.product as usize]
                    .list
                    .is_some_and(|l| self.list.contains(&l) && !self.list_done(l));
                if !needed {
                    continue;
                }
                let d = (it.pos - player).length();
                if let Some(g) = free.iter().find(|g| Self::in_gate(g, it.pos, 0.8)) {
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, *g));
                    }
                }
            }
            if let Some((_, g)) = best {
                return Some(g);
            }
        }
        let k = self.eerie_rng.gen_range(0..free.len());
        Some(free[k])
    }

    /// Freier Mitarbeiter, der am naechsten am Gang ist (nicht im Gespraech, nicht mit Wagen).
    fn free_employee_near(&self, p: Vec3, exclude: Option<usize>) -> Option<usize> {
        (0..self.employees.len())
            .filter(|&i| {
                let e = &self.employees[i];
                e.state != EmpState::Talking && !e.carrying && Some(i) != exclude
            })
            .min_by(|&a, &b| {
                let da = (self.employees[a].pos - p).length();
                let db = (self.employees[b].pos - p).length();
                da.total_cmp(&db)
            })
    }

    /// Eine Sperre beginnen (Mitarbeiter geht hin). `target`: Gang der naechsten fehlenden
    /// Ware erzwingen. `guard`: dieser Mitarbeiter, sonst der naechste freie.
    pub(super) fn start_closure(&mut self, target: bool, guard: Option<usize>) -> bool {
        self.start_closure_except(target, guard, None)
    }

    /// Wie `start_closure`, aber Mitarbeiter `except` wird nicht als Wache genommen (z. B. der,
    /// der dem Spieler nach einer schlechten Antwort folgt).
    pub(super) fn start_closure_except(
        &mut self,
        target: bool,
        guard: Option<usize>,
        except: Option<usize>,
    ) -> bool {
        if self.closure.is_some() {
            return false;
        }
        let Some(gate) = self.pick_closure_gate(target) else {
            return false;
        };
        let guard = guard.or_else(|| self.free_employee_near(gate.ends[0], except));
        let Some(g) = guard else { return false };
        let life = if self.tier == Tier::Collapse {
            LIFE_COLLAPSE
        } else {
            LIFE
        };
        let e = &mut self.employees[g];
        e.state = EmpState::Patrol;
        e.field.clear();
        e.follow = 0.0;
        self.closure = Some(Closure {
            gate,
            phase: Phase::Walk(0.0),
            life,
            guard: Some(g),
            blocked: Vec::new(),
        });
        true
    }

    /// Stehplatz der Wache: vor dem naeheren Gangende, aussen.
    fn guard_spot(&self, c: &Closure, from: Vec3) -> Vec3 {
        let [n, s] = c.gate.ends;
        if (from - n).length() <= (from - s).length() {
            self.world.collide(n - vec3(0.0, 0.0, 0.7), 0.25)
        } else {
            self.world.collide(s + vec3(0.0, 0.0, 0.7), 0.25)
        }
    }

    /// Zeit, Ablauf, Abbau; neue Sperren nach Zeitplan.
    pub(super) fn update_closures(&mut self, dt: f32) {
        let allowed = self.tipped
            && !self.paid
            && !self.tutorial_run
            && self.outcome == Outcome::Running
            && !self.umbau_running();
        if let Some(c) = &self.closure {
            // Umbau: das alte Gangende gibt es nicht mehr
            let same = self
                .world
                .aisle_gates
                .get(c.gate.aisle)
                .is_some_and(|g| g.ends == c.gate.ends);
            if !allowed || !same {
                if !same {
                    // neues Wegeraster: die alten Zellen gehoeren nicht mehr zur Sperre
                    if let Some(c) = &mut self.closure {
                        c.blocked.clear();
                    }
                }
                self.end_closure();
                return;
            }
        }
        let Some(mut c) = self.closure.take() else {
            if !allowed || self.checking_out() {
                return;
            }
            self.closure_timer -= dt;
            if self.closure_timer <= 0.0 {
                self.closure_timer = self.closure_gap();
                self.start_closure(false, None);
            }
            return;
        };
        // Wache weg (Liste geleert, Flucht ...): Sperre laeuft ohne sie weiter
        if c.guard.is_some_and(|g| g >= self.employees.len()) {
            c.guard = None;
        }
        c.phase = match c.phase {
            Phase::Walk(t) => {
                if t + dt > WALK_LIMIT || c.guard.is_none() {
                    self.closure = Some(c);
                    self.end_closure();
                    return;
                }
                Phase::Walk(t + dt)
            }
            Phase::Setup(t) if t + dt >= SETUP => {
                self.tape_up(&mut c);
                Phase::Active(0.0)
            }
            Phase::Setup(t) => Phase::Setup(t + dt),
            Phase::Active(t) if t + dt >= c.life => Phase::Teardown(0.0),
            Phase::Active(t) => Phase::Active(t + dt),
            Phase::Teardown(t) if t + dt >= TEARDOWN => {
                self.closure = Some(c);
                self.end_closure();
                return;
            }
            Phase::Teardown(t) => Phase::Teardown(t + dt),
        };
        self.closure = Some(c);
    }

    /// Band spannen: NPC-Zellen am Band sperren (nur solche, die vorher frei waren).
    fn tape_up(&mut self, c: &mut Closure) {
        let nav = &mut self.world.nav;
        if nav.npc_block.len() != nav.w * nav.h {
            nav.npc_block = vec![false; nav.w * nav.h];
        }
        for end in c.gate.ends {
            let r = world::NAV_CLEARANCE;
            for z in 0..nav.h {
                for x in 0..nav.w {
                    let p = nav.center(x, z);
                    if (p.z - end.z).abs() < TAPE_HALF + r
                        && (p.x - end.x).abs() < c.gate.half + r
                        && !nav.npc_block[z * nav.w + x]
                    {
                        nav.npc_block[z * nav.w + x] = true;
                        c.blocked.push(z * nav.w + x);
                    }
                }
            }
        }
        self.play(Sfx::Rattle, 0.35);
    }

    /// Sperre aufheben: Zellen freigeben, Wache zurueck auf Streife.
    pub(super) fn end_closure(&mut self) {
        let Some(c) = self.closure.take() else { return };
        let nav = &mut self.world.nav;
        for k in c.blocked {
            if let Some(b) = nav.npc_block.get_mut(k) {
                *b = false;
            }
        }
        if let Some(e) = c.guard.and_then(|g| self.employees.get_mut(g)) {
            e.brain.restock = 0.0;
            e.retarget = 0.0;
            e.field.clear();
        }
    }

    /// Gute Antwort: Gang wird gleich wieder frei.
    pub(super) fn open_closure_soon(&mut self) -> bool {
        let Some(c) = &mut self.closure else {
            return false;
        };
        match c.phase {
            Phase::Active(t) => {
                c.life = c.life.min(t + OPEN_SOON);
                true
            }
            Phase::Walk(_) | Phase::Setup(_) => {
                self.end_closure();
                true
            }
            Phase::Teardown(_) => false,
        }
    }

    /// Cheat: Markt kippen und sofort einen Gang sperren (Wache steht schon dort, Band haengt).
    pub(super) fn debug_closure(&mut self) -> String {
        self.tip();
        self.end_closure();
        if self.employees.is_empty() {
            self.spawn_employees(1);
        }
        if !self.start_closure(true, None) {
            return "Kein Gang frei (du stehst in allen?).".into();
        }
        let Some(mut c) = self.closure.take() else {
            return "Keine Sperre.".into();
        };
        if let Some(g) = c.guard {
            let spot = self.guard_spot(&c, c.gate.ends[0]);
            self.employees[g].pos = spot;
        }
        self.tape_up(&mut c);
        c.phase = Phase::Active(0.0);
        self.closure = Some(c);
        "Ein Gang ist gesperrt: Wird befüllt.".into()
    }

    /// Ist Mitarbeiter `i` gerade mit einer Sperre beschaeftigt (Weg, Aufbau, Wache, Abbau)?
    pub(super) fn is_guard(&self, i: usize) -> bool {
        self.closure.as_ref().is_some_and(|c| c.guard == Some(i))
    }

    /// Ein Schritt der Wache: hingehen, knien, Wache stehen (Blick in den Gang), abbauen.
    pub(super) fn guard_step(&mut self, i: usize, dt: f32) {
        let Some(c) = self.closure.clone() else {
            return;
        };
        let spot = self.guard_spot(&c, self.employees[i].pos);
        let look = (c.gate.ends[0] + c.gate.ends[1]) * 0.5;
        match c.phase {
            Phase::Walk(_) => {
                let e = &mut self.employees[i];
                e.state = EmpState::Patrol;
                if (e.pos - spot).length() < 0.5 {
                    if let Some(c) = &mut self.closure {
                        c.phase = Phase::Setup(0.0);
                    }
                    return;
                }
                if e.field.is_empty() || (e.target - spot).length() > 0.5 {
                    e.target = spot;
                    let mut field = std::mem::take(&mut e.field);
                    npc::bfs_into(&self.world.nav, spot, &mut field);
                    self.employees[i].field = field;
                }
                let e = &mut self.employees[i];
                let old = e.pos;
                e.pos = npc::step_along(&self.world, &e.field, e.pos, spot, WALK_SPEED, dt);
                e.moving = (e.pos - old).length_squared() > 1e-8;
                if e.moving {
                    e.yaw = lerp_angle(e.yaw, npc::yaw_towards(old, e.pos), (dt * 8.0).min(1.0));
                }
            }
            Phase::Setup(_) | Phase::Teardown(_) => {
                // kniet (Clip Kneel ueber brain.restock, siehe brain.rs)
                let e = &mut self.employees[i];
                e.moving = false;
                e.brain.restock = 0.5;
                e.yaw = lerp_angle(e.yaw, npc::yaw_towards(e.pos, look), (dt * 6.0).min(1.0));
            }
            Phase::Active(_) => {
                let e = &mut self.employees[i];
                e.moving = false;
                e.brain.restock = 0.0;
                e.yaw = lerp_angle(e.yaw, npc::yaw_towards(e.pos, look), (dt * 4.0).min(1.0));
            }
        }
    }

    /// Haelt Spieler (und geschobenen Wagen) vom gespannten Band fern: Bewegung nur, wenn sie
    /// nicht tiefer hineinfuehrt, sonst am Band entlanggleiten.
    pub(super) fn keep_out_of_closure(&mut self, old: Vec3, pos: Vec3) -> Vec3 {
        let Some(c) = self.closure.as_ref().filter(|c| c.taped()) else {
            return pos;
        };
        let (s, co) = self.player.yaw.sin_cos();
        let forward = vec3(s, 0.0, -co);
        let pushing = self.trolley.pushing;
        let ends = c.gate.ends;
        let half = c.gate.half;
        let depth = |p: Vec3, r: f32| {
            ends.iter()
                .map(|e| {
                    let dx = half + r - (p.x - e.x).abs();
                    let dz = TAPE_HALF + r - (p.z - e.z).abs();
                    dx.min(dz).max(0.0)
                })
                .fold(0.0f32, f32::max)
        };
        let worst = |p: Vec3| {
            let cart = if pushing {
                depth(p + forward * hands::CART_OFFSET, hands::CART_PUSH_RADIUS)
            } else {
                0.0
            };
            depth(p, PLAYER_RADIUS).max(cart)
        };
        let limit = worst(old) + 1e-4;
        if worst(pos) <= limit {
            return pos;
        }
        self.hint("Gang gesperrt: Wird befüllt.");
        for p in [vec3(pos.x, pos.y, old.z), vec3(old.x, pos.y, pos.z)] {
            if worst(p) <= limit {
                return p;
            }
        }
        old
    }

    /// Absperrband zeichnen: Pfosten und gelb-schwarzes Band an beiden Gangenden.
    pub(super) fn collect_closure_draws(&self, out: &mut Vec<MeshDraw>) {
        let Some(c) = self.closure.as_ref().filter(|c| c.taped()) else {
            return;
        };
        let post = Vec4::new(0.18, 0.18, 0.2, 0.0);
        for end in c.gate.ends {
            let lift = self.sway_at(end).y;
            let (x0, x1) = (end.x - c.gate.half, end.x + c.gate.half);
            for x in [x0, x1] {
                out.push(MeshDraw {
                    mesh: MeshId::Cube as usize,
                    model: Mat4::from_scale_rotation_translation(
                        vec3(0.05, 1.0, 0.05),
                        Quat::IDENTITY,
                        vec3(x, 0.5 + lift, end.z),
                    ),
                    tint: post,
                    ..Default::default()
                });
            }
            let segs = ((x1 - x0) / 0.25).ceil().max(1.0) as usize;
            for k in 0..segs {
                let x = x0 + (k as f32 + 0.5) * (x1 - x0) / segs as f32;
                let tint = if k % 2 == 0 {
                    Vec4::new(0.95, 0.8, 0.1, 0.15)
                } else {
                    Vec4::new(0.08, 0.08, 0.08, 0.1)
                };
                for y in [0.92f32, 0.55] {
                    out.push(MeshDraw {
                        mesh: MeshId::Cube as usize,
                        model: Mat4::from_scale_rotation_translation(
                            vec3((x1 - x0) / segs as f32, 0.06, 0.01),
                            Quat::IDENTITY,
                            vec3(x, y + lift, end.z),
                        ),
                        tint,
                        ..Default::default()
                    });
                }
            }
        }
    }
}
