use rusqlite::{params, Connection, Transaction};

use crate::utils::error::{AppError, AppResult};

pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub up: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_sessions",
        up: include_str!("../../migrations/001_create_sessions.up.sql"),
    },
    Migration {
        version: 2,
        name: "create_messages",
        up: include_str!("../../migrations/002_create_messages.up.sql"),
    },
    Migration {
        version: 3,
        name: "create_documents",
        up: include_str!("../../migrations/003_create_documents.up.sql"),
    },
    Migration {
        version: 4,
        name: "create_document_chunks",
        up: include_str!("../../migrations/004_create_document_chunks.up.sql"),
    },
    Migration {
        version: 5,
        name: "create_research_sessions",
        up: include_str!("../../migrations/005_create_research_sessions.up.sql"),
    },
    Migration {
        version: 6,
        name: "create_research_queries",
        up: include_str!("../../migrations/006_create_research_queries.up.sql"),
    },
    Migration {
        version: 7,
        name: "create_audit_log",
        up: include_str!("../../migrations/007_create_audit_log.up.sql"),
    },
    Migration {
        version: 8,
        name: "create_settings",
        up: include_str!("../../migrations/008_create_settings.up.sql"),
    },
    Migration {
        version: 9,
        name: "create_api_keys",
        up: include_str!("../../migrations/009_create_api_keys.up.sql"),
    },
    Migration {
        version: 10,
        name: "create_model_downloads",
        up: include_str!("../../migrations/010_create_model_downloads.up.sql"),
    },
    Migration {
        version: 11,
        name: "placeholder",
        up: include_str!("../../migrations/011_placeholder.up.sql"),
    },
    Migration {
        version: 12,
        name: "seed_default_settings",
        up: include_str!("../../migrations/012_seed_default_settings.up.sql"),
    },
    Migration {
        version: 13,
        name: "add_session_system_prompt",
        up: include_str!("../../migrations/013_add_session_system_prompt.up.sql"),
    },
    Migration {
        version: 14,
        name: "create_compliance_templates",
        up: include_str!("../../migrations/014_create_compliance_templates.up.sql"),
    },
];

pub fn run_migrations(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS _migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE version = ?1",
            params![migration.version],
            |row| row.get(0),
        )?;

        if already_applied {
            continue;
        }

        let tx: Transaction = conn
            .transaction()
            .map_err(|err| AppError::Migration(format!("Failed to start transaction: {err}")))?;

        tx.execute_batch(migration.up).map_err(|err| {
            AppError::Migration(format!(
                "Migration {} ({}) failed: {err}",
                migration.version, migration.name
            ))
        })?;

        tx.execute(
            "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        )
        .map_err(|err| AppError::Migration(format!("Failed to record migration: {err}")))?;

        tx.commit()
            .map_err(|err| AppError::Migration(format!("Failed to commit migration: {err}")))?;

        tracing::info!(
            version = migration.version,
            name = migration.name,
            "Applied migration"
        );
    }

    Ok(())
}

pub fn current_version(conn: &Connection) -> AppResult<u32> {
    let version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    Ok(version)
}
