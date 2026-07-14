# Sprint 2 — LLM Integration ✅ Complete

**Deliverable:** Gemma-family model running locally via llama.cpp; streaming inference from Rust to the UI.

## Tasks

| ID | Task | Implementation | Status |
|----|------|----------------|--------|
| S2-T1 | llama.cpp subprocess (server mode) | `llm/runtime.rs` spawns `llama-server` on `127.0.0.1:8081` | ✅ |
| S2-T2 | LLM runtime lifecycle | `start` / `stop` / `restart` + health polling + crash detection | ✅ |
| S2-T3 | Streaming inference | `llm/inference.rs` → OpenAI-compatible SSE → Tauri `Channel` + events | ✅ |
| S2-T4 | Model download + SHA-256 | `llm/model_manager.rs` + `model_downloads` table | ✅ |
| S2-T5 | GPU auto-detection | `llm/hardware.rs` via `nvidia-smi`, CPU fallback, `-ngl` selection | ✅ |
| S2-T6 | Hardware benchmark | `llm/benchmark.rs` — TTFT + tokens/sec report | ✅ |
| S2-T7 | Frontend UI | Premium LlmDemo.tsx — status badge, GPU card, progress bar, benchmark card | ✅ |
| S2-T8 | Delete model IPC | `delete_model` command wired to DB + runtime | ✅ |
| S2-T9 | System prompt | `models/prompts/default.md` — research assistant persona | ✅ |

## Prerequisites (developer machine)

1. **llama-server** on PATH or at `%APPDATA%/com.orvika.app/bin/llama-server.exe`
   - Build from [llama.cpp](https://github.com/ggerganov/llama.cpp) or install a release binary.
   - Override: `LLAMA_SERVER_PATH=C:\path\to\llama-server.exe`
2. **Model GGUF** — download via the app UI or place under `%APPDATA%/com.orvika.app/models/`.
3. **NVIDIA GPU (optional)** — `nvidia-smi` on PATH for GPU layers and VRAM reporting.

## IPC commands

- `get_llm_status`, `start_llm_server`, `stop_llm_server`, `restart_llm_server`
- `get_hardware_info`, `list_registry_models`, `list_downloaded_models`
- `download_model`, `cancel_model_download`, `set_active_model`, `delete_model`
- `register_model_path`, `run_llm_benchmark`, `stream_chat_completion`

## Events

- `model-download-progress` — bytes downloaded / total / phase
- `llm-runtime-status` — running / crashed / stopped
