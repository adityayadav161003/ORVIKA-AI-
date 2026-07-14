CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    research_mode_enabled BOOLEAN NOT NULL DEFAULT 0,
    cloud_provider  TEXT,
    privacy_level   TEXT NOT NULL DEFAULT 'balanced'
                    CHECK (privacy_level IN ('strict', 'balanced', 'permissive')),
    model_id        TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    metadata        TEXT
);

CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC);
CREATE INDEX idx_sessions_active ON sessions(is_active) WHERE is_active = 1;
