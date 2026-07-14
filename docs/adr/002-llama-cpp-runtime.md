# ADR 002: llama.cpp Runtime

**Status:** Accepted

## Context

Local LLM inference must run offline with GPU acceleration and CPU fallback.

## Decision

Use **llama.cpp** as a subprocess in server mode on localhost (Sprint 2).

## Consequences

- Process isolation for the LLM runtime
- Requires model download and checksum verification pipeline
- Backup options: candle, ort (documented as risks in PRD)
