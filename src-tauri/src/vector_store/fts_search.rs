use crate::utils::error::AppResult;
use crate::vector_store::types::SearchFilter;
use rusqlite::Connection;

pub fn search_bm25(
    conn: &Connection,
    query: &str,
    filter: &SearchFilter,
    top_k: usize,
) -> AppResult<Vec<(String, f32)>> {
    let sanitized = sanitize_query(query);
    if sanitized.is_empty() {
        return Ok(vec![]);
    }

    // SQLite FTS5 query joining the virtual table with the main document_chunks table
    let mut sql = "
        SELECT dc.id, bm25(document_chunks_fts) as score
        FROM document_chunks_fts fts
        JOIN document_chunks dc ON dc.rowid = fts.rowid
        WHERE fts.content MATCH ?1
    "
    .to_string();

    let mut sql_params: Vec<&dyn rusqlite::ToSql> = vec![&sanitized];

    // Keep optional pattern string in scope so its reference remains valid
    let pattern = filter
        .source_type
        .as_ref()
        .map(|src_type| format!("%\"sourceType\":\"{}\"%", src_type));

    if let Some(ref doc_id) = filter.document_id {
        sql_params.push(doc_id);
        sql.push_str(&format!(" AND dc.document_id = ?{}", sql_params.len()));
    }

    if let Some(ref p) = pattern {
        sql_params.push(p);
        sql.push_str(&format!(" AND dc.metadata LIKE ?{}", sql_params.len()));
    }

    if let Some(ref min_page) = filter.min_page {
        sql_params.push(min_page);
        sql.push_str(&format!(" AND dc.page_number >= ?{}", sql_params.len()));
    }

    if let Some(ref max_page) = filter.max_page {
        sql_params.push(max_page);
        sql.push_str(&format!(" AND dc.page_number <= ?{}", sql_params.len()));
    }

    sql.push_str(" ORDER BY score ASC"); // SQLite BM25 returns negative values, lower is better
    let limit_val = top_k as i64;
    sql_params.push(&limit_val);
    sql.push_str(&format!(" LIMIT ?{}", sql_params.len()));

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(sql_params), |row| {
        let id: String = row.get(0)?;
        let score: f32 = row.get(1)?;
        // Convert the raw score to a positive relevance score (higher is more relevant)
        // Since bm25 returns negative values (e.g., -5.0 is better than -1.0),
        // we can return -score so that it aligns with standard search scores.
        Ok((id, -score))
    })?;

    let mut hits = Vec::new();
    for row in rows {
        hits.push(row?);
    }

    Ok(hits)
}

fn sanitize_query(query: &str) -> String {
    let cleaned: String = query
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect();

    cleaned
        .split_whitespace()
        .map(|w| format!("\"{}\"", w))
        .collect::<Vec<_>>()
        .join(" OR ")
}
