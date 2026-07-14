import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Toggle } from "@/components/ui/Toggle";
import { Select } from "@/components/ui/Select";
import { Spinner } from "@/components/ui/Spinner";
import { cn } from "@/utils/cn";
import { useSettingsStore, type PrivacyLevel, type Theme } from "@/stores/settingsStore";

// ─── Types ────────────────────────────────────────────────────────────────────

interface ApiKeyInfo {
  provider: string;
  createdAt: string;
  lastUsedAt?: string;
}

interface DbStatus {
  path: string;
  version: number;
  walMode: boolean;
}

// ─── Sub-components ───────────────────────────────────────────────────────────

function SettingsSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-border bg-white p-6 shadow-sm">
      <h2 className="mb-4 font-mono text-xs uppercase tracking-widest text-accent font-semibold">{title}</h2>
      {children}
    </section>
  );
}

function Field({
  label,
  description,
  children,
}: {
  label: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-3 py-3 first:pt-0 last:pb-0 [&:not(:last-child)]:border-b [&:not(:last-child)]:border-border">
      <div>
        <p className="text-sm font-medium text-text-primary">{label}</p>
        {description && <p className="mt-0.5 text-xs text-text-muted">{description}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

// ─── API Key Row ──────────────────────────────────────────────────────────────

interface ApiKeyRowProps {
  provider: string;
  label: string;
  placeholder: string;
  prefix?: string;
  stored: boolean;
  onSave: (provider: string, key: string) => Promise<void>;
  onDelete: (provider: string) => Promise<void>;
}

function ApiKeyRow({
  provider,
  label,
  placeholder,
  prefix,
  stored,
  onSave,
  onDelete,
}: ApiKeyRowProps) {
  const [value, setValue] = useState("");
  const [show, setShow] = useState(false);
  const [saving, setSaving] = useState(false);
  const [feedback, setFeedback] = useState<{ ok: boolean; msg: string } | null>(null);

  const handleSave = async () => {
    if (!value.trim()) return;
    setSaving(true);
    setFeedback(null);
    try {
      const valid = await invoke<boolean>("validate_api_key_format", {
        provider,
        key: value.trim(),
      });
      if (!valid) {
        setFeedback({ ok: false, msg: `Key format invalid for ${label}` });
        return;
      }
      await onSave(provider, value.trim());
      setValue("");
      setFeedback({ ok: true, msg: "Saved and encrypted." });
      setTimeout(() => setFeedback(null), 3000);
    } catch (err) {
      setFeedback({ ok: false, msg: String(err) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="py-3 first:pt-0 last:pb-0 [&:not(:last-child)]:border-b [&:not(:last-child)]:border-border">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <p className="text-sm font-medium text-text-primary">{label}</p>
          <p className="mt-0.5 font-mono text-xs text-text-muted">
            {stored ? "✓ Key stored (encrypted)" : "No key stored"}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {stored && (
            <Button
              id={`btn-delete-key-${provider}`}
              size="sm"
              variant="ghost"
              className="text-accent-secondary hover:bg-accent-secondary/10 focus:ring-2 focus:ring-accent/40"
              onClick={() => void onDelete(provider)}
              aria-label={`Remove API key for ${label}`}
            >
              Remove
            </Button>
          )}
        </div>
      </div>

      <div className="mt-2 flex gap-2">
        <div className="relative flex-1">
          {prefix && (
            <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 font-mono text-xs text-text-muted">
              {prefix}
            </span>
          )}
          <Input
            id={`input-key-${provider}`}
            type={show ? "text" : "password"}
            placeholder={placeholder}
            value={value}
            onChange={(e) => setValue(e.target.value)}
            className={cn(prefix ? "pl-7" : "", "focus:ring-2 focus:ring-accent/40")}
            aria-label={`${label} API key input`}
          />
        </div>
        <button
          type="button"
          className="px-2 text-xs text-text-muted hover:text-accent focus:outline-none focus:ring-2 focus:ring-accent/40 rounded"
          onClick={() => setShow((v) => !v)}
          aria-label={show ? "Hide key characters" : "Show key characters"}
        >
          {show ? "Hide" : "Show"}
        </button>
        <Button
          id={`btn-save-key-${provider}`}
          size="sm"
          onClick={() => void handleSave()}
          loading={saving}
          disabled={!value.trim()}
          className="focus:ring-2 focus:ring-accent/40"
          aria-label={`Save API key for ${label}`}
        >
          Save
        </Button>
      </div>

      {feedback && (
        <p
          className={cn(
            "mt-1.5 font-mono text-xs",
            feedback.ok ? "text-green-600" : "text-accent-secondary",
          )}
        >
          {feedback.msg}
        </p>
      )}
    </div>
  );
}

// ─── Privacy level radio ──────────────────────────────────────────────────────

const PRIVACY_OPTIONS: { value: PrivacyLevel; label: string; description: string }[] = [
  {
    value: "strict",
    label: "Strict",
    description: "No external network calls. All analysis is fully local.",
  },
  {
    value: "balanced",
    label: "Balanced",
    description: "External queries require explicit user approval before sending.",
  },
  {
    value: "open",
    label: "Open",
    description: "External queries sent automatically for research. Best results, least privacy.",
  },
];

// ─── Main page ────────────────────────────────────────────────────────────────

interface TelemetryData {
  status: string;
  sessionCount: number;
  messageCount: number;
  documentCount: number;
  totalCloudSpending: number;
  averageLlmLatencyMs: number;
  hardware: {
    cpuBrand: string;
    physicalCores: number;
    logicalCores: number;
    totalMemoryGb: number;
    hasNvidiaGpu: boolean;
  };
}

export function SettingsPage() {
  const settings = useSettingsStore();
  const [providers, setProviders] = useState<ApiKeyInfo[]>([]);
  const [dbStatus, setDbStatus] = useState<DbStatus | null>(null);
  
  // Telemetry state
  const [telemetry, setTelemetry] = useState<TelemetryData | null>(null);
  const [loadingTelemetry, setLoadingTelemetry] = useState(false);

  useEffect(() => {
    void refreshProviders();
    invoke<DbStatus>("get_db_status")
      .then(setDbStatus)
      .catch(() => undefined);
  }, []);

  // Fetch telemetry diagnostics if opted-in
  useEffect(() => {
    if (settings.telemetryOptIn) {
      setLoadingTelemetry(true);
      invoke<TelemetryData>("get_local_telemetry")
        .then((data) => {
          if (data.status === "enabled") {
            setTelemetry(data);
          }
        })
        .catch((err) => console.error("Telemetry failed", err))
        .finally(() => setLoadingTelemetry(false));
    } else {
      setTelemetry(null);
    }
  }, [settings.telemetryOptIn]);

  const refreshProviders = async () => {
    try {
      const list = await invoke<ApiKeyInfo[]>("list_api_key_providers");
      setProviders(list);
    } catch {
      // browser mode
    }
  };

  const handleSaveKey = async (provider: string, key: string) => {
    await invoke("store_api_key", { provider, plaintextKey: key });
    await refreshProviders();
  };

  const handleDeleteKey = async (provider: string) => {
    await invoke("delete_api_key", { provider });
    await refreshProviders();
  };

  const isStored = (provider: string) => providers.some((p) => p.provider === provider);

  return (
    <div className="mx-auto max-w-3xl space-y-6 px-6 py-8">
      {/* ── API Keys ── */}
      <SettingsSection title="Provider API Keys">
        <p className="mb-4 text-sm text-text-secondary">
          Keys are encrypted with AES-256-GCM before being stored. They are machine-bound and
          never transmitted to any server.
        </p>
        <ApiKeyRow
          provider="openai"
          label="OpenAI"
          placeholder="sk-..."
          prefix="sk-"
          stored={isStored("openai")}
          onSave={handleSaveKey}
          onDelete={handleDeleteKey}
        />
        <ApiKeyRow
          provider="gemini"
          label="Google Gemini"
          placeholder="AIza..."
          stored={isStored("gemini")}
          onSave={handleSaveKey}
          onDelete={handleDeleteKey}
        />
        <ApiKeyRow
          provider="anthropic"
          label="Anthropic Claude"
          placeholder="sk-ant-..."
          prefix="sk-ant-"
          stored={isStored("anthropic")}
          onSave={handleSaveKey}
          onDelete={handleDeleteKey}
        />
      </SettingsSection>

      {/* ── Privacy ── */}
      <SettingsSection title="Privacy & Research">
        <p className="mb-4 text-sm text-text-secondary">
          Controls how the app handles external research queries.
        </p>
        <div className="space-y-4">
          <div className="space-y-2">
            {PRIVACY_OPTIONS.map((opt) => (
              <label
                key={opt.value}
                className={cn(
                  "flex cursor-pointer items-start gap-3 rounded-md border p-3 transition-colors focus-within:ring-2 focus-within:ring-accent/40",
                  settings.privacyLevel === opt.value
                    ? "border-accent bg-accent-light"
                    : "border-border hover:border-accent/30",
                )}
              >
                <input
                  type="radio"
                  name="privacy_level"
                  id={`radio-privacy-${opt.value}`}
                  value={opt.value}
                  checked={settings.privacyLevel === opt.value}
                  onChange={() => void settings.set("privacyLevel", opt.value)}
                  className="mt-0.5 accent-accent"
                />
                <div>
                  <p className="text-sm font-medium text-text-primary">{opt.label}</p>
                  <p className="mt-0.5 text-xs text-text-muted">{opt.description}</p>
                </div>
              </label>
            ))}
          </div>

          <div className="border-t border-border pt-4 space-y-4">
            <Field
              label="Default Cloud Provider"
              description="Default API endpoint used for outbound research queries."
            >
              <Select
                id="select-default-provider"
                value={settings.defaultCloudProvider}
                onChange={(e) => void settings.set("defaultCloudProvider", e.target.value)}
                options={[
                  { value: "openai", label: "OpenAI" },
                  { value: "gemini", label: "Google Gemini" },
                  { value: "anthropic", label: "Anthropic Claude" },
                ]}
                className="w-48 text-xs py-1.5 focus:ring-2 focus:ring-accent/40"
              />
            </Field>

            <Field
              label="Local Diagnostics Telemetry"
              description="Opt-in to local diagnostic tracking. Data never leaves your device."
            >
              <Toggle
                checked={settings.telemetryOptIn}
                onChange={(checked) => void settings.set("telemetryOptIn", checked)}
                aria-label="Opt-in to local diagnostics tracking"
              />
            </Field>
          </div>
        </div>
      </SettingsSection>

      {/* ── Telemetry Stats Panel ── */}
      {settings.telemetryOptIn && (
        <SettingsSection title="Local Diagnostics Center">
          {loadingTelemetry && !telemetry ? (
            <div className="flex items-center gap-2 justify-center py-6 text-xs text-text-muted">
              <Spinner /> Loading local metrics...
            </div>
          ) : telemetry ? (
            <div className="space-y-4">
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
                <div className="p-3 bg-surface rounded-md border border-border">
                  <p className="text-[10px] uppercase text-text-muted font-semibold tracking-wider font-mono">Sessions</p>
                  <p className="text-xl font-bold text-text-primary mt-1">{telemetry.sessionCount}</p>
                </div>
                <div className="p-3 bg-surface rounded-md border border-border">
                  <p className="text-[10px] uppercase text-text-muted font-semibold tracking-wider font-mono">Messages</p>
                  <p className="text-xl font-bold text-text-primary mt-1">{telemetry.messageCount}</p>
                </div>
                <div className="p-3 bg-surface rounded-md border border-border">
                  <p className="text-[10px] uppercase text-text-muted font-semibold tracking-wider font-mono">Files Parsed</p>
                  <p className="text-xl font-bold text-text-primary mt-1">{telemetry.documentCount}</p>
                </div>
                <div className="p-3 bg-surface rounded-md border border-border">
                  <p className="text-[10px] uppercase text-text-muted font-semibold tracking-wider font-mono">API Costs</p>
                  <p className="text-xl font-bold text-text-primary mt-1">${telemetry.totalCloudSpending.toFixed(3)}</p>
                </div>
              </div>

              <div className="text-xs font-mono text-text-secondary border-t border-border pt-3 space-y-2">
                <div className="flex justify-between">
                  <span>Avg Local Latency:</span>
                  <span className="text-text-primary font-semibold">
                    {telemetry.averageLlmLatencyMs > 0 
                      ? `${(telemetry.averageLlmLatencyMs / 1000).toFixed(2)} seconds`
                      : "0.00 seconds"}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>CPU Brand:</span>
                  <span className="text-text-primary font-semibold truncate max-w-[70%]">
                    {telemetry.hardware?.cpuBrand || "Unknown"}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>Physical / Logical Cores:</span>
                  <span className="text-text-primary font-semibold">
                    {telemetry.hardware?.physicalCores} / {telemetry.hardware?.logicalCores}
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>Total System Memory:</span>
                  <span className="text-text-primary font-semibold">
                    {telemetry.hardware?.totalMemoryGb} GB
                  </span>
                </div>
                <div className="flex justify-between">
                  <span>NVIDIA GPU (CUDA):</span>
                  <span className={cn(
                    "font-semibold",
                    telemetry.hardware?.hasNvidiaGpu ? "text-green-600" : "text-text-muted"
                  )}>
                    {telemetry.hardware?.hasNvidiaGpu ? "Available" : "Not Detected"}
                  </span>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-xs text-text-muted italic py-2">No diagnostics available.</div>
          )}
        </SettingsSection>
      )}

      {/* ── LLM Parameters ── */}
      <SettingsSection title="LLM Parameters">
        <Field
          label="Global System Prompt"
          description="Default instructions given to the LLM. Can be overridden per session."
        >
          <textarea
            id="input-system-prompt"
            value={settings.defaultSystemPrompt}
            onChange={(e) => void settings.set("defaultSystemPrompt", e.target.value)}
            className="h-24 w-full min-w-[300px] resize-y rounded-md border border-border bg-surface px-3 py-2 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>

        <Field
          label="Inference threads"
          description="CPU threads used for inference. Set to the number of physical cores."
        >
          <input
            id="input-inference-threads"
            type="number"
            min={1}
            max={32}
            value={settings.inferenceThreads}
            onChange={(e) =>
              void settings.set("inferenceThreads", parseInt(e.target.value, 10))
            }
            className="w-20 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>

        <Field
          label="GPU layers"
          description={`"auto" uses the recommended layer count from the hardware benchmark.`}
        >
          <input
            id="input-gpu-layers"
            type="text"
            value={settings.gpuLayers}
            onChange={(e) => void settings.set("gpuLayers", e.target.value)}
            className="w-24 rounded-md border border-border bg-surface px-3 py-1.5 font-mono text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>

        <Field
          label="GPU Context Size"
          description="Context window limit (n_ctx) for GPU token processing."
        >
          <input
            id="input-gpu-context-size"
            type="number"
            min={512}
            max={32768}
            step={512}
            value={settings.gpuContextSize}
            onChange={(e) =>
              void settings.set("gpuContextSize", parseInt(e.target.value, 10))
            }
            className="w-24 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>

        <Field
          label="GPU Batch Size"
          description="Batch size (n_batch) for prompt evaluation."
        >
          <input
            id="input-gpu-batch-size"
            type="number"
            min={8}
            max={2048}
            step={8}
            value={settings.gpuBatchSize}
            onChange={(e) =>
              void settings.set("gpuBatchSize", parseInt(e.target.value, 10))
            }
            className="w-24 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>
      </SettingsSection>

      {/* ── Application ── */}
      <SettingsSection title="Application">
        <Field label="Theme Mode" description="Persistent dark, light, or system appearance.">
          <Select
            id="select-app-theme"
            value={settings.theme}
            onChange={(e) => void settings.set("theme", e.target.value as Theme)}
            options={[
              { value: "system", label: "System Default" },
              { value: "light", label: "Light Mode" },
              { value: "dark", label: "Dark Mode" },
            ]}
            className="w-40 text-xs py-1.5 focus:ring-2 focus:ring-accent/40"
          />
        </Field>

        <Field
          label="Font Size"
          description="Adjust application readability typography scale."
        >
          <div className="flex items-center gap-2">
            <span className="text-xs text-text-muted">12px</span>
            <input
              id="input-font-size-slider"
              type="range"
              min="12"
              max="24"
              value={settings.fontSize}
              onChange={(e) => void settings.set("fontSize", parseInt(e.target.value, 10))}
              className="w-32 accent-accent cursor-pointer focus:outline-none focus:ring-2 focus:ring-accent/30 rounded"
              aria-label="Adjust font size slider"
            />
            <span className="text-xs font-mono font-bold text-text-primary">{settings.fontSize}px</span>
          </div>
        </Field>

        <Field
          label="Reduced Motion"
          description="Disable UI animation effects and transition states."
        >
          <Toggle
            checked={settings.reducedMotion}
            onChange={(checked) => void settings.set("reducedMotion", checked)}
            aria-label="Enable reduced motion"
          />
        </Field>

        <Field
          label="Auto-save interval"
          description="How often chat sessions are auto-saved (seconds)."
        >
          <input
            id="input-auto-save"
            type="number"
            min={10}
            max={300}
            step={10}
            value={settings.autoSaveInterval}
            onChange={(e) =>
              void settings.set("autoSaveInterval", parseInt(e.target.value, 10))
            }
            className="w-24 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>
      </SettingsSection>

      {/* ── Team Collaboration & Enterprise ── */}
      <SettingsSection title="Team Collaboration & Enterprise">
        <Field
          label="Enable Team Synchronization"
          description="Sync local sessions, messages, and audit logs with your team's self-hosted coordinator."
        >
          <Toggle
            checked={settings.teamSyncEnabled}
            onChange={(checked) => void settings.set("teamSyncEnabled", checked)}
            aria-label="Enable team database synchronization"
          />
        </Field>

        <Field
          label="Sync Coordinator URL"
          description="Endpoint URL of the team sync coordinator service."
        >
          <Input
            id="input-team-sync-url"
            type="text"
            value={settings.teamSyncUrl}
            onChange={(e) => void settings.set("teamSyncUrl", e.target.value)}
            disabled={!settings.teamSyncEnabled}
            className="w-80 text-sm focus:ring-2 focus:ring-accent/40"
            placeholder="https://sync.example.com/api"
            aria-label="Team sync coordinator URL"
          />
        </Field>

        <Field
          label="Sync Interval (seconds)"
          description="Frequency of database sync sweeps."
        >
          <input
            id="input-team-sync-interval"
            type="number"
            min={10}
            max={3600}
            step={10}
            value={settings.teamSyncInterval}
            onChange={(e) =>
              void settings.set("teamSyncInterval", parseInt(e.target.value, 10))
            }
            disabled={!settings.teamSyncEnabled}
            className="w-24 rounded-md border border-border bg-surface px-3 py-1.5 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
          />
        </Field>

        <Field
          label="SSO Authentication Token"
          description="Bearer token or assertion used for identity verification with OIDC/SSO."
        >
          <Input
            id="input-sso-token"
            type="password"
            value={settings.ssoToken}
            onChange={(e) => void settings.set("ssoToken", e.target.value)}
            className="w-80 text-sm focus:ring-2 focus:ring-accent/40"
            placeholder="eyJh..."
            aria-label="SSO Authentication Token"
          />
        </Field>

        <Field
          label="OIDC Discovery URL"
          description="OpenID Connect configuration endpoint for corporate identity provider."
        >
          <Input
            id="input-oidc-url"
            type="text"
            value={settings.oidcDiscoveryUrl}
            onChange={(e) => void settings.set("oidcDiscoveryUrl", e.target.value)}
            className="w-80 text-sm focus:ring-2 focus:ring-accent/40"
            placeholder="https://identity.example.com/.well-known/openid-configuration"
            aria-label="OIDC Discovery URL"
          />
        </Field>

        <Field
          label="Custom Model Registry URL"
          description="Enterprise-shared repository directory for custom GGUF models."
        >
          <Input
            id="input-model-registry-url"
            type="text"
            value={settings.modelRegistryUrl}
            onChange={(e) => void settings.set("modelRegistryUrl", e.target.value)}
            className="w-80 text-sm focus:ring-2 focus:ring-accent/40"
            placeholder="https://registry.example.com/models"
            aria-label="Custom model registry URL"
          />
        </Field>
      </SettingsSection>

      {/* ── About ── */}
      <SettingsSection title="About">
        <div className="space-y-2 font-mono text-xs text-text-muted">
          <div className="flex justify-between">
            <span>App version</span>
            <span className="text-text-primary font-semibold">0.3.0</span>
          </div>
          {dbStatus && (
            <>
              <div className="flex justify-between">
                <span>DB version</span>
                <span className="text-text-primary font-semibold">{dbStatus.version}</span>
              </div>
              <div className="flex justify-between">
                <span>WAL mode</span>
                <span className="text-text-primary font-semibold">{dbStatus.walMode ? "on" : "off"}</span>
              </div>
              <div className="flex flex-wrap justify-between gap-1">
                <span>Data directory</span>
                <span className="break-all text-right text-text-secondary">{dbStatus.path}</span>
              </div>
            </>
          )}
        </div>
      </SettingsSection>
    </div>
  );
}
