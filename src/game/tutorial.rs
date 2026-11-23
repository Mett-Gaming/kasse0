//! Tutorial-Modus (eigener Punkt im Hauptmenue): fuehrt durch einen ganzen Einkauf.
//! Phase 1 zeigt alle Tasten und hakt sie ab, danach erklaert ein Kasten die jeweilige Phase
//! (aus dem Spielzustand abgeleitet), Pfeile am Boden zeigen den Weg und benoetigte Waren
//! leuchten (guide.rs, wie die Hervorhebung in highlight.rs). Im Tutorial kann man nicht verlieren: Fehler werden nur
//! erklaert (`Game::tutorial_forgive`). Im normalen Einkauf gibt es nichts davon.

use super::*;

pub mod guide;

/// (Taste, Erklaerung)
pub const STEPS: &[(&str, &str)] = &[
    ("Maus", "Umsehen"),
    ("W A S D", "Gehen"),
    ("TAB halten", "Einkaufszettel"),
    ("Q", "Wagen abstellen/schieben"),
    ("E / Klick", "Ware greifen (Wasser!)"),
    ("E auf Wagen", "In den Wagen legen"),
    ("F am Wagen", "Ware herausnehmen"),
    ("Shift / Esc", "Rennen (auffällig)/Pause"),
];

/// So lange gibt es nach einem verziehenen Fehler eine Schonfrist (Sekunden)
const FORGIVE_GRACE: f32 = 4.0;

#[derive(Default)]
pub struct Tutorial {
    pub step: usize,
    pub looked: f32,
    pub walked: f32,
    pub listed: bool,
    pub toggles: u32,
    pub took_out: bool,
    pub timer: f32,
    /// Im Korridor umgedreht und schon gewarnt (erst nach vorn schauen, dann wieder)
    pub turn_warned: bool,
    /// Wegweiser: Pfeile am Boden und leuchtende Waren
    pub guide: guide::Guide,
}

impl Tutorial {
    /// Prueft den aktuellen Schritt. Gibt `true` zurueck, wenn das Tutorial fertig ist.
    pub fn advance(&mut self, dt: f32, holding: bool, cart_items: usize) -> bool {
        let done = match self.step {
            0 => self.looked > 2.0,
            1 => self.walked > 3.0,
            2 => self.listed,
            3 => self.toggles >= 2,
            4 => holding,
            5 => cart_items > 0,
            6 => self.took_out,
            _ => {
                self.timer += dt;
                self.timer > 6.0
            }
        };
        if done {
            self.step += 1;
            self.timer = 0.0;
        }
        self.keys_done()
    }

    pub fn keys_done(&self) -> bool {
        self.step >= STEPS.len()
    }
}

/// Abschnitte des Tutorials (aus dem Spielzustand abgeleitet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Controls,
    Water,
    Shopping,
    ToTill,
    Belt,
    Cashier,
    Pay,
    Escape,
}

pub const PHASES: usize = 8;

impl Phase {
    pub fn number(self) -> usize {
        self as usize + 1
    }

    pub fn title(self) -> &'static str {
        match self {
            Phase::Controls => "STEUERUNG",
            Phase::Water => "WASSER",
            Phase::Shopping => "EINKAUFEN",
            Phase::ToTill => "ZUR KASSE",
            Phase::Belt => "BAND",
            Phase::Cashier => "KASSIERERIN",
            Phase::Pay => "BEZAHLEN",
            Phase::Escape => "FLUCHT",
        }
    }

    /// Erklaerung, ggf. auf mehrere Seiten verteilt, die sich abwechseln (PAGE_TIME).
    /// Hoechstens 5 Zeilen je Seite, 36 Zeichen je Zeile (passt in den Kasten).
    pub fn pages(self) -> &'static [&'static [&'static str]] {
        match self {
            Phase::Controls => &[],
            Phase::Water => &[&[
                "Leg das Wasser in den Wagen (E).",
                "Dann raus aus der Getränkeabteilung.",
                "Folge den Pfeilen am Boden.",
            ]],
            Phase::Shopping => &[
                &[
                    "Der Markt ist gekippt. Kauf, was",
                    "auf dem Zettel steht (TAB): es",
                    "leuchtet. Falsch Beschriftetes",
                    "liegen lassen, nicht rennen.",
                ],
                &[
                    "KUNDEN kommen im echten Spiel näher,",
                    "wenn du nicht hinsiehst. Nicht",
                    "anrempeln, nicht anstarren, ihren",
                    "Tomatensaft nicht nehmen.",
                    "Im Tutorial verfolgt dich keiner.",
                ],
                &[
                    "MITARBEITER (ohne Gesicht) sprechen",
                    "dich im echten Spiel an: Antwort",
                    "mit 1/2/3. Gute Antworten bringen",
                    "Abstand, falsche machen auffällig.",
                    "Im Tutorial lassen sie dich in Ruhe.",
                ],
            ],
            Phase::ToTill => &[&[
                "Zettel komplett! Schieb den Wagen",
                "zur grünen Kasse, E am Band.",
            ]],
            Phase::Belt => &[&[
                "Zieh die Ware mit der Maus aufs",
                "Band. Alles ab 1 kg zuerst (rot).",
            ]],
            Phase::Cashier => &[&[
                "Fragen mit 1/2/3 beantworten.",
                "Schaut sie auf: still stehen und",
                "wegsehen. Bon prüfen, R reklamiert.",
            ]],
            Phase::Pay => &[&[
                "E am Kartenterminal. Karte ruhig",
                "durchziehen, PIN tippen. Bar:",
                "genau passend zahlen.",
            ]],
            Phase::Escape => &[&[
                "Alarm! Renn zum Ausgang (Pfeile).",
                "Im echten Spiel rennen alle auf dich",
                "zu. Im Korridor NICHT umdrehen.",
                "(Im Tutorial verfolgt dich keiner.)",
            ]],
        }
    }

    /// Seite, die gerade zu sehen ist (wechselt alle PAGE_TIME Sekunden).
    pub fn page(self, time: f32) -> (usize, &'static [&'static str]) {
        let p = self.pages();
        if p.is_empty() {
            return (0, &[]);
        }
        let k = (time / PAGE_TIME) as usize % p.len();
        (k, p[k])
    }
}

/// So lange bleibt eine Seite des Erklaer-Kastens stehen (Sekunden)
pub const PAGE_TIME: f32 = 8.0;

/// Erklaerleiste oben links ueber den Kassen-Fenstern (nur im Tutorial): Hinweise wechseln
/// alle HINT_TIME Sekunden, je hoechstens HINT_MAX Zeichen.
pub const HINT_TIME: f32 = 5.0;
pub const HINT_MAX: usize = 45;
pub const BELT_HINTS: &[&str] = &[
    "Ware mit gedrückter Maus aufs Band ziehen.",
    "Alles ab 1 kg zuerst aufs Band (rot).",
    "Roter Rahmen: sie schaut auf, Scanner ruht.",
    "Falscher Preis auf dem Bon? R reklamiert.",
    "Danach: Fragen mit 1/2/3, E am Terminal.",
];
pub const CARD_HINTS: &[&str] = &[
    "Erst E: Karte aus dem Portemonnaie nehmen.",
    "Dann die Maus gleichmäßig nach rechts.",
    "Nicht zu schnell und nicht zu langsam.",
    "Danach PIN merken und mit 0-9 eintippen.",
];
pub const CASH_HINTS: &[&str] = &[
    "Genau passend zahlen, nicht zu viel.",
    "A/D oder Mausrad wählt Schein oder Münze.",
    "E legt es hin, F nimmt es zurück.",
    "Beeil dich: der Balken zeigt die Restzeit.",
];

/// Welche Erklaerleiste gerade passt: Kassenfenster, Karte/PIN oder Bargeld.
pub fn ui_hints(g: &Game) -> Option<&'static [&'static str]> {
    if !g.tutorial_run {
        return None;
    }
    if g.till_paying() {
        return Some(match g.till.as_ref().map(|t| &t.pay) {
            Some(till::Pay::Cash(_)) => CASH_HINTS,
            _ => CARD_HINTS,
        });
    }
    g.belt_window_open().then_some(BELT_HINTS)
}

/// In welcher Phase ist man gerade? (None = kein Tutorial)
pub fn phase(g: &Game) -> Option<Phase> {
    let t = g.tutorial.as_ref()?;
    Some(if g.paid {
        Phase::Escape
    } else if g.till.is_some() {
        if g.till_paying() || g.scanned_complete() {
            Phase::Pay
        } else if !g.cart.is_empty() || g.held.is_some() || g.belt_window_open() {
            Phase::Belt
        } else {
            Phase::Cashier
        }
    } else if g.tipped {
        if g.list_complete() {
            Phase::ToTill
        } else {
            Phase::Shopping
        }
    } else if t.keys_done() {
        Phase::Water
    } else {
        Phase::Controls
    })
}

impl Game {
    /// Neuer Einkauf im Tutorial-Modus.
    pub fn new_tutorial(seed: u64) -> Self {
        let mut g = Self::new(seed);
        g.tutorial = Some(Tutorial::default());
        g.tutorial_run = true;
        g
    }

    /// Hat der Cheat "Tutorial starten" einen neuen Tutorial-Einkauf angefordert?
    /// (setzt die Anfrage zurueck; main.rs startet dann neu)
    pub fn take_tutorial_request(&mut self) -> bool {
        std::mem::take(&mut self.tutorial_request)
    }

    pub fn tutorial_phase(&self) -> Option<Phase> {
        phase(self)
    }

    /// Im Tutorial: Fehler nur erklaeren statt bestrafen (kurz rot, Ton, Schonfrist).
    pub(super) fn tutorial_forgive(&mut self, msg: &str) {
        self.red = self.red.max(0.8);
        self.glitch = self.glitch.max(0.5);
        self.play(Sfx::Sting, 0.7);
        self.say(msg, 4.5);
        self.grace = self.grace.max(FORGIVE_GRACE);
    }

    /// Tasten abhaken (nur im normalen Markt) und den Wegweiser nachfuehren.
    pub(super) fn update_tutorial(&mut self, dt: f32, input: &Input) {
        let tipped = self.tipped;
        let holding = self.held.is_some();
        let cart = self.cart.len();
        let Some(t) = &mut self.tutorial else { return };
        t.listed |= input.list_open;
        if !tipped && !t.keys_done() && t.advance(dt, holding, cart) {
            self.say(
                "Gut. Jetzt das Wasser in den Wagen und raus aus der Getränkeabteilung.",
                5.0,
            );
        }
        self.update_guide(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_advance_in_order() {
        let mut t = Tutorial::default();
        assert!(!t.advance(0.1, false, 0));
        assert_eq!(t.step, 0);
        t.looked = 3.0;
        t.advance(0.1, false, 0);
        assert_eq!(t.step, 1);
        t.walked = 5.0;
        t.listed = true;
        t.toggles = 2;
        t.advance(0.1, false, 0);
        t.advance(0.1, false, 0);
        t.advance(0.1, false, 0);
        assert_eq!(t.step, 4);
        t.advance(0.1, true, 0);
        t.advance(0.1, false, 1);
        t.took_out = true;
        t.advance(0.1, false, 1);
        assert_eq!(t.step, 7);
        assert!(!t.advance(1.0, false, 1));
        assert!(t.advance(6.0, false, 1));
    }

    #[test]
    fn normal_run_has_no_tutorial_window() {
        let g = Game::new(59);
        assert!(g.tutorial.is_none() && !g.tutorial_run);
        let t = Game::new_tutorial(59);
        assert!(t.tutorial.is_some() && t.tutorial_run);
        assert_eq!(t.tutorial_phase(), Some(Phase::Controls));
    }

    #[test]
    fn phase_follows_the_game() {
        let mut g = Game::new_tutorial(60);
        g.customers.clear();
        g.employees.clear();
        g.tutorial.as_mut().unwrap().step = STEPS.len();
        assert_eq!(g.tutorial_phase(), Some(Phase::Water));
        g.tip();
        assert!(g.tutorial.is_some(), "Kippen beendet das Tutorial");
        assert_eq!(g.tutorial_phase(), Some(Phase::Shopping));
        g.fill_list_cheap();
        assert_eq!(g.tutorial_phase(), Some(Phase::ToTill));
        // Ware auf dem Band: Band, alles gescannt: Bezahlen, bezahlt: Flucht
        let item = g.cart.remove(0);
        g.put_on_belt(item);
        assert_eq!(g.tutorial_phase(), Some(Phase::Belt));
        g.cart.clear();
        assert_eq!(g.tutorial_phase(), Some(Phase::Cashier));
        g.paid = true;
        assert_eq!(g.tutorial_phase(), Some(Phase::Escape));
    }

    #[test]
    fn tutorial_run_cannot_be_lost_to_customers() {
        let mut g = Game::new_tutorial(58);
        g.tip();
        g.customers.clear();
        g.employees.clear();
        g.spawn_customer_at(g.player.pos + vec3(0.3, 0.0, 0.0));
        for _ in 0..900 {
            g.update(1.0 / 60.0, &mut Input::default());
        }
        assert_eq!(g.outcome, Outcome::Running);
        assert_eq!(g.deaths, 0);
        assert!(g.dying.is_none());
        // greift gar nicht erst zu (kein verziehener Fang, keine Meldung)
        assert!(
            !g.message
                .as_ref()
                .is_some_and(|(m, _)| m.contains("Erwischt")),
            "Kunde greift im Tutorial zu"
        );
    }

    /// Spieler schaut weg, ist aufgefallen (alle wissen, wo er ist), ein Verfolger 5 m hinter
    /// ihm, ein Mitarbeiter, der ihm folgen will. Jagt der Kunde, wie nah kommt er in 6 s, und
    /// laeuft der Mitarbeiter gezielt auf einen zu oder spricht einen an?
    /// (Auf seiner zufaelligen Streife darf er einem naeher kommen - das ist kein Folgen.)
    fn pursuit(tutorial: bool, paid: bool) -> (bool, f32, bool) {
        let mut g = if tutorial {
            Game::new_tutorial(63)
        } else {
            Game::new(63)
        };
        g.tip();
        if paid {
            g.fill_list_cheap();
            g.scanned.append(&mut g.cart);
            let lane = g.open_lane().unwrap();
            g.player.pos = lane.scan_point;
            g.pay();
        }
        g.customers.clear();
        g.employees.clear();
        g.player.pos = g.world.spawn - vec3(0.0, 0.0, 3.0);
        g.god = true; // nur die Verfolgung messen, nicht das Fangen
        let p = g.player.pos;
        g.spawn_customer_at(p + vec3(0.0, 0.0, 5.0));
        g.customers[0].stalker = true;
        g.spawn_employees(1);
        g.employees[0].pos = g.world.collide(p + vec3(4.0, 0.0, 0.0), 0.25);
        g.employees[0].follow = 30.0;
        g.employees[0].ignore = 0.0;
        g.player.yaw = 0.0; // Blick nach -z, Verfolger hinter einem
        g.suspicion = 1.0;
        let c0 = (g.customers[0].pos - p).length();
        let mut hunted = false;
        let mut approached = false;
        for k in 0..360 {
            g.suspicion = 1.0;
            g.player.pos = p;
            g.update(1.0 / 60.0, &mut Input::default());
            hunted |= g.customers.iter().any(|c| c.brain.hunting);
            approached |= k > 0
                && g.employees
                    .iter()
                    .any(|e| matches!(e.state, EmpState::Approach | EmpState::Talking));
        }
        let c1 = (g.customers[0].pos - p).length();
        (hunted, c0 - c1, approached)
    }

    #[test]
    fn npcs_do_not_follow_you_at_all_in_the_tutorial() {
        // Gegenprobe: im normalen Spiel jagen sie (sonst misst der Test nichts)
        let (hunted, _, approached) = pursuit(false, false);
        assert!(hunted, "Gegenprobe: Kunde jagt nicht");
        assert!(approached, "Gegenprobe: Mitarbeiter kommt nicht");
        // Tutorial: weder im Markt noch bei der Flucht
        for paid in [false, true] {
            let (hunted, closer_c, approached) = pursuit(true, paid);
            assert!(!hunted, "Kunde jagt im Tutorial (Flucht: {paid})");
            assert!(
                closer_c < 1.0,
                "Kunde rueckt {closer_c} m naeher (Flucht: {paid})"
            );
            assert!(!approached, "Personal folgt im Tutorial (Flucht: {paid})");
        }
    }

    #[test]
    fn payment_and_belt_windows_explain_themselves_only_in_the_tutorial() {
        for list in [BELT_HINTS, CARD_HINTS, CASH_HINTS] {
            for h in list {
                assert!(h.chars().count() <= HINT_MAX, "zu lang: {h}");
            }
        }
        // normaler Einkauf: nie eine Erklaerleiste
        let mut g = Game::new(61);
        g.belt_window = Some(Default::default());
        assert!(ui_hints(&g).is_none());
        g.belt_window = None;
        g.debug_jump_to_pay(true);
        assert!(ui_hints(&g).is_none());
        // Tutorial: Kassenfenster, Bargeld, Karte
        let mut g = Game::new_tutorial(61);
        assert!(ui_hints(&g).is_none(), "Leiste ohne Fenster");
        g.belt_window = Some(Default::default());
        assert_eq!(ui_hints(&g), Some(BELT_HINTS));
        g.belt_window = None;
        g.debug_jump_to_pay(true);
        assert_eq!(ui_hints(&g), Some(CASH_HINTS));
        let mut g = Game::new_tutorial(62);
        g.debug_jump_to_pay(false);
        assert_eq!(ui_hints(&g), Some(CARD_HINTS));
    }

    #[test]
    fn phase_texts_fit_the_box() {
        use Phase::*;
        for p in [
            Controls, Water, Shopping, ToTill, Belt, Cashier, Pay, Escape,
        ] {
            assert!(p.number() <= PHASES);
            for page in p.pages() {
                assert!(!page.is_empty() && page.len() <= 5);
                for l in page.iter() {
                    assert!(l.chars().count() <= 36, "zu lang: {l}");
                }
            }
        }
        // Einkaufen erklaert auch Kunden und Mitarbeiter, die Seiten wechseln sich ab
        let pages = Shopping.pages();
        assert!(pages.iter().any(|p| p[0].starts_with("KUNDEN")));
        assert!(pages.iter().any(|p| p[0].starts_with("MITARBEITER")));
        assert_eq!(Shopping.page(0.0).0, 0);
        assert_eq!(Shopping.page(PAGE_TIME + 0.1).0, 1);
        assert_eq!(Shopping.page(PAGE_TIME * 3.0 + 0.1).0, 0);
        assert!(Controls.page(5.0).1.is_empty());
    }
}
