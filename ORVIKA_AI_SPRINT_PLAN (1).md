# ORVIKA AI — Advanced RAG Build Sprint Plan
**From scaffold to a production-grade, zero-data-egress research assistant for legal & healthcare**

Owner: Aditya · Prepared for: build agent (Claude Code / equivalent) · Repo: `ORVIKA-AI`

---

## 0. How to use this document

This is a **hand-off spec**, not a task list to skim. Give the agent this whole file plus repo access. Work sprint by sprint, in order — later sprints depend on earlier ones. Each sprint has:

- **Goal** — what "done" means in one sentence
- **Why it matters for accuracy** — this is an *advanced RAG* rebuild, not a CRUD app; every sprint should be justified by a retrieval/answer-quality reason, not just "make it work"
- **Tasks** — concrete, file-scoped
- **Acceptance criteria** — testable, not vibes
- **Definition of Done**
- **Depends on**

Do not let the agent mark a sprint done without the acceptance criteria passing. If the agent has to guess at a decision not covered here, it should stop and ask rather than improvise silently — especially anything touching encryption, data location, or network calls.

---

## 1. Product vision & positioning (read this before touching code)

This isn't a generic "chat with your PDFs" app. Two things drive every architecture decision in this document, and the agent should re-derive decisions from these when this spec doesn't cover a case:

**1. Zero data egress, no exceptions.** The user's documents never leave their desktop — not for embedding, not for inference, not for telemetry, not for crash reports. Every external network call in the entire product (the one exception being the explicitly opt-in Research Agent in Sprint 8) is a bug unless proven otherwise. This is why Weaviate runs as a local sidecar with `vectorizer: none` and cloud modules disabled (§3.2), why the embedding/rerank models run locally (Sprint 5), and why `security/network_monitor.rs` (Sprint 10) needs to be a real enforced allowlist, not a settings-page promise.

**2. Primary market: legal and healthcare.** Both fields sit on sensitive, often privileged/confidential documents (case files, medical records, discovery material) where the whole point is being able to interrogate a document without it — or even the fact that it's being reviewed — ever touching a third party's server. That's the sales pitch: secrecy by architecture, not by policy.

**What this changes about "accuracy":** the north star for retrieval quality in this product is not "helpful most of the time" — it's **"a person can miss a detail buried on page 340 of a deposition; the RAG must not."** That reframes a few things already in this plan:
- Recall matters more than precision when the two trade off. Sprint 11's recall@8 target (§4, §14) should be read as a floor, not a ceiling — for legal/healthcare use, consider an "exhaustive mode" in Sprint 6/7 that surfaces *all* chunks above a relevance threshold for a query, rather than a fixed top-k, when the user is doing compliance/discovery-style review rather than casual Q&A. Silently dropping a matching passage because it ranked 9th is a liability in these verticals, not just a UX nitpick.
- Citation grounding (Sprint 7) isn't a nice transparency feature here — it's the mechanism a lawyer or clinician needs to actually trust and verify an answer before relying on it. Every claim needs a page-accurate, click-through citation; "trust me" answers are not acceptable in this market.
- The marketing/positioning language (Sprint 14, and anything user-facing) should lead with the privacy/secrecy guarantee and the "won't miss what a human might" framing, not generic AI-assistant copy.

---

## 2. Ground truth: what's actually in the repo today

Before writing new code, the agent must know this isn't a blank slate — it's a well-organized **scaffold with the core engine missing**. Confirmed by direct inspection:

| Area | State |
|---|---|
| Tauri 2 + React 18 + TS shell, pages, design system, e2e specs | **Real, solid.** Don't rewrite. |
| SQLite schema (`sessions`, `messages`, `documents`, `document_chunks`, `research_sessions`, `research_queries`, `audit_log`, `settings`, `api_keys`, `model_downloads`, `compliance_templates`) | **Real**, via `sqlx`-style numbered migrations in `src-tauri/migrations/`. Keep and extend. |
| `llm/runtime.rs`, `llm/model_manager.rs`, `llm/hardware.rs` | **Real.** llama.cpp sidecar process, GPU detection, model download+checksum. This is the pattern to *copy* for the new Weaviate sidecar (see Sprint 4). |
| `python/manager.rs` + `python/*.py` | **Partially real, but wrong architecture.** Every call (`parse_document`, `embed_chunks`, `transcribe_audio`, `perform_ocr`) spawns a **fresh Python subprocess per invocation**, and `ensure_venv()` does a blocking `pip install` on first run. This works for a demo and will not survive real usage (model reload cost alone is 2–5s per call). Must become a persistent local server — see Sprint 5. |
| `vector_store/store.rs` | **A stub pretending to be a vector DB.** It's a linear-scan cosine similarity search over a single JSON file loaded fully into memory, with `Mutex` around the whole thing. ADR 003 claims FAISS; the code never used FAISS. This entire module is being replaced by Weaviate (Sprint 4). |
| `document/chunker.rs`, `document/parser.rs`, `document/ocr.rs`, `document/types.rs`, `embedding/engine.rs`, `embedding/types.rs`, `vector_store/search.rs`, `vector_store/types.rs`, `services/document.rs`, `services/chat.rs`, `services/research.rs`, `services/session.rs`, `services/settings.rs`, `services/privacy.rs`, `services/media.rs`, `llm/context.rs` | **All literally `// Sprint stub` — one line, no logic.** This is most of the actual product. |
| `docs/adr/003-faiss-vector-store.md` | Describes an architecture that was never built and is now superseded. Write ADR 004 in Sprint 4 to formally replace it. |

**Read this list back before starting.** The temptation for a coding agent is to see "vector_store/store.rs has 167 lines" and assume it's functional. It is not — it's a prototype that doesn't scale past a few hundred chunks and has no hybrid search, no filtering, no metadata-aware retrieval.

---

## 3. Architecture decision: Weaviate as the vector store

### 2.1 Why Weaviate (and what it buys you over the current store)

| Capability | Current (`vector_store/store.rs`) | Weaviate |
|---|---|---|
| Search | Brute-force cosine scan, O(n), whole index in RAM | HNSW ANN index, sub-linear |
| Retrieval mode | Vector-only | **Native hybrid search** (dense + BM25F, tunable `alpha`) — this alone is usually the single biggest accuracy win in a RAG system, because keyword/exact-term matches (names, IDs, exact phrases) are exactly what pure embedding similarity is bad at |
| Filtering | None | Structured `where` filters (by document, date, tags, page range) combined *with* vector search in one query |
| Multi-tenancy | N/A | Native per-session/per-workspace tenant isolation if you ever add multi-user |
| Persistence | One JSON file | LSM-tree backed, incremental writes, crash-safe |

### 2.2 How it stays "local-first" and private

Weaviate ships as a **single standalone binary** (no Docker required) that can run bound to `127.0.0.1` only, with on-disk persistence under the app's own data directory, and zero calls to any Weaviate Cloud service (that's an opt-in module you will not enable). This is architecturally identical to what `llm/runtime.rs` already does with `llama-server` — a sidecar binary, downloaded once, managed as a child process, health-checked over localhost HTTP.

**Bundle it the same way you bundle llama-server:**
- Ship/download the Weaviate binary for the target OS into `%APPDATA%/com.orvika.app/bin/weaviate(.exe)`
- Launch as a child process bound to `127.0.0.1:8079` (pick a fixed local port, document it in `llm/config.rs`-style config)
- Persist data at `%APPDATA%/com.orvika.app/data/weaviate/`
- Set env vars on the child process: `PERSISTENCE_DATA_PATH`, `DEFAULT_VECTORIZER_MODULE=none` (**critical** — you supply your own vectors from the local embedding model; do not let Weaviate call out to any external vectorizer), `ENABLE_MODULES=""` (empty — no cloud modules), `QUERY_DEFAULTS_LIMIT=25`, `CLUSTER_HOSTNAME=orvika-node1`
- No network egress: this process should never need to resolve any hostname other than `127.0.0.1`. Verify this explicitly in Sprint 4's acceptance test — run the app fully offline (disconnect network) after first setup and confirm ingestion + search still work end to end.

### 2.3 Schema (create this exact structure in Sprint 4)

Two Weaviate collections, vectors supplied externally (`vectorizer: none`):

```
Collection: DocumentChunk
Properties:
  content            text     (indexed for BM25 — this is what powers the keyword half of hybrid search)
  documentId         text     (filterable, not tokenized)
  sqliteChunkId      text     (foreign key back to document_chunks.id — Weaviate is the vector index, SQLite stays the source of truth for content + metadata)
  chunkIndex         int
  pageNumber         int
  sectionHeading     text
  headingPath        text     (breadcrumb, e.g. "Chapter 3 > Methodology > Data Collection")
  sourceType         text     (enum: "document" | "media_transcript")
  tokenCount         int
  createdAt          date
Vector: 1 per object, dimension = embedding model output (see Sprint 5 — 768 for bge-base-en-v1.5)
```

```
Collection: MediaSegment
Properties:
  content            text
  documentId         text     (media file treated as a document row too, for consistency)
  sqliteSegmentId    text
  startTimeSec       number
  endTimeSec         number
  speakerLabel       text     (nullable — only if diarization added later)
  sourceType         text     (constant: "media_transcript")
  createdAt          date
```

Keep `document_chunks` and a new `media_segments` table in SQLite as the durable source of truth (content, relationships, cascade deletes). Weaviate objects store a **copy** of `content` (needed for BM25) plus the FK back to SQLite — never make Weaviate the only place a chunk's text lives; you need to be able to fully rebuild the Weaviate index from SQLite at any time (this matters for migrations, corruption recovery, and reindexing after an embedding model upgrade).

### 2.4 Rust client approach

There is no first-party, fully-mature Rust client for Weaviate's current API. Do **not** pull in an unmaintained community crate as a black box. Instead:

- Write a small, explicit `weaviate_client.rs` wrapping `reqwest` calls to Weaviate's REST + GraphQL endpoints: `POST /v1/objects` (batch import), `POST /v1/batch/objects`, `POST /v1/graphql` (for hybrid search queries), `DELETE /v1/objects/{id}`, `GET /v1/.well-known/ready` (health check, mirrors `llm/runtime.rs`'s health check pattern).
- This keeps the surface area small, testable, and fully under your control — same philosophy as the existing `LlmRuntime`.

---

## 4. Non-functional requirements (the "exceptional" bar)

These apply across every sprint below. An agent should treat any of these as a blocking regression, not a nice-to-have.

| Requirement | Target |
|---|---|
| Cold start (app launch → ready to chat with existing index) | < 5s excluding first-run model downloads |
| Document ingestion (10-page PDF) → searchable | < 15s on a mid-range laptop, no GPU required |
| Query → first token streamed | < 3s (local 7–9B model, quantized) |
| Retrieval quality | See Sprint 11 eval harness (§13) — target ≥ 0.85 recall@8 as a floor; legal/healthcare "exhaustive mode" queries should be evaluated separately against near-100% recall on the golden set, since a missed passage is the failure mode this product exists to prevent |
| Offline guarantee | Zero network calls after model/binary setup, unless the user explicitly enables the Research Agent's web search for a specific query |
| No data leaves the device | Enforced at the network layer — see `security/network_monitor.rs` (already scaffolded), extend it to allowlist only `127.0.0.1` ports for llama-server + Weaviate, and explicitly-opted-in research endpoints |
| Every generated answer with document context | Must carry citations traceable to a real `document_chunks.id` — no answer should present retrieved-context claims without attribution |
| Crash resilience | Killing the app mid-ingestion must never corrupt SQLite or leave Weaviate in an inconsistent state (partial batches must be resumable/re-runnable, ingestion should be idempotent per document) |

---

## 5. Sprint 3 — Document Ingestion & Structure-Aware Chunking

**Goal:** Uploading a real PDF/DOCX/TXT/MD file produces high-quality, metadata-rich chunks in SQLite — no vectors yet.

**Why it matters for accuracy:** Chunking quality is the ceiling on everything downstream. Naive fixed-size chunking (what a first draft always defaults to) routinely splits sentences and separates a claim from the heading that gives it meaning. This sprint is where most "the AI gave a wrong/irrelevant answer" bugs actually originate, even though they get blamed on the LLM later.

**Tasks:**
1. `document/parser.rs` — implement using the existing `python/parser.py` (MarkItDown) via `PythonManager`, but return **structured** output: a list of `(heading_path: Vec<String>, page_number: Option<u32>, text: String)` blocks, not one flat string. Update `parser.py` to emit Markdown with page-break markers preserved (MarkItDown supports this for PDFs) so heading/page metadata survives.
2. `document/chunker.rs` — implement using the already-installed `text-splitter` crate (`features = ["markdown"]`, already in `Cargo.toml`):
   - Split by Markdown structure first (headers), not by raw character count
   - Target chunk size: 400–600 tokens, 15% overlap between adjacent chunks *within the same section* (do not overlap across a heading boundary — that's how you get chunks that are half-methodology, half-results, and confuse retrieval)
   - Every chunk carries: `heading_path` (breadcrumb from parser output), `page_number`, `start_char`/`end_char` offsets into the original document, `token_count` (use a real tokenizer count, not `.split_whitespace().len()`)
   - Reject/flag chunks under ~30 tokens (likely noise — stray headers, page numbers) — merge them into the neighboring chunk instead of indexing them standalone
3. `document/ocr.rs` — wire up existing `ocr_parser.py` for scanned PDFs/images; OCR output goes through the same chunker. Store the OCR confidence score in chunk metadata so low-confidence OCR text can be flagged in the UI later (a citation from a garbled OCR page should look different from a clean text extraction).
4. `document/types.rs` — define `ParsedBlock`, `Chunk` (mirrors `db::chunk_repo::NewChunk` but pre-embedding), `ChunkingConfig` (chunk size, overlap, min tokens — make these configurable, not hardcoded, since you'll want to tune them in Sprint 11).
5. `services/document.rs` — orchestrate: parse → chunk → `chunk_repo::insert_batch` → mark `documents.parsed_at`. This service is the thing `commands/documents.rs::upload_document` should call; move the inline logic currently living in the command handler into this service layer (commands should stay thin).

**Acceptance criteria:**
- Upload a 20-page PDF with headings; every resulting chunk in SQLite has a non-null `section_heading` for at least 90% of chunks (some intro/preamble text before the first heading is expected to lack one)
- No chunk crosses a top-level heading boundary
- Re-uploading the same file (same content hash) does not duplicate chunks — ingestion is idempotent
- Killing the process mid-parse of a 100-page document, then re-running ingestion, produces a correct, non-duplicated result

**Definition of Done:** Chunks visible in SQLite with correct metadata; no vectors yet (that's Sprint 4/5); unit tests for chunker boundary behavior (heading splits, overlap, short-chunk merging) in `src-tauri/src/document/chunker.rs` test module.

**Depends on:** nothing (can start immediately)

---

## 6. Sprint 4 — Weaviate Vector Store Integration

**Goal:** Weaviate runs as a managed sidecar, schema auto-provisions on first launch, and chunks (with placeholder/test vectors) can be written, filtered, and deleted.

**Tasks:**
1. `vector_store/types.rs` — define `WeaviateConfig` (host, port, data dir), `IndexableChunk` (chunk + vector + all Weaviate properties from §3.3), `SearchFilter` (documentId, sourceType, pageRange — maps to Weaviate `where` clauses).
2. New file `src-tauri/src/vector_store/sidecar.rs` — copy the architecture of `llm/runtime.rs` almost directly: `WeaviateRuntime` struct managing a `Child` process, `start()`/`stop()`/`health_check()` (poll `GET /v1/.well-known/ready`), binary resolution (download from GitHub releases the first time, checksum-verify, same pattern as `llm/model_manager.rs`).
3. New file `src-tauri/src/vector_store/schema.rs` — on first successful health check, `PUT`/`POST` the `DocumentChunk` and `MediaSegment` collection schemas from §3.3 if they don't already exist (idempotent — check `GET /v1/schema` first).
4. `vector_store/weaviate_client.rs` — thin REST/GraphQL client: `add_batch(chunks: Vec<IndexableChunk>)`, `delete_by_document(document_id)`, `hybrid_search(query_text, query_vector, filter, alpha, top_k)` (GraphQL, Sprint 6 will actually call this meaningfully), `count()`, `is_healthy()`.
5. Rewrite `vector_store/store.rs` → delete the JSON-file implementation entirely; `vector_store/mod.rs` now exposes the sidecar + client instead.
6. `services/document.rs` (extend from Sprint 3) — after chunking, once embeddings exist (Sprint 5), write to both SQLite (`chunk_repo::update_embedding_ids`, repurposed to store the Weaviate object UUID instead of a FAISS-style int) and Weaviate in the same transaction-adjacent flow; if the Weaviate write fails, the chunk row should be marked `is_indexed = 0` so it can be retried, not silently lost.
7. Write **ADR 004** (`docs/adr/004-weaviate-vector-store.md`) formally superseding ADR 003: document the sidecar approach, the "no external vectorizer, no cloud modules" decision, and the local-first guarantee. Mark ADR 003 as `Status: Superseded by ADR 004`.
8. Update `.github/workflows/release.yml` and packaging (Sprint 12 will finalize) to note the Weaviate binary needs bundling per-platform.

**Acceptance criteria:**
- On fresh app launch, Weaviate binary downloads (or is found if already present), starts, schema provisions — all observable via a status indicator (reuse the pattern already in the LLM tab's benchmark panel for the sidecar health)
- Killing the Weaviate process externally mid-session and issuing a query returns a clean, user-legible error (not a panic/crash) and the app offers to restart it
- Full offline test: disconnect network after first setup, restart app, ingest a new document, search — all succeeds with zero network calls (verify via `security/network_monitor.rs` or OS-level network monitoring during the test)
- Deleting a document cascades: SQLite `document_chunks` rows deleted (already works via `ON DELETE CASCADE`) **and** the corresponding Weaviate objects are deleted (this needs an explicit call — Weaviate doesn't know about your SQLite foreign keys)

**Definition of Done:** Round-trip write/read/delete against real Weaviate, integration test using a temp data dir + real sidecar process (not mocked — this is exactly the kind of integration bug that mocks hide).

**Depends on:** Sprint 3 (needs chunks to index)

---

## 7. Sprint 5 — Embedding Service Upgrade

**Goal:** Replace the per-call Python subprocess spawn with a persistent local embedding + reranking server, matching the performance philosophy already established by `llama-server`.

**Why it matters for accuracy *and* usability:** Sprint 6's re-ranking step needs a cross-encoder model loaded and warm — spawning a fresh Python process per rerank call (each with multi-second model load time) makes hybrid retrieval feel broken, not advanced. This also fixes the current `pip install` blocking the UI thread on first document upload.

**Tasks:**
1. New file `src-tauri/python/embedding_server.py` — a small FastAPI (or plain `http.server`) app, started once as a sidecar (same pattern as `llama-server` and the new Weaviate sidecar), exposing:
   - `POST /embed` — batch text → vectors, model: **`BAAI/bge-base-en-v1.5`** (768-dim, strong open-weights retrieval embedding, runs fine on CPU for reasonable batch sizes) — replace the currently-hardcoded `all-MiniLM-L6-v2` in `embedder.py` (384-dim, noticeably weaker retrieval quality; keep it only as a documented "low-resource fallback" option in settings)
   - `POST /rerank` — cross-encoder reranking, model: **`BAAI/bge-reranker-base`**, input: query + list of candidate chunk texts, output: relevance scores for re-sorting
   - `GET /health`
   - Both models loaded once at server start, kept warm in memory
2. `embedding/engine.rs` — replace stub; manages this server as a sidecar (again, mirror `llm/runtime.rs`: start/stop/health-check/port config), exposes `embed_batch(texts) -> Vec<Vec<f32>>` and `rerank(query, candidates) -> Vec<f32>` to the rest of the Rust codebase over HTTP.
3. `embedding/types.rs` — `EmbeddingRequest`/`Response`, `RerankRequest`/`Response`, `EmbeddingModelConfig` (model id, dimension, max sequence length — needed because bge-base truncates at 512 tokens; chunks from Sprint 3 must respect this).
4. Add an **embedding cache**: hash of chunk content → vector, stored in a small SQLite table or sled/redb key-value store, so re-ingesting an unchanged document (or re-embedding after a settings change) doesn't redundantly recompute vectors for identical text.
5. `python/manager.rs` — retire `embed_chunks()`'s per-call subprocess path; keep the venv-bootstrap logic (still needed for one-time setup) but redirect embedding calls through the new persistent server. Leave `parse_document`, `transcribe_audio`, `perform_ocr` on the existing subprocess-per-call pattern for now — those are one-shot, not latency-critical in a loop the way embedding/reranking are.
6. Update `src-tauri/python/requirements.txt` to pin `sentence-transformers`, add `fastapi`/`uvicorn` (or keep stdlib `http.server` if you want to avoid the extra dependency weight — agent's call, document the tradeoff).

**Acceptance criteria:**
- Embedding server survives across multiple document uploads without reloading models (verify via logged model-load timestamp — should appear exactly once per app session, not once per upload)
- Re-embedding an unchanged document is a cache hit and completes in milliseconds, not seconds
- `rerank()` returns scores in the same order requested, correctly correlated with the input candidates (write a unit test with an obviously-relevant vs. obviously-irrelevant pair and assert the relevant one scores higher)

**Definition of Done:** Benchmark panel (extend the existing LLM tab pattern) shows embedding throughput (chunks/sec) and rerank latency, so regressions are visible without digging into logs.

**Depends on:** Sprint 3 (chunks to embed), works in parallel with Sprint 4

---

## 8. Sprint 6 — Hybrid Retrieval, Re-ranking & Query Transformation

**Goal:** A single `retrieve(query, session_context, filters) -> Vec<RankedChunk>` function that is genuinely state-of-the-art for a local RAG system.

**Why it matters for accuracy:** This is the sprint that actually earns the phrase "advanced RAG." Everything before this was infrastructure; this is where retrieval quality is won or lost.

**Tasks:**
1. `vector_store/search.rs` — implement `hybrid_search`: call Weaviate's GraphQL `hybrid` operator with both the query vector (from Sprint 5's embedder) and raw query text, `alpha` configurable (default 0.5 — equal weight dense/sparse; expose in Settings for power users), `top_k = 20` candidates, plus any active `SearchFilter` (document scope, page range).
2. **Query transformation**, new `services/query_rewrite.rs`:
   - **Conversational rewriting**: if the user's message references prior turns ("what about the second one", "and in 2023?"), use the local LLM to rewrite it into a standalone query using the last N turns of chat history *before* retrieval. This alone fixes a huge class of "the RAG ignored my follow-up" bugs.
   - **HyDE (Hypothetical Document Embeddings)**, optional/configurable: for open-ended questions, have the local LLM generate a short hypothetical answer first, embed *that*, and use it alongside the literal query embedding for retrieval — measurably improves recall for questions phrased very differently from how the source document phrases the answer. Make this toggleable (adds latency; document the tradeoff in settings UI copy).
3. **Re-ranking step**: take the top 20 hybrid candidates, call `embedding::engine::rerank(query, candidate_texts)`, re-sort, keep top 5–8.
4. **Deduplication / diversity pass**: if two of the top chunks are near-duplicates (e.g., overlapping content from adjacent chunk boundaries), drop the lower-scored one and pull the next candidate up — avoid burning context budget on redundant text.
5. `llm/context.rs` — implement context assembly: given the final ranked chunks, build the prompt context block with clear per-chunk source tags (`[Source: filename.pdf, p.12]`), respecting the model's context window (truncate lowest-ranked chunks first if over budget, never silently truncate mid-chunk).
6. Expose a `research_plan`-style trace object (the frontend already has `ResearchPlanModal.tsx` scaffolded) showing: rewritten query → candidates retrieved → rerank scores → final chunks used. This transparency is a differentiator, not just a debug tool — surface it to the user via the existing Transparency page.

**Acceptance criteria:**
- A follow-up question ("what about page 5?") after an initial question correctly retrieves content from page 5 of the *same* document under discussion, not a generic top-match from the whole corpus
- Given a query using different vocabulary than the source document (e.g., asking about "revenue" when the document says "top-line income"), hybrid + rerank retrieves the relevant section where a naive keyword-only or embedding-only search in isolation would plausibly miss it — write this as an explicit regression test case
- Reranking measurably reorders the naive hybrid results in at least some test cases (if it never changes the order, something's wired wrong)

**Definition of Done:** This is the sprint that feeds Sprint 11's evaluation harness — don't mark it done from vibes; get at least a rough recall@8 number against a small hand-built test set before moving on.

**Depends on:** Sprint 4, Sprint 5

---

## 9. Sprint 7 — Grounded Chat Orchestration

**Goal:** `services/chat.rs` and `commands/chat.rs` turn retrieval into a trustworthy, cited, streaming answer.

**Tasks:**
1. `services/chat.rs` — orchestrate: rewrite query (Sprint 6) → retrieve (Sprint 6) → assemble context (Sprint 6) → build final prompt using `models/prompts/default.md`'s system prompt (already well-written — keep its "never fabricate sources" rule as a hard constraint, not just a suggestion) → stream via existing `llm/inference.rs` + Tauri IPC channel (already working from Sprint 2).
2. **Citation enforcement**: require the model to emit inline citation markers referencing chunk IDs (e.g. `[[chunk:abc123]]`) that the frontend renders as clickable footnotes linking to `DocumentViewer.tsx` at the right page. Post-generation, verify every citation marker corresponds to a chunk that was actually in the retrieved context — if the model invents a citation to a chunk that wasn't retrieved, strip it and log it (this is your hallucination tripwire).
3. **Refusal path**: if retrieval returns nothing above a relevance threshold (tune this empirically in Sprint 11), the system prompt should push the model to say so plainly rather than answer from general knowledge while implying it came from the user's documents — this distinction (documents vs. general knowledge) must be visually distinguishable in the chat UI, not just in the prose.
4. Session memory: persist retrieved-chunk history per message in `messages` table (extend schema — new migration `015_add_message_citations.up.sql`) so past citations remain clickable when reopening a session.

**Acceptance criteria:**
- Asking a question with no relevant documents uploaded produces an honest "I don't have that in your documents" response, not a confident hallucination
- Every citation in a rendered answer resolves to a real chunk, viewable at the correct page in `DocumentViewer.tsx`
- Streaming still works end-to-end (don't regress Sprint 2's work)

**Depends on:** Sprint 6

---

## 10. Sprint 8 — Research Agent (web-augmented, explicitly opt-in)

**Goal:** Implement `services/research.rs` / `commands/research.rs` behind the existing `ResearchPlanModal.tsx` UI: multi-step, user-approved web research that stays clearly separated from the private local RAG.

**Tasks:**
1. Plan step: local LLM decomposes the research question into sub-queries (this is local, no network yet).
2. **Explicit user confirmation gate** before any network call — surface the plan (already scaffolded in the modal) and require a click to proceed. This is a hard privacy boundary, not a UX nicety: the whole product's pitch is "your documents never leave your device," so anything that *does* leave the device must be unmistakably opt-in per use, not a global toggle that's easy to forget is on.
3. Execute web search/fetch (reuse whatever search backend you choose — this is the one place external network calls are legitimate) → synthesize with citations to URLs, kept clearly visually distinct from document-grounded citations (different citation style/color — never let a web citation look like a local-document citation).
4. Log every research session's external calls to `audit_log` (table already exists) for user transparency.

**Depends on:** Sprint 7

---

## 11. Sprint 9 — Media Pipeline Integration

**Goal:** Audio/video transcripts flow into the same Weaviate-backed retrieval as documents.

**Tasks:**
1. `services/media.rs` — orchestrate `transcribe_audio` (already works) → chunk transcript by speaker turn / time window (not raw character count — a transcript chunked mid-sentence across a speaker change is useless) → embed → index into the `MediaSegment` Weaviate collection.
2. New migration for `media_segments` SQLite table (mirrors `document_chunks`, with `start_time_sec`/`end_time_sec` instead of page numbers).
3. Citations for media answers should be clickable to a timestamp, jumping the (future) media player to that point — at minimum, display the timestamp clearly now even if seek-to-timestamp playback is a later polish item.

**Depends on:** Sprint 4, Sprint 5

---

## 12. Sprint 10 — Security, Privacy & Compliance Hardening

**Goal:** Make the privacy claims in the marketing copy actually true and verifiable, not just asserted.

**Tasks:**
1. `services/privacy.rs`, `security/pii_detector.rs` — implement PII detection over ingested documents (flag SSNs, emails, etc. in `DocumentViewer.tsx` — informational, not blocking) and over outgoing Research Agent queries (Sprint 8) — warn the user if a sub-query looks like it contains PII before it's sent externally.
2. `security/network_monitor.rs` — implement an actual allowlist enforcement: only `127.0.0.1` (llama-server, Weaviate, embedding server) plus explicitly-approved research endpoints (Sprint 8) may receive traffic; log and block anything else, surfaced in Settings.
3. `security/encryption.rs` — verify the SQLite-at-rest encryption story (the file already has real AES-GCM code from before the rebrand); confirm the master key handling via OS credential store (`security/credential.rs`, already implemented) actually gates access correctly on all three platforms.
4. `services/settings.rs`, `services/session.rs` — implement (currently stubs) — session CRUD, settings persistence backing the already-built `SettingsPage.tsx` and `settingsStore.ts`.
5. Compliance templates (`compliance_templates` table, `commands/security.rs`, `docs/compliance-guide.md`) — wire up whatever the admin/compliance-guide docs promise; audit whether those docs describe real implemented behavior or aspirational copy, and align one to the other.

**Depends on:** can run in parallel with Sprints 6–9, should complete before Sprint 12 (release)

---

## 13. Sprint 11 — RAG Evaluation Harness (do not skip this)

**Goal:** A repeatable, numeric answer to "did retrieval get better or worse this sprint?" — without this, every future tuning change (chunk size, alpha, rerank threshold) is a guess.

**Tasks:**
1. Build a small **golden test set**: 30–50 question/answer pairs against a handful of real test documents (mix of clean text PDFs, a scanned/OCR'd doc, a transcript). For each question, hand-label which chunk(s) should be retrieved.
2. Retrieval metrics: **recall@k** (is the right chunk in the top k?), **MRR** (how high is it ranked?) — computed directly, no LLM needed, fast to run on every change to chunking/retrieval code.
3. Answer-quality metrics: **faithfulness** (does the generated answer only claim things supported by retrieved context? — use the local LLM itself as a judge, asking it to check each sentence of the answer against the provided sources) and **citation accuracy** (do citations point to chunks that actually support the claim?).
4. Wire this into a CLI command (`cargo test --test rag_eval` or a small script) the agent runs after any retrieval-affecting change, and report the four numbers (recall@8, MRR, faithfulness, citation accuracy) as a simple table — track over time in `docs/` so regressions are visible in review.
5. Set the target from §4 (recall@8 ≥ 0.85) as a CI gate once the number is stable, not before — don't gate on a metric you haven't baselined yet.

**Depends on:** Sprint 6, 7 (needs the full pipeline to evaluate)

---

## 14. Sprint 12 — Packaging & Release

**Goal:** A real, installable, signed desktop app a stranger could download and run.

**Tasks:**
1. Bundle the Weaviate binary and embedding-server Python environment per platform (Windows first, per existing NSIS config in `tauri.conf.json`; macOS/Linux as stretch).
2. First-run setup wizard (`SetupWizard.tsx`, already scaffolded) should handle: llama model download, Weaviate binary fetch, Python venv + model downloads — with clear progress and total download size shown up front (these models are hundreds of MB to several GB; don't surprise the user mid-download).
3. Auto-update story (Tauri's updater plugin) — at minimum, an in-app "check for updates" that points to GitHub Releases.
4. Code signing for Windows (self-signed is fine for now if you don't have an EV cert yet — document this honestly, don't skip it silently, since unsigned installers trigger SmartScreen warnings users need to be told to expect).
5. Final full offline test from a clean VM: install, run setup wizard, ingest documents, chat, close, reopen, everything persists.

**Depends on:** everything above, functionally

---

## 15. Parallel/optional track — Marketing site + download page

(Separate from the desktop app build; can be worked on by a different session of the agent at any point, doesn't block the sprints above.) Covers: product landing page describing ORVIKA AI, feature explanation, download links pointing at GitHub Releases artifacts from Sprint 12, and clear "runs entirely on your device" messaging that's honest about what's implemented vs. planned at time of launch. Flag to me separately when you want to start this — it deserves its own focused pass rather than being squeezed in as an afterthought here.

---

## 16. Sprint 13 — In-App Model Marketplace

**Goal:** Turn the existing "download one of two hardcoded models" flow into a real marketplace: browse, filter, and download LLMs matched to the user's actual desktop hardware, with clear guidance rather than a bare file list.

**Why it matters here specifically:** part of the pitch to a solo/small-firm lawyer or clinician is that this runs entirely on hardware they already own — a laptop with no GPU, a workstation with a 4090, an M-series Mac. They shouldn't have to know what "Q4_K_M" or "VRAM" means to pick a model that will actually run well on their machine. This sprint is what makes that true.

**What already exists (don't rebuild it):** `llm/hardware.rs` detects NVIDIA GPUs via `nvidia-smi` and computes `recommended_gpu_layers`; `llm/model_manager.rs` handles download + SHA-256 checksum verification against `models/registry.json`; `ModelsPage.tsx` already renders a download UI with hardware info and progress. This sprint **extends** that foundation — it's real, just thin (2 models, NVIDIA-only detection).

**Tasks:**
1. `llm/hardware.rs` — extend `detect_hardware()` beyond NVIDIA-only:
   - Apple Silicon: detect via `sysctl`/`system_profiler`, report unified memory as the effective "VRAM" pool, set `backend: "metal"`
   - AMD: detect via `rocm-smi` if present, `backend: "rocm"`
   - CPU-only fallback: report total system RAM (not just "no GPU") — a 64GB CPU-only workstation can still run a well-chosen quantized model acceptably; the current fallback throws that information away
2. `models/registry.json` — expand from 2 entries into a real curated catalog, each entry gains:
   - `category` (general-purpose, long-context/document-heavy, coding, fast/lightweight) — no genuinely reliable open-weights "legal-tuned" or "medical-tuned" model exists as of early 2026 to my knowledge; **do not fabricate domain-tuned model claims in the UI** — instead, recommend long-context, high-instruction-following general models for legal/healthcare document work, and say so honestly in the model description rather than implying specialization that isn't real
   - `minRamGb` (for the CPU-only path, not just `minVramGb`)
   - `recommendedFor` (short human-readable line: "best default for most laptops", "best quality if you have a 16GB+ GPU", "fastest, lowest quality — for testing")
   - real `checksumSha256` values (currently empty strings — must be filled in before this ships; an empty checksum silently disables the verification `model_manager.rs` is supposed to do)
3. `ModelsPage.tsx` — rework into marketplace-style browsing: sort by "recommended for your machine" first (computed client-side from `HardwareInfo` + registry `min*` fields), visually distinguish "will run well" / "will run, slowly" / "not recommended for your hardware" rather than a flat list, and let the user override and download anyway if they want (never hard-block a download — just inform).
4. Keep the download flow's existing progress/cancel/checksum-verify behavior (`model_manager.rs`) — this sprint is about the catalog and recommendation logic, not re-plumbing the download mechanics.

**Acceptance criteria:**
- On a CPU-only machine (no GPU detected), the marketplace still shows sensible, clearly-labeled recommendations based on system RAM, instead of only showing GPU-tier models with no guidance
- Every model in the registry has a non-empty, verified checksum
- No UI copy claims a model has domain-specific (legal/medical) training unless that's actually true of the specific model listed

**Depends on:** Sprint 2 (existing LLM runtime — already done), can proceed independently of the RAG sprints (3–11)

---

## 17. Suggested order & rough sequencing

```
Sprint 3 (ingestion/chunking)
     │
     ├──► Sprint 4 (Weaviate)  ──┐
     │                            ├──► Sprint 6 (hybrid retrieval + rerank) ──► Sprint 7 (grounded chat) ──► Sprint 8 (research agent)
     └──► Sprint 5 (embedding)  ──┘                                                     │
                                                                                          ▼
Sprint 9 (media) ── parallel with 6–8, depends on 4+5                          Sprint 11 (eval harness)
Sprint 10 (security/privacy) ── parallel with 6–9                                        │
Sprint 13 (model marketplace) ── independent, can start any time after Sprint 2           ▼
                                                                                  Sprint 12 (packaging/release)
```

## 18. One instruction to the agent, explicitly

Do not silently downgrade any decision in this document because it's harder than a simpler alternative (e.g., swapping Weaviate back for a flat file "temporarily," or skipping the rerank step "for now"). If something here turns out to be genuinely impractical, stop and flag it back to Aditya with the specific blocker — don't quietly ship the easier, weaker version and call the sprint done.
