use serde::{Deserialize, Serialize};
use crate::cloud::traits::CloudProvider;

pub struct OpenAiProvider;

#[derive(Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

impl CloudProvider for OpenAiProvider {
    async fn execute_query(&self, query: &str, api_key: &str) -> Result<String, String> {
        let client = reqwest::Client::new();
        
        let prompt = format!(
            "Perform comprehensive web-like research on the following topic. Provide a detailed summary of key facts, figures, and findings:\n\nTopic: {}",
            query
        );

        let body = OpenAiChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: "You are an expert research analyst. You provide structured, detailed summaries based on your vast knowledge corpus.".to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            temperature: 0.3,
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error (Status {}): {}", status, err_text));
        }

        let resp_data: OpenAiChatResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI JSON response: {}", e))?;

        let choice = resp_data
            .choices
            .first()
            .ok_or_else(|| "No completion choices returned by OpenAI".to_string())?;

        Ok(choice.message.content.clone())
    }
}
