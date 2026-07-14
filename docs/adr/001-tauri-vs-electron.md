# ADR 001: Tauri vs Electron

**Status:** Accepted

## Context

We need a cross-platform desktop runtime for a privacy-focused research application with Rust backend integration.

## Decision

Use **Tauri 2** for the desktop shell.

## Consequences

- Smaller binary size and lower memory footprint than Electron
- Native Rust backend for LLM, database, and security modules
- Steeper initial learning curve for Tauri 2 capabilities model
