use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::fs;
use serde::{Serialize, Deserialize};

use crate::utils::error::{AppError, AppResult};

#[derive(Serialize, Deserialize)]
struct StoreData {
    dim: u32,
    vectors: Vec<VectorItem>,
    next_id: i64,
}

#[derive(Serialize, Deserialize)]
struct VectorItem {
    id: i64,
    embedding: Vec<f32>,
}

pub struct VectorStore {
    store_path: PathBuf,
    data: Mutex<StoreData>,
}

impl VectorStore {
    pub fn new(app_data_dir: &Path, dim: u32) -> AppResult<Self> {
        let index_dir = app_data_dir.join("data").join("vectors");
        std::fs::create_dir_all(&index_dir).map_err(|e| AppError::Other(e.to_string()))?;
        
        let store_path = index_dir.join("knowledge_base.json");
        
        let data = if store_path.exists() {
            tracing::info!("Loading vector store from {:?}", store_path);
            let content = fs::read_to_string(&store_path)
                .map_err(|e| AppError::Other(e.to_string()))?;
            serde_json::from_str::<StoreData>(&content)
                .map_err(|e| AppError::Other(e.to_string()))?
        } else {
            tracing::info!("Creating new vector store");
            StoreData {
                dim,
                vectors: Vec::new(),
                next_id: 1,
            }
        };

        if data.dim != dim {
            return Err(AppError::Other(format!(
                "Dimension mismatch: store has {}, requested {}",
                data.dim, dim
            )));
        }

        Ok(Self {
            store_path,
            data: Mutex::new(data),
        })
    }

    pub fn add_vectors(&self, embeddings: Vec<Vec<f32>>) -> AppResult<Vec<i64>> {
        if embeddings.is_empty() {
            return Ok(vec![]);
        }

        let mut data = self.data.lock().unwrap();
        let mut ids = Vec::with_capacity(embeddings.len());
        
        for emb in embeddings {
            if emb.len() != data.dim as usize {
                return Err(AppError::Other(format!(
                    "Invalid vector dimension: expected {}, got {}",
                    data.dim, emb.len()
                )));
            }
            
            // L2 normalize vector for cosine similarity to work seamlessly with inner product
            let norm: f32 = emb.iter().map(|v| v * v).sum::<f32>().sqrt();
            let normalized = if norm > 0.0 {
                emb.into_iter().map(|v| v / norm).collect()
            } else {
                emb
            };

            let id = data.next_id;
            data.next_id += 1;
            
            data.vectors.push(VectorItem {
                id,
                embedding: normalized,
            });
            ids.push(id);
        }

        // Save after insert
        let content = serde_json::to_string(&*data)
            .map_err(|e| AppError::Other(e.to_string()))?;
        fs::write(&self.store_path, content)
            .map_err(|e| AppError::Other(e.to_string()))?;

        Ok(ids)
    }

    pub fn remove_vectors(&self, ids: &[i64]) -> AppResult<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let mut data = self.data.lock().unwrap();
        data.vectors.retain(|v| !ids.contains(&v.id));

        let content = serde_json::to_string(&*data)
            .map_err(|e| AppError::Other(e.to_string()))?;
        fs::write(&self.store_path, content)
            .map_err(|e| AppError::Other(e.to_string()))?;

        Ok(())
    }

    pub fn clear(&self) -> AppResult<()> {
        let mut data = self.data.lock().unwrap();
        data.vectors.clear();
        data.next_id = 1;
        let content = serde_json::to_string(&*data)
            .map_err(|e| AppError::Other(e.to_string()))?;
        fs::write(&self.store_path, content)
            .map_err(|e| AppError::Other(e.to_string()))?;
        Ok(())
    }

    pub fn search(&self, query: &[f32], top_k: usize) -> AppResult<Vec<(i64, f32)>> {
        let data = self.data.lock().unwrap();
        
        if query.len() != data.dim as usize {
            return Err(AppError::Other(format!("Invalid query dimension")));
        }

        if data.vectors.is_empty() {
            return Ok(vec![]);
        }

        // Normalize query
        let norm: f32 = query.iter().map(|v| v * v).sum::<f32>().sqrt();
        let q_norm = if norm > 0.0 {
            query.iter().map(|v| v / norm).collect::<Vec<_>>()
        } else {
            query.to_vec()
        };

        let mut scored_vectors: Vec<(i64, f32)> = data.vectors.iter().map(|item| {
            let similarity: f32 = item.embedding.iter()
                .zip(q_norm.iter())
                .map(|(a, b)| a * b)
                .sum();
            
            // Distance is roughly 1.0 - similarity (since L2 normalized inner product gives cosine similarity)
            let distance = 1.0 - similarity;
            (item.id, distance)
        }).collect();

        // Sort by distance ascending
        scored_vectors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        scored_vectors.truncate(top_k);
        Ok(scored_vectors)
    }
}
