use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmServerState {
    Stopped,
    Starting,
    Running,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStatus {
    pub state: LlmServerState,
    pub host: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub healthy: bool,
    pub model_path: Option<String>,
    pub last_error: Option<String>,
    pub binary_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryModel {
    pub id: String,
    pub name: String,
    pub url: String,
    pub size_bytes: u64,
    pub checksum_sha256: String,
    pub quantization: String,
    pub min_vram_gb: u32,
    pub recommended_vram_gb: u32,
    pub context_length: u32,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadedModel {
    pub id: String,
    pub model_name: String,
    pub model_path: String,
    pub file_size: i64,
    pub checksum_sha256: String,
    pub quantization: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percent: f32,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub gpu_available: bool,
    pub gpu_name: Option<String>,
    pub vram_total_mb: Option<u64>,
    pub vram_free_mb: Option<u64>,
    pub recommended_gpu_layers: u32,
    pub backend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkReport {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub ttft_ms: f64,
    pub tokens_per_second: f64,
    pub total_ms: f64,
    pub backend: String,
    pub model_path: String,
    pub hardware: HardwareInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChatRequest {
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}
