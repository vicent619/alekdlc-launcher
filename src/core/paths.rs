use std::path::PathBuf;

pub struct Paths;

impl Paths {
    pub fn base() -> PathBuf {
        std::env::var_os(crate::secret!("APPDATA"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(crate::secret!("alekdlc"))
    }

    pub fn config() -> PathBuf {
        Self::base().join(crate::secret!("data"))
    }

    pub fn temp() -> PathBuf {
        Self::base().join(crate::secret!("temp"))
    }

    pub fn game() -> PathBuf {
        Self::base().join(crate::secret!("game"))
    }

    pub fn java_root() -> PathBuf {
        Self::base().join(crate::secret!("java"))
    }

    pub fn java(folder: &str) -> PathBuf {
        Self::java_root().join(folder)
    }

    pub fn mods() -> PathBuf {
        Self::game().join(crate::secret!("mods"))
    }

    pub fn game_display() -> String {
        Self::game().to_string_lossy().into_owned()
    }
}
