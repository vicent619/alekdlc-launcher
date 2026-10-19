use crate::anim::{Bezier, Tween};
use crate::core::state::{Screen, State};
use crate::geometry::{Point, Rect};
use crate::gfx::canvas::{Canvas, Layer};
use crate::core::state::Screen as ScreenKind;
use crate::i18n::{format, text};
use crate::theme::{Metrics, Palette, Timing};
use crate::ui::home::{HomeAction, HomeScreen};
use crate::ui::interaction::{Command, Pointer};
use crate::ui::login::{LoginAction, LoginScreen};
use crate::ui::message::MessageScreen;
use crate::ui::settings::{SettingsAction, SettingsScreen};
use crate::ui::splash::Splash;
use crate::ui::titlebar::{BarIntent, Titlebar};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    None,
    Minimize,
    Close,
    Drag,
    OpenUrl(String),
    Submit { login: String, password: String },
    Retry,
    Navigate(ScreenKind),
    ToggleMenu,
    CloseMenu,
    Cabinet,
    Logout,
    Play,
    Cancel,
    SetMemory(u32),
    SetLanguage(crate::i18n::Language),
    OpenFolder,
    Verify,
}

pub struct Key {
    pub named: Option<Named>,
    pub text: Option<String>,
    pub shift: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    Backspace,
    Enter,
    Tab,
    Left,
    Right,
    Home,
    End,
    SelectAll,
    Copy,
    Cut,
    Paste,
}

fn clipboard_get() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

fn clipboard_set(text: &str) {
    if text.is_empty() {
        return;
    }
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text.to_owned());
    }
}

pub struct Shell {
    pointer: Pointer,
    titlebar: Titlebar,
    splash: Splash,
    offline: MessageScreen,
    update: MessageScreen,
    login: LoginScreen,
    home: HomeScreen,
    settings: SettingsScreen,
    pending_settings: SettingsAction,
    screen: Screen,
    enter: Tween,
}

impl Shell {
    pub fn new(state: &State) -> Self {
        Self {
            pointer: Pointer::default(),
            titlebar: Titlebar::new(),
            splash: Splash::new(text(state.language, state.splash_key)),
            offline: MessageScreen::offline(),
            update: MessageScreen::update(),
            login: LoginScreen::new(state.language),
            home: HomeScreen::new(),
            settings: SettingsScreen::new(),
            pending_settings: SettingsAction::None,
            screen: state.screen,
            enter: Tween::at(1.0).over(Timing::SLOW).eased(Bezier::EASE).from(0.0),
        }
    }

    pub fn sync(&mut self, state: &State) {
        if self.screen != state.screen {
            self.screen = state.screen;
            self.enter = Tween::at(1.0).over(Timing::SLOW).eased(Bezier::EASE).from(0.0);
            if state.screen == Screen::Login {
                self.login.focus_field(0);
            }
        }
        self.splash.set_status(text(state.language, state.splash_key));
        self.offline.set_label(text(state.language, "offline.retry"));
        self.update.set_label(text(state.language, "update.download"));
        self.login.relabel(state.language);
    }

    pub fn draw(&mut self, canvas: &mut Canvas, state: &State) {
        let window = Rect::new(0.0, 0.0, Metrics::WINDOW_W, Metrics::WINDOW_H);
        canvas.radial(
            window,
            Point::new(Metrics::WINDOW_W * 0.5, 0.0),
            Metrics::WINDOW_W * 0.60,
            Metrics::WINDOW_H * 0.55,
            0.70,
            Palette::GLOW,
        );
        let e = self.enter.value();
        canvas.with(Layer::new().opacity(e).translate(0.0, 8.0 * (1.0 - e)), |c| {
            match state.screen {
                Screen::Splash => self.splash.draw(c),
                Screen::Offline => self.offline.draw(c, &text(state.language, "offline.title"), &text(state.language, "offline.lead")),
                Screen::Update => {
                    let version = state.version.clone().unwrap_or_default();
                    let lead = format(state.language, "update.lead", &[("version", &version)]);
                    self.update.draw(c, &text(state.language, "update.title"), &lead);
                }
                Screen::Login => self.login.draw(c, state),
                Screen::Home => self.home.draw(c, state),
                Screen::Settings => self.settings.draw(c, state),
            }
        });
        self.titlebar.draw(canvas);
        self.titlebar.draw_nav(canvas, state);
        crate::ui::toasts::draw(canvas, state);
        canvas.inset_border(window, Metrics::WINDOW_RADIUS, 1.0, Palette::WINDOW_RING);
    }

    pub fn pointer_moved(&mut self, canvas: &mut Canvas, state: &State, position: Point) {
        self.pointer.position = position;
        self.titlebar.pointer_moved(&self.pointer);
        self.titlebar.track_nav(canvas, state, &self.pointer);
        match state.screen {
            Screen::Offline => self.offline.track(self.offline.button_rect(canvas, &text(state.language, "offline.lead")), &self.pointer),
            Screen::Update => {
                let version = state.version.clone().unwrap_or_default();
                let lead = format(state.language, "update.lead", &[("version", &version)]);
                let rect = self.update.button_rect(canvas, &lead);
                self.update.track(rect, &self.pointer);
            }
            Screen::Login => self.login.track(canvas, state, &self.pointer),
            Screen::Home => self.home.track(canvas, state, &self.pointer),
            Screen::Settings => self.settings.track(canvas, state, &self.pointer),
            _ => {}
        }
    }

    pub fn scroll(&mut self, state: &State, delta: f32) {
        if state.screen == Screen::Settings {
            self.settings.scroll_by(delta);
        }
    }

    pub fn drag(&mut self, state: &State) -> Intent {
        if state.screen == Screen::Settings && self.settings.dragging() {
            return map_settings(self.settings.drag(state, &self.pointer));
        }
        Intent::None
    }

    pub fn pointer_left(&mut self, canvas: &mut Canvas, state: &State) {
        self.pointer_moved(canvas, state, Point::new(-1.0, -1.0));
    }

    pub fn pressed(&mut self, canvas: &mut Canvas, state: &State) -> Command {
        self.pointer.pressed = true;
        if self.titlebar.nav_hit(canvas, state, self.pointer.position) {
            self.titlebar.press_nav(canvas, state, &self.pointer);
            return Command::None;
        }
        let command = self.titlebar.pressed(&self.pointer);
        if command == Command::DragWindow {
            return command;
        }
        match state.screen {
            Screen::Offline => {
                let rect = self.offline.button_rect(canvas, &text(state.language, "offline.lead"));
                self.offline.press(rect, &self.pointer);
            }
            Screen::Update => {
                let version = state.version.clone().unwrap_or_default();
                let lead = format(state.language, "update.lead", &[("version", &version)]);
                let rect = self.update.button_rect(canvas, &lead);
                self.update.press(rect, &self.pointer);
            }
            Screen::Login => self.login.press(canvas, state, &self.pointer),
            Screen::Home => self.home.press(state, &self.pointer),
            Screen::Settings => self.pending_settings = self.settings.press(state, &self.pointer),
            _ => {}
        }
        command
    }

    pub fn take_press_intent(&mut self) -> Intent {
        map_settings(std::mem::replace(&mut self.pending_settings, SettingsAction::None))
    }

    pub fn released(&mut self, canvas: &mut Canvas, state: &State) -> (Command, Intent) {
        self.pointer.pressed = false;
        let bar = self.titlebar.nav_intent(canvas, state, &self.pointer, true);
        if bar != BarIntent::None {
            self.titlebar.track_nav(canvas, state, &self.pointer);
            return (Command::None, map_bar(bar));
        }
        let command = self.titlebar.released(&self.pointer);
        self.titlebar.pointer_moved(&self.pointer);
        let intent = match state.screen {
            Screen::Offline => {
                let rect = self.offline.button_rect(canvas, &text(state.language, "offline.lead"));
                if self.offline.release(rect, &self.pointer) {
                    Intent::Retry
                } else {
                    Intent::None
                }
            }
            Screen::Update => {
                let version = state.version.clone().unwrap_or_default();
                let lead = format(state.language, "update.lead", &[("version", &version)]);
                let rect = self.update.button_rect(canvas, &lead);
                if self.update.release(rect, &self.pointer) {
                    Intent::OpenUrl("/download".into())
                } else {
                    Intent::None
                }
            }
            Screen::Login => match self.login.release(canvas, state, &self.pointer) {
                LoginAction::Submit => self.submit_intent(),
                LoginAction::Forgot => Intent::OpenUrl("/recover".into()),
                LoginAction::None => Intent::None,
            },
            Screen::Home => match self.home.release(canvas, state, &self.pointer) {
                HomeAction::Play => Intent::Play,
                HomeAction::Cancel => Intent::Cancel,
                HomeAction::Buy => Intent::OpenUrl("/products".into()),
                HomeAction::Cabinet => Intent::OpenUrl("/account".into()),
                HomeAction::None => Intent::None,
            },
            Screen::Settings => map_settings(self.settings.release(canvas, state, &self.pointer)),
            _ => Intent::None,
        };
        (command, intent)
    }

    fn submit_intent(&self) -> Intent {
        Intent::Submit {
            login: self.login.login.value.trim().to_owned(),
            password: self.login.password.value.clone(),
        }
    }

    pub fn key(&mut self, state: &State, key: Key) -> Intent {
        if state.screen != Screen::Login {
            return Intent::None;
        }
        if let Some(named) = key.named {
            match named {
                Named::Enter => return self.submit_intent(),
                Named::Tab => self.login.advance_focus(),
                Named::Backspace => self.login.focused().backspace(),
                Named::Left => self.login.focused().move_caret(-1, key.shift),
                Named::Right => self.login.focused().move_caret(1, key.shift),
                Named::Home => self.login.focused().caret_home(key.shift),
                Named::End => self.login.focused().caret_end(key.shift),
                Named::SelectAll => self.login.focused().select_all(),
                Named::Copy => clipboard_set(&self.login.focused().selected_text()),
                Named::Cut => {
                    let field = self.login.focused();
                    let selected = field.selected_text();
                    if !selected.is_empty() {
                        clipboard_set(&selected);
                        field.backspace();
                    }
                }
                Named::Paste => {
                    if let Some(text) = clipboard_get() {
                        self.login.focused().insert(&text);
                    }
                }
            }
            return Intent::None;
        }
        if let Some(text) = key.text {
            self.login.focused().insert(&text);
        }
        Intent::None
    }

    pub fn tick(&mut self, dt: f32) {
        self.login.tick(dt);
    }

    pub fn animating(&self, state: &State) -> bool {
        self.enter.running()
            || !state.toasts.is_empty()
            || self.titlebar.animating()
            || self.titlebar.nav_animating(state.menu_open)
            || match state.screen {
                Screen::Splash => true,
                Screen::Offline => self.offline.animating(),
                Screen::Update => self.update.animating(),
                Screen::Login => self.login.animating(),
                Screen::Home => self.home.animating() || state.play.busy,
                Screen::Settings => self.settings.animating(),
                _ => false,
            }
    }
}

fn map_settings(action: SettingsAction) -> Intent {
    match action {
        SettingsAction::Memory(value) => Intent::SetMemory(value),
        SettingsAction::Language(lang) => Intent::SetLanguage(lang),
        SettingsAction::OpenFolder => Intent::OpenFolder,
        SettingsAction::Verify => Intent::Verify,
        SettingsAction::OpenUrl(path) => Intent::OpenUrl(path),
        SettingsAction::None => Intent::None,
    }
}

fn map_bar(bar: BarIntent) -> Intent {
    match bar {
        BarIntent::Navigate(screen) => Intent::Navigate(screen),
        BarIntent::ToggleMenu => Intent::ToggleMenu,
        BarIntent::CloseMenu => Intent::CloseMenu,
        BarIntent::Cabinet => Intent::Cabinet,
        BarIntent::Logout => Intent::Logout,
        BarIntent::None => Intent::None,
    }
}
