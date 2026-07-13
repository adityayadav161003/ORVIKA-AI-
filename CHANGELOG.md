# Changelog

## [0.3.0] - 2026-05-30

### Added

#### Backend (Rust)

- `db/session_repo.rs` - session create/list/get/delete, rename, touch, and message counts.
- `db/message_repo.rs` - message create/get/list and recent-context loading.
- `commands/chat.rs` - Sprint 4 chat IPC: `create_session`, `list_sessions`, `get_session`, `delete_session`, `get_messages`, `send_message`.
- `llm/inference.rs` now collects the streamed assistant response while forwarding tokens over `Channel<String>`, so `send_message` can persist the completed reply.

#### Frontend (React + TypeScript)

- `ChatPage.tsx` - full chat workspace replacing the Sprint 4 placeholder.
- `SessionSidebar` - session list, create, switch, and delete actions.
- `MessageInput` - textarea send flow with Enter submit and Shift+Enter newline behavior.
- `MessageBubble` - user and assistant bubbles with streaming support and safe markdown block rendering.

#### Docs

- `docs/sprint-4-plan.md` - implementation plan and completion matrix derived from `0mimo.html` and `0development.html`.

## [0.2.0] - 2026-05-30

### Added

#### Backend (Rust)

- `llm/runtime.rs` — `llama-server` subprocess management: start / stop / restart / crash detection
- `llm/inference.rs` — streaming chat completions via OpenAI-compatible SSE → Tauri `Channel<String>`
- `llm/model_manager.rs` — model download from HuggingFace Hub, `.part` temp file, SHA-256 verification, cancel support
- `llm/hardware.rs` — NVIDIA GPU detection via `nvidia-smi`, recommended `-ngl` GPU layers, CPU fallback
- `llm/benchmark.rs` — TTFT + tokens/sec benchmark using `complete_once`
- `llm/config.rs` — llama-server binary resolution (env var → app data bin/ → PATH)
- `llm/types.rs` — all Sprint 2 types: `LlmStatus`, `RegistryModel`, `DownloadedModel`, `BenchmarkReport`, etc.
- `commands/llm.rs` — 13 IPC commands: `get_llm_status`, `start_llm_server`, `stop_llm_server`, `restart_llm_server`, `get_hardware_info`, `list_registry_models`, `list_downloaded_models`, `download_model`, `cancel_model_download`, `set_active_model`, `register_model_path`, `run_llm_benchmark`, `stream_chat_completion`, `delete_model`
- `db/model_repo.rs` — CRUD for `model_downloads` SQLite table
- Events: `model-download-progress`, `llm-runtime-status`
- `benchmark.rs` — improved `prompt_tokens` estimate (word-count × 1.3 rather than hardcoded 1)

#### Frontend (React + TypeScript)

- `LlmDemo.tsx` — premium UI: live status badge, GPU info card, animated download progress bar, model list with active/delete actions, streaming chat with blinking cursor, benchmark metrics card
- `App.tsx` — polished shell: logo, pill tab navigation, live LLM status dot, DB version indicator, sticky header with blur backdrop

#### Models / Docs

- `models/registry.json` — Gemma 2 9B (Q4_K_M) + SmolLM2 360M (Q8_0) registry entries
- `models/prompts/default.md` — privacy-first research assistant system prompt
- `docs/sprint-2-plan.md` — all 9 Sprint 2 tasks marked complete

#### Migrations

- `010_create_model_downloads` — `model_downloads` table
- `011_placeholder` — sequence gap filler (keeps migration numbering contiguous)
- `012_seed_default_settings` — default app settings

## [0.1.0] - 2026-05-30

### Added

- Sprint 1 project scaffolding
- Tauri 2 + React + TypeScript application shell
- Design system primitives (Button, Input, Modal, Select, Toggle, Spinner)
- SQLite migration framework with full schema
- GitHub Actions CI pipeline
