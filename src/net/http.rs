use std::io::Read;
use std::time::Duration;

use ureq::Agent;
use ureq::http::StatusCode;

use crate::net::channel::Transport;
use crate::net::error::ApiError;

pub struct HttpTransport {
    agent: Agent,
    base: String,
}

impl HttpTransport {
    pub fn new(base: impl Into<String>) -> Self {
        let agent = Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .http_status_as_error(false)
            .user_agent("alekdlc-launcher")
            .build()
            .into();
        Self {
            agent,
            base: base.into(),
        }
    }
}

impl Transport for HttpTransport {
    fn post(&self, path: &str, channel: Option<&str>, body: &[u8]) -> Result<(u16, Vec<u8>), ApiError> {
        let url = format!("{}{}", self.base, path);
        let mut request = self.agent.post(&url).header("Content-Type", "application/json");
        if let Some(id) = channel {
            request = request.header("X-Channel", id);
        }
        let response = match request.send(body) {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(code)) => {
                return Ok((code, Vec::new()));
            }
            Err(_) => return Err(ApiError::offline()),
        };
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        response
            .into_body()
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|_| ApiError::offline())?;
        Ok((status, bytes))
    }
}

pub fn fetch(agent: &Agent, url: &str) -> Result<(u16, Box<dyn Read + Send + Sync>), ApiError> {
    match agent.get(url).call() {
        Ok(response) => {
            let status = response.status().as_u16();
            Ok((status, Box::new(response.into_body().into_reader())))
        }
        Err(ureq::Error::StatusCode(code)) => Ok((code, Box::new(std::io::empty()))),
        Err(_) => Err(ApiError::offline()),
    }
}

pub fn status_is_stale(status: u16) -> bool {
    status == StatusCode::FORBIDDEN.as_u16() || status == StatusCode::GONE.as_u16()
}
