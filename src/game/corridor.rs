//! Der Fluchtkorridor hinter der Ladentuer (Geometrie: world/gen/corridor.rs).
//! Nach dem Bezahlen rennen alle hinter einem her. Im Gang gilt: NICHT UMDREHEN. Wer zurueck-
//! schaut, ist sofort tot. Was hinter einem passiert, sieht man nur im Spiegel am Ende - dafuer
//! filmt die zweite Kamera (sonst der Ueberwachungsmonitor) vom gespiegelten Auge aus.

use super::*;

/// Ab hier (Kosinus zwischen Blick und Gangrichtung) warnt das Spiel: "NICHT UMDREHEN"
pub const TURN_WARN: f32 = 0.2;
/// Ab hier hat man sich umgedreht (gut 110 Grad): sofort Game Over
pub const TURN_DEAD: f32 = -0.35;
/// So schnell sind die Verfolger im engen Gang (im Markt: escape::CHASE_SPEED)
pub const CORRIDOR_CHASE_SPEED: f32 = 1.8;
/// Umdreh-Schreck: so lange ist das Gesicht im Bild (danach Abblende zum Game Over)
pub const TURN_FACE_TIME: f32 = 1.4;

impl Game {
    /// Steht man im Gang (ein Stueck hinter der Tuer)?
    pub fn in_corridor(&self) -> bool {
        self.world
            .corridor
            .is_some_and(|c| c.contains(self.player.pos) && self.player.pos.z > c.z0 + 0.6)
    }

    /// Wie weit schaut man gerade nach vorn (1) bzw. zurueck (-1)? Gangrichtung ist +z.
    pub fn corridor_facing(&self) -> f32 {
        let f = self.view_dir();
        vec3(f.x, 0.0, f.z).normalize_or_zero().z
    }

    /// Umdreh-Warnung 0..1 fuer das HUD (0 = schaut nach vorn).
    pub fn turn_warning(&self) -> f32 {
        if !(self.chasing() && self.in_corridor()) {
            return 0.0;
        }
        ((TURN_WARN - self.corridor_facing()) / (TURN_WARN - TURN_DEAD)).clamp(0.0, 1.0)
    }

    /// Im Gang: Hinweis beim Betreten, Umdrehen = sofort tot.
    pub(super) fn update_corridor(&mut self) {
        if !self.chasing()
            || !self.in_corridor()
            || self.world.exit_zone.contains_xz(self.player.pos)
        {
            return;
        }
        if !self.corridor_entered {
            self.corridor_entered = true;
            self.say("Lauf. Und dreh dich NICHT um.", 4.0);
            self.play(Sfx::Sting, 0.4);
        }
        let facing = self.corridor_facing();
        let turned = facing < TURN_DEAD;
        if self.tutorial_run {
            // Tutorial: warnen statt Game Over, einmal je Umdrehen
            let warn = match &mut self.tutorial {
                Some(t) if turned && !t.turn_warned => {
                    t.turn_warned = true;
                    true
                }
                Some(t) => {
                    if facing > 0.0 {
                        t.turn_warned = false;
                    }
                    false
                }
                None => false,
            };
            if warn {
                self.tutorial_forgive(
                    "Du hast dich umgedreht! Im echten Spiel ist das Game Over. Weiter!",
                );
            }
            return;
        }
        if turned && !self.god {
            self.turn_scare();
        }
    }

    /// Umgedreht: Jumpscare wie die anderen Schrecken (scares.rs) - Schrei, Farbumkehr-
    /// Stroboskop, Lichtblitz, Zoom-Ruck - und das leere Gesicht des naechsten Verfolgers
    /// rast zitternd auf die Kamera zu (scares.rs, collect_scare_people). Danach Game Over.
    fn turn_scare(&mut self) {
        self.turned_back = true;
        self.outcome = Outcome::Caught;
        self.outcome_timer = 0.0;
        self.red = 1.0;
        self.glitch = 1.0;
        self.fire_scare(1.0);
        self.play(Sfx::Shriek, 1.3);
        self.play(Sfx::Sting, 1.0);
        // der naechste Verfolger ist es, der da steht
        let p = self.player.pos;
        let near = self
            .customers
            .iter()
            .map(|c| (c.pos, c.variant))
            .chain(self.employees.iter().map(|e| (e.pos, e.variant)))
            .min_by(|a, b| (a.0 - p).length().total_cmp(&(b.0 - p).length()));
        let staff = crate::people::people_of(true);
        self.catcher_variant = Some(near.map_or(staff[0], |n| n.1));
        // die Kamera bleibt auf ihn gerichtet
        let f = self.view_dir();
        self.catcher = Some(p + vec3(f.x, 0.0, f.z).normalize_or_zero() * 0.6);
    }

    /// Umdreh-Schreck: das Gesicht vor der Kamera (Abstand, Zittern), None = vorbei/kein Schreck.
    pub fn turn_scare_face(&self) -> Option<(usize, f32, Vec3)> {
        if !self.turned_back || self.outcome != Outcome::Caught {
            return None;
        }
        let t = self.outcome_timer;
        if t >= TURN_FACE_TIME {
            return None;
        }
        let v = self.catcher_variant?;
        // in 0,3 s von 1,4 m auf 0,32 m heran, dann haelt es zitternd still
        let k = (t / 0.3).min(1.0);
        let dist = 1.4 - (1.4 - 0.32) * k * k;
        let s = self.time;
        let shake = vec3((s * 61.0).sin(), (s * 47.0).sin(), 0.0) * (0.012 + 0.02 * (1.0 - k));
        Some((v, dist, shake))
    }

    /// Cheat/Test: bezahlt, Alarm laeuft, man steht kurz hinter der Tuer im Gang und schaut
    /// nach vorn. Die Verfolger starten wie beim echten Alarm im Markt.
    pub fn debug_jump_to_corridor(&mut self) {
        if !self.paid {
            self.tip();
            self.fill_list_cheap();
            self.scanned.append(&mut self.cart);
            self.belt.clear();
            self.held = None;
            self.dialog = None;
            self.pay();
        }
        let Some(c) = self.world.corridor else { return };
        self.player.pos = vec3((c.x0 + c.x1) * 0.5, 0.0, c.z0 + 1.5);
        self.player.yaw = std::f32::consts::PI;
        self.player.pitch = 0.0;
        self.player.vel = Vec3::ZERO;
        self.trolley.pushing = false;
        self.field_timer = 0.0;
    }

    /// Kamera fuers Spiegelbild: vom am Spiegel gespiegelten Auge aus, Bildausschnitt genau
    /// die Spiegelflaeche (schiefes Frustum). None = nicht im Gang.
    pub fn mirror_view(&self) -> Option<(Mat4, Vec3)> {
        let c = self.world.corridor?;
        if !c.contains(self.player.pos) {
            return None;
        }
        let eye = self.eye();
        let zm = c.mirror_center.z;
        let n = (zm - eye.z).max(0.05);
        let cam = vec3(eye.x, eye.y, zm + n);
        let view = Mat4::look_to_rh(cam, -Vec3::Z, Vec3::Y);
        let (mn, mx) = (
            c.mirror_center - c.mirror_half,
            c.mirror_center + c.mirror_half,
        );
        let (l, r) = (mn.x - cam.x, mx.x - cam.x);
        let (b, t) = (mn.y - cam.y, mx.y - cam.y);
        // weit genug, dass der ganze Gang (doppelter Abstand) ins Bild passt
        let far = 2.0 * n + 50.0;
        let proj = Mat4::from_cols(
            Vec4::new(2.0 * n / (r - l), 0.0, 0.0, 0.0),
            Vec4::new(0.0, 2.0 * n / (t - b), 0.0, 0.0),
            Vec4::new((r + l) / (r - l), (t + b) / (t - b), far / (n - far), -1.0),
            Vec4::new(0.0, 0.0, n * far / (n - far), 0.0),
        );
        Some((proj * view, cam))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bezahlt, Alarm laeuft, Spieler steht im Gang und schaut nach vorn (+z).
    fn in_corridor(seed: u64) -> Game {
        let mut g = Game::new(seed);
        g.tip();
        g.fill_list_cheap();
        g.scanned.append(&mut g.cart);
        g.pay();
        let c = g.world.corridor.expect("kein Korridor nach dem Bezahlen");
        g.customers.clear();
        g.employees.clear();
        g.player.pos = vec3((c.x0 + c.x1) * 0.5, 0.0, c.z0 + 3.0);
        g.player.yaw = std::f32::consts::PI; // Blick nach +z
        g.player.pitch = 0.0;
        g.update(1.0 / 60.0, &mut Input::default());
        g
    }

    #[test]
    fn paying_opens_a_long_corridor_with_mirror_and_exit() {
        let g = in_corridor(81);
        let c = g.world.corridor.unwrap();
        assert!(c.z1 - c.z0 > 30.0, "Gang zu kurz");
        assert!(g.in_corridor());
        assert!(g.chasing());
        // Spiegel am Ende, Ausgang kurz davor, aber nicht gleich hinter der Tuer
        assert!(c.mirror_center.z > c.z1 - 0.2);
        assert!(g.world.exit_zone.min.z > c.z0 + 30.0);
        assert!(!g.world.exit_zone.contains_xz(g.player.pos));
        // Navigationsraster reicht bis ans Ende
        let end = vec3((c.x0 + c.x1) * 0.5, 0.0, c.z1 - 1.0);
        assert!(npc::nearest_walkable(&g.world.nav, end).is_some());
    }

    #[test]
    fn corridor_is_always_passable_and_different_every_run() {
        // gleicher Seed, mehrmals bezahlt: die Hindernisse liegen jedes Mal anders,
        // aber der Weg zum Ausgang ist immer frei
        let mut layouts = std::collections::HashSet::new();
        for _ in 0..12 {
            let g = in_corridor(82);
            let exit = g.world.exit_zone;
            let goal = vec3(
                (exit.min.x + exit.max.x) * 0.5,
                0.0,
                (exit.min.z + exit.max.z) * 0.5,
            );
            let field = npc::bfs(&g.world.nav, goal);
            let d = npc::field_at(&g.world.nav, &field, g.player.pos);
            assert!(
                d != npc::UNREACHABLE,
                "Ausgang vom Gang aus nicht erreichbar"
            );
            let c = g.world.corridor.unwrap();
            let key: Vec<i32> = g
                .world
                .solids
                .iter()
                .filter(|s| s.min.z > c.z0 + 1.0 && s.max.z < c.z1 - 3.0 && s.max.y < 2.0)
                .map(|s| (s.min.x * 10.0) as i32 + (s.min.z * 10.0) as i32 * 1000)
                .collect();
            assert!(!key.is_empty(), "keine Hindernisse");
            layouts.insert(key);
        }
        assert!(
            layouts.len() > 8,
            "Hindernisse kaum zufaellig: {}",
            layouts.len()
        );
    }

    #[test]
    fn tutorial_turning_around_only_warns() {
        let mut g = in_corridor(84);
        g.tutorial_run = true;
        g.tutorial = Some(crate::game::tutorial::Tutorial::default());
        g.player.yaw = 0.0; // Blick zurueck
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(g.outcome, Outcome::Running, "im Tutorial Game Over");
        assert!(!g.turned_back);
        assert!(g.tutorial.as_ref().unwrap().turn_warned);
        // wieder nach vorn: die Warnung kann erneut kommen
        g.player.yaw = std::f32::consts::PI;
        g.update(1.0 / 60.0, &mut Input::default());
        assert!(!g.tutorial.as_ref().unwrap().turn_warned);
    }

    #[test]
    fn turning_around_in_the_corridor_is_instant_game_over() {
        let mut g = in_corridor(83);
        // seitlich schauen ist erlaubt (Warnung), umdrehen nicht
        g.player.yaw = std::f32::consts::PI * 0.5;
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(
            g.outcome,
            Outcome::Running,
            "zur Seite schauen ist schon tot"
        );
        assert!(g.turn_warning() > 0.0, "keine Warnung beim Seitenblick");
        g.player.yaw = 0.0; // Blick zurueck (-z)
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(g.outcome, Outcome::Caught);
        assert!(g.turned_back);
    }

    #[test]
    fn turning_around_triggers_a_jumpscare_face() {
        let mut g = in_corridor(87);
        g.player.yaw = 0.0;
        g.update(1.0 / 60.0, &mut Input::default());
        assert!(g.turned_back);
        // Schreck-Effekt laeuft (Farbumkehr/Blitz/Zoom wie bei den anderen Schrecken)
        assert!(
            g.scare_flash() > 0.0 && g.scare_zoom() < 1.0,
            "kein Schreck-Effekt"
        );
        // das Gesicht rast heran ...
        let far = g.turn_scare_face().expect("kein Gesicht").1;
        for _ in 0..20 {
            g.update(1.0 / 60.0, &mut Input::default());
        }
        let near = g.turn_scare_face().expect("Gesicht zu frueh weg").1;
        assert!(
            near < far && near < 0.5,
            "Gesicht kommt nicht heran: {far} -> {near}"
        );
        let mut people = Vec::new();
        g.collect_people(&mut people);
        let eye = g.eye();
        assert!(
            people
                .iter()
                .any(|p| (p.pos - eye).length() < 2.0 && p.pos.y > eye.y - 2.0),
            "Gesicht wird nicht gezeichnet"
        );
        // ... und ist nach dem Schreck wieder weg, der Effekt klingt ab
        for _ in 0..(TURN_FACE_TIME * 60.0) as usize {
            g.update(1.0 / 60.0, &mut Input::default());
        }
        assert!(g.turn_scare_face().is_none());
        assert_eq!(g.scare_flash(), 0.0, "Schreck-Effekt hoert nicht auf");
    }

    #[test]
    fn cheat_jumps_into_the_corridor() {
        let mut g = Game::new(86);
        let i = CHEATS.iter().position(|c| c.starts_with("Flucht")).unwrap();
        g.cheat(i);
        g.update(1.0 / 60.0, &mut Input::default());
        assert!(g.paid && g.chasing());
        assert!(g.in_corridor(), "Cheat setzt nicht in den Korridor");
        assert_eq!(g.outcome, Outcome::Running);
    }

    #[test]
    fn reaching_the_exit_wins() {
        let mut g = in_corridor(84);
        let exit = g.world.exit_zone;
        g.player.pos = vec3(exit.min.x + 0.3, 0.0, (exit.min.z + exit.max.z) * 0.5);
        g.update(1.0 / 60.0, &mut Input::default());
        assert_eq!(g.outcome, Outcome::Won);
    }

    #[test]
    fn mirror_camera_sits_behind_the_mirror_and_looks_back() {
        let g = in_corridor(85);
        let c = g.world.corridor.unwrap();
        let (vp, cam) = g.mirror_view().expect("kein Spiegelbild im Gang");
        assert!(cam.z > c.mirror_center.z, "Kamera nicht hinter dem Spiegel");
        // die Ecken des Spiegels landen auf den Bildecken
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
            let p = c.mirror_center + vec3(sx * c.mirror_half.x, sy * c.mirror_half.y, 0.0);
            let q = vp * p.extend(1.0);
            let (x, y) = (q.x / q.w, q.y / q.w);
            assert!(
                (x.abs() - 1.0).abs() < 1e-3 && (y.abs() - 1.0).abs() < 1e-3,
                "{x} {y}"
            );
        }
        // der Spieler ist im Bild (er steht vor dem Spiegel)
        let q = vp * (g.player.pos + vec3(0.0, 1.0, 0.0)).extend(1.0);
        assert!(q.w > 0.0 && (q.x / q.w).abs() < 1.0 && (q.y / q.w).abs() < 1.0);
        // im Markt gibt es kein Spiegelbild
        assert!(Game::new(85).mirror_view().is_none());
    }
}
