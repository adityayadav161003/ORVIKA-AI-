use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::utils::error::{AppError, AppResult};

/// An AES-256-GCM cipher bound to this machine.
///
/// The 256-bit key is derived from `hostname + OS username` via SHA-256.
/// This means API keys encrypted here are tied to this machine — they cannot
/// be decrypted if the SQLite database is copied elsewhere. That is the
/// intended behaviour for v1: per-machine, no user master password.
pub struct Aes256GcmCipher {
    cipher: Aes256Gcm,
}

/// Serialisable blob stored in SQLite (hex-encoded fields).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedBlob {
    /// AES-256-GCM ciphertext (hex).
    pub ciphertext: String,
    /// 96-bit nonce (hex, 24 hex chars).
    pub nonce: String,
}

impl Aes256GcmCipher {
    /// Derive a machine-bound encryption key.
    ///
    /// Combines hostname and OS username with a fixed application salt,
    /// then hashes with SHA-256 to produce the 256-bit AES key.
    /// Retrieve the master key from the Credential Manager, falling back to a derived machine key.
    pub fn from_machine_key() -> Self {
        // Attempt to get master key from Windows Credential Manager
        let key_bytes = match super::credential::get_or_create_master_key() {
            Ok(key) => {
                if key.len() == 32 {
                    let mut key_arr = [0u8; 32];
                    key_arr.copy_from_slice(&key);
                    key_arr
                } else {
                    tracing::warn!("Retrieved key has invalid length, falling back to derived key");
                    Self::derive_machine_key()
                }
            }
            Err(e) => {
                tracing::warn!("Failed to retrieve key from Credential Manager: {}. Falling back to derived key.", e);
                Self::derive_machine_key()
            }
        };

        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        Self {
            cipher: Aes256Gcm::new(key),
        }
    }

    /// Helper to derive the fallback machine key
    fn derive_machine_key() -> [u8; 32] {
        let hostname = hostname();
        let username = username();
        let salt = b"orvika_ai_v1_key_salt";

        let mut hasher = Sha256::new();
        hasher.update(hostname.as_bytes());
        hasher.update(b":");
        hasher.update(username.as_bytes());
        hasher.update(b":");
        hasher.update(salt);
        hasher.finalize().into()
    }

    /// Encrypt `plaintext` using AES-256-GCM with a fresh random nonce.
    pub fn encrypt(&self, plaintext: &[u8]) -> AppResult<EncryptedBlob> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| AppError::Encryption(e.to_string()))?;

        Ok(EncryptedBlob {
            ciphertext: hex_encode(&ciphertext),
            nonce: hex_encode(&nonce_bytes),
        })
    }

    /// Decrypt an `EncryptedBlob` and return the original plaintext bytes.
    ///
    /// Returns an error if the blob was tampered with (GCM tag mismatch).
    pub fn decrypt(&self, blob: &EncryptedBlob) -> AppResult<Vec<u8>> {
        let ciphertext =
            hex_decode(&blob.ciphertext).map_err(|e| AppError::Encryption(e.to_string()))?;
        let nonce_bytes =
            hex_decode(&blob.nonce).map_err(|e| AppError::Encryption(e.to_string()))?;

        if nonce_bytes.len() != 12 {
            return Err(AppError::Encryption("Invalid nonce length".into()));
        }

        let nonce = Nonce::from_slice(&nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| {
                AppError::Encryption("Decryption failed — ciphertext may be tampered".into())
            })
    }

    /// Convenience: encrypt a UTF-8 string and return a blob.
    pub fn encrypt_str(&self, plaintext: &str) -> AppResult<EncryptedBlob> {
        self.encrypt(plaintext.as_bytes())
    }

    /// Convenience: decrypt a blob and return a UTF-8 string.
    pub fn decrypt_str(&self, blob: &EncryptedBlob) -> AppResult<String> {
        let bytes = self.decrypt(blob)?;
        String::from_utf8(bytes).map_err(|e| AppError::Encryption(e.to_string()))
    }
}

// ─── Platform helpers ─────────────────────────────────────────────────────────

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown_host".to_string())
}

fn username() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown_user".to_string())
}

// ─── Hex helpers ──────────────────────────────────────────────────────────────

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Odd-length hex string".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> Aes256GcmCipher {
        Aes256GcmCipher::from_machine_key()
    }

    #[test]
    fn round_trip_bytes() {
        let c = cipher();
        let plaintext = b"hello, ORVIKA AI";
        let blob = c.encrypt(plaintext).expect("encrypt");
        let recovered = c.decrypt(&blob).expect("decrypt");
        assert_eq!(recovered, plaintext);
    }

    #[test]
    fn round_trip_string() {
        let c = cipher();
        let secret = "sk-openai-supersecret-api-key";
        let blob = c.encrypt_str(secret).expect("encrypt_str");
        let recovered = c.decrypt_str(&blob).expect("decrypt_str");
        assert_eq!(recovered, secret);
    }

    #[test]
    fn unique_nonces() {
        let c = cipher();
        let blob1 = c.encrypt(b"same").expect("e1");
        let blob2 = c.encrypt(b"same").expect("e2");
        // Different nonces → different ciphertexts even for identical plaintext
        assert_ne!(blob1.nonce, blob2.nonce);
        assert_ne!(blob1.ciphertext, blob2.ciphertext);
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let c = cipher();
        let mut blob = c.encrypt(b"secret").expect("encrypt");
        // Flip the first byte of the ciphertext
        let mut bytes = hex_decode(&blob.ciphertext).unwrap();
        bytes[0] ^= 0xff;
        blob.ciphertext = hex_encode(&bytes);
        assert!(c.decrypt(&blob).is_err());
    }
}
