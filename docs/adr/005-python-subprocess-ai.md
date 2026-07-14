# ADR 005: Python Subprocess AI

**Status:** Accepted

## Context

Document parsing, transcription, and embedding require Python ecosystem libraries (MarkItDown, Faster-Whisper, Sentence-Transformers).

## Decision

Run Python workloads as **JSON stdin/stdout subprocesses** orchestrated by Rust (Sprint 7+).

## Consequences

- Clean isolation between Rust and Python runtimes
- Requires bundling Python runtime for distribution
- IPC protocol defined in `python/shared/json_ipc.py`
