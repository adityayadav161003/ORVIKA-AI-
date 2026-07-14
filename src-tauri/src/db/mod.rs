use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OpenFlags};
use serde::Serialize;

use crate::utils::error::{AppError, AppResult};

pub mod api_key_repo;
pub mod compliance_repo;
pub mod audit_repo;
pub mod chunk_repo;
pub mod connection;
pub mod document_repo;
pub mod message_repo;
pub mod migration;
pub mod model_repo;
pub mod research_repo;
pub mod session_repo;
pub mod settings_repo;

pub use connection::configure_connection;
pub use migration::{current_version, run_migrations};

#[derive(Debug)]
pub struct Database {
    path: PathBuf,
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatus {
    pub path: String,
    pub version: u32,
    pub wal_mode: bool,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        configure_connection(&conn)?;

        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    pub fn run_migrations(&self) -> AppResult<()> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| AppError::Migration("Database lock poisoned".into()))?;
        run_migrations(&mut conn)
    }

    pub fn status(&self) -> AppResult<DbStatus> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AppError::Migration("Database lock poisoned".into()))?;

        let wal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;

        Ok(DbStatus {
            path: self.path.display().to_string(),
            version: current_version(&conn)?,
            wal_mode: wal_mode.eq_ignore_ascii_case("wal"),
        })
    }

    pub fn with_connection<F, R>(&self, f: F) -> AppResult<R>
    where
        F: FnOnce(&Connection) -> AppResult<R>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|_| AppError::Migration("Database lock poisoned".into()))?;
        f(&conn)
    }

    #[cfg(test)]
    pub fn connection(&self) -> AppResult<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::Migration("Database lock poisoned".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrations_apply_cleanly_twice() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join("app.db");
        let db = Database::open(&db_path).expect("open db");

        db.run_migrations().expect("first migration run");
        db.run_migrations()
            .expect("second migration run idempotent");

        let status = db.status().expect("status");
        assert!(status.wal_mode);
        let expected_version = migration::MIGRATIONS
            .iter()
            .map(|m| m.version)
            .max()
            .expect("migrations");
        assert_eq!(status.version, expected_version);
    }
}
