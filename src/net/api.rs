use serde::Deserialize;
use serde_json::{json, Value};

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
    pub maintenance: bool,
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
            &json!({ "login": login, "password": password, "hwid": hwid }),
        )
    }

    pub fn refresh(&self, refresh_token: &str, hwid: &str) -> Result<Auth, ApiError> {
        self.typed(
            crate::secret!("/api/v1/launcher/auth/refresh"),
            &json!({ "refreshToken": refresh_token, "hwid": hwid }),
        )
    }

    pub fn logout(&self, access_token: &str, refresh_token: &str) -> Result<(), ApiError> {
        self.channel
            .call(
                crate::secret!("/api/v1/launcher/auth/logout"),
                &json!({ "accessToken": access_token, "refreshToken": refresh_token }),
            )
            .map(|_| ())
    }

    pub fn manifest(&self) -> Result<Manifest, ApiError> {
        self.typed(crate::secret!("/api/v1/launcher/manifest"), &json!({}))
    }

    pub fn session(&self, access_token: &str, hwid: &str) -> Result<GameSession, ApiError> {
        self.typed(
            crate::secret!("/api/v1/launcher/session"),
            &json!({ "accessToken": access_token, "hwid": hwid }),
        )
    }

    pub fn protection(&self, client_token: &str) -> Result<Protection, ApiError> {
        self.typed(
            crate::secret!("/api/v1/client/protection"),
            &json!({ "clientToken": client_token }),
        )
    }

    fn typed<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &Value) -> Result<T, ApiError> {
        let value = self.channel.call(path, body)?;
        serde_json::from_value(value).map_err(|_| ApiError::new("INTERNAL", 0))
    }
}
