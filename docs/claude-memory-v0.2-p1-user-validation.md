# Claude 记忆 v0.2 P1 用户验证指引

> 目标：在真实 Claude Code 环境中对比 AgentScope 加载链模拟器与 `/memory` 命令输出，验证 P1 实现正确性。
> 验证方式：纯观察，不改动任何文件。
> 适用平台：Linux (AppImage) / macOS (开发模式)

---

## 1. 安全声明

**本次验证全程只观察，不修改任何 Claude 资产。**

- 不创建、修改、删除 `~/.claude/` 下任何文件
- 不创建、修改、删除任何项目的 `CLAUDE.md`、`.claude/CLAUDE.md`、`CLAUDE.local.md`
- 不创建、修改、删除任何项目的 `.claude/rules/` 或 `.claude/settings.json`
- 验证完成后不需要清理任何测试文件

**如果验证过程中意外触碰了真实文件**：
- 立即停止操作
- 保留被修改的文件状态
- 报告：哪个文件、什么操作、预期变化
- 等待指示，不自行恢复

---

## 2. 验证前准备

### 2.1 启动 AgentScope

**Linux**（AppImage）：
```bash
# 假设 AppImage 位于：
# /home/yufei/Repo/agent-scope/src-tauri/target/release/bundle/appimage/agent-scope_0.2.0_amd64.AppImage

chmod +x agent-scope_0.2.0_amd64.AppImage
./agent-scope_0.2.0_amd64.AppImage
```

**macOS**（开发模式）：
```bash
cd /path/to/agent-scope
npm run tauri dev
```

### 2.2 选择 2 个验证 cwd

**最低要求**：

| # | cwd 类型 | 示例 | 要求 |
|---|---------|------|------|
| 1 | 项目根目录 | `/home/yufei/Repo/agent-scope` | 已有 `CLAUDE.md` 且含 rules |
| 2 | 该项目深层子目录 | `/home/yufei/Repo/agent-scope/src-tauri/src/collectors/claude_memory` | 祖先目录有 `CLAUDE.md`，当前目录无 `CLAUDE.md` |

优先选用 Claude 资产较丰富（有 rules、有 auto memory）的项目。

---

## 3. Claude Code 侧记录（每个 cwd 做一次）

在终端中：

```bash
# 1. 进入目标目录
cd <目标 cwd>

# 2. 启动 Claude Code
claude

# 3. 在 Claude Code 会话中执行
/memory
```

**请记录/截图的 `/memory` 输出内容**：
1. 完整的加载顺序列表（从上到下）
2. 每个加载项的 scope 标注（user / project / local / auto）
3. path-scoped rules 的展示方式（是否单独列出？是否在启动链中？）
4. Auto Memory 的展示（如果有）：行数、大小、是否截断
5. 被排除的 assets（如果有）
6. 任何 warnings 或 errors

**不需要**为了验证而创建文件。只使用当前已有环境。

---

## 4. AgentScope 侧记录（每个 cwd 做一次）

1. 打开 AgentScope 桌面应用
2. 切换到 **Claude 记忆** 域（顶部导航）
3. 点击侧边栏 **加载链模拟器**
4. 在输入框中输入与 Claude Code 中相同的 cwd 路径
5. 点击 **模拟加载**
6. 记录以下内容：

**A 区域：启动链**（与 Claude `/memory` 的启动链对照）
- 文件列表和顺序
- 每个文件的 scope 和 asset_type

**B 区域：路径作用域规则**
- 规则名称和 paths 模式
- 是否与 Claude `/memory` 展示一致

**其他**：
- 被排除资产（若有）
- warnings（若有）
- Auto Memory 观察结果（若有）
- host_profile 信息

---

## 5. 对照结论填写模板

将每个 cwd 的对照结果填入以下格式：

```markdown
### 用例: /home/yufei/Repo/agent-scope（项目根目录）

#### 对比项

| 对比项 | 结果 | 说明 |
|--------|------|------|
| 启动链顺序 | 一致 / 差异 | 具体差异描述 |
| scope 标注 | 一致 / 差异 | |
| path-scoped rules | 一致 / 差异 | |
| Auto Memory | 一致 / 差异 / 本批未覆盖 | |
| excluded assets | 一致 / 差异 / 环境中不存在 | |
| warnings | 一致 / 差异 / 环境中不存在 | |

#### 差异说明（如有）

#### /memory 输出
```
{粘贴 /memory 完整输出或截图}
```

#### AgentScope 模拟输出
```
{粘贴模拟结果摘要或截图}
```
```

---

## 6. 本批明确不要求验证

以下项目**不在本轮验证范围**，无需尝试：

| 项 | 原因 |
|----|------|
| A9（祖先 `.claude/CLAUDE.md`） | 需在祖先目录创建 `.claude/CLAUDE.md`，当前实现不扫描 |
| A10（当前目录 `.claude/CLAUDE.md`） | 若项目天然存在该文件则可观察，否则不创建（单元测试已覆盖） |
| A11（祖先 `CLAUDE.local.md`） | 若祖先目录天然存在该文件则可观察，否则不创建（单元测试已覆盖） |
| E4（非 git 目录 Auto Memory） | 暂不验证，后续用 `/tmp` 隔离环境 |
| P2 `@import` 解析 | 尚未实现 |
| 编辑 / 删除 / 同步 | P3/P4 范围，尚未实现 |

---

## 7. 验证完成报告

验证完成后请提供：

1. 每个 cwd 的对照结论（上表格式）
2. `/memory` 输出截图或文本
3. AgentScope 模拟输出截图或文本
4. AgentScope 版本号（显示在加载链模拟器页面）
5. Claude Code 版本号（`claude --version`）
6. 操作系统
