import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/Button";
import { Spinner } from "@/components/ui/Spinner";
import { cn } from "@/utils/cn";

// ─── Types ───────────────────────────────────────────────────────────────────

interface ResearchSession {
  id: string;
  sessionId: string;
  messageId: string;
  status: "planning" | "approved" | "in_progress" | "completed" | "failed";
  totalQueries: number;
  completedQueries: number;
  knowledgeGaps?: string | null;
  createdAt: string;
}

interface ResearchQuery {
  id: string;
  researchSessionId: string;
  queryIndex: number;
  topic: string;
  rawQuery?: string | null;
  sanitizedQuery: string;
  riskLevel: "low" | "medium" | "high";
  status: "pending" | "approved" | "rejected" | "sent" | "completed" | "failed" | "blocked";
  userApproved?: boolean | null;
  response?: string | null;
  createdAt: string;
}

interface ResearchPlanResult {
  session: ResearchSession;
  queries: ResearchQuery[];
}

interface SessionWithQueryText extends ResearchSession {
  userQuery?: string;
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

const STATUS_CONFIGS = {
  planning: { label: "Planning", style: "bg-yellow-500/10 text-yellow-500 border-yellow-500/20" },
  approved: { label: "Approved", style: "bg-blue-500/10 text-blue-500 border-blue-500/20" },
  in_progress: {
    label: "In Progress",
    style: "bg-orange-500/10 text-orange-500 border-orange-500/20 animate-pulse",
  },
  completed: { label: "Completed", style: "bg-green-500/10 text-green-500 border-green-500/20" },
  failed: { label: "Failed", style: "bg-red-500/10 text-red-500 border-red-500/20" },
  pending: { label: "Pending", style: "bg-text-muted/10 text-text-muted border-text-muted/20" },
  rejected: { label: "Rejected", style: "bg-red-500/5 text-red-500/60 border-red-500/10" },
  sent: {
    label: "Executing",
    style: "bg-orange-500/10 text-orange-400 border-orange-500/20 animate-pulse",
  },
  blocked: { label: "Blocked", style: "bg-red-500/10 text-red-500 border-red-500/20" },
} as const;

function StatusBadge({ status }: { status: keyof typeof STATUS_CONFIGS }) {
  const cfg = STATUS_CONFIGS[status] ?? STATUS_CONFIGS.pending;
  return (
    <span
      className={cn(
        "px-2 py-0.5 rounded text-[10px] font-mono font-medium border uppercase tracking-wider",
        cfg.style,
      )}
    >
      {cfg.label}
    </span>
  );
}

// ─── Main Component ──────────────────────────────────────────────────────────

export function ResearchPage() {
  const [sessions, setSessions] = useState<SessionWithQueryText[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [activeDetails, setActiveDetails] = useState<ResearchPlanResult | null>(null);
  const [loadingList, setLoadingList] = useState(true);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [expandedQueryId, setExpandedQueryId] = useState<string | null>(null);

  // Load all research sessions
  const loadSessions = useCallback(async (preferredId?: string | null) => {
    setLoadingList(true);
    try {
      const list = await invoke<ResearchSession[]>("list_research_sessions");

      // Fetch user query text for each session from messages table to show context
      const enrichedList: SessionWithQueryText[] = await Promise.all(
        list.map(async (sess) => {
          try {
            const specificMsg = await invoke<{ role: string; content: string }[] | null>(
              "get_messages",
              {
                sessionId: sess.sessionId,
                limit: 10,
                offset: 0,
              },
            ).then(
              (msgs) => msgs?.find((m) => m.role === "user")?.content ?? "Hybrid Research Query",
            );

            return { ...sess, userQuery: specificMsg };
          } catch {
            return { ...sess, userQuery: "Hybrid Research Session" };
          }
        }),
      );

      setSessions(enrichedList);

      const nextId = preferredId ?? enrichedList[0]?.id ?? null;
      if (nextId) {
        setSelectedSessionId(nextId);
        await loadDetails(nextId);
      }
    } catch (err) {
      console.error("Error loading research sessions:", err);
    } finally {
      setLoadingList(false);
    }
  }, []);

  const loadDetails = async (id: string) => {
    setLoadingDetails(true);
    try {
      const details = await invoke<ResearchPlanResult>("get_research_session_details", {
        researchSessionId: id,
      });
      setActiveDetails(details);
      if (details.queries.length > 0) {
        setExpandedQueryId(details.queries[0].id);
      }
    } catch (err) {
      console.error("Error loading session details:", err);
    } finally {
      setLoadingDetails(false);
    }
  };

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  const handleSelectSession = (id: string) => {
    setSelectedSessionId(id);
    void loadDetails(id);
  };

  const handleDeleteSession = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    if (!confirm("Are you sure you want to delete this research session history?")) return;
    try {
      await invoke("delete_research_session", { researchSessionId: id });
      const nextId = selectedSessionId === id ? null : selectedSessionId;
      await loadSessions(nextId);
    } catch (err) {
      console.error("Error deleting session:", err);
    }
  };

  const handleExportReport = () => {
    if (!activeDetails) return;
    const sess = activeDetails.session;
    const queries = activeDetails.queries;

    let md = `# Research Report — Session ${sess.id.substring(0, 8)}\n`;
    md += `*Generated: ${new Date(sess.createdAt).toLocaleString()}*\n\n`;

    if (sess.knowledgeGaps) {
      md += `## Knowledge Gaps Identified\n> ${sess.knowledgeGaps}\n\n`;
    }

    md += `## Web Search Findings\n\n`;
    for (const q of queries) {
      if (q.status === "completed" && q.response) {
        md += `### ${q.topic}\n`;
        md += `**Query:** \`${q.sanitizedQuery}\`\n\n`;
        md += `${q.response}\n\n`;
        md += `---\n\n`;
      }
    }

    const blob = new Blob([md], { type: "text/markdown;charset=utf-8;" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.setAttribute("download", `research_report_${sess.id.substring(0, 8)}.md`);
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  };

  return (
    <div className="flex h-full min-h-[640px] bg-surface overflow-hidden">
      {/* Left panel: List */}
      <aside className="w-80 shrink-0 border-r border-border bg-white flex flex-col">
        <div className="p-4 border-b border-border">
          <h2 className="font-serif text-lg font-bold text-accent">Research History</h2>
          <p className="text-xs text-text-muted mt-0.5">
            View your local and cloud hybrid intelligence logs.
          </p>
        </div>

        <div className="flex-1 overflow-y-auto p-3 space-y-2">
          {loadingList ? (
            <div className="flex items-center gap-2 text-sm text-text-muted justify-center py-8">
              <Spinner /> Loading history...
            </div>
          ) : sessions.length === 0 ? (
            <div className="text-text-muted italic text-center text-sm py-8">
              No research sessions yet.
            </div>
          ) : (
            sessions.map((sess) => (
              <div
                key={sess.id}
                onClick={() => handleSelectSession(sess.id)}
                className={cn(
                  "p-3 rounded-lg border text-left cursor-pointer relative group transition-all hover:shadow-sm",
                  selectedSessionId === sess.id
                    ? "bg-accent-light border-accent/30 shadow-sm"
                    : "bg-surface border-border hover:border-accent/20",
                )}
              >
                <button
                  onClick={(e) => handleDeleteSession(sess.id, e)}
                  className="absolute right-2 top-2 text-text-muted hover:text-red-500 opacity-0 group-hover:opacity-100 transition-opacity p-0.5"
                  title="Delete Log"
                >
                  ×
                </button>
                <h4
                  className="font-medium text-xs text-text-primary truncate pr-4"
                  title={sess.userQuery}
                >
                  {sess.userQuery}
                </h4>
                <div className="flex items-center justify-between mt-2.5">
                  <span className="text-[10px] text-text-muted font-mono">
                    {new Date(sess.createdAt).toLocaleDateString()}
                  </span>
                  <div className="flex items-center gap-1.5">
                    <span className="text-[10px] text-text-secondary font-mono">
                      {sess.completedQueries}/{sess.totalQueries}
                    </span>
                    <StatusBadge status={sess.status} />
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      </aside>

      {/* Right panel: Details */}
      <main className="flex-1 overflow-y-auto bg-white flex flex-col min-w-0">
        {loadingDetails ? (
          <div className="flex-1 flex items-center justify-center gap-2 text-text-muted">
            <Spinner /> Loading details...
          </div>
        ) : !activeDetails ? (
          <div className="flex-1 flex flex-col items-center justify-center p-8 text-center text-text-muted">
            <div className="text-4xl mb-4">🔍</div>
            <h3 className="font-serif text-lg font-semibold text-text-primary">
              No Session Selected
            </h3>
            <p className="text-sm max-w-xs mt-1 text-text-muted leading-relaxed">
              Select a research session from the sidebar or start a new research chat with RAG.
            </p>
          </div>
        ) : (
          <div className="flex-1 flex flex-col max-w-4xl mx-auto w-full p-6 space-y-6">
            {/* Header */}
            <header className="flex flex-wrap items-start justify-between gap-4 border-b border-border pb-4">
              <div className="min-w-0 flex-1">
                <h1 className="font-serif text-2xl font-bold text-text-primary">
                  {sessions.find((s) => s.id === activeDetails.session.id)?.userQuery ??
                    "Research Session"}
                </h1>
                <div className="flex flex-wrap items-center gap-3 mt-1 text-xs text-text-muted font-mono">
                  <span>ID: {activeDetails.session.id.substring(0, 12)}...</span>
                  <span>•</span>
                  <span>Created: {new Date(activeDetails.session.createdAt).toLocaleString()}</span>
                </div>
              </div>
              <div className="flex items-center gap-2">
                <StatusBadge status={activeDetails.session.status} />
                {activeDetails.session.status === "completed" && (
                  <Button size="sm" onClick={handleExportReport}>
                    Export Report (.md)
                  </Button>
                )}
              </div>
            </header>

            {/* Knowledge Gaps */}
            {activeDetails.session.knowledgeGaps && (
              <section className="p-4 bg-orange-500/5 border border-orange-500/10 rounded-lg">
                <h3 className="text-xs font-semibold text-orange-800 uppercase tracking-wider flex items-center gap-1.5">
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    className="text-orange-600"
                  >
                    <path
                      d="M12 9V14M12 17.5V18M21 12C21 16.9706 16.9706 21 12 21C7.02944 21 3 16.9706 3 12C3 7.02944 7.02944 3 12 3C16.9706 3 21 7.02944 21 12Z"
                      stroke="currentColor"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                  Identified Knowledge Gaps
                </h3>
                <p className="text-sm text-text-secondary mt-1.5 leading-relaxed">
                  {activeDetails.session.knowledgeGaps}
                </p>
              </section>
            )}

            {/* Queries & Results */}
            <section className="space-y-4">
              <h2 className="text-sm font-semibold text-text-primary uppercase tracking-wider flex items-center gap-2">
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  className="text-blue-500"
                >
                  <path
                    d="M4 6H20M4 12H20M4 18H20"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  />
                </svg>
                Research Targets & Findings ({activeDetails.queries.length})
              </h2>

              <div className="space-y-3">
                {activeDetails.queries.map((q) => {
                  const isExpanded = expandedQueryId === q.id;
                  return (
                    <div
                      key={q.id}
                      className={cn(
                        "border rounded-lg overflow-hidden transition-all",
                        isExpanded
                          ? "border-accent/40 shadow-sm"
                          : "border-border hover:border-accent/20",
                      )}
                    >
                      {/* Query Trigger Row */}
                      <div
                        onClick={() => setExpandedQueryId(isExpanded ? null : q.id)}
                        className="flex items-center justify-between p-4 bg-surface cursor-pointer select-none"
                      >
                        <div className="flex items-center gap-3 min-w-0">
                          <span className="font-mono text-xs text-text-muted bg-white border border-border px-1.5 py-0.5 rounded">
                            #{q.queryIndex + 1}
                          </span>
                          <span className="font-medium text-sm text-text-primary truncate">
                            {q.topic}
                          </span>
                        </div>
                        <div className="flex items-center gap-3 shrink-0">
                          <StatusBadge status={q.status} />
                          <svg
                            width="16"
                            height="16"
                            viewBox="0 0 24 24"
                            fill="none"
                            className={cn(
                              "text-text-muted transition-transform duration-200",
                              isExpanded && "rotate-180",
                            )}
                          >
                            <path
                              d="M6 9L12 15L18 9"
                              stroke="currentColor"
                              strokeWidth="2"
                              strokeLinecap="round"
                              strokeLinejoin="round"
                            />
                          </svg>
                        </div>
                      </div>

                      {/* Query Details & Content */}
                      {isExpanded && (
                        <div className="p-4 border-t border-border bg-white space-y-3">
                          <div className="flex flex-wrap items-center gap-4 text-xs font-mono">
                            <div className="text-text-muted">
                              Sanitized Query:{" "}
                              <span className="text-text-secondary select-all font-semibold">
                                "{q.sanitizedQuery}"
                              </span>
                            </div>
                            <div className="flex items-center gap-1 text-text-muted">
                              Risk Level:
                              <span
                                className={cn(
                                  "px-1.5 py-0.5 rounded text-[10px] font-bold uppercase",
                                  q.riskLevel === "high" && "bg-red-500/10 text-red-500",
                                  q.riskLevel === "medium" && "bg-yellow-500/10 text-yellow-600",
                                  q.riskLevel === "low" && "bg-green-500/10 text-green-600",
                                )}
                              >
                                {q.riskLevel}
                              </span>
                            </div>
                          </div>

                          <div className="space-y-1.5">
                            <h4 className="text-xs font-bold text-text-secondary uppercase tracking-wider">
                              Findings Summary
                            </h4>
                            {q.status === "completed" && q.response ? (
                              <div className="p-3.5 bg-surface border border-border rounded text-sm text-text-primary leading-relaxed whitespace-pre-wrap font-sans">
                                {q.response}
                              </div>
                            ) : q.status === "rejected" ? (
                              <p className="text-sm text-red-500 italic">
                                This query was rejected or unchecked by the user.
                              </p>
                            ) : q.status === "sent" ? (
                              <p className="text-sm text-text-muted italic flex items-center gap-2">
                                <Spinner /> Querying cloud endpoints...
                              </p>
                            ) : (
                              <p className="text-sm text-text-muted italic">Pending execution.</p>
                            )}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
