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

## [2026-05-06] 参数解析兼容性

- `fpga_project_coarse_cfo` 的自定义 `parameters.py` 可能缺少模板默认字段，例如 `axi_lite_addr_width`。
- 采集层 `ProjectConfig` 已为非关键字段配置 `#[serde(default)]`，命令层 payload 类型也需要保持同样兼容性，避免中间 JSON/结构体反序列化链路重新变严格。
- 前端参数快照应把非关键配置字段声明为可选，并继续通过 `formatValue` 将 `undefined` 显示为 `--`。
