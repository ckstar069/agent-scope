import { useCallback, useEffect, useMemo, useState } from "react";

import { Layout } from "@/components/Layout";
import { AgentMonitor } from "@/features/agent-monitor";
import { ClaudeHistory } from "@/features/claude-history";
import { ClaudeMemory } from "@/features/claude-memory";
import { Dashboard } from "@/features/dashboard";
import { ProjectDetail } from "@/features/project-detail";
import { GeneralSettings, ProjectSettings } from "@/features/settings";

export type AppDomain = "projects" | "monitoring" | "claude-memory" | "settings";

export type ProjectPage = "overview" | "detail";
export type MonitoringPage = "agents" | "claude-history";
export type ClaudeMemoryPage = "assets" | "load-chain";
export type SettingsPage = "project" | "general";

const STORAGE_KEY_DOMAIN = "agent-scope:domain";
const STORAGE_KEY_PROJECT = "agent-scope:current-project";

function App() {
  const [activeDomain, setActiveDomain] = useState<AppDomain>(() => {
    const stored = localStorage.getItem(STORAGE_KEY_DOMAIN) as AppDomain | null;
    return stored && ["projects", "monitoring", "claude-memory", "settings"].includes(stored)
      ? stored
      : "projects";
  });

  const [projectPage, setProjectPage] = useState<ProjectPage>("overview");
  const [monitoringPage, setMonitoringPage] = useState<MonitoringPage>("agents");
  const [claudeMemoryPage, setClaudeMemoryPage] = useState<ClaudeMemoryPage>("assets");
  const [settingsPage, setSettingsPage] = useState<SettingsPage>("project");

  const [selectedProject, setSelectedProject] = useState<string>(() => {
    return localStorage.getItem(STORAGE_KEY_PROJECT) || "";
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY_DOMAIN, activeDomain);
  }, [activeDomain]);

  useEffect(() => {
    if (selectedProject) {
      localStorage.setItem(STORAGE_KEY_PROJECT, selectedProject);
    } else {
      localStorage.removeItem(STORAGE_KEY_PROJECT);
    }
  }, [selectedProject]);

  const handleDomainChange = useCallback((domain: AppDomain) => {
    setActiveDomain(domain);
    // 切换域时重置到该域的默认页面
    if (domain === "projects") setProjectPage("overview");
    if (domain === "monitoring") setMonitoringPage("agents");
    if (domain === "claude-memory") setClaudeMemoryPage("assets");
    if (domain === "settings") setSettingsPage("project");
  }, []);

  const handleSelectProject = useCallback((projectPath: string) => {
    setSelectedProject(projectPath);
    setProjectPage("detail");
    setActiveDomain("projects");
  }, []);

  const handleBackToOverview = useCallback(() => {
    setSelectedProject("");
    setProjectPage("overview");
  }, []);

  const activePage = useMemo(() => {
    switch (activeDomain) {
      case "projects":
        return projectPage;
      case "monitoring":
        return monitoringPage;
      case "claude-memory":
        return claudeMemoryPage;
      case "settings":
        return settingsPage;
    }
  }, [activeDomain, projectPage, monitoringPage, claudeMemoryPage, settingsPage]);

  const page = useMemo(() => {
    switch (activeDomain) {
      case "projects": {
        if (projectPage === "detail" && selectedProject) {
          return (
            <ProjectDetail
              projectPath={selectedProject}
              onSelectProject={handleSelectProject}
            />
          );
        }
        return (
          <Dashboard
            onNavigateSettings={() => {
              setActiveDomain("settings");
              setSettingsPage("project");
            }}
          />
        );
      }
      case "monitoring": {
        if (monitoringPage === "claude-history") {
          return <ClaudeHistory />;
        }
        return <AgentMonitor />;
      }
      case "claude-memory": {
        return <ClaudeMemory page={claudeMemoryPage} />;
      }
      case "settings": {
        if (settingsPage === "general") {
          return <GeneralSettings />;
        }
        return <ProjectSettings />;
      }
    }
  }, [activeDomain, projectPage, monitoringPage, claudeMemoryPage, settingsPage, selectedProject, handleSelectProject, handleBackToOverview]);

  return (
    <Layout
      activeDomain={activeDomain}
      activePage={activePage}
      selectedProject={selectedProject}
      onDomainChange={handleDomainChange}
      onProjectPageChange={useCallback((page: string) => setProjectPage(page as ProjectPage), [])}
      onMonitoringPageChange={useCallback((page: string) => setMonitoringPage(page as MonitoringPage), [])}
      onClaudeMemoryPageChange={useCallback((page: string) => setClaudeMemoryPage(page as ClaudeMemoryPage), [])}
      onSettingsPageChange={useCallback((page: string) => setSettingsPage(page as SettingsPage), [])}
      onSelectProject={handleSelectProject}
    >
      {page}
    </Layout>
  );
}

export default App;
