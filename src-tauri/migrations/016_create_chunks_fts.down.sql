-- Rollback migration for FTS5 table and triggers.
-- Since recreating the document_chunks table carries data loss risk,
-- we simply drop the virtual table and triggers.

DROP TRIGGER IF EXISTS fts_insert_trigger;
DROP TRIGGER IF EXISTS fts_delete_trigger;
DROP TRIGGER IF EXISTS fts_update_trigger;
DROP TABLE IF EXISTS document_chunks_fts;
