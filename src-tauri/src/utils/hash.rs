use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::utils::error::{AppError, AppResult};

pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    hex_encode(&digest)
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hex_encode(&hasher.finalize()))
}

pub fn verify_file_checksum(path: &Path, expected_hex: &str) -> AppResult<()> {
    if expected_hex.trim().is_empty() {
        return Ok(());
    }

    let actual = sha256_file(path)?;
    let expected = expected_hex.trim().to_ascii_lowercase();

    if actual != expected {
        return Err(AppError::Other(format!(
            "Checksum mismatch: expected {expected}, got {actual}"
        )));
    }

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_file_matches() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "hello").unwrap();
        let hash = sha256_file(file.path()).unwrap();
        assert_eq!(hash, sha256_hex(b"hello"));
    }
}
