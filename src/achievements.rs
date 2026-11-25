//! Erfolge (Beschreibung: `erfolge/erfolge.json`): beobachtet den laufenden Einkauf und meldet
//! dem Launcher. Wer im Einkauf einen Cheat benutzt, bekommt fuer diesen Einkauf nichts mehr.
//! Zaehler ueber mehrere Einkaeufe: `%APPDATA%\.mettlauncher\kasse0\erfolge.txt`.

use crate::game::escalation::Tier;
use crate::game::till::TILL_LIVES;
use crate::game::trip::Trip;
use crate::game::{Game, Outcome};
use mettgine::launcher::Achievements;
use mettgine::settings::{self, KeyValues};

const FILE: &str = "erfolge.txt";
/// Schneller Einkauf (Sekunden)
const QUICK: f32 = 10.0 * 60.0;

/// Meldung an den Launcher; `None` = freischalten, sonst der Stand.
type Report = (&'static str, Option<u32>);

/// Beobachter der Einkaeufe.
#[derive(Default)]
pub struct Tracker {
    wins: u32,
    /// dieser Einkauf ist schon abgerechnet
    over: bool,
    /// in `erfolge.txt` schreiben
    keep: bool,
}

impl Tracker {
    pub fn load() -> Self {
        let kv = settings::load(crate::settings::APP, FILE);
        Self {
            wins: kv.u32("wins", 0, 0, u32::MAX),
            over: false,
            keep: true,
        }
    }

    /// Stand aus frueheren Einkaeufen melden (einmal nach dem Start), auch wenn damals kein
    /// Launcher lief.
    pub fn report_totals(&self, a: &Achievements) {
        send(a, &self.totals());
    }

    fn totals(&self) -> Vec<Report> {
        if self.wins == 0 {
            return Vec::new();
        }
        vec![("kassenbon", None), ("stammkunde", Some(self.wins))]
    }

    /// Neuer Einkauf beginnt.
    pub fn start(&mut self) {
        self.over = false;
    }

    /// Jedes Bild im Spiel.
    pub fn step(&mut self, g: &Game, a: &Achievements) {
        send(a, &self.observe(g));
    }

    fn observe(&mut self, g: &Game) -> Vec<Report> {
        let mut out = Vec::new();
        if g.cheated {
            return out;
        }
        if g.tutorial_run {
            // Das Tutorial zaehlt nur fuer sich
            if g.outcome == Outcome::Won {
                out.push(("ausgelernt", None));
            }
            return out;
        }
        let now = [
            (g.tipped, "irgendwas_ist_anders"),
            (g.trip != Trip::None, "nicht_dein_saft"),
            (!g.butts.is_empty(), "raucherpause"),
            (g.sign_falls.iter().any(Option::is_some), "frisch_gewischt"),
            (g.tipped && g.world.in_wing(g.player.pos), "hinter_der_wand"),
            (g.tier == Tier::Collapse, "grosseinkauf"),
            (g.paid, "gesicht_gezeigt"),
            (g.turned_back, "nicht_umdrehen"),
            (g.till_caught, "durchschaut"),
        ];
        out.extend(now.iter().filter(|n| n.0).map(|n| (n.1, None)));
        if g.outcome != Outcome::Running && !std::mem::replace(&mut self.over, true) {
            if g.outcome == Outcome::Won {
                self.won(g, &mut out);
            } else {
                out.push(("nicht_rausgekommen", None));
            }
        }
        out
    }

    fn won(&mut self, g: &Game, out: &mut Vec<Report>) {
        self.wins = self.wins.saturating_add(1);
        if self.keep {
            let mut kv = KeyValues::default();
            kv.set("wins", self.wins);
            settings::save(crate::settings::APP, FILE, &kv);
        }
        out.extend(self.totals());
        if g.deaths == 0 {
            out.push(("unauffaellig", None));
        }
        if g.run_time < QUICK {
            out.push(("schnell_rein_schnell_raus", None));
        }
        if let Some(t) = &g.till {
            if t.lives == TILL_LIVES {
                out.push(("perfekte_kundschaft", None));
            }
            if t.cash {
                out.push(("stimmt_so", None));
            }
        }
    }
}

fn send(a: &Achievements, reports: &[Report]) {
    for (id, value) in reports {
        match value {
            None => a.unlock(id),
            Some(v) => a.progress(id, *v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = include_str!("../erfolge/erfolge.json");

    fn has(out: &[Report], id: &str) -> bool {
        assert!(
            JSON.contains(&format!("\"id\": \"{id}\"")),
            "{id} fehlt in erfolge.json"
        );
        out.iter().any(|r| r.0 == id)
    }

    #[test]
    fn winning_counts_once_per_shopping_trip() {
        let (mut t, mut g) = (Tracker::default(), Game::new(1));
        assert!(t.observe(&g).is_empty());
        g.outcome = Outcome::Won;
        g.run_time = 60.0;
        let out = t.observe(&g);
        assert!(has(&out, "kassenbon") && has(&out, "stammkunde"));
        assert!(has(&out, "unauffaellig") && has(&out, "schnell_rein_schnell_raus"));
        assert!(t.observe(&g).is_empty());
        t.start();
        g.deaths = 1;
        g.run_time = QUICK;
        let out = t.observe(&g);
        assert!(out.contains(&("stammkunde", Some(2))));
        assert!(!has(&out, "unauffaellig") && !has(&out, "schnell_rein_schnell_raus"));
    }

    #[test]
    fn a_cheat_blocks_the_whole_trip_but_not_the_next_one() {
        let (mut t, mut g) = (Tracker::default(), Game::new(1));
        g.cheat(0); // Unverwundbar an ...
        g.cheat(0); // ... und wieder aus: zaehlt trotzdem
        g.tipped = true;
        g.outcome = Outcome::Won;
        assert!(t.observe(&g).is_empty() && t.wins == 0);
        let mut next = Game::new(1);
        t.start();
        next.outcome = Outcome::Won;
        assert!(has(&t.observe(&next), "kassenbon"));
    }

    #[test]
    fn every_cheat_counts() {
        for i in 0..crate::game::CHEATS.len() {
            let mut g = Game::new(2);
            g.cheat(i);
            assert!(g.cheated, "{}", crate::game::CHEATS[i]);
        }
    }

    #[test]
    fn events_and_ids() {
        let (mut t, mut g) = (Tracker::default(), Game::new(1));
        g.tipped = true;
        g.tier = Tier::Collapse;
        g.paid = true;
        g.turned_back = true;
        g.till_caught = true;
        g.trip = Trip::Drinking { t: 0.0 };
        g.outcome = Outcome::Caught;
        let out = t.observe(&g);
        for id in [
            "irgendwas_ist_anders",
            "grosseinkauf",
            "gesicht_gezeigt",
            "nicht_umdrehen",
            "durchschaut",
            "nicht_dein_saft",
            "nicht_rausgekommen",
        ] {
            assert!(has(&out, id), "{id}");
        }
        for id in [
            "raucherpause",
            "frisch_gewischt",
            "hinter_der_wand",
            "perfekte_kundschaft",
            "stimmt_so",
        ] {
            assert!(!has(&out, id), "{id}");
        }
    }

    #[test]
    fn the_tutorial_only_counts_for_itself() {
        let (mut t, mut g) = (Tracker::default(), Game::new_tutorial(1));
        g.tipped = true;
        g.outcome = Outcome::Won;
        let out = t.observe(&g);
        assert!(has(&out, "ausgelernt") && out.len() == 1 && t.wins == 0);
    }
}
