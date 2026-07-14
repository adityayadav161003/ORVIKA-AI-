CREATE TABLE research_sessions (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    message_id      TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    status          TEXT NOT NULL DEFAULT 'planning'
                    CHECK (status IN ('planning', 'approved', 'in_progress', 'completed', 'failed')),
    total_queries   INTEGER NOT NULL DEFAULT 0,
    completed_queries INTEGER NOT NULL DEFAULT 0,
    knowledge_gaps  TEXT,
    metadata        TEXT
);

CREATE INDEX idx_research_session ON research_sessions(session_id);
CREATE INDEX idx_research_status ON research_sessions(status);
