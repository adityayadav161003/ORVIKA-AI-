# ADR 007: Local AI Sidecar (Persistent Server)

**Status:** Accepted (supersedes ADR 005 for latency-critical tasks)

## Context

Spawning a fresh Python subprocess per embedding/reranking invocation (as decided in ADR 005) incurs a model load overhead of 2-5 seconds per call. For RAG indexing and search queries, this overhead is unacceptable and breaks user flow.

## Decision

Run a persistent local Python server using **FastAPI and Uvicorn** on port `8082`.

- Models (`BAAI/bge-base-en-v1.5` and `BAAI/bge-reranker-base`) are loaded once at startup and kept warm.
- Chroma PersistentClient is initialized once and reused across requests.
- Non-latency-critical tasks (MarkItDown parsing, audio transcription, OCR) remain as subprocess-per-call tasks to avoid server bloat.
- Rust side uses a persistent `EmbeddingEngine` sidecar manager (mirroring `LlmRuntime` sidecar architecture) to start, stop, and health-check the server.
- SQLite cache table `embedding_cache` is implemented in Rust to skip HTTP requests for identical text chunks.

## Consequences

- Ingestion and search performance improve from multiple seconds to single-digit milliseconds.
- Clean API separation between Rust and local python sidecar.
- One extra process to lifecycle manage.
