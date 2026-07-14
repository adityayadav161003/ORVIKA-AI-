use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::utils::error::{AppError, AppResult};
use crate::utils::id::new_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub research_mode_enabled: bool,
    pub cloud_provider: Option<String>,
    pub privacy_level: String,
    pub model_id: String,
    pub is_active: bool,
    pub metadata: Option<String>,
    pub system_prompt: Option<String>,
    pub message_count: u32,
}

pub fn create(conn: &Connection, name: &str, model_id: &str) -> AppResult<Session> {
    let id = new_id();
    let trimmed_name = name.trim();
    let display_name = if trimmed_name.is_empty() {
        "New chat"
    } else {
        trimmed_name
    };

    conn.execute(
        "INSERT INTO sessions (id, name, model_id)
         VALUES (?1, ?2, ?3)",
        params![id, display_name, model_id],
    )?;

    get(conn, &id)?.ok_or_else(|| AppError::Other("Session was not created".to_string()))
}

pub fn list(conn: &Connection) -> AppResult<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT
           s.id,
           s.name,
           s.created_at,
           s.updated_at,
           s.research_mode_enabled,
           s.cloud_provider,
           s.privacy_level,
           s.model_id,
           s.is_active,
           s.metadata,
           s.system_prompt,
           COUNT(m.id) AS message_count
         FROM sessions s
         LEFT JOIN messages m ON m.session_id = s.id
         WHERE s.is_active = 1
         GROUP BY s.id
         ORDER BY datetime(s.updated_at) DESC, s.rowid DESC",
    )?;

    let rows = stmt.query_map([], session_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn get(conn: &Connection, session_id: &str) -> AppResult<Option<Session>> {
    conn.query_row(
        "SELECT
           s.id,
           s.name,
           s.created_at,
           s.updated_at,
           s.research_mode_enabled,
           s.cloud_provider,
           s.privacy_level,
           s.model_id,
           s.is_active,
           s.metadata,
           s.system_prompt,
           COUNT(m.id) AS message_count
         FROM sessions s
         LEFT JOIN messages m ON m.session_id = s.id
         WHERE s.id = ?1 AND s.is_active = 1
         GROUP BY s.id",
        params![session_id],
        session_from_row,
    )
    .optional()
    .map_err(AppError::from)
}

pub fn delete(conn: &Connection, session_id: &str) -> AppResult<()> {
    let deleted = conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    if deleted == 0 {
        return Err(AppError::Other(format!("Session not found: {session_id}")));
    }
    Ok(())
}

pub fn touch(conn: &Connection, session_id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE sessions SET updated_at = datetime('now') WHERE id = ?1",
        params![session_id],
    )?;
    Ok(())
}

pub fn rename(conn: &Connection, session_id: &str, name: &str) -> AppResult<Session> {
    let trimmed_name = name.trim();
    if trimmed_name.is_empty() {
        return Err(AppError::Other("Session name cannot be empty".to_string()));
    }

    let updated = conn.execute(
        "UPDATE sessions
         SET name = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![trimmed_name, session_id],
    )?;
    if updated == 0 {
        return Err(AppError::Other(format!("Session not found: {session_id}")));
    }

    get(conn, session_id)?.ok_or_else(|| AppError::Other("Session not found".to_string()))
}

pub fn update_system_prompt(conn: &Connection, session_id: &str, prompt: Option<&str>) -> AppResult<()> {
    let updated = conn.execute(
        "UPDATE sessions
         SET system_prompt = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![prompt, session_id],
    )?;
    if updated == 0 {
        return Err(AppError::Other(format!("Session not found: {session_id}")));
    }
    Ok(())
}

fn session_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        name: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        research_mode_enabled: row.get::<_, i64>(4)? != 0,
        cloud_provider: row.get(5)?,
        privacy_level: row.get(6)?,
        model_id: row.get(7)?,
        is_active: row.get::<_, i64>(8)? != 0,
        metadata: row.get(9)?,
        system_prompt: row.get(10)?,
        message_count: row.get::<_, i64>(11)? as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                research_mode_enabled BOOLEAN NOT NULL DEFAULT 0,
                cloud_provider TEXT,
                privacy_level TEXT NOT NULL DEFAULT 'balanced',
                model_id TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                metadata TEXT,
                system_prompt TEXT
            );
            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                source_type TEXT,
                sources TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                tokens_used INTEGER,
                latency_ms INTEGER,
                metadata TEXT
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn create_and_list_session() {
        let conn = in_memory();
        let session = create(&conn, "Research notes", "local-default").unwrap();

        let sessions = list(&conn).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, session.id);
        assert_eq!(sessions[0].name, "Research notes");
        assert_eq!(sessions[0].message_count, 0);
    }

    #[test]
    fn delete_removes_session() {
        let conn = in_memory();
        let session = create(&conn, "Research notes", "local-default").unwrap();
        delete(&conn, &session.id).unwrap();

        assert!(get(&conn, &session.id).unwrap().is_none());
    }
}
