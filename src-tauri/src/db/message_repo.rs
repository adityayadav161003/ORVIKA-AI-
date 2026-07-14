use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::utils::error::{AppError, AppResult};
use crate::utils::id::new_id;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub source_type: Option<String>,
    pub sources: Option<String>,
    pub created_at: String,
    pub tokens_used: Option<u32>,
    pub latency_ms: Option<u64>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewMessage<'a> {
    pub session_id: &'a str,
    pub role: &'a str,
    pub content: &'a str,
    pub source_type: Option<&'a str>,
    pub sources: Option<&'a str>,
    pub tokens_used: Option<u32>,
    pub latency_ms: Option<u64>,
    pub metadata: Option<&'a str>,
}

pub fn create(conn: &Connection, input: NewMessage<'_>) -> AppResult<Message> {
    let id = new_id();

    conn.execute(
        "INSERT INTO messages (
             id,
             session_id,
             role,
             content,
             source_type,
             sources,
             tokens_used,
             latency_ms,
             metadata
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            input.session_id,
            input.role,
            input.content,
            input.source_type,
            input.sources,
            input.tokens_used.map(i64::from),
            input.latency_ms.map(|value| value as i64),
            input.metadata,
        ],
    )?;

    get(conn, &id)?.ok_or_else(|| AppError::Other("Message was not created".to_string()))
}

pub fn update_content(conn: &Connection, message_id: &str, content: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET content = ?1 WHERE id = ?2",
        params![content, message_id],
    )?;
    Ok(())
}

pub fn update_metadata(
    conn: &Connection,
    message_id: &str,
    tokens_used: Option<u32>,
    latency_ms: Option<u64>,
    metadata: Option<&str>,
) -> AppResult<()> {
    conn.execute(
        "UPDATE messages SET tokens_used = ?1, latency_ms = ?2, metadata = ?3 WHERE id = ?4",
        params![
            tokens_used.map(i64::from),
            latency_ms.map(|v| v as i64),
            metadata,
            message_id
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, message_id: &str) -> AppResult<Option<Message>> {
    let mut stmt = conn.prepare(
        "SELECT
           id,
           session_id,
           role,
           content,
           source_type,
           sources,
           created_at,
           tokens_used,
           latency_ms,
           metadata
         FROM messages
         WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![message_id])?;

    if let Some(row) = rows.next()? {
        return message_from_row(row).map(Some).map_err(AppError::from);
    }

    Ok(None)
}

pub fn list_for_session(
    conn: &Connection,
    session_id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
) -> AppResult<Vec<Message>> {
    let limit = limit.unwrap_or(100).min(500);
    let offset = offset.unwrap_or(0);
    let mut stmt = conn.prepare(
        "SELECT
           id,
           session_id,
           role,
           content,
           source_type,
           sources,
           created_at,
           tokens_used,
           latency_ms,
           metadata
         FROM messages
         WHERE session_id = ?1
         ORDER BY datetime(created_at) ASC, rowid ASC
         LIMIT ?2 OFFSET ?3",
    )?;

    let rows = stmt.query_map(params![session_id, limit, offset], message_from_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)
}

pub fn recent_for_context(
    conn: &Connection,
    session_id: &str,
    limit: u32,
) -> AppResult<Vec<Message>> {
    let limit = limit.min(100);
    let mut stmt = conn.prepare(
        "SELECT
           id,
           session_id,
           role,
           content,
           source_type,
           sources,
           created_at,
           tokens_used,
           latency_ms,
           metadata
         FROM messages
         WHERE session_id = ?1
         ORDER BY datetime(created_at) DESC, rowid DESC
         LIMIT ?2",
    )?;

    let mut messages = stmt
        .query_map(params![session_id, limit], message_from_row)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    messages.reverse();
    Ok(messages)
}

fn message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        session_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        source_type: row.get(4)?,
        sources: row.get(5)?,
        created_at: row.get(6)?,
        tokens_used: row.get::<_, Option<i64>>(7)?.map(|value| value as u32),
        latency_ms: row.get::<_, Option<i64>>(8)?.map(|value| value as u64),
        metadata: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
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
    fn create_and_list_messages_in_order() {
        let conn = in_memory();
        create(
            &conn,
            NewMessage {
                session_id: "s1",
                role: "user",
                content: "Hello",
                source_type: None,
                sources: None,
                tokens_used: None,
                latency_ms: None,
                metadata: None,
            },
        )
        .unwrap();
        create(
            &conn,
            NewMessage {
                session_id: "s1",
                role: "assistant",
                content: "Hi",
                source_type: Some("local"),
                sources: None,
                tokens_used: Some(1),
                latency_ms: Some(25),
                metadata: None,
            },
        )
        .unwrap();

        let messages = list_for_session(&conn, "s1", None, None).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].source_type, Some("local".to_string()));
    }
}
