# ADR 004: SQLite Primary Database

**Status:** Accepted

## Context

The app needs embedded relational storage for sessions, messages, documents, research, and audit logs.

## Decision

Use **SQLite 3** via `rusqlite` with WAL mode, foreign keys, and a custom migration runner.

## Consequences

- Zero-config local database suitable for desktop
- Migration files versioned in `src-tauri/migrations/`
- Vector data stored separately in FAISS
