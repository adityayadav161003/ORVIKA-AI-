use rusqlite::{params, Connection};
use serde::Serialize;

use crate::security::encryption::{Aes256GcmCipher, EncryptedBlob};
use crate::utils::error::{AppError, AppResult};

/// Metadata about a stored API key (no key material exposed).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    pub provider: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

/// Store an API key encrypted with AES-256-GCM.
///
/// The blob (ciphertext + nonce) is serialised to JSON and stored in the
/// `encrypted_key` column. `iv` stores the nonce separately for clarity.
pub fn store_key(
    conn: &Connection,
    cipher: &Aes256GcmCipher,
    provider: &str,
    plaintext_key: &str,
) -> AppResult<()> {
    let blob = cipher.encrypt_str(plaintext_key)?;
    let blob_json =
        serde_json::to_string(&blob).map_err(|e| AppError::Encryption(e.to_string()))?;

    conn.execute(
        "INSERT INTO api_keys (provider, encrypted_key, iv)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(provider) DO UPDATE SET
           encrypted_key = excluded.encrypted_key,
           iv = excluded.iv",
        params![provider, blob_json, blob.nonce],
    )?;
    Ok(())
}

/// Retrieve and decrypt an API key. Returns `None` if no key is stored.
pub fn get_key(
    conn: &Connection,
    cipher: &Aes256GcmCipher,
    provider: &str,
) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT encrypted_key FROM api_keys WHERE provider = ?1")?;
    let mut rows = stmt.query(params![provider])?;

    if let Some(row) = rows.next()? {
        let blob_json: String = row.get(0)?;
        let blob: EncryptedBlob = serde_json::from_str(&blob_json)
            .map_err(|e| AppError::Encryption(format!("Corrupt key blob for {provider}: {e}")))?;
        let plaintext = cipher.decrypt_str(&blob)?;

        // Update last_used_at
        let _ = conn.execute(
            "UPDATE api_keys SET last_used_at = datetime('now') WHERE provider = ?1",
            params![provider],
        );

        return Ok(Some(plaintext));
    }

    Ok(None)
}

/// Delete a stored API key.
pub fn delete_key(conn: &Connection, provider: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM api_keys WHERE provider = ?1",
        params![provider],
    )?;
    Ok(())
}

/// List providers that have a key stored (no key material).
pub fn list_providers(conn: &Connection) -> AppResult<Vec<ApiKeyInfo>> {
    let mut stmt =
        conn.prepare("SELECT provider, created_at, last_used_at FROM api_keys ORDER BY provider")?;
    let rows = stmt.query_map([], |row| {
        Ok(ApiKeyInfo {
            provider: row.get(0)?,
            created_at: row.get(1)?,
            last_used_at: row.get(2)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE api_keys (
                provider       TEXT PRIMARY KEY,
                encrypted_key  TEXT NOT NULL,
                iv             TEXT NOT NULL,
                created_at     TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at   TEXT
            );",
        )
        .unwrap();
        conn
    }

    fn cipher() -> Aes256GcmCipher {
        Aes256GcmCipher::from_machine_key()
    }

    #[test]
    fn store_and_retrieve() {
        let conn = in_memory();
        let c = cipher();
        store_key(&conn, &c, "openai", "sk-test-key-abc123").unwrap();
        let retrieved = get_key(&conn, &c, "openai").unwrap();
        assert_eq!(retrieved, Some("sk-test-key-abc123".to_string()));
    }

    #[test]
    fn missing_provider_returns_none() {
        let conn = in_memory();
        let c = cipher();
        let key = get_key(&conn, &c, "anthropic").unwrap();
        assert!(key.is_none());
    }

    #[test]
    fn delete_removes_key() {
        let conn = in_memory();
        let c = cipher();
        store_key(&conn, &c, "gemini", "my-gemini-key").unwrap();
        delete_key(&conn, "gemini").unwrap();
        assert!(get_key(&conn, &c, "gemini").unwrap().is_none());
    }

    #[test]
    fn upsert_replaces_key() {
        let conn = in_memory();
        let c = cipher();
        store_key(&conn, &c, "openai", "old-key").unwrap();
        store_key(&conn, &c, "openai", "new-key").unwrap();
        let retrieved = get_key(&conn, &c, "openai").unwrap();
        assert_eq!(retrieved, Some("new-key".to_string()));
    }
}
