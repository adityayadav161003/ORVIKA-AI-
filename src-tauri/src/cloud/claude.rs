use crate::cloud::traits::CloudProvider;
use serde::{Deserialize, Serialize};

pub struct ClaudeProvider;

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

impl CloudProvider for ClaudeProvider {
    async fn execute_query(&self, query: &str, api_key: &str) -> Result<String, String> {
        let client = reqwest::Client::new();

        let prompt = format!(
            "You are an expert research analyst. Perform comprehensive web-like research on the following topic. Provide a detailed summary of key facts, figures, and findings:\n\nTopic: {}",
            query
        );

        let body = ClaudeRequest {
            model: "claude-3-5-haiku-20241022".to_string(), // standard Claude 3.5 Haiku
            max_tokens: 2048,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            return Err(format!(
                "Claude API error (Status {}): {}",
                status, err_text
            ));
        }

        let resp_data: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Claude JSON response: {}", e))?;

        let mut output = String::new();
        for block in resp_data.content {
            if block.block_type == "text" {
                if let Some(text) = block.text {
                    output.push_str(&text);
                }
            }
        }

        if output.is_empty() {
            return Err("Empty text returned by Claude".to_string());
        }

        Ok(output)
    }
}
