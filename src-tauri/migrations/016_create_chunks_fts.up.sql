-- Migration to create virtual FTS5 table for document chunks and set up triggers to keep it in sync.

CREATE VIRTUAL TABLE document_chunks_fts USING fts5(
    content,
    content='document_chunks',
    content_rowid='rowid'
);

-- Sync trigger on Insert
CREATE TRIGGER fts_insert_trigger AFTER INSERT ON document_chunks
BEGIN
    INSERT INTO document_chunks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Sync trigger on Delete
CREATE TRIGGER fts_delete_trigger AFTER DELETE ON document_chunks
BEGIN
    INSERT INTO document_chunks_fts(document_chunks_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

-- Sync trigger on Update
CREATE TRIGGER fts_update_trigger AFTER UPDATE ON document_chunks
BEGIN
    INSERT INTO document_chunks_fts(document_chunks_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO document_chunks_fts(rowid, content) VALUES (new.rowid, new.content);
END;
