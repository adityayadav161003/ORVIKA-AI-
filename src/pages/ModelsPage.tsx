import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Spinner } from "@/components/ui/Spinner";
import { cn } from "@/utils/cn";

// ─── Types ───────────────────────────────────────────────────────────────────

interface LlmStatus {
  state: "stopped" | "starting" | "running" | "crashed";
  host: string;
  port: number;
  pid?: number;
  healthy: boolean;
  modelPath?: string;
  lastError?: string;
  binaryPath?: string;
}

interface HardwareInfo {
  gpuAvailable: boolean;
  gpuName?: string;
  vramTotalMb?: number;
  vramFreeMb?: number;
  recommendedGpuLayers: number;
  backend: string;
}

interface RegistryModel {
  id: string;
  name: string;
  sizeBytes: number;
  quantization: string;
  minVramGb: number;
  filename: string;
}

interface DownloadedModel {
  id: string;
  modelName: string;
  modelPath: string;
  fileSize: number;
  quantization: string;
  isActive: boolean;
}

interface DownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number;
  percent: number;
  phase: string;
}

interface BenchmarkReport {
  ttftMs: number;
  tokensPerSecond: number;
  completionTokens: number;
  promptTokens: number;
  totalMs: number;
  backend: string;
  modelPath: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)} s`;
  return `${ms.toFixed(0)} ms`;
}

const STATE_CONFIG = {
  stopped: { label: "Stopped", dot: "bg-text-muted", badge: "text-text-muted bg-surface" },
  starting: { label: "Starting…", dot: "bg-yellow-500 animate-pulse", badge: "text-yellow-700 bg-yellow-50" },
  running: { label: "Running", dot: "bg-green-500", badge: "text-green-700 bg-green-50" },
  crashed: { label: "Crashed", dot: "bg-accent-secondary", badge: "text-accent-secondary bg-accent-secondary/10" },
} as const;

// ─── Sub-components ───────────────────────────────────────────────────────────

function StatusBadge({ status }: { status: LlmStatus }) {
  const cfg = STATE_CONFIG[status.state] ?? STATE_CONFIG.stopped;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 font-mono text-xs font-medium",
        cfg.badge,
      )}
    >
      <span className={cn("h-1.5 w-1.5 rounded-full", cfg.dot)} />
      {cfg.label}
      {status.state === "running" && !status.healthy && (
        <span className="ml-0.5 opacity-70">(unhealthy)</span>
      )}
    </span>
  );
}

function ProgressBar({ progress }: { progress: DownloadProgress }) {
  const pct = Math.min(100, Math.max(0, progress.percent));
  const phaseLabel =
    progress.phase === "verifying"
      ? "Verifying SHA-256…"
      : progress.phase === "complete"
        ? "Complete"
        : `Downloading ${pct.toFixed(1)}%`;

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between font-mono text-xs text-text-secondary">
        <span>{phaseLabel}</span>
        <span>
          {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
        </span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface">
        <div
          className="h-full rounded-full bg-accent transition-all duration-300"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function GpuCard({ hw }: { hw: HardwareInfo }) {
  return (
    <div className="rounded-lg border border-border bg-surface px-4 py-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="font-mono text-xs uppercase tracking-widest text-text-muted">Hardware</p>
          <p className="mt-1 text-sm font-semibold text-text-primary">
            {hw.gpuAvailable ? hw.gpuName : "CPU only"}
          </p>
          {hw.gpuAvailable && (
            <p className="mt-0.5 font-mono text-xs text-text-secondary">
              {hw.vramTotalMb ? `${(hw.vramTotalMb / 1024).toFixed(1)} GB VRAM` : "VRAM unknown"} ·{" "}
              {hw.vramFreeMb ? `${(hw.vramFreeMb / 1024).toFixed(1)} GB free` : ""}
            </p>
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          <span className="rounded-md border border-border bg-white px-2 py-0.5 font-mono text-xs text-text-secondary">
            {hw.backend.toUpperCase()}
          </span>
          {hw.gpuAvailable && (
            <span className="rounded-md border border-accent/30 bg-accent-light px-2 py-0.5 font-mono text-xs text-accent">
              {hw.recommendedGpuLayers} GPU layers
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

function BenchmarkCard({ report }: { report: BenchmarkReport }) {
  const metrics = [
    { label: "TTFT", value: formatMs(report.ttftMs), highlight: true },
    { label: "Throughput", value: `${report.tokensPerSecond.toFixed(1)} tok/s`, highlight: true },
    { label: "Tokens", value: `${report.completionTokens}` },
    { label: "Total time", value: formatMs(report.totalMs) },
    { label: "Backend", value: report.backend.toUpperCase() },
  ];

  return (
    <div className="rounded-lg border border-border bg-surface p-4">
      <p className="mb-3 font-mono text-xs uppercase tracking-widest text-text-muted">
        Last benchmark
      </p>
      <div className="flex flex-wrap gap-4">
        {metrics.map((m) => (
          <div key={m.label}>
            <p className="font-mono text-xs text-text-muted">{m.label}</p>
            <p
              className={cn(
                "font-mono text-lg font-semibold",
                m.highlight ? "text-accent" : "text-text-primary",
              )}
            >
              {m.value}
            </p>
          </div>
        ))}
      </div>
      <p className="mt-2 truncate font-mono text-xs text-text-muted">
        Model: {report.modelPath.split(/[\\/]/).pop()}
      </p>
    </div>
  );
}

// ─── Main component ───────────────────────────────────────────────────────────

export function ModelsPage() {
  const [llmStatus, setLlmStatus] = useState<LlmStatus | null>(null);
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [registry, setRegistry] = useState<RegistryModel[]>([]);
  const [downloaded, setDownloaded] = useState<DownloadedModel[]>([]);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [selectedModel, setSelectedModel] = useState("");
  const [localPath, setLocalPath] = useState("");
  const [benchmark, setBenchmark] = useState<BenchmarkReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  // ── Data fetching ───────────────────────────────────────────────────────────

  const refresh = useCallback(async () => {
    try {
      const [status, hw, reg, dl] = await Promise.all([
        invoke<LlmStatus>("get_llm_status"),
        invoke<HardwareInfo>("get_hardware_info"),
        invoke<RegistryModel[]>("list_registry_models"),
        invoke<DownloadedModel[]>("list_downloaded_models"),
      ]);
      setLlmStatus(status);
      setHardware(hw);
      setRegistry(reg);
      setDownloaded(dl);
      if (!selectedModel && reg[0]) setSelectedModel(reg[0].id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [selectedModel]);

  useEffect(() => {
    void refresh();
    const unlistenProgress = listen<DownloadProgress>("model-download-progress", (event) => {
      setDownloadProgress(event.payload);
      if (event.payload.phase === "complete") {
        setTimeout(() => setDownloadProgress(null), 2000);
        void refresh();
      }
    });
    const unlistenRuntime = listen<LlmStatus>("llm-runtime-status", (event) => {
      setLlmStatus(event.payload);
    });
    return () => {
      void unlistenProgress.then((fn) => fn());
      void unlistenRuntime.then((fn) => fn());
    };
  }, [refresh]);

  // ── Action helpers ──────────────────────────────────────────────────────────

  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleDownload = () =>
    run(async () => {
      setDownloadProgress(null);
      await invoke("download_model", { modelId: selectedModel });
    });

  const handleRegisterPath = () =>
    run(async () => {
      if (!localPath.trim()) throw new Error("Please enter a path to a GGUF file");
      await invoke("register_model_path", {
        registryId: selectedModel,
        filePath: localPath,
      });
      setLocalPath("");
    });

  const handleImportCustomGguf = async () => {
    setError(null);
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'GGUF Model Files',
          extensions: ['gguf']
        }]
      });

      if (!selected) return;
      const filePath = Array.isArray(selected) ? selected[0] : selected;

      const modelName = prompt(
        "Enter a display name for this custom model:",
        filePath.split(/[\\/]/).pop()?.replace(".gguf", "") || "Custom Model"
      );
      if (modelName === null) return;

      setBusy(true);
      await invoke("import_custom_gguf", {
        filePath,
        modelName,
      });
      await refresh();
      alert("Successfully imported custom GGUF model!");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleSetActive = (modelId: string) =>
    run(() => invoke("set_active_model", { modelId }));

  const handleDeleteModel = (modelId: string) =>
    run(() => invoke("delete_model", { modelId }));

  const handleStart = () => run(() => invoke("start_llm_server"));
  const handleStop = () => run(() => invoke("stop_llm_server"));
  const handleRestart = () => run(() => invoke("restart_llm_server"));

  const handleBenchmark = () =>
    run(async () => {
      const report = await invoke<BenchmarkReport>("run_llm_benchmark");
      setBenchmark(report);
    });

  // ── Render ──────────────────────────────────────────────────────────────────

  const isRunning = llmStatus?.state === "running" && llmStatus.healthy;
  const activeModel = downloaded.find((m) => m.isActive);

  return (
    <div className="mx-auto max-w-4xl space-y-6 px-6 py-8">
      {/* Page header */}
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="font-serif text-3xl font-bold text-accent">Model Management</h1>
          <p className="mt-1 text-sm text-text-secondary">
            Manage local LLMs, hardware resources, and benchmarks.
          </p>
        </div>
        {llmStatus && <StatusBadge status={llmStatus} />}
      </header>

      {/* Error banner */}
      {error && (
        <div
          role="alert"
          className="flex items-start gap-3 rounded-lg border border-accent-secondary/30 bg-accent-secondary/5 px-4 py-3"
        >
          <span className="mt-0.5 text-accent-secondary">⚠</span>
          <p className="text-sm text-accent-secondary">{error}</p>
          <button
            type="button"
            className="ml-auto shrink-0 text-xs text-text-muted hover:text-accent-secondary"
            onClick={() => setError(null)}
          >
            ✕
          </button>
        </div>
      )}

      {/* Hardware info */}
      {hardware ? (
        <GpuCard hw={hardware} />
      ) : (
        <div className="flex items-center gap-2 text-sm text-text-muted">
          <Spinner /> Detecting hardware…
        </div>
      )}

      {/* ── Server control ── */}
      <section className="rounded-lg border border-border bg-white p-5 shadow-sm">
        <h2 className="mb-4 font-mono text-xs uppercase tracking-widest text-accent">
          Runtime Control
        </h2>

        {llmStatus ? (
          <dl className="mb-4 grid gap-y-1 font-mono text-xs sm:grid-cols-2">
            <div className="text-text-muted">
              Endpoint:{" "}
              <span className="text-text-primary">
                http://{llmStatus.host}:{llmStatus.port}
              </span>
            </div>
            {llmStatus.pid && (
              <div className="text-text-muted">
                PID: <span className="text-text-primary">{llmStatus.pid}</span>
              </div>
            )}
            <div className="text-text-muted sm:col-span-2">
              Binary:{" "}
              <span className="text-text-primary">{llmStatus.binaryPath ?? "not found on PATH"}</span>
            </div>
            <div className="text-text-muted sm:col-span-2">
              Active model:{" "}
              <span className="text-text-primary">
                {activeModel?.modelName ?? llmStatus.modelPath?.split(/[\\/]/).pop() ?? "none"}
              </span>
            </div>
            {llmStatus.lastError && (
              <div className="sm:col-span-2 text-accent-secondary">Error: {llmStatus.lastError}</div>
            )}
          </dl>
        ) : (
          <div className="mb-4 flex items-center gap-2 text-sm text-text-muted">
            <Spinner /> Loading…
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          <Button
            id="btn-start-server"
            size="sm"
            onClick={() => void handleStart()}
            disabled={busy || llmStatus?.state === "running"}
          >
            Start server
          </Button>
          <Button
            id="btn-stop-server"
            size="sm"
            variant="secondary"
            onClick={() => void handleStop()}
            disabled={busy || llmStatus?.state === "stopped"}
          >
            Stop
          </Button>
          <Button
            id="btn-restart-server"
            size="sm"
            variant="ghost"
            onClick={() => void handleRestart()}
            disabled={busy}
          >
            Restart
          </Button>
          {busy && <Spinner />}
        </div>
      </section>

      {/* ── Model management ── */}
      <section className="rounded-lg border border-border bg-white p-5 shadow-sm">
        <h2 className="mb-4 font-mono text-xs uppercase tracking-widest text-accent">
          Models
        </h2>

        {/* Registry picker */}
        <div className="mb-4 space-y-3">
          <label className="block text-xs font-medium text-text-secondary">
            Registry model
          </label>
          <select
            id="select-registry-model"
            className="w-full rounded-md border border-border bg-surface px-3 py-2 text-sm text-text-primary focus:outline-none focus:ring-2 focus:ring-accent/30"
            value={selectedModel}
            onChange={(e) => setSelectedModel(e.target.value)}
          >
            {registry.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name} — {formatBytes(m.sizeBytes)} · {m.quantization}
              </option>
            ))}
          </select>

          {downloadProgress && <ProgressBar progress={downloadProgress} />}

          <div className="flex flex-wrap gap-2">
            <Button
              id="btn-download-model"
              size="sm"
              onClick={() => void handleDownload()}
              disabled={busy || !selectedModel}
            >
              Download from HF Hub
            </Button>
            <Button
              id="btn-cancel-download"
              size="sm"
              variant="secondary"
              onClick={() => void invoke("cancel_model_download")}
              disabled={!downloadProgress || downloadProgress.phase === "complete"}
            >
              Cancel
            </Button>
          </div>
        </div>

        {/* Local import */}
        <div className="mb-5 space-y-2 border-t border-border pt-4">
          <label className="block text-xs font-medium text-text-secondary">
            Import local GGUF file
          </label>
          <div className="flex flex-col sm:flex-row gap-2">
            <div className="flex-1 flex gap-2">
              <Input
                id="input-local-path"
                placeholder="C:\models\gemma-2-9b-it-Q4_K_M.gguf"
                value={localPath}
                onChange={(e) => setLocalPath(e.target.value)}
                className="focus:ring-2 focus:ring-accent/40"
                aria-label="Local path to GGUF file"
              />
              <Button
                id="btn-import-model"
                size="sm"
                variant="secondary"
                onClick={() => void handleRegisterPath()}
                disabled={busy || !localPath.trim()}
                className="focus:ring-2 focus:ring-accent/40"
              >
                Import
              </Button>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-xs text-text-muted">or</span>
              <Button
                id="btn-browse-custom-gguf"
                size="sm"
                variant="secondary"
                onClick={handleImportCustomGguf}
                disabled={busy}
                className="focus:ring-2 focus:ring-accent/40"
                aria-label="Browse and import custom GGUF file"
              >
                Browse & Import GGUF
              </Button>
            </div>
          </div>
        </div>

        {/* Downloaded models list */}
        {downloaded.length > 0 ? (
          <div className="space-y-1">
            <p className="mb-2 text-xs font-medium text-text-secondary">Downloaded models</p>
            <ul className="divide-y divide-border rounded-md border border-border">
              {downloaded.map((m) => (
                <li
                  key={m.id}
                  className={cn(
                    "flex flex-wrap items-center justify-between gap-2 px-3 py-2.5",
                    m.isActive && "bg-accent-light",
                  )}
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="truncate text-sm font-medium text-text-primary">
                        {m.modelName}
                      </span>
                      {m.isActive && (
                        <span className="shrink-0 rounded-full bg-accent px-1.5 py-0.5 font-mono text-xs text-white">
                          active
                        </span>
                      )}
                    </div>
                    <p className="font-mono text-xs text-text-muted">
                      {m.quantization} · {formatBytes(m.fileSize)}
                    </p>
                  </div>
                  <div className="flex gap-1.5">
                    {!m.isActive && (
                      <Button
                        id={`btn-set-active-${m.id}`}
                        size="sm"
                        variant="secondary"
                        onClick={() => void handleSetActive(m.id)}
                        disabled={busy}
                      >
                        Set active
                      </Button>
                    )}
                    <Button
                      id={`btn-delete-${m.id}`}
                      size="sm"
                      variant="ghost"
                      className="text-accent-secondary hover:bg-accent-secondary/10"
                      onClick={() => void handleDeleteModel(m.id)}
                      disabled={busy}
                    >
                      Delete
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <p className="text-sm text-text-muted">
            No models downloaded yet. Select a model above and click{" "}
            <em>Download from HF Hub</em>.
          </p>
        )}
      </section>



      {/* ── Benchmark ── */}
      <section className="rounded-lg border border-border bg-white p-5 shadow-sm">
        <div className="mb-4 flex items-center justify-between">
          <h2 className="font-mono text-xs uppercase tracking-widest text-accent">
            Hardware Benchmark
          </h2>
          {!isRunning && (
            <span className="rounded-md bg-surface px-2 py-0.5 font-mono text-xs text-text-muted">
              Start server first
            </span>
          )}
        </div>

        <p className="mb-3 text-sm text-text-secondary">
          Measures time-to-first-token (TTFT) and throughput (tokens/sec) using a short fixed
          prompt.
        </p>

        <Button
          id="btn-run-benchmark"
          size="sm"
          variant="secondary"
          onClick={() => void handleBenchmark()}
          disabled={busy || !isRunning}
          loading={busy}
        >
          Run benchmark
        </Button>

        {benchmark && (
          <div className="mt-4">
            <BenchmarkCard report={benchmark} />
          </div>
        )}
      </section>
    </div>
  );
}
