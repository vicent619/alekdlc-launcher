use std::time::Instant;

use crate::i18n::Language;
use crate::net::api::{Manifest, Profile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub kind: ToastKind,
    pub born: Instant,
    pub dying: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Splash,
    Offline,
    Update,
    Login,
    Home,
    Settings,
}

#[derive(Clone, Debug)]
pub struct Account {
    pub access: String,
    pub refresh: String,
    pub profile: Profile,
}

#[derive(Clone, Debug, Default)]
pub struct LoginForm {
    pub error: Option<String>,
    pub invalid: bool,
    pub busy: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PlayState {
    pub busy: bool,
    pub label: String,
    pub detail: String,
    pub progress: Option<f32>,
    pub cancellable: bool,
    pub error: Option<String>,
}

pub struct State {
    pub screen: Screen,
    pub splash_key: &'static str,
    pub language: Language,
    pub version: Option<String>,
    pub maintenance: bool,
    pub account: Option<Account>,
    pub manifest: Option<Manifest>,
    pub login: LoginForm,
    pub play: PlayState,
    pub memory: u32,
    pub menu_open: bool,
    pub toasts: Vec<Toast>,
    next_toast: u64,
}

impl State {
    pub fn toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.next_toast += 1;
        self.toasts.push(Toast {
            id: self.next_toast,
            text: text.into(),
            kind,
            born: Instant::now(),
            dying: None,
        });
    }

    pub fn expire_toasts(&mut self) {
        let now = Instant::now();
        for toast in &mut self.toasts {
            if toast.dying.is_none() && now.duration_since(toast.born).as_secs_f32() > 3.6 {
                toast.dying = Some(now);
            }
        }
        self.toasts.retain(|t| {
            t.dying.map(|d| now.duration_since(d).as_secs_f32() < 0.22).unwrap_or(true)
        });
    }
}

impl State {
    pub fn pending_bytes(&self) -> u64 {
        self.manifest
            .as_ref()
            .map(|m| m.files.iter().map(|f| f.size).sum())
            .unwrap_or(0)
    }

    pub fn has_client(&self) -> bool {
        self.has_file("client")
    }

    pub fn has_file(&self, name: &str) -> bool {
        self.manifest
            .as_ref()
            .map(|m| m.files.iter().any(|f| f.file == name && f.size > 0))
            .unwrap_or(false)
    }

    pub fn has_required(&self) -> bool {
        self.manifest.is_some()
            && self.has_file("client")
            && self.has_file("java")
            && self.has_file("fabric_api")
            && self.has_file("game")
            && self.has_file("mods")
    }

    pub fn game_version(&self) -> Option<String> {
        self.manifest
            .as_ref()
            .and_then(|m| m.game.as_ref())
            .map(|g| g.minecraft.clone())
    }

    pub fn is_staff(&self) -> bool {
        matches!(
            self.account.as_ref().map(|a| a.profile.role.as_str()),
            Some("ADMIN") | Some("MODERATOR")
        )
    }
}

impl State {
    pub fn new(language: Language, memory: u32) -> Self {
        Self {
            screen: Screen::Splash,
            splash_key: "splash.connecting",
            language,
            version: None,
            maintenance: false,
            account: None,
            manifest: None,
            login: LoginForm::default(),
            play: PlayState::default(),
            memory,
            menu_open: false,
            toasts: Vec::new(),
            next_toast: 0,
        }
    }
}
