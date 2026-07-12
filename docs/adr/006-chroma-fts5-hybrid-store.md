# ADR 006: Chroma + SQLite FTS5 Hybrid Store

**Status:** Accepted

## Context

Document RAG requires both semantic (dense) search to capture conceptual meaning and keyword (sparse) search to capture exact terms like docket numbers, drug names, patient IDs, and case citations. The previous plan specified FAISS or Weaviate, but Weaviate requires packaging a separate Go binary sidecar, while FAISS lacks metadata filtering and hybrid search capabilities.

## Decision

Use a hybrid retrieval architecture consisting of:

1. **Embedded ChromaDB** running inside the local Python sidecar process for dense vector retrieval. Anonymized telemetry is explicitly disabled (`anonymized_telemetry=False`) to satisfy the zero data egress guarantee.
2. **SQLite FTS5** virtual table `document_chunks_fts` for keyword retrieval. SQLite FTS5 requires zero external processes and operates in-process. Triggers are used to keep the virtual table in sync automatically with `document_chunks`.
3. **Reciprocal Rank Fusion (RRF)** in Rust to combine the ranked candidates from semantic and keyword searches using a standard constant $k=60$.

## Consequences

- Zero network dependency or local socket overhead for the keyword index.
- Perfect handling of exact terms via SQLite FTS5 alongside conceptual search via Chroma.
- Simpler packaging footprint since Chroma runs in-process inside the Python sidecar.
