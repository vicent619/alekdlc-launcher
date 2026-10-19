use std::time::Instant;

use crate::i18n::Language;
use crate::install::installer::Phase;
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
pub enum ClientBuild {
    Free,
    Premium,
}

impl ClientBuild {
    pub fn code(self) -> String {
        match self {
            ClientBuild::Free => crate::secret!("free").to_owned(),
            ClientBuild::Premium => crate::secret!("premium").to_owned(),
        }
    }

    pub fn default_java(self) -> String {
        match self {
            ClientBuild::Free => crate::secret!("java_free").to_owned(),
            ClientBuild::Premium => crate::secret!("java").to_owned(),
        }
    }

    pub fn parse(code: &str) -> Self {
        if code == crate::secret!("premium") {
            ClientBuild::Premium
        } else {
            ClientBuild::Free
        }
    }
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
    pub phase: Option<Phase>,
    pub since: Option<Instant>,
}

pub struct State {
    pub screen: Screen,
    pub splash_key: String,
    pub language: Language,
    pub version: Option<String>,
    pub maintenance: bool,
    pub account: Option<Account>,
    pub manifest: Option<Manifest>,
    pub pending: Option<u64>,
    pub login: LoginForm,
    pub play: PlayState,
    pub memory: u32,
    pub build: ClientBuild,
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
        self.pending.unwrap_or(0)
    }

    pub fn checking(&self) -> bool {
        self.manifest.is_some() && self.pending.is_none()
    }

    pub fn manifest_bytes(&self) -> u64 {
        self.manifest
            .as_ref()
            .map(|m| m.files.iter().map(|f| f.size).sum())
            .unwrap_or(0)
    }

    pub fn has_file(&self, name: &str) -> bool {
        self.manifest
            .as_ref()
            .map(|m| m.files.iter().any(|f| f.file == name && f.size > 0))
            .unwrap_or(false)
    }

    pub fn premium_active(&self) -> bool {
        self.account
            .as_ref()
            .and_then(|a| a.profile.premium.as_ref())
            .map(|p| p.active)
            .unwrap_or(false)
    }

    pub fn selected_build(&self) -> ClientBuild {
        if self.build == ClientBuild::Premium && !self.premium_active() {
            ClientBuild::Free
        } else {
            self.build
        }
    }

    pub fn java_file(&self) -> String {
        let build = self.selected_build();
        let code = build.code();
        self.manifest
            .as_ref()
            .and_then(|m| m.builds.iter().find(|b| b.id == code))
            .and_then(|b| b.java.clone())
            .unwrap_or_else(|| build.default_java())
    }

    pub fn client_file(&self) -> Option<String> {
        let code = self.selected_build().code();
        let client = self
            .manifest
            .as_ref()
            .and_then(|m| m.builds.iter().find(|b| b.id == code))
            .and_then(|b| b.client.clone())?;
        if self.has_file(&client) {
            Some(client)
        } else {
            None
        }
    }

    pub fn runtime(&self) -> crate::install::installer::Runtime {
        crate::install::installer::Runtime {
            file: self.java_file(),
            folder: self.selected_build().code(),
            client: self.client_file(),
        }
    }

    pub fn build_available(&self, build: ClientBuild) -> bool {
        let code = build.code();
        self.manifest
            .as_ref()
            .map(|m| m.builds.iter().any(|b| b.id == code && b.available))
            .unwrap_or(false)
    }

    pub fn has_required(&self) -> bool {
        self.manifest.is_some()
            && self.build_available(self.selected_build())
            && self.has_file(&self.java_file())
            && self.has_file(crate::secret!("fabric_api"))
            && self.has_file(crate::secret!("game"))
            && self.has_file(crate::secret!("mods"))
    }

    pub fn game_version(&self) -> Option<String> {
        self.manifest
            .as_ref()
            .and_then(|m| m.game.as_ref())
            .map(|g| g.minecraft.clone())
    }

    pub fn bypasses_maintenance(&self) -> bool {
        let Some(account) = self.account.as_ref() else {
            return false;
        };
        let role = account.profile.role.as_str();
        role == crate::secret!("ADMIN") || role == crate::secret!("MODERATOR")
    }
}

impl State {
    pub fn new(language: Language, memory: u32) -> Self {
        Self {
            screen: Screen::Splash,
            splash_key: crate::secret!("splash.connecting").to_owned(),
            language,
            version: None,
            maintenance: false,
            account: None,
            manifest: None,
            pending: None,
            login: LoginForm::default(),
            play: PlayState::default(),
            memory,
            build: ClientBuild::Free,
            menu_open: false,
            toasts: Vec::new(),
            next_toast: 0,
        }
    }
}
