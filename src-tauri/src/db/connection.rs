use rusqlite::Connection;

use crate::utils::error::AppResult;

pub fn configure_connection(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA cache_size = -64000;
        PRAGMA mmap_size = 268435456;
        ",
    )?;
    Ok(())
}
