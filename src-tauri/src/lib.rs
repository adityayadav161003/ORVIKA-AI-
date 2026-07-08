use std::sync::Arc;

use tauri::Manager;

mod commands;
mod config;
mod db;
mod llm;
mod python;
mod security;
mod utils;

// Module stubs for future sprints
#[allow(dead_code)]
#[allow(dead_code)]
mod cloud;
#[allow(dead_code)]
mod document;
#[allow(dead_code)]
mod embedding;
mod media;
#[allow(dead_code)]
mod services;
pub mod vector_store;

use db::model_repo;
use db::Database;
use llm::model_manager::ModelDownloadState;
use llm::LlmRuntime;
use python::manager::PythonManager;
use security::Aes256GcmCipher;
use utils::logger;
use vector_store::VectorStore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(app_data_dir.join("data"))?;

            let db_path = app_data_dir.join("data").join("app.db");
            tracing::info!(path = %db_path.display(), "Opening database");

            let database = Arc::new(Database::open(&db_path)?);
            database.run_migrations()?;

            let sync_service = Arc::new(services::sync::SyncService::new(database.clone()));
            sync_service.start_background_sync();

            let runtime = Arc::new(LlmRuntime::new(app_data_dir.clone()));
            if let Ok(Some(active)) = database.with_connection(model_repo::active_model) {
                runtime.set_model_path(Some(active.model_path));
            }

            let cipher = Arc::new(Aes256GcmCipher::from_machine_key());
            let python_manager = Arc::new(PythonManager::new(app_data_dir.clone()));

            let embedding_engine = Arc::new(crate::embedding::EmbeddingEngine::new(
                app_data_dir.clone(),
                database.clone(),
                python_manager.clone(),
            ));
            embedding_engine.start()?;

            let vector_store = Arc::new(VectorStore::new(&app_data_dir, database.clone())?);

            app.manage(database);
            app.manage(runtime);
            app.manage(Arc::new(ModelDownloadState::new()));
            app.manage(cipher);
            app.manage(python_manager);
            app.manage(embedding_engine);
            app.manage(vector_store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // DB status
            commands::get_db_status,
            // Chat
            commands::create_session,
            commands::list_sessions,
            commands::get_session,
            commands::delete_session,
            commands::update_session_system_prompt,
            commands::update_session_cloud_provider,
            commands::get_messages,
            commands::send_message,
            commands::cancel_chat_stream,
            // LLM runtime
            commands::get_llm_status,
            commands::get_hardware_info,
            commands::list_registry_models,
            commands::list_downloaded_models,
            commands::start_llm_server,
            commands::stop_llm_server,
            commands::restart_llm_server,
            commands::set_active_model,
            commands::download_model,
            commands::cancel_model_download,
            commands::register_model_path,
            commands::import_custom_gguf,
            commands::run_llm_benchmark,
            commands::stream_chat_completion,
            commands::delete_model,
            // Settings
            commands::get_setting,
            commands::set_setting,
            commands::get_all_settings,
            // API keys (encrypted)
            commands::store_api_key,
            commands::get_api_key,
            commands::delete_api_key,
            commands::list_api_key_providers,
            commands::validate_api_key_format,
            // Documents
            commands::upload_document,
            commands::list_documents,
            commands::get_document_chunks,
            commands::delete_document,
            commands::search_document,
            commands::rebuild_vector_store,
            commands::summarize_document,
            commands::get_media_transcript,
            commands::generate_meeting_summary,
            // Research
            commands::generate_research_plan,
            commands::approve_research_plan,
            commands::execute_research,
            commands::list_research_sessions,
            commands::get_research_session_details,
            commands::delete_research_session,
            // Security & Compliance
            commands::list_audit_logs,
            commands::clear_audit_logs,
            commands::get_audit_stats,
            commands::reset_api_spending,
            commands::generate_compliance_report,
            commands::get_local_telemetry,
            commands::create_compliance_template,
            commands::get_compliance_template,
            commands::list_compliance_templates,
            commands::delete_compliance_template,
            commands::validate_sso_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
