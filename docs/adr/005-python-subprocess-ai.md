# ADR 005: Python Subprocess AI

**Status:** Partially superseded by [ADR 007](file:///c:/Users/adipi/Downloads/ORVIKA-AI/ORVIKA-AI/docs/adr/007-local-ai-sidecar.md) (persistent FastAPI server for latency-critical tasks)

## Context

Document parsing, transcription, and embedding require Python ecosystem libraries (MarkItDown, Faster-Whisper, Sentence-Transformers).

## Decision

Run Python workloads as **JSON stdin/stdout subprocesses** orchestrated by Rust (Sprint 7+).

## Consequences

- Clean isolation between Rust and Python runtimes
- Requires bundling Python runtime for distribution
- IPC protocol defined in `python/shared/json_ipc.py`
