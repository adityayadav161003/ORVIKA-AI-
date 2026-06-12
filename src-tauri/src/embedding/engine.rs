use crate::db::Database;
use crate::python::manager::PythonManager;
use crate::utils::error::{AppError, AppResult};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const AI_SERVER_PORT: u16 = 8082;
pub const AI_SERVER_HOST: &str = "127.0.0.1";

pub struct EmbeddingEngine {
    app_data_dir: PathBuf,
    db: Arc<Database>,
    python_manager: Arc<PythonManager>,
    state: Mutex<EngineInner>,
    cancel_health: AtomicBool,
    client: Client,
}

struct EngineInner {
    child: Option<Child>,
    running: bool,
    last_error: Option<String>,
}

impl EmbeddingEngine {
    pub fn new(
        app_data_dir: PathBuf,
        db: Arc<Database>,
        python_manager: Arc<PythonManager>,
    ) -> Self {
        // Initialize the cache table in app database dynamically
        let _ = db.with_connection(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS embedding_cache (
                    content_hash TEXT PRIMARY KEY,
                    vector BLOB NOT NULL
                );",
                [],
            )?;
            Ok(())
        });

        Self {
            app_data_dir,
            db,
            python_manager,
            state: Mutex::new(EngineInner {
                child: None,
                running: false,
                last_error: None,
            }),
            cancel_health: AtomicBool::new(false),
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client"),
        }
    }

    pub fn start(&self) -> AppResult<()> {
        let mut inner = self.state.lock().unwrap();
        if inner.running {
            return Ok(());
        }

        let python_exe = self.python_manager.python_exe()?;
        let script_path = self.python_manager.resolve_script("ai_server.py")?;

        let chroma_dir = self.app_data_dir.join("data").join("chroma");
        std::fs::create_dir_all(&chroma_dir).ok();

        tracing::info!(
            "Starting Python AI sidecar server on port {}...",
            AI_SERVER_PORT
        );

        let child = Command::new(&python_exe)
            .arg(&script_path)
            .arg("--host")
            .arg(AI_SERVER_HOST)
            .arg("--port")
            .arg(AI_SERVER_PORT.to_string())
            .arg("--chroma-path")
            .arg(chroma_dir.to_str().unwrap())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| AppError::Other(format!("Failed to start AI sidecar process: {err}")))?;

        inner.child = Some(child);
        inner.running = true;
        inner.last_error = None;

        // Wait until healthy
        let mut healthy = false;
        let health_url = format!("http://{}:{}/health", AI_SERVER_HOST, AI_SERVER_PORT);
        for _ in 0..120 {
            // 60 seconds (120 * 500ms)
            if self
                .client
                .get(&health_url)
                .send()
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                healthy = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        if !healthy {
            if let Some(mut c) = inner.child.take() {
                let _ = c.kill();
            }
            inner.running = false;
            return Err(AppError::Other(
                "AI sidecar health check failed to respond in time".into(),
            ));
        }

        tracing::info!("AI sidecar server is healthy and running.");
        Ok(())
    }

    pub fn stop(&self) -> AppResult<()> {
        self.cancel_health.store(true, Ordering::SeqCst);
        let mut inner = self.state.lock().unwrap();
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        inner.running = false;
        Ok(())
    }

    pub fn embed_batch(&self, texts: &Vec<String>) -> AppResult<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let mut results = vec![None; texts.len()];
        let mut misses = Vec::new();

        // 1. Check cache
        self.db.with_connection(|conn| {
            for (idx, text) in texts.iter().enumerate() {
                let hash = compute_hash(text);
                let mut stmt =
                    conn.prepare("SELECT vector FROM embedding_cache WHERE content_hash = ?1")?;
                let mut rows = stmt.query(rusqlite::params![hash])?;
                if let Some(row) = rows.next()? {
                    let blob: Vec<u8> = row.get(0)?;
                    if let Ok(vec) = serde_json::from_slice::<Vec<f32>>(&blob) {
                        results[idx] = Some(vec);
                        continue;
                    }
                }
                misses.push((idx, text.clone()));
            }
            Ok(())
        })?;

        if misses.is_empty() {
            return Ok(results.into_iter().map(Option::unwrap).collect());
        }

        // 2. Fetch misses from sidecar API
        let miss_texts: Vec<String> = misses.iter().map(|(_, text)| text.clone()).collect();

        #[derive(serde::Serialize)]
        struct EmbedReq {
            texts: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct EmbedResp {
            embeddings: Vec<Vec<f32>>,
        }

        let url = format!("http://{}:{}/embed", AI_SERVER_HOST, AI_SERVER_PORT);
        let resp = self
            .client
            .post(&url)
            .json(&EmbedReq { texts: miss_texts })
            .send()
            .map_err(|e| {
                AppError::Other(format!("Failed to connect to embedding sidecar: {}", e))
            })?;

        if !resp.status().is_success() {
            let err_msg = resp.text().unwrap_or_default();
            return Err(AppError::Other(format!(
                "Embedding sidecar returned error: {}",
                err_msg
            )));
        }

        let embed_resp: EmbedResp = resp
            .json()
            .map_err(|e| AppError::Other(format!("Failed to parse embedding response: {}", e)))?;

        if embed_resp.embeddings.len() != misses.len() {
            return Err(AppError::Other(
                "Embedding sidecar returned mismatched number of embeddings".into(),
            ));
        }

        // 3. Update cache and merge results
        self.db.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "INSERT OR REPLACE INTO embedding_cache (content_hash, vector) VALUES (?1, ?2)",
            )?;
            for (i, (original_idx, text)) in misses.iter().enumerate() {
                let embedding = &embed_resp.embeddings[i];
                let hash = compute_hash(text);
                let blob = serde_json::to_vec(embedding).unwrap();
                stmt.execute(rusqlite::params![hash, blob])?;
                results[*original_idx] = Some(embedding.clone());
            }
            Ok(())
        })?;

        Ok(results.into_iter().map(Option::unwrap).collect())
    }

    pub fn rerank(&self, query: &str, candidates: &Vec<String>) -> AppResult<Vec<f32>> {
        if candidates.is_empty() {
            return Ok(vec![]);
        }

        #[derive(serde::Serialize)]
        struct RerankReq<'a> {
            query: &'a str,
            candidates: &'a Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct RerankResp {
            scores: Vec<f32>,
        }

        let url = format!("http://{}:{}/rerank", AI_SERVER_HOST, AI_SERVER_PORT);
        let resp = self
            .client
            .post(&url)
            .json(&RerankReq { query, candidates })
            .send()
            .map_err(|e| AppError::Other(format!("Failed to connect to rerank sidecar: {}", e)))?;

        if !resp.status().is_success() {
            let err_msg = resp.text().unwrap_or_default();
            return Err(AppError::Other(format!(
                "Rerank sidecar returned error: {}",
                err_msg
            )));
        }

        let rerank_resp: RerankResp = resp
            .json()
            .map_err(|e| AppError::Other(format!("Failed to parse rerank response: {}", e)))?;

        Ok(rerank_resp.scores)
    }
}

fn compute_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}
