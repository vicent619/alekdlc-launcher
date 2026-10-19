use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha1::Sha1;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
pub enum Algo {
    Sha1,
    Sha256,
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn file_digest(path: &Path, algo: Algo) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; 65536];
    match algo {
        Algo::Sha1 => {
            let mut hasher = Sha1::new();
            loop {
                let read = reader.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Some(hex(&hasher.finalize()))
        }
        Algo::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let read = reader.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            Some(hex(&hasher.finalize()))
        }
    }
}

pub fn matches(path: &Path, algo: Algo, expected: &str) -> bool {
    if expected.is_empty() {
        return path.exists();
    }
    file_digest(path, algo)
        .map(|digest| digest.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}
