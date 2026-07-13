import { invoke } from "@tauri-apps/api/core";

export interface SystemBenchmarkResult {
  startupTimeMs: number;
  ipcLatencyMs: number;
  llmBenchmark?: {
    promptTokens: number;
    completionTokens: number;
    ttftMs: number;
    tokensPerSecond: number;
    totalMs: number;
    backend: string;
    modelPath: string;
  };
}

/**
 * Measure application cold start time using the browser Performance API.
 */
export function getStartupTime(): number {
  if (typeof window !== "undefined" && window.performance && window.performance.timing) {
    const timing = window.performance.timing;
    return timing.domContentLoadedEventEnd - timing.navigationStart;
  }
  return 0;
}

/**
 * Measure Tauri IPC round-trip latency by making a lightweight call.
 */
export async function measureIpcLatency(): Promise<number> {
  const start = performance.now();
  await invoke("get_db_status");
  return performance.now() - start;
}

/**
 * Run a full system benchmark, combining startup, IPC, and model inference metrics.
 */
export async function runSystemBenchmark(): Promise<SystemBenchmarkResult> {
  const startupTimeMs = getStartupTime();
  const ipcLatencyMs = await measureIpcLatency();

  let llmBenchmark;
  try {
    llmBenchmark =
      await invoke<NonNullable<SystemBenchmarkResult["llmBenchmark"]>>("run_llm_benchmark");
  } catch (err) {
    console.warn("LLM model benchmark not run or failed:", err);
  }

  const report: SystemBenchmarkResult = {
    startupTimeMs,
    ipcLatencyMs,
    llmBenchmark,
  };

  console.log("System Benchmark Report:", report);
  return report;
}
