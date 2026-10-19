use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::net::crypto::{derive_key, random, Cipher};

const NONCE_LEN: usize = 12;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Persisted {
    pub refresh: Option<String>,
    pub language: Option<String>,
    pub memory: Option<u32>,
    #[serde(default)]
    pub installed: std::collections::HashMap<String, String>,
}

pub struct Storage {
    path: PathBuf,
    cipher: Cipher,
    data: Persisted,
}

impl Storage {
    pub fn load(hwid: &str) -> Self {
        let path = Self::location();
        let cipher = Cipher::new(Self::key(hwid));
        let data = fs::read(&path)
            .ok()
            .and_then(|bytes| Self::decode(&cipher, &bytes))
            .unwrap_or_default();
        Self { path, cipher, data }
    }

    fn key(hwid: &str) -> [u8; 32] {
        derive_key(hwid.as_bytes(), b"alekdlc/store/salt/v1", b"alekdlc/store/key")
    }

    fn decode(cipher: &Cipher, bytes: &[u8]) -> Option<Persisted> {
        if bytes.len() <= NONCE_LEN {
            return None;
        }
        let (nonce, body) = bytes.split_at(NONCE_LEN);
        let plain = cipher.open(nonce, body, &[])?;
        serde_json::from_slice(&plain).ok()
    }

    fn location() -> PathBuf {
        crate::core::paths::Paths::config()
    }

    fn flush(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let Ok(plain) = serde_json::to_vec(&self.data) else {
            return;
        };
        let nonce = random(NONCE_LEN);
        let Some(sealed) = self.cipher.seal(&nonce, &plain, &[]) else {
            return;
        };
        let mut out = nonce;
        out.extend_from_slice(&sealed);
        let _ = fs::write(&self.path, out);
    }

    pub fn installed(&self) -> &std::collections::HashMap<String, String> {
        &self.data.installed
    }

    pub fn set_installed(&mut self, file: &str, sha256: &str) {
        self.data.installed.insert(file.to_owned(), sha256.to_owned());
        self.flush();
    }

    pub fn merge_installed(&mut self, entries: std::collections::HashMap<String, String>) {
        if entries.is_empty() {
            return;
        }
        self.data.installed = entries;
        self.flush();
    }

    pub fn clear_installed(&mut self) {
        self.data.installed.clear();
        self.flush();
    }

    pub fn refresh(&self) -> Option<String> {
        self.data.refresh.clone()
    }

    pub fn set_refresh(&mut self, token: Option<String>) {
        self.data.refresh = token;
        self.flush();
    }

    pub fn language(&self) -> Option<String> {
        self.data.language.clone()
    }

    pub fn set_language(&mut self, language: &str) {
        self.data.language = Some(language.to_owned());
        self.flush();
    }

    pub fn memory(&self) -> Option<u32> {
        self.data.memory
    }

    pub fn set_memory(&mut self, memory: u32) {
        self.data.memory = Some(memory);
        self.flush();
    }
}
