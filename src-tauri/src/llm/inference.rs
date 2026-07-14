use std::time::Instant;

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

use crate::llm::config::CHAT_COMPLETIONS_PATH;
use crate::llm::types::{ChatMessage, StreamChatRequest};
use crate::utils::error::{AppError, AppResult};

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    stream: bool,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

pub async fn stream_chat_completion<F>(
    client: &Client,
    base_url: &str,
    request: StreamChatRequest,
    channel: Channel<String>,
    mut on_chunk: Option<F>,
    cancel_flag: Option<&std::sync::atomic::AtomicBool>,
) -> AppResult<String>
where
    F: FnMut(&str),
{
    let url = format!("{base_url}{CHAT_COMPLETIONS_PATH}");
    let body = ChatCompletionRequest {
        messages: request.messages,
        stream: true,
        max_tokens: request.max_tokens.unwrap_or(512),
        temperature: request.temperature.unwrap_or(0.7),
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|err| AppError::Other(format!("Inference request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("Inference error {status}: {text}")));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full_response = String::new();

    while let Some(chunk) = stream.next().await {
        if let Some(flag) = cancel_flag {
            if flag.load(std::sync::atomic::Ordering::Relaxed) {
                flag.store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
        
        let chunk = chunk.map_err(|err| AppError::Other(format!("Stream read error: {err}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }

            let json_str = line.strip_prefix("data: ").unwrap_or(&line);
            if json_str.is_empty() {
                continue;
            }

            if let Ok(parsed) = serde_json::from_str::<StreamChunk>(json_str) {
                for choice in parsed.choices {
                    if let Some(token) = choice.delta.content {
                        if !token.is_empty() {
                            full_response.push_str(&token);
                            
                            if let Some(ref mut cb) = on_chunk {
                                cb(&full_response);
                            }

                            channel
                                .send(token)
                                .map_err(|err| AppError::Other(format!("Channel send: {err}")))?;
                        }
                    }
                }
            }
        }
    }

    Ok(full_response)
}

pub async fn complete_once(
    client: &Client,
    base_url: &str,
    prompt: &str,
    max_tokens: u32,
) -> AppResult<(String, f64, u32)> {
    let started = Instant::now();
    let url = format!("{base_url}{CHAT_COMPLETIONS_PATH}");
    let body = ChatCompletionRequest {
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        stream: true,
        max_tokens,
        temperature: 0.1,
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|err| AppError::Other(format!("Benchmark request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("Benchmark error {status}: {text}")));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut full = String::new();
    let mut ttft_ms: Option<f64> = None;
    let mut token_count: u32 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| AppError::Other(format!("Stream read error: {err}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }

            let json_str = line.strip_prefix("data: ").unwrap_or(&line);
            if let Ok(parsed) = serde_json::from_str::<StreamChunk>(json_str) {
                for choice in parsed.choices {
                    if let Some(token) = choice.delta.content {
                        if !token.is_empty() {
                            if ttft_ms.is_none() {
                                ttft_ms = Some(started.elapsed().as_secs_f64() * 1000.0);
                            }
                            token_count += 1;
                            full.push_str(&token);
                        }
                    }
                }
            }
        }
    }

    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok((full, ttft_ms.unwrap_or(total_ms), token_count))
}

pub async fn chat_completion(
    client: &Client,
    base_url: &str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
) -> AppResult<String> {
    let url = format!("{base_url}{CHAT_COMPLETIONS_PATH}");
    let body = ChatCompletionRequest {
        messages,
        stream: false,
        max_tokens,
        temperature,
    };

    #[derive(Debug, Deserialize)]
    struct ChatChoice {
        message: ChatMessage,
    }

    #[derive(Debug, Deserialize)]
    struct ChatResponse {
        choices: Vec<ChatChoice>,
    }

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|err| AppError::Other(format!("Chat completion request failed: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Other(format!("Chat completion error {status}: {text}")));
    }

    let parsed = response.json::<ChatResponse>().await
        .map_err(|err| AppError::Other(format!("Failed to parse chat response: {err}")))?;

    if let Some(choice) = parsed.choices.into_iter().next() {
        Ok(choice.message.content)
    } else {
        Err(AppError::Other("Empty choices array in chat response".into()))
    }
}
