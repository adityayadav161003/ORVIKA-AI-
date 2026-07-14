import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface LlmStatus {
  state: "stopped" | "starting" | "running" | "crashed";
  host: string;
  port: number;
  pid?: number;
  healthy: boolean;
  modelPath?: string;
  lastError?: string;
  binaryPath?: string;
}

interface LlmStore {
  status: LlmStatus | null;
  setStatus: (s: LlmStatus) => void;
  refresh: () => Promise<void>;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useLlmStore = create<LlmStore>((set) => ({
  status: null,

  setStatus: (status) => set({ status }),

  refresh: async () => {
    try {
      const status = await invoke<LlmStatus>("get_llm_status");
      set({ status });
    } catch {
      // browser mode — ignore
    }
  },
}));

// ─── Global event listener (initialised once in main.tsx) ────────────────────

let _unlistenLlmStatus: (() => void) | null = null;

export async function initLlmStore() {
  const store = useLlmStore.getState();
  await store.refresh();

  if (_unlistenLlmStatus) return; // already listening
  const unlisten = await listen<LlmStatus>("llm-runtime-status", (event) => {
    useLlmStore.getState().setStatus(event.payload);
  });
  _unlistenLlmStatus = unlisten;
}
