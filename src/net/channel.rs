use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use serde_json::Value;

use crate::net::crypto::{Cipher, Ephemeral, derive_key, random};
use crate::net::error::ApiError;

const PROTOCOL: u32 = 1;

#[derive(Deserialize)]
struct HandshakeReply {
    #[serde(rename = "channelId")]
    channel_id: String,
    #[serde(rename = "serverKey")]
    server_key: String,
    #[serde(rename = "serverNonce")]
    server_nonce: String,
    signature: String,
}

#[derive(Deserialize)]
struct Envelope {
    seq: Option<u64>,
    ts: Option<i64>,
    iv: Option<String>,
    payload: Option<String>,
    code: Option<String>,
}

struct Session {
    id: String,
    seq: u64,
    c2s: Cipher,
    s2c: Cipher,
}

pub trait Transport: Send + Sync {
    fn post(&self, path: &str, channel: Option<&str>, body: &[u8]) -> Result<(u16, Vec<u8>), ApiError>;
}

pub struct SecureChannel {
    transport: Box<dyn Transport>,
    verifying: VerifyingKey,
    session: Mutex<Option<Session>>,
}

impl SecureChannel {
    pub fn new(transport: Box<dyn Transport>, server_key: &str) -> Self {
        let bytes: [u8; 32] = STANDARD.decode(server_key).expect("key").try_into().expect("key length");
        Self {
            transport,
            verifying: VerifyingKey::from_bytes(&bytes).expect("verifying key"),
            session: Mutex::new(None),
        }
    }

    pub fn call(&self, path: &str, body: &Value) -> Result<Value, ApiError> {
        let bytes = self.call_raw(path, body)?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes).map_err(|_| ApiError::new("INTERNAL", 0))
    }

    pub fn call_raw(&self, path: &str, body: &Value) -> Result<Vec<u8>, ApiError> {
        let payload = serde_json::to_vec(body).map_err(|_| ApiError::new("INTERNAL", 0))?;
        self.send(path, &payload, true)
    }

    fn send(&self, path: &str, payload: &[u8], retry: bool) -> Result<Vec<u8>, ApiError> {
        self.ensure_session()?;
        let (id, seq, request) = {
            let mut guard = self.session.lock().unwrap();
            let session = guard.as_mut().ok_or_else(|| ApiError::new("CHANNEL_INVALID", 0))?;
            session.seq += 1;
            let seq = session.seq;
            let ts = now_millis();
            let iv = random(12);
            let aad = format!("{}|{}|{}|{}|POST|{}", crate::secret!("c2s"), session.id, seq, ts, path);
            let ciphertext = session
                .c2s
                .seal(&iv, payload, aad.as_bytes())
                .ok_or_else(|| ApiError::new("CHANNEL_INVALID", 0))?;
            let request = serde_json::json!({
                "seq": seq,
                "ts": ts,
                "iv": STANDARD.encode(&iv),
                "payload": STANDARD.encode(&ciphertext)
            });
            (session.id.clone(), seq, serde_json::to_vec(&request).unwrap())
        };
        let (status, raw) = self.transport.post(path, Some(&id), &request)?;
        let envelope: Envelope = serde_json::from_slice(&raw).unwrap_or(Envelope {
            seq: None,
            ts: None,
            iv: None,
            payload: None,
            code: None,
        });
        let Some(env_iv) = envelope.iv else {
            let code = envelope.code.unwrap_or_else(|| "CHANNEL_INVALID".into());
            if retry && (code == "CHANNEL_INVALID" || code == "CHANNEL_REPLAY") {
                *self.session.lock().unwrap() = None;
                return self.send(path, payload, false);
            }
            return Err(ApiError::new(code, status));
        };
        if envelope.seq != Some(seq) {
            return Err(ApiError::new("CHANNEL_INVALID", status));
        }
        let plain = {
            let guard = self.session.lock().unwrap();
            let session = guard.as_ref().ok_or_else(|| ApiError::new("CHANNEL_INVALID", 0))?;
            let aad = format!("{}|{}|{}|{}|{}", crate::secret!("s2c"), session.id, seq, envelope.ts.unwrap_or(0), status);
            let iv = STANDARD.decode(&env_iv).map_err(|_| ApiError::new("CHANNEL_INVALID", status))?;
            let data = STANDARD
                .decode(envelope.payload.unwrap_or_default())
                .map_err(|_| ApiError::new("CHANNEL_INVALID", status))?;
            session
                .s2c
                .open(&iv, &data, aad.as_bytes())
                .ok_or_else(|| ApiError::new("CHANNEL_INVALID", status))?
        };
        if status >= 400 {
            let error: Value = serde_json::from_slice(&plain).unwrap_or(Value::Null);
            let code = error.get("code").and_then(Value::as_str).unwrap_or("INTERNAL");
            return Err(ApiError::new(code, status));
        }
        Ok(plain)
    }

    fn ensure_session(&self) -> Result<(), ApiError> {
        if self.session.lock().unwrap().is_some() {
            return Ok(());
        }
        let session = self.handshake()?;
        *self.session.lock().unwrap() = Some(session);
        Ok(())
    }

    fn handshake(&self) -> Result<Session, ApiError> {
        let ephemeral = Ephemeral::generate();
        let client_key = ephemeral.public();
        let client_nonce = random(32);
        let request = serde_json::json!({
            "version": PROTOCOL,
            "clientKey": STANDARD.encode(client_key),
            "clientNonce": STANDARD.encode(&client_nonce)
        });
        let (status, raw) = self
            .transport
            .post(crate::secret!("/api/v1/handshake"), None, &serde_json::to_vec(&request).unwrap())?;
        if status >= 400 {
            let value: Value = serde_json::from_slice(&raw).unwrap_or(Value::Null);
            let code = value.get("code").and_then(Value::as_str).unwrap_or("CHANNEL_INVALID");
            return Err(ApiError::new(code, status));
        }
        let reply: HandshakeReply = serde_json::from_slice(&raw).map_err(|_| ApiError::new("CHANNEL_INVALID", status))?;
        let server_key: [u8; 32] = STANDARD
            .decode(&reply.server_key)
            .ok()
            .and_then(|b| b.try_into().ok())
            .ok_or_else(|| ApiError::new("CHANNEL_INVALID", status))?;
        let server_nonce = STANDARD.decode(&reply.server_nonce).map_err(|_| ApiError::new("CHANNEL_INVALID", status))?;
        let signature = STANDARD.decode(&reply.signature).map_err(|_| ApiError::new("CHANNEL_INVALID", status))?;
        let mut transcript = Vec::new();
        transcript.extend_from_slice(crate::secret!("alekdlc-handshake-v1").as_bytes());
        transcript.extend_from_slice(&client_key);
        transcript.extend_from_slice(&server_key);
        transcript.extend_from_slice(&client_nonce);
        transcript.extend_from_slice(&server_nonce);
        transcript.extend_from_slice(reply.channel_id.as_bytes());
        let signature = Signature::from_slice(&signature).map_err(|_| ApiError::new("CHANNEL_UNTRUSTED", 0))?;
        self.verifying
            .verify_strict(&transcript, &signature)
            .map_err(|_| ApiError::new("CHANNEL_UNTRUSTED", 0))?;
        let shared = ephemeral.shared(server_key);
        let mut salt = client_nonce.clone();
        salt.extend_from_slice(&server_nonce);
        Ok(Session {
            id: reply.channel_id,
            seq: 0,
            c2s: Cipher::new(derive_key(&shared, &salt, crate::secret!("alekdlc/v1/c2s").as_bytes())),
            s2c: Cipher::new(derive_key(&shared, &salt, crate::secret!("alekdlc/v1/s2c").as_bytes())),
        })
    }
}

fn now_millis() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}
