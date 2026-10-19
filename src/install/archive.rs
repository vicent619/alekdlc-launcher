use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::install::download::Partial;
use crate::install::hash::hex;
use crate::install::pool::{Pool, Signal};
use crate::net::error::ApiError;

const BUFFER: usize = 131072;

pub struct Entry {
    pub name: String,
    pub target: PathBuf,
    pub sha: String,
}

pub struct Archive<'a> {
    path: &'a Path,
}

impl<'a> Archive<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }

    pub fn extract<C, E>(&self, entries: &[Entry], pool: &Pool, cancelled: &C, mut observe: E) -> Result<(), ApiError>
    where
        C: Fn() -> bool + Sync,
        E: FnMut(usize),
    {
        let positions = self.positions()?;
        let located: Vec<(usize, usize)> = entries
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| positions.get(&entry.name).map(|&position| (slot, position)))
            .collect();
        pool.run(
            &located,
            || self.open(),
            |zip, &(slot, position), _| {
                if cancelled() {
                    return Err(ApiError::new(crate::secret!("CANCELLED"), 0));
                }
                unpack(zip, position, &entries[slot])
            },
            |signal| {
                if let Signal::Finished(index) = signal {
                    observe(located[index].0);
                }
            },
        )
    }

    fn positions(&self) -> Result<HashMap<String, usize>, ApiError> {
        let zip = self.open()?;
        let mut positions = HashMap::with_capacity(zip.len());
        for position in 0..zip.len() {
            if let Some(Ok(name)) = zip.name_for_index(position) {
                positions.insert(name.replace('\\', "/"), position);
            }
        }
        Ok(positions)
    }

    fn open(&self) -> Result<ZipArchive<File>, ApiError> {
        let file = File::open(self.path).map_err(|_| io())?;
        ZipArchive::new(file).map_err(|_| corrupt())
    }
}

fn unpack(zip: &mut ZipArchive<File>, position: usize, entry: &Entry) -> Result<(), ApiError> {
    let mut source = zip.by_index(position).map_err(|_| corrupt())?;
    if let Some(parent) = entry.target.parent() {
        fs::create_dir_all(parent).map_err(|_| io())?;
    }
    let partial = Partial::new(&entry.target);
    let file = File::create(&partial.path).map_err(|_| io())?;
    let mut writer = BufWriter::with_capacity(BUFFER, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; BUFFER];
    loop {
        let read = source.read(&mut buffer).map_err(|_| corrupt())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        writer.write_all(&buffer[..read]).map_err(|_| io())?;
    }
    writer.flush().map_err(|_| io())?;
    drop(writer);
    if !entry.sha.is_empty() && !hex(&hasher.finalize()).eq_ignore_ascii_case(&entry.sha) {
        return Err(corrupt());
    }
    partial.commit(&entry.target).map_err(|_| io())
}

fn corrupt() -> ApiError {
    ApiError::new(crate::secret!("CHECKSUM"), 0)
}

fn io() -> ApiError {
    ApiError::new(crate::secret!("IO"), 0)
}
