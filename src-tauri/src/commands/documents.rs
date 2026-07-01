use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::db::chunk_repo::{self, DocumentChunk, NewChunk};
use crate::db::document_repo::{self, Document, NewDocument};
use crate::db::Database;
use crate::embedding::EmbeddingEngine;
use crate::python::manager::PythonManager;
use crate::vector_store::VectorStore;

use crate::llm::inference;
use crate::llm::types::{ChatMessage, StreamChatRequest};
use crate::llm::LlmRuntime;
use crate::utils::error::AppError;
use crate::utils::error::AppResult;
use tauri::ipc::Channel;

#[tauri::command]
pub async fn upload_document(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    python_manager: State<'_, Arc<PythonManager>>,
    embedding_engine: State<'_, Arc<EmbeddingEngine>>,
    vector_store: State<'_, Arc<VectorStore>>,
    file_path: String,
    session_id: Option<String>,
) -> Result<Document, String> {
    let source_path = PathBuf::from(&file_path);
    if !source_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    let file_size = fs::metadata(&source_path).map(|m| m.len()).unwrap_or(0);

    if file_size == 0 {
        return Err(format!("File is empty: {}", file_path));
    }

    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let doc_id = Uuid::new_v4().to_string();

    // Copy file to app data dir
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let docs_dir = app_data_dir.join("documents");
    fs::create_dir_all(&docs_dir).map_err(|e| e.to_string())?;

    let dest_path = docs_dir.join(format!("{}_{}", doc_id, file_name));
    fs::copy(&source_path, &dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    // Create DB record
    let new_doc = NewDocument {
        id: &doc_id,
        session_id: session_id.as_deref(),
        filename: &file_name,
        file_path: dest_path.to_str().unwrap_or(""),
        file_size,
        file_type: &ext,
    };

    let doc = database
        .with_connection(|conn| document_repo::create(conn, new_doc))
        .map_err(|e| e.to_string())?;

    // Spawn async task — delegate all parsing/chunking to the service layer.
    let db = database.inner().clone();
    let pm = python_manager.inner().clone();
    let ee = embedding_engine.inner().clone();
    let vs = vector_store.inner().clone();
    let file_type_clone = ext.clone();

    tauri::async_runtime::spawn(async move {
        match crate::services::document::ingest(
            &doc_id,
            &dest_path,
            &file_type_clone,
            &db,
            &pm,
            &ee,
            &vs,
            None, // use default ChunkingConfig
        )
        .await
        {
            Ok(chunk_count) => {
                tracing::info!(
                    document_id = %doc_id,
                    chunk_count,
                    "Document ingestion finished"
                );
            }
            Err(err) => {
                tracing::error!(document_id = %doc_id, error = %err, "Document ingestion failed");
            }
        }
    });

    Ok(doc)
}

#[tauri::command]
pub async fn list_documents(
    database: State<'_, Arc<Database>>,
    session_id: Option<String>,
) -> Result<Vec<Document>, String> {
    database
        .with_connection(|conn| document_repo::list(conn, session_id.as_deref()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_document(
    database: State<'_, Arc<Database>>,
    python_manager: State<'_, Arc<PythonManager>>,
    vector_store: State<'_, Arc<VectorStore>>,
    document_id: String,
    query: String,
) -> Result<Vec<DocumentChunk>, String> {
    let embeddings = python_manager
        .embed_chunks(vec![query.clone()])
        .map_err(|e| e.to_string())?;
    let query_emb = embeddings.into_iter().next().unwrap_or_default();

    let filter = crate::vector_store::types::SearchFilter {
        document_id: Some(document_id.clone()),
        source_type: Some("document".to_string()),
        min_page: None,
        max_page: None,
    };

    let hits = vector_store
        .search(&query, query_emb, 5, filter)
        .await
        .map_err(|e| e.to_string())?;
    let chunk_ids: Vec<String> = hits.into_iter().map(|(id, _)| id).collect();

    if chunk_ids.is_empty() {
        return Ok(vec![]);
    }

    let chunks = database
        .with_connection(|conn| chunk_repo::get_chunks_by_ids(conn, &chunk_ids))
        .map_err(|e| e.to_string())?;

    // Sort chunks in the exact order returned by the search (RRF order)
    let mut ordered_chunks = Vec::with_capacity(chunks.len());
    for id in chunk_ids {
        if let Some(chunk) = chunks.iter().find(|c| c.id == id) {
            ordered_chunks.push(chunk.clone());
        }
    }

    Ok(ordered_chunks)
}

#[tauri::command]
pub async fn summarize_document(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    document_id: String,
    on_token: Channel<String>,
) -> Result<String, String> {
    runtime.ensure_running().map_err(|err| err.to_string())?;

    let doc_text = database
        .with_connection(|conn| {
            let chunks = chunk_repo::get_for_document(conn, &document_id)?;
            let mut full_text = String::new();
            // Limit to approx first 50 chunks (usually covers intro/executive summary well)
            for chunk in chunks.into_iter().take(50) {
                full_text.push_str(&chunk.content);
                full_text.push_str("\n\n");
            }
            Ok::<String, AppError>(full_text)
        })
        .map_err(|err| err.to_string())?;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are an expert AI summarizer. Provide a concise, highly accurate, and well-structured summary of the following document excerpt. Highlight the main topic and key takeaways.".to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Please summarize this document:\n\n{}", doc_text),
        }
    ];

    let summary = inference::stream_chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        StreamChatRequest {
            messages,
            max_tokens: Some(1024),
            temperature: Some(0.3),
        },
        on_token,
        None::<fn(&str)>,
        Some(&runtime.cancel_inference),
    )
    .await
    .map_err(|err| err.to_string())?;

    Ok(summary)
}

#[tauri::command]
pub fn get_document_chunks(
    database: State<'_, Arc<Database>>,
    document_id: String,
) -> Result<Vec<DocumentChunk>, String> {
    database
        .with_connection(|conn| chunk_repo::get_for_document(conn, &document_id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_document(
    database: State<'_, Arc<Database>>,
    vector_store: State<'_, Arc<VectorStore>>,
    document_id: String,
) -> Result<(), String> {
    let doc = database
        .with_connection(|conn| document_repo::get(conn, &document_id))
        .map_err(|e| e.to_string())?
        .ok_or("Document not found")?;

    // Cascade delete in Chroma
    vector_store
        .remove_document(&document_id)
        .await
        .map_err(|e| e.to_string())?;

    database
        .with_connection(|conn| document_repo::delete(conn, &document_id))
        .map_err(|e| e.to_string())?;

    // Clean up file
    let _ = fs::remove_file(&doc.file_path);

    Ok(())
}

#[tauri::command]
pub async fn rebuild_vector_store(
    database: State<'_, Arc<Database>>,
    embedding_engine: State<'_, Arc<EmbeddingEngine>>,
    vector_store: State<'_, Arc<VectorStore>>,
) -> Result<(), String> {
    let docs = database
        .with_connection(|conn| document_repo::list(conn, None))
        .map_err(|e| e.to_string())?;

    vector_store.clear().await.map_err(|e| e.to_string())?;

    for doc in docs {
        let chunks = database
            .with_connection(|conn| chunk_repo::get_for_document(conn, &doc.id))
            .map_err(|e| e.to_string())?;

        if chunks.is_empty() {
            continue;
        }

        let chunk_texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let chunk_ids: Vec<String> = chunks.iter().map(|c| c.id.clone()).collect();

        tracing::info!(
            "Re-embedding {} chunks for document {}",
            chunks.len(),
            doc.id
        );
        let embeddings = embedding_engine
            .embed_batch(&chunk_texts)
            .map_err(|e| e.to_string())?;

        let mut indexable_chunks = Vec::with_capacity(chunks.len());
        for (i, emb) in embeddings.into_iter().enumerate() {
            let chunk = &chunks[i];
            indexable_chunks.push(crate::vector_store::types::IndexableChunk {
                id: chunk.id.clone(),
                embedding: emb,
                document_id: doc.id.clone(),
                chunk_index: chunk.chunk_index,
                page_number: chunk.page_number,
                section_heading: chunk.section_heading.clone(),
                heading_path: None,
                source_type: "document".to_string(),
                token_count: chunk.token_count,
                content: chunk.content.clone(),
            });
        }

        vector_store
            .add_chunks(indexable_chunks)
            .await
            .map_err(|e| e.to_string())?;

        database
            .with_connection(|conn| {
                chunk_repo::update_indexing_status(conn, &chunk_ids, true)?;
                document_repo::update_indexed_status(conn, &doc.id)
            })
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontEndSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

#[tauri::command]
pub async fn get_media_transcript(
    database: State<'_, Arc<Database>>,
    document_id: String,
) -> Result<Vec<FrontEndSegment>, String> {
    let chunks = database
        .with_connection(|conn| chunk_repo::get_for_document(conn, &document_id))
        .map_err(|e| e.to_string())?;

    let mut segments = Vec::new();
    for chunk in chunks {
        if let Some(meta_str) = chunk.metadata {
            if let Ok(meta_json) = serde_json::from_str::<serde_json::Value>(&meta_str) {
                let start = meta_json["startTime"].as_f64().unwrap_or(0.0);
                let end = meta_json["endTime"].as_f64().unwrap_or(0.0);
                segments.push(FrontEndSegment {
                    start,
                    end,
                    text: chunk.content,
                });
            }
        }
    }
    Ok(segments)
}

#[tauri::command]
pub async fn generate_meeting_summary(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    document_id: String,
    detail_level: String,
    on_token: Channel<String>,
) -> Result<String, String> {
    runtime.ensure_running().map_err(|err| err.to_string())?;

    let doc_text = database
        .with_connection(|conn| {
            let chunks = chunk_repo::get_for_document(conn, &document_id)?;
            let mut full_text = String::new();
            for chunk in chunks.into_iter().take(50) {
                full_text.push_str(&chunk.content);
                full_text.push_str("\n\n");
            }
            Ok::<String, AppError>(full_text)
        })
        .map_err(|err| err.to_string())?;

    let prompt = format!(
        "You are an expert AI secretary. Analyze the following meeting transcript and generate a structured summary.
        Include:
        - Main topics discussed
        - Key decisions made
        - Action items with owners if mentioned
        
        Make the summary {} in detail.",
        detail_level
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("Please summarize this meeting transcript:\n\n{}", doc_text),
        },
    ];

    let summary = inference::stream_chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        StreamChatRequest {
            messages,
            max_tokens: Some(1500),
            temperature: Some(0.3),
        },
        on_token,
        None::<fn(&str)>,
        Some(&runtime.cancel_inference),
    )
    .await
    .map_err(|err| err.to_string())?;

    Ok(summary)
}
