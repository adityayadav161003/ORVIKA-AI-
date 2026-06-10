use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub texts: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RerankRequest {
    pub query: String,
    pub candidates: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RerankResponse {
    pub scores: Vec<f32>,
}
