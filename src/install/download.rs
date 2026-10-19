use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::install::hash::{matches, Algo};
use crate::net::error::ApiError;

pub struct Http {
    agent: Agent,
}

impl Http {
    pub fn new() -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .user_agent("alekdlc-launcher")
            .http_status_as_error(false)
            .build()
            .into();
        Self { agent }
    }

    pub fn json<T: DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        let mut response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
        if response.status().as_u16() >= 400 {
            return Err(ApiError::new("NOT_FOUND", response.status().as_u16()));
        }
        response
            .body_mut()
            .read_json::<T>()
            .map_err(|_| ApiError::new("INTERNAL", 0))
    }

    pub fn text(&self, url: &str) -> Result<String, ApiError> {
        let mut response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
        if response.status().as_u16() >= 400 {
            return Err(ApiError::new("NOT_FOUND", response.status().as_u16()));
        }
        response
            .body_mut()
            .read_to_string()
            .map_err(|_| ApiError::offline())
    }

    pub fn download<F: FnMut(u64)>(
        &self,
        url: &str,
        target: &Path,
        mut progress: F,
    ) -> Result<u64, ApiError> {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| ApiError::new("IO", 0))?;
        }
        let response = self.agent.get(url).call().map_err(|_| ApiError::offline())?;
        if response.status().as_u16() >= 400 {
            return Err(ApiError::new("NOT_FOUND", response.status().as_u16()));
        }
        let temp = target.with_extension("part");
        let mut file = File::create(&temp).map_err(|_| ApiError::new("IO", 0))?;
        let mut reader = response.into_body().into_reader();
        let mut buffer = vec![0u8; 131072];
        let mut total = 0u64;
        loop {
            let read = reader.read(&mut buffer).map_err(|_| ApiError::offline())?;
            if read == 0 {
                break;
            }
            file.write_all(&buffer[..read]).map_err(|_| ApiError::new("IO", 0))?;
            total += read as u64;
            progress(total);
        }
        file.flush().ok();
        drop(file);
        fs::rename(&temp, target).map_err(|_| ApiError::new("IO", 0))?;
        Ok(total)
    }

    pub fn ensure<F: FnMut(u64)>(
        &self,
        url: &str,
        target: &Path,
        algo: Algo,
        expected: &str,
        progress: F,
    ) -> Result<bool, ApiError> {
        if matches(target, algo, expected) {
            return Ok(false);
        }
        self.download(url, target, progress)?;
        if !matches(target, algo, expected) {
            let _ = fs::remove_file(target);
            return Err(ApiError::new("CHECKSUM", 0));
        }
        Ok(true)
    }
}
