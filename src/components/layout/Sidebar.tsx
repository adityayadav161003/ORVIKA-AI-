import { NavLink } from "react-router-dom";
import { cn } from "@/utils/cn";
import { useLlmStore } from "@/stores/llmStore";

// ─── Nav items ───────────────────────────────────────────────────────────────

interface NavItem {
  to: string;
  label: string;
  icon: string;
  comingSoon?: boolean;
}

const NAV: NavItem[] = [
  { to: "/chat", label: "Chat", icon: "💬" },
  { to: "/documents", label: "Documents", icon: "📄" },
  { to: "/media", label: "Media", icon: "🎬" },
  { to: "/research", label: "Research", icon: "🔍" },
  { to: "/transparency", label: "Transparency", icon: "🛡" },
  { to: "/models", label: "Models", icon: "⚙" },
  { to: "/settings", label: "Settings", icon: "⚙" },
];

// prettier-ignore
const NAV_ICONS: Record<string, React.ReactNode> = {
  "/chat": <ChatIcon />,
  "/documents": <DocumentIcon />,
  "/media": <MediaIcon />,
  "/research": <ResearchIcon />,
  "/transparency": <ShieldIcon />,
  "/models": <ModelIcon />,
  "/settings": <SettingsIcon />,
};

// ─── SVG icons ───────────────────────────────────────────────────────────────

function ChatIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path d="M2 5a2 2 0 012-2h7a2 2 0 012 2v4a2 2 0 01-2 2H9l-3 3v-3H4a2 2 0 01-2-2V5z" />
      <path d="M15 7v2a4 4 0 01-4 4H9.828l-1.766 1.767c.28.149.599.233.938.233h2l3 3v-3h2a2 2 0 002-2V9a2 2 0 00-2-2h-1z" />
    </svg>
  );
}

function DocumentIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path
        fillRule="evenodd"
        d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4zm2 6a1 1 0 011-1h6a1 1 0 110 2H7a1 1 0 01-1-1zm1 3a1 1 0 100 2h6a1 1 0 100-2H7z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function ResearchIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path
        fillRule="evenodd"
        d="M8 4a4 4 0 100 8 4 4 0 000-8zM2 8a6 6 0 1110.89 3.476l4.817 4.817a1 1 0 01-1.414 1.414l-4.816-4.816A6 6 0 012 8z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function ShieldIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path
        fillRule="evenodd"
        d="M2.166 4.999A11.954 11.954 0 0010 1.944 11.954 11.954 0 0017.834 5c.11.65.166 1.32.166 2.001 0 5.225-3.34 9.67-8 11.317C5.34 16.67 2 12.225 2 7c0-.682.057-1.35.166-2.001zm11.541 3.708a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function ModelIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path d="M3 12v3c0 1.657 3.134 3 7 3s7-1.343 7-3v-3c0 1.657-3.134 3-7 3s-7-1.343-7-3z" />
      <path d="M3 7v3c0 1.657 3.134 3 7 3s7-1.343 7-3V7c0 1.657-3.134 3-7 3S3 8.657 3 7z" />
      <path d="M17 5c0 1.657-3.134 3-7 3S3 6.657 3 5s3.134-3 7-3 7 1.343 7 3z" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path
        fillRule="evenodd"
        d="M11.49 3.17c-.38-1.56-2.6-1.56-2.98 0a1.532 1.532 0 01-2.286.948c-1.372-.836-2.942.734-2.106 2.106.54.886.061 2.042-.947 2.287-1.561.379-1.561 2.6 0 2.978a1.532 1.532 0 01.947 2.287c-.836 1.372.734 2.942 2.106 2.106a1.532 1.532 0 012.287.947c.379 1.561 2.6 1.561 2.978 0a1.533 1.533 0 012.287-.947c1.372.836 2.942-.734 2.106-2.106a1.533 1.533 0 01.947-2.287c1.561-.379 1.561-2.6 0-2.978a1.532 1.532 0 01-.947-2.287c.836-1.372-.734-2.942-2.106-2.106a1.532 1.532 0 01-2.287-.947zM10 13a3 3 0 100-6 3 3 0 000 6z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function MediaIcon() {
  return (
    <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
      <path
        fillRule="evenodd"
        d="M4 3a2 2 0 00-2 2v10a2 2 0 002 2h12a2 2 0 002-2V5a2 2 0 00-2-2H4zm3 2h2v2H7V5zm4 0h2v2h-2V5zM5 9v6h10V9H5z"
        clipRule="evenodd"
      />
    </svg>
  );
}

// ─── Sidebar component ────────────────────────────────────────────────────────

interface SidebarProps {
  collapsed: boolean;
}

export function Sidebar({ collapsed }: SidebarProps) {
  const llmStatus = useLlmStore((s) => s.status);
  const isOnline = llmStatus?.state === "running" && llmStatus.healthy;
  const isCrashed = llmStatus?.state === "crashed";

  const dotColor = isOnline
    ? "bg-green-500"
    : isCrashed
      ? "bg-accent-secondary"
      : "bg-text-muted/40";

  return (
    <aside
      className={cn(
        "flex h-full flex-col border-r border-border bg-white transition-all duration-200",
        collapsed ? "w-14" : "w-52",
      )}
    >
      {/* Logo mark */}
      <div
        className={cn(
          "flex items-center gap-2.5 border-b border-border px-3 py-4",
          collapsed && "justify-center px-0",
        )}
      >
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-accent">
          <span className="font-mono text-sm font-bold text-white">P</span>
        </div>
        {!collapsed && (
          <div className="min-w-0">
            <p className="truncate font-mono text-xs font-semibold uppercase tracking-widest text-accent">
              Private AI
            </p>
          </div>
        )}
      </div>

      {/* Nav */}
      <nav className="flex-1 overflow-y-auto py-3">
        {NAV.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            id={`nav${item.to.replace("/", "-")}`}
            className={({ isActive }) =>
              cn(
                "group flex items-center gap-3 px-3 py-2 text-sm transition-colors",
                collapsed && "justify-center px-0 py-3",
                isActive
                  ? "bg-accent-light text-accent"
                  : "text-text-secondary hover:bg-surface hover:text-accent",
                item.comingSoon && "opacity-40 pointer-events-none",
              )
            }
            title={collapsed ? item.label : undefined}
          >
            {({ isActive }) => (
              <>
                <span
                  className={cn(
                    "shrink-0 transition-colors",
                    isActive ? "text-accent" : "text-text-muted group-hover:text-accent",
                  )}
                >
                  {NAV_ICONS[item.to]}
                </span>
                {!collapsed && (
                  <span className="truncate font-medium">
                    {item.label}
                    {item.comingSoon && (
                      <span className="ml-1.5 rounded bg-surface px-1 py-0.5 font-mono text-xs text-text-muted">
                        soon
                      </span>
                    )}
                  </span>
                )}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      {/* LLM status at bottom */}
      <div
        className={cn(
          "flex items-center gap-2 border-t border-border px-3 py-3",
          collapsed && "justify-center px-0",
        )}
      >
        <span className={cn("h-2 w-2 shrink-0 rounded-full", dotColor)} />
        {!collapsed && (
          <span className="truncate font-mono text-xs text-text-muted">
            {isOnline ? "LLM online" : isCrashed ? "LLM crashed" : "LLM offline"}
          </span>
        )}
      </div>
    </aside>
  );
}
