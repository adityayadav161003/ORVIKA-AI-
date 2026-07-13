import { createHashRouter, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { ChatPage } from "@/pages/ChatPage";
import { DocumentsPage } from "@/pages/DocumentsPage";
import { MediaPage } from "@/pages/MediaPage";
import { ResearchPage } from "@/pages/ResearchPage";
import { TransparencyPage } from "@/pages/TransparencyPage";
import { ModelsPage } from "@/pages/ModelsPage";
import { SettingsPage } from "@/pages/SettingsPage";

export const router = createHashRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to="/chat" replace /> },
      { path: "chat", element: <ChatPage /> },
      { path: "documents", element: <DocumentsPage /> },
      { path: "media", element: <MediaPage /> },
      { path: "research", element: <ResearchPage /> },
      { path: "transparency", element: <TransparencyPage /> },
      { path: "models", element: <ModelsPage /> },
      { path: "settings", element: <SettingsPage /> },
    ],
  },
]);
