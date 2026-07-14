use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};

use crate::utils::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: u32,
    pub content: String,
    pub page_number: Option<u32>,
    pub section_heading: Option<String>,
    pub embedding_id: Option<u32>,
    pub token_count: u32,
    pub start_char: Option<u32>,
    pub end_char: Option<u32>,
    pub metadata: Option<String>,
}

pub struct NewChunk {
    pub id: String,
    pub document_id: String,
    pub chunk_index: u32,
    pub content: String,
    pub token_count: u32,
    pub page_number: Option<u32>,
    pub section_heading: Option<String>,
    pub metadata: Option<String>,
}

fn map_chunk(row: &Row) -> rusqlite::Result<DocumentChunk> {
    Ok(DocumentChunk {
        id: row.get("id")?,
        document_id: row.get("document_id")?,
        chunk_index: row.get("chunk_index")?,
        content: row.get("content")?,
        page_number: row.get("page_number")?,
        section_heading: row.get("section_heading")?,
        embedding_id: row.get("embedding_id")?,
        token_count: row.get("token_count")?,
        start_char: row.get("start_char")?,
        end_char: row.get("end_char")?,
        metadata: row.get("metadata")?,
    })
}

pub fn insert_batch(conn: &Connection, chunks: &[NewChunk]) -> AppResult<()> {
    conn.execute("BEGIN TRANSACTION", [])?;
    
    let result: AppResult<()> = (|| {
        let mut stmt = conn.prepare(
            "INSERT INTO document_chunks (
                id, document_id, chunk_index, content, token_count, page_number, section_heading, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;
        
        for chunk in chunks {
            stmt.execute(params![
                &chunk.id,
                &chunk.document_id,
                chunk.chunk_index,
                &chunk.content,
                chunk.token_count,
                chunk.page_number,
                chunk.section_heading,
                chunk.metadata,
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


pub fn get_for_document(conn: &Connection, document_id: &str) -> AppResult<Vec<DocumentChunk>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM document_chunks WHERE document_id = ?1 ORDER BY chunk_index ASC"
    )?;
    
    let rows = stmt.query_map(params![document_id], map_chunk)?;
    
    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row?);
    }
    Ok(chunks)
}

pub fn delete_for_document(conn: &Connection, document_id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM document_chunks WHERE document_id = ?1", params![document_id])?;
    Ok(())
}

pub fn update_embedding_ids(conn: &Connection, document_id: &str, chunk_ids: &[String], embedding_ids: &[i64]) -> AppResult<()> {
    if chunk_ids.len() != embedding_ids.len() {
        return Err(crate::utils::error::AppError::Other("Mismatched chunks and embeddings".into()));
    }

    conn.execute("BEGIN TRANSACTION", [])?;
    
    let result: AppResult<()> = (|| {
        let mut stmt = conn.prepare(
            "UPDATE document_chunks SET embedding_id = ?1 WHERE id = ?2"
        )?;
        
        for (chunk_id, emb_id) in chunk_ids.iter().zip(embedding_ids.iter()) {
            stmt.execute(params![emb_id, chunk_id])?;
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

pub fn get_chunks_by_embedding_ids(conn: &Connection, embedding_ids: &[i64]) -> AppResult<Vec<DocumentChunk>> {
    if embedding_ids.is_empty() {
        return Ok(vec![]);
    }

    // Construct IN clause dynamically
    let placeholders: Vec<String> = (0..embedding_ids.len()).map(|_| "?".to_string()).collect();
    let query = format!(
        "SELECT * FROM document_chunks WHERE embedding_id IN ({})",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&query)?;
    
    let params: Vec<&dyn rusqlite::ToSql> = embedding_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
    let rows = stmt.query_map(rusqlite::params_from_iter(params), map_chunk)?;
    
    let mut chunks = Vec::new();
    for row in rows {
        chunks.push(row?);
    }
    Ok(chunks)
}
