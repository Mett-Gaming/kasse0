//! Ein Run: Einkaufen, Kippen, Kunden, Mitarbeiter, Kasse, Ausgang.

pub mod belt_window;
mod brain;
mod cheats;
mod checkout;
mod corridor;
mod crowd;
mod death;
pub mod director;
mod draws;
pub mod escape;
mod highlight;
mod monitor;
pub mod pfand;
mod scare_cheats;
pub mod scare_kinds;
mod scares;
mod senses;
pub use scare_cheats::FIRST_SCARE_CHEAT;
pub mod smoke;
mod tension;
pub use cheats::{cheat_groups, CHEATS, PFAND_CHEATS, STAMINA_CHEAT, TUTORIAL_CHEAT};
pub mod closures;
pub mod dejavu;
pub mod dialog;
pub mod escalation;
mod hands;
mod mirage;
pub mod npc;
pub mod till;
pub mod trip;
pub mod tutorial;
pub mod umbau;
pub mod watcher;
pub mod wet_signs;

use crate::assets::{half_extent_idx, MeshId};
use crate::audio::Sfx;
use crate::catalog::{catalog, misspell};
use crate::render::MeshDraw;
use crate::world::{self, World, WorldParams};
use dialog::{verdict, DialogState, Verdict, ANSWER_TIME, QUESTIONS};
use escalation::{tier_for_progress, tier_for_time, tier_for_value, Tier};
use glam::{vec3, Mat4, Quat, Vec3, Vec4};
use npc::{Customer, EmpState, Employee, Pose, Shopper};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

pub const EYE_HEIGHT: f32 = 1.62;
const PLAYER_RADIUS: f32 = 0.3;
const WALK_SPEED: f32 = 1.5;
const RUN_SPEED: f32 = 2.56;
/// Ausdauer: so viele Sekunden Rennen am Stueck, so viele Sekunden bis wieder voll
const STAMINA_RUN: f32 = 5.0;
const STAMINA_REGEN: f32 = 4.0;
const BASE_SENSITIVITY: f32 = 0.0022;
const REACH: f32 = 2.3;
const CATCH_DIST: f32 = 0.8;
const SCAN_TIME: f32 = 0.9;

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub run: bool,
    /// linker Stick: rechts, vor (-1..1); ersetzt die vier Richtungen
    pub stick: (f32, f32),
    /// Einkaufszettel wird gerade angezeigt (fuer das Tutorial)
    pub list_open: bool,
    /// Flanken (werden in update() verbraucht)
    pub interact: bool,
    pub cart_toggle: bool,
    pub take_out: bool,
    /// Mausrad-Rasten seit dem letzten Update (Ware im Wagen waehlen)
    pub wheel: i32,
    pub choice: Option<usize>,
    /// Zifferntaste 0-9 (PIN am Kartenterminal)
    pub digit: Option<u8>,
    /// G: Item aus der Tasche benutzen
    pub use_item: bool,
    /// R: an der Kasse reklamieren (falsch gescannt)
    pub complain: bool,
    /// C oder Strg gehalten: ducken
    pub crouch: bool,
    /// Leertaste: springen (Flanke)
    pub jump: bool,
    /// Kassenfenster (belt_window.rs): Mauszeiger im HUD-Raster, Taste gehalten,
    /// gerade gedrueckt / losgelassen (Flanken werden in update() verbraucht)
    pub ui_mouse: (f32, f32),
    pub ui_down: bool,
    pub ui_pressed: bool,
    pub ui_released: bool,
}

pub struct Player {
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    bob_phase: f32,
    step_acc: f32,
    /// Ausdauer 0..1 (Rennen kostet, Gehen und Stehen erholt)
    pub stamina: f32,
    /// Ausdauer leer: Rennen erst wieder ab einem Drittel
    pub exhausted: bool,
    /// rennt gerade wirklich (Taste, in Bewegung, Ausdauer da)
    pub running: bool,
    /// geduckt 0..1 (weich), Sprunghoehe und -tempo
    pub crouch: f32,
    pub jump_y: f32,
    pub jump_v: f32,
    /// Sprung mit geschobenem Wagen (beim Aufsetzen faellt Ware heraus)
    pub cart_jump: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickupKind {
    /// Ware aus dem Katalog; `fake` = falsch beschriftete Anomalie
    Product { product: usize, fake: bool },
    /// Tomatensaft eines Kunden
    Tomato { customer: usize },
    /// Leergut (pfand/): keine Ware
    Deposit(pfand::Empty),
}

/// Eine konkrete Ware (in der Hand, am Boden oder im Wagen).
#[derive(Clone, Debug)]
pub struct ItemData {
    pub name: String,
    pub cents: i64,
    pub kind: PickupKind,
    /// globaler Modell-Index
    pub mesh: usize,
    pub tint: Vec3,
}

impl ItemData {
    pub fn half(&self) -> Vec3 {
        half_extent_idx(self.mesh)
    }

    /// Aus einer Ware im Laden.
    pub fn from_shelf(it: &world::ShelfItem) -> Self {
        let mut d = Self::from_shelf_raw(it);
        if it.rot == 1 {
            d.name.push_str(" (verdorben)");
            d.tint *= vec3(0.5, 0.55, 0.3);
        }
        d
    }

    fn from_shelf_raw(it: &world::ShelfItem) -> Self {
        let p = &catalog().products[it.product as usize];
        if it.fake {
            let seed = (it.pos.x * 31.0 + it.pos.z * 17.0) as u64;
            ItemData {
                name: misspell(&p.name, seed),
                cents: fake_price(seed as u32),
                kind: PickupKind::Product {
                    product: it.product as usize,
                    fake: true,
                },
                mesh: p.mesh,
                tint: p.tint * vec3(0.7, 1.05, 0.85),
            }
        } else {
            ItemData {
                name: p.name.clone(),
                cents: p.cents as i64,
                kind: PickupKind::Product {
                    product: it.product as usize,
                    fake: false,
                },
                mesh: p.mesh,
                tint: p.tint,
            }
        }
    }

    /// Zettel-Eintrag, den diese Ware erfuellt (falsche Ware zaehlt nicht).
    pub fn list_entry(&self) -> Option<usize> {
        match self.kind {
            PickupKind::Product {
                product,
                fake: false,
            } => catalog().products[product].list,
            _ => None,
        }
    }
}

/// Unsinnige Preise fuer falsch beschriftete Ware (Anomalie).
pub fn fake_price(seed: u32) -> i64 {
    const P: [i64; 6] = [0, -100, 99_999, 1, 666, 4];
    P[(seed as usize) % P.len()]
}

pub type CartItem = ItemData;

/// Sonderware ausserhalb der Regale (der Tomatensaft der Kunden).
pub struct Pickup {
    pub data: ItemData,
    pub pos: Vec3,
    pub taken: bool,
}

/// Eine fallen gelassene Ware.
pub struct Loose {
    pub data: ItemData,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
}

/// Der Einkaufswagen als Objekt in der Welt.
pub struct Trolley {
    pub pos: Vec3,
    pub yaw: f32,
    pub pushing: bool,
}

/// Worauf man gerade schaut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Focus {
    /// Ware im Regal (Index in world.items)
    Shelf(usize),
    Pickup(usize),
    Loose(usize),
    Cart,
    /// Kassenband der offenen Kasse
    Belt,
    /// Kartenterminal der offenen Kasse
    Terminal,
    /// Mitarbeiter, der gerade den eigenen Wagen wegschiebt
    Employee(usize),
    /// Tuer im Fluchtkorridor (Index in corridor.stations)
    Door(usize),
    /// leere Flasche am Boden (Index in pfand.bottles)
    Bottle(usize),
    /// Pfandautomat (Index in wing.machines)
    Machine(usize),
    /// Kiosk (pfand/kiosk.rs)
    Kiosk,
    /// Ablage der Kiosk-Maschine, in der Gekauftes liegt (pfand/kiosk.rs)
    Tray,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Running,
    Won,
    Caught,
}

pub use checkout::BeltItem;

pub struct Game {
    pub seed: u64,
    pub params: WorldParams,
    pub world: World,
    pub player: Player,
    pub time: f32,
    pub run_time: f32,
    pub glitch: f32,
    pub red: f32,
    pub tipped: bool,

    pub list: Vec<usize>,
    pub pickups: Vec<Pickup>,
    /// Inhalt des Einkaufswagens
    pub cart: Vec<CartItem>,
    pub trolley: Trolley,
    pub held: Option<ItemData>,
    pub pfand: pfand::Pfand,
    pub loose: Vec<Loose>,
    pub tutorial: Option<tutorial::Tutorial>,
    /// Tutorial-Modus: man kann nicht verlieren, Fehler werden nur erklaert
    pub tutorial_run: bool,
    pub suspicion: f32,
    /// in diesem Einkauf wurde ein Cheat benutzt (dann keine Erfolge, achievements.rs)
    pub cheated: bool,
    /// Cheat: Kunden koennen einen nicht erwischen
    pub god: bool,
    /// Cheat: Ausdauer geht nie aus
    pub endless_stamina: bool,
    /// Schonfrist nach einem Gespraech
    pub grace: f32,
    /// Wie oft man schon erwischt wurde (Vignette), laufende Todesanimation
    pub deaths: u32,
    pub dying: Option<f32>,
    /// letzter Funkenschauer je Kabelende (fuer das Knistern)
    spark_bursts: Vec<i64>,
    /// Jumpscares (siehe scares.rs): Abklingzeit, Stille davor, laufende Schrecken
    scare_cd: f32,
    hush_t: f32,
    behind: Option<usize>,
    cashier_scare: Option<f32>,
    cashier_done: bool,
    cart_ghost: scares::CartGhost,
    cart_unseen: f32,
    hole_done: Vec<bool>,
    hanging: Option<(Vec3, f32)>,
    cable_drops: Vec<(Vec3, f32)>,
    /// Figur, die einen erwischt hat (Gesicht in Grossaufnahme)
    catcher_variant: Option<usize>,
    /// Ueberwachungsmonitor: wie lange man hinschaut, laufender Spuk, schon passiert
    monitor_watch: f32,
    monitor_ghost: Option<f32>,
    /// Stromausfall-Schreck: (Sekunden seit Beginn, Kundin, die im Dunkeln vor einen gestellt wurde)
    blackout: Option<(f32, Option<usize>)>,
    /// Cheat: naechste Fata Morgana dreht sich garantiert um
    mirage_force_scary: bool,
    /// Schreck-Effekt (Restzeit, Staerke) und unterschwellige Blitze (Fortschritt, Wartezeit)
    scare_fx: f32,
    scare_fx_strength: f32,
    subliminal: f32,
    subliminal_wait: f32,
    /// Cheat: das Menue soll den Troll-Jumpscare zeigen (main.rs holt das ab)
    troll_request: bool,
    /// Cheat: neuen Einkauf im Tutorial-Modus starten (main.rs holt das ab)
    tutorial_request: bool,
    pub holograms: Vec<mirage::Hologram>,
    mirage_timer: f32,
    tipped_time: f32,
    squeak_acc: f32,
    /// Strecke bis zum naechsten Quietschen, Drehung seit dem letzten
    squeak_next: f32,
    /// angewaehlte Ware im Wagen (F nimmt sie heraus)
    cart_sel: Option<usize>,
    /// zuletzt anvisierte Ware im Wagen
    cart_aim: Option<usize>,
    cart_turn: f32,
    cart_stuck: f32,
    squeak_hinted: bool,
    pub tier: Tier,
    pub customers: Vec<Customer>,
    pub employees: Vec<Employee>,
    pub shoppers: Vec<Shopper>,
    pub dialog: Option<DialogState>,
    /// Ware auf dem Kassenband (vorne = Index 0) und bereits gescannte Ware
    pub belt: Vec<BeltItem>,
    pub scanned: Vec<ItemData>,
    /// Kassen-Minispiel (laeuft ab der ersten Ware auf dem Band)
    pub till: Option<till::Till>,
    /// Run an der Kasse verloren (die Kassiererin hat einen durchschaut)
    pub till_caught: bool,
    /// Flucht zum Ausgang nach dem Bezahlen (Sekunden seit dem Alarm)
    pub chase: Option<f32>,
    /// Im Fluchtkorridor umgedreht (Game Over), Korridor schon betreten (Hinweis)
    pub turned_back: bool,
    /// Fluchtkorridor: Tueren, Regale, Front ... (game/corridor/), von der Front eingeholt
    pub corridor_run: Option<corridor::Run>,
    pub front_caught: bool,
    /// Die nutzlose Zigarette (smoke.rs), Kippen am Boden, ausgeatmeter Rauch (Sekunden)
    /// Tomatensaft-Rausch (trip.rs) und Pfuetzen davon
    pub trip: trip::Trip,
    /// Umbau im Dunkeln (umbau.rs): Licht aus, Markt baut sich um, Licht wieder an
    pub umbau: Option<umbau::Umbau>,
    /// Gestalt in der Ferne (watcher.rs) und Zeit bis zur naechsten
    pub watcher: Option<watcher::Watcher>,
    watcher_timer: f32,
    /// Deja-vu-Szene (dejavu.rs) und Zeit bis zur naechsten
    pub dejavu: Option<dejavu::Dejavu>,
    dejavu_timer: f32,
    /// umgeworfene Warnschilder (wet_signs.rs), Index wie `world.signs`
    pub sign_falls: Vec<Option<wet_signs::SignFall>>,
    /// gesperrter Gang (closures.rs) und Zeit bis zur naechsten Sperre
    pub closure: Option<closures::Closure>,
    closure_timer: f32,
    pub puddles: Vec<trip::Puddle>,
    pub cig: smoke::Cigarette,
    pub butts: Vec<smoke::Butt>,
    pub exhale: f32,
    alarm_timer: f32,
    /// Takt der Verfolger-Schritte und des Herzschlags bei der Flucht
    step_acc: f32,
    heart_acc: f32,
    /// Figur der Kassiererin (Personal-Variante)
    pub cashier_variant: usize,
    /// Druck an der Kasse aus (fuer die alten Tests; die Druck-Tests schalten ihn ein)
    pub calm_till: bool,
    /// Kassenfenster offen (belt_window.rs)
    pub belt_window: Option<belt_window::BeltWindow>,
    scan_timer: f32,
    pub paid: bool,
    pub outcome: Outcome,
    pub outcome_timer: f32,
    pub catcher: Option<Vec3>,
    pub message: Option<(String, f32)>,
    pub announcement: Option<(String, f32)>,
    pub focus: Option<Focus>,
    /// Zeigt der Blick auf den Wagen (Ablegen moeglich)?
    pub aim_cart: bool,
    pub at_register: bool,
    pub sfx: Vec<(Sfx, f32)>,
    pub sensitivity: f32,
    pub fov_deg: f32,
    pub aspect: f32,
    rng: Pcg64,
    /// eigener Zufall fuer Gestalt und Deja-vu (watcher.rs, dejavu.rs), damit sie den Ablauf
    /// von NPCs und Schrecken nicht verschieben
    eerie_rng: Pcg64,

    player_field: Vec<u16>,
    /// Geraeusche dieses Bilds: (Ort, Hoerweite)
    noises: Vec<(Vec3, f32)>,
    /// passende Ware in Griffnaehe (highlight.rs)
    highlight: Option<highlight::Highlight>,
    /// Spannungskurve (director.rs)
    pub director: director::Director,
    /// Schrecken dieses Bilds (main.rs leert die Liste) und die letzten Arten
    pub scares: Vec<scare_kinds::ScareKind>,
    recent_scares: Vec<scare_kinds::ScareKind>,
    /// Stil des laufenden Schrecks (Klang, Bildwirkung)
    scare_style: scare_kinds::ScareStyle,
    /// Zusatz-Schrecken (Decke, Kuehltruhe, Wagen, ...)
    extra: scare_kinds::ExtraScares,
    /// Geraeusche mit Ort: (Klang, Lautstaerke, Ort); main.rs verteilt sie im Stereobild
    pub sfx_at: Vec<(Sfx, f32, Vec3)>,
    /// Cheat: Zustand der NPCs ueber ihren Koepfen anzeigen
    pub npc_debug: bool,
    /// wie lange der Spieler schon stillsteht; hat er gerade ins Regal gegriffen?
    player_still: f32,
    player_reached: bool,
    /// Weg des rollenden Geisterwagens zum Spieler (ohne die engen Regal-Durchgaenge)
    cart_field: Vec<u16>,
    cart_field_timer: f32,
    field_timer: f32,
    next_announce: f32,
    collapse_timer: f32,
    stare_timer: f32,
    msg_cooldown: f32,
    /// Wie stark man sich in letzter Zeit umgedreht hat (fuer "wenn man nicht hinsieht")
    turn_acc: f32,
    was_in_drinks: bool,
    world_dirty: bool,
    statics_dirty: bool,
}

/// Kunden im gekippten Markt: leicht fahl, die Kleidung bleibt erkennbar
const CUSTOMER_TINTS: [Vec3; 6] = [
    Vec3::new(0.92, 0.9, 0.86),
    Vec3::new(0.85, 0.88, 0.9),
    Vec3::new(0.9, 0.86, 0.86),
    Vec3::new(0.86, 0.9, 0.84),
    Vec3::new(0.95, 0.94, 0.9),
    Vec3::new(0.82, 0.82, 0.86),
];

/// Normale Kundschaft: Originalfarben
const SHOPPER_TINTS: [Vec3; 6] = [
    Vec3::new(1.0, 1.0, 1.0),
    Vec3::new(0.97, 0.98, 1.0),
    Vec3::new(1.0, 0.98, 0.96),
    Vec3::new(0.98, 1.0, 0.98),
    Vec3::new(1.0, 1.0, 0.97),
    Vec3::new(0.97, 0.97, 0.97),
];

pub const ANNOUNCEMENTS: &[&str] = &[
    "Kunde Nummer 0, bitte zur Kasse 0.",
    "Liebe Kunden, heute im Angebot: Tomatensaft.",
    "Bitte lassen Sie Ihren Einkaufswagen nicht unbeaufsichtigt.",
    "Der Markt schließt in Kürze. Der Markt schließt nicht.",
    "Wir wünschen Ihnen einen angenehmen Einkauf. Bitte bleiben Sie.",
    "Ein Mitarbeiter bitte in Gang 0.",
    "Bitte schauen Sie nicht weg.",
    "Kasse 0 ist jetzt für Sie geöffnet.",
    "In Gang 4 wurde ein Gesicht gefunden. Bitte an der Information abholen.",
    "Der Kunde direkt hinter Ihnen möchte bitte zur Information.",
    "Heute im Angebot: Ihre Kindheit. Solange der Vorrat reicht.",
    "Wir haben die Kunden gezählt. Es ist einer zu viel.",
    "Bitte atmen Sie leiser. Die anderen Kunden schlafen.",
    "Das Licht in Gang 2 ist kein Licht. Bitte nicht hineinsehen.",
    "Die Ausgänge wurden verlegt. Wohin, ist nicht bekannt.",
    "Frisch eingetroffen in der Obstabteilung: Zähne.",
    "Es ist 25 Uhr. Der Markt hat für Sie geöffnet.",
    "Bitte melden Sie verdächtige Kunden. Sie sind ein verdächtiger Kunde.",
    "Ihr Einkaufswagen vermisst Sie. Bitte kehren Sie zu ihm zurück.",
    "Wir bitten um Ruhe. Etwas hört zu.",
    "Die Kühltruhe in Gang 6 ist nicht leer. Bitte nicht öffnen.",
    "Das ist eine Durchsage. Das ist keine Durchsage.",
    "Kunde 0, Ihre Mutter wartet am Ausgang. Es gibt keinen Ausgang.",
    "Bitte zählen Sie Ihre Finger. Melden Sie Abweichungen an der Kasse.",
    "Ein Mitarbeiter bitte in Gang 0. Ein Mitarbeiter ist bereits in Gang 0.",
    "Liebe Kunden, wir wissen, wo Sie wohnen. Danke für Ihre Treue.",
];

impl Game {
    pub fn new(seed: u64) -> Self {
        let cat = catalog();
        let mut rng = Pcg64::seed_from_u64(seed ^ 0x5EED_0F_CAFE);

        // Einkaufszettel: Wasser + 4..6 weitere Dinge
        let water = cat
            .list_index("Mineralwasser")
            .expect("Mineralwasser fehlt im Katalog");
        let mut others: Vec<usize> = (0..cat.list.len()).filter(|&l| l != water).collect();
        others.shuffle(&mut rng);
        let n = rng.gen_range(4..=6);
        let mut list = vec![water];
        list.extend_from_slice(&others[..n]);

        let mut params = WorldParams::normal(seed);
        params.open_lane = rng.gen_range(0..4);
        let world = world::generate(&params);

        // leicht nach unten schauen, damit man den Wagen sieht
        let player = Player {
            pos: world.spawn,
            vel: Vec3::ZERO,
            yaw: world.spawn_yaw,
            pitch: -0.32,
            bob_phase: 0.0,
            step_acc: 0.0,
            stamina: 1.0,
            exhausted: false,
            running: false,
            crouch: 0.0,
            jump_y: 0.0,
            jump_v: 0.0,
            cart_jump: false,
        };
        let trolley = Trolley {
            pos: world.spawn + vec3(0.0, 0.0, -hands::CART_OFFSET),
            yaw: world.spawn_yaw,
            pushing: true,
        };
        let mut g = Self {
            seed,
            params,
            world,
            player,
            time: 0.0,
            run_time: 0.0,
            glitch: 0.6,
            red: 0.0,
            tipped: false,
            list,
            pickups: Vec::new(),
            cart: Vec::new(),
            trolley,
            held: None,
            loose: Vec::new(),
            tutorial: None,
            tutorial_run: false,
            suspicion: 0.0,
            cheated: false,
            god: false,
            endless_stamina: false,
            grace: 0.0,
            deaths: 0,
            dying: None,
            spark_bursts: Vec::new(),
            scare_cd: 15.0,
            hush_t: 0.0,
            behind: None,
            cashier_scare: None,
            cashier_done: false,
            cart_ghost: Default::default(),
            cart_unseen: 0.0,
            hole_done: Vec::new(),
            hanging: None,
            cable_drops: Vec::new(),
            catcher_variant: None,
            monitor_watch: 0.0,
            monitor_ghost: None,
            blackout: None,
            mirage_force_scary: false,
            scare_fx: 0.0,
            scare_fx_strength: 0.0,
            subliminal: -1.0,
            subliminal_wait: 0.0,
            troll_request: false,
            tutorial_request: false,
            holograms: Vec::new(),
            mirage_timer: 12.0,
            tipped_time: 0.0,
            squeak_acc: 0.0,
            squeak_next: 3.0,
            cart_sel: None,
            cart_aim: None,
            cart_turn: 0.0,
            cart_stuck: 0.0,
            squeak_hinted: false,
            tier: Tier::Calm,
            customers: Vec::new(),
            employees: Vec::new(),
            shoppers: Vec::new(),
            dialog: None,
            belt: Vec::new(),
            scanned: Vec::new(),
            till: None,
            till_caught: false,
            chase: None,
            turned_back: false,
            corridor_run: None,
            front_caught: false,
            trip: trip::Trip::None,
            umbau: None,
            watcher: None,
            watcher_timer: 40.0,
            dejavu: None,
            dejavu_timer: 25.0,
            sign_falls: Vec::new(),
            closure: None,
            closure_timer: 45.0,
            puddles: Vec::new(),
            cig: smoke::Cigarette::None,
            butts: Vec::new(),
            exhale: 0.0,
            alarm_timer: 0.0,
            step_acc: 0.0,
            heart_acc: 0.0,
            // fest aus dem Seed (nicht aus dem RNG, damit sich sonst nichts verschiebt)
            cashier_variant: {
                let staff = crate::people::people_of(true);
                staff[(seed as usize) % staff.len()]
            },
            calm_till: cfg!(test),
            belt_window: None,
            pfand: pfand::Pfand::new(seed),
            scan_timer: 0.0,
            paid: false,
            outcome: Outcome::Running,
            outcome_timer: 0.0,
            catcher: None,
            message: Some((
                "Erst das Wasser holen. Getränke: hinten rechts.".into(),
                7.0,
            )),
            announcement: None,
            focus: None,
            aim_cart: false,
            at_register: false,
            sfx: Vec::new(),
            sensitivity: 1.0,
            fov_deg: 70.0,
            aspect: 16.0 / 9.0,
            rng,
            eerie_rng: Pcg64::seed_from_u64(seed ^ 0xEE21_E000_D00D),
            player_field: Vec::new(),
            noises: Vec::new(),
            highlight: None,
            director: Default::default(),
            scares: Vec::new(),
            recent_scares: Vec::new(),
            scare_style: scare_kinds::ScareKind::Generic.style(),
            extra: Default::default(),
            sfx_at: Vec::new(),
            npc_debug: false,
            player_still: 0.0,
            player_reached: false,
            cart_field: Vec::new(),
            cart_field_timer: 0.0,
            field_timer: 0.0,
            next_announce: 0.0,
            collapse_timer: 0.0,
            stare_timer: 0.0,
            msg_cooldown: 0.0,
            turn_acc: 0.0,
            was_in_drinks: false,
            world_dirty: true,
            statics_dirty: true,
        };
        g.spawn_shoppers();
        g
    }

    /// Zettel-Eintrag "Mineralwasser"
    pub fn water_entry(&self) -> usize {
        self.list[0]
    }

    pub fn take_world_dirty(&mut self) -> bool {
        std::mem::take(&mut self.world_dirty)
    }

    /// Ware im Laden hat sich geaendert (genommen oder Weltwechsel): Instanzen neu hochladen.
    pub fn take_statics_dirty(&mut self) -> bool {
        std::mem::take(&mut self.statics_dirty)
    }

    fn say(&mut self, text: impl Into<String>, secs: f32) {
        self.message = Some((text.into(), secs));
    }

    /// Hinweis, der nicht dauernd wiederholt wird.
    fn hint(&mut self, text: &str) {
        if self.msg_cooldown <= 0.0 {
            self.say(text, 3.0);
            self.msg_cooldown = 6.0;
        }
    }

    fn play(&mut self, s: Sfx, vol: f32) {
        self.sfx.push((s, vol));
    }

    // ------------------------------------------------------------------ Welt

    fn rebuild_world(&mut self) {
        let new = world::generate(&self.params);
        let old = std::mem::replace(&mut self.world, new);
        let w = &self.world;
        self.player.pos = w.collide(w.remap_from(&old, self.player.pos), PLAYER_RADIUS);
        for c in &mut self.customers {
            c.pos = w.collide(w.remap_from(&old, c.pos), 0.25);
            c.home = w.remap_from(&old, c.home);
        }
        for e in &mut self.employees {
            e.pos = w.collide(w.remap_from(&old, e.pos), 0.25);
            e.retarget = 0.0;
        }
        for p in &mut self.pickups {
            p.pos = w.remap_from(&old, p.pos);
        }
        for l in &mut self.loose {
            l.pos = w.remap_from(&old, l.pos);
        }
        self.trolley.pos = w.collide(w.remap_from(&old, self.trolley.pos), 0.3);
        self.field_timer = 0.0;
        self.world_dirty = true;
        self.statics_dirty = true;
        self.pfand_after_rebuild(&old);
    }

    /// Debug/Story: Markt kippen.
    pub fn tip(&mut self) {
        if self.tipped {
            return;
        }
        self.tipped = true;
        self.shoppers.clear();
        // die Tasten-Liste ist vorbei; im Tutorial-Modus geht die Anleitung weiter
        if !self.tutorial_run {
            self.tutorial = None;
        }
        self.params.tipped = true;
        self.apply_tier_params();
        self.rebuild_world();
        self.was_in_drinks = self.world.drinks_zone.contains_xz(self.player.pos);
        self.glitch = 1.0;
        self.play(Sfx::Sting, 0.6);
        self.next_announce = 10.0;
        // Was man im normalen Markt eingepackt hat, ist weg - nur das Wasser bleibt
        let water = self.water_entry();
        let before = self.cart.len();
        self.cart.retain(|c| c.list_entry() == Some(water));
        if self
            .held
            .as_ref()
            .is_some_and(|h| h.list_entry() != Some(water))
        {
            self.held = None;
        }
        self.spawn_customers(self.tier.customers());
        self.assign_stalkers();
        self.spawn_employees(self.tier.employees());
        if self.cart.len() < before {
            self.say(
                "Irgendwas ist anders. Dein Wagen ist leer - nur das Wasser ist noch da.",
                5.0,
            );
        } else {
            self.say("Irgendwas ist anders.", 4.0);
        }
        self.check_tier();
    }

    fn apply_tier_params(&mut self) {
        self.params.stretch = self.tier.stretch();
        self.params.darkness = self.tier.darkness();
        self.params.warp = self.tier.warp();
        self.params.walls = self.tier.walls();
        self.params.mix = self.tier.mix();
        self.params.holes = self.tier.holes();
        self.params.ceiling_holes = self.tier.ceiling_holes();
        self.params.rot = self.tier.rot();
        self.params.giants = self.tier.giants();
        self.params.wing = self.tier.wing();
        self.params.decay = self.tier.level();
    }

    pub fn cart_value(&self) -> i64 {
        self.bought().map(|c| c.cents).sum::<i64>().max(0)
    }

    pub fn list_done(&self, item: usize) -> bool {
        self.bought().any(|c| c.list_entry() == Some(item))
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn list_complete(&self) -> bool {
        self.list.iter().all(|&i| self.list_done(i))
    }

    /// Stufe sofort setzen (Cheat): wie ein Aufstieg in check_tier.
    pub(super) fn force_tier(&mut self, new: Tier) {
        if new <= self.tier {
            return;
        }
        let old = self.tier;
        self.tier = new;
        self.apply_tier_params();
        self.rebuild_world();
        self.glitch = 0.8;
        let extra_c = new.customers().saturating_sub(old.customers());
        let extra_e = new.employees().saturating_sub(old.employees());
        self.spawn_customers(extra_c);
        self.assign_stalkers();
        self.spawn_employees(extra_e);
    }

    fn check_tier(&mut self) {
        if !self.tipped {
            return;
        }
        // Es zaehlt das Schlimmste aus Warenwert, Zettel-Fortschritt und Zeit im gekippten Markt
        let done = self.list.iter().filter(|&&l| self.list_done(l)).count();
        let new = tier_for_value(self.cart_value() as u32)
            .max(tier_for_progress(done, self.list.len()))
            .max(tier_for_time(self.tipped_time));
        self.raise_tier(new);
    }

    /// Stufe live erhoehen: Der Markt baut sich im Dunkeln um (umbau.rs), der Text zur
    /// neuen Stufe erscheint, wenn das Licht wieder da ist.
    pub(super) fn raise_tier(&mut self, new: Tier) {
        if new <= self.tier {
            return;
        }
        let old = self.tier;
        self.tier = new;
        self.apply_tier_params();
        self.play(Sfx::Sting, 0.35);
        let extra_c = new.customers().saturating_sub(old.customers());
        let extra_e = new.employees().saturating_sub(old.employees());
        self.spawn_customers(extra_c);
        self.assign_stalkers();
        self.spawn_employees(extra_e);
        let msg = match new {
            Tier::Uneasy => "Die Gänge sind länger geworden.",
            Tier::Wrong => "Es wird dunkler. Sie werden schneller.",
            Tier::Collapse => "Der Markt hält das nicht mehr aus.",
            Tier::Calm => "",
        };
        self.start_umbau(Some(msg).filter(|m| !m.is_empty()));
        let (a, b) = new.announce_interval();
        self.next_announce = self.next_announce.min(self.rng.gen_range(a..b) * 0.3);
    }

    /// Fuer Tests/Cheats: je offenem Zettelpunkt die billige Variante direkt in den Wagen.
    pub fn fill_list_cheap(&mut self) {
        let cat = catalog();
        for &l in &self.list.clone() {
            if self.list_done(l) {
                continue;
            }
            let p = cat.list[l].cheap[0];
            let prod = &cat.products[p];
            self.cart.push(ItemData {
                name: prod.name.clone(),
                cents: prod.cents as i64,
                kind: PickupKind::Product {
                    product: p,
                    fake: false,
                },
                mesh: prod.mesh,
                tint: prod.tint,
            });
        }
        self.check_tier();
    }

    /// Fuer Tests/Screenshots: alles Billige einpacken, bezahlen, vor die Tuer stellen.
    pub fn debug_finish_shopping(&mut self) {
        self.fill_list_cheap();
        self.scanned.append(&mut self.cart);
        self.customers.clear();
        self.employees.clear();
        self.paid = true;
        self.params.door_open = true;
        self.rebuild_world();
        self.player.pos = self.world.door_center + vec3(0.0, 0.0, 2.0);
    }

    // ---------------------------------------------------------------- Sicht

    pub fn eye(&self) -> Vec3 {
        let p = &self.player;
        let bob = p.bob_phase.sin() * 0.025 * (p.vel.length() / WALK_SPEED).min(1.0);
        // Todesanimation: man sackt zusammen
        let fall = self
            .dying_progress()
            .map_or(0.0, |k| ((k - 0.3) / 0.4).clamp(0.0, 1.0));
        // geduckt tiefer, im Sprung hoeher
        let h = EYE_HEIGHT * (1.0 - 0.45 * p.crouch) + p.jump_y;
        p.pos
            + vec3(
                0.0,
                h * (1.0 - 0.8 * fall) + bob + self.sway_at(p.pos).y,
                0.0,
            )
    }

    /// Parameter fuer das Live-Verbiegen der Gaenge (Shader und Spiel).
    pub fn sway_params(&self) -> Vec4 {
        let base = if self.tipped {
            let (lat, up) = self.tier.sway();
            Vec4::new(lat, up, self.world.aisle_z1, self.world.aisle_x1)
        } else {
            Vec4::ZERO
        };
        // im Tomatensaft-Rausch wogen die Gaenge zusaetzlich (trip.rs)
        self.trip_sway(base)
    }

    /// Wie weit der Boden an dieser Stelle gerade verschoben ist.
    pub fn sway_at(&self, p: Vec3) -> Vec3 {
        world::sway(self.sway_params(), p, self.time)
    }

    pub fn view_dir(&self) -> Vec3 {
        let p = &self.player;
        // beim Kotzen kippt der Blick nach unten (trip.rs)
        let pitch = (p.pitch + self.vomit_pitch()).clamp(-1.5, 1.5);
        vec3(
            pitch.cos() * p.yaw.sin(),
            pitch.sin(),
            -pitch.cos() * p.yaw.cos(),
        )
    }

    /// Ist der Punkt im Sichtfeld und nicht verdeckt?
    pub fn sees(&self, target: Vec3) -> bool {
        let eye = self.eye();
        let to = target - eye;
        let dist = to.length();
        if dist < 0.3 {
            return true;
        }
        let half_v = self.fov_deg.to_radians() * 0.5;
        let half_h = (half_v.tan() * self.aspect).atan();
        let fwd = self.view_dir();
        let flat_f = vec3(fwd.x, 0.0, fwd.z).normalize_or_zero();
        let flat_t = vec3(to.x, 0.0, to.z).normalize_or_zero();
        let h_angle = flat_f.dot(flat_t).clamp(-1.0, 1.0).acos();
        if h_angle > half_h + 0.08 {
            return false;
        }
        let v_angle = (to.y / dist).asin() - self.player.pitch;
        if v_angle.abs() > half_v + 0.15 {
            return false;
        }
        self.world.line_of_sight(eye, target)
    }

    /// Blick um einen Winkel drehen (Controller), unabhaengig von der Mausempfindlichkeit.
    pub fn turn(&mut self, yaw: f32, pitch: f32) {
        let s = BASE_SENSITIVITY * self.sensitivity;
        self.look(yaw / s, pitch / s);
    }

    pub fn look(&mut self, dx: f32, dy: f32) {
        if self.dialog.is_some() || self.outcome != Outcome::Running {
            return;
        }
        // Im Kassenfenster ist die Maus ein Zeiger, kein Blick
        if self.pointer_window_open() {
            return;
        }
        // Am Kartenterminal zieht die Maus die Karte durch, der Blick bleibt stehen
        if self.swiping() {
            self.swipe_move(dx);
            return;
        }
        if self.till_paying() {
            return;
        }
        let s = BASE_SENSITIVITY * self.sensitivity;
        self.turn_acc += (dx * s).abs();
        if let Some(t) = &mut self.tutorial {
            t.looked += (dx * s).abs() + (dy * s).abs();
        }
        self.player.yaw += dx * s;
        self.player.pitch = (self.player.pitch - dy * s).clamp(-1.45, 1.45);
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let view = Mat4::look_to_rh(self.eye(), self.view_dir(), Vec3::Y);
        // Rausch (trip.rs): Sichtfeld atmet, Bild schwankt um die Blickachse
        let fov = self.fov_deg * self.scare_zoom() * self.trip_fov();
        let proj = Mat4::perspective_rh(fov.to_radians(), aspect, 0.05, 140.0);
        Mat4::from_rotation_z(self.trip_roll()) * proj * view
    }

    // --------------------------------------------------------------- Update

    pub fn update(&mut self, dt: f32, input: &mut Input) {
        self.time += dt;
        self.glitch = (self.glitch - dt * 0.6).max(0.0);
        self.red = (self.red - dt * 1.5).max(0.0);
        self.grace = (self.grace - dt).max(0.0);
        self.msg_cooldown -= dt;
        if let Some((_, t)) = &mut self.message {
            *t -= dt;
            if *t <= 0.0 {
                self.message = None;
            }
        }
        if let Some((_, t)) = &mut self.announcement {
            *t -= dt;
            if *t <= 0.0 {
                self.announcement = None;
            }
        }

        // Umbau im Dunkeln (umbau.rs) - auch nach dem Ende sauber abschliessen
        self.update_umbau(dt);

        if self.outcome != Outcome::Running || self.dying.is_some() {
            self.close_pointer_window();
        }
        if self.outcome != Outcome::Running {
            self.outcome_timer += dt;
            // ein Schreck im letzten Moment (z. B. umgedreht im Korridor) klingt noch aus
            self.scare_fx = (self.scare_fx - dt).max(0.0);
            if let Some(c) = self.catcher {
                // zum Faenger drehen
                let to = c + vec3(0.0, 1.55, 0.0) - self.eye();
                let yaw = to.x.atan2(-to.z);
                self.player.yaw = lerp_angle(self.player.yaw, yaw, (dt * 8.0).min(1.0));
                self.player.pitch +=
                    ((to.y / to.length()).asin() - self.player.pitch) * (dt * 8.0).min(1.0);
            }
            input.interact = false;
            input.cart_toggle = false;
            input.take_out = false;
            input.use_item = false;
            input.jump = false;
            input.choice = None;
            input.digit = None;
            input.complain = false;
            input.ui_pressed = false;
            input.ui_released = false;
            return;
        }
        // Todesanimation: keine Eingaben, keine NPCs
        if self.dying.is_some() {
            self.update_dying(dt);
            input.interact = false;
            input.cart_toggle = false;
            input.take_out = false;
            input.use_item = false;
            input.jump = false;
            input.choice = None;
            input.digit = None;
            input.complain = false;
            input.ui_pressed = false;
            input.ui_released = false;
            input.wheel = 0;
            return;
        }
        self.run_time += dt;
        // Zifferntasten: erst Antworten und PIN, sonst die Zigarette (smoke.rs)
        let cig_key = if self.digits_busy() {
            None
        } else {
            input.digit
        };

        if self.dialog.is_some() {
            self.update_dialog(dt, input);
        } else if self.belt_window_open() {
            // Kassenfenster: Waren mit der Maus aufs Band ziehen (belt_window.rs)
            self.update_belt_window(input);
        } else if self.machine_window_open() {
            // Pfandautomat (pfand/machine.rs)
            self.update_machine(dt, input);
        } else if self.kiosk_open() {
            self.update_kiosk(input);
        } else if self.till_paying() {
            // Karte durchziehen / PIN: man steht am Terminal still
            self.update_till_pay(dt, input);
        } else {
            self.update_movement(dt, input);
            self.update_focus();
            if input.interact {
                self.interact();
            }
            if input.cart_toggle {
                self.toggle_cart();
            }
            if input.wheel != 0 && matches!(self.focus, Some(Focus::Cart | Focus::Belt)) {
                self.cycle_cart_selection(-input.wheel);
            }
            if input.take_out {
                self.take_out();
            }
            if input.use_item {
                self.use_item();
            }
            if input.wheel != 0 && !matches!(self.focus, Some(Focus::Cart | Focus::Belt)) {
                self.cycle_item(-input.wheel);
            }
        }
        // Kassen-Minispiel: Fragen, Weggehen, Misstrauen, Leben
        self.update_till(dt, input);
        self.update_cigarette(dt, cig_key);
        self.update_trip(dt);
        self.update_items(dt);
        input.interact = false;
        input.cart_toggle = false;
        input.take_out = false;
        input.use_item = false;
        input.jump = false;
        input.wheel = 0;
        input.choice = None;
        input.digit = None;
        input.complain = false;
        input.ui_pressed = false;
        input.ui_released = false;

        // Tutorial-Modus: Tasten abhaken, Wegweiser (tutorial.rs)
        self.update_tutorial(dt, input);

        self.update_loose(dt);
        self.update_mirages(dt);
        self.update_watcher(dt);
        self.update_dejavu(dt);
        self.update_wet_signs(dt);
        self.update_closures(dt);
        self.update_spark_sounds();
        self.update_scares(dt, input);
        // Zeit im gekippten Markt kann die Stufe auch ohne Einkauf anheben
        self.check_tier();
        self.update_belt(dt);
        self.update_highlight();
        self.update_npcs(dt, input);
        self.update_suspicion(dt, input);
        self.update_events(dt);
        self.update_chase(dt);
        self.update_corridor(dt);

        // Die Schwelle: Getraenkeabteilung betreten und wieder verlassen kippt den Markt.
        let inside = self.world.drinks_zone.contains_xz(self.player.pos);
        if self.was_in_drinks && !inside && !self.tipped {
            self.tip();
        }
        self.was_in_drinks = inside;

        // Gewonnen: bezahlt und draussen
        if self.paid
            && self.world.exit_zone.contains_xz(self.player.pos)
            && self.outcome == Outcome::Running
        {
            self.outcome = Outcome::Won;
            self.outcome_timer = 0.0;
            self.play(Sfx::Chime, 0.6);
        }
    }
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    a + d * t
}

#[cfg(test)]
mod tests;
