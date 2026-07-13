use std::path::PathBuf;
use std::sync::Arc;

use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::db::model_repo;
use crate::db::Database;
use crate::llm::benchmark;
use crate::llm::hardware::detect_hardware;
use crate::llm::inference;
use crate::llm::model_manager::{self, ModelDownloadState};
use crate::llm::types::{
    BenchmarkReport, DownloadedModel, HardwareInfo, LlmStatus, RegistryModel, StreamChatRequest,
};
use crate::llm::LlmRuntime;

#[tauri::command]
pub fn get_llm_status(runtime: State<'_, Arc<LlmRuntime>>) -> Result<LlmStatus, String> {
    Ok(runtime.status())
}

#[tauri::command]
pub fn get_hardware_info() -> Result<HardwareInfo, String> {
    Ok(detect_hardware())
}

#[derive(serde::Deserialize)]
struct RegistryFileWrapper {
    models: Vec<RegistryModel>,
}

#[tauri::command]
pub async fn list_registry_models(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<RegistryModel>, String> {
    let registry_url = database
        .with_connection(|conn| crate::db::settings_repo::get(conn, "model_registry_url"))
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    if !registry_url.is_empty() && registry_url.starts_with("http") {
        let client = reqwest::Client::new();
        match client.get(&registry_url).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<RegistryFileWrapper>().await {
                        Ok(data) => return Ok(data.models),
                        Err(err) => {
                            tracing::warn!("Failed to parse remote model registry JSON: {}", err);
                        }
                    }
                } else {
                    tracing::warn!("Remote registry returned non-200 status: {}", res.status());
                }
            }
            Err(err) => {
                tracing::warn!("Failed to fetch remote model registry: {}", err);
            }
        }
    }

    model_manager::load_registry().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_downloaded_models(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<DownloadedModel>, String> {
    database
        .with_connection(model_repo::list_downloaded)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_llm_server(
    runtime: State<'_, Arc<LlmRuntime>>,
    app: AppHandle,
) -> Result<LlmStatus, String> {
    runtime.start(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_llm_server(runtime: State<'_, Arc<LlmRuntime>>) -> Result<LlmStatus, String> {
    runtime.stop().map_err(|e| e.to_string())?;
    Ok(runtime.status())
}

#[tauri::command]
pub fn restart_llm_server(
    runtime: State<'_, Arc<LlmRuntime>>,
    app: AppHandle,
) -> Result<LlmStatus, String> {
    runtime.restart(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_active_model(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    model_id: String,
) -> Result<DownloadedModel, String> {
    database
        .with_connection(|conn| model_repo::set_active(conn, &model_id))
        .map_err(|e| e.to_string())?;

    let active = database
        .with_connection(model_repo::active_model)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Active model not found after update".to_string())?;

    runtime.set_model_path(Some(active.model_path.clone()));

    let status = runtime.status();
    if status.state == crate::llm::types::LlmServerState::Running {
        let _ = runtime.restart(&app);
    }

    Ok(active)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    download_state: State<'_, Arc<ModelDownloadState>>,
    model_id: String,
) -> Result<DownloadedModel, String> {
    let app_data = runtime.app_data_dir();
    model_manager::download_model(
        app,
        Arc::clone(&database),
        app_data,
        Arc::clone(&download_state),
        model_id,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_model_download(
    download_state: State<'_, Arc<ModelDownloadState>>,
) -> Result<(), String> {
    download_state
        .cancel
        .store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn register_model_path(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    registry_id: String,
    file_path: String,
) -> Result<DownloadedModel, String> {
    let model = model_manager::import_model_file(
        &database,
        &runtime.app_data_dir(),
        PathBuf::from(file_path).as_path(),
        &registry_id,
    )
    .map_err(|e| e.to_string())?;
    Ok(model)
}

#[tauri::command]
pub async fn run_llm_benchmark(
    runtime: State<'_, Arc<LlmRuntime>>,
) -> Result<BenchmarkReport, String> {
    benchmark::run_benchmark(&runtime)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stream_chat_completion(
    runtime: State<'_, Arc<LlmRuntime>>,
    request: StreamChatRequest,
    on_token: Channel<String>,
) -> Result<(), String> {
    runtime.ensure_running().map_err(|e| e.to_string())?;
    let _ = inference::stream_chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        request,
        on_token,
        None::<fn(&str)>,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_model(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    model_id: String,
) -> Result<(), String> {
    // If the model being deleted is currently active, clear the runtime path
    let status = runtime.status();
    if status.model_path.is_some() {
        if let Ok(Some(active)) = database.with_connection(model_repo::active_model) {
            if active.id == model_id {
                runtime.set_model_path(None);
            }
        }
    }
    database
        .with_connection(|conn| model_repo::delete_model(conn, &model_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_custom_gguf(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    file_path: String,
    model_name: String,
) -> Result<DownloadedModel, String> {
    let source_path = PathBuf::from(&file_path);
    if !source_path.exists() {
        return Err(format!("Source GGUF file not found: {}", file_path));
    }

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "gguf" {
        return Err("Only .gguf files can be imported".to_string());
    }

    let filename = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("model.gguf")
        .to_string();

    let app_data_dir = runtime.app_data_dir();
    let models_dir = crate::llm::config::models_dir(&app_data_dir);
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let dest_path = models_dir.join(&filename);
    if source_path != dest_path {
        if dest_path.exists() {
            std::fs::remove_file(&dest_path).map_err(|e| e.to_string())?;
        }
        std::fs::copy(&source_path, &dest_path)
            .map_err(|e| format!("Failed to copy GGUF file: {}", e))?;
    }

    let file_size = std::fs::metadata(&dest_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let custom_id = format!("custom-{}", uuid::Uuid::new_v4().to_string());
    let model = DownloadedModel {
        id: custom_id,
        model_name: if model_name.trim().is_empty() {
            filename.replace(".gguf", "")
        } else {
            model_name.trim().to_string()
        },
        model_path: dest_path.to_string_lossy().to_string(),
        file_size,
        checksum_sha256: "custom-import".to_string(),
        quantization: "unknown".to_string(),
        is_active: false,
    };

    database
        .with_connection(|conn| model_repo::upsert_download(conn, &model))
        .map_err(|e| e.to_string())?;

    Ok(model)
}
