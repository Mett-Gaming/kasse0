//! KASSE 0: liminaler Supermarkt-Horror.
//! Startet mit `cargo run --release [-- <seed>]`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod assets;
mod audio;
mod autoshot;
mod catalog;
mod game;
mod hud;
mod people;
mod render;
mod settings;
mod signs;
mod ui;
mod world;

use audio::{Audio, Mood, Sfx};
use game::{Game, Outcome};
use hud::Screen;
use settings::Settings;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Fullscreen, Window, WindowId},
};

const FIXED_STEP: f32 = 1.0 / 60.0;

struct Running {
    // Reihenfolge wichtig: Renderer (Surface) vor dem Fenster freigeben.
    renderer: render::Renderer,
    window: Window,
}

struct App {
    running: Option<Running>,
    game: Game,
    settings: Settings,
    audio: Option<Audio>,
    screen: Screen,
    sel: usize,
    ui: ui::Ui,
    draws: Vec<render::MeshDraw>,
    ghosts: Vec<render::MeshDraw>,
    /// animierte Figuren dieses Frames und ihre Knochen-Matrizen
    people: Vec<people::PersonDraw>,
    skins: Vec<render::SkinDraw>,
    bones: Vec<glam::Mat4>,
    input: game::Input,
    show_list: bool,
    mouse_delta: (f32, f32),
    cursor: (f32, f32),
    grabbed: bool,
    last_frame: Instant,
    accumulator: f32,
    clock: f32,
    fade: f32,
    fixed_seed: Option<u64>,
    /// Test-Modus: laeuft alle Screens durch und speichert Screenshots
    autoshot: Option<(std::path::PathBuf, f32, usize)>,
    shot_request: Option<std::path::PathBuf>,
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
        let audio = Audio::new(settings.volume);
        let mut game = Game::new(random_seed());
        game.player.pos = game.world.spawn + glam::vec3(0.0, 0.0, -2.0);
        Self {
            running: None,
            game,
            settings,
            audio,
            screen: Screen::Title,
            sel: 0,
            ui: ui::Ui::default(),
            draws: Vec::new(),
            ghosts: Vec::new(),
            people: Vec::new(),
            skins: Vec::new(),
            bones: Vec::new(),
            input: game::Input::default(),
            show_list: false,
            mouse_delta: (0.0, 0.0),
            cursor: (0.0, 0.0),
            grabbed: false,
            last_frame: Instant::now(),
            accumulator: 0.0,
            clock: 0.0,
            fade: 1.0,
            fixed_seed,
            autoshot: std::env::var_os("KASSE0_AUTOSHOT")
                .map(|d| (std::path::PathBuf::from(d), 0.0, 0)),
            shot_request: None,
            perf: (std::env::var_os("KASSE0_PERF").is_some()
                || std::env::var_os("KASSE0_AUTOSHOT").is_some())
            .then_some((0, 0.0, 0.0)),
        }
    }

    fn sfx(&self, s: Sfx) {
        if let Some(a) = &self.audio {
            a.play(s);
        }
    }

    fn set_grab(&mut self, grab: bool) {
        let Some(r) = &self.running else { return };
        if grab {
            let ok = r
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| r.window.set_cursor_grab(CursorGrabMode::Confined))
                .is_ok();
            r.window.set_cursor_visible(false);
            self.grabbed = ok;
        } else {
            let _ = r.window.set_cursor_grab(CursorGrabMode::None);
            r.window.set_cursor_visible(true);
            self.grabbed = false;
        }
    }

    fn apply_settings(&mut self) {
        self.game.sensitivity = self.settings.sensitivity;
        self.game.fov_deg = self.settings.fov;
        if let Some(a) = &mut self.audio {
            a.set_master(self.settings.volume);
        }
        if let Some(r) = &self.running {
            let want = self.settings.fullscreen;
            if r.window.fullscreen().is_some() != want {
                r.window
                    .set_fullscreen(want.then_some(Fullscreen::Borderless(None)));
            }
        }
    }

    fn start_game(&mut self, seed: u64) {
        self.game = Game::new(seed);
        if self.settings.tutorial {
            self.game.tutorial = Some(game::tutorial::Tutorial::default());
        }
        self.apply_settings();
        self.input = game::Input::default();
        self.accumulator = 0.0;
        self.fade = 1.0;
        self.screen = Screen::Playing;
        self.set_grab(true);
        if let Some(r) = &self.running {
            r.window.set_title(&format!("KASSE 0  |  Band #{seed}"));
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

    /// Menuepunkt ausloesen. `dir` = +1/-1 fuer Optionswerte.
    fn activate(&mut self, event_loop: &ActiveEventLoop, dir: i32) {
        self.sfx(Sfx::Click);
        match self.screen {
            Screen::Title => match self.sel {
                0 => {
                    let seed = self.fixed_seed.take().unwrap_or_else(random_seed);
                    self.start_game(seed);
                }
                1 => self.go(Screen::Options { from_pause: false }),
                _ => event_loop.exit(),
            },
            Screen::Paused => match self.sel {
                0 => self.go(Screen::Playing),
                1 => self.go(Screen::Options { from_pause: true }),
                2 => self.start_game(self.game.seed),
                3 => self.to_title(),
                _ => event_loop.exit(),
            },
            Screen::End => match self.sel {
                0 => self.start_game(self.game.seed),
                1 => self.start_game(random_seed()),
                _ => self.to_title(),
            },
            Screen::Options { from_pause } => {
                let s = &mut self.settings;
                let d = dir as f32;
                match self.sel {
                    0 => s.sensitivity = (s.sensitivity + 0.1 * d).clamp(0.2, 3.0),
                    1 => s.volume = (s.volume + 0.1 * d).clamp(0.0, 1.0),
                    2 => s.vhs = (s.vhs + 0.25 * d).clamp(0.0, 1.5),
                    3 => s.fov = (s.fov + 5.0 * d).clamp(55.0, 100.0),
                    4 => s.fullscreen = !s.fullscreen,
                    5 => s.tutorial = !s.tutorial,
                    _ => {
                        self.settings.save();
                        let back = if from_pause {
                            Screen::Paused
                        } else {
                            Screen::Title
                        };
                        self.go(back);
                        self.sel = 1;
                        return;
                    }
                }
                // Rundungsfehler der Schritte vermeiden
                s.sensitivity = (s.sensitivity * 10.0).round() / 10.0;
                s.volume = (s.volume * 10.0).round() / 10.0;
                self.apply_settings();
                self.settings.save();
            }
            Screen::Cheats => {
                if self.sel < game::CHEATS.len() {
                    self.game.cheat(self.sel);
                }
                self.go(Screen::Playing);
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
        if let Some(r) = &self.running {
            r.window.set_title("KASSE 0");
        }
    }

    fn back(&mut self) {
        match self.screen {
            Screen::Playing => self.go(Screen::Paused),
            Screen::Paused => self.go(Screen::Playing),
            Screen::Options { from_pause } => {
                self.settings.save();
                self.go(if from_pause {
                    Screen::Paused
                } else {
                    Screen::Title
                });
                self.sel = 1;
            }
            Screen::Cheats => self.go(Screen::Playing),
            Screen::End | Screen::Title => {}
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if let Some((n, sum, worst)) = &mut self.perf {
            let raw = dt;
            *n += 1;
            *sum += raw;
            *worst = worst.max(raw);
            if *sum > 5.0 {
                eprintln!(
                    "PERF {:.2} ms/Frame ({:.0} fps), schlechtester {:.1} ms, {} Waren im Regal",
                    *sum / *n as f32 * 1000.0,
                    *n as f32 / *sum,
                    *worst * 1000.0,
                    self.game.world.items.len()
                );
                *n = 0;
                *sum = 0.0;
                *worst = 0.0;
            }
        }
        self.clock += dt;
        self.fade = (self.fade - dt * 1.2).max(0.0);
        if let Some((dir, mut timer, step)) = self.autoshot.clone() {
            timer -= dt;
            if timer <= 0.0 {
                match self.autoshot_step(step, &dir) {
                    Some(wait) => self.autoshot = Some((dir, wait, step + 1)),
                    None => {
                        event_loop.exit();
                        return;
                    }
                }
            } else {
                self.autoshot = Some((dir, timer, step));
            }
        }
        let Some(size) = self.running.as_ref().map(|r| r.window.inner_size()) else {
            return;
        };
        let aspect = size.width.max(1) as f32 / size.height.max(1) as f32;
        self.game.aspect = aspect;

        match self.screen {
            Screen::Playing => {
                let (mx, my) = std::mem::take(&mut self.mouse_delta);
                self.game.look(mx, my);
                self.accumulator += dt;
                while self.accumulator >= FIXED_STEP {
                    let t_upd = Instant::now();
                    self.game.update(FIXED_STEP, &mut self.input);
                    if self.perf.is_some() && t_upd.elapsed().as_secs_f32() > 0.008 {
                        eprintln!(
                            "PERF Update {:.1} ms",
                            t_upd.elapsed().as_secs_f32() * 1000.0
                        );
                    }
                    self.accumulator -= FIXED_STEP;
                }
                let wait = if self.game.outcome == Outcome::Caught {
                    2.2
                } else {
                    1.2
                };
                if self.game.outcome != Outcome::Running && self.game.outcome_timer > wait {
                    self.go(Screen::End);
                }
            }
            Screen::Title => {
                // Langsamer Kameraschwenk durch den (noch normalen) Markt
                let mut idle = game::Input::default();
                self.game.update(dt, &mut idle);
                self.game.player.yaw = (self.clock * 0.06).sin() * 0.9 - 0.3;
                self.game.player.pitch = 0.05;
            }
            _ => {}
        }

        if let Some(a) = &self.audio {
            for (s, v) in self.game.sfx.drain(..) {
                a.play_vol(s, v);
            }
            a.update(&Mood {
                playing: self.screen != Screen::Title,
                tipped: self.game.tipped,
                tier: self.game.tier.level(),
                time: self.clock,
                paused: matches!(
                    self.screen,
                    Screen::Paused | Screen::Options { from_pause: true }
                ),
            });
        } else {
            self.game.sfx.clear();
        }

        // UI aufbauen
        self.ui.clear();
        match self.screen {
            Screen::Title => hud::draw_title(&mut self.ui, self.sel, self.clock, self.game.seed),
            Screen::Playing => hud::draw_hud(&mut self.ui, &self.game, self.show_list, self.clock),
            Screen::Paused => hud::draw_pause(&mut self.ui, self.sel, self.clock, &self.game),
            Screen::Options { from_pause } => hud::draw_options(
                &mut self.ui,
                self.sel,
                self.clock,
                &self.settings,
                from_pause,
            ),
            Screen::End => hud::draw_end(&mut self.ui, &self.game, self.sel, self.clock),
            Screen::Cheats => hud::draw_cheats(&mut self.ui, self.sel, self.clock, &self.game),
        }

        let Some(r) = self.running.as_mut() else {
            return;
        };
        if let Some(path) = self.shot_request.take() {
            r.renderer.capture_next(path);
        }
        let t_up = Instant::now();
        if self.game.take_world_dirty() {
            if let Err(e) = r
                .renderer
                .upload_world(&self.game.world.instances, &self.game.world.lights)
            {
                eprintln!("Fehler beim Hochladen der Welt: {e}");
                event_loop.exit();
                return;
            }
        }
        if self.game.take_statics_dirty() {
            let mut statics = Vec::new();
            self.game.world.item_draws(&mut statics);
            if let Err(e) = r.renderer.upload_statics(&statics) {
                eprintln!("Fehler beim Hochladen der Waren: {e}");
                event_loop.exit();
                return;
            }
        }
        if self.perf.is_some() && t_up.elapsed().as_secs_f32() > 0.005 {
            eprintln!(
                "PERF Upload {:.1} ms",
                t_up.elapsed().as_secs_f32() * 1000.0
            );
        }
        self.draws.clear();
        self.game.collect_draws(&mut self.draws);
        self.ghosts.clear();
        self.game.collect_ghosts(&mut self.ghosts);
        // Figuren: Animation abtasten, Knochen fuer die GPU sammeln
        self.people.clear();
        self.game.collect_people(&mut self.people);
        self.skins.clear();
        self.bones.clear();
        for p in &self.people {
            let first_bone = self.bones.len() as u32;
            people::anims(people::PEOPLE[p.variant % people::PEOPLE.len()].1).sample(
                p.clip,
                p.time,
                &mut self.bones,
            );
            self.skins.push(render::SkinDraw {
                skin: p.variant,
                model: glam::Mat4::from_rotation_translation(
                    glam::Quat::from_rotation_y(p.yaw),
                    p.pos,
                ),
                tint: p.tint,
                first_bone,
                ghost: p.ghost,
            });
        }
        let g = &self.game;
        let params = render::FrameParams {
            view_proj: g.view_proj(aspect),
            cam_pos: g.eye(),
            time: g.time,
            tipped: g.tipped,
            fog_color: g.world.fog_color,
            fog_density: g.world.fog_density,
            // In Menues lesbar bleiben
            glitch: if self.screen == Screen::Playing {
                g.glitch.max(g.suspicion * 0.2)
            } else {
                0.0
            },
            vhs: self.settings.vhs,
            fade: self.fade.max(g.death_fade()),
            red: g.red.max(g.death_vignette()),
            decay: g.poster_decay(),
            sway: g.sway_params(),
        };
        if let Err(e) = r.renderer.draw(
            &params,
            &self.draws,
            &self.ghosts,
            &self.skins,
            &self.bones,
            &self.ui.verts,
        ) {
            eprintln!("Renderfehler: {e}");
            event_loop.exit();
        }
    }

    fn key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, pressed: bool, repeat: bool) {
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
                if pressed {
                    if (1..=3).contains(&d) {
                        self.input.choice = Some(d as usize - 1);
                    }
                    if !repeat {
                        self.input.digit = Some(d);
                    }
                }
                return;
            }
            match code {
                KeyCode::KeyW | KeyCode::ArrowUp => self.input.forward = pressed,
                KeyCode::KeyS | KeyCode::ArrowDown => self.input.back = pressed,
                KeyCode::KeyA | KeyCode::ArrowLeft => self.input.left = pressed,
                KeyCode::KeyD | KeyCode::ArrowRight => self.input.right = pressed,
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.input.run = pressed,
                KeyCode::Tab => {
                    self.show_list = pressed;
                    self.input.list_open = pressed;
                }
                KeyCode::KeyQ if pressed && !repeat => self.input.cart_toggle = true,
                KeyCode::KeyF if pressed && !repeat => self.input.take_out = true,
                KeyCode::KeyE | KeyCode::Space if pressed && !repeat => self.input.interact = true,
                KeyCode::Escape | KeyCode::KeyP if pressed && !repeat => {
                    if self.game.outcome == Outcome::Running {
                        self.go(Screen::Paused);
                    }
                }
                KeyCode::F1 if pressed && !repeat && self.game.outcome == Outcome::Running => {
                    self.go(Screen::Cheats)
                }
                _ => {}
            }
            return;
        }
        if !pressed {
            return;
        }
        let n = hud::menu_len(self.screen);
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.sel = (self.sel + n - 1) % n.max(1);
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                self.sel = (self.sel + 1) % n.max(1);
                self.sfx(Sfx::Blip);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA
                if matches!(self.screen, Screen::Options { .. }) && self.sel < 6 =>
            {
                self.activate(event_loop, -1)
            }
            KeyCode::ArrowRight | KeyCode::KeyD
                if matches!(self.screen, Screen::Options { .. }) && self.sel < 6 =>
            {
                self.activate(event_loop, 1)
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyE if !repeat => {
                self.activate(event_loop, 1)
            }
            KeyCode::Escape if !repeat => self.back(),
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("KASSE 0")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Fenster konnte nicht erstellt werden: {e}");
                event_loop.exit();
                return;
            }
        };
        let skins = people::load_skins();
        let meshes = assets::load_all();
        let renderer = match render::Renderer::new(&window, &meshes, &skins, &ui::font_atlas(), {
            let a = signs::atlas();
            (&a.pixels, signs::ATLAS_W, a.height)
        }) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Vulkan-Initialisierung fehlgeschlagen: {e}");
                event_loop.exit();
                return;
            }
        };
        self.running = Some(Running { renderer, window });
        self.apply_settings();
        self.last_frame = Instant::now();
        // Mit festem Seed direkt ins Spiel (praktisch zum Testen)
        if let Some(seed) = self
            .fixed_seed
            .filter(|_| std::env::var_os("KASSE0_DIRECT").is_some())
        {
            self.start_game(seed);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.running {
                    r.renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(false) => {
                if self.screen == Screen::Playing
                    && self.game.outcome == Outcome::Running
                    && self.autoshot.is_none()
                {
                    self.go(Screen::Paused);
                }
                self.set_grab(false);
            }
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(r) = &self.running {
                    let s = r.window.inner_size();
                    let x = position.x as f32 / s.width.max(1) as f32 * ui::W;
                    let y = position.y as f32 / s.height.max(1) as f32 * ui::H;
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
            }
            WindowEvent::MouseWheel { delta, .. } if self.screen == Screen::Playing => {
                let y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(p) => (p.y / 40.0) as f32,
                };
                self.input.wheel += y.round() as i32;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button,
                ..
            } => {
                if self.screen == Screen::Playing {
                    if !self.grabbed {
                        self.set_grab(true);
                    } else if button == MouseButton::Left {
                        self.input.interact = true;
                    }
                } else if let Some(i) = hud::menu_hit(self.screen, self.cursor.0, self.cursor.1) {
                    self.sel = i;
                    let dir = if button == MouseButton::Right { -1 } else { 1 };
                    if button == MouseButton::Left || button == MouseButton::Right {
                        self.activate(event_loop, dir);
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
                self.key(event_loop, code, state == ElementState::Pressed, repeat);
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.grabbed && self.screen == Screen::Playing {
                self.mouse_delta.0 += delta.0 as f32;
                self.mouse_delta.1 += delta.1 as f32;
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(r) = &self.running {
            r.window.request_redraw();
        }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixed_seed = std::env::args().nth(1).and_then(|s| s.parse().ok());
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(fixed_seed);
    event_loop.run_app(&mut app)?;
    Ok(())
}
