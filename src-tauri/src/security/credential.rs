#[cfg(target_os = "windows")]
use wincredentials::{read_credential, write_credential, credential::Credential};

use crate::utils::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(target_os = "windows")]
fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Invalid hex length".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| e.to_string())
        })
        .collect()
}

/// Retrieve the master key from the Windows Credential Manager or create a new one.
#[cfg(target_os = "windows")]
pub fn get_or_create_master_key() -> AppResult<Vec<u8>> {
    let name = "OrvikaAI/MasterKey";
    if let Ok(cred) = read_credential(name) {
        let key_bytes = decode_hex(&cred.secret)
            .map_err(|e| AppError::Encryption(format!("Failed to decode master key hex: {:?}", e)))?;
        Ok(key_bytes)
    } else {
        // Generate a new 32-byte key
        use rand::RngCore;
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);

        let secret_hex = encode_hex(&key);
        let cred = Credential {
            username: "AppMasterKey".to_string(),
            secret: secret_hex,
        };

        write_credential(name, cred)
            .map_err(|e| AppError::Encryption(format!("Failed to write master key to Windows Credential Manager: {:?}", e)))?;
        Ok(key)
    }
}

/// Fallback for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn get_or_create_master_key() -> AppResult<Vec<u8>> {
    tracing::warn!("Windows Credential Manager is not supported on this platform.");
    Err(AppError::Encryption("Windows Credential Manager not supported".into()))
}
