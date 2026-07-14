CREATE TABLE messages (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content         TEXT NOT NULL,
    source_type     TEXT CHECK (source_type IN ('local', 'research', 'mixed', NULL)),
    sources         TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    tokens_used     INTEGER,
    latency_ms      INTEGER,
    metadata        TEXT
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);
CREATE INDEX idx_messages_role ON messages(role);
