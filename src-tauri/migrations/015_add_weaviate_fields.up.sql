-- Sprint 3/4: Extend document_chunks with Weaviate integration fields.
--
-- weaviate_id: the UUID Weaviate assigns to the indexed object (FK-in-spirit
--              back to Weaviate's DocumentChunk collection).  NULL until Sprint 4
--              writes the chunk to Weaviate.
--
-- is_indexed: 0 = not yet indexed in Weaviate (either Sprint 4 hasn't run yet,
--             or the Weaviate write failed and needs retry).
--             1 = successfully indexed.
--
-- heading_path: full breadcrumb stored as plain text ("H1 > H2 > H3"),
--               replicated here from the chunk metadata JSON for easy querying.

ALTER TABLE document_chunks ADD COLUMN weaviate_id TEXT;
ALTER TABLE document_chunks ADD COLUMN is_indexed INTEGER NOT NULL DEFAULT 0;
ALTER TABLE document_chunks ADD COLUMN heading_path TEXT;
