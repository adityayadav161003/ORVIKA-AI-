import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { cn } from "@/utils/cn";
import { useSettingsStore } from "@/stores/settingsStore";

// ─── Types ────────────────────────────────────────────────────────────────────

type Step = 1 | 2 | 3;

interface RegistryModel {
  id: string;
  name: string;
  sizeBytes: number;
  quantization: string;
  minVramGb: number;
}

interface DownloadProgress {
  modelId: string;
  downloadedBytes: number;
  totalBytes: number;
  percent: number;
  phase: string;
}

function formatBytes(b: number) {
  if (b >= 1e9) return `${(b / 1e9).toFixed(1)} GB`;
  return `${(b / 1e6).toFixed(0)} MB`;
}

// ─── Step 1: Welcome ─────────────────────────────────────────────────────────

function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="space-y-6 text-center">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-accent mx-auto">
        <span className="font-mono text-2xl font-bold text-white">P</span>
      </div>
      <div>
        <h2 className="font-serif text-2xl font-bold text-text-primary">
          Welcome to ORVIKA AI
        </h2>
        <p className="mt-2 text-sm text-text-secondary">
          Your AI runs entirely on your device. Your documents never leave your computer.
        </p>
      </div>
      <ul className="space-y-3 text-left">
        {[
          { icon: "🔒", text: "Zero document content transmitted externally — ever" },
          { icon: "💻", text: "Local LLM inference via llama.cpp + GPU acceleration" },
          { icon: "🔍", text: "Optional cloud research with privacy sanitization" },
          { icon: "🗃", text: "All data stays in your local SQLite database" },
        ].map((item) => (
          <li key={item.text} className="flex items-start gap-3 text-sm">
            <span className="mt-0.5 text-lg leading-none">{item.icon}</span>
            <span className="text-text-secondary">{item.text}</span>
          </li>
        ))}
      </ul>
      <Button id="btn-setup-next-1" className="w-full" onClick={onNext}>
        Get Started →
      </Button>
    </div>
  );
}

// ─── Step 2: Download model ───────────────────────────────────────────────────

function ModelStep({ onNext, onSkip }: { onNext: () => void; onSkip: () => void }) {
  const [models, setModels] = useState<RegistryModel[]>([]);
  const [selected, setSelected] = useState("");
  const [progress, setProgress] = useState<DownloadProgress | null>(null);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void invoke<RegistryModel[]>("list_registry_models")
      .then((m) => {
        setModels(m);
        if (m[0]) setSelected(m[0].id);
      })
      .catch(() => setModels([]));

    const unsub = listen<DownloadProgress>("model-download-progress", (e) => {
      setProgress(e.payload);
      if (e.payload.phase === "complete") setDone(true);
    });
    return () => {
      void unsub.then((fn) => fn());
    };
  }, []);

  const handleDownload = async () => {
    setError(null);
    try {
      await invoke("download_model", { modelId: selected });
    } catch (err) {
      setError(String(err));
    }
  };

  const pct = Math.min(100, progress?.percent ?? 0);

  return (
    <div className="space-y-5">
      <div>
        <h2 className="font-serif text-xl font-bold text-text-primary">Download a Model</h2>
        <p className="mt-1 text-sm text-text-secondary">
          Choose a model to download. SmolLM2 is small and fast; Gemma is more capable.
        </p>
      </div>

      <div className="space-y-2">
        {models.map((m) => (
          <label
            key={m.id}
            className={cn(
              "flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition-colors",
              selected === m.id ? "border-accent bg-accent-light" : "border-border hover:border-accent/30",
            )}
          >
            <input
              type="radio"
              name="setup_model"
              value={m.id}
              checked={selected === m.id}
              onChange={() => setSelected(m.id)}
              className="mt-0.5 accent-accent"
            />
            <div>
              <p className="text-sm font-medium text-text-primary">{m.name}</p>
              <p className="font-mono text-xs text-text-muted">
                {formatBytes(m.sizeBytes)} · {m.quantization} · min {m.minVramGb} GB VRAM
              </p>
            </div>
          </label>
        ))}
        {models.length === 0 && (
          <p className="text-sm text-text-muted">No models available (check internet connection).</p>
        )}
      </div>

      {progress && !done && (
        <div className="space-y-1.5">
          <div className="flex justify-between font-mono text-xs text-text-secondary">
            <span>
              {progress.phase === "verifying" ? "Verifying SHA-256…" : `${pct.toFixed(1)}%`}
            </span>
            <span>
              {formatBytes(progress.downloadedBytes)} / {formatBytes(progress.totalBytes)}
            </span>
          </div>
          <div className="h-2 overflow-hidden rounded-full bg-surface">
            <div className="h-full rounded-full bg-accent transition-all" style={{ width: `${pct}%` }} />
          </div>
        </div>
      )}

      {done && (
        <p className="text-sm font-medium text-green-600">✓ Model downloaded successfully!</p>
      )}

      {error && <p className="text-sm text-accent-secondary">{error}</p>}

      <div className="flex gap-2">
        {!done ? (
          <>
            <Button
              id="btn-setup-download"
              onClick={() => void handleDownload()}
              disabled={!selected || !!progress}
            >
              Download
            </Button>
            <Button id="btn-setup-skip-model" variant="ghost" onClick={onSkip}>
              Skip for now
            </Button>
          </>
        ) : (
          <Button id="btn-setup-next-2" className="w-full" onClick={onNext}>
            Continue →
          </Button>
        )}
      </div>
    </div>
  );
}

// ─── Step 3: API key (optional) ───────────────────────────────────────────────

function ApiKeyStep({ onFinish }: { onFinish: () => void }) {
  const [key, setKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSave = async () => {
    if (!key.trim()) return;
    setSaving(true);
    setError(null);
    try {
      const valid = await invoke<boolean>("validate_api_key_format", {
        provider: "openai",
        key: key.trim(),
      });
      if (!valid) throw new Error("Key doesn't look like a valid OpenAI API key (should start with sk-)");
      await invoke("store_api_key", { provider: "openai", plaintextKey: key.trim() });
      setSaved(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-5">
      <div>
        <h2 className="font-serif text-xl font-bold text-text-primary">
          Optional: API Key
        </h2>
        <p className="mt-1 text-sm text-text-secondary">
          Add an OpenAI API key to enable the Research Agent (cloud queries use{" "}
          <strong>only sanitized public queries</strong> — never your documents).
        </p>
      </div>

      {!saved ? (
        <>
          <Input
            id="input-setup-openai-key"
            type="password"
            placeholder="sk-..."
            value={key}
            onChange={(e) => setKey(e.target.value)}
          />
          {error && <p className="text-sm text-accent-secondary">{error}</p>}
          <div className="flex gap-2">
            <Button
              id="btn-setup-save-key"
              onClick={() => void handleSave()}
              loading={saving}
              disabled={!key.trim()}
            >
              Save key
            </Button>
            <Button id="btn-setup-skip-key" variant="ghost" onClick={onFinish}>
              Skip
            </Button>
          </div>
        </>
      ) : (
        <div className="space-y-4">
          <p className="text-sm text-green-600">✓ API key saved and encrypted.</p>
          <Button id="btn-setup-finish" className="w-full" onClick={onFinish}>
            Finish setup →
          </Button>
        </div>
      )}
    </div>
  );
}

// ─── Wizard shell ─────────────────────────────────────────────────────────────

export function SetupWizard() {
  const completeSetup = useSettingsStore((s) => s.completeSetup);
  const [step, setStep] = useState<Step>(1);

  const finish = () => void completeSetup();

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm">
      <div className="w-full max-w-md rounded-2xl border border-border bg-white p-8 shadow-2xl">
        {/* Step indicator */}
        <div className="mb-6 flex items-center gap-2">
          {([1, 2, 3] as Step[]).map((s) => (
            <div
              key={s}
              className={cn(
                "h-1.5 flex-1 rounded-full transition-colors",
                s <= step ? "bg-accent" : "bg-surface",
              )}
            />
          ))}
        </div>

        {step === 1 && <WelcomeStep onNext={() => setStep(2)} />}
        {step === 2 && (
          <ModelStep onNext={() => setStep(3)} onSkip={() => setStep(3)} />
        )}
        {step === 3 && <ApiKeyStep onFinish={finish} />}
      </div>
    </div>
  );
}
