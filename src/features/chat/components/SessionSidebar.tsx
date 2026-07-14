import { Button } from "@/components/ui/Button";
import { cn } from "@/utils/cn";
import type { ChatSession } from "../types";

interface SessionSidebarProps {
  sessions: ChatSession[];
  activeSessionId: string | null;
  creating: boolean;
  deletingSessionId: string | null;
  disabled?: boolean;
  onCreate: () => void;
  onSelect: (sessionId: string) => void;
  onDelete: (sessionId: string) => void;
}

function formatDate(value: string) {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return "";
  return parsed.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function SessionSidebar({
  sessions,
  activeSessionId,
  creating,
  deletingSessionId,
  disabled = false,
  onCreate,
  onSelect,
  onDelete,
}: SessionSidebarProps) {
  return (
    <aside className="flex h-full w-full flex-col border-r border-border bg-white md:w-72">
      <div className="flex items-center justify-between gap-2 border-b border-border px-3 py-3">
        <h2 className="font-mono text-xs font-semibold uppercase text-text-muted">Sessions</h2>
        <Button size="sm" onClick={onCreate} disabled={creating || disabled}>
          New
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {sessions.length === 0 ? (
          <div className="px-3 py-8 text-sm text-text-muted">No sessions yet.</div>
        ) : (
          <ul className="space-y-1">
            {sessions.map((session) => {
              const active = session.id === activeSessionId;
              const deleting = deletingSessionId === session.id;
              return (
                <li key={session.id}>
                  <div
                    className={cn(
                      "group grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 rounded-md border px-3 py-2 transition-colors",
                      active
                        ? "border-accent/30 bg-accent-light"
                        : "border-transparent hover:border-border hover:bg-surface",
                    )}
                  >
                    <button
                      type="button"
                      className="min-w-0 text-left"
                      disabled={disabled}
                      onClick={() => onSelect(session.id)}
                    >
                      <span className="block truncate text-sm font-medium text-text-primary">
                        {session.name}
                      </span>
                      <span className="mt-0.5 block truncate font-mono text-xs text-text-muted">
                        {session.messageCount} messages
                        {session.updatedAt ? ` - ${formatDate(session.updatedAt)}` : ""}
                      </span>
                    </button>
                    <button
                      type="button"
                      aria-label={`Delete ${session.name}`}
                      className="rounded px-2 py-1 font-mono text-xs text-text-muted opacity-70 hover:bg-white hover:text-accent-secondary disabled:cursor-not-allowed disabled:opacity-30 group-hover:opacity-100"
                      disabled={disabled || deleting}
                      onClick={() => onDelete(session.id)}
                    >
                      {deleting ? "..." : "Del"}
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </aside>
  );
}
