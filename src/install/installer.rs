use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::paths::Paths;
use crate::install::download::Http;
use crate::install::hash::{matches, Algo};
use crate::net::api::Manifest;
use crate::net::channel::SecureChannel;
use crate::net::error::ApiError;

const MAIN_CLASS: &str = "net.fabricmc.loader.impl.launch.knot.KnotClient";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Checking,
    DownloadClient,
    DownloadJava,
    DownloadGame,
    DownloadMods,
    Verifying,
    Unpacking,
}

#[derive(Deserialize)]
struct GameIndex {
    #[serde(rename = "assetIndex")]
    asset_index: Option<String>,
    #[serde(default)]
    files: Vec<GameEntry>,
}

#[derive(Deserialize)]
struct GameEntry {
    path: String,
    sha256: String,
    #[serde(default)]
    size: u64,
}

struct Item {
    url: String,
    target: PathBuf,
    sha: String,
    size: u64,
    phase: Phase,
}

pub struct Outcome {
    pub java_dir: PathBuf,
    pub game_dir: PathBuf,
    pub client_jar: PathBuf,
    pub classpath: Vec<PathBuf>,
    pub main_class: String,
    pub asset_index: String,
    pub installed: HashMap<String, String>,
}

pub struct Installer<'a> {
    http: Http,
    #[allow(dead_code)]
    channel: &'a SecureChannel,
    #[allow(dead_code)]
    access: &'a str,
    installed: HashMap<String, String>,
}

impl<'a> Installer<'a> {
    pub fn new(channel: &'a SecureChannel, access: &'a str, installed: HashMap<String, String>) -> Self {
        Self {
            http: Http::new(),
            channel,
            access,
            installed,
        }
    }

    pub fn run<F: FnMut(Phase, u64, u64), C: Fn() -> bool>(
        &mut self,
        manifest: &Manifest,
        mut emit: F,
        cancelled: C,
    ) -> Result<Outcome, ApiError> {
        emit(Phase::Checking, 0, 0);
        let game_dir = Paths::game();
        let java_dir = Paths::java();
        let mut items = Vec::new();
        let mut asset_index = String::new();
        let mut keep_mods: Option<HashSet<String>> = None;

        for file in &manifest.files {
            let url = file.url.clone().filter(|u| !u.is_empty());
            match file.file.as_str() {
                "client" => {
                    if let Some(url) = url {
                        items.push(Item {
                            url: local_url(&url),
                            target: Paths::mods().join("client.jar"),
                            sha: file.sha256.clone(),
                            size: file.size,
                            phase: Phase::DownloadClient,
                        });
                    }
                }
                "java" => {
                    if let Some(url) = url {
                        items.push(Item {
                            url: local_url(&url),
                            target: java_dir.join("java.zip"),
                            sha: file.sha256.clone(),
                            size: file.size,
                            phase: Phase::DownloadJava,
                        });
                    }
                }
                "fabric_api" => {
                    if let Some(url) = url {
                        items.push(Item {
                            url: local_url(&url),
                            target: Paths::mods().join("fabric-api.jar"),
                            sha: file.sha256.clone(),
                            size: file.size,
                            phase: Phase::DownloadGame,
                        });
                    }
                }
                "game" => {
                    if let Some(base) = url {
                        let base = local_url(&base);
                        let index: GameIndex = self.http.json(&format!("{base}/index"))?;
                        if let Some(id) = index.asset_index {
                            asset_index = id;
                        }
                        for entry in index.files {
                            items.push(Item {
                                url: format!("{base}/{}", encode_path(&entry.path)),
                                target: game_dir.join(&entry.path),
                                sha: entry.sha256,
                                size: entry.size,
                                phase: Phase::DownloadGame,
                            });
                        }
                    }
                }
                "mods" => {
                    if let Some(base) = url {
                        let base = local_url(&base);
                        let index: GameIndex = self.http.json(&format!("{base}/index"))?;
                        let mut keep = HashSet::new();
                        for entry in index.files {
                            let name = entry.path.rsplit('/').next().unwrap_or(&entry.path).to_owned();
                            keep.insert(name.clone());
                            items.push(Item {
                                url: format!("{base}/{}", encode_path(&name)),
                                target: Paths::mods().join(&name),
                                sha: entry.sha256,
                                size: entry.size,
                                phase: Phase::DownloadMods,
                            });
                        }
                        keep_mods = Some(keep);
                    }
                }
                _ => {}
            }
        }

        let pending: Vec<Item> = items.into_iter().filter(|i| !self.satisfied(i)).collect();
        let total: u64 = pending.iter().map(|i| i.size.max(1)).sum();
        let mut done = 0u64;
        let mut new_installed = self.installed.clone();

        for item in pending {
            if cancelled() {
                return Err(ApiError::new("CANCELLED", 0));
            }
            let base = done;
            let key = item.target.to_string_lossy().into_owned();
            self.http.ensure(&item.url, &item.target, Algo::Sha256, &item.sha, |n| {
                emit(item.phase, base + n, total);
            })?;
            done += item.size.max(1);
            emit(item.phase, done, total);
            if !item.sha.is_empty() {
                new_installed.insert(key, item.sha.clone());
            }
        }

        if let Some(keep) = keep_mods {
            prune_mods(&Paths::mods(), &keep);
        }

        emit(Phase::Unpacking, done, total);
        self.unpack_java(&java_dir)?;

        self.installed = new_installed;
        Ok(Outcome {
            java_dir,
            client_jar: game_dir.join("mods").join("client.jar"),
            classpath: classpath(&game_dir),
            main_class: MAIN_CLASS.to_owned(),
            asset_index,
            game_dir,
            installed: self.installed.clone(),
        })
    }

    fn satisfied(&self, item: &Item) -> bool {
        if item.sha.is_empty() {
            return item.target.exists();
        }
        let key = item.target.to_string_lossy();
        if self.installed.get(key.as_ref()) == Some(&item.sha) && item.target.exists() {
            return true;
        }
        matches(&item.target, Algo::Sha256, &item.sha)
    }

    fn unpack_java(&self, java_dir: &Path) -> Result<(), ApiError> {
        let zip_path = java_dir.join("java.zip");
        let marker = java_dir.join("javaw.exe");
        if marker.exists() {
            return Ok(());
        }
        let file = fs::File::open(&zip_path).map_err(|_| ApiError::new("IO", 0))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|_| ApiError::new("UNPACK", 0))?;
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).map_err(|_| ApiError::new("UNPACK", 0))?;
            let Some(path) = entry.enclosed_name() else { continue };
            let out = java_dir.join(&path);
            if entry.is_dir() {
                let _ = fs::create_dir_all(&out);
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).map_err(|_| ApiError::new("IO", 0))?;
            }
            let mut target = fs::File::create(&out).map_err(|_| ApiError::new("IO", 0))?;
            std::io::copy(&mut entry, &mut target).map_err(|_| ApiError::new("UNPACK", 0))?;
        }
        Ok(())
    }
}

fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn prune_mods(mods_dir: &Path, keep: &HashSet<String>) {
    let Ok(entries) = fs::read_dir(mods_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jar") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == "client.jar" || name == "fabric-api.jar" || keep.contains(name) {
            continue;
        }
        let _ = fs::remove_file(&path);
    }
}

fn classpath(game_dir: &Path) -> Vec<PathBuf> {
    let mut jars = Vec::new();
    for root in ["versions", "libraries"] {
        collect_jars(&game_dir.join(root), &mut jars);
    }
    jars.sort();
    jars
}

fn collect_jars(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jars(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jar") {
            out.push(path);
        }
    }
}

fn local_url(url: &str) -> String {
    let site = crate::config::Config::site_url();
    let api = crate::config::Config::api_url();
    if let Some(rest) = url.strip_prefix(&site) {
        format!("{api}{rest}")
    } else {
        url.to_owned()
    }
}
