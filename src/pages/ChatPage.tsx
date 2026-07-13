import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Button } from "@/components/ui/Button";
import { Spinner } from "@/components/ui/Spinner";
import { Select } from "@/components/ui/Select";
import { MessageList } from "@/features/chat/components/MessageList";
import { MessageInput } from "@/features/chat/components/MessageInput";
import { SessionSidebar } from "@/features/chat/components/SessionSidebar";
import { ResearchPlanModal } from "@/features/chat/components/ResearchPlanModal";
import { formatChatForExport, downloadStringAsFile } from "@/features/chat/utils/export";
import type {
  ChatMessage,
  ChatSession,
  DownloadedModel,
  SendMessageResult,
  ResearchPlanResult,
} from "@/features/chat/types";
import { useLlmStore } from "@/stores/llmStore";
import { cn } from "@/utils/cn";

function makePendingMessage(
  role: "user" | "assistant",
  sessionId: string,
  content = "",
): ChatMessage {
  return {
    id: `pending-${role}-${Date.now()}`,
    sessionId,
    role,
    content,
    sourceType: role === "assistant" ? "local" : null,
    sources: null,
    createdAt: new Date().toISOString(),
    tokensUsed: null,
    latencyMs: null,
    metadata: null,
  };
}

function runtimeLabel(state: string | undefined, healthy: boolean | undefined) {
  if (state === "running" && healthy) return "LLM ready";
  if (state === "starting") return "Starting";
  if (state === "crashed") return "Runtime error";
  return "LLM offline";
}

export function ChatPage() {
  const llmStatus = useLlmStore((state) => state.status);
  const refreshLlm = useLlmStore((state) => state.refresh);
  const [sessions, setSessions] = useState<ChatSession[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [loading, setLoading] = useState(true);
  const [loadingMessages, setLoadingMessages] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [creating, setCreating] = useState(false);
  const [deletingSessionId, setDeletingSessionId] = useState<string | null>(null);
  const [startingRuntime, setStartingRuntime] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editingPrompt, setEditingPrompt] = useState(false);
  const [promptDraft, setPromptDraft] = useState("");
  const activeSessionIdRef = useRef<string | null>(null);

  const [isResearchMode, setIsResearchMode] = useState(false);
  const [researchPlan, setResearchPlan] = useState<ResearchPlanResult | null>(null);

  const activeSession = useMemo(
    () => sessions.find((session) => session.id === activeSessionId) ?? null,
    [activeSessionId, sessions],
  );

  const [providerOverride, setProviderOverride] = useState<string>("default");

  useEffect(() => {
    if (activeSession) {
      setProviderOverride(activeSession.cloudProvider ?? "default");
    } else {
      setProviderOverride("default");
    }
  }, [activeSession]);
  const runtimeReady = llmStatus?.state === "running" && llmStatus.healthy;
  const busy = streaming || creating || Boolean(deletingSessionId);

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId;
    if (activeSessionId) {
      const savedDraft = localStorage.getItem(`chat-draft-${activeSessionId}`);
      if (savedDraft) setDraft(savedDraft);
      else setDraft("");
    } else {
      setDraft("");
    }
  }, [activeSessionId]);

  useEffect(() => {
    if (activeSessionId) {
      if (draft) {
        localStorage.setItem(`chat-draft-${activeSessionId}`, draft);
      } else {
        localStorage.removeItem(`chat-draft-${activeSessionId}`);
      }
    }
  }, [draft, activeSessionId]);

  const loadMessages = useCallback(async (sessionId: string) => {
    setLoadingMessages(true);
    try {
      const loaded = await invoke<ChatMessage[]>("get_messages", {
        sessionId,
        limit: 200,
        offset: 0,
      });
      setMessages(loaded);
    } finally {
      setLoadingMessages(false);
    }
  }, []);

  const refreshSessions = useCallback(
    async (preferredSessionId?: string | null) => {
      const loaded = await invoke<ChatSession[]>("list_sessions");
      setSessions(loaded);

      const current = activeSessionIdRef.current;
      const nextSessionId =
        (preferredSessionId && loaded.some((session) => session.id === preferredSessionId)
          ? preferredSessionId
          : null) ??
        (current && loaded.some((session) => session.id === current) ? current : null) ??
        loaded[0]?.id ??
        null;

      setActiveSessionId(nextSessionId);
      if (nextSessionId) {
        await loadMessages(nextSessionId);
      } else {
        setMessages([]);
      }
    },
    [loadMessages],
  );

  useEffect(() => {
    let mounted = true;
    async function load() {
      setLoading(true);
      setError(null);
      try {
        await refreshSessions();
      } catch (err) {
        if (mounted) setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (mounted) setLoading(false);
      }
    }

    void load();
    return () => {
      mounted = false;
    };
  }, [refreshSessions]);

  const resolveModelId = async () => {
    try {
      const models = await invoke<DownloadedModel[]>("list_downloaded_models");
      return models.find((model) => model.isActive)?.id ?? models[0]?.id ?? "local-default";
    } catch {
      return "local-default";
    }
  };

  const handleCreateSession = async () => {
    setCreating(true);
    setError(null);
    try {
      const modelId = await resolveModelId();
      const session = await invoke<ChatSession>("create_session", {
        name: "New chat",
        modelId,
      });
      setActiveSessionId(session.id);
      setMessages([]);
      await refreshSessions(session.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setCreating(false);
    }
  };

  const handleSelectSession = async (sessionId: string) => {
    if (sessionId === activeSessionId || streaming) return;
    setActiveSessionId(sessionId);
    setError(null);
    setEditingPrompt(false);
    try {
      await loadMessages(sessionId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleDeleteSession = async (sessionId: string) => {
    setDeletingSessionId(sessionId);
    setError(null);
    try {
      await invoke("delete_session", { sessionId });
      const nextId =
        activeSessionId === sessionId
          ? (sessions.find((session) => session.id !== sessionId)?.id ?? null)
          : activeSessionId;
      await refreshSessions(nextId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setDeletingSessionId(null);
    }
  };

  const handleStartRuntime = async () => {
    setStartingRuntime(true);
    setError(null);
    try {
      await invoke("start_llm_server");
      await refreshLlm();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStartingRuntime(false);
    }
  };

  const handleSend = async () => {
    if (!activeSessionId || !draft.trim() || streaming) return;

    const sessionId = activeSessionId;
    const content = draft.trim();

    setDraft("");
    setStreaming(true);
    setError(null);

    if (isResearchMode) {
      try {
        const plan = await invoke<ResearchPlanResult>("generate_research_plan", {
          sessionId,
          message: content,
          contextChunks: [], // We rely on backend RAG for this right now, actually backend RAG isn't integrated perfectly yet, wait let's just pass empty
        });
        if (plan.session.status === "approved") {
          // Skip review and execute research immediately if auto-approved
          void handleApproveResearch(plan.session.id);
        } else {
          setResearchPlan(plan);
        }
        await loadMessages(sessionId);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setStreaming(false);
      }
      return;
    }

    const pendingUser = makePendingMessage("user", sessionId, content);
    const pendingAssistant = makePendingMessage("assistant", sessionId);

    setMessages((current) => [...current, pendingUser, pendingAssistant]);

    try {
      const channel = new Channel<string>();
      channel.onmessage = (token) => {
        setMessages((current) =>
          current.map((message) =>
            message.id === pendingAssistant.id
              ? { ...message, content: message.content + token }
              : message,
          ),
        );
      };

      const result = await invoke<SendMessageResult>("send_message", {
        sessionId,
        content,
        maxTokens: 768,
        temperature: 0.7,
        onToken: channel,
      });

      setMessages((current) =>
        current.map((message) => {
          if (message.id === pendingUser.id) return result.userMessage;
          if (message.id === pendingAssistant.id) return result.assistantMessage;
          return message;
        }),
      );
      await refreshSessions(sessionId);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await loadMessages(sessionId).catch(() => undefined);
    } finally {
      setStreaming(false);
    }
  };

  const handleApproveResearch = async (researchSessionId: string) => {
    setResearchPlan(null);
    setStreaming(true);
    setError(null);

    const pendingAssistant = makePendingMessage("assistant", activeSessionId!);
    pendingAssistant.content =
      "⏳ Generating web research queries & executing privacy sanitization...";
    setMessages((current) => [...current, pendingAssistant]);

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await listen<{ status: string }>("research-status-update", (event) => {
        setMessages((current) => {
          const last = current[current.length - 1];
          if (last && last.role === "assistant" && last.id.startsWith("pending-")) {
            return current.slice(0, -1).concat({
              ...last,
              content: event.payload.status,
            });
          }
          return current;
        });
      });

      const channel = new Channel<string>();
      let cleared = false;
      channel.onmessage = (token) => {
        setMessages((current) =>
          current.map((message) => {
            if (message.id === pendingAssistant.id) {
              const newContent = cleared ? message.content + token : token;
              cleared = true;
              return { ...message, content: newContent };
            }
            return message;
          }),
        );
      };

      await invoke("execute_research", {
        researchSessionId,
        onToken: channel,
      });

      await loadMessages(activeSessionId!);
      await refreshSessions(activeSessionId!);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      await loadMessages(activeSessionId!).catch(() => undefined);
    } finally {
      if (unlisten) unlisten();
      setStreaming(false);
    }
  };

  const handleSavePrompt = async () => {
    if (!activeSession) return;
    try {
      await invoke("update_session_system_prompt", {
        sessionId: activeSession.id,
        prompt: promptDraft.trim() || null,
      });
      await invoke("update_session_cloud_provider", {
        sessionId: activeSession.id,
        provider: providerOverride === "default" ? null : providerOverride,
      });
      await refreshSessions(activeSession.id);
      setEditingPrompt(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleExport = () => {
    if (!activeSession) return;
    const content = formatChatForExport(activeSession, messages);
    const filename = `${activeSession.name.replace(/[^a-z0-9]/gi, "_").toLowerCase()}_export.md`;
    downloadStringAsFile(content, filename);
  };

  const statusLabel = runtimeLabel(llmStatus?.state, llmStatus?.healthy);
  const sendDisabled = !activeSessionId || !runtimeReady || loadingMessages;

  return (
    <div className="flex h-full min-h-[720px] flex-col overflow-hidden bg-surface md:flex-row">
      <div className="h-56 shrink-0 md:h-full">
        <SessionSidebar
          sessions={sessions}
          activeSessionId={activeSessionId}
          creating={creating}
          deletingSessionId={deletingSessionId}
          disabled={busy}
          onCreate={() => void handleCreateSession()}
          onSelect={(sessionId) => void handleSelectSession(sessionId)}
          onDelete={(sessionId) => void handleDeleteSession(sessionId)}
        />
      </div>

      <section className="flex min-w-0 flex-1 flex-col overflow-hidden">
        <header className="border-b border-border bg-white px-4 py-3">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <h2 className="truncate font-serif text-xl font-semibold text-text-primary">
                  {activeSession?.name ?? "Chat"}
                </h2>
                {activeSession && (
                  <button
                    type="button"
                    onClick={() => {
                      setPromptDraft(activeSession.systemPrompt ?? "");
                      setEditingPrompt(!editingPrompt);
                    }}
                    className="rounded px-2 py-0.5 text-xs text-text-muted hover:bg-surface hover:text-text-primary"
                  >
                    {editingPrompt ? "Cancel edit" : "Settings"}
                  </button>
                )}
                {activeSession && (
                  <button
                    type="button"
                    onClick={handleExport}
                    className="rounded px-2 py-0.5 text-xs text-text-muted hover:bg-surface hover:text-text-primary"
                  >
                    Export
                  </button>
                )}
              </div>
              <p className="font-mono text-xs text-text-muted">
                {activeSession
                  ? `${activeSession.messageCount} saved messages`
                  : "No active session"}
              </p>
            </div>

            <div className="flex items-center gap-2">
              <span
                className={cn(
                  "rounded-full border px-2.5 py-1 font-mono text-xs",
                  runtimeReady
                    ? "border-green-200 bg-green-50 text-green-700"
                    : "border-border bg-surface text-text-muted",
                )}
              >
                {statusLabel}
              </span>
              {!runtimeReady && (
                <Button
                  size="sm"
                  variant="secondary"
                  loading={startingRuntime}
                  onClick={() => void handleStartRuntime()}
                >
                  Start
                </Button>
              )}
            </div>
          </div>

          {error && (
            <div className="mt-3 flex items-start gap-3 rounded-md border border-accent-secondary/30 bg-accent-secondary/5 px-3 py-2 text-sm text-accent-secondary">
              <p className="min-w-0 flex-1">{error}</p>
              <button
                type="button"
                className="font-mono text-xs text-accent-secondary/70 hover:text-accent-secondary"
                onClick={() => setError(null)}
              >
                Close
              </button>
            </div>
          )}

          {editingPrompt && activeSession && (
            <div className="mt-3 rounded-md border border-border bg-surface p-3 space-y-3">
              <div>
                <label
                  htmlFor="system-prompt"
                  className="mb-1 block text-sm font-medium text-text-primary"
                >
                  Session System Prompt
                </label>
                <p className="mb-2 text-xs text-text-muted">
                  Leave empty to use the global default prompt.
                </p>
                <textarea
                  id="system-prompt"
                  value={promptDraft}
                  onChange={(e) => setPromptDraft(e.target.value)}
                  className="h-20 w-full resize-y rounded-md border border-border bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent/30"
                  placeholder="e.g. You are a helpful assistant..."
                />
              </div>

              <div>
                <label
                  htmlFor="cloud-provider-override"
                  className="mb-1 block text-sm font-medium text-text-primary"
                >
                  Research Provider Override
                </label>
                <p className="mb-2 text-xs text-text-muted">
                  Override the global default cloud research provider for this session.
                </p>
                <Select
                  id="cloud-provider-override"
                  value={providerOverride}
                  onChange={(e) => setProviderOverride(e.target.value)}
                  options={[
                    { value: "default", label: "Use Global Default" },
                    { value: "openai", label: "OpenAI" },
                    { value: "gemini", label: "Google Gemini" },
                    { value: "anthropic", label: "Anthropic Claude" },
                  ]}
                  className="text-xs py-1.5 focus:ring-2 focus:ring-accent/40"
                />
              </div>

              <div className="mt-2 flex justify-end gap-2 pt-2 border-t border-border">
                <Button size="sm" variant="ghost" onClick={() => setEditingPrompt(false)}>
                  Cancel
                </Button>
                <Button size="sm" onClick={() => void handleSavePrompt()}>
                  Save Settings
                </Button>
              </div>
            </div>
          )}
        </header>

        <div className="min-h-0 flex-1 overflow-hidden">
          {loading || loadingMessages ? (
            <div className="flex h-full items-center justify-center gap-2 text-sm text-text-muted">
              <Spinner />
              Loading chat
            </div>
          ) : messages.length > 0 ? (
            <MessageList messages={messages} streaming={streaming} />
          ) : (
            <div className="flex h-full items-center justify-center px-4 text-center">
              <div>
                <h3 className="font-serif text-2xl font-semibold text-text-primary">
                  {activeSession ? "Ready" : "Create a session"}
                </h3>
                <p className="mt-2 max-w-sm text-sm text-text-secondary">
                  {activeSession
                    ? "Your local conversation starts here."
                    : "Sessions keep chat history separate and persistent."}
                </p>
                {!activeSession && (
                  <Button
                    className="mt-4"
                    loading={creating}
                    onClick={() => void handleCreateSession()}
                  >
                    New session
                  </Button>
                )}
              </div>
            </div>
          )}
        </div>

        <MessageInput
          value={draft}
          disabled={sendDisabled}
          sending={streaming}
          isResearchMode={isResearchMode}
          onChange={setDraft}
          onSubmit={() => void handleSend()}
          onCancel={() => {
            void invoke("cancel_chat_stream");
            setStreaming(false);
          }}
          onResearchModeChange={setIsResearchMode}
        />
      </section>
      {researchPlan && (
        <ResearchPlanModal
          plan={researchPlan}
          onApprove={() => handleApproveResearch(researchPlan.session.id)}
          onCancel={() => setResearchPlan(null)}
        />
      )}
    </div>
  );
}
