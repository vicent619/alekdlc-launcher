use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
use winit::window::{Icon, Window, WindowAttributes, WindowId};

use crate::config::Config;
use crate::core::state::{Account, Screen, State};
use crate::core::storage::Storage;
use crate::core::worker::{Event, Task, Worker};
use crate::geometry::Point;
use crate::gfx::canvas::Canvas;
use crate::gfx::capture::Capture;
use crate::gfx::surface::GlSurfaceTarget;
use crate::i18n::{self, Language, error_text};
use crate::install::installer::Phase;
use crate::theme::{Metrics, Palette};
use crate::ui::interaction::Command;
use crate::ui::root::{Intent, Key as ShellKey, Named, Shell};

static ICON: &[u8] = include_bytes!("../assets/icon.png");

struct Session {
    target: GlSurfaceTarget,
    canvas: Canvas,
    capture: Option<Capture>,
    shown: bool,
}

pub struct App {
    session: Option<Session>,
    state: State,
    shell: Shell,
    storage: Storage,
    worker: Worker,
    events: Receiver<Event>,
    hwid: String,
    last_frame: Instant,
    finished: bool,
    pending_exit: bool,
    cancel: Option<Arc<AtomicBool>>,
    modifiers: winit::keyboard::ModifiersState,
}

impl App {
    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Wait);
        let proxy = event_loop.create_proxy();
        let hwid = crate::core::hwid::compute();
        let mut storage = Storage::load(&hwid);
        let language = storage
            .language()
            .map(|code| Language::parse(&code))
            .unwrap_or(Language::Ru);
        let memory = storage.memory().unwrap_or(4).clamp(Config::MEMORY_MIN, Config::MEMORY_MAX);
        let state = State::new(language, memory);
        let shell = Shell::new(&state);
        let (worker, events) = Worker::spawn(move || {
            let _ = proxy.send_event(());
        });
        let mut state = state;
        let mut shell = shell;
        if let Ok(forced) = std::env::var("ALEK_SCREEN") {
            state.screen = match forced.as_str() {
                "offline" => Screen::Offline,
                "update" => Screen::Update,
                "login" | "login_error" => Screen::Login,
                "home" => Screen::Home,
                "settings" => Screen::Settings,
                _ => Screen::Splash,
            };
            state.version = Some("1.1.0".into());
            if forced == "login_error" {
                state.login.error = Some(error_text(language, "INVALID_CREDENTIALS"));
                state.login.invalid = true;
            }
            if matches!(state.screen, Screen::Home | Screen::Settings) {
                state.account = Some(debug_account());
                state.manifest = Some(debug_manifest());
                if std::env::var("ALEK_MENU").is_ok() {
                    state.menu_open = true;
                }
                if std::env::var("ALEK_TOAST").is_ok() {
                    state.toast(crate::i18n::text(language, "toast.loggedOut"), crate::core::state::ToastKind::Success);
                }
                if std::env::var("ALEK_BUSY").is_ok() {
                    state.play = crate::core::state::PlayState {
                        busy: true,
                        label: crate::i18n::text(language, "play.downloadClient"),
                        detail: "128,4 МБ / 565,1 МБ".to_string(),
                        progress: Some(0.227),
                        cancellable: true,
                        error: None,
                    };
                }
            }
            shell.sync(&state);
        } else {
            worker.submit(Task::Boot {
                refresh: storage.refresh(),
                hwid: hwid.clone(),
            });
        }
        let mut app = Self {
            session: None,
            state,
            shell,
            storage,
            worker,
            events,
            hwid,
            last_frame: Instant::now(),
            finished: false,
            pending_exit: false,
            cancel: None,
            modifiers: winit::keyboard::ModifiersState::empty(),
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    fn attributes() -> WindowAttributes {
        Window::default_attributes()
            .with_title("AlekDLC")
            .with_inner_size(LogicalSize::new(Metrics::WINDOW_W, Metrics::WINDOW_H))
            .with_resizable(false)
            .with_decorations(false)
            .with_transparent(true)
            .with_visible(false)
            .with_window_icon(Self::icon())
    }

    fn icon() -> Option<Icon> {
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(ICON).ok()?;
        let rgba = pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect();
        Icon::from_rgba(rgba, pixmap.width(), pixmap.height()).ok()
    }

    fn canvas(&mut self) -> Option<&mut Canvas> {
        self.session.as_mut().map(|s| &mut s.canvas)
    }

    fn redraw(&mut self) {
        let Some(session) = self.session.as_mut() else { return };
        let window = session.target.window();
        let size = window.inner_size();
        let dpi = window.scale_factor() as f32;
        let dt = self.last_frame.elapsed().as_secs_f32();
        self.last_frame = Instant::now();
        self.shell.tick(dt);
        self.state.expire_toasts();
        session
            .canvas
            .begin(size.width, size.height, dpi, Metrics::WINDOW_RADIUS, Palette::BG);
        self.shell.draw(&mut session.canvas, &self.state);
        session.canvas.finish();
        if let Some(capture) = session.capture.as_mut() {
            self.finished |= capture.tick(&session.target.gl(), size.width, size.height);
        }
        session.target.present();
        if !session.shown {
            session.shown = true;
            session.target.window().set_visible(true);
        }
    }

    fn drain_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.handle_event(event);
        }
        self.shell.sync(&self.state);
        self.request_redraw();
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Version(Ok(info)) => {
                self.state.version = Some(info.version.clone());
                self.state.maintenance = info.maintenance;
                if newer(&info.version, Config::VERSION) {
                    self.state.screen = Screen::Update;
                } else if self.storage.refresh().is_none() {
                    self.go_login(None);
                } else {
                    self.state.splash_key = "splash.signing";
                }
            }
            Event::Version(Err(_)) => {
                self.state.screen = Screen::Offline;
            }
            Event::Authed(Ok((auth, manifest))) => {
                self.storage.set_refresh(Some(auth.refresh_token.clone()));
                if let Some(manifest) = &manifest {
                    self.state.maintenance = manifest.maintenance;
                }
                self.state.manifest = manifest;
                self.state.account = Some(Account {
                    access: auth.access_token,
                    refresh: auth.refresh_token,
                    profile: auth.profile,
                });
                self.state.login.busy = false;
                self.state.login.error = None;
                self.state.login.invalid = false;
                self.state.screen = Screen::Home;
                if cfg!(debug_assertions) && std::env::var("ALEK_MENU").is_ok() {
                    self.state.menu_open = true;
                }
                if cfg!(debug_assertions) && std::env::var("ALEK_SETTINGS").is_ok() {
                    self.state.screen = Screen::Settings;
                }
            }
            Event::Authed(Err(error)) => {
                let on_login = self.state.screen == Screen::Login;
                if error.is("OFFLINE") || error.is("UPSTREAM_DOWN") {
                    if on_login {
                        self.state.login.busy = false;
                        self.state.login.error = Some(error_text(self.state.language, &error.code));
                    } else {
                        self.state.screen = Screen::Offline;
                    }
                    return;
                }
                self.storage.set_refresh(None);
                self.state.login.busy = false;
                self.state.login.invalid = error.is("INVALID_CREDENTIALS");
                let message = if error.is("INVALID_TOKEN") && !on_login {
                    None
                } else {
                    Some(error_text(self.state.language, &error.code))
                };
                self.go_login(message);
            }
            Event::Progress { phase, done, total } => {
                self.state.play.busy = true;
                self.state.play.label = i18n::text(self.state.language, phase_key(phase));
                let indeterminate = total == 0 || matches!(phase, crate::install::installer::Phase::Checking | crate::install::installer::Phase::Verifying | crate::install::installer::Phase::Unpacking);
                self.state.play.progress = if indeterminate { None } else { Some((done as f32 / total as f32).clamp(0.0, 1.0)) };
                self.state.play.cancellable = matches!(phase, crate::install::installer::Phase::DownloadClient | crate::install::installer::Phase::DownloadJava | crate::install::installer::Phase::DownloadGame);
                self.state.play.detail = if total > 0 && !indeterminate {
                    format!(
                        "{} / {}",
                        i18n::format_bytes(self.state.language, done),
                        i18n::format_bytes(self.state.language, total)
                    )
                } else {
                    String::new()
                };
            }
            Event::PlayDone { installed } => {
                self.persist_installed(installed);
                if let Some(session) = self.session.as_ref() {
                    session.target.window().set_visible(false);
                }
                self.pending_exit = true;
            }
            Event::PlayFailed { code, installed } => {
                self.persist_installed(installed);
                self.state.play = crate::core::state::PlayState::default();
                if code == "CANCELLED" {
                    self.state.toast(i18n::text(self.state.language, "toast.cancelled"), crate::core::state::ToastKind::Info);
                    return;
                }
                if code == "INVALID_TOKEN" {
                    self.state.account = None;
                    self.storage.set_refresh(None);
                    self.go_login(Some(error_text(self.state.language, "INVALID_TOKEN")));
                    return;
                }
                self.state.toast(error_text(self.state.language, &code), crate::core::state::ToastKind::Error);
            }
        }
    }

    fn persist_installed(&mut self, installed: std::collections::HashMap<String, String>) {
        self.storage.merge_installed(installed);
    }

    fn go_login(&mut self, error: Option<String>) {
        self.state.login.busy = false;
        self.state.login.error = error;
        self.state.account = None;
        self.state.screen = Screen::Login;
    }

    fn apply_intent(&mut self, intent: Intent) {
        match intent {
            Intent::None => {}
            Intent::Minimize => self.window_op(|w| w.set_minimized(true)),
            Intent::Close => {}
            Intent::Drag => {}
            Intent::OpenUrl(path) => {
                let url = format!("{}{}", Config::site_url(), path);
                let _ = open_url(&url);
            }
            Intent::Retry => {
                self.state.screen = Screen::Splash;
                self.state.splash_key = "splash.connecting";
                self.worker.submit(Task::Boot {
                    refresh: self.storage.refresh(),
                    hwid: self.hwid.clone(),
                });
            }
            Intent::Submit { login, password } => {
                if login.is_empty() || password.is_empty() {
                    self.state.login.invalid = true;
                    self.state.login.error = Some(error_text(self.state.language, "VALIDATION"));
                    return;
                }
                self.state.login.busy = true;
                self.state.login.error = None;
                self.worker.submit(Task::Login {
                    login,
                    password,
                    hwid: self.hwid.clone(),
                });
            }
            Intent::Navigate(screen) => {
                self.state.menu_open = false;
                self.state.screen = screen;
            }
            Intent::ToggleMenu => self.state.menu_open = !self.state.menu_open,
            Intent::CloseMenu => self.state.menu_open = false,
            Intent::Cabinet => {
                self.state.menu_open = false;
                let url = format!("{}{}", Config::site_url(), "/account");
                let _ = open_url(&url);
            }
            Intent::Logout => {
                self.state.menu_open = false;
                if let Some(account) = self.state.account.take() {
                    self.worker.submit(Task::Logout {
                        access: account.access,
                        refresh: account.refresh,
                    });
                }
                self.storage.set_refresh(None);
                self.state.manifest = None;
                self.go_login(None);
                self.state.toast(i18n::text(self.state.language, "toast.loggedOut"), crate::core::state::ToastKind::Success);
            }
            Intent::Play => {
                if self.state.play.busy || self.state.account.is_none() {
                    return;
                }
                let Some(account) = self.state.account.as_ref() else { return };
                let cancel = Arc::new(AtomicBool::new(false));
                self.cancel = Some(cancel.clone());
                self.state.play = crate::core::state::PlayState {
                    busy: true,
                    label: i18n::text(self.state.language, "play.checking"),
                    cancellable: true,
                    ..Default::default()
                };
                self.worker.submit(Task::Play(crate::core::worker::PlayRequest {
                    access: account.access.clone(),
                    hwid: self.hwid.clone(),
                    username: account.profile.username.clone(),
                    memory: self.state.memory,
                    installed: self.storage.installed().clone(),
                    cancel,
                }));
            }
            Intent::Cancel => {
                if let Some(cancel) = &self.cancel {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            Intent::SetMemory(value) => {
                if self.state.memory != value {
                    self.state.memory = value;
                    self.storage.set_memory(value);
                }
            }
            Intent::SetLanguage(lang) => {
                if self.state.language != lang {
                    self.state.language = lang;
                    self.storage.set_language(lang.code());
                }
            }
            Intent::OpenFolder => {
                let path = crate::core::paths::Paths::game();
                let _ = std::fs::create_dir_all(&path);
                let _ = std::process::Command::new("explorer").arg(path).spawn();
            }
            Intent::Verify => {
                self.storage.clear_installed();
                self.state.toast(i18n::text(self.state.language, "settings.verifyDone"), crate::core::state::ToastKind::Info);
            }
        }
    }

    fn window_op<F: FnOnce(&Window)>(&self, op: F) {
        if let Some(session) = self.session.as_ref() {
            op(session.target.window());
        }
    }

    fn request_redraw(&self) {
        if let Some(session) = self.session.as_ref() {
            session.target.window().request_redraw();
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.session.is_some() {
            return;
        }
        let target = GlSurfaceTarget::open(event_loop, Self::attributes()).expect("gl surface");
        let canvas = Canvas::new(target.gl()).expect("canvas");
        self.session = Some(Session {
            target,
            canvas,
            capture: Capture::from_env(),
            shown: false,
        });
        self.redraw();
    }

    fn user_event(&mut self, _: &ActiveEventLoop, _: ()) {
        self.drain_events();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(session) = self.session.as_ref() {
                    session.target.resize(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let dpi = self.session.as_ref().map(|s| s.target.window().scale_factor()).unwrap_or(1.0);
                let logical = position.to_logical::<f32>(dpi);
                let point = Point::new(logical.x, logical.y);
                let drag = if let Some(session) = self.session.as_mut() {
                    let (canvas, shell, state) = (&mut session.canvas, &mut self.shell, &self.state);
                    shell.pointer_moved(canvas, state, point);
                    shell.drag(state)
                } else {
                    Intent::None
                };
                self.apply_intent(drag);
                self.request_redraw();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y * 48.0,
                    winit::event::MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.shell.scroll(&self.state, amount);
                self.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                if let Some(session) = self.session.as_mut() {
                    self.shell.pointer_left(&mut session.canvas, &self.state);
                }
                self.request_redraw();
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_click(state, event_loop);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    self.handle_key(event);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.finished || self.pending_exit {
            event_loop.exit();
            return;
        }
        if let Some(session) = self.session.as_ref() {
            if session.capture.is_some() || self.shell.animating(&self.state) {
                session.target.window().request_redraw();
            }
        }
    }
}

impl App {
    fn handle_click(&mut self, state: ElementState, event_loop: &ActiveEventLoop) {
        let Some(session) = self.session.as_mut() else { return };
        let canvas = &mut session.canvas;
        let (command, intent) = match state {
            ElementState::Pressed => {
                let command = self.shell.pressed(canvas, &self.state);
                (command, self.shell.take_press_intent())
            }
            ElementState::Released => self.shell.released(canvas, &self.state),
        };
        match command {
            Command::DragWindow => {
                let _ = session.target.window().drag_window();
            }
            Command::Minimize => session.target.window().set_minimized(true),
            Command::Close => {
                event_loop.exit();
                return;
            }
            Command::None => {}
        }
        self.apply_intent(intent);
        self.shell.sync(&self.state);
        self.request_redraw();
    }

    fn handle_key(&mut self, event: KeyEvent) {
        let ctrl = self.modifiers.control_key();
        let shift = self.modifiers.shift_key();
        let mut named = match &event.logical_key {
            Key::Named(NamedKey::Backspace) => Some(Named::Backspace),
            Key::Named(NamedKey::Enter) => Some(Named::Enter),
            Key::Named(NamedKey::Tab) => Some(Named::Tab),
            Key::Named(NamedKey::ArrowLeft) => Some(Named::Left),
            Key::Named(NamedKey::ArrowRight) => Some(Named::Right),
            Key::Named(NamedKey::Home) => Some(Named::Home),
            Key::Named(NamedKey::End) => Some(Named::End),
            _ => None,
        };
        if ctrl && named.is_none() {
            if let PhysicalKey::Code(code) = event.physical_key {
                named = match code {
                    KeyCode::KeyA => Some(Named::SelectAll),
                    KeyCode::KeyC => Some(Named::Copy),
                    KeyCode::KeyV => Some(Named::Paste),
                    KeyCode::KeyX => Some(Named::Cut),
                    _ => None,
                };
            }
        }
        let text = event.text.as_ref().map(|t| t.to_string()).filter(|_| named.is_none() && !ctrl);
        let intent = self.shell.key(&self.state, ShellKey { named, text, shift });
        self.apply_intent(intent);
        self.shell.sync(&self.state);
        self.request_redraw();
    }
}

fn phase_key(phase: Phase) -> &'static str {
    match phase {
        Phase::Checking => "play.checking",
        Phase::DownloadClient => "play.downloadClient",
        Phase::DownloadJava => "play.downloadJava",
        Phase::DownloadGame => "play.downloadGame",
        Phase::DownloadMods => "play.downloadMods",
        Phase::Verifying => "play.verifying",
        Phase::Unpacking => "play.unpacking",
    }
}

fn newer(remote: &str, local: &str) -> bool {
    compare(remote, local) > 0
}

fn compare(a: &str, b: &str) -> i32 {
    let mut ai = a.split('.');
    let mut bi = b.split('.');
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return 0,
            (left, right) => {
                let l: i64 = left.unwrap_or("0").parse().unwrap_or(0);
                let r: i64 = right.unwrap_or("0").parse().unwrap_or(0);
                if l != r {
                    return if l > r { 1 } else { -1 };
                }
            }
        }
    }
}

#[allow(dead_code)]
fn debug_account() -> Account {
    use crate::net::api::{Premium, Profile};
    Account {
        access: String::new(),
        refresh: String::new(),
        profile: Profile {
            username: "admin".into(),
            role: "ADMIN".into(),
            avatar: None,
            premium: Some(Premium { active: true, expires_at: Some(1797600000) }),
            registered_at: Some("2026-10-16T00:00:00Z".into()),
        },
    }
}

#[allow(dead_code)]
fn debug_manifest() -> crate::net::api::Manifest {
    use crate::net::api::{FileInfo, Game, Manifest};
    Manifest {
        game: Some(Game { minecraft: "1.21.11".into(), loader: None, loader_version: None }),
        files: vec![
            FileInfo { file: "client".into(), version: None, size: 9500294, sha256: String::new(), chunk_size: Some(1048576), url: None },
            FileInfo { file: "java".into(), version: None, size: 90122, sha256: String::new(), chunk_size: None, url: Some(String::new()) },
        ],
        maintenance: false,
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map(|_| ())
}
