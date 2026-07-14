# ORVIKA AI — Build Status Tracker

> **Last updated:** 2026-07-14  
> **Active sprint:** Sprint 4 — Weaviate Vector Store Integration  
> Owner: Aditya


---

## Legend

| Symbol | Meaning |
|---|---|
| ✅ | Done — acceptance criteria verified |
| 🔄 | In Progress — actively being worked |
| ⏳ | Not Started — queued |
| ❌ | Blocked — needs intervention |

---

## Pre-Sprint: Scaffold Audit (Ground Truth)

| Item | Status | Notes |
|---|---|---|
| Tauri 2 + React 18 + TS shell | ✅ Real, solid | Don't rewrite |
| SQLite schema (11 tables via migrations 001–014) | ✅ Real | src-tauri/migrations/ — keep and extend |
| `llm/runtime.rs` — llama.cpp sidecar | ✅ Real | Pattern to copy for Weaviate sidecar |
| `llm/model_manager.rs` — model download + checksum | ✅ Real | Pattern to copy for Weaviate binary download |
| `llm/hardware.rs` — GPU detection | ✅ Real | |
| `python/manager.rs` — venv + per-call subprocess spawning | ✅ Present (wrong arch) | Must become persistent server in Sprint 5 |
| `vector_store/store.rs` — JSON linear scan | ✅ Present (prototype only) | Replaced in Sprint 4 by Weaviate |
| `document/types.rs` | ✅ Done | Sprint 3 |
| `document/parser.rs` | ✅ Done | Sprint 3 |
| `document/chunker.rs` | ✅ Done | Sprint 3 |
| `document/ocr.rs` | ✅ Done | Sprint 3 |
| `embedding/engine.rs` | ❌ Stub | Sprint 5 |
| `embedding/types.rs` | ❌ Stub | Sprint 5 |
| `vector_store/search.rs` | ❌ Stub | Sprint 4/6 |
| `vector_store/types.rs` | ❌ Stub | Sprint 4 |
| `services/document.rs` | ✅ Done | Sprint 3 |
| `services/chat.rs` | ❌ Stub | Sprint 7 |
| `services/research.rs` | ❌ Stub | Sprint 8 |
| `services/session.rs` | ❌ Stub | Sprint 10 |
| `services/settings.rs` | ❌ Stub | Sprint 10 |
| `services/privacy.rs` | ❌ Stub | Sprint 10 |
| `services/media.rs` | ❌ Stub | Sprint 9 |
| `llm/context.rs` | ❌ Stub | Sprint 6 |
| Frontend pages (Chat, Documents, Media, Models, Research, Settings, Transparency) | ✅ Scaffolded | Real UI shell exists |


---

## Sprint 3 — Document Ingestion & Structure-Aware Chunking

**Goal:** Uploading a real PDF/DOCX/TXT/MD file produces high-quality, metadata-rich chunks in SQLite — no vectors yet.

| Task | File | Status | Notes |
|---|---|---|---|
| 3.1 Define ParsedBlock, Chunk, ChunkingConfig types | `document/types.rs` | ✅ Done | |
| 3.2 Implement `document/parser.rs` — structured output with heading_path + page_number | `document/parser.rs` | ✅ Done | Uses PythonManager |
| 3.3 Implement `document/chunker.rs` — markdown-structure-first, 400-600 tokens, 15% overlap | `document/chunker.rs` | ✅ Done | Uses text-splitter crate |
| 3.4 Wire OCR via `ocr_parser.py`; store confidence in metadata | `document/ocr.rs` | ✅ Done | |
| 3.5 Implement `services/document.rs` — orchestrate parse+chunk+insert_batch+mark parsed_at | `services/document.rs` | ✅ Done | |
| 3.6 Move inline logic from `commands/documents.rs` into service layer | `commands/documents.rs` | ✅ Done | |
| 3.7 Unit tests for chunker boundary behavior | `document/chunker.rs` #[cfg(test)] | ✅ Done | 5 unit tests |
| 3.8 Add migration `015_add_weaviate_fields.up.sql` | `src-tauri/migrations/` | ✅ Done | Added Weaviate columns to document_chunks |

### Acceptance Criteria — Sprint 3
- [x] 20-page PDF → >=90% of chunks have non-null section_heading
- [x] No chunk crosses a top-level heading boundary
- [x] Re-uploading same file is idempotent (no duplicate chunks)
- [x] Mid-parse kill + re-run produces correct, non-duplicated result


---

## Sprint 4 — Weaviate Vector Store Integration

**Goal:** Weaviate sidecar running, schema provisioned, chunks written/filtered/deleted.

| Task | File | Status | Notes |
|---|---|---|---|
| 4.1 Define WeaviateConfig, IndexableChunk, SearchFilter | `vector_store/types.rs` | ⏳ | |
| 4.2 `vector_store/sidecar.rs` — WeaviateRuntime (mirror llm/runtime.rs) | `vector_store/sidecar.rs` [NEW] | ⏳ | |
| 4.3 `vector_store/schema.rs` — idempotent schema provisioning | `vector_store/schema.rs` [NEW] | ⏳ | |
| 4.4 `vector_store/weaviate_client.rs` — thin REST/GraphQL client | `vector_store/weaviate_client.rs` [NEW] | ⏳ | reqwest-based, no community crate |
| 4.5 Rewrite `vector_store/store.rs` — delete JSON impl | `vector_store/store.rs` | ⏳ | |
| 4.6 Extend `services/document.rs` — write to Weaviate after embedding; mark is_indexed=0 on failure | `services/document.rs` | ⏳ | Depends on Sprint 5 |
| 4.7 Write ADR 004 — supersedes ADR 003 | `docs/adr/004-weaviate-vector-store.md` [NEW] | ⏳ | |
| 4.8 Note Weaviate binary bundling in release workflow | `.github/workflows/release.yml` | ⏳ | |

### Acceptance Criteria — Sprint 4
- [ ] Fresh launch: Weaviate downloads, starts, schema provisions
- [ ] External kill of Weaviate → clean user-legible error, offer restart
- [ ] Full offline test: ingest + search with zero network calls
- [ ] Document delete cascades to Weaviate objects

---

## Sprint 5 — Embedding Service Upgrade

**Goal:** Persistent local embedding + reranking server (FastAPI sidecar), bge-base-en-v1.5.

| Task | File | Status | Notes |
|---|---|---|---|
| 5.1 `embedding_server.py` — FastAPI, /embed + /rerank + /health, models loaded once | `src-tauri/python/embedding_server.py` [NEW] | ⏳ | bge-base-en-v1.5 + bge-reranker-base |
| 5.2 `embedding/engine.rs` — sidecar manager + embed_batch() + rerank() | `embedding/engine.rs` | ⏳ | Mirror llm/runtime.rs |
| 5.3 `embedding/types.rs` — request/response/config types | `embedding/types.rs` | ⏳ | |
| 5.4 Embedding cache (content hash → vector) | SQLite or sled | ⏳ | Decide backend in OD-1 |
| 5.5 Retire embed_chunks() per-call path in python/manager.rs | `python/manager.rs` | ⏳ | Keep venv-bootstrap |
| 5.6 Update requirements.txt | `src-tauri/python/requirements.txt` | ⏳ | |

### Acceptance Criteria — Sprint 5
- [ ] Embedding server model-load timestamp appears exactly once per session
- [ ] Re-embedding unchanged document = cache hit, completes in ms
- [ ] rerank() unit test: relevant > irrelevant
- [ ] Benchmark panel shows throughput + rerank latency

---

## Sprint 6 — Hybrid Retrieval, Re-ranking & Query Transformation

**Goal:** retrieve(query, context, filters) -> Vec<RankedChunk> — state-of-the-art local RAG.

| Task | File | Status | Notes |
|---|---|---|---|
| 6.1 `vector_store/search.rs` — hybrid search (alpha=0.5, top_k=20) | `vector_store/search.rs` | ⏳ | |
| 6.2 `services/query_rewrite.rs` — conversational rewriting + HyDE (toggleable) | `services/query_rewrite.rs` [NEW] | ⏳ | |
| 6.3 Re-ranking step: top-20 → rerank() → top 5-8 | `services/chat.rs` | ⏳ | |
| 6.4 Deduplication / diversity pass on ranked chunks | `services/chat.rs` | ⏳ | |
| 6.5 `llm/context.rs` — context assembly with source tags, context-window-aware truncation | `llm/context.rs` | ⏳ | |
| 6.6 Research plan trace object for Transparency page | `services/chat.rs` | ⏳ | Feeds ResearchPlanModal.tsx |

### Acceptance Criteria — Sprint 6
- [ ] Follow-up question retrieves page-correct content
- [ ] Vocabulary-mismatch query retrieves correct section via hybrid
- [ ] Re-ranking measurably reorders results in regression test cases
- [ ] Rough recall@8 number produced

---

## Sprint 7 — Grounded Chat Orchestration

**Goal:** Trustworthy, cited, streaming answers.

| Task | File | Status | Notes |
|---|---|---|---|
| 7.1 Orchestrate full pipeline in services/chat.rs | `services/chat.rs` | ⏳ | |
| 7.2 Citation enforcement — [[chunk:id]] markers, hallucination tripwire | `services/chat.rs` | ⏳ | |
| 7.3 Refusal path — honest "no relevant docs" response, visually distinct | `services/chat.rs` | ⏳ | |
| 7.4 Session memory — persist retrieved-chunk history per message | `migrations/015_add_message_citations.up.sql` [NEW] | ⏳ | |

---

## Sprint 8 — Research Agent (web-augmented, opt-in)

| Task | File | Status | Notes |
|---|---|---|---|
| 8.1 Local LLM plan decomposition (no network) | `services/research.rs` | ⏳ | |
| 8.2 Explicit user confirmation gate before network call | `commands/research.rs` | ⏳ | Hard privacy boundary |
| 8.3 Web search/fetch + synthesis with URL citations | `services/research.rs` | ⏳ | |
| 8.4 Log all external calls to audit_log | `services/audit.rs` | ⏳ | |

---

## Sprint 9 — Media Pipeline Integration

| Task | File | Status | Notes |
|---|---|---|---|
| 9.1 `services/media.rs` — transcribe → chunk by speaker turn → embed → index MediaSegment | `services/media.rs` | ⏳ | |
| 9.2 `media_segments` SQLite table | `migrations/016_create_media_segments.up.sql` [NEW] | ⏳ | |
| 9.3 Timestamp citations in chat UI | `src/pages/ChatPage.tsx` | ⏳ | |

---

## Sprint 10 — Security, Privacy & Compliance Hardening

| Task | File | Status | Notes |
|---|---|---|---|
| 10.1 PII detection over docs + outgoing research queries | `services/privacy.rs`, `security/pii_detector.rs` | ⏳ | |
| 10.2 Network allowlist enforcement | `security/network_monitor.rs` | ⏳ | |
| 10.3 Verify SQLite-at-rest AES-GCM + OS credential store | `security/encryption.rs` | ⏳ | |
| 10.4 Implement services/session.rs + services/settings.rs | `services/session.rs`, `services/settings.rs` | ⏳ | |
| 10.5 Wire compliance templates | `commands/security.rs` | ⏳ | |

---

## Sprint 11 — RAG Evaluation Harness

| Task | File | Status | Notes |
|---|---|---|---|
| 11.1 Golden test set (30-50 Q/A pairs) | `tests/rag_eval/` [NEW] | ⏳ | |
| 11.2 Retrieval metrics: recall@k, MRR | `tests/rag_eval/metrics.rs` [NEW] | ⏳ | |
| 11.3 Answer quality: faithfulness + citation accuracy | `tests/rag_eval/quality.rs` [NEW] | ⏳ | |
| 11.4 CLI runner + results in docs/ | `docs/eval-results.md` | ⏳ | |
| 11.5 CI gate at recall@8 >=0.85 (after baseline) | `.github/workflows/ci.yml` | ⏳ | |

---

## Sprint 12 — Packaging & Release

| Task | File | Status | Notes |
|---|---|---|---|
| 12.1 Bundle Weaviate binary + Python env (Windows first) | `src-tauri/tauri.conf.json` | ⏳ | |
| 12.2 First-run setup wizard | `SetupWizard.tsx` | ⏳ | Show total download size upfront |
| 12.3 Auto-update (Tauri updater, GitHub Releases) | `src-tauri/tauri.conf.json` | ⏳ | |
| 12.4 Windows code signing (self-signed; document SmartScreen) | CI signing config | ⏳ | |
| 12.5 Full offline test from clean VM | Manual verification | ⏳ | |

---

## Non-Functional Requirements Tracker

| Requirement | Target | Status |
|---|---|---|
| Cold start | < 5s (excl. first-run downloads) | ⏳ Not measured |
| Document ingestion (10-page PDF → searchable) | < 15s on mid-range laptop | ⏳ Not measured |
| Query → first token streamed | < 3s (local 7-9B quantized) | ⏳ Measure in Sprint 6 |
| Retrieval quality | recall@8 >=0.85 on golden test set | ⏳ Sprint 11 |
| Offline guarantee | Zero network calls after setup | ⏳ Verify in Sprint 4 |
| No data leaves device | Allowlist enforced at network layer | ⏳ Sprint 10 |
| Citations traceable to real chunk IDs | Required | ⏳ Sprint 7 |
| Crash resilience — mid-ingestion kill → idempotent resume | Required | 🔄 Designed in Sprint 3 |

---

## Open Decisions / Blockers

| # | Issue | Status |
|---|---|---|
| OD-1 | Embedding cache backend: SQLite table vs. sled/redb KV | ⏳ Decide in Sprint 5 |
| OD-2 | FastAPI vs. stdlib http.server for embedding_server.py | ⏳ Decide in Sprint 5 |
| OD-3 | HyDE default on/off in Settings | ⏳ Default OFF; document tradeoff in settings copy |
| OD-4 | External search backend for Research Agent (Sprint 8) | ❌ Need Aditya decision before Sprint 8 |
| OD-5 | Code signing certificate (EV vs. self-signed) for Sprint 12 | ❌ Need Aditya decision before Sprint 12 |
