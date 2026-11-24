//! Druck auf den Spieler: Dialoge mit Mitarbeitern, Auffaelligkeit, Durchsagen und Umbauten.

use super::*;

impl Game {
    pub(super) fn open_dialog(&mut self, employee: usize, question: usize, cart_talk: bool) {
        let mut order = [0usize, 1, 2];
        order.shuffle(&mut self.rng);
        self.employees[employee].state = EmpState::Talking;
        self.dialog = Some(DialogState {
            employee,
            question,
            order,
            timer: ANSWER_TIME,
            reply: None,
            chosen: None,
            cart_talk,
        });
        self.player.vel = Vec3::ZERO;
        self.sfx.push((Sfx::Blip, 0.6));
    }

    pub(super) fn update_dialog(&mut self, dt: f32, input: &Input) {
        let Some(d) = &mut self.dialog else { return };
        let emp_pos = self.employees[d.employee].pos;
        // Kamera zum Mitarbeiter drehen
        let to = emp_pos + vec3(0.0, 1.65, 0.0) - (self.player.pos + vec3(0.0, EYE_HEIGHT, 0.0));
        let yaw = to.x.atan2(-to.z);
        self.player.yaw = lerp_angle(self.player.yaw, yaw, (dt * 5.0).min(1.0));
        self.player.pitch +=
            ((to.y / to.length()).asin() - self.player.pitch) * (dt * 5.0).min(1.0);

        if let Some((_, t)) = &mut d.reply {
            *t -= dt;
            if *t <= 0.0 {
                let v = d.chosen.map(verdict).unwrap_or(Verdict::Bad);
                let who = d.employee;
                let e = &mut self.employees[d.employee];
                e.state = EmpState::Patrol;
                if d.cart_talk {
                    // Er laesst den Wagen dort stehen, wo er gerade ist
                    e.carrying = false;
                    e.cart_cd = 40.0;
                }
                // kurze Schonfrist, um wegzukommen
                self.grace = 2.5;
                match v {
                    Verdict::Good => {
                        e.ignore = 25.0;
                        e.follow = 0.0;
                    }
                    Verdict::Neutral => {
                        e.ignore = 12.0;
                    }
                    Verdict::Bad => {
                        e.ignore = 6.0;
                        e.follow = 20.0;
                    }
                }
                e.retarget = 0.0;
                self.dialog = None;
                // Folge fuer die Gaenge (closures.rs): gute Antwort macht einen gesperrten Gang
                // wieder auf, schlechte laesst den Gang der naechsten fehlenden Ware sperren
                match v {
                    Verdict::Good => {
                        if self.open_closure_soon() {
                            self.say("\u{201E}Ich mach kurz auf.\u{201C}", 2.5);
                        }
                    }
                    Verdict::Bad if self.tipped && !self.tutorial_run => {
                        if self.start_closure_except(true, None, Some(who)) {
                            self.say("Irgendwo wird ein Gang gesperrt.", 2.5);
                        }
                    }
                    _ => {}
                }
            }
            return;
        }
        d.timer -= dt;
        let choice = input.choice.filter(|&c| c < 3);
        if choice.is_none() && d.timer > 0.0 {
            return;
        }
        let (answer, reply, sus) = match choice {
            Some(k) => {
                let a = d.order[k];
                match verdict(a) {
                    Verdict::Good => (Some(a), dialog::REPLY_GOOD, 0.0),
                    Verdict::Neutral => (Some(a), dialog::REPLY_NEUTRAL, 0.08),
                    Verdict::Bad => (Some(a), dialog::REPLY_BAD, 0.3),
                }
            }
            None => (Some(2), dialog::REPLY_BAD, 0.2), // geschwiegen
        };
        d.chosen = answer;
        d.reply = Some((reply, dialog::REPLY_TIME));
        let silent = choice.is_none();
        let cart = d.cart_talk;
        self.suspicion += sus;
        self.sfx.push((Sfx::Blip, 0.6));
        // Folgen sichtbar machen
        let feedback = match answer.map(verdict) {
            _ if silent => "Du hast geschwiegen. Er folgt dir jetzt. (auffälliger)",
            Some(Verdict::Good) => "Er glaubt dir und lässt dich eine Weile in Ruhe.",
            Some(Verdict::Neutral) => "Er mustert dich. Du bist etwas auffälliger.",
            _ => "Er folgt dir jetzt. Du bist deutlich auffälliger.",
        };
        let feedback = if cart {
            format!("Er lässt deinen Wagen los. {feedback}")
        } else {
            feedback.to_string()
        };
        self.message = Some((feedback, 4.0));
        if sus > 0.1 {
            self.glitch = self.glitch.max(0.6);
            self.red = self.red.max(0.35);
            self.sfx.push((Sfx::Sting, 0.25));
        }
    }

    pub(super) fn update_suspicion(&mut self, dt: f32, _input: &Input) {
        if !self.tipped {
            return;
        }
        let before = self.suspicion;
        let eye = self.eye();
        let fwd = self.view_dir();
        let moving = self.player.vel.length() > 0.5;

        // Rennen in Sichtweite
        let watched = self
            .customers
            .iter()
            .any(|c| c.seen && (c.pos - self.player.pos).length() < 14.0)
            || self.employees.iter().any(|e| {
                (e.pos - self.player.pos).length() < 14.0
                    && self.world.line_of_sight(e.pos + vec3(0.0, 1.7, 0.0), eye)
            });
        if self.player.running && moving && watched {
            self.suspicion += 0.15 * dt;
            self.hint("Renn nicht. Du fällst auf.");
        }

        // Anstarren
        let mut staring = false;
        for c in &self.customers {
            let to = c.pos + vec3(0.0, 1.5, 0.0) - eye;
            let d = to.length();
            if c.seen && d < 5.0 && fwd.dot(to / d) > 0.994 {
                staring = true;
            }
        }
        if staring {
            self.stare_timer += dt;
            if self.stare_timer > 1.5 {
                self.suspicion += 0.12 * dt;
                self.hint("Starr sie nicht an.");
            }
        } else {
            self.stare_timer = 0.0;
        }

        // Anrempeln
        let mut bumped = false;
        for c in &mut self.customers {
            let d = vec3(
                c.pos.x - self.player.pos.x,
                0.0,
                c.pos.z - self.player.pos.z,
            )
            .length();
            if c.seen && d < 1.05 && c.bump_cd <= 0.0 {
                c.bump_cd = 2.5;
                bumped = true;
            }
        }
        if bumped {
            self.suspicion += 0.15;
            self.say("Du hast jemanden angerempelt.", 2.5);
        }

        if (self.suspicion - before).abs() < 1e-6 {
            self.suspicion -= 0.02 * dt;
        }
        self.suspicion = self.suspicion.clamp(0.0, 1.0);
        if self.suspicion >= 1.0 && before < 1.0 {
            self.say("Sie haben dich bemerkt.", 4.0);
            self.play(Sfx::Sting, 0.5);
        }
    }

    pub(super) fn update_events(&mut self, dt: f32) {
        if !self.tipped {
            return;
        }
        self.next_announce -= dt;
        if self.next_announce <= 0.0 {
            let (a, b) = self.tier.announce_interval();
            self.next_announce = self.rng.gen_range(a..b);
            let text = ANNOUNCEMENTS[self.rng.gen_range(0..ANNOUNCEMENTS.len())];
            self.announcement = Some((text.to_string(), 6.0));
            self.play(Sfx::Chime, 0.7);
        }
        // Ab "unruhig" baut sich der Markt immer wieder um (Gaenge, Durchgaenge, Waende,
        // Lichter) - aber nur, wenn man gerade nicht hinsieht (umdrehen, Boden/Decke
        // anstarren) oder viel zu lange wartet. Im Kollaps wandert auch die offene Kasse.
        self.turn_acc *= (-dt * 2.0).exp();
        if let Some(interval) = self.tier.rebuild_interval().filter(|_| !self.paid) {
            self.collapse_timer += dt;
            let looking_away = self.turn_acc > 1.6 || self.player.pitch.abs() > 0.7;
            if self.collapse_timer > interval
                && !self.umbau_running()
                && !self.checking_out()
                && !self.till_active()
                && (looking_away || self.collapse_timer > interval * 3.0)
            {
                self.collapse_timer = 0.0;
                self.params.shuffle = self.params.shuffle.wrapping_add(1);
                if self.tier == Tier::Collapse {
                    self.params.open_lane = self.rng.gen_range(0..4);
                }
                // im Dunkeln umbauen (umbau.rs), der Text kommt mit dem Licht zurueck
                self.start_umbau(Some("Etwas hat sich verschoben."));
            }
        }
    }
}
