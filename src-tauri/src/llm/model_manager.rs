use std::fs::{self, File};
use std::io::Write;

use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use crate::db::model_repo;
use crate::db::Database;
use crate::llm::config::{self, resolve_registry_path};
use crate::llm::types::{DownloadProgress, DownloadedModel, RegistryModel};
use crate::utils::error::{AppError, AppResult};
use crate::utils::hash::verify_file_checksum;

#[derive(Debug, Deserialize)]
struct RegistryFile {
    models: Vec<RegistryModel>,
}

pub struct ModelDownloadState {
    pub cancel: AtomicBool,
    pub in_progress: AtomicBool,
}

impl ModelDownloadState {
    pub fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            in_progress: AtomicBool::new(false),
        }
    }
}

pub fn load_registry() -> AppResult<Vec<RegistryModel>> {
    let path = resolve_registry_path();
    let bytes = fs::read(&path).map_err(|err| {
        AppError::Config(format!(
            "Failed to read registry at {}: {err}",
            path.display()
        ))
    })?;
    let parsed: RegistryFile = serde_json::from_slice(&bytes)
        .map_err(|err| AppError::Config(format!("Invalid registry.json: {err}")))?;
    Ok(parsed.models)
}

pub fn registry_model(id: &str) -> AppResult<RegistryModel> {
    load_registry()?
        .into_iter()
        .find(|m| m.id == id)
        .ok_or_else(|| AppError::Config(format!("Unknown model id: {id}")))
}

pub async fn download_model(
    app: AppHandle,
    database: Arc<Database>,
    app_data_dir: PathBuf,
    download_state: Arc<ModelDownloadState>,
    model_id: String,
) -> AppResult<DownloadedModel> {
    if download_state
        .in_progress
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(AppError::Other(
            "Another download is already in progress".into(),
        ));
    }

    download_state.cancel.store(false, Ordering::SeqCst);

    let result = download_model_inner(
        app,
        database,
        app_data_dir,
        download_state.clone(),
        model_id,
    )
    .await;

    download_state.in_progress.store(false, Ordering::SeqCst);
    result
}

async fn download_model_inner(
    app: AppHandle,
    database: Arc<Database>,
    app_data_dir: PathBuf,
    download_state: Arc<ModelDownloadState>,
    model_id: String,
) -> AppResult<DownloadedModel> {
    let model = registry_model(&model_id)?;
    if model.url.trim().is_empty() {
        return Err(AppError::Config(format!(
            "Model {} has no download URL configured",
            model.id
        )));
    }

    let models_dir = config::models_dir(&app_data_dir);
    fs::create_dir_all(&models_dir)?;
    let dest = models_dir.join(&model.filename);
    let partial = models_dir.join(format!("{}.part", model.filename));

    emit_progress(&app, &model_id, 0, model.size_bytes, "downloading");

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|err| AppError::Other(err.to_string()))?;

    let response = client
        .get(&model.url)
        .send()
        .await
        .map_err(|err| AppError::Other(format!("Download failed: {err}")))?;

    if !response.status().is_success() {
        return Err(AppError::Other(format!(
            "Download HTTP {}",
            response.status()
        )));
    }

    let total = response.content_length().unwrap_or(model.size_bytes);
    let mut file = File::create(&partial)?;
    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if download_state.cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = fs::remove_file(&partial);
            return Err(AppError::Other("Download cancelled".into()));
        }

        let chunk =
            chunk.map_err(|err| AppError::Other(format!("Download stream error: {err}")))?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        emit_progress(&app, &model_id, downloaded, total, "downloading");
    }

    file.flush()?;
    drop(file);

    fs::rename(&partial, &dest)?;

    emit_progress(&app, &model_id, downloaded, total, "verifying");
    verify_file_checksum(&dest, &model.checksum_sha256)?;

    let file_size = fs::metadata(&dest)?.len() as i64;
    let downloaded_model = DownloadedModel {
        id: model.id.clone(),
        model_name: model.name.clone(),
        model_path: dest.display().to_string(),
        file_size,
        checksum_sha256: model.checksum_sha256.clone(),
        quantization: model.quantization.clone(),
        is_active: false,
    };

    database.with_connection(|conn| model_repo::upsert_download(conn, &downloaded_model))?;

    emit_progress(&app, &model_id, downloaded, total, "complete");

    Ok(downloaded_model)
}

pub fn register_local_model(database: &Database, model: DownloadedModel) -> AppResult<()> {
    database.with_connection(|conn| model_repo::upsert_download(conn, &model))
}

pub fn import_model_file(
    database: &Database,
    app_data_dir: &Path,
    source: &Path,
    registry_id: &str,
) -> AppResult<DownloadedModel> {
    let registry = registry_model(registry_id)?;
    let models_dir = config::models_dir(app_data_dir);
    fs::create_dir_all(&models_dir)?;
    let dest = models_dir.join(&registry.filename);

    if source != dest.as_path() {
        if dest.exists() {
            fs::remove_file(&dest)?;
        }
        fs::copy(source, &dest)?;
    }

    verify_file_checksum(&dest, &registry.checksum_sha256)?;

    let file_size = fs::metadata(&dest)?.len() as i64;
    let model = DownloadedModel {
        id: registry.id,
        model_name: registry.name,
        model_path: dest.display().to_string(),
        file_size,
        checksum_sha256: registry.checksum_sha256,
        quantization: registry.quantization,
        is_active: false,
    };

    database.with_connection(|conn| model_repo::upsert_download(conn, &model))?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_models() {
        let models = load_registry().expect("registry");
        assert!(!models.is_empty());
        assert!(models
            .iter()
            .all(|m| !m.id.is_empty() && !m.filename.is_empty()));
    }
}

fn emit_progress(app: &AppHandle, model_id: &str, downloaded: u64, total: u64, phase: &str) {
    let percent = if total > 0 {
        (downloaded as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    let _ = app.emit(
        "model-download-progress",
        DownloadProgress {
            model_id: model_id.to_string(),
            downloaded_bytes: downloaded,
            total_bytes: total,
            percent,
            phase: phase.to_string(),
        },
    );
}
