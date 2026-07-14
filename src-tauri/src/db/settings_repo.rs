use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::utils::error::{AppError, AppResult};

/// A row from the `settings` table.
#[derive(Debug, Clone)]
pub struct SettingRow {
    pub key: String,
    pub value: String,
    pub encrypted: bool,
}

/// Get a single plain-text setting value.
pub fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1 AND encrypted = 0")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        return Ok(Some(row.get(0)?));
    }
    Ok(None)
}

/// Upsert a plain-text setting.
pub fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value, encrypted)
         VALUES (?1, ?2, 0)
         ON CONFLICT(key) DO UPDATE SET
           value = excluded.value,
           encrypted = 0,
           updated_at = datetime('now')",
        params![key, value],
    )?;
    Ok(())
}

/// Get all non-encrypted settings as a HashMap.
pub fn get_all(conn: &Connection) -> AppResult<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings WHERE encrypted = 0")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (k, v) = row.map_err(AppError::from)?;
        map.insert(k, v);
    }
    Ok(map)
}

/// Delete a setting by key.
pub fn delete(conn: &Connection, key: &str) -> AppResult<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (
                key        TEXT PRIMARY KEY,
                value      TEXT NOT NULL,
                encrypted  BOOLEAN NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn get_set_round_trip() {
        let conn = in_memory();
        set(&conn, "theme", "dark").unwrap();
        let val = get(&conn, "theme").unwrap();
        assert_eq!(val, Some("dark".to_string()));
    }

    #[test]
    fn upsert_replaces() {
        let conn = in_memory();
        set(&conn, "theme", "light").unwrap();
        set(&conn, "theme", "dark").unwrap();
        assert_eq!(get(&conn, "theme").unwrap(), Some("dark".to_string()));
    }

    #[test]
    fn missing_key_returns_none() {
        let conn = in_memory();
        assert!(get(&conn, "nonexistent").unwrap().is_none());
    }
}
