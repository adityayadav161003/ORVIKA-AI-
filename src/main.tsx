import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/globals.css";

interface TauriInternals {
  invoke: (cmd: string, args?: unknown) => Promise<unknown>;
  metadata: { iframe: boolean };
  listeners: Record<string, unknown>;
}

// Tauri browser mock polyfill
if (typeof window !== "undefined" && !("__TAURI_INTERNALS__" in window)) {
  const customWindow = window as unknown as { __TAURI_INTERNALS__: TauriInternals };
  customWindow.__TAURI_INTERNALS__ = {
    invoke: async (cmd: string, args?: unknown) => {
      console.warn(`[Browser Mock Mode] invoke("${cmd}") called with:`, args);
      if (cmd.startsWith("list_") || cmd === "get_messages" || cmd === "get_audit_logs") {
        return [];
      }
      if (cmd === "get_all_settings") {
        return {
          first_run_complete: "true", // Skip onboarding in browser preview
        };
      }
      if (cmd === "get_llm_status") {
        return { state: "stopped", host: "localhost", port: 11434, healthy: false };
      }
      if (cmd === "get_db_status") {
        return { path: "browser-in-memory", version: 14, walMode: false };
      }
      return null;
    },
    metadata: {
      iframe: false,
    },
    listeners: {}
  };
}

import { initLlmStore } from "./stores/llmStore";

// Start listening for LLM runtime events globally
void initLlmStore();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
