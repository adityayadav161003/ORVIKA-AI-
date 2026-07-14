use rusqlite::{params, Connection, OptionalExtension};

/// Checks if a query has already been successfully executed and returns the cached response if available.
pub fn get_cached_response(conn: &Connection, query_text: &str) -> Option<String> {
    conn.query_row(
        "SELECT response FROM research_queries 
         WHERE sanitized_query = ?1 AND status = 'completed' AND response IS NOT NULL 
         LIMIT 1",
        params![query_text],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .unwrap_or(None)
}
