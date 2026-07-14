-- SQLite does not support DROP COLUMN in older versions.
-- The canonical rollback is to recreate the table without the new columns.
-- In practice this migration is unlikely to be rolled back; this file is
-- provided for completeness.
--
-- To roll back: recreate document_chunks without weaviate_id, is_indexed,
-- heading_path.  Not implemented here to avoid the risk of data loss from
-- an automated down-migration running unexpectedly.
SELECT 1; -- no-op placeholder
