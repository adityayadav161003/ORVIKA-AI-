# ORVIKA AI — Build Status Tracker

> **Last updated:** 2026-07-14  
> **Active sprint:** Sprint 6 — Hybrid Retrieval, Re-ranking & Query Transformation  
> Owner: Aditya

---

## Legend

| Symbol | Meaning                             |
| ------ | ----------------------------------- |
| ✅     | Done — acceptance criteria verified |
| 🔄     | In Progress — actively being worked |
| ⏳     | Not Started — queued                |
| ❌     | Blocked — needs intervention        |

---

## Pre-Sprint: Scaffold Audit (Ground Truth)

| Item                                                                              | Status                      | Notes                                        |
| --------------------------------------------------------------------------------- | --------------------------- | -------------------------------------------- |
| Tauri 2 + React 18 + TS shell                                                     | ✅ Real, solid              | Don't rewrite                                |
| SQLite schema (11 tables via migrations 001–014)                                  | ✅ Real                     | src-tauri/migrations/ — keep and extend      |
| `llm/runtime.rs` — llama.cpp sidecar                                              | ✅ Real                     | Pattern to copy for Weaviate sidecar         |
| `llm/model_manager.rs` — model download + checksum                                | ✅ Real                     | Pattern to copy for Weaviate binary download |
| `llm/hardware.rs` — GPU detection                                                 | ✅ Real                     |                                              |
| `python/manager.rs` — venv + per-call subprocess spawning                         | ✅ Present (wrong arch)     | Must become persistent server in Sprint 5    |
| `vector_store/store.rs` — JSON linear scan                                        | ✅ Present (prototype only) | Replaced in Sprint 4 by Weaviate             |
| `document/types.rs`                                                               | ✅ Done                     | Sprint 3                                     |
| `document/parser.rs`                                                              | ✅ Done                     | Sprint 3                                     |
| `document/chunker.rs`                                                             | ✅ Done                     | Sprint 3                                     |
| `document/ocr.rs`                                                                 | ✅ Done                     | Sprint 3                                     |
| `embedding/engine.rs`                                                             | ✅ Done                     | Sprint 5                                     |
| `embedding/types.rs`                                                              | ✅ Done                     | Sprint 5                                     |
| `vector_store/search.rs`                                                          | ✅ Done                     | Sprint 4/6                                   |
| `vector_store/types.rs`                                                           | ✅ Done                     | Sprint 4                                     |
| `services/document.rs`                                                            | ✅ Done                     | Sprint 3                                     |
| `services/chat.rs`                                                                | ❌ Stub                     | Sprint 7                                     |
| `services/research.rs`                                                            | ❌ Stub                     | Sprint 8                                     |
| `services/session.rs`                                                             | ❌ Stub                     | Sprint 10                                    |
| `services/settings.rs`                                                            | ❌ Stub                     | Sprint 10                                    |
| `services/privacy.rs`                                                             | ❌ Stub                     | Sprint 10                                    |
| `services/media.rs`                                                               | ❌ Stub                     | Sprint 9                                     |
| `llm/context.rs`                                                                  | ❌ Stub                     | Sprint 6                                     |
| Frontend pages (Chat, Documents, Media, Models, Research, Settings, Transparency) | ✅ Scaffolded               | Real UI shell exists                         |

---

## Sprint 3 — Document Ingestion & Structure-Aware Chunking

**Goal:** Uploading a real PDF/DOCX/TXT/MD file produces high-quality, metadata-rich chunks in SQLite — no vectors yet.

| Task                                                                                        | File                               | Status  | Notes                                     |
| ------------------------------------------------------------------------------------------- | ---------------------------------- | ------- | ----------------------------------------- |
| 3.1 Define ParsedBlock, Chunk, ChunkingConfig types                                         | `document/types.rs`                | ✅ Done |                                           |
| 3.2 Implement `document/parser.rs` — structured output with heading_path + page_number      | `document/parser.rs`               | ✅ Done | Uses PythonManager                        |
| 3.3 Implement `document/chunker.rs` — markdown-structure-first, 400-600 tokens, 15% overlap | `document/chunker.rs`              | ✅ Done | Uses text-splitter crate                  |
| 3.4 Wire OCR via `ocr_parser.py`; store confidence in metadata                              | `document/ocr.rs`                  | ✅ Done |                                           |
| 3.5 Implement `services/document.rs` — orchestrate parse+chunk+insert_batch+mark parsed_at  | `services/document.rs`             | ✅ Done |                                           |
| 3.6 Move inline logic from `commands/documents.rs` into service layer                       | `commands/documents.rs`            | ✅ Done |                                           |
| 3.7 Unit tests for chunker boundary behavior                                                | `document/chunker.rs` #[cfg(test)] | ✅ Done | 5 unit tests                              |
| 3.8 Add migration `015_add_weaviate_fields.up.sql`                                          | `src-tauri/migrations/`            | ✅ Done | Added Weaviate columns to document_chunks |

### Acceptance Criteria — Sprint 3

- [x] 20-page PDF → >=90% of chunks have non-null section_heading
- [x] No chunk crosses a top-level heading boundary
- [x] Re-uploading same file is idempotent (no duplicate chunks)
- [x] Mid-parse kill + re-run produces correct, non-duplicated result

---

## Sprint 4 — Chroma + SQLite FTS5 Hybrid Store Integration

**Goal:** Chroma runs embedded inside local FastAPI sidecar, SQLite FTS5 mirrors chunk content, and chunks can be written, filtered, and deleted.

| Task                                                                                   | File                                             | Status  | Notes                                    |
| -------------------------------------------------------------------------------------- | ------------------------------------------------ | ------- | ---------------------------------------- |
| 4.1 Define IndexableChunk, SearchFilter, FusedCandidate                                | `vector_store/types.rs`                          | ✅ Done |                                          |
| 4.2 Migration 016 — create FTS5 table and sync triggers                                | `src-tauri/migrations/`                          | ✅ Done | `016_create_chunks_fts.up.sql`           |
| 4.3 `vector_store/chroma_client.rs` — FastAPI vector store interface                   | `vector_store/chroma_client.rs` [NEW]            | ✅ Done | reqwest-based client                     |
| 4.4 `vector_store/fts_search.rs` — SQLite keyword BM25 search                          | `vector_store/fts_search.rs` [NEW]               | ✅ Done | FTS5 MATCH with sanitized terms          |
| 4.5 `vector_store/fusion.rs` — Reciprocal Rank Fusion (RRF)                            | `vector_store/fusion.rs` [NEW]                   | ✅ Done | Score merging with k=60                  |
| 4.6 Rewrite `vector_store/store.rs` — replace JSON scan                                | `vector_store/store.rs`                          | ✅ Done | Orchestrates Chroma client + FTS5 search |
| 4.7 Extend `services/document.rs` — write chunks to Chroma and FTS5; update is_indexed | `services/document.rs`                           | ✅ Done | Embedded during ingestion                |
| 4.8 Write ADR 006 & 007 — supersedes ADR 003 & 005                                     | `docs/adr/006-chroma-fts5-hybrid-store.md` [NEW] | ✅ Done | Chroma + FTS5 & Local AI Sidecar ADRs    |

### Acceptance Criteria — Sprint 4

- [x] Round-trip write -> query -> delete using embedded Chroma (FastAPI) and FTS5
- [x] FTS5 BM25 matches keyword terms correctly
- [x] Reciprocal Rank Fusion (RRF) unit tests verify correct candidate order
- [x] Deleting a document cascades to both SQLite chunks and Chroma DB records

---

## Sprint 5 — Local AI Sidecar (Embedding, Rerank & Chroma Index)

**Goal:** Persistent local embedding + reranking server (FastAPI sidecar), bge-base-en-v1.5.

| Task                                                                        | File                                  | Status  | Notes                                 |
| --------------------------------------------------------------------------- | ------------------------------------- | ------- | ------------------------------------- |
| 5.1 `ai_server.py` — FastAPI sidecar, exposes /embed + /rerank + /vector/\* | `src-tauri/python/ai_server.py` [NEW] | ✅ Done | Loaded once, telemetry disabled       |
| 5.2 `embedding/engine.rs` — sidecar manager + embed_batch() + rerank()      | `embedding/engine.rs`                 | ✅ Done | Manages startup on port 8082          |
| 5.3 `embedding/types.rs` — JSON request/response types                      | `embedding/types.rs`                  | ✅ Done |                                       |
| 5.4 Embedding cache — SHA256 content hash -> vector                         | SQLite cache table                    | ✅ Done | `embedding_cache` table inside app.db |
| 5.5 Redirect embed_chunks() in manager.rs to FastAPI server                 | `python/manager.rs`                   | ✅ Done | No subprocess-spawning overhead       |
| 5.6 Update requirements.txt                                                 | `src-tauri/python/requirements.txt`   | ✅ Done | Added fastapi, uvicorn, and chromadb  |

### Acceptance Criteria — Sprint 5

- [x] Models loaded exactly once at startup
- [x] Cache hits retrieve vector in single-digit milliseconds
- [x] Rerank correctly prioritizes relevant chunks
- [x] Subprocess overhead eliminated for RAG indexing

---

## Sprint 6 — Hybrid Retrieval, Re-ranking & Query Transformation

**Goal:** retrieve(query, context, filters) -> Vec<RankedChunk> — state-of-the-art local RAG.

| Task                                                                                      | File                              | Status | Notes                       |
| ----------------------------------------------------------------------------------------- | --------------------------------- | ------ | --------------------------- |
| 6.1 `vector_store/search.rs` — hybrid search (alpha=0.5, top_k=20)                        | `vector_store/search.rs`          | ⏳     |                             |
| 6.2 `services/query_rewrite.rs` — conversational rewriting + HyDE (toggleable)            | `services/query_rewrite.rs` [NEW] | ⏳     |                             |
| 6.3 Re-ranking step: top-20 → rerank() → top 5-8                                          | `services/chat.rs`                | ⏳     |                             |
| 6.4 Deduplication / diversity pass on ranked chunks                                       | `services/chat.rs`                | ⏳     |                             |
| 6.5 `llm/context.rs` — context assembly with source tags, context-window-aware truncation | `llm/context.rs`                  | ⏳     |                             |
| 6.6 Research plan trace object for Transparency page                                      | `services/chat.rs`                | ⏳     | Feeds ResearchPlanModal.tsx |

### Acceptance Criteria — Sprint 6

- [ ] Follow-up question retrieves page-correct content
- [ ] Vocabulary-mismatch query retrieves correct section via hybrid
- [ ] Re-ranking measurably reorders results in regression test cases
- [ ] Rough recall@8 number produced

---

## Sprint 7 — Grounded Chat Orchestration

**Goal:** Trustworthy, cited, streaming answers.

| Task                                                                     | File                                                | Status | Notes |
| ------------------------------------------------------------------------ | --------------------------------------------------- | ------ | ----- |
| 7.1 Orchestrate full pipeline in services/chat.rs                        | `services/chat.rs`                                  | ⏳     |       |
| 7.2 Citation enforcement — [[chunk:id]] markers, hallucination tripwire  | `services/chat.rs`                                  | ⏳     |       |
| 7.3 Refusal path — honest "no relevant docs" response, visually distinct | `services/chat.rs`                                  | ⏳     |       |
| 7.4 Session memory — persist retrieved-chunk history per message         | `migrations/015_add_message_citations.up.sql` [NEW] | ⏳     |       |

---

## Sprint 8 — Research Agent (web-augmented, opt-in)

| Task                                                    | File                   | Status | Notes                 |
| ------------------------------------------------------- | ---------------------- | ------ | --------------------- |
| 8.1 Local LLM plan decomposition (no network)           | `services/research.rs` | ⏳     |                       |
| 8.2 Explicit user confirmation gate before network call | `commands/research.rs` | ⏳     | Hard privacy boundary |
| 8.3 Web search/fetch + synthesis with URL citations     | `services/research.rs` | ⏳     |                       |
| 8.4 Log all external calls to audit_log                 | `services/audit.rs`    | ⏳     |                       |

---

## Sprint 9 — Media Pipeline Integration

| Task                                                                                      | File                                                | Status | Notes |
| ----------------------------------------------------------------------------------------- | --------------------------------------------------- | ------ | ----- |
| 9.1 `services/media.rs` — transcribe → chunk by speaker turn → embed → index MediaSegment | `services/media.rs`                                 | ⏳     |       |
| 9.2 `media_segments` SQLite table                                                         | `migrations/016_create_media_segments.up.sql` [NEW] | ⏳     |       |
| 9.3 Timestamp citations in chat UI                                                        | `src/pages/ChatPage.tsx`                            | ⏳     |       |

---

## Sprint 10 — Security, Privacy & Compliance Hardening

| Task                                                      | File                                              | Status | Notes |
| --------------------------------------------------------- | ------------------------------------------------- | ------ | ----- |
| 10.1 PII detection over docs + outgoing research queries  | `services/privacy.rs`, `security/pii_detector.rs` | ⏳     |       |
| 10.2 Network allowlist enforcement                        | `security/network_monitor.rs`                     | ⏳     |       |
| 10.3 Verify SQLite-at-rest AES-GCM + OS credential store  | `security/encryption.rs`                          | ⏳     |       |
| 10.4 Implement services/session.rs + services/settings.rs | `services/session.rs`, `services/settings.rs`     | ⏳     |       |
| 10.5 Wire compliance templates                            | `commands/security.rs`                            | ⏳     |       |

---

## Sprint 11 — RAG Evaluation Harness

| Task                                                  | File                              | Status | Notes |
| ----------------------------------------------------- | --------------------------------- | ------ | ----- |
| 11.1 Golden test set (30-50 Q/A pairs)                | `tests/rag_eval/` [NEW]           | ⏳     |       |
| 11.2 Retrieval metrics: recall@k, MRR                 | `tests/rag_eval/metrics.rs` [NEW] | ⏳     |       |
| 11.3 Answer quality: faithfulness + citation accuracy | `tests/rag_eval/quality.rs` [NEW] | ⏳     |       |
| 11.4 CLI runner + results in docs/                    | `docs/eval-results.md`            | ⏳     |       |
| 11.5 CI gate at recall@8 >=0.85 (after baseline)      | `.github/workflows/ci.yml`        | ⏳     |       |

---

## Sprint 12 — Packaging & Release

| Task                                                          | File                        | Status | Notes                            |
| ------------------------------------------------------------- | --------------------------- | ------ | -------------------------------- |
| 12.1 Bundle Weaviate binary + Python env (Windows first)      | `src-tauri/tauri.conf.json` | ⏳     |                                  |
| 12.2 First-run setup wizard                                   | `SetupWizard.tsx`           | ⏳     | Show total download size upfront |
| 12.3 Auto-update (Tauri updater, GitHub Releases)             | `src-tauri/tauri.conf.json` | ⏳     |                                  |
| 12.4 Windows code signing (self-signed; document SmartScreen) | CI signing config           | ⏳     |                                  |
| 12.5 Full offline test from clean VM                          | Manual verification         | ⏳     |                                  |

---

## Non-Functional Requirements Tracker

| Requirement                                               | Target                              | Status                  |
| --------------------------------------------------------- | ----------------------------------- | ----------------------- |
| Cold start                                                | < 5s (excl. first-run downloads)    | ⏳ Not measured         |
| Document ingestion (10-page PDF → searchable)             | < 15s on mid-range laptop           | ⏳ Not measured         |
| Query → first token streamed                              | < 3s (local 7-9B quantized)         | ⏳ Measure in Sprint 6  |
| Retrieval quality                                         | recall@8 >=0.85 on golden test set  | ⏳ Sprint 11            |
| Offline guarantee                                         | Zero network calls after setup      | ⏳ Verify in Sprint 4   |
| No data leaves device                                     | Allowlist enforced at network layer | ⏳ Sprint 10            |
| Citations traceable to real chunk IDs                     | Required                            | ⏳ Sprint 7             |
| Crash resilience — mid-ingestion kill → idempotent resume | Required                            | 🔄 Designed in Sprint 3 |

---

## Open Decisions / Blockers

| #    | Issue                                                       | Status                                             |
| ---- | ----------------------------------------------------------- | -------------------------------------------------- |
| OD-1 | Embedding cache backend: SQLite table vs. sled/redb KV      | ⏳ Decide in Sprint 5                              |
| OD-2 | FastAPI vs. stdlib http.server for embedding_server.py      | ⏳ Decide in Sprint 5                              |
| OD-3 | HyDE default on/off in Settings                             | ⏳ Default OFF; document tradeoff in settings copy |
| OD-4 | External search backend for Research Agent (Sprint 8)       | ❌ Need Aditya decision before Sprint 8            |
| OD-5 | Code signing certificate (EV vs. self-signed) for Sprint 12 | ❌ Need Aditya decision before Sprint 12           |
