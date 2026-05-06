# F4 范围忠实度检查 — 发现与结论

## 检查时间
2026-05-06

## 范围概述
验证 ptv v0.1 所有 15 个任务（0.1-3.3）的实现是否 1:1 符合规范，无缺失、无蔓延。

---

## Task 合规性详细审查

### Wave 0（基础建设）

| 任务 | 预期文件 | 实际文件 | 状态 | 备注 |
|------|----------|----------|------|------|
| 0.1 abtop-collector | abtop-collector/src/... | 10 个 .rs 文件 + Cargo.toml | ✅ 完整 | 无 ratatui/crossterm，可独立编译 |
| 0.2 Tauri 骨架 | src-tauri/, src/, package.json | 同上 | ✅ 完整 | 含 greet IPC、Tailwind、shadcn/ui、Recharts |
| 0.3 Shell-out config | src-tauri/src/collectors/config.rs | collectors/template/config.rs | ✅ 完整 | 路径在 template/ 子目录下，功能等价 |
| 0.4 文件监听 | src-tauri/src/watcher.rs | 同上 | ✅ 完整 | 含 18 个单元测试，覆盖所有边界 |

### Wave 1（Rust 数据层）

| 任务 | 预期文件 | 实际文件 | 状态 | 备注 |
|------|----------|----------|------|------|
| 1.1 项目注册表 | src-tauri/src/registry.rs | 同上 | ✅ 完整 | 含 15 个单元测试，持久化、去重、错误处理 |
| 1.2 模板采集器 | src-tauri/src/collectors/template/ | 4 个采集器 + mod.rs | ✅ 完整 | Stage/Config/Memory/Git + WatchedCollector |
| 1.3 Agent 采集器 | src-tauri/src/collectors/agent/ | collectors/agent/mod.rs | ✅ 完整 | 2 秒轮询 + agent-update 事件 + 测试 |

### Wave 2（集成层 + 前端基础）

| 任务 | 预期文件 | 实际文件 | 状态 | 备注 |
|------|----------|----------|------|------|
| 1.4 Tauri commands | src-tauri/src/commands.rs | 同上 | ✅ 完整 | 6 个 command + AppState + 序列化层 |
| 2.0 前端脚手架 | src/hooks/useTauri.ts, src/components/ | useTauri.ts + 8 个 UI 组件 | ✅ 完整 | invoke/listen 封装 + shadcn 组件 |

### Wave 3（UI 面板）

| 任务 | 预期文件 | 实际文件 | 状态 | 备注 |
|------|----------|----------|------|------|
| 2.1 仪表盘 | panels/Dashboard.tsx | pages/Dashboard.tsx | ✅ 完整 | 路径差异（pages/ vs panels/），功能完整 |
| 2.2 项目详情 | panels/ProjectDetail.tsx | pages/ProjectDetail.tsx | ✅ 完整 | 4 个面板全部实现，含空/错状态 |
| 2.3 Agent 监控 | panels/AgentMonitor.tsx | pages/AgentMonitor.tsx | ⚠️ 部分 | **缺失：Recharts 面积图**（计划要求"近 2 分钟趋势"面积图，实际用 CSS 进度条代替） |
| 2.4 设置面板 | panels/Settings.tsx | pages/Settings.tsx | ⚠️ 部分 | **缺失：Tauri dialog 浏览按钮**（计划要求文件浏览对话框，实际只有文本输入框） |

### Wave 4（集成验证）

| 任务 | 预期文件 | 实际文件 | 状态 | 备注 |
|------|----------|----------|------|------|
| 3.1 E2E 数据流验证 | e2e/ 测试 + 验证 | 5 个 spec 文件 | ✅ 基本 | 测试在浏览器环境运行（Tauri API 不可用），主要验证空/错状态 |
| 3.2 Playwright E2E | playwright.config.ts + e2e/ | 同上 | ✅ 完整 | 25+ test cases，覆盖 4 个面板 + 导航 |
| 3.3 打包构建 | .dmg + AppImage | tauri.conf.json 已配置 | ✅ 配置完整 | 未在 Linux 测试机实际执行构建验证 |

---

## Must NOT Have 合规检查

| 禁止项 | 状态 | 证据 |
|--------|------|------|
| ❌ 自动扫描文件系统 | ✅ 无 | 仅 GUI 手动添加 |
| ❌ 写入被监控项目 | ✅ 无 | 所有采集器只读 |
| ❌ 启动/停止 Claude 进程 | ✅ 无 | AgentCollector 只读 |
| ❌ ratatui/crossterm | ✅ 无 | cargo tree 验证 |
| ❌ 告警/通知/声音 | ✅ 无 | 未引入 |
| ❌ 历史趋势图/数据分析 | ✅ 无 | 未引入（v0.2 功能） |
| ❌ 跨项目参数对比 | ✅ 无 | 未引入（v0.2 功能） |
| ❌ 项目创建/克隆 | ✅ 无 | 未引入 |
| ❌ 远程监控或 Web 服务 | ✅ 无 | 未引入 |
| ❌ 测试报告聚合 | ✅ 无 | 未引入（v0.2 功能） |

---

## 跨任务污染（Contamination）

**状态：CLEAN**

- 各任务文件按模块隔离，无 Task N 修改 Task M 文件的情况
- lib.rs 作为集成入口引用所有模块，属于正常架构设计
- commands.rs 聚合各采集器数据，属于 Task 1.4 的职责范围

---

## 未计入变更（Unaccounted）

**状态：2 项轻微**

1. `.vscode/extensions.json` — 开发环境配置，属于脚手架附带
2. `public/tauri.svg`, `public/vite.svg` — 脚手架默认图标

以上属于 create-tauri-app 模板自带，不影响范围。

---

## VERDICT

### 总体评分

```
Tasks:      [13/15 完全合规 | 2/15 部分合规]
Contamination:  [CLEAN]
Unaccounted:    [2 轻微项]
Must NOT Have:  [10/10 合规]
```

### 结论：CONDITIONAL APPROVE（条件通过）

**理由：**
1. 核心功能（项目注册、数据采集、Agent 监控、前端面板）全部实现
2. 15 个任务均有对应代码文件，无任务完全缺失
3. 仅 2 个 Minor 功能缺失：
   - Task 2.3 的 Recharts 面积图（不影响 Agent 监控核心功能）
   - Task 2.4 的 Tauri dialog 浏览按钮（用户仍可手动输入路径）
4. 所有 Must NOT Have 护栏均未违反
5. 无跨任务污染

### 建议修复（如时间允许）
- Task 2.3：添加 Recharts 面积图展示 Token 速率历史趋势
- Task 2.4：集成 `@tauri-apps/plugin-dialog` 实现文件浏览按钮
