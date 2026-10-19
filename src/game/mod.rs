pub mod command;
pub mod launch;
pub mod window;

use sha1::{Digest, Sha1};

pub fn offline_uuid(username: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("OfflinePlayer:{username}").as_bytes());
    let mut bytes = hasher.finalize()[..16].to_vec();
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}
