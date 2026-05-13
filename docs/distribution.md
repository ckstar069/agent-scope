# AgentScope 软件分发指南

本文档说明 AgentScope 在 Linux、macOS、Windows 三个平台上的分发形式、使用方式及运行时文件位置。

---

## Linux

| 分发形式 | 文件示例 | 使用方式 |
|---------|---------|---------|
| **AppImage** | `agent-scope_0.2.0_amd64.AppImage` | `chmod +x` 后双击运行，或命令行 `./agent-scope_0.2.0_amd64.AppImage` |
| **deb** | `agent-scope_0.2.0_amd64.deb` | `sudo dpkg -i agent-scope_0.2.0_amd64.deb`，安装后从应用菜单启动 |
| **rpm** | `agent-scope-0.2.0-1.x86_64.rpm` | `sudo rpm -i agent-scope-0.2.0-1.x86_64.rpm`，安装后从应用菜单启动 |

### 运行时文件

| 文件 | 位置 | 说明 |
|-----|------|------|
| `projects.json` | `~/.local/share/agent-scope/projects.json` | 项目注册表（用户添加的项目列表） |

### 外部数据目录（只读）

| 目录 | 位置 | 说明 |
|-----|------|------|
| Claude 配置目录 | `~/.claude/` | Claude Code CLI 生成的会话历史、配置文件等，AgentScope 只读取不写入 |

---

## macOS

| 分发形式 | 文件示例 | 使用方式 |
|---------|---------|---------|
| **.app Bundle** | `AgentScope.app` | 双击运行，或从 Launchpad 启动 |
| **.dmg** | `agent-scope_0.2.0_x64.dmg` | 挂载后拖入 `Applications` 文件夹 |

### 运行时文件

| 文件 | 位置 | 说明 |
|-----|------|------|
| `projects.json` | `~/Library/Application Support/agent-scope/projects.json` | 项目注册表 |

### 外部数据目录（只读）

| 目录 | 位置 | 说明 |
|-----|------|------|
| Claude 配置目录 | `~/.claude/` | Claude Code CLI 生成的会话历史、配置文件等，AgentScope 只读取不写入 |

---

## Windows

| 分发形式 | 文件示例 | 使用方式 |
|---------|---------|---------|
| **可执行文件** | `agent-scope.exe` | 直接双击运行（依赖系统已安装 WebView2 运行时） |
| **NSIS 安装包** | `agent-scope_0.2.0_x64-setup.exe` | 双击安装到 `Program Files`，自动创建开始菜单和桌面快捷方式 |
| **MSI 安装包** | `agent-scope_0.2.0_x64_en-US.msi` | 双击安装或命令行静默安装，支持组策略部署 |

### 运行时文件

| 文件 | 位置 | 说明 |
|-----|------|------|
| `projects.json` | `%LOCALAPPDATA%\agent-scope\projects.json`（即 `C:\Users\<用户名>\AppData\Local\agent-scope\projects.json`） | 项目注册表 |

### 外部数据目录（只读）

| 目录 | 位置 | 说明 |
|-----|------|------|
| Claude 配置目录 | `%USERPROFILE%\.claude\`（即 `C:\Users\<用户名>\.claude\`） | Claude Code CLI 生成的会话历史、配置文件等，AgentScope 只读取不写入 |

---

## 各形式对比

| 特性 | AppImage / agent-scope.exe | deb / rpm | NSIS / MSI | .app / .dmg |
|-----|-------------------|-----------|------------|-------------|
| 是否需要安装 | 否 | 是 | 是 | 是（.app 可直接运行） |
| 是否需要管理员权限 | 否 | 是 | 是 | 否 |
| 是否创建卸载入口 | 否 | 是（系统包管理器） | 是（控制面板） | 是（拖入废纸篓） |
| 是否创建快捷方式 | 否 | 是 | 是 | 是（.dmg） |
| 适合场景 | 便携使用、快速体验 | Linux 系统标准安装 | Windows 用户标准安装 | macOS 用户标准安装 |
| 依赖要求 | WebView2（Windows） | 无 | 无 | 无 |
