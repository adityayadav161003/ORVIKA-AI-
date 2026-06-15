use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexableChunk {
    pub id: String,
    pub embedding: Vec<f32>,
    pub document_id: String,
    pub chunk_index: u32,
    pub page_number: Option<u32>,
    pub section_heading: Option<String>,
    pub heading_path: Option<String>,
    pub source_type: String, // e.g., "document" or "media_transcript"
    pub token_count: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchFilter {
    pub document_id: Option<String>,
    pub source_type: Option<String>,
    pub min_page: Option<u32>,
    pub max_page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FusedCandidate {
    pub chunk_id: String,
    pub dense_rank: Option<usize>,
    pub sparse_rank: Option<usize>,
    pub fused_score: f32,
}
