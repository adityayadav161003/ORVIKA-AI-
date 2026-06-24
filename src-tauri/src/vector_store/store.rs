use crate::db::Database;
use crate::utils::error::AppResult;
use crate::vector_store::chroma_client::ChromaClient;
use crate::vector_store::types::{IndexableChunk, SearchFilter};
use std::path::Path;
use std::sync::Arc;

pub struct VectorStore {
    chroma_client: ChromaClient,
    db: Arc<Database>,
}

impl VectorStore {
    pub fn new(app_data_dir: &Path, db: Arc<Database>) -> AppResult<Self> {
        let chroma_client = ChromaClient::new("http://127.0.0.1:8082".to_string());
        Ok(Self { chroma_client, db })
    }

    pub async fn add_chunks(&self, chunks: Vec<IndexableChunk>) -> AppResult<()> {
        self.chroma_client.add_batch(chunks).await
    }

    pub async fn remove_document(&self, document_id: &str) -> AppResult<()> {
        self.chroma_client.delete_by_document(document_id).await
    }

    pub async fn clear(&self) -> AppResult<()> {
        let mut filter1 = std::collections::HashMap::new();
        filter1.insert(
            "sourceType".to_string(),
            serde_json::Value::String("document".to_string()),
        );
        let _ = self.chroma_client.delete_by_filter(filter1).await;

        let mut filter2 = std::collections::HashMap::new();
        filter2.insert(
            "sourceType".to_string(),
            serde_json::Value::String("media_transcript".to_string()),
        );
        let _ = self.chroma_client.delete_by_filter(filter2).await;

        Ok(())
    }

    pub async fn search(
        &self,
        query: &str,
        query_vector: Vec<f32>,
        limit: usize,
        filter: SearchFilter,
    ) -> AppResult<Vec<(String, f32)>> {
        // 1. Semantic (dense) search from Chroma
        let chroma_res = self
            .chroma_client
            .query(
                query_vector,
                limit * 2, // fetch slightly more for fusion
                filter.document_id.as_deref(),
                filter.source_type.as_deref(),
            )
            .await?;

        let mut dense_results = Vec::new();
        for (id, dist) in chroma_res
            .ids
            .into_iter()
            .zip(chroma_res.distances.into_iter())
        {
            dense_results.push((id, dist));
        }

        // 2. Keyword (sparse) search from SQLite FTS5
        let sparse_results = self.db.with_connection(|conn| {
            crate::vector_store::fts_search::search_bm25(conn, query, &filter, limit * 2)
        })?;

        // 3. Reciprocal Rank Fusion (RRF)
        let fused = crate::vector_store::fusion::reciprocal_rank_fusion(
            dense_results,
            sparse_results,
            60.0,
        );

        let mut hits: Vec<(String, f32)> = fused
            .into_iter()
            .map(|c| (c.chunk_id, c.fused_score))
            .collect();

        hits.truncate(limit);
        Ok(hits)
    }
}
