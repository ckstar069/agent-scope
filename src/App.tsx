import { useMemo, useState } from "react";

import { Layout } from "@/components/Layout";
import { AgentMonitor } from "@/pages/AgentMonitor";
import { Dashboard } from "@/pages/Dashboard";
import { ProjectDetail } from "@/pages/ProjectDetail";
import { Settings } from "@/pages/Settings";

export type AppRoute = "dashboard" | "projects" | "agents" | "settings";

function App() {
  const [activeRoute, setActiveRoute] = useState<AppRoute>("dashboard");
  const [currentProjectPath, setCurrentProjectPath] = useState("");

  function handleRouteChange(route: AppRoute, projectPath?: string) {
    if (projectPath !== undefined) {
      setCurrentProjectPath(projectPath);
    }
    setActiveRoute(route);
  }

  const page = useMemo(() => {
    switch (activeRoute) {
      case "projects":
        return <ProjectDetail projectPath={currentProjectPath} />;
      case "agents":
        return <AgentMonitor />;
      case "settings":
        return <Settings />;
      case "dashboard":
      default:
        return <Dashboard onRouteChange={handleRouteChange} />;
    }
  }, [activeRoute, currentProjectPath]);

  return (
    <Layout activeRoute={activeRoute} onRouteChange={handleRouteChange}>
      {page}
    </Layout>
  );
}

export default App;
