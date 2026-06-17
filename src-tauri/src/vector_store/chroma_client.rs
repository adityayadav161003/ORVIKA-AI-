use crate::utils::error::{AppError, AppResult};
use crate::vector_store::types::IndexableChunk;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct ChromaClient {
    client: Client,
    base_url: String,
}

impl ChromaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url,
        }
    }

    pub async fn add_batch(&self, chunks: Vec<IndexableChunk>) -> AppResult<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        #[derive(Serialize)]
        struct AddRequest {
            ids: Vec<String>,
            embeddings: Vec<Vec<f32>>,
            metadatas: Vec<HashMap<String, serde_json::Value>>,
            documents: Vec<String>,
        }

        let mut ids = Vec::with_capacity(chunks.len());
        let mut embeddings = Vec::with_capacity(chunks.len());
        let mut metadatas = Vec::with_capacity(chunks.len());
        let mut documents = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            ids.push(chunk.id.clone());
            embeddings.push(chunk.embedding.clone());
            documents.push(chunk.content.clone());

            let mut meta = HashMap::new();
            meta.insert(
                "documentId".to_string(),
                serde_json::Value::String(chunk.document_id),
            );
            meta.insert(
                "chunkIndex".to_string(),
                serde_json::Value::Number(serde_json::Number::from(chunk.chunk_index)),
            );
            if let Some(p) = chunk.page_number {
                meta.insert(
                    "pageNumber".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(p)),
                );
            }
            if let Some(sh) = chunk.section_heading {
                meta.insert("sectionHeading".to_string(), serde_json::Value::String(sh));
            }
            if let Some(hp) = chunk.heading_path {
                meta.insert("headingPath".to_string(), serde_json::Value::String(hp));
            }
            meta.insert(
                "sourceType".to_string(),
                serde_json::Value::String(chunk.source_type),
            );
            meta.insert(
                "tokenCount".to_string(),
                serde_json::Value::Number(serde_json::Number::from(chunk.token_count)),
            );

            metadatas.push(meta);
        }

        let url = format!("{}/vector/add", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&AddRequest {
                ids,
                embeddings,
                metadatas,
                documents,
            })
            .send()
            .await
            .map_err(|e| AppError::Other(format!("Failed to send add request to Chroma: {}", e)))?;

        if !resp.status().is_success() {
            let err_msg = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "Chroma add returned error: {}",
                err_msg
            )));
        }

        Ok(())
    }

    pub async fn query(
        &self,
        query_embedding: Vec<f32>,
        n_results: usize,
        document_id: Option<&str>,
        source_type: Option<&str>,
    ) -> AppResult<ChromaQueryResponse> {
        #[derive(Serialize)]
        struct QueryRequest {
            query_embedding: Vec<f32>,
            n_results: usize,
            r#where: Option<HashMap<String, serde_json::Value>>,
        }

        let mut filter = HashMap::new();
        if let Some(doc_id) = document_id {
            filter.insert(
                "documentId".to_string(),
                serde_json::Value::String(doc_id.to_string()),
            );
        }
        if let Some(src_t) = source_type {
            filter.insert(
                "sourceType".to_string(),
                serde_json::Value::String(src_t.to_string()),
            );
        }

        let where_clause = if filter.is_empty() {
            None
        } else if filter.len() == 1 {
            Some(filter)
        } else {
            // Chroma's $and logic: {"$and": [{"documentId": "doc1"}, {"sourceType": "document"}]}
            let mut and_vec = Vec::new();
            for (k, v) in filter {
                let mut map = HashMap::new();
                map.insert(k, v);
                and_vec.push(serde_json::Value::Object(map.into_iter().collect()));
            }
            let mut and_map = HashMap::new();
            and_map.insert("$and".to_string(), serde_json::Value::Array(and_vec));
            Some(and_map)
        };

        let url = format!("{}/vector/query", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&QueryRequest {
                query_embedding,
                n_results,
                r#where: where_clause,
            })
            .send()
            .await
            .map_err(|e| AppError::Other(format!("Failed to query Chroma: {}", e)))?;

        if !resp.status().is_success() {
            let err_msg = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "Chroma query returned error: {}",
                err_msg
            )));
        }

        let response: ChromaQueryResponse = resp.json().await.map_err(|e| {
            AppError::Other(format!("Failed to parse Chroma query response: {}", e))
        })?;

        Ok(response)
    }

    pub async fn delete_by_document(&self, document_id: &str) -> AppResult<()> {
        let mut filter = HashMap::new();
        filter.insert(
            "documentId".to_string(),
            serde_json::Value::String(document_id.to_string()),
        );
        self.delete_by_filter(filter).await
    }

    pub async fn delete_by_filter(
        &self,
        filter: HashMap<String, serde_json::Value>,
    ) -> AppResult<()> {
        #[derive(Serialize)]
        struct DeleteRequest {
            r#where: HashMap<String, serde_json::Value>,
        }

        let url = format!("{}/vector/delete", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&DeleteRequest { r#where: filter })
            .send()
            .await
            .map_err(|e| AppError::Other(format!("Failed to delete from Chroma: {}", e)))?;

        if !resp.status().is_success() {
            let err_msg = resp.text().await.unwrap_or_default();
            return Err(AppError::Other(format!(
                "Chroma delete returned error: {}",
                err_msg
            )));
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChromaQueryResponse {
    pub ids: Vec<String>,
    pub distances: Vec<f32>,
    pub metadatas: Vec<HashMap<String, serde_json::Value>>,
    pub documents: Vec<String>,
}
