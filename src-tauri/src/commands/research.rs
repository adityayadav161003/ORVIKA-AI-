use std::sync::Arc;
use tauri::{AppHandle, State, Emitter};
use tauri::ipc::Channel;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::db::research_repo::{
    self, NewResearchQuery, NewResearchSession, ResearchQuery, ResearchSession,
};
use crate::db::message_repo::{self, NewMessage, Message};
use crate::db::session_repo;
use crate::db::chunk_repo;
use crate::db::document_repo;
use crate::db::api_key_repo;
use crate::db::settings_repo;
use crate::llm::LlmRuntime;
use crate::llm::types::{ChatMessage, StreamChatRequest};
use crate::llm::inference;
use crate::security::pii_detector;
use crate::security::Aes256GcmCipher;
use crate::python::manager::PythonManager;
use crate::vector_store::VectorStore;
use crate::cloud::traits::CloudProvider;

#[derive(Debug, Deserialize)]
pub struct ResearchPlanResponse {
    pub knowledge_gaps: Option<String>,
    pub queries: Vec<ResearchQueryResponse>,
}

#[derive(Debug, Deserialize)]
pub struct ResearchQueryResponse {
    pub topic: String,
    pub query: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchPlanResult {
    pub session: ResearchSession,
    pub queries: Vec<ResearchQuery>,
}

#[tauri::command]
pub async fn generate_research_plan(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    session_id: String,
    message: String,
    context_chunks: Vec<String>,
) -> Result<ResearchPlanResult, String> {
    runtime.ensure_running().map_err(|err| err.to_string())?;

    // Save User Message
    let new_msg = NewMessage {
        session_id: &session_id,
        role: "user",
        content: &message,
        source_type: None,
        sources: None,
        tokens_used: Some(0),
        latency_ms: Some(0),
        metadata: None,
    };
    
    let msg = database.with_connection(|conn| message_repo::create(conn, new_msg))
        .map_err(|e| e.to_string())?;
    
    let msg_id = msg.id;

    // Step 2: Build LLM Prompt
    let mut context_text = String::new();
    for chunk in context_chunks {
        context_text.push_str(&chunk);
        context_text.push_str("\n\n");
    }

    let system_prompt = r#"You are an expert Research Assistant.
Your task is to analyze the user's question alongside the provided context documents.
Determine what information is MISSING from the local context to fully answer the user's question.
Identify "Knowledge Gaps" and propose 3-5 specific web search queries to fill those gaps.

You MUST respond with a raw JSON object and nothing else. Do not use markdown blocks.
The JSON must follow this exact schema:
{
  "knowledge_gaps": "A short summary of what is missing from the local documents.",
  "queries": [
    {
      "topic": "A short topic label",
      "query": "The exact search query to execute on the web"
    }
  ]
}"#;

    let user_prompt = format!(
        "Context:\n{}\n\nUser Question:\n{}\n\nGenerate the JSON research plan.",
        context_text, message
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        }
    ];

    // Step 3: Ask LLM for JSON
    let full_response = inference::chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        messages,
        2048,
        0.1,
    ).await.map_err(|err| err.to_string())?;

    // Step 4: Parse JSON
    // Clean up potential markdown blocks if the LLM didn't listen
    let cleaned = full_response
        .replace("```json", "")
        .replace("```", "")
        .trim()
        .to_string();

    let plan: ResearchPlanResponse = match serde_json::from_str(&cleaned) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to parse JSON research plan: {}\nResponse was: {}", e, full_response);
            return Err(format!("Failed to parse JSON plan from LLM: {}", e));
        }
    };

    // Step 5: Save Research Session & Queries
    let research_session_id = Uuid::new_v4().to_string();

    let mut new_queries = Vec::new();
    let mut query_ids = Vec::new();
    let mut highest_risk = "low";
    
    for (idx, q) in plan.queries.iter().enumerate() {
        let q_id = Uuid::new_v4().to_string();
        query_ids.push(q_id.clone());
        
        // Pass the query through our heuristics scanner
        let pii_result = pii_detector::sanitize_query(&q.query);
        
        if pii_result.redact_count > 0 {
            let _ = database.with_connection(|conn| {
                crate::services::audit::log_pii_redacted(
                    conn,
                    Some(&session_id),
                    &q.query,
                    &pii_result.sanitized_text,
                    &pii_result.risk_level
                )
            });
        }

        // Track highest risk
        highest_risk = match (highest_risk, pii_result.risk_level.as_str()) {
            ("high", _) | (_, "high") => "high",
            ("medium", _) | (_, "medium") => "medium",
            _ => "low",
        };
        
        new_queries.push(NewResearchQuery {
            id: q_id,
            research_session_id: research_session_id.clone(),
            query_index: idx as u32,
            topic: q.topic.clone(),
            raw_query: Some(q.query.clone()),
            sanitized_query: pii_result.sanitized_text,
            risk_level: pii_result.risk_level,
            status: "pending".to_string(),
        });
    }

    // Auto-approve threshold logic
    let threshold = database.with_connection(|conn| {
        settings_repo::get(conn, "security.auto_approve_threshold")
    }).map_err(|e| e.to_string())?.unwrap_or_else(|| "never".to_string());

    let auto_approve = match threshold.as_str() {
        "low" => highest_risk == "low",
        "medium" => highest_risk == "low" || highest_risk == "medium",
        "high" => true,
        _ => false, // "never"
    };

    let session_status = if auto_approve { "approved" } else { "planning" };
    let query_status = if auto_approve { "approved" } else { "pending" };

    // Update query statuses in list
    for q in &mut new_queries {
        q.status = query_status.to_string();
    }

    let new_rs = NewResearchSession {
        id: &research_session_id,
        session_id: &session_id,
        message_id: &msg_id,
        status: session_status,
        total_queries: plan.queries.len() as u32,
        knowledge_gaps: plan.knowledge_gaps.as_deref(),
    };

    let rs = database.with_connection(|conn| research_repo::create_session(conn, new_rs))
        .map_err(|e| e.to_string())?;

    database.with_connection(|conn| research_repo::create_queries(conn, &new_queries))
        .map_err(|e| e.to_string())?;

    let saved_queries = database.with_connection(|conn| research_repo::list_queries(conn, &research_session_id))
        .map_err(|e| e.to_string())?;

    Ok(ResearchPlanResult {
        session: rs,
        queries: saved_queries,
    })
}

#[tauri::command]
pub async fn approve_research_plan(
    database: State<'_, Arc<Database>>,
    research_session_id: String,
    approved_query_ids: Vec<String>,
) -> Result<(), String> {
    database.with_connection(|conn| {
        research_repo::update_session_status(conn, &research_session_id, "approved")?;
        
        let all_queries = research_repo::list_queries(conn, &research_session_id)?;
        for q in all_queries {
            let is_approved = approved_query_ids.contains(&q.id);
            let status = if is_approved { "approved" } else { "rejected" };
            research_repo::update_query_status(conn, &q.id, status, Some(is_approved))?;
        }
        
        Ok::<(), crate::utils::error::AppError>(())
    }).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_research(
    app: AppHandle,
    database: State<'_, Arc<Database>>,
    cipher: State<'_, Arc<Aes256GcmCipher>>,
    runtime: State<'_, Arc<LlmRuntime>>,
    python_manager: State<'_, Arc<PythonManager>>,
    vector_store: State<'_, Arc<VectorStore>>,
    research_session_id: String,
    on_token: Channel<String>,
) -> Result<String, String> {
    // 1. Get research session
    let rs = database.with_connection(|conn| {
        research_repo::get_session(conn, &research_session_id)
    }).map_err(|e| e.to_string())?.ok_or("Research session not found")?;

    // 2. Get general session and cloud provider
    let session = database.with_connection(|conn| {
        session_repo::get(conn, &rs.session_id)
    }).map_err(|e| e.to_string())?.ok_or("General chat session not found")?;

    let default_provider = database.with_connection(|conn| {
        crate::db::settings_repo::get(conn, "default_cloud_provider")
    }).map_err(|e| e.to_string())?.unwrap_or_else(|| "openai".to_string());

    let provider_name = session.cloud_provider.clone().unwrap_or(default_provider);

    // 3. Fetch API Key
    let api_key = database.with_connection(|conn| {
        api_key_repo::get_key(conn, &cipher, &provider_name)
    }).map_err(|e| e.to_string())?.ok_or(format!("API key for cloud provider '{}' is not set. Please go to Settings to add it.", provider_name))?;

    // 4. List all approved queries for this research session
    let queries = database.with_connection(|conn| {
        research_repo::list_queries(conn, &research_session_id)
    }).map_err(|e| e.to_string())?;

    let approved_queries: Vec<_> = queries.into_iter().filter(|q| q.status == "approved").collect();
    if approved_queries.is_empty() {
        return Err("No approved queries to execute".to_string());
    }

    // Check API monthly spending limit
    let limit_str = database.with_connection(|conn| {
        settings_repo::get(conn, "security.api_spending_limit")
    }).map_err(|e| e.to_string())?.unwrap_or_else(|| "50.0".to_string());

    let current_str = database.with_connection(|conn| {
        settings_repo::get(conn, "security.api_spending_current")
    }).map_err(|e| e.to_string())?.unwrap_or_else(|| "0.0".to_string());

    let limit = limit_str.parse::<f64>().unwrap_or(50.0);
    let current = current_str.parse::<f64>().unwrap_or(0.0);

    if current >= limit {
        let msg = format!("API Spending Limit Exceeded. Monthly limit: ${:.2}, current spending: ${:.2}. Research blocked.", limit, current);
        let _ = database.with_connection(|conn| {
            crate::services::audit::log_blocked_call(
                conn,
                Some(&rs.session_id),
                &provider_name,
                "blocked_request_spending_limit",
                &msg
            )
        });
        return Err(msg);
    }

    // Update status to in_progress
    database.with_connection(|conn| {
        research_repo::update_session_status(conn, &research_session_id, "in_progress")
    }).map_err(|e| e.to_string())?;

    // 5. Execute each query
    for (idx, q) in approved_queries.iter().enumerate() {
        let _ = app.emit("research-status-update", serde_json::json!({
            "status": format!("🔍 Querying {} for '{}' ({} of {})...", provider_name.to_uppercase(), q.topic, idx + 1, approved_queries.len())
        }));

        // Update status of query to sent
        database.with_connection(|conn| {
            research_repo::update_query_status(conn, &q.id, "sent", None)
        }).map_err(|e| e.to_string())?;

        // Check cache
        let cached = database.with_connection(|conn| {
            Ok(crate::cloud::cache::get_cached_response(conn, &q.sanitized_query))
        }).ok().flatten();

        let response_text = if let Some(cached_resp) = cached {

            let _ = app.emit("research-status-update", serde_json::json!({
                "status": format!("💡 Found cached response for '{}'...", q.topic)
            }));
            cached_resp
        } else {
            // Validate outbound payload for unredacted PII in dev mode
            if let Err(err) = crate::security::network_monitor::validate_outbound_payload(&q.sanitized_query) {
                let _ = database.with_connection(|conn| {
                    crate::services::audit::log_blocked_call(
                        conn,
                        Some(&rs.session_id),
                        &provider_name,
                        &q.sanitized_query,
                        &format!("Blocked by Network Monitor: {}", err)
                    )
                });
                return Err(err.to_string());
            }

            // Execute cloud provider call
            let res = match provider_name.as_str() {
                "openai" => crate::cloud::openai::OpenAiProvider.execute_query(&q.sanitized_query, &api_key).await,
                "gemini" => crate::cloud::gemini::GeminiProvider.execute_query(&q.sanitized_query, &api_key).await,
                "anthropic" => crate::cloud::claude::ClaudeProvider.execute_query(&q.sanitized_query, &api_key).await,
                _ => return Err(format!("Unsupported cloud provider: {}", provider_name)),
            }.map_err(|e| e.to_string())?;

            // Log successful outbound cloud call
            let _ = database.with_connection(|conn| {
                crate::services::audit::log_cloud_call(
                    conn,
                    Some(&rs.session_id),
                    &provider_name,
                    q.raw_query.as_deref().unwrap_or(&q.sanitized_query),
                    &q.sanitized_query,
                    &res,
                    &q.risk_level
                )
            });

            res
        };

        // Save Response, increment completed count, and increment spending limit
        let provider_cost = match provider_name.as_str() {
            "openai" => 0.01,
            "gemini" => 0.005,
            "anthropic" => 0.02,
            _ => 0.005,
        };

        database.with_connection(|conn| {
            research_repo::update_query_response(conn, &q.id, &response_text, "completed")?;
            research_repo::increment_completed_queries(conn, &research_session_id)?;
            
            // Increment spending current
            let current_spent_str = settings_repo::get(conn, "security.api_spending_current")?
                .unwrap_or_else(|| "0.0".to_string());
            let current_spent = current_spent_str.parse::<f64>().unwrap_or(0.0);
            let new_spent = current_spent + provider_cost;
            settings_repo::set(conn, "security.api_spending_current", &new_spent.to_string())?;

            Ok::<(), crate::utils::error::AppError>(())
        }).map_err(|e| e.to_string())?;
    }

    // Mark session as completed
    database.with_connection(|conn| {
        research_repo::update_session_status(conn, &research_session_id, "completed")
    }).map_err(|e| e.to_string())?;

    // 6. Compile responses
    let mut compilation = String::new();
    for q in &approved_queries {
        let updated_q = database.with_connection(|conn| {
            research_repo::list_queries(conn, &research_session_id)
        }).map_err(|e| e.to_string())?.into_iter().find(|x| x.id == q.id).unwrap();

        compilation.push_str(&format!("### Research Topic: {}\n", updated_q.topic));
        compilation.push_str(&format!("Query: {}\n", updated_q.sanitized_query));
        compilation.push_str(&format!("Findings:\n{}\n\n", updated_q.response.unwrap_or_default()));
    }

    // 7. Get original user message and query vector store for context if documents are linked
    let user_msg = database.with_connection(|conn| {
        message_repo::get(conn, &rs.message_id)
    }).map_err(|e| e.to_string())?.ok_or("User query message not found")?;

    let mut rag_context_text = String::new();
    let mut source_names = std::collections::HashSet::new();

    // Trigger RAG search
    if let Ok(embeddings) = python_manager.embed_chunks(vec![user_msg.content.clone()]) {
        if let Some(query_emb) = embeddings.into_iter().next() {
            if let Ok(hits) = vector_store.search(&query_emb, 5) {
                let embedding_ids: Vec<i64> = hits.into_iter().map(|(id, _)| id).collect();
                if !embedding_ids.is_empty() {
                    let _ = database.with_connection(|conn| {
                        if let Ok(chunks) = chunk_repo::get_chunks_by_embedding_ids(conn, &embedding_ids) {
                            for chunk in chunks {
                                rag_context_text.push_str(&chunk.content);
                                rag_context_text.push_str("\n\n");
                                if let Ok(Some(doc)) = document_repo::get(conn, &chunk.document_id) {
                                    source_names.insert(doc.filename);
                                }
                            }
                        }
                        Ok::<(), crate::utils::error::AppError>(())
                    });
                }
            }
        }
    }

    let _ = app.emit("research-status-update", serde_json::json!({
        "status": "✍ Synthesizing findings with local context..."
    }));

    // 8. Call local LLM streaming inference
    runtime.ensure_running().map_err(|err| err.to_string())?;

    let system_prompt = r#"You are an expert AI Research Assistant.
Your task is to synthesize a final answer to the user's question.
You MUST combine the local document context (private data) with the external web research findings.

Structure your answer clearly, indicating which parts came from the local documents and which came from the web research.
Keep private data private. Quote citations accurately."#;

    let user_prompt = format!(
        "Local Document Context:\n{}\n\nWeb Research Findings:\n{}\n\nUser Question:\n{}\n\nPlease synthesize the final answer.",
        rag_context_text, compilation, user_msg.content
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user_prompt,
        }
    ];

    let sources_json = if source_names.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&source_names.into_iter().collect::<Vec<_>>()).unwrap_or_default())
    };

    // Create assistant message in SQLite
    let assistant_message_db = database.with_connection(|conn| {
        message_repo::create(
            conn,
            NewMessage {
                session_id: &rs.session_id,
                role: "assistant",
                content: "",
                source_type: Some("mixed"),
                sources: sources_json.as_deref(),
                tokens_used: None,
                latency_ms: None,
                metadata: None,
            },
        )
    }).map_err(|e| e.to_string())?;

    let started = std::time::Instant::now();
    let mut chunk_count = 0;

    let assistant_content = inference::stream_chat_completion(
        runtime.http_client(),
        &runtime.base_url(),
        StreamChatRequest {
            messages,
            max_tokens: Some(2048),
            temperature: Some(0.3),
        },
        on_token,
        Some(|partial: &str| {
            chunk_count += 1;
            if chunk_count % 20 == 0 {
                let _ = database.with_connection(|conn| {
                    message_repo::update_content(conn, &assistant_message_db.id, partial)
                });
            }
        }),
        Some(&runtime.cancel_inference),
    ).await.map_err(|e| e.to_string())?;

    let latency_ms = started.elapsed().as_millis() as u64;
    let tokens_used = assistant_content.split_whitespace().count() as u32;

    // Save final content and touch session
    database.with_connection(|conn| {
        message_repo::update_content(conn, &assistant_message_db.id, &assistant_content)?;
        message_repo::update_metadata(
            conn, 
            &assistant_message_db.id, 
            Some(tokens_used), 
            Some(latency_ms), 
            None
        )?;
        session_repo::touch(conn, &rs.session_id)?;
        Ok::<(), crate::utils::error::AppError>(())
    }).map_err(|e| e.to_string())?;

    Ok(assistant_content)
}

#[tauri::command]
pub async fn list_research_sessions(
    database: State<'_, Arc<Database>>,
) -> Result<Vec<ResearchSession>, String> {
    database.with_connection(research_repo::list_all_sessions)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_research_session_details(
    database: State<'_, Arc<Database>>,
    research_session_id: String,
) -> Result<ResearchPlanResult, String> {
    let session = database.with_connection(|conn| {
        research_repo::get_session(conn, &research_session_id)
    }).map_err(|e| e.to_string())?.ok_or("Research session not found")?;

    let queries = database.with_connection(|conn| {
        research_repo::list_queries(conn, &research_session_id)
    }).map_err(|e| e.to_string())?;

    Ok(ResearchPlanResult { session, queries })
}

#[tauri::command]
pub async fn delete_research_session(
    database: State<'_, Arc<Database>>,
    research_session_id: String,
) -> Result<(), String> {
    database.with_connection(|conn| research_repo::delete_session(conn, &research_session_id))
        .map_err(|e| e.to_string())
}
