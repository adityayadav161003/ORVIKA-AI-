CREATE TABLE research_queries (
    id              TEXT PRIMARY KEY,
    research_session_id TEXT NOT NULL REFERENCES research_sessions(id) ON DELETE CASCADE,
    query_index     INTEGER NOT NULL,
    topic           TEXT NOT NULL,
    raw_query       TEXT,
    sanitized_query TEXT NOT NULL,
    risk_level      TEXT NOT NULL
                    CHECK (risk_level IN ('low', 'medium', 'high')),
    status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected', 'sent', 'completed', 'failed', 'blocked')),
    user_approved   BOOLEAN,
    cloud_provider  TEXT,
    cloud_model     TEXT,
    response        TEXT,
    tokens_used     INTEGER,
    latency_ms      INTEGER,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at    TEXT,
    sanitization_log TEXT,
    metadata        TEXT
);

CREATE INDEX idx_rq_session ON research_queries(research_session_id);
CREATE INDEX idx_rq_status ON research_queries(status);
