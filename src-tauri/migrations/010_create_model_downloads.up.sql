CREATE TABLE model_downloads (
    id              TEXT PRIMARY KEY,
    model_name      TEXT NOT NULL,
    model_path      TEXT NOT NULL,
    file_size       INTEGER NOT NULL,
    checksum_sha256 TEXT NOT NULL,
    quantization    TEXT NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT 0,
    downloaded_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_model_active ON model_downloads(is_active) WHERE is_active = 1;
