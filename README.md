# ORVIKA AI

Local-first desktop research assistant built with **Tauri 2**, **React 18**, **TypeScript**, and **Rust**.

## Sprint status

**Sprint 1 (complete):** Tauri + React scaffold, design system, SQLite migrations, CI.

**Sprint 2 (in progress):** Local LLM via llama.cpp — see [docs/sprint-2-plan.md](docs/sprint-2-plan.md).

- `llama-server` subprocess on `127.0.0.1:8081`
- Model download + optional SHA-256 verification (`models/registry.json`)
- Streaming chat via Tauri IPC channel
- GPU detection and benchmark panel in the **LLM** tab

## Prerequisites

- **Node.js** 20+ and npm
- **Rust** 1.78+ ([rustup.rs](https://rustup.rs)) — add `%USERPROFILE%\.cargo\bin` to PATH
- **Windows:** [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++ workload (for Tauri)
- **llama-server** ([llama.cpp](https://github.com/ggerganov/llama.cpp)) on PATH or `LLAMA_SERVER_PATH`

## Getting started

```bash
npm install
npm run dev          # Vite dev server (browser)
npm run tauri dev    # Tauri desktop app
```

## Scripts

| Command               | Description                        |
| --------------------- | ---------------------------------- |
| `npm run dev`         | Start Vite dev server on port 1420 |
| `npm run tauri dev`   | Launch Tauri app with hot reload   |
| `npm run build`       | Build frontend to `dist/`          |
| `npm run tauri build` | Build production desktop installer |
| `npm run lint`        | ESLint                             |
| `npm run format`      | Prettier check                     |
| `npm run test`        | Vitest unit tests                  |

## Project structure

```
src/              React frontend (TypeScript)
src-tauri/        Rust backend (Tauri)
config/           Vite, Tailwind, TypeScript, rustfmt
docs/             Architecture docs and ADRs
models/           LLM registry and system prompts
.github/          CI/CD workflows
```

## Database

On first launch, SQLite is created at:

```
%APPDATA%/com.orvika.app/data/app.db
```

Migrations run automatically. WAL mode, foreign keys, and busy timeout are configured per the technical spec.

## Documentation

- `development.html` — Technical documentation suite
- `mimo.html` — Product requirements (PRD v2.0)
- `docs/adr/` — Architecture decision records

## License

Proprietary — see LICENSE.
