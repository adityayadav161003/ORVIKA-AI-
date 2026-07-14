use rusqlite::{params, Connection, Row, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSession {
    pub id: String,
    pub session_id: String,
    pub message_id: String,
    pub status: String,
    pub total_queries: u32,
    pub completed_queries: u32,
    pub knowledge_gaps: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchQuery {
    pub id: String,
    pub research_session_id: String,
    pub query_index: u32,
    pub topic: String,
    pub raw_query: Option<String>,
    pub sanitized_query: String,
    pub risk_level: String,
    pub status: String,
    pub user_approved: Option<bool>,
    pub response: Option<String>,
    pub created_at: String,
}

pub struct NewResearchSession<'a> {
    pub id: &'a str,
    pub session_id: &'a str,
    pub message_id: &'a str,
    pub status: &'a str,
    pub total_queries: u32,
    pub knowledge_gaps: Option<&'a str>,
}

pub struct NewResearchQuery {
    pub id: String,
    pub research_session_id: String,
    pub query_index: u32,
    pub topic: String,
    pub raw_query: Option<String>,
    pub sanitized_query: String,
    pub risk_level: String,
    pub status: String,
}

fn map_session(row: &Row) -> rusqlite::Result<ResearchSession> {
    Ok(ResearchSession {
        id: row.get("id")?,
        session_id: row.get("session_id")?,
        message_id: row.get("message_id")?,
        status: row.get("status")?,
        total_queries: row.get("total_queries")?,
        completed_queries: row.get("completed_queries")?,
        knowledge_gaps: row.get("knowledge_gaps")?,
        created_at: row.get("created_at")?,
    })
}

fn map_query(row: &Row) -> rusqlite::Result<ResearchQuery> {
    Ok(ResearchQuery {
        id: row.get("id")?,
        research_session_id: row.get("research_session_id")?,
        query_index: row.get("query_index")?,
        topic: row.get("topic")?,
        raw_query: row.get("raw_query")?,
        sanitized_query: row.get("sanitized_query")?,
        risk_level: row.get("risk_level")?,
        status: row.get("status")?,
        user_approved: row.get("user_approved")?,
        response: row.get("response")?,
        created_at: row.get("created_at")?,
    })
}

pub fn create_session(conn: &Connection, session: NewResearchSession) -> AppResult<ResearchSession> {
    conn.execute(
        "INSERT INTO research_sessions (id, session_id, message_id, status, total_queries, knowledge_gaps)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            session.id,
            session.session_id,
            session.message_id,
            session.status,
            session.total_queries,
            session.knowledge_gaps
        ],
    )?;

    get_session(conn, session.id).map(|opt| opt.expect("Research session not found after insert"))
}

pub fn create_queries(conn: &Connection, queries: &[NewResearchQuery]) -> AppResult<()> {
    conn.execute("BEGIN TRANSACTION", [])?;
    
    let result: AppResult<()> = (|| {
        let mut stmt = conn.prepare(
            "INSERT INTO research_queries (id, research_session_id, query_index, topic, raw_query, sanitized_query, risk_level, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;
        
        for q in queries {
            stmt.execute(params![
                q.id,
                q.research_session_id,
                q.query_index,
                q.topic,
                q.raw_query,
                q.sanitized_query,
                q.risk_level,
                q.status
            ])?;
        }
        Ok(())
    })();
    
    if result.is_ok() {
        conn.execute("COMMIT", [])?;
    } else {
        conn.execute("ROLLBACK", [])?;
    }
    result
}

pub fn get_session(conn: &Connection, id: &str) -> AppResult<Option<ResearchSession>> {
    conn.query_row(
        "SELECT * FROM research_sessions WHERE id = ?1",
        params![id],
        map_session,
    ).optional().map_err(Into::into)
}

pub fn list_queries(conn: &Connection, research_session_id: &str) -> AppResult<Vec<ResearchQuery>> {
    let mut stmt = conn.prepare("SELECT * FROM research_queries WHERE research_session_id = ?1 ORDER BY query_index ASC")?;
    let rows = stmt.query_map(params![research_session_id], map_query)?;
    
    let mut queries = Vec::new();
    for row in rows {
        queries.push(row?);
    }
    Ok(queries)
}

pub fn update_session_status(conn: &Connection, id: &str, status: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE research_sessions SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

pub fn update_query_status(conn: &Connection, id: &str, status: &str, user_approved: Option<bool>) -> AppResult<()> {
    conn.execute(
        "UPDATE research_queries SET status = ?1, user_approved = COALESCE(?2, user_approved) WHERE id = ?3",
        params![status, user_approved, id],
    )?;
    Ok(())
}

pub fn update_query_response(conn: &Connection, id: &str, response: &str, status: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE research_queries SET response = ?1, status = ?2 WHERE id = ?3",
        params![response, status, id],
    )?;
    Ok(())
}

pub fn increment_completed_queries(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE research_sessions SET completed_queries = completed_queries + 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn list_all_sessions(conn: &Connection) -> AppResult<Vec<ResearchSession>> {
    let mut stmt = conn.prepare("SELECT * FROM research_sessions ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], map_session)?;
    
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

pub fn delete_session(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM research_sessions WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

