CREATE TABLE audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp       TEXT NOT NULL DEFAULT (datetime('now')),
    event_type      TEXT NOT NULL,
    session_id      TEXT,
    details         TEXT NOT NULL,
    outbound_content TEXT,
    destination     TEXT,
    response_summary TEXT,
    sanitization_result TEXT,
    risk_level      TEXT
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_session ON audit_log(session_id);
