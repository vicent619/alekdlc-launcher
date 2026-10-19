use std::process::Command;

use sha2::{Digest, Sha256};

pub fn compute() -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"alekdlc-hwid-v1");
    for key in ["PROCESSOR_IDENTIFIER", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE"] {
        if let Some(value) = std::env::var_os(key) {
            hasher.update([0]);
            hasher.update(value.to_string_lossy().as_bytes());
        }
    }
    if let Some(guid) = machine_guid() {
        hasher.update([0]);
        hasher.update(guid.as_bytes());
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn machine_guid() -> Option<String> {
    let output = Command::new("reg")
        .args([
            "query",
            "HKLM\\SOFTWARE\\Microsoft\\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find(|line| line.contains("MachineGuid"))
        .and_then(|line| line.split_whitespace().last())
        .map(|value| value.to_owned())
}
