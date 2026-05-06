## [2026-05-06] Wave 3 Bug Fix Required

**Issue**: App.tsx has hardcoded `currentProjectPath = ""`, causing ProjectDetail to always show empty state.
**Fix needed**: Add selectedProjectPath state to App.tsx and pass it through Dashboard.
**Impact**: Without this fix, end-to-end testing will fail for ProjectDetail panel.

## [2026-05-06] Wave 4 Plan

- Task 3.0: Fix App.tsx project path state management
- Task 3.1+3.2: Install Playwright + write E2E tests + run validation
- Task 3.3: Tauri build for macOS .dmg and Linux AppImage

## Key Decisions

- Playwright will be used for E2E testing (as specified in plan)
- Tauri build targets: macOS .dmg + Linux AppImage
- Linux test machine: 100.85.255.89 (yufei/yufei)
- App.tsx routing uses simple state (not React Router)
