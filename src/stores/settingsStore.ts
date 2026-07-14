import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

// ─── Types ────────────────────────────────────────────────────────────────────

export type PrivacyLevel = "strict" | "balanced" | "open";
export type Theme = "light" | "dark" | "system";

export interface AppSettings {
  privacyLevel: PrivacyLevel;
  theme: Theme;
  inferenceThreads: number;
  gpuLayers: string; // "auto" or a number string
  gpuContextSize: number;
  gpuBatchSize: number;
  autoSaveInterval: number;
  isFirstRun: boolean;
  defaultSystemPrompt: string;
  defaultCloudProvider: string;
  fontSize: number;
  reducedMotion: boolean;
  telemetryOptIn: boolean;
  teamSyncEnabled: boolean;
  teamSyncUrl: string;
  teamSyncInterval: number;
  ssoToken: string;
  oidcDiscoveryUrl: string;
  modelRegistryUrl: string;
}

const DEFAULTS: AppSettings = {
  privacyLevel: "balanced",
  theme: "system",
  inferenceThreads: 4,
  gpuLayers: "auto",
  gpuContextSize: 2048,
  gpuBatchSize: 512,
  autoSaveInterval: 30,
  isFirstRun: true,
  defaultSystemPrompt: "You are a helpful, harmless, and honest AI assistant. You always answer directly and concisely.",
  defaultCloudProvider: "openai",
  fontSize: 14,
  reducedMotion: false,
  telemetryOptIn: false,
  teamSyncEnabled: false,
  teamSyncUrl: "https://sync.example.com/api",
  teamSyncInterval: 60,
  ssoToken: "",
  oidcDiscoveryUrl: "",
  modelRegistryUrl: "https://registry.example.com/models",
};

interface SettingsStore extends AppSettings {
  loaded: boolean;
  load: () => Promise<void>;
  set: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => Promise<void>;
  completeSetup: () => Promise<void>;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function applyTheme(theme: Theme) {
  let activeTheme: "light" | "dark" = "light";
  if (theme === "system") {
    activeTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } else {
    activeTheme = theme;
  }
  document.documentElement.setAttribute("data-theme", activeTheme);
}

function applyFontSize(fontSize: number) {
  document.documentElement.style.fontSize = `${fontSize}px`;
}

function applyReducedMotion(reducedMotion: boolean) {
  if (reducedMotion) {
    document.documentElement.classList.add("reduced-motion");
  } else {
    document.documentElement.classList.remove("reduced-motion");
  }
}

if (typeof window !== "undefined") {
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const store = useSettingsStore.getState();
    if (store.theme === "system") {
      applyTheme("system");
    }
  });
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useSettingsStore = create<SettingsStore>((set) => ({
  ...DEFAULTS,
  loaded: false,

  load: async () => {
    try {
      const raw = await invoke<Record<string, string>>("get_all_settings");

      const settings: AppSettings = {
        privacyLevel: (raw["privacy_level"] as PrivacyLevel) ?? DEFAULTS.privacyLevel,
        theme: (raw["theme"] as Theme) ?? DEFAULTS.theme,
        inferenceThreads: parseInt(raw["inference_threads"] ?? String(DEFAULTS.inferenceThreads), 10),
        gpuLayers: raw["gpu_layers"] ?? DEFAULTS.gpuLayers,
        gpuContextSize: parseInt(raw["gpu_context_size"] ?? String(DEFAULTS.gpuContextSize), 10),
        gpuBatchSize: parseInt(raw["gpu_batch_size"] ?? String(DEFAULTS.gpuBatchSize), 10),
        autoSaveInterval: parseInt(
          raw["auto_save_interval"] ?? String(DEFAULTS.autoSaveInterval),
          10,
        ),
        isFirstRun: raw["first_run_complete"] !== "true",
        defaultSystemPrompt: raw["default_system_prompt"] ?? DEFAULTS.defaultSystemPrompt,
        defaultCloudProvider: raw["default_cloud_provider"] ?? DEFAULTS.defaultCloudProvider,
        fontSize: parseInt(raw["font_size"] ?? String(DEFAULTS.fontSize), 10),
        reducedMotion: raw["reduced_motion"] === "true",
        telemetryOptIn: raw["telemetry_opt_in"] === "true",
        teamSyncEnabled: raw["team_sync_enabled"] === "true",
        teamSyncUrl: raw["team_sync_url"] ?? DEFAULTS.teamSyncUrl,
        teamSyncInterval: parseInt(raw["team_sync_interval"] ?? String(DEFAULTS.teamSyncInterval), 10),
        ssoToken: raw["sso_token"] ?? DEFAULTS.ssoToken,
        oidcDiscoveryUrl: raw["oidc_discovery_url"] ?? DEFAULTS.oidcDiscoveryUrl,
        modelRegistryUrl: raw["model_registry_url"] ?? DEFAULTS.modelRegistryUrl,
      };

      applyTheme(settings.theme);
      applyFontSize(settings.fontSize);
      applyReducedMotion(settings.reducedMotion);
      
      set({ ...settings, loaded: true });
    } catch {
      // browser mode — use defaults, not first run
      applyTheme(DEFAULTS.theme);
      applyFontSize(DEFAULTS.fontSize);
      applyReducedMotion(DEFAULTS.reducedMotion);
      set({ ...DEFAULTS, isFirstRun: false, loaded: true });
    }
  },

  set: async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
    // Optimistic update
    set({ [key]: value } as unknown as Partial<SettingsStore>);

    if (key === "theme") applyTheme(value as Theme);
    if (key === "fontSize") applyFontSize(value as number);
    if (key === "reducedMotion") applyReducedMotion(value as boolean);

    // Persist
    const dbKey = camelToSnake(key as string);
    try {
      await invoke("set_setting", { key: dbKey, value: String(value) });
    } catch {
      // ignore in browser mode
    }
  },

  completeSetup: async () => {
    try {
      await invoke("set_setting", { key: "first_run_complete", value: "true" });
    } catch {
      // ignore in browser mode
    }
    set({ isFirstRun: false });
  },
}));

function camelToSnake(camel: string): string {
  return camel.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
}
