use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use ureq::http::header::{CONTENT_RANGE, RANGE};
use ureq::http::Response;
use ureq::{Agent, Body};
use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};

use crate::core::paths::Paths;
use crate::install::hash::{file_digest, hex, Algo};
use crate::install::pool::{Pool, Signal};
use crate::net::error::ApiError;

pub const LARGE: u64 = 32 * 1024 * 1024;

const BUFFER: usize = 131072;
const ARCHIVE_TIMEOUT: Duration = Duration::from_secs(3600);
const SEGMENT: u64 = 8 * 1024 * 1024;
const SEGMENT_TIMEOUT: Duration = Duration::from_secs(600);
const SEGMENT_ATTEMPTS: u32 = 5;
const ARCHIVE_PARTS: usize = 8;
const FILE_PARTS: usize = 4;
const RETRY_DELAY: Duration = Duration::from_millis(1500);
const POLL: Duration = Duration::from_millis(100);

pub struct Http {
    agent: Agent,
}

impl Http {
    pub fn new() -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .user_agent(crate::secret!("alekdlc-launcher"))
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }

    pub fn json<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let mut response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
        if response.status().as_u16() >= 400 {
            return Err(ApiError::new(crate::secret!("NOT_FOUND"), response.status().as_u16()));
        }
        response
            .body_mut()
            .read_json::<T>()
            .map_err(|_| ApiError::new(crate::secret!("INTERNAL"), 0))
    }

    pub fn text(&self, url: &str) -> Result<String, ApiError> {
        let mut response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
        if response.status().as_u16() >= 400 {
            return Err(ApiError::new(crate::secret!("NOT_FOUND"), response.status().as_u16()));
        }
        response
            .body_mut()
            .read_to_string()
            .map_err(|_| ApiError::offline())
    }

    pub fn ensure<F: FnMut(u64), C: Fn() -> bool + Sync>(
        &self,
        url: &str,
        target: &Path,
        expected: &str,
        size: u64,
        mut progress: F,
        cancelled: &C,
    ) -> Result<(), ApiError> {
        let partial = Partial::new(target);
        let digest = if size >= LARGE && self.length(url) == Some(size) {
            self.ranged(url, &partial.path, size, FILE_PARTS, &mut progress, cancelled)?;
            file_digest(&partial.path, Algo::Sha256).ok_or_else(io)?
        } else {
            let response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
            receive(response, &partial.path, progress, cancelled)?
        };
        if !expected.is_empty() && !digest.eq_ignore_ascii_case(expected) {
            return Err(ApiError::new(crate::secret!("CHECKSUM"), 0));
        }
        partial.commit(target).map_err(|_| io())
    }

    pub fn archive<F: FnMut(u64, u64), C: Fn() -> bool + Sync>(
        &self,
        url: &str,
        target: &Path,
        mut progress: F,
        cancelled: &C,
    ) -> Result<(), ApiError> {
        if let Some(total) = self.length(url).filter(|&total| total > 0) {
            return self.ranged(url, target, total, ARCHIVE_PARTS, |done| progress(done, total), cancelled);
        }
        let response = self
            .agent
            .get(url)
            .config()
            .timeout_global(Some(ARCHIVE_TIMEOUT))
            .build()
            .call()
            .map_err(|_| ApiError::offline())?;
        let total = response.body().content_length().unwrap_or(0);
        receive(response, target, |done| progress(done, total), cancelled).map(|_| ())
    }

    fn length(&self, url: &str) -> Option<u64> {
        let response = self.agent.get(url).header(RANGE, crate::secret!("bytes=0-0")).call().ok()?;
        if response.status().as_u16() != 206 {
            return None;
        }
        let range = response.headers().get(CONTENT_RANGE)?.to_str().ok()?;
        range.rsplit('/').next()?.trim().parse().ok()
    }

    fn ranged<F: FnMut(u64), C: Fn() -> bool + Sync>(
        &self,
        url: &str,
        target: &Path,
        total: u64,
        parts: usize,
        mut progress: F,
        cancelled: &C,
    ) -> Result<(), ApiError> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| io())?;
        }
        File::create(target).and_then(|file| file.set_len(total)).map_err(|_| io())?;
        let segments: Vec<(u64, u64)> = (0..total.div_ceil(SEGMENT))
            .map(|index| (index * SEGMENT, ((index + 1) * SEGMENT).min(total)))
            .collect();
        let mut flight: HashMap<usize, u64> = HashMap::new();
        let mut settled = 0u64;
        Pool::new(parts).run(
            &segments,
            || OpenOptions::new().write(true).open(target).map_err(|_| io()),
            |file, &(start, end), report| self.segment(url, file, start, end, report, cancelled),
            |signal| {
                match signal {
                    Signal::Progress(index, bytes) => {
                        flight.insert(index, bytes);
                    }
                    Signal::Finished(index) => {
                        flight.remove(&index);
                        settled += segments[index].1 - segments[index].0;
                    }
                }
                progress(settled + flight.values().sum::<u64>());
            },
        )
    }

    fn segment<C: Fn() -> bool>(
        &self,
        url: &str,
        file: &mut File,
        start: u64,
        end: u64,
        report: &dyn Fn(u64),
        cancelled: &C,
    ) -> Result<(), ApiError> {
        let mut attempt = 1;
        loop {
            match self.piece(url, file, start, end, report, cancelled) {
                Err(error) if attempt < SEGMENT_ATTEMPTS && transient(&error) => {
                    report(0);
                    pause(RETRY_DELAY * attempt, cancelled)?;
                    attempt += 1;
                }
                result => return result,
            }
        }
    }

    fn piece<C: Fn() -> bool>(
        &self,
        url: &str,
        file: &mut File,
        start: u64,
        end: u64,
        report: &dyn Fn(u64),
        cancelled: &C,
    ) -> Result<(), ApiError> {
        let response = self
            .agent
            .get(url)
            .header(RANGE, format!("{}{start}-{}", crate::secret!("bytes="), end - 1))
            .config()
            .timeout_global(Some(SEGMENT_TIMEOUT))
            .build()
            .call()
            .map_err(|_| ApiError::offline())?;
        let status = response.status().as_u16();
        if status >= 400 {
            return Err(ApiError::new(crate::secret!("NOT_FOUND"), status));
        }
        if status != 206 {
            return Err(ApiError::offline());
        }
        file.seek(SeekFrom::Start(start)).map_err(|_| io())?;
        let mut writer = BufWriter::with_capacity(BUFFER, &mut *file);
        let mut reader = response.into_body().into_reader();
        let mut buffer = vec![0u8; BUFFER];
        let length = end - start;
        let mut done = 0u64;
        while done < length {
            if cancelled() {
                return Err(ApiError::new(crate::secret!("CANCELLED"), 0));
            }
            let read = reader.read(&mut buffer).map_err(|_| ApiError::offline())?;
            if read == 0 {
                break;
            }
            let take = (read as u64).min(length - done) as usize;
            writer.write_all(&buffer[..take]).map_err(|_| io())?;
            done += take as u64;
            report(done);
        }
        writer.flush().map_err(|_| io())?;
        if done != length {
            return Err(ApiError::offline());
        }
        Ok(())
    }
}

pub fn transient(error: &ApiError) -> bool {
    error.is(crate::secret!("OFFLINE")) || error.status == 429 || error.status >= 500
}

pub fn pause<C: Fn() -> bool>(duration: Duration, cancelled: &C) -> Result<(), ApiError> {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        if cancelled() {
            return Err(ApiError::new(crate::secret!("CANCELLED"), 0));
        }
        thread::sleep(POLL.min(deadline.saturating_duration_since(Instant::now())));
    }
    Ok(())
}

fn receive<F: FnMut(u64), C: Fn() -> bool>(
    response: Response<Body>,
    target: &Path,
    mut progress: F,
    cancelled: &C,
) -> Result<String, ApiError> {
    if response.status().as_u16() >= 400 {
        return Err(ApiError::new(crate::secret!("NOT_FOUND"), response.status().as_u16()));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|_| io())?;
    }
    let file = File::create(target).map_err(|_| io())?;
    let mut writer = BufWriter::with_capacity(BUFFER, file);
    let mut reader = response.into_body().into_reader();
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER];
    let mut total = 0u64;
    loop {
        if cancelled() {
            return Err(ApiError::new(crate::secret!("CANCELLED"), 0));
        }
        let read = reader.read(&mut buffer).map_err(|_| ApiError::offline())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read]).map_err(|_| io())?;
        total += read as u64;
        progress(total);
    }
    writer.flush().map_err(|_| io())?;
    Ok(hex(&hasher.finalize()))
}

fn io() -> ApiError {
    ApiError::new(crate::secret!("IO"), 0)
}

pub struct Partial {
    pub path: PathBuf,
    committed: bool,
}

impl Partial {
    pub fn new(target: &Path) -> Self {
        let root = Paths::temp();
        let digest = hex(&Sha256::digest(target.as_os_str().as_encoded_bytes()));
        let dir = root.join(&digest[..2]);
        if !dir.is_dir() {
            let fresh = !root.is_dir();
            if fs::create_dir_all(&dir).is_ok() && fresh {
                hide(&root);
            }
        }
        let name = format!("{}{}", &digest[..24], crate::secret!(".part"));
        Self { path: dir.join(name), committed: false }
    }

    pub fn commit(mut self, target: &Path) -> std::io::Result<()> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&self.path, target)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for Partial {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub struct Scratch;

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(Paths::temp());
    }
}

fn hide(dir: &Path) {
    let path: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
    }
}
