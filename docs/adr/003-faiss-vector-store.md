# ADR 003: FAISS Vector Store

**Status:** Superseded by [ADR 006](file:///c:/Users/adipi/Downloads/ORVIKA-AI/ORVIKA-AI/docs/adr/006-chroma-fts5-hybrid-store.md)

## Context

Document RAG requires fast similarity search over local embeddings.

## Decision

Use **FAISS** with on-disk persistence under `%APPDATA%/orvika-ai/data/vectors/`.

## Consequences

- Excellent search performance for local workloads
- Index dimension must match embedding model (384-dim for all-MiniLM-L6-v2)
