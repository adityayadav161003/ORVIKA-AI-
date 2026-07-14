# ORVIKA AI — Graphify Context File
# Purpose: Preserve full build context across chat sessions so nothing is lost.
# Load this file at the start of every new session to instantly restore state.

---

## Project Identity

- **Name:** ORVIKA AI
- **Type:** Local-first desktop research assistant
- **Stack:** Tauri 2 + React 18 + TypeScript (frontend) + Rust (backend) + Python sidecars
- **Repo root:** `c:\Users\adipi\Downloads\ORVIKA-AI\ORVIKA-AI`
- **Key spec:** `ORVIKA_AI_SPRINT_PLAN.md` — hand-off spec, must be read in full before starting work
- **Status tracker:** `ORVIKA_BUILD_STATUS.md` — live progress; update every time a task completes

---

## Architecture Summary

```
Frontend (src/)
  React 18 + TypeScript + TailwindCSS
  Pages: Chat, Documents, Media, Models, Research, Settings, Transparency
  State: Zustand stores (src/stores/)
  Router: react-router-dom (src/router.tsx)

Backend (src-tauri/src/)
  Rust + Tauri 2
  Commands (thin handlers) → Services (orchestration) → DB repos + sidecars

Sidecars (all on localhost, no external network):
  llama-server      127.0.0.1:8081  (LLM inference — REAL, Sprint 2 complete)
  weaviate          127.0.0.1:8079  (Vector DB — Sprint 4)
  embedding-server  127.0.0.1:8082  (FastAPI embed/rerank — Sprint 5)

Database:
  SQLite at %APPDATA%/com.orvika.app/data/app.db
  14 migrations exist (001–014); next will be 015

Python:
  Venv at %APPDATA%/com.orvika.app/python_venv
  Currently: per-call subprocess spawning (wrong arch, fix in Sprint 5)
  Scripts: src-tauri/python/ (parser.py, embedder.py, ocr_parser.py, requirements.txt)
```

---

## Critical Files Map

| What | Path |
|---|---|
| Rust entry | `src-tauri/src/lib.rs` |
| LLM sidecar (pattern to copy) | `src-tauri/src/llm/runtime.rs` |
| LLM model download (pattern to copy) | `src-tauri/src/llm/model_manager.rs` |
| Python manager (per-call subprocess) | `src-tauri/src/python/manager.rs` |
| DB connection + migrations | `src-tauri/src/db/connection.rs`, `src-tauri/src/db/migration.rs` |
| Chunk DB repo | `src-tauri/src/db/chunk_repo.rs` |
| Document command handler | `src-tauri/src/commands/documents.rs` |
| Vector store (JSON stub — replace) | `src-tauri/src/vector_store/store.rs` |
| Cargo.toml | `src-tauri/Cargo.toml` |
| Vite config | `config/vite.config.ts` |
| Migrations dir | `src-tauri/migrations/` |
| Python requirements | `src-tauri/python/requirements.txt` |
| System prompt | `models/prompts/default.md` |
| ADR dir | `docs/adr/` |

---

## What Is Real vs. Stub (as of session start 2026-07-14)

### REAL (do not rewrite):
- Tauri 2 + React 18 shell, pages, design system, routing
- SQLite schema (migrations 001–015): sessions, messages, documents, document_chunks, research_sessions, research_queries, audit_log, settings, api_keys, model_downloads, compliance_templates
- `llm/runtime.rs`, `llm/model_manager.rs`, `llm/hardware.rs`, `llm/inference.rs`, `llm/config.rs`, `llm/benchmark.rs`, `llm/types.rs`
- `python/manager.rs` (wrong architecture but functional for one-shot calls)
- All DB repos in `src-tauri/src/db/`
- All command handlers in `src-tauri/src/commands/` (thin, mostly wired)
- `services/audit.rs`, `services/sync.rs`, `services/document.rs` (real implementations)
- `document/types.rs`, `document/parser.rs`, `document/chunker.rs`, `document/ocr.rs` (Sprint 3 complete)

### STUBS (one line: `// Sprint stub`):
- `embedding/engine.rs`, `embedding/types.rs`
- `vector_store/search.rs`, `vector_store/types.rs`
- `services/chat.rs`, `services/research.rs`, `services/session.rs`, `services/settings.rs`, `services/privacy.rs`, `services/media.rs`
- `llm/context.rs`


### PROTOTYPE (functional but wrong for production):
- `vector_store/store.rs` — linear JSON scan, entire index in RAM, no hybrid, O(n). **Replace entirely in Sprint 4.**

---

## Sprint 13 (model marketplace) ── independent, can start any time after Sprint 2

## Product Vision (legal/healthcare — drives every architecture decision)

- **Zero data egress, no exceptions.** User documents never leave the desktop — not for embedding, not for inference, not for telemetry. Every external network call is a bug unless proven otherwise. The ONE exception: Research Agent (Sprint 8), opt-in per-query.
- **Primary market: legal + healthcare.** Case files, medical records, discovery material — confidential by nature. "Secrecy by architecture, not by policy."
- **Recall > precision.** A lawyer cannot miss a detail on page 340 of a deposition. Sprint 6/7 must support an "exhaustive mode" that surfaces ALL chunks above a relevance threshold (not fixed top-k) for compliance/discovery queries. Missing the 9th match is a liability, not a UX nitpick.
- **Citations are the trust mechanism.** Lawyers and clinicians need page-accurate, click-through citations to verify before relying on any answer. "Trust me" answers are not acceptable.

## Sprint Sequence & Dependencies

```
Sprint 3 (ingestion/chunking) ──► ✅ Done
     │
     ├──► Sprint 4 (Weaviate sidecar + schema + client) ← ACTIVE START
     │                    │
     └──► Sprint 5 (embedding server + reranker)
                          │
                    Sprint 6 (hybrid retrieval + query rewrite + rerank)
                          │
                    Sprint 7 (grounded chat — citations + refusal)
                          │
                    Sprint 8 (research agent — opt-in web, explicit gate)
                          │
Sprint 9 (media transcripts) ─── parallel with 6-8, depends on 4+5
Sprint 10 (security/privacy) ─── parallel with 6-9
                          │
                    Sprint 11 (RAG eval harness)
                          │
                    Sprint 12 (packaging + release)

```

---

## Key Design Decisions (Non-Negotiable per spec)

1. **Weaviate as vector store** — sidecar binary on 127.0.0.1:8079, no Docker, no Weaviate Cloud modules, `DEFAULT_VECTORIZER_MODULE=none`, `ENABLE_MODULES=""`. Data at `%APPDATA%/com.orvika.app/data/weaviate/`.
2. **No external network calls** by default — Research Agent web search is the ONLY exception, and it requires explicit per-query user confirmation.
3. **Weaviate client is handwritten** — thin `reqwest` wrapper for REST + GraphQL. No unmaintained community crate.
4. **Embedding model: `BAAI/bge-base-en-v1.5`** (768-dim) — replaces `all-MiniLM-L6-v2` (384-dim, weaker). Keep old as documented fallback.
5. **Reranker: `BAAI/bge-reranker-base`** — cross-encoder, loaded in embedding sidecar alongside embedder.
6. **SQLite is source of truth** — Weaviate is the vector index only. Always possible to rebuild Weaviate from SQLite. Chunks stored in both: SQLite for content+metadata, Weaviate for vectors + BM25 text copy + FK back to SQLite.
7. **Chunking strategy** — markdown-structure-first (not fixed-size), 400–600 tokens, 15% overlap within same section only (never across heading boundary), min 30 tokens (merge smaller into neighbor).
8. **Citations are mandatory** — every answer with document context must carry `[[chunk:id]]` markers resolving to real retrieved chunks. Hallucinated citations must be detected and stripped.
9. **HyDE** — optional, default OFF, toggleable in settings.
10. **Do NOT silently downgrade** — if something in the spec is impractical, stop and flag it explicitly rather than shipping a weaker version and calling it done.

---

## Weaviate Schema (implement exactly in Sprint 4)

### Collection: DocumentChunk
```
content            text     (BM25 indexed)
documentId         text     (filterable)
sqliteChunkId      text     (FK to document_chunks.id)
chunkIndex         int
pageNumber         int
sectionHeading     text
headingPath        text     (e.g. "Chapter 3 > Methodology > Data Collection")
sourceType         text     (enum: "document" | "media_transcript")
tokenCount         int
createdAt          date
Vector: dim=768 (bge-base-en-v1.5), vectorizer: none
```

### Collection: MediaSegment
```
content            text
documentId         text
sqliteSegmentId    text
startTimeSec       number
endTimeSec         number
speakerLabel       text     (nullable)
sourceType         text     (constant: "media_transcript")
createdAt          date
```

---

## Ports & Config

| Service | Host | Port | Data dir |
|---|---|---|---|
| llama-server | 127.0.0.1 | 8081 | %APPDATA%/com.orvika.app/bin/ |
| weaviate | 127.0.0.1 | 8079 | %APPDATA%/com.orvika.app/data/weaviate/ |
| embedding-server | 127.0.0.1 | 8082 | %APPDATA%/com.orvika.app/python_venv/ |
| SQLite | — | — | %APPDATA%/com.orvika.app/data/app.db |

---

## Active Session Notes

- **Session started:** 2026-07-14T09:58 IST
- **Current status:** Sprint 3 (Ingestion & Chunking) is 100% complete, fully verified by unit tests, compiled, and committed to git.
- **Active sprint being planned:** Sprint 4 (Weaviate Vector Store Integration)
- **Next milestone:** Implement Weaviate sidecar download/runtime launcher + client REST/GraphQL client


---

## How to Resume a Session

1. Read this file (`ORVIKA_GRAPHIFY_CONTEXT.md`) first — full context in one place.
2. Read `ORVIKA_BUILD_STATUS.md` — see exactly what's done and what's next.
3. Check git log for last commit to see what code was written.
4. Continue from the first `🔄 In Progress` or first `⏳ Not Started` task in the active sprint.
5. Never mark a sprint done without its acceptance criteria passing — test, don't assume.
