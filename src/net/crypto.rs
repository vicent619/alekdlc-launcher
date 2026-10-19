use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

pub struct Ephemeral {
    secret: StaticSecret,
}

impl Ephemeral {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).expect("entropy");
        Self {
            secret: StaticSecret::from(bytes),
        }
    }

    pub fn public(&self) -> [u8; 32] {
        PublicKey::from(&self.secret).to_bytes()
    }

    pub fn shared(&self, peer: [u8; 32]) -> [u8; 32] {
        self.secret.diffie_hellman(&PublicKey::from(peer)).to_bytes()
    }
}

pub fn random(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    getrandom::fill(&mut bytes).expect("entropy");
    bytes
}

pub fn derive_key(shared: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hkdf = Hkdf::<Sha256>::new(Some(salt), shared);
    let mut key = [0u8; 32];
    hkdf.expand(info, &mut key).expect("hkdf");
    key
}

pub struct Cipher {
    cipher: Aes256Gcm,
}

impl Cipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new((&key).into()),
        }
    }

    pub fn seal(&self, iv: &[u8], plaintext: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
        self.cipher
            .encrypt(Nonce::from_slice(iv), Payload { msg: plaintext, aad })
            .ok()
    }

    pub fn open(&self, iv: &[u8], ciphertext: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
        self.cipher
            .decrypt(Nonce::from_slice(iv), Payload { msg: ciphertext, aad })
            .ok()
    }
}
