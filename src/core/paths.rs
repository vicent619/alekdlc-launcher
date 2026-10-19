use std::path::PathBuf;

pub struct Paths;

impl Paths {
    pub fn base() -> PathBuf {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("alekdlc")
    }

    pub fn config() -> PathBuf {
        Self::base().join("data")
    }

    pub fn game() -> PathBuf {
        Self::base().join("game")
    }

    pub fn java() -> PathBuf {
        Self::base().join("java")
    }

    pub fn mods() -> PathBuf {
        Self::game().join("mods")
    }

    pub fn game_display() -> String {
        Self::game().to_string_lossy().into_owned()
    }
}
