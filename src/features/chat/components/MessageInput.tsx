import type { FormEvent, KeyboardEvent } from "react";
import { Button } from "@/components/ui/Button";

interface MessageInputProps {
  value: string;
  disabled?: boolean;
  sending?: boolean;
  isResearchMode?: boolean;
  onChange: (value: string) => void;
  onSubmit: () => void;
  onCancel?: () => void;
  onResearchModeChange?: (enabled: boolean) => void;
}

export function MessageInput({
  value,
  disabled = false,
  sending = false,
  isResearchMode = false,
  onChange,
  onSubmit,
  onCancel,
  onResearchModeChange,
}: MessageInputProps) {
  const canSend = value.trim().length > 0 && !disabled && !sending;

  const handleSubmit = (event: FormEvent) => {
    event.preventDefault();
    if (canSend) onSubmit();
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      if (canSend) onSubmit();
    }
  };

  return (
    <div className="border-t border-border bg-white p-3 flex flex-col gap-2">
      {onResearchModeChange && (
        <div className="flex items-center gap-2 px-1">
          <button
            type="button"
            onClick={() => onResearchModeChange(!isResearchMode)}
            className={`flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-full transition-colors border ${
              isResearchMode
                ? "bg-accent text-white border-accent"
                : "bg-surface text-text-muted border-border hover:text-text-primary"
            }`}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
              <path
                d="M21 15V19C21 19.5304 20.7893 20.0391 20.4142 20.4142C20.0391 20.7893 19.5304 21 19 21H5C4.46957 21 3.96086 20.7893 3.58579 20.4142C3.21071 20.0391 3 19.5304 3 19V15M7 10L12 15M12 15L17 10M12 15V3"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
            Deep Research {isResearchMode ? "ON" : "OFF"}
          </button>
        </div>
      )}
      <form onSubmit={handleSubmit} className="flex items-end gap-2">
        <textarea
          id="chat-message-input"
          value={value}
          rows={1}
          disabled={disabled || sending}
          placeholder={
            isResearchMode ? "Ask a complex question to research..." : "Message the local model"
          }
          className={`max-h-40 min-h-11 flex-1 resize-none rounded-md border bg-surface px-3 py-2 text-sm leading-6 text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 disabled:cursor-not-allowed disabled:opacity-60 transition-colors ${
            isResearchMode
              ? "border-accent/50 focus:border-accent focus:ring-accent/20"
              : "border-border focus:border-accent focus:ring-accent/10"
          }`}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={handleKeyDown}
        />
        {sending && onCancel ? (
          <Button type="button" variant="secondary" onClick={onCancel}>
            Stop
          </Button>
        ) : (
          <Button
            type="submit"
            disabled={!canSend}
            loading={sending}
            className={isResearchMode ? "bg-accent hover:bg-accent/90" : ""}
          >
            Send
          </Button>
        )}
      </form>
    </div>
  );
}
