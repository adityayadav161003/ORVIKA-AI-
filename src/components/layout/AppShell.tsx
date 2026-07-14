import { useState } from "react";
import { Outlet, useLocation } from "react-router-dom";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";

const PAGE_TITLES: Record<string, string> = {
  "/chat": "Chat",
  "/documents": "Documents",
  "/research": "Research",
  "/transparency": "Transparency",
  "/models": "Model Management",
  "/settings": "Settings",
};

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);
  const location = useLocation();

  const title = PAGE_TITLES[location.pathname] ?? "ORVIKA AI";

  return (
    <div className="flex h-screen overflow-hidden bg-surface">
      {/* Sidebar */}
      <Sidebar collapsed={collapsed} />

      {/* Main area */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Header */}
        <header className="flex items-center gap-3 border-b border-border bg-white px-4 py-3">
          {/* Collapse toggle */}
          <button
            id="btn-toggle-sidebar"
            type="button"
            className="rounded p-1 text-text-muted hover:bg-surface hover:text-accent"
            onClick={() => setCollapsed((c) => !c)}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            <svg viewBox="0 0 20 20" fill="currentColor" className="h-5 w-5">
              <path
                fillRule="evenodd"
                d="M3 5a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 10a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1zM3 15a1 1 0 011-1h12a1 1 0 110 2H4a1 1 0 01-1-1z"
                clipRule="evenodd"
              />
            </svg>
          </button>

          <h1 className="font-serif text-base font-semibold text-text-primary">{title}</h1>
        </header>

        {/* Page content */}
        <main className="flex-1 overflow-y-auto">
          <Outlet />
        </main>

        {/* Status bar */}
        <StatusBar />
      </div>
    </div>
  );
}
