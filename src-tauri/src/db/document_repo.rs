use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub id: String,
    pub session_id: Option<String>,
    pub filename: String,
    pub file_path: String,
    pub file_size: u64,
    pub file_type: String,
    pub page_count: Option<u32>,
    pub chunk_count: u32,
    pub parsed_at: Option<String>,
    pub created_at: String,
    pub is_indexed: bool,
    pub index_path: Option<String>,
    pub metadata: Option<String>,
}

pub struct NewDocument<'a> {
    pub id: &'a str,
    pub session_id: Option<&'a str>,
    pub filename: &'a str,
    pub file_path: &'a str,
    pub file_size: u64,
    pub file_type: &'a str,
}

fn map_document(row: &Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get("id")?,
        session_id: row.get("session_id")?,
        filename: row.get("filename")?,
        file_path: row.get("file_path")?,
        file_size: row.get("file_size")?,
        file_type: row.get("file_type")?,
        page_count: row.get("page_count")?,
        chunk_count: row.get("chunk_count")?,
        parsed_at: row.get("parsed_at")?,
        created_at: row.get("created_at")?,
        is_indexed: row.get("is_indexed")?,
        index_path: row.get("index_path")?,
        metadata: row.get("metadata")?,
    })
}

pub fn create(conn: &Connection, doc: NewDocument) -> AppResult<Document> {
    conn.execute(
        "INSERT INTO documents (
            id, session_id, filename, file_path, file_size, file_type
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            doc.id,
            doc.session_id,
            doc.filename,
            doc.file_path,
            doc.file_size,
            doc.file_type
        ],
    )?;

    get(conn, doc.id).map(|opt| opt.expect("Document not found after insert"))
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Option<Document>> {
    conn.query_row(
        "SELECT * FROM documents WHERE id = ?1",
        params![id],
        map_document,
    )
    .optional()
    .map_err(Into::into)
}

pub fn list(conn: &Connection, session_id: Option<&str>) -> AppResult<Vec<Document>> {
    let mut query = "SELECT * FROM documents".to_string();
    let mut query_params: Vec<&dyn rusqlite::ToSql> = Vec::new();

    if let Some(ref sid) = session_id {
        query.push_str(" WHERE session_id = ?1");
        query_params.push(sid);
    }

    query.push_str(" ORDER BY created_at DESC");

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(query_params), map_document)?;

    let mut docs = Vec::new();
    for row in rows {
        docs.push(row?);
    }
    Ok(docs)
}

pub fn update_parsed_status(conn: &Connection, id: &str, chunk_count: u32) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET chunk_count = ?1, parsed_at = datetime('now') WHERE id = ?2",
        params![chunk_count, id],
    )?;
    Ok(())
}

pub fn update_indexed_status(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET is_indexed = 1 WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    Ok(())
}
