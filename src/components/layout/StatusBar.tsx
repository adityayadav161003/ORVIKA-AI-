import { cn } from "@/utils/cn";
import { useLlmStore } from "@/stores/llmStore";
import { useSettingsStore } from "@/stores/settingsStore";

const PRIVACY_COLORS = {
  strict: "text-green-700 bg-green-50 border-green-200",
  balanced: "text-yellow-700 bg-yellow-50 border-yellow-200",
  open: "text-blue-700 bg-blue-50 border-blue-200",
};

const PRIVACY_LABELS = {
  strict: "🔒 Strict",
  balanced: "⚖ Balanced",
  open: "🌐 Open",
};

export function StatusBar() {
  const llmStatus = useLlmStore((s) => s.status);
  const privacyLevel = useSettingsStore((s) => s.privacyLevel);

  const isRunning = llmStatus?.state === "running" && llmStatus.healthy;
  const isCrashed = llmStatus?.state === "crashed";
  const isStarting = llmStatus?.state === "starting";

  const serverDotColor = isRunning
    ? "bg-green-500"
    : isStarting
      ? "bg-yellow-400 animate-pulse"
      : isCrashed
        ? "bg-accent-secondary"
        : "bg-text-muted/40";

  const modelName = llmStatus?.modelPath
    ? llmStatus.modelPath.split(/[\\/]/).pop()
    : "No model loaded";

  return (
    <footer
      id="status-bar"
      className="flex items-center justify-between border-t border-border bg-white px-4 py-1.5"
    >
      {/* Left: privacy level */}
      <div className="flex items-center gap-3">
        <span
          className={cn(
            "rounded-full border px-2 py-0.5 font-mono text-xs font-medium",
            PRIVACY_COLORS[privacyLevel],
          )}
        >
          {PRIVACY_LABELS[privacyLevel]}
        </span>
      </div>

      {/* Right: model + server state */}
      <div className="flex items-center gap-4 font-mono text-xs text-text-muted">
        <span className="max-w-52 truncate hidden sm:block" title={modelName}>
          {modelName}
        </span>
        <div className="flex items-center gap-1.5">
          <span className={cn("h-2 w-2 rounded-full", serverDotColor)} />
          <span>
            {isRunning
              ? "LLM ready"
              : isStarting
                ? "Starting…"
                : isCrashed
                  ? "Crashed"
                  : "LLM offline"}
          </span>
        </div>
      </div>
    </footer>
  );
}
