# ORVIKA AI — Advanced RAG Build Sprint Plan

**From scaffold to a production-grade, zero-data-egress research assistant for legal & healthcare**

Owner: Aditya · Prepared for: build agent (Claude Code / equivalent) · Repo: `ORVIKA-AI`

---

## 0. How to use this document

This is a **hand-off spec**, not a task list to skim. Give the agent this whole file plus repo access. Work sprint by sprint, in order — later sprints depend on earlier ones. Each sprint has:

- **Goal** — what "done" means in one sentence
- **Why it matters for accuracy** — this is an _advanced RAG_ rebuild, not a CRUD app; every sprint should be justified by a retrieval/answer-quality reason, not just "make it work"
- **Tasks** — concrete, file-scoped
- **Acceptance criteria** — testable, not vibes
- **Definition of Done**
- **Depends on**

Do not let the agent mark a sprint done without the acceptance criteria passing. If the agent has to guess at a decision not covered here, it should stop and ask rather than improvise silently — especially anything touching encryption, data location, or network calls.

---

## 1. Product vision & positioning (read this before touching code)

This isn't a generic "chat with your PDFs" app. Two things drive every architecture decision in this document, and the agent should re-derive decisions from these when this spec doesn't cover a case:

**1. Zero data egress, no exceptions.** The user's documents never leave their desktop — not for embedding, not for inference, not for telemetry, not for crash reports. Every external network call in the entire product (the one exception being the explicitly opt-in Research Agent in Sprint 9) is a bug unless proven otherwise. This is why Chroma runs embedded with telemetry disabled and no cloud modules (§3.2), why the embedding/rerank models run locally (Sprint 5), and why `security/network_monitor.rs` (Sprint 11) needs to be a real enforced allowlist, not a settings-page promise.

**2. Primary market: legal and healthcare.** Both fields sit on sensitive, often privileged/confidential documents (case files, medical records, discovery material) where the whole point is being able to interrogate a document without it — or even the fact that it's being reviewed — ever touching a third party's server. That's the sales pitch: secrecy by architecture, not by policy.

**What this changes about "accuracy":** the north star for retrieval quality in this product is not "helpful most of the time" — it's **"a person can miss a detail buried on page 340 of a deposition; the RAG must not."** That reframes a few things already in this plan:

- Recall matters more than precision when the two trade off. Sprint 12's recall@8 target (§4, §14) should be read as a floor, not a ceiling — for legal/healthcare use, consider an "exhaustive mode" in Sprint 6/7 that surfaces _all_ chunks above a relevance threshold for a query, rather than a fixed top-k, when the user is doing compliance/discovery-style review rather than casual Q&A. Silently dropping a matching passage because it ranked 9th is a liability in these verticals, not just a UX nitpick.
- Citation grounding (Sprint 7) isn't a nice transparency feature here — it's the mechanism a lawyer or clinician needs to actually trust and verify an answer before relying on it. Every claim needs a page-accurate, click-through citation; "trust me" answers are not acceptable in this market.
- The marketing/positioning language (Sprint 14, and anything user-facing) should lead with the privacy/secrecy guarantee and the "won't miss what a human might" framing, not generic AI-assistant copy.

---

## 2. Ground truth: what's actually in the repo today

Before writing new code, the agent must know this isn't a blank slate — it's a well-organized **scaffold with the core engine missing**. Confirmed by direct inspection:

| Area                                                                                                                                                                                                                                                                                                                  | State                                                                                                                                                                                                                                                                                                                                                                                                     |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tauri 2 + React 18 + TS shell, pages, design system, e2e specs                                                                                                                                                                                                                                                        | **Real, solid.** Don't rewrite.                                                                                                                                                                                                                                                                                                                                                                           |
| SQLite schema (`sessions`, `messages`, `documents`, `document_chunks`, `research_sessions`, `research_queries`, `audit_log`, `settings`, `api_keys`, `model_downloads`, `compliance_templates`)                                                                                                                       | **Real**, via `sqlx`-style numbered migrations in `src-tauri/migrations/`. Keep and extend.                                                                                                                                                                                                                                                                                                               |
| `llm/runtime.rs`, `llm/model_manager.rs`, `llm/hardware.rs`                                                                                                                                                                                                                                                           | **Real.** llama.cpp sidecar process, GPU detection, model download+checksum. This is the pattern that already informed how the embedding/rerank server is being run as a sidecar too (Sprint 5).                                                                                                                                                                                                          |
| `python/manager.rs` + `python/*.py`                                                                                                                                                                                                                                                                                   | **Partially real, but wrong architecture.** Every call (`parse_document`, `embed_chunks`, `transcribe_audio`, `perform_ocr`) spawns a **fresh Python subprocess per invocation**, and `ensure_venv()` does a blocking `pip install` on first run. This works for a demo and will not survive real usage (model reload cost alone is 2–5s per call). Must become a persistent local server — see Sprint 5. |
| `vector_store/store.rs`                                                                                                                                                                                                                                                                                               | **A stub pretending to be a vector DB.** It's a linear-scan cosine similarity search over a single JSON file loaded fully into memory, with `Mutex` around the whole thing. ADR 003 claims FAISS; the code never used FAISS. This entire module is being replaced by ChromaDB + SQLite FTS5 (Sprint 4).                                                                                                   |
| `document/chunker.rs`                                                                                                                                                                                                                                                                                                 | **Done — Sprint 3 shipped.** Structure-aware chunking is live; treat as real going forward, don't re-touch unless a later sprint's eval numbers say otherwise.                                                                                                                                                                                                                                            |
| `document/parser.rs`, `document/ocr.rs`, `document/types.rs`, `embedding/engine.rs`, `embedding/types.rs`, `vector_store/search.rs`, `vector_store/types.rs`, `services/chat.rs`, `services/research.rs`, `services/session.rs`, `services/settings.rs`, `services/privacy.rs`, `services/media.rs`, `llm/context.rs` | **All literally `// Sprint stub` — one line, no logic.** (`services/document.rs` shipped with Sprint 3.) This is most of the actual product.                                                                                                                                                                                                                                                              |
| `docs/adr/003-faiss-vector-store.md`                                                                                                                                                                                                                                                                                  | Describes an architecture that was never built and is now superseded. Write ADR 004 in Sprint 4 to formally replace it — now documenting ChromaDB, not Weaviate (see §3).                                                                                                                                                                                                                                 |

**Read this list back before starting.** The temptation for a coding agent is to see "vector_store/store.rs has 167 lines" and assume it's functional. It is not — it's a prototype that doesn't scale past a few hundred chunks and has no hybrid search, no filtering, no metadata-aware retrieval.

---

## 3. Architecture decision: ChromaDB (embedded) + SQLite FTS5 for hybrid retrieval

**Revised from the original plan, which specified Weaviate.** You asked to switch to Chroma for being local and fast — that instinct is right, but the actual reason to prefer it here isn't raw speed (see §3.1). Read this whole section before the agent implements it, because the switch changes the retrieval design, not just which database name appears in the code.

### 3.1 Why ChromaDB — and where the "Weaviate → Chroma" reasoning needs a correction

At the corpus size a single desktop user will realistically have (thousands to low hundreds-of-thousands of chunks), **Chroma and Weaviate are not meaningfully different in raw query speed** — both sit on an HNSW index under the hood, and at this scale either returns results in single-digit milliseconds. So "Chroma is faster" isn't really the win. The actual, correct reasons to prefer Chroma for this specific product are:

1. **It's embeddable in-process — no sidecar binary at all.** Weaviate requires bundling, downloading, checksumming, and lifecycle-managing a separate Go server binary (exactly like `llama-server`). Chroma's `PersistentClient` runs _inside the same Python process_ as the embedding/reranking server from Sprint 5. That's one fewer moving part to package (Sprint 13 gets simpler — no second binary to sign/bundle per OS), one fewer process to health-check, and one fewer thing that can be left running or crash independently.
2. **It matches "local storage" literally** — data lives as files under the app's data directory with zero server protocol overhead, which is a good fit for a single-user desktop app rather than a system built for distributed, multi-tenant, high-concurrency deployments (which is what Weaviate is actually optimized for, and which you don't need).

**The correction you need to hear:** Chroma does **not** have Weaviate's native, tunable hybrid search (dense + BM25F in one query). Out of the box, Chroma is a vector similarity engine — full-text/keyword matching is not its strength. For a legal/healthcare product whose whole pitch is "never miss a detail," dropping hybrid search would be a real accuracy regression, not a neutral swap — exact-term queries (a case citation, a statute number, a drug name, a patient ID) are precisely what pure embedding similarity handles worst, and precisely what these verticals search for constantly.

**The fix, which keeps the "local and fast" goal intact and actually strengthens the local-first story:** implement the keyword half yourself using **SQLite FTS5** — which you already have, for free, in the same SQLite database that stores `document_chunks`. No new dependency, no new process.

- Chroma → dense/semantic candidates (top-N by vector similarity)
- SQLite FTS5 (`document_chunks_fts`, a virtual table mirroring `document_chunks.content`) → sparse/keyword candidates (top-N by BM25, which FTS5 computes natively)
- Fuse the two ranked lists yourself in Rust using **Reciprocal Rank Fusion (RRF)** before handing candidates to the reranker (Sprint 5's cross-encoder)

This is arguably a _better_ fit for the zero-data-egress pitch than the original Weaviate plan, not just an equivalent swap: everything — vectors, keyword index, metadata — now lives in-process or in SQLite, with no local server socket involved in retrieval at all.

### 3.2 How it stays local-first and private

- Chroma's `PersistentClient(path=...)` writes to `%APPDATA%/com.orvika.app/data/chroma/`, inside the same embedding server process from Sprint 5 (see §7) — not a separate sidecar.
- No Chroma Cloud, no telemetry: explicitly set `anonymized_telemetry=False` in the Chroma client settings (Chroma's Python client defaults to sending anonymous usage telemetry — this **must** be disabled; leaving the default on would be a real violation of the zero-egress guarantee and should be treated as a P0 bug if the agent misses it).
- SQLite FTS5 requires no new process — it's a compile-time feature of SQLite (verify the `rusqlite` crate is built with the `bundled` feature and FTS5 enabled; `rusqlite = { version = "0.32", features = ["bundled"] }` is already in `Cargo.toml` — bundled builds include FTS5 by default, but confirm with a smoke test rather than assuming).

### 3.3 Data model

Keep `document_chunks` (SQLite) as the durable source of truth, exactly as in the original plan — that part of the design was correct and doesn't change with the storage swap.

```sql
-- New migration: 015_create_chunks_fts.up.sql
CREATE VIRTUAL TABLE document_chunks_fts USING fts5(
    content,
    content='document_chunks',
    content_rowid='rowid'
);
-- Triggers to keep FTS in sync on insert/update/delete of document_chunks
```

Chroma collection (created via the embedding server's Python process, not raw Rust):

```
Collection: "orvika_chunks"
  id: sqlite chunk id (string) — this is the join key back to SQLite, same role the Weaviate FK played
  embedding: vector, dim = embedding model output (768 for bge-base-en-v1.5, per Sprint 5)
  metadata: { documentId, chunkIndex, pageNumber, sectionHeading, headingPath, sourceType, tokenCount }
  document: chunk content (Chroma stores this too, for its own convenience — SQLite remains the canonical copy)
```

One Chroma collection is enough for both document chunks and media segments — use `metadata.sourceType` to distinguish rather than a second collection (simpler than the original two-collection Weaviate plan, and there's no operational reason to split them once there's no per-collection server overhead).

### 3.4 Access pattern from Rust

Rust doesn't talk to Chroma directly. It goes through the same local HTTP server that already hosts embedding and reranking (Sprint 5's `ai_server.py`), which gets three more endpoints:

- `POST /vector/add` — chunk id + embedding + metadata → Chroma `collection.add(...)`
- `POST /vector/query` — query embedding + `n_results` + optional `where` metadata filter → Chroma `collection.query(...)`
- `POST /vector/delete` — by `documentId` metadata filter → Chroma `collection.delete(where=...)`

`vector_store/types.rs` defines the Rust-side request/response structs; `vector_store/search.rs` (Sprint 6) is where the RRF fusion between this and FTS5 actually happens — see §8.

---

## 5. Sprint 3 — Document Ingestion & Structure-Aware Chunking

**Status: ✅ Complete, shipped.** Keeping the original spec below as the historical record of what "done" meant — don't re-derive requirements from scratch if this needs revisiting later; check here first.

**Goal:** Uploading a real PDF/DOCX/TXT/MD file produces high-quality, metadata-rich chunks in SQLite — no vectors yet.

**Why it matters for accuracy:** Chunking quality is the ceiling on everything downstream. Naive fixed-size chunking (what a first draft always defaults to) routinely splits sentences and separates a claim from the heading that gives it meaning. This sprint is where most "the AI gave a wrong/irrelevant answer" bugs actually originate, even though they get blamed on the LLM later.

**What shipped:** `document/parser.rs`, `document/chunker.rs`, `document/ocr.rs`, `document/types.rs`, `services/document.rs` — structure-aware Markdown chunking via `text-splitter`, heading-path/page-number metadata, idempotent re-ingestion, OCR wired for scanned docs.

**Depends on:** nothing — this was the starting point.

---

## 6. Sprint 4 — Chroma + SQLite FTS5 Hybrid Store Integration

**Goal:** Chroma runs embedded inside the (soon-to-exist) Python sidecar, SQLite FTS5 mirrors chunk content for keyword search, and chunks can be written, filtered, and deleted through both — with a real Reciprocal Rank Fusion function ready for Sprint 6 to call.

**Tasks:**

1. `vector_store/types.rs` — define `IndexableChunk` (chunk + vector + all Chroma metadata from §3.3), `SearchFilter` (documentId, sourceType, pageRange — maps to Chroma's `where` filter), `FusedCandidate` (chunk id, dense rank, sparse rank, fused RRF score).
2. Migration `015_create_chunks_fts.up.sql` — the FTS5 virtual table + sync triggers from §3.3. **Verify FTS5 is actually compiled into the bundled SQLite before writing a single line of Rust against it** — write a two-line smoke test (`CREATE VIRTUAL TABLE ... USING fts5(...)` against a throwaway in-memory connection) and confirm it doesn't error before assuming the feature is there.
3. New file `src-tauri/src/vector_store/chroma_client.rs` — thin `reqwest` client against the Sprint 5 sidecar's `/vector/add`, `/vector/query`, `/vector/delete` endpoints (§3.4). No GraphQL, no separate binary — this is a plain internal HTTP call to a server you already control.
4. New file `src-tauri/src/vector_store/fts_search.rs` — `search_bm25(query: &str, filter: SearchFilter, top_k: usize) -> Vec<(chunk_id, bm25_score)>` against `document_chunks_fts`.
5. New file `src-tauri/src/vector_store/fusion.rs` — pure function `reciprocal_rank_fusion(dense: Vec<(ChunkId, f32)>, sparse: Vec<(ChunkId, f32)>, k: f32) -> Vec<FusedCandidate>`, implementing `score(d) = Σ 1/(k + rank_i(d))` across whichever of the two lists contain `d` (standard constant `k ≈ 60`). This is pure and synchronous — no I/O — so it should be trivial to unit test with a hand-computed example (do that; it's the kind of function where an off-by-one in rank indexing is easy to miss silently).
6. Delete `vector_store/store.rs`'s JSON-blob implementation entirely; `vector_store/mod.rs` now exposes `chroma_client`, `fts_search`, and `fusion`.
7. `services/document.rs` (extend from Sprint 3) — after chunking + embedding (Sprint 5), write to SQLite (`chunk_repo::update_embedding_ids`, now storing a Chroma-side id rather than a FAISS-style int) and to Chroma via `chroma_client::add_batch`; if the Chroma write fails, mark the chunk `is_indexed = 0` for retry rather than losing it silently. FTS5 population happens automatically via the trigger from step 2 — no separate write path needed there.
8. Finalize **ADR 004** (`docs/adr/004-chroma-fts5-vector-store.md`) — §3 of this document is the content; the agent should transcribe/adapt it into ADR form and mark ADR 003 (FAISS) as `Status: Superseded by ADR 004`.

**Acceptance criteria:**

- Round-trip write → dense query → sparse query → delete, all correct, with a real embedded Chroma instance (not mocked)
- `reciprocal_rank_fusion` unit test: given two hand-constructed ranked lists with a known overlap, the fused order matches a hand-computed expectation
- An exact-term query (e.g., a specific docket number or drug name present in test documents) is found via the FTS5/BM25 path even when it scores poorly on pure embedding similarity — write this as an explicit regression case, it's the whole reason hybrid exists
- Full offline test: disconnect network after first setup, ingest a new document, search — succeeds with zero network calls
- Deleting a document cascades to both the SQLite chunk rows (already works) **and** the corresponding Chroma entries (`chroma_client::delete_by_document`) — Chroma doesn't know about your SQLite foreign keys, this needs an explicit call

**Definition of Done:** Integration test using a temp data dir + the real embedded Chroma client (via Sprint 5's sidecar), not a mock — this is exactly the kind of bug mocks hide.

**Depends on:** Sprint 3 (needs chunks to index), and functionally on Sprint 5 (the sidecar Chroma runs inside doesn't exist until then — build these two sprints in tandem if it's easier for the agent to do so, they're tightly coupled)

---

## 7. Sprint 5 — Local AI Sidecar (Embedding, Rerank & Chroma Index)

**Goal:** One persistent local Python server replaces both the per-call subprocess spawning _and_ hosts the embedded Chroma index — matching the performance philosophy already established by `llama-server`, and keeping the "no extra binary to bundle" promise from §3.1.

**Why it matters for accuracy _and_ usability:** Sprint 6's re-ranking step needs a cross-encoder model loaded and warm — spawning a fresh Python process per call (each with multi-second model load time) makes hybrid retrieval feel broken, not advanced. Folding Chroma into the same process means there's still only one extra sidecar to manage, not two.

**Tasks:**

1. New file `src-tauri/python/ai_server.py` (rename from the originally-planned `embedding_server.py` now that it does more than embedding) — a small FastAPI (or plain `http.server`) app, started once as a sidecar, exposing:
   - `POST /embed` — batch text → vectors, model: **`BAAI/bge-base-en-v1.5`** (768-dim) — replace the currently-hardcoded `all-MiniLM-L6-v2` in `embedder.py` (384-dim, noticeably weaker retrieval quality; keep it only as a documented "low-resource fallback" in settings)
   - `POST /rerank` — cross-encoder reranking, model: **`BAAI/bge-reranker-base`**
   - `POST /vector/add`, `POST /vector/query`, `POST /vector/delete` — thin wrappers around a module-level `chromadb.PersistentClient(path=..., settings=Settings(anonymized_telemetry=False))` (see §3.2 — **telemetry must be explicitly disabled**, treat leaving it on as a P0 bug)
   - `GET /health`
   - Embedding and rerank models loaded once at server start, kept warm; Chroma client opened once, reused across requests
2. `embedding/engine.rs` — replace stub; manages this server as a sidecar (mirror `llm/runtime.rs`: start/stop/health-check/port config), exposes `embed_batch(texts) -> Vec<Vec<f32>>` and `rerank(query, candidates) -> Vec<f32>` to the rest of the Rust codebase over HTTP. `vector_store/chroma_client.rs` (Sprint 4) talks to the same running server on its `/vector/*` routes.
3. `embedding/types.rs` — `EmbeddingRequest`/`Response`, `RerankRequest`/`Response`, `EmbeddingModelConfig` (model id, dimension, max sequence length — bge-base truncates at 512 tokens; Sprint 3's chunks must respect this).
4. **Embedding cache**: hash of chunk content → vector, in a small SQLite table, so re-ingesting an unchanged document doesn't redundantly recompute vectors.
5. `python/manager.rs` — retire `embed_chunks()`'s per-call subprocess path; keep venv-bootstrap logic (still needed for one-time setup) but redirect embedding/rerank/vector calls through the persistent server. Leave `parse_document`, `transcribe_audio`, `perform_ocr` on the existing subprocess-per-call pattern — those are one-shot, not latency-critical in a loop the way embedding/rerank/vector-search are.
6. Update `src-tauri/python/requirements.txt` — pin `sentence-transformers`, `chromadb`, add `fastapi`/`uvicorn` (or stdlib `http.server` if you want to avoid the extra weight — agent's call, document the tradeoff either way).

**Acceptance criteria:**

- Models and the Chroma client load exactly once per app session (logged timestamp), not once per upload or per query
- Re-embedding an unchanged document is a cache hit, milliseconds not seconds
- `rerank()` returns scores correctly correlated with input order (unit test: obviously-relevant vs. obviously-irrelevant pair, relevant one scores higher)
- Killing this sidecar externally mid-session and issuing a query returns a clean, user-legible error and the app offers to restart it (same standard as `llm/runtime.rs` already meets)

**Definition of Done:** Benchmark panel (extend the existing LLM tab pattern) shows embedding throughput, rerank latency, and vector query latency — regressions visible without digging into logs.

**Depends on:** Sprint 3 (chunks to embed); tightly coupled with Sprint 4 (build together, see note above)

---

## 8. Sprint 6 — Hybrid Retrieval (RRF), Re-ranking & Query Transformation

**Goal:** A single `retrieve(query, session_context, filters) -> Vec<RankedChunk>` function that is genuinely state-of-the-art for a local RAG system.

**Why it matters for accuracy:** This is the sprint that actually earns the phrase "advanced RAG." Everything before this was infrastructure; this is where retrieval quality is won or lost.

**Tasks:**

1. `vector_store/search.rs` — implement `hybrid_search`: run the dense query (`chroma_client::query`) and the sparse query (`fts_search::search_bm25`) **concurrently** (`tokio::join!`), each returning top ~30, then fuse via `fusion::reciprocal_rank_fusion` (Sprint 4) into a single ranked list, respecting any active `SearchFilter` (document scope, page range) applied to both sides before fusion.
2. **Query transformation**, new `services/query_rewrite.rs`:
   - **Conversational rewriting**: if the user's message references prior turns ("what about the second one", "and in 2023?"), use the local LLM to rewrite it into a standalone query using the last N turns of chat history _before_ retrieval. This alone fixes a huge class of "the RAG ignored my follow-up" bugs.
   - **HyDE**, optional/configurable: for open-ended questions, have the local LLM generate a short hypothetical answer first, embed _that_, and use it alongside the literal query embedding for the dense half of retrieval — improves recall for questions phrased very differently from the source document. Toggleable (adds latency; document the tradeoff in settings UI copy).
3. **Re-ranking step**: take the top ~20 fused candidates, call `embedding::engine::rerank(query, candidate_texts)`, re-sort, keep top 5–8 for normal queries — or, when the query is flagged as "exhaustive mode" (§1, routed through the orchestrator in Sprint 8), keep everything above a relevance floor instead of a fixed count.
4. **Deduplication / diversity pass**: drop near-duplicate chunks (overlapping adjacent-boundary content), pull the next fused candidate up instead — don't burn context budget on redundant text.
5. `llm/context.rs` — implement context assembly: per-chunk source tags (`[Source: filename.pdf, p.12]`), respecting the model's context window (truncate lowest-ranked chunks first if over budget, never mid-chunk).
6. Expose a trace object (the frontend already has `ResearchPlanModal.tsx` scaffolded, and the Transparency page exists) showing: rewritten query → dense candidates → sparse candidates → fused ranking → rerank scores → final chunks used. This transparency is a real differentiator for the legal/healthcare pitch (§1), not just a debug tool.

**Acceptance criteria:**

- A follow-up question ("what about page 5?") correctly retrieves content from page 5 of the _same_ document under discussion, not a generic top-match from the whole corpus
- An exact-term query (case citation, drug name, ID number) is retrieved correctly via the sparse/FTS5 side even in cases where dense-only similarity would plausibly rank it low — explicit regression test
- A paraphrased query ("revenue" vs. the document's "top-line income") is retrieved correctly via the dense side even when sparse/BM25 alone would miss it — the complementary regression test to the one above
- Reranking measurably reorders the fused results in at least some test cases

**Definition of Done:** Feeds Sprint 12's evaluation harness directly — get at least a rough recall@8 number against a small hand-built test set before calling this sprint done.

**Depends on:** Sprint 4, Sprint 5

---

## 9. Sprint 7 — Grounded Chat Orchestration

**Goal:** `services/chat.rs` and `commands/chat.rs` turn retrieval into a trustworthy, cited, streaming answer.

**Tasks:**

1. `services/chat.rs` — orchestrate: rewrite query (Sprint 6) → retrieve (Sprint 6) → assemble context (Sprint 6) → build final prompt using `models/prompts/default.md`'s system prompt (keep its "never fabricate sources" rule as a hard constraint) → stream via existing `llm/inference.rs` + Tauri IPC channel (already working from Sprint 2).
2. **Citation enforcement**: require inline citation markers referencing chunk IDs (e.g. `[[chunk:abc123]]`) rendered as clickable footnotes linking to `DocumentViewer.tsx` at the right page. Post-generation, verify every citation marker corresponds to a chunk actually in the retrieved context — strip and log any invented citation (your hallucination tripwire).
3. **Refusal path**: if retrieval returns nothing above a relevance threshold, the model should say so plainly rather than answer from general knowledge while implying it came from the user's documents — this distinction must be visually distinguishable in the chat UI, not just in the prose.
4. Session memory: persist retrieved-chunk history per message (new migration `016_add_message_citations.up.sql`) so past citations stay clickable when reopening a session.

**Acceptance criteria:**

- No-relevant-documents case produces an honest "not in your documents" response, not a confident hallucination
- Every citation resolves to a real chunk, viewable at the correct page
- Streaming still works end-to-end (don't regress Sprint 2)

**Depends on:** Sprint 6

---

## 10. Sprint 8 — Agent Orchestrator, Personalization & Scoped BYOK

**Goal:** A small, bounded, _auditable_ router that decides how each query is handled (standard retrieval / exhaustive scan / external research), plus a local-only personalization layer that adapts tone and defaults over time without ever touching what gets retrieved, plus a hard, code-level boundary around the BYOK cloud key so it can never reach the document pipeline.

**Read this before building it — where "just add agents" needs a correction:** "Agent" and "orchestrator" are doing a lot of work in the request that led to this sprint, and it's worth being precise about what that should and shouldn't mean here. An open-ended agent loop — where an LLM decides at runtime which tools to call, in what order, how many times, until it decides it's done — is genuinely useful for something like the Research Agent (Sprint 9), where the task is inherently open-ended (decompose a question, search, read, maybe search again). It is the **wrong** model for the core private document RAG path, for a reason specific to this product's own positioning (§1): a lawyer or clinician needs to be able to know _why_ an answer came out the way it did, deterministically, every time. A free-roaming agent that sometimes decides to skip retrieval, or re-queries three times, or takes a different path on a re-run of the identical question, is a liability in that context, not a feature. So:

- **The orchestrator is a small, fixed decision function, not an autonomous agent.** It classifies the incoming query (using the local LLM as a classifier only, not as a planner-with-tool-loop) into one of a small closed set of paths: `standard_qa`, `exhaustive_scan`, `research_agent` (only if the user explicitly invokes it — see Sprint 9). New file `services/orchestrator.rs`. Given the same query and the same document state, it should route the same way every time — that determinism is the point.
- Implement this _before_ wiring the Research Agent behind it, so Sprint 9 becomes "add one more path to an existing router" rather than the router being built around one agent's needs.

**Personalization — local-only, and bounded in one specific way that matters:**

1. New tables: `user_profile` (key/value store of learned preferences — default answer length, default citation density, glossary terms) and `feedback_events` (thumbs up/down per message, which citations got opened, timestamped) — both plain SQLite, both 100% local, both exportable and clearable from Settings (this is a compliance-relevant transparency feature, not just a nice-to-have, given §1's positioning).
2. Signals to learn from over time: explicit feedback (thumbs up/down), implicit feedback (does the user open/expand citations, or ignore them — a proxy for whether the answer's detail level matches what they wanted), a **personal glossary** built from recurring domain terms/abbreviations across _this user's own_ document corpus (e.g., if their documents consistently use "MI" to mean a specific term in context, that can inform query expansion in Sprint 6's rewrite step).
3. **The hard guardrail, stated explicitly because it's the easiest thing to get subtly wrong:** personalization may only change _how_ an answer is delivered — length, tone, default citation verbosity, which document set is searched by default — and must **never** influence which chunks get retrieved or how they're ranked in a way that could cause a relevant passage to be dropped. Concretely: the personalization layer must not feed into `vector_store/search.rs`'s candidate selection or the reranker's scoring at all. It sits entirely on the generation side, after retrieval is complete. Exhaustive mode (§1, §8-of-Sprint-6) must be fully personalization-blind — always. Write this as an explicit test: two "different" simulated user profiles issuing the identical exhaustive-mode query against the identical document set must retrieve byte-identical chunk sets.

**Scoped BYOK — formalizing and hardening what's already partly built:** `db/api_key_repo.rs` (AES-256-GCM encrypted storage), `cloud/traits.rs` + `cloud/{claude,openai,gemini}.rs`, and `commands/research.rs`'s existing PII-sanitization-before-outbound-call flow are already real, working code — this sprint isn't starting from zero. What's missing is an _enforced_ boundary, not just an intended one:

1. Add a repo-level check (a simple grep-based test run in CI is entirely sufficient — this doesn't need anything fancy) asserting `crate::cloud::` is never imported from `document/`, `embedding/`, `vector_store/`, or `services::chat`. The design intention already keeps these separate; this makes it structurally impossible for a future change to quietly wire a document query into a cloud call without the test failing loudly.
2. Surface, in Settings, exactly which provider (if any) is configured, and add a persistent, unmissable UI indicator whenever a BYOK cloud call is about to fire — not just an audit log entry after the fact (the log is good and should stay, but it's not a substitute for in-the-moment visibility).
3. **A real bug to fix while touching this code, not a hypothetical one:** `cloud/claude.rs`'s current `execute_query` doesn't perform web research at all — it just asks the cloud model to "perform comprehensive web-like research on the following topic" from its own training data and calls that "research." That's not research, it's a relabeled model recall, and it can confidently return stale or fabricated information while looking like a sourced result. This needs a real web search + fetch step (an actual search API call) _before_ the cloud LLM is invoked, so the cloud LLM's job becomes synthesis of real, fetched sources — not standing in for a search engine it isn't. Fix this as part of Sprint 9, but flag it here because it's a correctness issue in code that already exists, not just a gap.

**Acceptance criteria:**

- Same query, same documents, same session → same orchestrator routing decision, every time (determinism test)
- Deleting/clearing personalization data from Settings actually removes it and reverts behavior to defaults, verifiably
- The exhaustive-mode identical-retrieval test from above passes
- CI fails if `cloud::` is imported anywhere in the document/embedding/retrieval/chat modules
- No BYOK cloud call happens without the in-the-moment UI indicator firing

**Depends on:** Sprint 7 (needs a working chat pipeline to route into)

---

## 11. Sprint 9 — Research Agent (web-augmented, explicitly opt-in)

**Goal:** Implement the actual web-research path behind `ResearchPlanModal.tsx`, as one path the Sprint 8 orchestrator can route into — real web search, not a relabeled model-recall call.

**Tasks:**

1. Plan step (local, no network): local LLM decomposes the research question into sub-queries — this part of `commands/research.rs` already exists and works; move its logic into `services/research.rs` (currently a stub) so the command handler stays thin.
2. **Explicit user confirmation gate** before any network call — surface the plan and require a click to proceed, every time, not behind a global "always allow" toggle. This is a hard privacy boundary (§1), not a UX nicety.
3. **Fix the core issue flagged in Sprint 8:** add a real web search call (a search API — pick one that fits your budget/BYOK model) and fetch the actual result pages _before_ handing anything to the cloud LLM from `cloud/{claude,openai,gemini}.rs`. The cloud call's job becomes "synthesize these fetched sources," with real URLs to cite — not "tell me what you already know about this topic."
4. Citations to real URLs, kept visually distinct from document-grounded citations (different style/color — never let a web citation look like a local-document citation).
5. Log every research session's external calls to `audit_log` (table already exists, `log_cloud_call` already implemented) for user transparency.

**Depends on:** Sprint 8 (orchestrator + BYOK hardening)

---

## 12. Sprint 10 — Media Pipeline Integration

**Goal:** Audio/video transcripts flow into the same Chroma + FTS5 hybrid retrieval as documents.

**Tasks:**

1. `services/media.rs` — orchestrate `transcribe_audio` (already works) → chunk transcript by speaker turn / time window (not raw character count) → embed → index into the same Chroma collection (`sourceType: "media_transcript"`, §3.3) and mirror into `document_chunks_fts` via the same trigger mechanism (media segments should live in a `document_chunks`-shaped table, or a parallel `media_segments` table with its own FTS5 mirror — agent's call, but keep the retrieval code path unified rather than forking hybrid search into a document version and a media version).
2. New migration for `media_segments` (mirrors `document_chunks`, with `start_time_sec`/`end_time_sec` instead of page numbers).
3. Citations for media answers clickable to a timestamp; at minimum, display the timestamp clearly even before seek-to-timestamp playback exists.

**Depends on:** Sprint 4, Sprint 5

---

## 13. Sprint 11 — Security, Privacy & Compliance Hardening

**Goal:** Make the privacy claims in the marketing copy actually true and verifiable, not just asserted.

**Tasks:**

1. `services/privacy.rs`, `security/pii_detector.rs` — PII detection over ingested documents (flag SSNs, emails, etc. in `DocumentViewer.tsx`) and over outgoing Research Agent queries (Sprint 9) — this second half already partly exists via `pii_detector::sanitize_query` in `commands/research.rs`; extract/harden into the service layer.
2. `security/network_monitor.rs` — real allowlist enforcement: only `127.0.0.1` (llama-server, the Sprint 5 AI sidecar) plus explicitly-approved research endpoints (Sprint 9) may receive traffic; log and block anything else, surfaced in Settings. Note there's no separate Weaviate port to allowlist anymore — the sidecar footprint is smaller under the Chroma approach (§3.1), which simplifies this sprint slightly.
3. `security/encryption.rs` — verify the SQLite-at-rest AES-GCM story end to end; confirm `security/credential.rs`'s OS-credential-store master key handling gates access correctly on all three platforms.
4. `services/settings.rs`, `services/session.rs` — implement (currently stubs) — session CRUD, settings persistence backing `SettingsPage.tsx`/`settingsStore.ts`.
5. Compliance templates (`compliance_templates` table, `commands/security.rs`, `docs/compliance-guide.md`) — audit whether those docs describe real implemented behavior or aspirational copy, and align one to the other.

**Depends on:** can run in parallel with Sprints 6–10, should complete before Sprint 13 (release)

---

## 14. Sprint 12 — RAG Evaluation Harness (do not skip this)

**Goal:** A repeatable, numeric answer to "did retrieval get better or worse this sprint?" — without this, every future tuning change (chunk size, RRF constant, rerank threshold) is a guess.

**Tasks:**

1. Golden test set: 30–50 question/answer pairs against real test documents (clean text PDF, a scanned/OCR'd doc, a transcript). Hand-label which chunk(s) should be retrieved per question.
2. Retrieval metrics: recall@k, MRR — computed directly, fast to run on every retrieval-affecting change.
3. Answer-quality metrics: faithfulness (local LLM as judge, checking each answer sentence against retrieved context) and citation accuracy.
4. A personalization-specific regression check, given Sprint 8's guardrail: run the golden set once with a "blank" personalization profile and once with a synthetically populated one, and assert retrieval-side metrics (recall@k, MRR) are identical — only generation-side qualities (tone, length) should differ.
5. Wire into a CLI command/script reporting recall@8, MRR, faithfulness, citation accuracy as a simple table, tracked over time in `docs/`.
6. Set §4's target (recall@8 ≥ 0.85 general floor, near-100% for exhaustive-mode queries) as a CI gate once the number is stable, not before.

**Depends on:** Sprint 6, 7 (needs the full pipeline to evaluate), benefits from Sprint 8 existing (for the personalization regression check)

---

## 15. Sprint 13 — Packaging & Release

**Goal:** A real, installable, signed desktop app a stranger could download and run.

**Tasks:**

1. Bundle the Sprint 5 Python AI sidecar (torch, sentence-transformers, chromadb, and dependencies) per platform — this is the heavier packaging problem now (vs. the originally-planned separate Weaviate binary), since it's a full Python environment, not a single Go binary. Investigate PyInstaller (or similar) to produce a self-contained executable rather than requiring users to have Python installed — treat this as a real, non-trivial task, not a footnote.
2. First-run setup wizard (`SetupWizard.tsx`, already scaffolded): llama model download, Python AI sidecar environment + model downloads (bge-base, bge-reranker) — clear progress and total download size shown up front.
3. Auto-update story (Tauri's updater plugin) — at minimum, in-app "check for updates" pointing to GitHub Releases.
4. Code signing for Windows (self-signed acceptable for now if no EV cert yet — document honestly, don't skip silently; unsigned installers trigger SmartScreen warnings users need to be warned about).
5. Final full offline test from a clean VM: install, setup wizard, ingest documents, chat (standard + exhaustive mode), close, reopen, everything persists including personalization data.

**Depends on:** everything above, functionally

---

## 16. Parallel/optional track — Marketing site + download page

(Separate from the desktop app build; can be worked on at any point, doesn't block the sprints above.) Product landing page describing ORVIKA AI, feature explanation, download links pointing at GitHub Releases artifacts from Sprint 13, and clear "runs entirely on your device" messaging honest about what's implemented vs. planned at launch. Flag separately when you want to start this.

---

## 17. Sprint 14 — In-App Model Marketplace

**Goal:** Turn the existing "download one of two hardcoded models" flow into a real marketplace: browse, filter, and download LLMs matched to the user's actual desktop hardware, with clear guidance rather than a bare file list.

**Why it matters here specifically:** part of the pitch to a solo/small-firm lawyer or clinician is that this runs entirely on hardware they already own. They shouldn't have to know what "Q4_K_M" or "VRAM" means to pick a model that'll run well on their machine.

**What already exists (don't rebuild it):** `llm/hardware.rs` detects NVIDIA GPUs via `nvidia-smi`; `llm/model_manager.rs` handles download + checksum verification against `models/registry.json`; `ModelsPage.tsx` already renders a download UI. This sprint extends that — it's real, just thin (2 models, NVIDIA-only detection).

**Tasks:**

1. `llm/hardware.rs` — extend beyond NVIDIA-only: Apple Silicon (`sysctl`/`system_profiler`, unified memory as effective VRAM, `backend: "metal"`), AMD (`rocm-smi`, `backend: "rocm"`), CPU-only fallback reporting total system RAM instead of just "no GPU."
2. `models/registry.json` — expand into a real curated catalog: `category`, `minRamGb`, `recommendedFor`, real (non-empty) `checksumSha256` values. **Do not fabricate domain-tuned (legal/medical) model claims** — no genuinely reliable open-weights model of that kind exists as of early 2026 to my knowledge; recommend long-context, strong-instruction-following general models instead and say so honestly.
3. `ModelsPage.tsx` — marketplace-style browsing: "recommended for your machine" sorted first, "will run well"/"will run, slowly"/"not recommended" labeling, never hard-blocking a download.
4. Keep the existing download/checksum/progress mechanics (`model_manager.rs`) — this sprint is catalog and recommendation logic, not re-plumbing.

**Acceptance criteria:**

- CPU-only machines get sensible, clearly-labeled RAM-based recommendations, not just GPU-tier models with no guidance
- Every registry model has a verified, non-empty checksum
- No UI copy claims domain-specific training that isn't real

**Depends on:** Sprint 2 (existing LLM runtime), independent of the RAG sprints

---

## 18. Suggested order & rough sequencing

```
Sprint 3 (✅ done)
     │
     ├──► Sprint 4 + Sprint 5 (Chroma/FTS5 + AI sidecar — build together) ──┐
     │                                                                        ├──► Sprint 6 (RRF hybrid + rerank) ──► Sprint 7 (grounded chat)
     │                                                                        │              │
     │                                                                        │              ▼
     │                                                                        │      Sprint 8 (orchestrator + personalization + BYOK hardening)
     │                                                                        │              │
     │                                                                        │              ▼
     │                                                                        │      Sprint 9 (research agent — real web search)
     │                                                                        │
     ├──► Sprint 10 (media) ── parallel with 6–9, depends on 4+5
     ├──► Sprint 11 (security/privacy) ── parallel with 6–9
     └──► Sprint 14 (model marketplace) ── independent, any time after Sprint 2

                                                                    Sprint 12 (eval harness) ── depends on 6, 7, benefits from 8
                                                                              │
                                                                              ▼
                                                                    Sprint 13 (packaging/release)
```

---

## 19. One instruction to the agent, explicitly

Do not silently downgrade any decision in this document because it's harder than a simpler alternative — swapping Chroma back for a flat file "temporarily," skipping the rerank step "for now," or building the orchestrator as an open-ended agent loop because it's easier than a bounded router. If something here turns out to be genuinely impractical, stop and flag it back to Aditya with the specific blocker — don't quietly ship the easier, weaker version and call the sprint done.
