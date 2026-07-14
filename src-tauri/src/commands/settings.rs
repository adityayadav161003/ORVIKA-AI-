use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::db::api_key_repo;
use crate::db::settings_repo;
use crate::db::Database;
use crate::security::Aes256GcmCipher;

// ─── Generic settings ─────────────────────────────────────────────────────────

/// Get a single setting value by key.
#[tauri::command]
pub fn get_setting(
    database: State<'_, Arc<Database>>,
    key: String,
) -> Result<Option<String>, String> {
    database
        .with_connection(|conn| settings_repo::get(conn, &key))
        .map_err(|e| e.to_string())
}

/// Set a single setting value.
#[tauri::command]
pub fn set_setting(
    database: State<'_, Arc<Database>>,
    key: String,
    value: String,
) -> Result<(), String> {
    database
        .with_connection(|conn| settings_repo::set(conn, &key, &value))
        .map_err(|e| e.to_string())
}

/// Get all non-encrypted settings as a flat map.
#[tauri::command]
pub fn get_all_settings(
    database: State<'_, Arc<Database>>,
) -> Result<HashMap<String, String>, String> {
    database
        .with_connection(settings_repo::get_all)
        .map_err(|e| e.to_string())
}

// ─── Encrypted API keys ───────────────────────────────────────────────────────

/// Store an API key encrypted with AES-256-GCM. Replaces any existing key.
#[tauri::command]
pub fn store_api_key(
    database: State<'_, Arc<Database>>,
    cipher: State<'_, Arc<Aes256GcmCipher>>,
    provider: String,
    plaintext_key: String,
) -> Result<(), String> {
    database
        .with_connection(|conn| api_key_repo::store_key(conn, &cipher, &provider, &plaintext_key))
        .map_err(|e| e.to_string())
}

/// Retrieve and decrypt an API key. Returns `null` if not found.
#[tauri::command]
pub fn get_api_key(
    database: State<'_, Arc<Database>>,
    cipher: State<'_, Arc<Aes256GcmCipher>>,
    provider: String,
) -> Result<Option<String>, String> {
    database
        .with_connection(|conn| api_key_repo::get_key(conn, &cipher, &provider))
        .map_err(|e| e.to_string())
}

/// Delete a stored API key.
#[tauri::command]
pub fn delete_api_key(database: State<'_, Arc<Database>>, provider: String) -> Result<(), String> {
    database
        .with_connection(|conn| api_key_repo::delete_key(conn, &provider))
        .map_err(|e| e.to_string())
}

/// List providers with keys stored (no key material exposed).
#[tauri::command]
pub fn list_api_key_providers(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<api_key_repo::ApiKeyInfo>, String> {
    database
        .with_connection(api_key_repo::list_providers)
        .map_err(|e| e.to_string())
}

/// Check if a plaintext API key has the expected format for a provider.
///
/// Performs format validation only — does NOT make a network call in v1.
/// Returns `true` if the key looks valid, `false` otherwise.
#[tauri::command]
pub fn validate_api_key_format(provider: String, key: String) -> Result<bool, String> {
    let trimmed = key.trim();
    let valid = match provider.as_str() {
        "openai" => trimmed.starts_with("sk-") && trimmed.len() >= 20,
        "gemini" => trimmed.len() >= 20,
        "anthropic" => trimmed.starts_with("sk-ant-") && trimmed.len() >= 20,
        _ => !trimmed.is_empty(),
    };
    Ok(valid)
}
