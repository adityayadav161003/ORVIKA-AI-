CREATE TABLE document_chunks (
    id              TEXT PRIMARY KEY,
    document_id     TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    chunk_index     INTEGER NOT NULL,
    content         TEXT NOT NULL,
    page_number     INTEGER,
    section_heading TEXT,
    embedding_id    INTEGER,
    token_count     INTEGER NOT NULL,
    start_char      INTEGER,
    end_char        INTEGER,
    metadata        TEXT
);

CREATE INDEX idx_chunks_document ON document_chunks(document_id, chunk_index);
CREATE INDEX idx_chunks_page ON document_chunks(document_id, page_number);
CREATE INDEX idx_chunks_embedding ON document_chunks(embedding_id);
