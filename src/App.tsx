import { useEffect, useMemo, useState } from "react";

import { Layout } from "@/components/Layout";
import { AgentMonitor } from "@/pages/AgentMonitor";
import { ClaudeHistory } from "@/pages/ClaudeHistory";
import { Dashboard } from "@/pages/Dashboard";
import { ProjectDetail } from "@/pages/ProjectDetail";
import { Settings } from "@/pages/Settings";

export type AppRoute = "dashboard" | "agents" | "settings" | "claude-history";

const STORAGE_KEY = "agent-scope:current-project";

function App() {
  const [activeRoute, setActiveRoute] = useState<AppRoute>("dashboard");
  const [currentProjectPath, setCurrentProjectPath] = useState(() => {
    return localStorage.getItem(STORAGE_KEY) || "";
  });

  useEffect(() => {
    if (currentProjectPath) {
      localStorage.setItem(STORAGE_KEY, currentProjectPath);
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  }, [currentProjectPath]);

  function handleRouteChange(route: AppRoute) {
    setActiveRoute(route);
  }

  function handleSelectProject(projectPath: string) {
    setCurrentProjectPath(projectPath);
  }

  function handleBackToDashboard() {
    setCurrentProjectPath("");
  }

  const page = useMemo(() => {
    switch (activeRoute) {
      case "agents":
        return <AgentMonitor />;
      case "claude-history":
        return <ClaudeHistory />;
      case "settings":
        return <Settings />;
      case "dashboard":
      default:
        if (currentProjectPath) {
          return (
            <ProjectDetail
              projectPath={currentProjectPath}
              onSelectProject={handleSelectProject}
              onBack={handleBackToDashboard}
            />
          );
        }
        return (
          <Dashboard
            onSelectProject={handleSelectProject}
            onNavigateSettings={() => setActiveRoute("settings")}
          />
        );
    }
  }, [activeRoute, currentProjectPath]);

  return (
    <Layout activeRoute={activeRoute} onRouteChange={handleRouteChange}>
      {page}
    </Layout>
  );
}

export default App;
