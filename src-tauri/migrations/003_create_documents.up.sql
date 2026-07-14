CREATE TABLE documents (
    id              TEXT PRIMARY KEY,
    session_id      TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    filename        TEXT NOT NULL,
    file_path       TEXT NOT NULL,
    file_size       INTEGER NOT NULL,
    file_type       TEXT NOT NULL,
    page_count      INTEGER,
    chunk_count     INTEGER NOT NULL DEFAULT 0,
    parsed_at       TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    is_indexed      BOOLEAN NOT NULL DEFAULT 0,
    index_path      TEXT,
    metadata        TEXT
);

CREATE INDEX idx_documents_session ON documents(session_id);
CREATE INDEX idx_documents_type ON documents(file_type);
CREATE INDEX idx_documents_indexed ON documents(is_indexed);
