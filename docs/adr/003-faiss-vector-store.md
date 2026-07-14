# ADR 003: FAISS Vector Store

**Status:** Accepted

## Context

Document RAG requires fast similarity search over local embeddings.

## Decision

Use **FAISS** with on-disk persistence under `%APPDATA%/orvika-ai/data/vectors/`.

## Consequences

- Excellent search performance for local workloads
- Index dimension must match embedding model (384-dim for all-MiniLM-L6-v2)
