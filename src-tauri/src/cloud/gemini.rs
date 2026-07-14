use serde::{Deserialize, Serialize};
use crate::cloud::traits::CloudProvider;

pub struct GeminiProvider;

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiResponseContent>,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    parts: Option<Vec<GeminiResponsePart>>,
}

#[derive(Deserialize)]
struct GeminiResponsePart {
    text: Option<String>,
}

impl CloudProvider for GeminiProvider {
    async fn execute_query(&self, query: &str, api_key: &str) -> Result<String, String> {
        let client = reqwest::Client::new();
        
        let prompt = format!(
            "You are an expert research analyst. Perform comprehensive web-like research on the following topic. Provide a detailed summary of key facts, figures, and findings:\n\nTopic: {}",
            query
        );

        let body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}",
            api_key
        );

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API error (Status {}): {}", status, err_text));
        }

        let resp_data: GeminiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini JSON response: {}", e))?;

        let candidate = resp_data
            .candidates
            .and_then(|c| c.into_iter().next())
            .ok_or_else(|| "No completion candidates returned by Gemini".to_string())?;

        let parts = candidate
            .content
            .and_then(|c| c.parts)
            .ok_or_else(|| "No parts in candidate content".to_string())?;

        let mut output = String::new();
        for part in parts {
            if let Some(text) = part.text {
                output.push_str(&text);
            }
        }

        if output.is_empty() {
            return Err("Empty text returned by Gemini".to_string());
        }

        Ok(output)
    }
}
