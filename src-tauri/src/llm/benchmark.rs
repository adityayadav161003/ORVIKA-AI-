use crate::llm::hardware::detect_hardware;
use crate::llm::inference::complete_once;
use crate::llm::runtime::LlmRuntime;
use crate::llm::types::BenchmarkReport;
use crate::utils::error::{AppError, AppResult};

const BENCHMARK_PROMPT: &str = "Reply with exactly one short sentence about local AI privacy.";

/// Rough approximation: split on whitespace and multiply by 1.3 (average
/// sub-word token overhead). Good enough for the benchmark report.
fn estimate_tokens(text: &str) -> u32 {
    let words = text.split_whitespace().count();
    ((words as f64) * 1.3).ceil() as u32
}

pub async fn run_benchmark(runtime: &LlmRuntime) -> AppResult<BenchmarkReport> {
    runtime.ensure_running()?;

    let hardware = detect_hardware();
    let status = runtime.status();
    let model_path = status.model_path.unwrap_or_default();

    let (text, ttft_ms, token_count) = complete_once(
        runtime.http_client(),
        &runtime.base_url(),
        BENCHMARK_PROMPT,
        64,
    )
    .await?;

    if token_count == 0 {
        return Err(AppError::Other(
            "Benchmark produced no tokens — is the model loaded?".into(),
        ));
    }

    let total_ms = ttft_ms.max(1.0);
    let tokens_per_second = (token_count as f64 / total_ms) * 1000.0;
    let prompt_tokens = estimate_tokens(BENCHMARK_PROMPT);

    tracing::info!(
        ttft_ms,
        token_count,
        tokens_per_second,
        prompt_tokens,
        chars = text.len(),
        "LLM benchmark complete"
    );

    Ok(BenchmarkReport {
        prompt_tokens,
        completion_tokens: token_count,
        ttft_ms,
        tokens_per_second,
        total_ms,
        backend: hardware.backend.clone(),
        model_path,
        hardware,
    })
}
