import { useEffect } from "react";
import { RouterProvider } from "react-router-dom";
import { router } from "./router";
import { SetupWizard } from "@/components/onboarding/SetupWizard";
import { useSettingsStore } from "@/stores/settingsStore";

function App() {
  const { loaded, isFirstRun, load } = useSettingsStore();

  useEffect(() => {
    void load();
  }, [load]);

  if (!loaded) {
    // Minimal loading splash while settings are fetched
    return (
      <div className="flex h-screen items-center justify-center bg-surface">
        <div className="flex items-center gap-3 text-text-muted">
          <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-accent border-r-transparent" />
          <span className="font-mono text-sm">Starting…</span>
        </div>
      </div>
    );
  }

  return (
    <>
      <RouterProvider router={router} />
      {isFirstRun && <SetupWizard />}
    </>
  );
}

export default App;
