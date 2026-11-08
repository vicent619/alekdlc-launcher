use serde::Deserialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::net::channel::SecureChannel;
use crate::net::error::ApiError;

#[derive(Clone, Debug, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    #[serde(default)]
    pub maintenance: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Premium {
    #[serde(default)]
    pub active: bool,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<i64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Profile {
    pub username: String,
    pub role: String,
    pub avatar: Option<String>,
    pub premium: Option<Premium>,
    #[serde(rename = "registeredAt")]
    pub registered_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Auth {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub profile: Profile,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Game {
    pub minecraft: String,
    pub loader: Option<String>,
    #[serde(rename = "loaderVersion")]
    pub loader_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileInfo {
    pub file: String,
    pub version: Option<String>,
    pub size: u64,
    pub sha256: String,
    #[serde(rename = "chunkSize")]
    pub chunk_size: Option<u64>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GameSession {
    #[serde(rename = "clientToken")]
    pub client_token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Protection {
    pub token: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub game: Option<Game>,
    #[serde(default)]
    pub files: Vec<FileInfo>,
    #[serde(default)]
    pub builds: Vec<BuildInfo>,
    #[serde(default)]
    pub maintenance: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BuildInfo {
    pub id: String,
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub java: Option<String>,
    #[serde(default)]
    pub available: bool,
}

pub struct Api<'a> {
    channel: &'a SecureChannel,
}

impl<'a> Api<'a> {
    pub fn new(channel: &'a SecureChannel) -> Self {
        Self { channel }
    }

    pub fn version(&self) -> Result<VersionInfo, ApiError> {
        self.typed(crate::secret!("/api/v1/launcher/version"), &json!({}))
    }

    pub fn login(&self, login: &str, password: &str, hwid: &str) -> Result<Auth, ApiError> {
        self.typed(
            crate::secret!("/api/v1/launcher/auth/login"),
            &json!({ crate::secret!("login"): login, crate::secret!("password"): password, crate::secret!("hwid"): hwid, crate::secret!("launcher"): Config::version() }),
        )
    }

    pub fn refresh(&self, refresh_token: &str, hwid: &str) -> Result<Auth, ApiError> {
        self.typed(
            crate::secret!("/api/v1/launcher/auth/refresh"),
            &json!({ crate::secret!("refreshToken"): refresh_token, crate::secret!("hwid"): hwid, crate::secret!("launcher"): Config::version() }),
        )
    }

    pub fn logout(&self, access_token: &str, refresh_token: &str) -> Result<(), ApiError> {
        self.channel
            .call(
                crate::secret!("/api/v1/launcher/auth/logout"),
                &json!({ crate::secret!("accessToken"): access_token, crate::secret!("refreshToken"): refresh_token }),
            )
            .map(|_| ())
    }

    pub fn manifest(&self, access_token: &str) -> Result<Manifest, ApiError> {
        self.typed(crate::secret!("/api/v1/launcher/manifest"), &json!({ crate::secret!("accessToken"): access_token }))
    }

    pub fn session(&self, access_token: &str, hwid: &str) -> Result<GameSession, ApiError> {
        self.typed(
            crate::secret!("/api/v1/launcher/session"),
            &json!({ crate::secret!("accessToken"): access_token, crate::secret!("hwid"): hwid, crate::secret!("launcher"): Config::version() }),
        )
    }

    pub fn protection(&self, client_token: &str, build: &str) -> Result<Protection, ApiError> {
        self.typed(
            crate::secret!("/api/v1/client/protection"),
            &json!({ crate::secret!("clientToken"): client_token, crate::secret!("build"): build }),
        )
    }

    fn typed<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &Value) -> Result<T, ApiError> {
        let value = self.channel.call(path, body)?;
        serde_json::from_value(value).map_err(|_| ApiError::new(crate::secret!("INTERNAL"), 0))
    }
}
