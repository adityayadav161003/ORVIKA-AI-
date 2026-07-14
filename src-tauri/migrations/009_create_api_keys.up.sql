CREATE TABLE api_keys (
    provider        TEXT PRIMARY KEY,
    encrypted_key   TEXT NOT NULL,
    iv              TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at    TEXT
);
