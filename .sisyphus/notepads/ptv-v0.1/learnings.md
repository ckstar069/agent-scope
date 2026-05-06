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

## [2026-05-06] 被监控项目错误提示分类

- `ParameterError::Display` 直接作为前端 `config_error` 展示，应输出面向用户的中文说明，而不是内部路径或英文异常摘要。
- Python shell-out 失败可用 stderr 粗分类：`SyntaxError`/`invalid syntax` 为语法错误，`Traceback` 为运行时错误，当前双花括号模板问题会落入 TypeError 但用户语义上更接近语法/模板写错。
- Git collector 不应把非仓库和 Git 不可用吞成默认 clean 状态，否则前端无法区别“工作区干净”和“无法采集 Git”。
- `ProjectDetail` 面板内容区比副标题更适合展示分类错误，副标题保留稳定的数据源说明。
