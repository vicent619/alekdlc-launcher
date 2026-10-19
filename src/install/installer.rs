use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;

use serde::Deserialize;

use crate::core::paths::Paths;
use crate::install::archive::{Archive, Entry};
use crate::install::download::{pause, transient, Http, Partial, LARGE};
use crate::install::hash::{matches, Algo};
use crate::install::pool::{Pool, Signal};
use crate::net::api::Manifest;
use crate::net::error::ApiError;

const WORKERS: usize = 6;
const FILE_WORKERS: usize = 12;
const ATTEMPTS: u32 = 4;
const RETRY_DELAY: Duration = Duration::from_millis(1500);
const ARCHIVE_MIN_FILES: usize = 32;
const ARCHIVE_MANY_FILES: usize = 800;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Checking,
    ConnectingClient,
    WaitingClient,
    DownloadClient,
    DownloadJava,
    DownloadGame,
    DownloadMods,
    Unpacking,
    Verifying,
}

#[derive(Deserialize)]
struct GameIndex {
    #[serde(rename = "assetIndex")]
    asset_index: Option<String>,
    #[serde(default)]
    size: u64,
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
    origin: Option<Origin>,
}

struct Origin {
    bundle: usize,
    entry: String,
}

impl Item {
    fn bundled(base: &str, path: &str, target: PathBuf, sha: String, size: u64, phase: Phase) -> Self {
        Self {
            url: format!("{base}/{}", encode_path(path)),
            target,
            sha,
            size,
            phase,
            origin: None,
        }
    }

    fn from(mut self, bundle: usize, entry: &str) -> Self {
        self.origin = Some(Origin { bundle, entry: entry.to_owned() });
        self
    }

    fn weight(&self) -> u64 {
        self.size.max(1)
    }

    fn belongs(&self, bundle: usize) -> bool {
        self.origin.as_ref().is_some_and(|origin| origin.bundle == bundle)
    }
}

struct Bundle {
    base: String,
    bytes: u64,
}

impl Bundle {
    fn name(&self) -> &str {
        self.base.rsplit('/').next().unwrap_or_default()
    }

    fn worth(&self, items: &[&Item]) -> bool {
        let bytes: u64 = items.iter().map(|item| item.weight()).sum();
        items.len() >= ARCHIVE_MANY_FILES || (items.len() >= ARCHIVE_MIN_FILES && bytes * 2 >= self.bytes)
    }
}

struct Job {
    bundle: usize,
    slots: Vec<usize>,
    share: u64,
    transfer: u64,
}

impl Job {
    fn progress(&self, received: u64, written: u64) -> u64 {
        received.min(self.transfer) + scale(written, self.share, self.share - self.transfer)
    }
}

enum Note {
    Received(usize, u64),
    Fetched(usize, Result<Partial, ApiError>),
    Written(usize, usize),
    Extracted(Result<(), ApiError>),
    Drained,
}

struct Plan {
    items: Vec<Item>,
    bundles: Vec<Bundle>,
    asset_index: String,
    keep_mods: Option<HashSet<String>>,
    keep_java: Option<HashSet<PathBuf>>,
}

impl Plan {
    fn bundle(&mut self, base: &str, index: &GameIndex) -> usize {
        let bytes = if index.size > 0 { index.size } else { index.files.iter().map(|entry| entry.size).sum() };
        self.bundles.push(Bundle { base: base.to_owned(), bytes });
        self.bundles.len() - 1
    }
}

struct Meter {
    settled: u64,
    total: u64,
}

impl Meter {
    fn at(&self, extra: u64) -> u64 {
        (self.settled + extra).min(self.total)
    }
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

#[derive(Clone, Debug)]
pub struct Runtime {
    pub file: String,
    pub folder: String,
    pub client: Option<String>,
}

pub struct Installer {
    http: Http,
    installed: HashMap<String, String>,
    runtime: Runtime,
}

impl Installer {
    pub fn new(installed: HashMap<String, String>, runtime: Runtime) -> Self {
        Self {
            http: Http::new(),
            installed,
            runtime,
        }
    }

    pub fn installed(&self) -> &HashMap<String, String> {
        &self.installed
    }

    pub fn pending_bytes(&mut self, manifest: &Manifest) -> Result<u64, ApiError> {
        let plan = self.plan(manifest)?;
        Ok(self.pending(plan.items).iter().map(|item| item.size).sum())
    }

    fn plan(&self, manifest: &Manifest) -> Result<Plan, ApiError> {
        let game_dir = Paths::game();
        let java_dir = Paths::java(&self.runtime.folder);
        let mut plan = Plan {
            items: Vec::new(),
            bundles: Vec::new(),
            asset_index: String::new(),
            keep_mods: None,
            keep_java: None,
        };

        for file in &manifest.files {
            let Some(url) = file.url.as_deref().filter(|u| !u.is_empty()).map(local_url) else {
                continue;
            };
            let kind = file.file.as_str();
            if self.runtime.client.as_deref() == Some(kind) {
                plan.items.push(Item {
                    url,
                    target: Paths::mods().join(crate::secret!("client.jar")),
                    sha: file.sha256.clone(),
                    size: file.size,
                    phase: Phase::DownloadClient,
                    origin: None,
                });
            } else if kind == self.runtime.file {
                let index = self.index(&url)?;
                let bundle = plan.bundle(&url, &index);
                let mut keep = HashSet::new();
                for entry in index.files {
                    let target = java_dir.join(&entry.path);
                    keep.insert(target.clone());
                    plan.items.push(Item::bundled(&url, &entry.path, target, entry.sha256, entry.size, Phase::DownloadJava).from(bundle, &entry.path));
                }
                plan.keep_java = Some(keep);
            } else if kind == crate::secret!("fabric_api") {
                plan.items.push(Item {
                    url,
                    target: Paths::mods().join(crate::secret!("fabric-api.jar")),
                    sha: file.sha256.clone(),
                    size: file.size,
                    phase: Phase::DownloadGame,
                    origin: None,
                });
            } else if kind == crate::secret!("game") {
                let index = self.index(&url)?;
                let bundle = plan.bundle(&url, &index);
                if let Some(id) = index.asset_index {
                    plan.asset_index = id;
                }
                for entry in index.files {
                    let target = game_dir.join(&entry.path);
                    plan.items.push(Item::bundled(&url, &entry.path, target, entry.sha256, entry.size, Phase::DownloadGame).from(bundle, &entry.path));
                }
            } else if kind == crate::secret!("mods") {
                let index = self.index(&url)?;
                let bundle = plan.bundle(&url, &index);
                let mut keep = HashSet::new();
                for entry in index.files {
                    let name = entry.path.rsplit('/').next().unwrap_or(&entry.path).to_owned();
                    keep.insert(name.clone());
                    let target = Paths::mods().join(&name);
                    plan.items.push(Item::bundled(&url, &name, target, entry.sha256, entry.size, Phase::DownloadMods).from(bundle, &entry.path));
                }
                plan.keep_mods = Some(keep);
            }
        }
        Ok(plan)
    }

    fn index(&self, base: &str) -> Result<GameIndex, ApiError> {
        self.http.json(&format!("{base}/{}", crate::secret!("index")))
    }

    pub fn run<F: FnMut(Phase, u64, u64), C: Fn() -> bool + Sync>(
        &mut self,
        manifest: &Manifest,
        mut emit: F,
        cancelled: C,
    ) -> Result<Outcome, ApiError> {
        emit(Phase::Checking, 0, 0);
        let game_dir = Paths::game();
        let java_dir = Paths::java(&self.runtime.folder);
        let plan = self.plan(manifest)?;
        let pending = self.pending(plan.items);
        let mut meter = Meter {
            settled: 0,
            total: pending.iter().map(Item::weight).sum(),
        };

        let mut jobs = Vec::new();
        for (index, bundle) in plan.bundles.iter().enumerate() {
            let slots: Vec<usize> = (0..pending.len()).filter(|&slot| pending[slot].belongs(index)).collect();
            let items: Vec<&Item> = slots.iter().map(|&slot| &pending[slot]).collect();
            if !bundle.worth(&items) {
                continue;
            }
            let share: u64 = items.iter().map(|item| item.weight()).sum();
            jobs.push(Job { bundle: index, slots, share, transfer: share / 10 * 9 });
        }
        let mut unpacked = vec![false; pending.len()];
        for slot in self.unpack(&plan.bundles, &jobs, &pending, &mut meter, &mut emit, &cancelled)? {
            unpacked[slot] = true;
        }

        let rest: Vec<&Item> = pending.iter().zip(&unpacked).filter(|(_, done)| !**done).map(|(item, _)| item).collect();
        let mut phases = Vec::new();
        for item in &rest {
            if !phases.contains(&item.phase) {
                phases.push(item.phase);
            }
        }
        for phase in phases {
            let group: Vec<&Item> = rest.iter().copied().filter(|item| item.phase == phase).collect();
            self.download(&group, phase, &mut meter, &mut emit, &cancelled)?;
        }

        if let Some(keep) = plan.keep_mods {
            prune_mods(&Paths::mods(), &keep);
        }
        prune_legacy_java();
        if let Some(keep) = plan.keep_java {
            prune_tree(&java_dir, &keep);
            self.installed.retain(|key, _| !Path::new(key).starts_with(&java_dir) || keep.contains(Path::new(key)));
        }

        Ok(Outcome {
            java_dir,
            client_jar: Paths::mods().join(crate::secret!("client.jar")),
            classpath: classpath(&game_dir),
            main_class: crate::secret!("net.fabricmc.loader.impl.launch.knot.KnotClient").to_owned(),
            asset_index: plan.asset_index,
            game_dir,
            installed: self.installed.clone(),
        })
    }

    fn unpack<F: FnMut(Phase, u64, u64), C: Fn() -> bool + Sync>(
        &mut self,
        bundles: &[Bundle],
        jobs: &[Job],
        pending: &[Item],
        meter: &mut Meter,
        emit: &mut F,
        cancelled: &C,
    ) -> Result<Vec<usize>, ApiError> {
        if jobs.is_empty() {
            return Ok(Vec::new());
        }
        let http = &self.http;
        let mut extracted = Vec::new();
        let mut received = vec![0u64; jobs.len()];
        let mut written = vec![0u64; jobs.len()];
        let mut downloading = Some(0);
        let mut failure = None;
        thread::scope(|scope| {
            let (notes, inbox) = channel::<Note>();
            let (ready, queue) = channel::<(usize, Partial)>();
            let fetcher = notes.clone();
            scope.spawn(move || {
                for (index, job) in jobs.iter().enumerate() {
                    if cancelled() {
                        break;
                    }
                    let bundle = &bundles[job.bundle];
                    let scratch = Partial::new(&Paths::base().join(format!("{}{}", bundle.name(), crate::secret!(".zip"))));
                    let result = http.archive(
                        &format!("{}/{}", bundle.base, crate::secret!("archive")),
                        &scratch.path,
                        |done, length| {
                            let _ = fetcher.send(Note::Received(index, scale(done, length, job.transfer)));
                        },
                        cancelled,
                    );
                    let _ = fetcher.send(Note::Fetched(index, result.map(|()| scratch)));
                }
                let _ = fetcher.send(Note::Drained);
            });
            let extractor = notes.clone();
            scope.spawn(move || {
                for (index, scratch) in queue {
                    let entries: Vec<Entry> = jobs[index]
                        .slots
                        .iter()
                        .map(|&slot| {
                            let item = &pending[slot];
                            Entry {
                                name: item.origin.as_ref().map(|origin| origin.entry.clone()).unwrap_or_default(),
                                target: item.target.clone(),
                                sha: item.sha.clone(),
                            }
                        })
                        .collect();
                    let result = Archive::new(&scratch.path).extract(&entries, &Pool::new(WORKERS), cancelled, |position| {
                        let _ = extractor.send(Note::Written(index, position));
                    });
                    drop(scratch);
                    let _ = extractor.send(Note::Extracted(result));
                }
            });
            drop(notes);
            let mut ready = Some(ready);
            for note in inbox {
                match note {
                    Note::Received(index, bytes) => {
                        received[index] = bytes;
                        downloading = Some(index);
                    }
                    Note::Fetched(index, Ok(scratch)) => {
                        received[index] = jobs[index].transfer;
                        downloading = None;
                        if let Some(ready) = &ready {
                            let _ = ready.send((index, scratch));
                        }
                    }
                    Note::Fetched(index, Err(error)) => {
                        received[index] = 0;
                        downloading = None;
                        if error.is(crate::secret!("CANCELLED")) {
                            failure.get_or_insert(error);
                        }
                    }
                    Note::Written(index, position) => {
                        let slot = jobs[index].slots[position];
                        extracted.push(slot);
                        written[index] += pending[slot].weight();
                    }
                    Note::Extracted(Err(error)) if error.is(crate::secret!("CANCELLED")) => {
                        failure.get_or_insert(error);
                    }
                    Note::Extracted(_) => {}
                    Note::Drained => {
                        ready = None;
                    }
                }
                let phase = downloading.map_or(Phase::Unpacking, |index| pending[jobs[index].slots[0]].phase);
                let done: u64 = jobs.iter().enumerate().map(|(index, job)| job.progress(received[index], written[index])).sum();
                emit(phase, meter.at(done), meter.total);
            }
        });
        for &slot in &extracted {
            self.remember(&pending[slot].target, &pending[slot].sha);
        }
        meter.settled += written.iter().sum::<u64>();
        match failure {
            Some(error) => Err(error),
            None => Ok(extracted),
        }
    }

    fn download<F: FnMut(Phase, u64, u64), C: Fn() -> bool + Sync>(
        &mut self,
        items: &[&Item],
        phase: Phase,
        meter: &mut Meter,
        emit: &mut F,
        cancelled: &C,
    ) -> Result<(), ApiError> {
        let (large, mut small): (Vec<&Item>, Vec<&Item>) = items.iter().copied().partition(|item| item.size >= LARGE);
        small.sort_by(|a, b| b.size.cmp(&a.size));
        self.transfer(&large, 1, phase, meter, emit, cancelled)?;
        self.transfer(&small, FILE_WORKERS, phase, meter, emit, cancelled)
    }

    fn transfer<F: FnMut(Phase, u64, u64), C: Fn() -> bool + Sync>(
        &mut self,
        items: &[&Item],
        workers: usize,
        phase: Phase,
        meter: &mut Meter,
        emit: &mut F,
        cancelled: &C,
    ) -> Result<(), ApiError> {
        let http = &self.http;
        let mut flight: HashMap<usize, u64> = HashMap::new();
        let mut finished = Vec::new();
        let result = Pool::new(workers).run(
            items,
            || Ok(()),
            |_, item, report| fetch(http, item, report, cancelled),
            |signal| {
                match signal {
                    Signal::Progress(index, bytes) => {
                        flight.insert(index, bytes.min(items[index].size));
                    }
                    Signal::Finished(index) => {
                        flight.remove(&index);
                        meter.settled += items[index].weight();
                        finished.push(index);
                    }
                }
                emit(phase, meter.at(flight.values().sum()), meter.total);
            },
        );
        for index in finished {
            self.remember(&items[index].target, &items[index].sha);
        }
        result
    }

    fn pending(&mut self, items: Vec<Item>) -> Vec<Item> {
        let mut verdicts: Vec<Option<bool>> = items.iter().map(|item| self.known(item)).collect();
        let suspects: Vec<usize> = (0..items.len()).filter(|&index| verdicts[index].is_none()).collect();
        let intact: Vec<AtomicBool> = items.iter().map(|_| AtomicBool::new(false)).collect();
        let hashers = thread::available_parallelism().map_or(4, |count| count.get()).clamp(2, 8);
        let _ = Pool::new(hashers).run(
            &suspects,
            || Ok(()),
            |_, &index, _| {
                let item = &items[index];
                intact[index].store(matches(&item.target, Algo::Sha256, &item.sha), Ordering::Relaxed);
                Ok(())
            },
            |_| {},
        );
        for &index in &suspects {
            let ok = intact[index].load(Ordering::Relaxed);
            if ok {
                self.remember(&items[index].target, &items[index].sha);
            }
            verdicts[index] = Some(ok);
        }
        items.into_iter().zip(verdicts).filter(|(_, verdict)| *verdict != Some(true)).map(|(item, _)| item).collect()
    }

    fn known(&self, item: &Item) -> Option<bool> {
        if item.sha.is_empty() {
            return Some(item.target.exists());
        }
        let Ok(meta) = fs::metadata(&item.target) else {
            return Some(false);
        };
        if !meta.is_file() || (item.size > 0 && meta.len() != item.size) {
            return Some(false);
        }
        let key = item.target.to_string_lossy();
        (self.installed.get(key.as_ref()) == Some(&stamp(&item.sha, &meta))).then_some(true)
    }

    fn remember(&mut self, path: &Path, sha: &str) {
        if sha.is_empty() {
            return;
        }
        if let Ok(meta) = fs::metadata(path) {
            self.installed.insert(path.to_string_lossy().into_owned(), stamp(sha, &meta));
        }
    }
}

fn fetch<C: Fn() -> bool + Sync>(http: &Http, item: &Item, report: &dyn Fn(u64), cancelled: &C) -> Result<(), ApiError> {
    let mut attempt = 1;
    loop {
        match http.ensure(&item.url, &item.target, &item.sha, item.size, report, cancelled) {
            Err(error) if attempt < ATTEMPTS && transient(&error) => {
                report(0);
                pause(RETRY_DELAY * attempt, cancelled)?;
                attempt += 1;
            }
            result => return result,
        }
    }
}

fn scale(value: u64, of: u64, span: u64) -> u64 {
    if of == 0 {
        return value.min(span);
    }
    (u128::from(value.min(of)) * u128::from(span) / u128::from(of)) as u64
}

fn stamp(sha: &str, meta: &fs::Metadata) -> String {
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{sha}:{}:{modified}", meta.len())
}

fn prune_legacy_java() {
    let Ok(entries) = fs::read_dir(Paths::java_root()) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_dir() && (name == crate::secret!("free") || name == crate::secret!("premium")) {
            continue;
        }
        if path.is_dir() {
            let _ = fs::remove_dir_all(&path);
        } else {
            let _ = fs::remove_file(&path);
        }
    }
}

fn prune_tree(dir: &Path, keep: &HashSet<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            prune_tree(&path, keep);
            let _ = fs::remove_dir(&path);
        } else if !keep.contains(&path) {
            let _ = fs::remove_file(&path);
        }
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
        if !is_jar(&path) {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == crate::secret!("client.jar") || name == crate::secret!("fabric-api.jar") || keep.contains(name) {
            continue;
        }
        let _ = fs::remove_file(&path);
    }
}

fn classpath(game_dir: &Path) -> Vec<PathBuf> {
    let mut jars = Vec::new();
    collect_jars(&game_dir.join(crate::secret!("versions")), &mut jars);
    collect_jars(&game_dir.join(crate::secret!("libraries")), &mut jars);
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
        } else if is_jar(&path) {
            out.push(path);
        }
    }
}

fn is_jar(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some(crate::secret!("jar"))
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
