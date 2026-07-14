use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::db::message_repo::{self, Message, NewMessage};
use crate::db::session_repo::{self, Session};
use crate::db::settings_repo;
use crate::db::Database;
use crate::llm::inference;
use crate::llm::types::{ChatMessage, StreamChatRequest};
use crate::llm::LlmRuntime;
use crate::python::manager::PythonManager;
use crate::vector_store::VectorStore;
use crate::db::chunk_repo::{self, DocumentChunk};
use crate::db::document_repo;
use crate::utils::error::AppError;

const DEFAULT_SYSTEM_PROMPT: &str = include_str!("../../../models/prompts/chat.system.txt");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResult {
    pub user_message: Message,
    pub assistant_message: Message,
}

#[tauri::command]
pub fn create_session(
    database: State<'_, Arc<Database>>,
    name: String,
    model_id: String,
) -> Result<Session, String> {
    let model_id = if model_id.trim().is_empty() {
        "local-default"
    } else {
        model_id.trim()
    };

    database
        .with_connection(|conn| session_repo::create(conn, &name, model_id))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn list_sessions(database: State<'_, Arc<Database>>) -> Result<Vec<Session>, String> {
    database
        .with_connection(session_repo::list)
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_session(
    database: State<'_, Arc<Database>>,
    session_id: String,
) -> Result<Option<Session>, String> {
    database
        .with_connection(|conn| session_repo::get(conn, &session_id))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn delete_session(
    database: State<'_, Arc<Database>>,
    session_id: String,
) -> Result<(), String> {
    database
        .with_connection(|conn| session_repo::delete(conn, &session_id))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn update_session_system_prompt(
    database: State<'_, Arc<Database>>,
    session_id: String,
    prompt: Option<String>,
) -> Result<(), String> {
    database
        .with_connection(|conn| session_repo::update_system_prompt(conn, &session_id, prompt.as_deref()))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn update_session_cloud_provider(
    database: State<'_, Arc<Database>>,
    session_id: String,
    provider: Option<String>,
) -> Result<(), String> {
    database
        .with_connection(|conn| {
            conn.execute(
                "UPDATE sessions SET cloud_provider = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![provider, session_id],
            )?;
            Ok::<(), crate::utils::error::AppError>(())
        })
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_messages(
    database: State<'_, Arc<Database>>,
    session_id: String,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<Message>, String> {
    database
        .with_connection(|conn| message_repo::list_for_session(conn, &session_id, limit, offset))
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn send_message(
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    python_manager: State<'_, Arc<PythonManager>>,
    vector_store: State<'_, Arc<VectorStore>>,
    session_id: String,
    content: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    on_token: Channel<String>,
) -> Result<SendMessageResult, String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    runtime.ensure_running().map_err(|err| err.to_string())?;

    let user_message = database
        .with_connection(|conn| {
            let session = session_repo::get(conn, &session_id)?
                .ok_or_else(|| AppError::Other(format!("Session not found: {session_id}")))?;
            let user_message = message_repo::create(
                conn,
                NewMessage {
                    session_id: &session_id,
                    role: "user",
                    content: &content,
                    source_type: None,
                    sources: None,
                    tokens_used: None,
                    latency_ms: None,
                    metadata: None,
                },
            )?;

            if session.message_count == 0 && session.name == "New chat" {
                let title = title_from_message(&content);
                let _ = session_repo::rename(conn, &session_id, &title);
            } else {
                session_repo::touch(conn, &session_id)?;
            }

            Ok(user_message)
        })
        .map_err(|err| err.to_string())?;

    let mut rag_chunks = Vec::new();
    let mut source_names = std::collections::HashSet::new();

    // Trigger RAG search
    if let Ok(embeddings) = python_manager.embed_chunks(vec![content.clone()]) {
        if let Some(query_emb) = embeddings.into_iter().next() {
            if let Ok(hits) = vector_store.search(&query_emb, 5) {
                let embedding_ids: Vec<i64> = hits.into_iter().map(|(id, _)| id).collect();
                if !embedding_ids.is_empty() {
                    let _ = database.with_connection(|conn| {
                        if let Ok(chunks) = chunk_repo::get_chunks_by_embedding_ids(conn, &embedding_ids) {
                            for chunk in chunks {
                                if let Ok(Some(doc)) = document_repo::get(conn, &chunk.document_id) {
                                    if doc.session_id.is_none() || doc.session_id.as_deref() == Some(&session_id) {
                                        rag_chunks.push(chunk);
                                        source_names.insert(doc.filename);
                                    }
                                }
                            }
                        }
                        Ok::<(), AppError>(())
                    });
                }
            }
        }
    }

    let request_messages = database
        .with_connection(|conn| build_context_messages(conn, &session_id, &rag_chunks))
        .map_err(|err| err.to_string())?;

    let sources_json = if source_names.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&source_names.into_iter().collect::<Vec<_>>()).unwrap_or_default())
    };

    let assistant_message_db = database
        .with_connection(|conn| {
            message_repo::create(
                conn,
                NewMessage {
                    session_id: &session_id,
                    role: "assistant",
                    content: "",
                    source_type: Some("local"),
                    sources: sources_json.as_deref(),
                    tokens_used: None,
                    latency_ms: None,
                    metadata: None,
                },
            )
        })
        .map_err(|err| err.to_string())?;

    let started = Instant::now();
    let mut chunk_count = 0;
    
    let assistant_content = inference::stream_chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        StreamChatRequest {
            messages: request_messages,
            max_tokens,
            temperature,
        },
        on_token,
        Some(|partial: &str| {
            chunk_count += 1;
            // Persist roughly every 20 chunks (~5-10 words)
            if chunk_count % 20 == 0 {
                let _ = database.with_connection(|conn| {
                    message_repo::update_content(conn, &assistant_message_db.id, partial)
                });
            }
        }),
        Some(&runtime.cancel_inference),
    )
    .await
    .map_err(|err| err.to_string())?;

    let latency_ms = started.elapsed().as_millis() as u64;
    let tokens_used = assistant_content.split_whitespace().count() as u32;

    let assistant_message = database
        .with_connection(|conn| {
            message_repo::update_content(conn, &assistant_message_db.id, &assistant_content)?;
            message_repo::update_metadata(
                conn, 
                &assistant_message_db.id, 
                Some(tokens_used), 
                Some(latency_ms), 
                None
            )?;
            session_repo::touch(conn, &session_id)?;
            message_repo::get(conn, &assistant_message_db.id)
                .map(|msg| msg.unwrap_or(assistant_message_db.clone()))
        })
        .map_err(|err| err.to_string())?;

    Ok(SendMessageResult {
        user_message,
        assistant_message,
    })
}

fn build_context_messages(
    conn: &rusqlite::Connection,
    session_id: &str,
    rag_chunks: &[DocumentChunk],
) -> Result<Vec<ChatMessage>, AppError> {
    let session = session_repo::get(conn, session_id)?
        .ok_or_else(|| AppError::Other("Session not found for context".to_string()))?;

    // 1. Resolve System Prompt
    let mut system_prompt = session.system_prompt.clone().unwrap_or_default();
    if system_prompt.trim().is_empty() {
        system_prompt = settings_repo::get(conn, "default_system_prompt")?
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string());
    }

    if !rag_chunks.is_empty() {
        system_prompt.push_str("\n\nHere is some context retrieved from the user's documents that might be relevant to their latest query:\n");
        for chunk in rag_chunks {
            system_prompt.push_str(&format!("\n---\n{}\n---", chunk.content));
        }
    }

    // 2. Fetch more messages to allow for truncation
    let recent = message_repo::recent_for_context(conn, session_id, 200)?;

    // 3. Truncation Strategy
    // We assume 1 char ~= 0.25 tokens. Target max context ~ 6000 tokens for messages.
    let target_max_tokens = 6000;
    let mut accumulated_tokens = (system_prompt.chars().count() / 4) as u32;

    let mut context = Vec::new();
    
    // Iterate from newest to oldest
    for message in recent.iter().rev() {
        if !matches!(message.role.as_str(), "user" | "assistant" | "system") {
            continue;
        }

        let msg_tokens = (message.content.chars().count() / 4) as u32;
        if accumulated_tokens + msg_tokens > target_max_tokens {
            break; // Context window full, drop older messages
        }

        accumulated_tokens += msg_tokens;
        context.push(ChatMessage {
            role: message.role.clone(),
            content: message.content.clone(),
        });
    }

    // Reverse back to chronological order (oldest first)
    context.reverse();

    // 4. Assemble final messages list
    let mut messages = Vec::with_capacity(context.len() + 1);

    if !context.iter().any(|message| message.role == "system") {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system_prompt.trim().to_string(),
        });
    }

    messages.extend(context);

    Ok(messages)
}

fn title_from_message(content: &str) -> String {
    let mut title: String = content
        .split_whitespace()
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");

    if title.chars().count() > 48 {
        title = title.chars().take(48).collect();
    }

    if title.is_empty() {
        "New chat".to_string()
    } else {
        title
    }
}

#[tauri::command]
pub fn cancel_chat_stream(runtime: State<'_, Arc<LlmRuntime>>) {
    runtime.cancel_inference();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_from_message() {
        assert_eq!(title_from_message(""), "New chat");
        assert_eq!(
            title_from_message("Hello world how are you doing today my friend?"),
            "Hello world how are you doing today my"
        );
        let long_msg = "This is a very long message that definitely exceeds forty eight characters in length to see what happens";
        let title = title_from_message(long_msg);
        assert_eq!(title.len(), 43);
        assert!(title.starts_with("This is a very long message that"));
    }

    #[test]
    fn test_chat_flow_persistence() {
        use crate::db::Database;
        use crate::utils::error::AppError;
        use tempfile::tempdir;

        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        db.run_migrations().unwrap();
        
        let session_id = db.with_connection(|conn| {
            let session = crate::db::session_repo::create(
                conn, 
                "Test Session",
                "test-model",
            ).unwrap();
            Ok::<String, AppError>(session.id)
        }).unwrap();

        db.with_connection(|conn| {
            let user_msg = crate::db::message_repo::create(conn, crate::db::message_repo::NewMessage {
                session_id: &session_id,
                role: "user",
                content: "Hello",
                source_type: None,
                sources: None,
                tokens_used: None,
                latency_ms: None,
                metadata: None,
            }).unwrap();

            let assistant_msg = crate::db::message_repo::create(conn, crate::db::message_repo::NewMessage {
                session_id: &session_id,
                role: "assistant",
                content: "Hi there!",
                source_type: Some("local"),
                sources: None,
                tokens_used: Some(10),
                latency_ms: Some(100),
                metadata: None,
            }).unwrap();

            assert_eq!(user_msg.content, "Hello");
            assert_eq!(assistant_msg.content, "Hi there!");

            let messages = crate::db::message_repo::list_for_session(conn, &session_id, None, None).unwrap();
            assert_eq!(messages.len(), 2);
            Ok::<(), AppError>(())
        }).unwrap();
    }
}
