//! KASSE 0: liminaler Supermarkt-Horror.
//! Startet mit `cargo run --release [-- <seed>]`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod air;
mod assets;
mod audio;
mod autoshot;
mod catalog;
mod controls;
// Engine-Teile aus Mettgine unter den gewohnten Namen
use mettgine::app::{AppConfig, Stage};
use mettgine::presence::Status;
use mettgine::time::FixedStep;
pub use mettgine::{crash, render, ui};
mod game;
mod hud;
mod pad;
mod people;
mod settings;
mod signs;
mod world;

use audio::{Audio, Mood, Sfx};
use game::{Game, Outcome};
use hud::options::{self, Page, Row};
use hud::Screen;
use mettgine::input::Bind;
use settings::Settings;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

const FIXED_STEP: f32 = 1.0 / 60.0;

/// Wuensche an das Fenster; erfuellt werden sie nach jedem Ereignis (siehe `App::sync`).
#[derive(Default)]
struct Wishes {
    grab: Option<bool>,
    fullscreen: Option<bool>,
    title: Option<String>,
    /// Verzoegerung und Bildrate neu einstellen
    pace: bool,
    quit: bool,
}

struct App {
    wishes: Wishes,
    game: Game,
    settings: Settings,
    audio: Audio,
    screen: Screen,
    sel: usize,
    ui: ui::Ui,
    draws: Vec<render::MeshDraw>,
    ghosts: Vec<render::MeshDraw>,
    /// Staub, Rauch, Atem
    air: air::Air,
    air_draws: Vec<mettgine::particles::ParticleInst>,
    /// animierte Figuren dieses Frames und ihre Knochen-Matrizen
    people: Vec<people::PersonDraw>,
    skins: Vec<render::SkinDraw>,
    /// Figuren nur fuer den Ueberwachungsmonitor (man selbst, und was hinter einem steht)
    monitor_skins: Vec<render::SkinDraw>,
    bones: Vec<glam::Mat4>,
    input: game::Input,
    keys: controls::Keys,
    pad: pad::PadState,
    /// Aktion, die gerade eine neue Taste bekommt
    rebinding: Option<controls::Action>,
    /// Geraet, fuer das die Hinweise gerade gelten (None = neu rechnen)
    hints_for: Option<bool>,
    show_list: bool,
    mouse_delta: (f32, f32),
    cursor: (f32, f32),
    grabbed: bool,
    /// Kassenfenster im letzten Bild offen (dann Maus wieder einfangen, wenn es zugeht)
    window_was_open: bool,
    takt: FixedStep,
    clock: f32,
    fade: f32,
    fixed_seed: Option<u64>,
    /// Laeuft gerade der Tutorial-Modus? (Neustart bleibt im gleichen Modus)
    tutorial_mode: bool,
    /// Test-Modus: laeuft alle Screens durch und speichert Screenshots
    autoshot: Option<(std::path::PathBuf, f32, usize)>,
    shot_request: Option<std::path::PathBuf>,
    /// Sekunden, die der Troll-Jumpscare im Optionsmenue noch laeuft
    menu_scare: f32,
    /// Frame-Zeit-Log (KASSE0_PERF=1 oder im Autoshot): (Frames, Summe, schlechtester Frame)
    perf: Option<(u32, f32, f32)>,
}

fn random_seed() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    (nanos as u64 ^ (nanos >> 64) as u64) % 1_000_000
}

impl App {
    fn new(fixed_seed: Option<u64>) -> Self {
        let settings = Settings::load();
        let audio = Audio::new(settings.volume, settings.music);
        let mut game = Game::new(random_seed());
        game.player.pos = game.world.spawn + glam::vec3(0.0, 0.0, -2.0);
        Self {
            wishes: Wishes::default(),
            game,
            settings,
            audio,
            screen: Screen::Title,
            sel: 0,
            ui: ui::Ui::default(),
            draws: Vec::new(),
            ghosts: Vec::new(),
            air: air::Air::new(),
            air_draws: Vec::new(),
            people: Vec::new(),
            skins: Vec::new(),
            monitor_skins: Vec::new(),
            bones: Vec::new(),
            input: game::Input::default(),
            keys: controls::load(),
            pad: Default::default(),
            rebinding: None,
            hints_for: None,
            show_list: false,
            mouse_delta: (0.0, 0.0),
            cursor: (0.0, 0.0),
            grabbed: false,
            window_was_open: false,
            takt: FixedStep::new(FIXED_STEP),
            clock: 0.0,
            fade: 1.0,
            fixed_seed,
            tutorial_mode: false,
            autoshot: std::env::var_os("KASSE0_AUTOSHOT")
                .map(|d| (std::path::PathBuf::from(d), 0.0, 0)),
            shot_request: None,
            menu_scare: 0.0,
            perf: (std::env::var_os("KASSE0_PERF").is_some()
                || std::env::var_os("KASSE0_AUTOSHOT").is_some())
            .then_some((0, 0.0, 0.0)),
        }
    }

    fn sfx(&mut self, s: Sfx) {
        self.audio.play(s);
    }

    fn sfx_vol(&mut self, s: Sfx, vol: f32) {
        self.audio.play_vol(s, vol);
    }

    /// Troll-Jumpscare starten (Optionen, oder Cheat).
    fn start_menu_scare(&mut self) {
        self.menu_scare = hud::MENU_SCARE_TIME;
        self.sfx_vol(Sfx::Shriek, 1.5);
        self.sfx_vol(Sfx::Scare, 1.3);
        self.sfx_vol(Sfx::Sting, 1.2);
    }

    /// Sekunden seit dem Troll-Jumpscare (None = laeuft nicht)
    fn menu_scare_elapsed(&self) -> Option<f32> {
        (self.menu_scare > 0.0).then(|| hud::MENU_SCARE_TIME - self.menu_scare)
    }

    fn set_grab(&mut self, grab: bool) {
        self.wishes.grab = Some(grab);
        self.grabbed = grab;
    }

    /// Randloses Vollbild laut Optionen; Autoshot bleibt im Fenster (feste Bildgroesse).
    fn wanted_fullscreen(&self) -> bool {
        self.settings.fullscreen && self.autoshot.is_none()
    }

    /// Wuensche ans Fenster erfuellen.
    fn sync(&mut self, stage: &mut Stage) {
        let w = std::mem::take(&mut self.wishes);
        if let Some(grab) = w.grab {
            stage.set_grab(grab);
            self.grabbed = stage.grabbed();
        }
        if let Some(full) = w.fullscreen {
            stage.set_fullscreen(full);
        }
        if w.pace {
            stage
                .renderer
                .set_low_latency(if self.settings.low_latency {
                    render::LowLatency::On
                } else {
                    render::LowLatency::Off
                });
            stage.renderer.set_fps_limit(self.settings.fps_limit as f32);
        }
        if let Some(t) = w.title {
            stage.window.set_title(&t);
        }
        if w.quit {
            stage.quit();
        }
    }

    fn apply_settings(&mut self) {
        self.game.sensitivity = self.settings.sensitivity;
        self.game.fov_deg = self.settings.fov;
        self.audio
            .set_volumes(self.settings.volume, self.settings.music);
        self.wishes.fullscreen = Some(self.wanted_fullscreen());
        self.wishes.pace = true;
    }

    /// Neuer Einkauf; `tutorial` = Tutorial-Modus (Anleitung, Wegweiser, entschaerft).
    fn start_game(&mut self, seed: u64, tutorial: bool) {
        self.tutorial_mode = tutorial;
        self.game = if tutorial {
            Game::new_tutorial(seed)
        } else {
            Game::new(seed)
        };
        self.apply_settings();
        self.input = game::Input::default();
        self.takt.reset();
        self.fade = 1.0;
        self.screen = Screen::Playing;
        self.set_grab(true);
        self.wishes.title = Some(if tutorial {
            "KASSE 0  |  Tutorial".to_string()
        } else {
            format!("KASSE 0  |  Band #{seed}")
        });
    }

    /// Wurde gerade ein Tutorial-Einkauf gewonnen? (Endbildschirm -> Lob vor dem Hauptmenue)
    fn tutorial_won(&self) -> bool {
        self.game.tutorial_run && self.game.outcome == Outcome::Won
    }

    /// Optionen-Bildschirm; die Zeile "Tutorial spielen" gibt es erst nach einem geschafften
    /// Tutorial.
    fn options_screen(&self, from_pause: bool) -> Screen {
        Screen::Options {
            from_pause,
            tutorial: self.settings.tutorial_done,
            page: Page::Display,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.sel = 0;
        let playing = screen == Screen::Playing;
        self.set_grab(playing);
        if !playing {
            self.input = game::Input::default();
            self.show_list = false;
        }
    }

    /// Gewaehlte Zeile, wenn gerade Optionen offen sind.
    fn option_row(&self) -> Option<Row> {
        match self.screen {
            Screen::Options { tutorial, page, .. } => {
                options::rows(page, tutorial).get(self.sel).copied()
            }
            _ => None,
        }
    }

    /// Anderen Reiter der Optionen zeigen.
    fn open_page(&mut self, to: Page) {
        if let Screen::Options { page, .. } = &mut self.screen {
            *page = to;
            // auf den Reitern bleiben, damit man weiterblaettern kann
            self.sel = 0;
            self.sfx(Sfx::Blip);
        }
    }

    fn leave_options(&mut self) {
        self.settings.save();
        self.go(match self.screen {
            Screen::Options {
                from_pause: true, ..
            } => Screen::Paused,
            _ => Screen::Title,
        });
        // zurueck auf "Optionen" (in Pause und Hauptmenue die 2. Zeile)
        self.sel = 1;
    }

    fn activate_option(&mut self, dir: i32) {
        let (Some(row), Screen::Options { page, .. }) = (self.option_row(), self.screen) else {
            return;
        };
        match row {
            Row::Tabs => return self.open_page(page.next(dir)),
            Row::Back => return self.leave_options(),
            // Jumpscares lassen sich nicht abschalten - der Versuch loest einen aus
            Row::Jumpscares => return self.start_menu_scare(),
            Row::Keys => {
                let Screen::Options { from_pause, .. } = self.screen else {
                    return;
                };
                return self.go(Screen::Keys { from_pause });
            }
            Row::Tutorial => {
                self.settings.save();
                return self.start_game(random_seed(), true);
            }
            _ => self.settings.change(row, dir),
        }
        self.apply_settings();
        self.settings.save();
    }

    /// Mausklick in den Optionen: Reiter und Schieberegler treffen, sonst wie im Menue.
    fn click_option(&mut self, dir: i32) {
        let x = self.cursor.0;
        match self.option_row() {
            Some(Row::Tabs) => {
                if let Some(p) = options::tab_at(x) {
                    self.sfx(Sfx::Click);
                    self.open_page(p);
                }
            }
            Some(row) => match options::slider_value(row, x) {
                Some(v) if dir > 0 => {
                    self.settings.set_number(row, v);
                    self.sfx(Sfx::Click);
                    self.apply_settings();
                    self.settings.save();
                }
                _ => self.activate(dir),
            },
            None => {}
        }
    }

    /// Menuepunkt ausloesen. `dir` = +1/-1 fuer Optionswerte.
    fn activate(&mut self, dir: i32) {
        self.sfx(Sfx::Click);
        match self.screen {
            Screen::Title => match self.sel {
                0 => {
                    // allererster Einkauf: erst fragen, ob man das Tutorial spielen will
                    if self.settings.tutorial_asked {
                        let seed = self.fixed_seed.take().unwrap_or_else(random_seed);
                        self.start_game(seed, false);
                    } else {
                        self.go(Screen::AskTutorial);
                    }
                }
                1 => self.go(self.options_screen(false)),
                _ => self.wishes.quit = true,
            },
            Screen::AskTutorial => {
                // gefragt wird nur einmal, egal wie die Antwort ist
                self.settings.tutorial_asked = true;
                self.settings.save();
                if self.sel == 0 {
                    let seed = self.fixed_seed.take().unwrap_or_else(random_seed);
                    self.start_game(seed, true);
                } else {
                    self.go(Screen::Champ);
                }
            }
            Screen::Champ => {
                let seed = self.fixed_seed.take().unwrap_or_else(random_seed);
                self.start_game(seed, false);
            }
            Screen::Paused => match self.sel {
                0 => self.go(Screen::Playing),
                1 => self.go(self.options_screen(true)),
                2 => self.start_game(self.game.seed, self.tutorial_mode),
                3 => self.to_title(),
                _ => self.wishes.quit = true,
            },
            Screen::End => match self.sel {
                0 => self.start_game(self.game.seed, self.tutorial_mode),
                1 => self.start_game(random_seed(), false),
                // nach einem geschafften Tutorial: erst das Lob, dann ins Hauptmenue
                _ if self.tutorial_won() => self.go(Screen::TutorialDone),
                _ => self.to_title(),
            },
            Screen::TutorialDone => self.to_title(),
            Screen::Options { .. } => self.activate_option(dir),
            Screen::Keys { .. } => self.activate_key_row(),
            // Kategorien: Kategorie oeffnen oder zurueck ins Spiel
            Screen::Cheats { group: None } => {
                if self.sel < game::cheat_groups().len() {
                    let g = self.sel;
                    self.go(Screen::Cheats { group: Some(g) });
                } else {
                    self.go(Screen::Playing);
                }
            }
            // Cheats einer Kategorie: ausloesen (und weiterspielen) oder zurueck zu den Kategorien
            Screen::Cheats { group: Some(g) } => {
                let list = game::cheat_groups()
                    .get(g)
                    .map(|c| c.2.clone())
                    .unwrap_or_default();
                match list.get(self.sel) {
                    Some(&i) => {
                        self.game.cheat(i);
                        self.go(Screen::Playing);
                        if self.game.take_troll_request() {
                            self.start_menu_scare();
                        }
                        // Cheat "Tutorial starten": neuer Einkauf im Tutorial-Modus
                        if self.game.take_tutorial_request() {
                            self.start_game(random_seed(), true);
                        }
                    }
                    None => {
                        self.go(Screen::Cheats { group: None });
                        self.sel = g;
                    }
                }
            }
            Screen::Playing => {}
        }
    }

    fn to_title(&mut self) {
        let mut g = Game::new(random_seed());
        g.player.pos = g.world.spawn + glam::vec3(0.0, 0.0, -2.0);
        self.game = g;
        self.apply_settings();
        self.fade = 1.0;
        self.go(Screen::Title);
        self.wishes.title = Some("KASSE 0".to_string());
    }

    fn back(&mut self) {
        match self.screen {
            Screen::Playing => self.go(Screen::Paused),
            Screen::Paused => self.go(Screen::Playing),
            Screen::Options { .. } => self.leave_options(),
            Screen::Keys { from_pause } => {
                let mut to = self.options_screen(from_pause);
                if let Screen::Options { page, .. } = &mut to {
                    *page = Page::Controls;
                }
                self.go(to);
                self.sel = options::rows(Page::Controls, true)
                    .iter()
                    .position(|r| *r == Row::Keys)
                    .unwrap_or(0);
            }
            Screen::Cheats { group: Some(g) } => {
                self.go(Screen::Cheats { group: None });
                self.sel = g;
            }
            Screen::Cheats { group: None } => self.go(Screen::Playing),
            // Frage abbrechen: zurueck ins Hauptmenue (beim naechsten Einkauf wieder gefragt)
            Screen::AskTutorial => self.go(Screen::Title),
            // "OK Champ" wegklicken: wie OK
            Screen::Champ => {
                let seed = self.fixed_seed.take().unwrap_or_else(random_seed);
                self.start_game(seed, false);
            }
            Screen::TutorialDone => self.to_title(),
            Screen::End | Screen::Title => {}
        }
    }

    fn run_frame(&mut self, stage: &mut Stage, dt: f32) {
        if let Some((n, sum, worst)) = &mut self.perf {
            let raw = dt;
            *n += 1;
            *sum += raw;
            *worst = worst.max(raw);
            if *sum > 5.0 {
                crash::log(&format!(
                    "PERF {:.2} ms/Frame ({:.0} fps), schlechtester {:.1} ms, {} Waren im Regal",
                    *sum / *n as f32 * 1000.0,
                    *n as f32 / *sum,
                    *worst * 1000.0,
                    self.game.world.items.len()
                ));
                *n = 0;
                *sum = 0.0;
                *worst = 0.0;
            }
        }
        self.clock += dt;
        let before = self.menu_scare_elapsed();
        self.menu_scare = (self.menu_scare - dt).max(0.0);
        // zweiter Einschlag, wenn die Nachricht reinknallt
        if let (Some(b), Some(a)) = (before, self.menu_scare_elapsed()) {
            if b < hud::MENU_SCARE_FACE && a >= hud::MENU_SCARE_FACE {
                self.sfx_vol(Sfx::Scare, 1.4);
                self.sfx_vol(Sfx::Sting, 1.4);
            }
        }
        self.fade = (self.fade - dt * 1.2).max(0.0);
        let splash = stage.splash_active();
        if let Some((dir, mut timer, step)) = self.autoshot.clone().filter(|_| !splash) {
            timer -= dt;
            if timer <= 0.0 {
                match self.autoshot_step(step, &dir) {
                    Some(wait) => self.autoshot = Some((dir, wait, step + 1)),
                    None => {
                        self.wishes.quit = true;
                        return;
                    }
                }
            } else {
                self.autoshot = Some((dir, timer, step));
            }
        }
        let aspect = stage.aspect();
        self.game.aspect = aspect;
        // Screenshots wechseln den Bildschirm: Wuensche gleich erfuellen
        self.sync(stage);

        self.poll_pad(stage, dt);

        match self.screen {
            Screen::Playing => {
                // Kassenfenster: Mauszeiger frei, Position geht ans Spiel (game/belt_window.rs)
                let window = self.game.belt_window_open();
                if window && self.grabbed {
                    self.set_grab(false);
                } else if !window && !self.grabbed && self.window_was_open {
                    self.set_grab(true);
                }
                self.window_was_open = window;
                self.input.ui_mouse = self.cursor;
                let (mx, my) = std::mem::take(&mut self.mouse_delta);
                self.game.look(mx, my);
                self.takt.add(dt);
                while self.takt.step() {
                    let t_upd = Instant::now();
                    self.game.update(FIXED_STEP, &mut self.input);
                    if self.perf.is_some() && t_upd.elapsed().as_secs_f32() > 0.008 {
                        crash::log(&format!(
                            "PERF Update {:.1} ms",
                            t_upd.elapsed().as_secs_f32() * 1000.0
                        ));
                    }
                }
                self.air.update(&self.game, dt);
                let wait = if self.game.outcome == Outcome::Caught {
                    2.2
                } else {
                    1.2
                };
                if self.game.outcome != Outcome::Running && self.game.outcome_timer > wait {
                    // Tutorial geschafft: ab jetzt "Tutorial spielen" in den Optionen
                    if self.game.outcome == Outcome::Won
                        && self.game.tutorial_run
                        && !self.settings.tutorial_done
                    {
                        self.settings.tutorial_done = true;
                        self.settings.save();
                    }
                    self.go(Screen::End);
                }
            }
            Screen::Title | Screen::AskTutorial | Screen::Champ => {
                // Langsamer Kameraschwenk durch den (noch normalen) Markt
                let mut idle = game::Input::default();
                self.game.update(dt, &mut idle);
                self.game.player.yaw = (self.clock * 0.06).sin() * 0.9 - 0.3;
                self.game.player.pitch = 0.05;
            }
            _ => {}
        }

        let sounds = std::mem::take(&mut self.game.sfx);
        self.rumble(stage, dt, &sounds);
        for (s, v) in sounds {
            self.audio.play_vol(s, v);
        }
        self.game.scares.clear();
        // Geraeusche mit Ort: hinter Regalen dumpf
        let eye = self.game.eye();
        for (s, v, at) in std::mem::take(&mut self.game.sfx_at) {
            let hidden = self.game.sound_hidden(at);
            let muffled = if hidden { audio::HIDDEN } else { 0.0 };
            self.audio.play_at(s, v, at, muffled);
        }
        let listener = audio::Listener {
            pos: eye,
            forward: self.game.view_dir(),
            ..Default::default()
        };
        self.audio.update(
            &Mood {
                playing: !hud::on_title(self.screen),
                tipped: self.game.tipped,
                tier: self.game.tier.level(),
                time: self.clock,
                paused: matches!(
                    self.screen,
                    Screen::Paused
                        | Screen::Keys { from_pause: true }
                        | Screen::Options {
                            from_pause: true,
                            ..
                        }
                ),
                hush: self.game.hush(),
                muffle: self.game.wing_muffle(),
                dull: (self.game.trip_level() * 0.7).max(self.game.dying_progress().unwrap_or(0.0)),
            },
            listener,
            dt,
        );

        // UI aufbauen
        self.ui.clear();
        match self.screen {
            Screen::Title => hud::draw_title(&mut self.ui, self.sel, self.clock, self.game.seed),
            Screen::AskTutorial | Screen::Champ | Screen::TutorialDone => {
                hud::draw_ask_tutorial(&mut self.ui, self.screen, self.sel, self.clock)
            }
            Screen::Playing => hud::draw_hud(&mut self.ui, &self.game, self.show_list, self.clock),
            Screen::Paused => hud::draw_pause(&mut self.ui, self.sel, self.clock, &self.game),
            Screen::Options {
                from_pause, page, ..
            } => options::draw(
                &mut self.ui,
                self.sel,
                self.clock,
                &self.settings,
                from_pause,
                page,
            ),
            Screen::Keys { from_pause } => hud::keys::draw(
                &mut self.ui,
                self.sel,
                self.clock,
                &self.keys,
                self.rebinding,
                from_pause,
            ),
            Screen::End => hud::draw_end(&mut self.ui, &self.game, self.sel, self.clock),
            Screen::Cheats { group } => {
                hud::draw_cheats(&mut self.ui, self.sel, self.clock, &self.game, group)
            }
        }
        self.draw_pin_digit(stage.input.using_pad);
        if self.menu_scare > 0.0 {
            hud::draw_menu_scare(
                &mut self.ui,
                hud::MENU_SCARE_TIME - self.menu_scare,
                self.clock,
            );
        }
        // Cheat: Zustand der NPCs ueber ihren Koepfen
        if self.screen == Screen::Playing && self.game.npc_debug {
            let vp = self.game.view_proj(aspect);
            let labels: Vec<(f32, f32, String)> = self
                .game
                .npc_debug_labels()
                .into_iter()
                .filter_map(|(at, text)| {
                    let p = render::project(&vp, at)?;
                    let inside = p.x > 0.0 && p.x < 1.0 && p.y > 0.0 && p.y < 1.0;
                    inside.then(|| (p.x * ui::W, p.y * ui::H, text))
                })
                .collect();
            hud::draw_npc_debug(&mut self.ui, &labels, &self.game);
        }

        if let Some(path) = self.shot_request.take() {
            stage.renderer.capture_next(path);
        }
        let t_up = Instant::now();
        if self.game.take_world_dirty() {
            if let Err(e) = stage
                .renderer
                .upload_world(&self.game.world.instances, &self.game.world.lights)
            {
                crash::fatal(
                    "Grafikspeicher",
                    &format!("Fehler beim Hochladen der Welt: {e}"),
                    "Der Markt passte nicht in den Grafikspeicher.",
                );
                self.wishes.quit = true;
                return;
            }
        }
        if self.game.take_statics_dirty() {
            let mut statics = Vec::new();
            self.game.world.item_draws(&mut statics);
            if let Err(e) = stage.renderer.upload_statics(&statics) {
                crash::fatal(
                    "Grafikspeicher",
                    &format!("Fehler beim Hochladen der Waren: {e}"),
                    "Die Waren passten nicht in den Grafikspeicher.",
                );
                self.wishes.quit = true;
                return;
            }
        }
        if self.perf.is_some() && t_up.elapsed().as_secs_f32() > 0.005 {
            crash::log(&format!(
                "PERF Upload {:.1} ms",
                t_up.elapsed().as_secs_f32() * 1000.0
            ));
        }
        self.draws.clear();
        self.game.collect_draws(&mut self.draws);
        self.game.trip_colorize(&mut self.draws);
        self.ghosts.clear();
        self.game.collect_ghosts(&mut self.ghosts);
        // Figuren: Animation abtasten, Knochen fuer die GPU sammeln
        self.people.clear();
        self.game.collect_people(&mut self.people);
        if self.menu_scare > 0.0 {
            // "Jumpscares aus" gibt es nicht
            let staff = people::people_of(true);
            self.people.push(self.game.face_in_view(staff[0], 0.24));
        }
        self.skins.clear();
        self.bones.clear();
        add_skins(&self.people, &mut self.skins, &mut self.bones);
        self.people.clear();
        self.game.collect_monitor_people(&mut self.people);
        self.monitor_skins.clear();
        add_skins(&self.people, &mut self.monitor_skins, &mut self.bones);
        let g = &self.game;
        let troll = (self.menu_scare > 0.0).then(|| hud::MENU_SCARE_TIME - self.menu_scare);
        let mut view_proj = g.view_proj(aspect);
        // Kamera wackelt beim Schreck (je nach Art)
        let (sx, sy, sr) = g.scare_shake();
        if sx != 0.0 || sy != 0.0 || sr != 0.0 {
            view_proj = glam::Mat4::from_translation(glam::vec3(sx, sy, 0.0))
                * glam::Mat4::from_rotation_z(sr)
                * view_proj;
        }
        if let Some(e) = troll {
            // Bild wackelt und kippt (stark beim Einschlag, danach weniger)
            let k = if e < hud::MENU_SCARE_FACE { 1.0 } else { 0.35 };
            let n = |s: f32| {
                (((self.clock * 60.0).floor() * 12.9898 + s).sin() * 43758.547).fract() - 0.5
            };
            view_proj =
                glam::Mat4::from_translation(glam::vec3(n(1.0) * 0.16 * k, n(2.0) * 0.16 * k, 0.0))
                    * glam::Mat4::from_rotation_z(n(3.0) * 0.25 * k)
                    * view_proj;
        }
        let strobe = troll
            .is_some_and(|e| e > 0.07 && e < hud::MENU_SCARE_FACE && ((e * 14.0) as i32) % 2 == 1);
        let troll_inv = troll.is_some_and(|e| e > 0.07 && ((e * 9.0) as i32) % 3 == 1);
        let fade = self
            .fade
            .max(g.death_fade())
            .max(g.blackout())
            .max(if strobe { 0.85 } else { 0.0 });
        let post = render::Vhs {
            strength: self.settings.vhs,
            dark: g.tipped,
            // In Menues lesbar bleiben
            glitch: if self.menu_scare > 0.0 {
                1.0
            } else if self.screen == Screen::Playing {
                g.glitch.max(g.suspicion * 0.2).max(g.trip_glitch())
            } else {
                0.0
            },
            fade,
            red: g
                .red
                .max(g.death_vignette())
                .max(if troll.is_some() { 1.0 } else { 0.0 }),
            invert: if troll_inv {
                1.0
            } else {
                g.scare_invert().max(g.trip_invert())
            },
            flash: if troll.is_some_and(|e| e < 0.3) {
                1.0
            } else {
                g.scare_flash().max(g.trip_flash())
            },
        };
        let rooms = render::Rooms {
            dark: g.tipped,
            // im Tomatensaft-Rausch: Regenbogen-Nebel (game/trip.rs)
            fog_color: g.trip_fog(g.world.fog_color),
            fog_density: g.trip_fog_density(g.world.fog_density),
            decay: g.poster_decay(),
            sway: g.sway_params(),
        };
        let (mon_w, mon_h) = MONITOR_SIZE;
        let input = render::FrameInput {
            view: render::View {
                view_proj,
                pos: g.eye(),
            },
            second_camera: g
                .monitor_view(mon_w as f32 / mon_h as f32)
                .map(|(view_proj, pos)| render::View { view_proj, pos }),
            time: g.time,
            lighting: render::Lighting::Rooms(rooms),
            post: render::Post::Vhs(post),
            meshes: &self.draws,
            ghosts: &self.ghosts,
            skins: &self.skins,
            camera_skins: &self.monitor_skins,
            bones: &self.bones,
            ui: &self.ui.verts,
            ..Default::default()
        };
        self.air.instances(g.eye(), &mut self.air_draws);
        stage.renderer.queue_particles(&self.air_draws);
        stage.draw(&input);
    }

    fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if let Some(a) = self.rebinding {
            if pressed && !repeat {
                self.finish_rebind(a, Bind::Key(code));
            }
            return;
        }
        if code == KeyCode::F12 && pressed && !repeat {
            let dir = std::env::var_os("USERPROFILE")
                .map(std::path::PathBuf::from)
                .unwrap_or_default()
                .join("Pictures");
            let name = format!(
                "kasse0_{}.bmp",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            );
            self.shot_request = Some(dir.join(name));
            self.game.sfx.push((Sfx::Click, 0.5));
            return;
        }
        if self.screen == Screen::Playing {
            // Ziffern: 1-3 waehlen Antworten, 0-9 tippen die PIN am Kartenterminal
            if let Some(d) = digit_key(code) {
                if pressed && !repeat {
                    self.digit(d);
                }
                return;
            }
            match code {
                // Esc schliesst zuerst das Kassenfenster, sonst Pause
                KeyCode::Escape if pressed && !repeat && self.game.belt_window_open() => {
                    self.game.close_belt_window();
                }
                KeyCode::Escape | KeyCode::KeyP
                    if pressed
                        && !repeat
                        && (code == KeyCode::Escape
                            || self.action_of(Bind::Key(code)).is_none()) =>
                {
                    if self.game.outcome == Outcome::Running {
                        self.go(Screen::Paused);
                    }
                }
                KeyCode::F1 if pressed && !repeat && self.game.outcome == Outcome::Running => {
                    self.go(Screen::Cheats { group: None })
                }
                _ if !repeat => {
                    if let Some(a) = self.action_of(Bind::Key(code)) {
                        self.action(a, pressed);
                    }
                }
                _ => {}
            }
            return;
        }
        if !pressed {
            return;
        }
        let n = hud::menu_len(self.screen);
        if let Screen::Options { page, .. } = self.screen {
            // Reiter wechseln: Tab / E weiter, Q zurueck
            match code {
                KeyCode::Tab | KeyCode::KeyE if !repeat => return self.open_page(page.next(1)),
                KeyCode::KeyQ if !repeat => return self.open_page(page.next(-1)),
                _ => {}
            }
        }
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.sel = (self.sel + n - 1) % n.max(1);
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                self.sel = (self.sel + 1) % n.max(1);
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA if self.option_row().is_some_and(Row::is_value) => {
                self.activate(-1)
            }
            KeyCode::ArrowRight | KeyCode::KeyD if self.option_row().is_some_and(Row::is_value) => {
                self.activate(1)
            }
            // Cheat-Menue: rechts oeffnet die Kategorie, links geht zurueck
            KeyCode::ArrowRight | KeyCode::KeyD
                if self.screen == (Screen::Cheats { group: None })
                    && self.sel < game::cheat_groups().len() =>
            {
                self.activate(1)
            }
            // lange Kategorie (zwei Spalten): links/rechts wechselt die Spalte
            KeyCode::ArrowRight | KeyCode::KeyD
                if hud::two_column_rows(self.screen).is_some_and(|r| self.sel + r < n) =>
            {
                self.sel += hud::two_column_rows(self.screen).unwrap();
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA
                if hud::two_column_rows(self.screen).is_some_and(|r| self.sel >= r) =>
            {
                self.sel -= hud::two_column_rows(self.screen).unwrap();
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA
                if matches!(self.screen, Screen::Cheats { group: Some(_) }) =>
            {
                self.back()
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyE if !repeat => {
                self.activate(1)
            }
            KeyCode::Escape if !repeat => self.back(),
            _ => {}
        }
    }
}

impl mettgine::app::Game for App {
    fn starts_fullscreen(&self) -> bool {
        self.wanted_fullscreen()
    }

    fn renderer(&mut self, window: &Window) -> mettgine::app::Result<render::Renderer> {
        let skins = people::load_skins();
        let meshes = assets::load_all();
        let font = ui::font_atlas();
        let signs = signs::atlas();
        let config = render::Config {
            signs: Some((&signs.pixels, signs::ATLAS_W, signs.height)),
            // grob aufgeloest und hart skaliert: Camcorder-Bild
            resolution: render::Resolution::Fixed(ui::W as u32, ui::H as u32),
            ui: render::UiSize::Fixed(ui::W as u32, ui::H as u32),
            second_camera: Some(MONITOR_SIZE),
            smooth: false,
            last_wins: false,
            // Reflex/Anti-Lag moeglich, Grund bei Grafikkarten-Ausfaellen im Absturzbericht
            low_latency: true,
            diagnostics: true,
            world: WORLD,
            ..render::Config::new(&meshes, &skins, (&font, 128, 128))
        };
        render::Renderer::new(window, &config)
    }

    fn ready(&mut self, stage: &mut Stage) {
        self.apply_settings();
        // Mit festem Seed direkt ins Spiel (praktisch zum Testen)
        if let Some(seed) = self
            .fixed_seed
            .filter(|_| std::env::var_os("KASSE0_DIRECT").is_some())
        {
            self.start_game(seed, false);
        }
        self.sync(stage);
    }

    fn frame(&mut self, stage: &mut Stage, dt: f32) {
        self.run_frame(stage, dt);
        self.sync(stage);
    }

    fn event(&mut self, stage: &mut Stage, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(true) => self.pad.focused = true,
            WindowEvent::Focused(false) => {
                self.pad.focused = false;
                if self.screen == Screen::Playing
                    && self.game.outcome == Outcome::Running
                    && self.autoshot.is_none()
                {
                    self.go(Screen::Paused);
                }
                self.set_grab(false);
            }
            WindowEvent::CursorMoved { .. } => {
                let (x, y) = stage.cursor();
                self.cursor = (x, y);
                if self.screen != Screen::Playing {
                    if let Some(i) = hud::menu_hit(self.screen, x, y) {
                        if i != self.sel {
                            self.sel = i;
                            self.sfx(Sfx::Blip);
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.screen == Screen::Playing => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                };
                self.input.wheel += y.round() as i32;
            }
            // Kassenfenster: Ware loslassen
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button,
                ..
            } => {
                if *button == MouseButton::Left {
                    self.input.ui_released = true;
                    self.input.ui_down = false;
                }
                if let Some(a) = self.action_of(Bind::Mouse(*button)) {
                    self.action(a, false);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if self.screen == Screen::Playing && self.game.belt_window_open() {
                    // Kassenfenster: Ware anfassen
                    if *button == MouseButton::Left {
                        self.input.ui_pressed = true;
                        self.input.ui_down = true;
                    }
                } else if self.screen == Screen::Playing {
                    if !self.grabbed {
                        self.set_grab(true);
                    } else if let Some(a) = self.action_of(Bind::Mouse(*button)) {
                        self.action(a, true);
                    }
                } else if let Some(a) = self.rebinding {
                    self.finish_rebind(a, Bind::Mouse(*button));
                } else if let Some(i) = hud::menu_hit(self.screen, self.cursor.0, self.cursor.1) {
                    self.sel = i;
                    let dir = if *button == MouseButton::Right { -1 } else { 1 };
                    if *button == MouseButton::Left || *button == MouseButton::Right {
                        if matches!(self.screen, Screen::Options { .. }) {
                            self.click_option(dir);
                        } else {
                            self.activate(dir);
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                self.key(*code, *state == ElementState::Pressed, *repeat);
            }
            _ => {}
        }
        self.sync(stage);
    }

    fn mouse_motion(&mut self, _stage: &mut Stage, dx: f32, dy: f32) {
        if self.screen == Screen::Playing {
            self.mouse_delta.0 += dx;
            self.mouse_delta.1 += dy;
        }
    }

    fn crash_context(&self) {
        let g = &self.game;
        crash::context("Bildschirm", format!("{:?}", self.screen));
        crash::context("Seed", g.seed.to_string());
        crash::context(
            "Markt",
            format!(
                "{} (Stufe: {})",
                if g.tipped { "gekippt" } else { "normal" },
                g.tier.name()
            ),
        );
        crash::context("Spieler", format!("{:.1?}", g.player.pos));
        crash::context(
            "NPCs",
            format!(
                "{} Kunden, {} Personal",
                g.customers.len(),
                g.employees.len()
            ),
        );
    }

    /// Was der Launcher in Discord zeigt. Verraet nicht, was im Markt passiert.
    fn status(&self) -> Option<Status> {
        Some(match self.screen {
            Screen::Playing | Screen::Paused | Screen::Cheats { .. } => {
                let done = (0..self.game.list.len())
                    .filter(|&i| self.game.list_done(self.game.list[i]))
                    .count();
                let what = if self.game.tutorial_run {
                    "Im Tutorial"
                } else {
                    "Kauft ein"
                };
                let total = self.game.list.len();
                Status::new(what, format!("Einkaufszettel: {done} von {total}"))
            }
            Screen::End if self.game.outcome == Outcome::Won => {
                Status::new("Einkauf geschafft", "")
            }
            Screen::End => Status::new("Nicht mehr rausgekommen", ""),
            _ => Status::new("Im Menü", ""),
        })
    }
}

/// Zifferntaste (obere Reihe oder Ziffernblock) als Zahl 0-9.
fn digit_key(code: KeyCode) -> Option<u8> {
    Some(match code {
        KeyCode::Digit0 | KeyCode::Numpad0 => 0,
        KeyCode::Digit1 | KeyCode::Numpad1 => 1,
        KeyCode::Digit2 | KeyCode::Numpad2 => 2,
        KeyCode::Digit3 | KeyCode::Numpad3 => 3,
        KeyCode::Digit4 | KeyCode::Numpad4 => 4,
        KeyCode::Digit5 | KeyCode::Numpad5 => 5,
        KeyCode::Digit6 | KeyCode::Numpad6 => 6,
        KeyCode::Digit7 | KeyCode::Numpad7 => 7,
        KeyCode::Digit8 | KeyCode::Numpad8 => 8,
        KeyCode::Digit9 | KeyCode::Numpad9 => 9,
        _ => return None,
    })
}

/// Welt-Materialien (Poster, Schilder, Monitor, Spiegel) aus shaders/world.wgsl
const WORLD: render::WorldShaders<'static> = render::WorldShaders {
    vert: include_bytes!(concat!(env!("OUT_DIR"), "/world.vert.spv")),
    frag: include_bytes!(concat!(env!("OUT_DIR"), "/world.frag.spv")),
};

/// Aufloesung des Ueberwachungsmonitors (zweite Kamera)
const MONITOR_SIZE: (u32, u32) = (160, 90);

/// Version aus dem Git-Tag (build.rs), fuer Absturzberichte
pub const VERSION: &str = env!("KASSE0_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixed_seed = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let config = AppConfig::new("kasse0", "KASSE 0", VERSION);
    mettgine::app::run(config, || Ok(App::new(fixed_seed)))
}

/// Figuren abtasten: Knochen fuer die GPU sammeln, SkinDraws anlegen.
fn add_skins(
    people_in: &[people::PersonDraw],
    skins: &mut Vec<render::SkinDraw>,
    bones: &mut Vec<glam::Mat4>,
) {
    for p in people_in {
        let first_bone = bones.len() as u32;
        let model =
            glam::Mat4::from_rotation_translation(glam::Quat::from_rotation_y(p.yaw), p.pos);
        let mut fx = p.fx;
        // Blickziel in Figuren-Koordinaten (vorne = +z)
        fx.look = p.look_at.map(|w| model.inverse().transform_point3(w));
        people::anims(people::PEOPLE[p.variant % people::PEOPLE.len()].1)
            .pose(p.clip, p.time, &fx, bones);
        skins.push(render::SkinDraw {
            skin: p.variant,
            model: if p.flip {
                // kopfueber: um die Querachse gedreht, Fuesse an p.pos
                glam::Mat4::from_rotation_translation(
                    glam::Quat::from_rotation_y(p.yaw)
                        * glam::Quat::from_rotation_x(std::f32::consts::PI),
                    p.pos,
                )
            } else {
                glam::Mat4::from_rotation_translation(glam::Quat::from_rotation_y(p.yaw), p.pos)
            },
            tint: p.tint,
            first_bone,
            ghost: p.ghost,
            distort: p.fx.distort,
            ..Default::default()
        });
    }
}
