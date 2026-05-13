# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

AgentScope is a cross-platform Tauri v2 desktop app that monitors AI Agent sessions and project status. It provides a dashboard for tracking project stages, Git status, agent activity, memory files, and session transcripts across multiple projects. Previously known as PTV (Project Template Visualizer).

**Tech stack**: Tauri v2 (Rust backend + WebView frontend), React 19 + TypeScript, Tailwind CSS v4 + shadcn/ui (base-nova style), Recharts, Playwright E2E tests.

## Common Commands

```bash
# Frontend development (Vite only, no Rust backend)
npm run dev

# Full Tauri development mode (desktop window + hot reload)
npm run tauri dev

# Build frontend for production
npm run build

# Build desktop app (AppImage / deb / NSIS)
npm run tauri build

# Rust checks (run from src-tauri/)
cd src-tauri && cargo check

# Rust unit tests
cd src-tauri && cargo test

# Run a specific Rust test
cd src-tauri && cargo test test_name

# E2E tests (Playwright, uses Vite dev server — not Tauri)
npm test

# Run a specific E2E test file
npx playwright test dashboard.spec.ts

# E2E tests with UI mode
npm run test:ui

# View E2E test report
npm run test:report
```

## Architecture

### Frontend (`src/`)

- **Routing**: `App.tsx` manages route state (`dashboard` | `agents` | `settings`). Project detail is rendered within the dashboard route when `currentProjectPath` is set.
- **Tauri bridge**: `src/hooks/useTauri.ts` wraps `@tauri-apps/api` invoke/listen calls. Frontend calls Rust commands via `invoke('command_name', args)`.
- **State management**: No global state library — React `useState` + props drilling. Theme and font size are persisted via `localStorage`.
- **Components**: shadcn/ui components live in `src/components/ui/`. Custom components follow kebab-case filenames.

### Backend (`src-tauri/src/`)

- **`lib.rs`**: Tauri app builder. Sets up plugins, loads `ProjectRegistry`, starts `AgentCollector` background thread, registers all commands.
- **`commands.rs`**: All Tauri commands exposed to frontend (`add_project`, `get_project_data`, `start_watching`, etc.). Defines `AppState` struct holding registry, watchers, agent collector, and template fingerprint cache.
- **`registry.rs`**: `ProjectRegistry` — manages registered projects, persists to `{data_local_dir}/agent-scope/projects.json`. Uses path canonicalization for deduplication. Automatically strips Windows `\\?\` prefix on load.
- **`collectors/template/`**: Data collection for template projects:
  - `stage.rs` — reads `.current_stage` file
  - `config.rs` — parses `config/parameters.py` via python3 subprocess. **Windows-specific**: uses `python_command()` wrapper (`cmd /c chcp 65001`) to set UTF-8 console encoding; passes `PYTHONIOENCODING=utf-8` and `PYTHONUTF8=1` env vars to prevent `UnicodeEncodeError` on Chinese Windows.
  - `git.rs` — git status via `git status` subprocess
  - `project_files.rs` — scans whitelisted directories (CLAUDE.md, AGENTS.md, .claude/rules, .sisyphus/notepads, docs/design, etc.)
  - `session_transcript.rs` — parses session JSON files from `.sisyphus/sessions/`
  - `template_fingerprint.rs` — snapshot of template directory file paths for origin detection
- **`collectors/agent/mod.rs`**: `AgentCollector` — wraps `abtop-collector` crate, polls every 2 seconds for active agent sessions. Maps sessions to registered projects by cwd prefix match. Emits `agent-update` Tauri events.
- **`watcher.rs`**: `FileWatcher` — polling-based file watcher (500ms interval) used by `WatchedCollector` for live project updates. 5-second debounce.

### Key Data Flows

1. **Project data (dashboard/detail)**: Frontend calls `get_project_data` → `TemplateDataCollector` gathers stage + config + git + files in one shot. For live updates, `start_watching` creates a `WatchedCollector` that emits `template-update` events.
2. **Agent monitoring**: `AgentCollector` runs a background thread, polls `MultiCollector::collect()` every 2s, groups by project via cwd prefix match, emits `agent-update` events. Frontend listens via `useTauri` hook.
3. **Memory marking**: Frontend calls `save_candidate_memory` → appends markdown entry to `.sisyphus/notepads/project-memory/decisions.md`.

### abtop-collector Dependency

The `abtop-collector/` directory is a local Rust crate (not a git submodule). It provides `MultiCollector` for agent session discovery and system process monitoring. If you modify its API, you must also update the call sites in `src-tauri/src/collectors/agent/mod.rs`.

## Testing

- **E2E**: Playwright tests in `e2e/` run against the Vite dev server (not Tauri). Tauri invoke/listen APIs are not available in the browser test environment — tests cover UI rendering, error states, and empty states only.
- **Rust unit tests**: Located inline in `#[cfg(test)]` modules. `registry.rs` and `collectors/template/mod.rs` have the most comprehensive test coverage.

## Testing Environments

| OS | Address | User/Pass | SSH | Code Path |
|:---|:---|:---|:---|:---|
| Windows 10+ | `192.168.3.10` | `yufei` / `yufei` | `sshpass` configured | `C:\Repositories\ai_project_template_visualization` |
| Ubuntu 24.04 | `100.85.255.89` | `yufei` / `yufei` | `sshpass` configured | `/home/yufei/Repo/ai_project_template_visualization` |

## Development Notes

- **Vite port**: Fixed at 1420. The dev server configuration in `vite.config.ts` ignores `src-tauri/**` from file watching.
- **TypeScript**: Strict mode enabled (`noUnusedLocals`, `noUnusedParameters`).
- **Path alias**: `@/` maps to `src/` in both Vite and TypeScript configs.
- **Windows paths**: Registry loading automatically strips `\\?\` prefix. External command execution on Windows hides the command window (`CREATE_NO_WINDOW` flag in `silent_command`).
- **Windows Python encoding**: `config.rs` uses `python_command()` which wraps the Python invocation in `cmd /c chcp 65001` to set UTF-8 console codepage, plus `PYTHONUTF8=1` env var to prevent `UnicodeEncodeError` when `parameters.py` prints non-ASCII characters.
- **Language**: All comments and commit messages are in Chinese. Commit format: `[模块] 动作: 描述`.
