# AgentScope 信息架构命名 + 品牌壳层 + 视觉基线设计方案

> 版本：v0.1（设计阶段，待审核）
> 日期：2026-05-22
> 范围：App Shell（顶栏/侧栏/品牌区）+ 视觉基线，不涉及功能页面重设计

---

## 1. 当前问题判断

### 1.1 命名维度不一致

当前顶栏：

```
项目监控    Claude Code    设置
```

- "项目监控"是**功能动作**命名（监控什么）
- "Claude Code"是**产品/品牌**命名（用什么工具）
- "设置"是**功能类型**命名（做什么操作）

三个域在命名维度上跳跃，用户无法建立稳定的认知模型。"项目监控"暗示这是关于"监控"的动作集合，但 Dashboard 实际上是项目工作台（概览 + 详情 + 项目级记忆），不仅仅是"监控"。

### 1.2 品牌区过弱

当前左上角：

```
<span class="text-sm font-semibold tracking-tight">AgentScope</span>
```

- 字号 `text-sm`（14px），与导航 Tab 同级
- 无品牌标记、无色彩区分、无留白隔离
- 在视觉层级上与"项目监控"等 Tab 平级，导致：
  - 新用户可能以为 AgentScope 是可点击的菜单项
  - 品牌存在感不足，像开发占位符

### 1.3 界面过素，缺少产品气质

当前视觉特征：

- **优势**：干净、信息密度适中、Linear 风格色彩体系已有基础
- **劣势**：
  - 顶栏 (`h-12`) 与侧栏 (`h-10` 域标签) 之间缺乏层级节奏
  - 主内容区背景与卡片背景对比度不足（light 模式 `--background: oklch(0.99)` vs `--card: oklch(1.0)`，几乎无差异）
  - 卡片样式统一使用 shadcn 默认 `rounded-lg border border-border bg-card`，无产品特有层次
  - 缺少"实时观测台"应有的信息层级：什么在动、什么静止、什么是可交互的
  - Dashboard 的 stage 彩色条是唯一视觉亮点，但 SummaryTile/StatusTile/ProjectCard 之间层次模糊

---

## 2. 信息架构与命名方案

### 2.1 核心矛盾

当前两个主要工作区：

| 工作区 | 内容 | 本质 |
|--------|------|------|
| 项目域 | Dashboard（项目卡片、Stage、Git、Agent 数）+ 项目详情 + 项目记忆 | **模板项目的工作台** |
| Claude Code 域 | Agent 监控、会话管理、记忆资产、加载链模拟器 | **Claude Code 的观测与配置中心** |

关键洞察：**这两个域不是"监控"vs"监控"的关系，而是"项目工作台"vs"Claude Code 工具台"的关系。**

### 2.2 候选方案比较

#### 方案 A：模板项目 / Claude Code / 设置（推荐）

```
模板项目    Claude Code    设置
```

- **命名维度**：按**核心对象**命名（模板项目 = ai_project_template 创建的 FPGA 项目；Claude Code = Claude Code 工具生态）
- **一致性**：两个主工作区都是"对象"命名，设置是配置入口
- **区分度**：清楚区分"我在管理模板项目"和"我在看 Claude Code 相关的东西"
- **扩展性**：
  - 若后续增加非模板项目的监控，"模板项目"名称仍然准确
  - 若 Claude Code 域增加健康检测、编辑等功能，"Claude Code"名称仍然准确
  - 若后续增加其他 AI 工具（如 Copilot、Cursor），可在设置中配置，或在 Claude Code 域内增加子页
- **劣势**："模板项目"略长，但与"Claude Code"长度接近，视觉上平衡

#### 方案 B：模板项目监控 / Claude Code / 设置

- **问题**：保留了"监控"动作语义，但 Dashboard 不只是监控（还有详情、记忆），且与 Claude Code 域的 Agent 监控产生命名重叠
- **不推荐**：维度仍不一致（动作 vs 对象）

#### 方案 C：项目工作台 / Claude Code / 设置

- **优势**：更通用，"工作台"比"模板项目"更亲切
- **劣势**：
  - "工作台"过于宽泛，失去了与 ai_project_template 的关联暗示
  - 若后续 AgentScope 扩展为通用 AI Agent 工作台，"项目工作台"会与新功能冲突
  - 当前用户已习惯"项目"概念，改为"工作台"增加认知成本

#### 方案 D：FPGA 项目 / Claude Code / 设置

- **劣势**：过于具体，若后续支持非 FPGA 模板则名称失效

### 2.3 推荐方案

**采用方案 A：模板项目 / Claude Code / 设置**

```
┌─────────────────────────────────────────────────────────────┐
│ AgentScope  [模板项目]  [Claude Code]  [设置]                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

- 侧边栏域标签同步更新："项目监控"→"模板项目"
- AppDomain 内部 ID 保持 `projects` / `monitoring` / `settings` 不变（仅改显示文本）
- 所有文档口径同步更新

---

## 3. 品牌区设计

### 3.1 设计原则

- **不新增复杂 Logo**：AgentScope 是桌面工作台工具，不是消费级 App，品牌应克制、工程化
- **拉开与导航 Tab 的层次**：品牌区必须明显区别于可交互的导航项
- **给品牌区固定宽度**：避免 Tab 位置随品牌文字长度跳动

### 3.2 具体方案

#### 位置与尺寸

```
┌─────────────────────────────────────────────────────────────┐
│ ■  AgentScope    │  [模板项目]  [Claude Code]  [设置]        │
│     品牌区 180px   │  导航起始位置固定                         │
│                    │                                        │
└─────────────────────────────────────────────────────────────┘
```

- 品牌区在第一批中采用静态布局，与 Sidebar 结构对齐以避免错位，侧重增强品牌层级并与导航拉开，不实现复杂的 180px / 48px 宽度切换逻辑。
- 品牌区与导航 Tab 之间使用 1px 竖线和留白拉开层级。
- 顶栏高度维持 `h-12`（48px），但内部结构重构。

#### 品牌 Mark

使用 **极简几何标记** 替代纯文字：

```
◐   ← 一个被垂直线分割的圆，左半实色、右半描边
```

含义：
- 左半 = 观测（被监控的项目/Agent）
- 右半 = 分析（数据呈现、洞察）
- 整体 = Scope（视野、范围）

实现方式：
- 纯 CSS / SVG，无需图片资源
- 尺寸 `20px × 20px`
- Light 模式：左 `oklch(0.3 0 0)`，右 `oklch(0.3 0 0 / 40%)` + `stroke`
- Dark 模式：左 `oklch(0.9 0 0)`，右 `oklch(0.9 0 0 / 40%)` + `stroke`

#### 品牌文字

```
AgentScope
```

- 字号：`text-base`（15px 或 16px），比导航 Tab 的 `text-sm`（14px）大一级
- 字重：`font-bold`（700），导航 Tab 为 `font-medium`（500）
- 颜色：与前景色一致（`text-foreground`），但 Mark 提供额外识别
- 字间距：`tracking-tight` 保持

#### 品牌区背景（关键层次）

品牌区增加微妙的背景区分：

```css
/* Light 模式 */
background: linear-gradient(90deg, var(--sidebar) 0%, transparent 100%);
/* 或更克制的方案 */
border-right: 1px solid var(--border);
```

推荐：**不加背景渐变，仅加右侧 `1px` 分隔线**。理由：
- AgentScope 整体风格应冷静、不装饰
- 分隔线足够建立品牌区与导航区的结构区分
- 保持 Linear 风格的 flat 美学

### 3.3 最终品牌区效果

```
┌─────────────────────────────────────────────────────────────┐
│ ◐ AgentScope  |  [模板项目]  [Claude Code]  [设置]          │
│   ↑ 20px mark  ↑ bold 15px    ↑ medium 14px                │
│   品牌区与导航区有 1px 竖线分隔                               │
└─────────────────────────────────────────────────────────────┘
```

---

## 4. 视觉风格方向：精密观测台

### 4.1 视觉关键词

| 关键词 | 解释 |
|--------|------|
| 冷静 | 低饱和色彩，无情绪化装饰 |
| 精密 | 信息层级分明，一眼找到关键指标 |
| 实时感 | 脉冲、微动效、状态色提示"正在发生" |
| 工程感 | 等宽字体用于数据、清晰的网格对齐 |
| 克制 | 无渐变背景、无装饰光斑、无插画 |

### 4.2 参考物

- **Linear**：色彩体系、surface 层级、卡片阴影克制
- **Datadog / Grafana（轻量版）**：仪表盘信息密度、状态色使用
- **Apple 活动监视器**：实时数据的冷静呈现

### 4.3 色彩层级建议

当前问题：Light 模式下 `--background` (oklch 0.99) 与 `--card` (oklch 1.0) 几乎无差异，导致所有卡片浮在同一平面。

#### 建议调整（仅 light 模式，dark 模式保持当前）

```css
/* 当前 → 建议 */
--background: oklch(0.97 0 0);    /* 从 0.99 降到 0.97，让主背景略深 */
--card: oklch(1.0 0 0);           /* 保持纯白，作为最高 surface */
--sidebar: oklch(0.985 0 0);      /* 保持当前，略深于 background */
```

效果：
- 主内容区背景：浅灰（`oklch 0.97`）
- 卡片：纯白（`oklch 1.0`），在浅灰背景上自然浮起
- 侧栏：`oklch 0.985`，与主内容区有微弱区分
- 顶栏：使用 `--sidebar` 色（`oklch 0.985`），与侧栏形成"L 形壳层"

#### 顶栏/侧栏壳层统一

```
背景色层级（由深到浅）：
  顶栏 + 侧栏  →  oklch(0.985)   [壳层，统一]
  主内容区     →  oklch(0.97)    [工作台面，略深]
  卡片         →  oklch(1.0)     [信息载体，纯白]
  卡片内部 tile →  oklch(0.98)   [嵌套信息，略灰]
```

### 4.4 Surface 层级规范

| 层级 | Token | 用途 | 当前值 | 建议值 |
|------|-------|------|--------|--------|
| Shell | `--sidebar` | 顶栏 + 侧栏背景 | oklch(0.985) | 保持 |
| Workspace | `--background` | 主内容区背景 | oklch(0.99) | **oklch(0.97)** |
| Card | `--card` | 主要信息卡片 | oklch(1.0) | 保持 |
| Tile | 新增 `--tile` | 卡片内嵌小 tile | 无（用 muted） | **oklch(0.98)** |
| Popover | `--popover` | 浮层 | oklch(1.0) | 保持 |

### 4.5 卡片层级规范

当前 Dashboard 有三种卡片，视觉上无区分：

1. **SummaryTile**（汇总统计卡片：项目总数、活跃 Agent、未提交变更、项目状态）
2. **ProjectCard**（主实体卡片：项目信息、Stage、Git、Agent）
3. **StatusTile**（卡片内部小 tile：Git 分支、活跃 Agent、最近活动）

#### 建议层级

```
SummaryTile（汇总卡）
├── 样式：当前 `Card` 默认样式
├── 特征：无边框强化，纯白 surface，作为"仪表盘刻度"
└── 变化：无

ProjectCard（主实体卡）
├── 样式：在 `Card` 基础上增加微妙阴影
├── 特征：是用户主要操作对象（可点击进详情）
└── 变化：增加 `shadow-sm`（light 模式）/ `shadow-none`（dark 模式）
       hover 时 `shadow-md` + `border-primary/30`

StatusTile（内部 tile）
├── 样式：不使用 `Card`，改用独立 style
├── 特征：嵌套在 ProjectCard 内部，承载细分指标
└── 变化：
       background: var(--tile)  /* oklch(0.98) */
       border: 1px solid var(--border)  /* 保持 */
       border-radius: calc(var(--radius) * 0.6)  /* 比 Card 略小 */
```

### 4.6 标题与信息密度

#### 标题层级

当前 Dashboard 标题：

```
Dashboard              ← 小号标签，muted-foreground
项目仪表盘            ← text-3xl font-semibold
汇总已注册...          ← text-sm text-muted-foreground
```

问题："Dashboard"英文标签与中文界面不协调。

建议：

```
模板项目              ← 小号标签，使用域名称（中文）
项目仪表盘            ← text-3xl font-semibold（保持）
汇总已注册...          ← text-sm text-muted-foreground（保持）
```

或更进一步，将小号标签改为面包屑形式：

```
模板项目 / 概览
```

#### 信息密度调整

当前主内容区 padding：`p-4 sm:p-6 lg:p-8`

建议：在 1280px 以上屏幕增加 padding 到 `p-8 lg:p-10`，让大屏幕有呼吸感。

### 4.7 状态色与徽章规则

当前状态色体系已有基础（`--status-success` 等），但使用方式不一致：

- Dashboard：使用 `text-stage-l1` / `text-stage-l3` / `text-destructive` 等 stage 色
- AgentMonitor：使用 `border-stage-l1/40 bg-stage-l1/15 text-stage-l1`
- 问题：stage 色被用于状态语义，但 stage 色本质是"阶段"色，不应与"状态"色混用

#### 建议：状态色与 stage 色分离

| 用途 | 当前 | 建议 |
|------|------|------|
| Stage 进度 | `text-stage-l1` ~ `l5` | 保持，仅用于 stage badge |
| 成功/干净 | `text-stage-l5` | 改用 `text-status-success` |
| 警告 | `text-stage-l3` | 改用 `text-status-warning` |
| 错误 | `text-destructive` | 改用 `text-status-error`，与 destructive 区分 |
| 信息 | `text-primary` | 保持或改用 `text-status-info` |

**第一批暂不全面替换**，仅在 SummaryTile 和 AgentMonitor 的 status badge 中试点使用 `--status-*` 系列。

### 4.8 实时监控/观测气质体现

以下位置应体现"实时观测"气质：

1. **Agent Monitor 汇总卡片**：
   - "会话总数"数字旁增加一个微小的 **呼吸点**（pulsing dot，2px，绿色 `#22c55e`）
   - 仅在 Agent 事件活跃时显示
   - 不增加额外 DOM，使用 `::after` 伪元素

2. **Dashboard 活跃 Agent 汇总卡**：
   - 当 `totalAgents > 0` 时，数字使用 `font-mono` + 略大字号
   - 背景使用极微弱的动态色（`bg-status-success/5`）

3. **顶栏品牌 Mark**：
   - 当任意 Agent 活跃时，Mark 的右半部分可变为微弱脉冲色
   - 过于花哨，**暂不实施**，留作后续评估

4. **刷新/更新时间戳**：
   - 当前格式：`05/22 13:47`
   - 建议格式：`13:47:29 · 每 2 秒更新`
   - 使用等宽字体，`text-xs text-muted-foreground`

---

## 5. 组件与页面层级基线

### 5.1 App Shell 结构调整

```tsx
// Layout.tsx 结构
<div className="flex h-screen flex-col overflow-hidden">
  {/* TopNav: h-12, bg-sidebar, border-b */}
  <TopNav />

  <div className="flex min-h-0 flex-1">
    {/* Sidebar: 左侧壳层，bg-sidebar */}
    <Sidebar />

    {/* Main: 工作台面，bg-background（新值 oklch 0.97）*/}
    <main className="flex min-w-0 flex-1 flex-col bg-background">
      <ScrollArea className="h-full">
        <div className="min-h-full p-4 sm:p-6 lg:p-8 xl:p-10">
          {children}
        </div>
      </ScrollArea>
    </main>
  </div>
</div>
```

### 5.2 TopNav 结构

```tsx
<header className="flex h-12 shrink-0 items-center border-b border-border bg-sidebar px-0">
  {/* 品牌区：固定宽 180px，右侧 1px 分隔线 */}
  <div className="flex h-full w-[180px] items-center gap-2.5 border-r border-border px-4">
    {/* Mark：20px SVG */}
    <svg width="20" height="20" viewBox="0 0 20 20">...</svg>
    {/* 品牌文字：text-base font-bold */}
    <span className="text-base font-bold tracking-tight">AgentScope</span>
  </div>

  {/* 导航区 */}
  <nav className="ml-1 flex items-center gap-0.5 px-2" aria-label="大域导航">
    {/* 导航按钮：text-sm font-medium（保持）*/}
  </nav>
</header>
```

### 5.3 Sidebar 结构调整

当前 Sidebar 顶部有 `h-10` 的域标签区域，显示"项目监控"/"Claude Code"/"设置"。

问题：这个域标签与 TopNav 的激活 Tab 重复信息，且占用垂直空间。

**建议：移除 Sidebar 顶部的域标签区域，将空间还给子导航内容。**

理由：
- TopNav 已经明确显示当前域
- Sidebar 的域标签是重复信息
- 移除后可增加 1-2 个项目列表项的可见高度

```tsx
{/* 移除这一段 */}
{/* <div className="flex h-10 items-center gap-2 border-b border-sidebar-border px-3">
     {isExpanded && <p className="truncate text-xs ...">...</p>}
   </div> */}
```

### 5.4 卡片基线

```tsx
// SummaryTile（汇总卡）
<Card className="...">
  {/* 保持当前样式 */}
</Card>

// ProjectCard（主实体卡）
<Card className="group cursor-pointer overflow-hidden transition-all
  hover:border-primary/40 hover:shadow-md
  focus-visible:border-primary/60 focus-visible:ring-1 focus-visible:ring-primary/20
  shadow-sm">
  {/* 增加 shadow-sm，hover 时 shadow-md */}
</Card>

// StatusTile（内部 tile）
<div className="rounded-md border border-border bg-tile p-3">
  {/* 不使用 Card，改用自定义样式 */}
</div>
```

---

## 6. 第一批实施范围

### 6.1 明确包含（P0）

| # | 修改项 | 文件 | 说明 |
|---|--------|------|------|
| 1 | 顶栏命名：项目监控 → 模板项目 | `src/components/TopNav.tsx` | 仅改 `label`，不改 `id` |
| 2 | 侧边栏域标签：同步更新 | `src/components/Sidebar.tsx` | 域标签文字 + 移除顶部域标签区域 |
| 3 | 品牌区重构 | `src/components/TopNav.tsx` | Mark SVG + 文字样式 + 右侧分隔线 |
| 4 | 主背景色调整 | `src/index.css` | `--background: oklch(0.97)` |
| 5 | 新增 `--tile` token | `src/index.css` | `--tile: oklch(0.98)` |
| 6 | 卡片阴影层级 | `src/features/dashboard/index.tsx` | ProjectCard 增加 `shadow-sm`，StatusTile 改用 `bg-tile` |
| 7 | Dashboard 标题标签 | `src/features/dashboard/index.tsx` | "Dashboard" → "模板项目" |
| 8 | 页面 padding 微调 | `src/components/Layout.tsx` | 增加 `xl:p-10` |
| 9 | 文档口径同步 | `docs/*.md` | 所有提到"项目监控"的活跃文档 |

### 6.2 明确不包含（延后）

| # | 项目 | 原因 |
|---|------|------|
| 1 | Agent Monitor 状态色全面替换 | 范围大，且当前 stage 色使用已稳定，第二批评估 |
| 2 | 品牌 Mark 脉冲动效 | 过于花哨，第一批保持静态 |
| 3 | Sidebar 项目列表样式重设计 | 当前功能完整，第二批与项目详情页统一优化 |
| 4 | 加载链模拟器页面精修 | 当前为 P1 新功能，视觉基线到位后再精修 |
| 5 | 设置页面精修 | 低频页面，第二批统一处理 |
| 6 | 记忆资产页面精修 | 当前功能完整，第二批评估 |

### 6.3 验证页面

第一批验证时应重点截图/检查：

1. **Dashboard（项目仪表盘）**：品牌区 + 新背景色 + 卡片阴影 + StatusTile 新样式
2. **Agent Monitor（Agent 监控）**：品牌区 + 新背景色 + 侧边栏无域标签
3. **Claude Memory（记忆资产）**：品牌区 + 侧边栏结构
4. **Settings（设置）**：品牌区 + 侧边栏结构

---

## 7. 风险与约束

### 7.1 风险

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| `--background` 从 oklch(0.99) 降到 oklch(0.97) 后，某些页面元素对比度不足 | 中 | 中 | 实施后全量截图对比，必要时调回 0.98 |
| "模板项目"名称用户不接受 | 低 | 高 | 保留"项目监控"作为备选，通过配置切换 |
| 品牌 Mark SVG 在不同平台渲染差异 | 低 | 低 | 使用基础 SVG 路径，避免复杂滤镜 |
| Sidebar 移除域标签后，折叠状态下用户迷失 | 中 | 低 | 折叠时品牌区+TopNav 已足够指示当前域 |

### 7.2 约束

- 不改任何功能逻辑
- 不改 Claude 资产读写
- 不改后端 API
- 保持 E2E 测试通过（仅涉及文本的测试需同步更新）
- 保持 Tailwind CSS v4 + shadcn/ui 兼容性
- Dark 模式保持当前色彩体系（仅 light 模式调整背景色）

---

## 8. 审核与最终决议 (已确认)

本设计方案已通过第一批评审，最终决议及收紧设计要求如下：

1. **顶栏命名**：确定采用 `模板项目 / Claude Code / 设置`。
2. **品牌 Mark**：采用静态、可替换的极简几何半圆分割方案（`◐` SVG），不将其定为最终 logo，第一批保持静态，无任何脉冲动效。
3. **品牌区布局**：第一批侧重增强品牌层级并与导航拉开，不实现复杂的 `180px / 48px` 宽度切换逻辑，以避免与可折叠 Sidebar 产生错位。
4. **背景与表面层次**：Light 模式主背景调整为 `oklch(0.97 0 0)`，卡片为更高 Surface `oklch(1.0)`，嵌套 Tile 使用新 `--tile: oklch(0.98)` 层次。若主背景过灰或 muted 文本可读性下降，则回调至 `oklch(0.98 0 0)`。
5. **明确不包含（延后到第二批评估）**：
   - Agent Monitor 状态色体系重构与全面替换
   - 呼吸点 / pulsing dot 动效
   - 品牌 Mark 动效
   - 刷新时间格式系统性改造
   - 加载链模拟器及设置页面精修

---

## 9. 第一批预计修改文件清单

```
src/components/TopNav.tsx              ← 品牌区 + 命名 + 结构
src/components/Sidebar.tsx             ← 移除域标签区 + 命名同步
src/components/Layout.tsx              ← padding 微调
src/index.css                          ← --background + --tile + 状态色
src/features/dashboard/index.tsx       ← 卡片阴影 + StatusTile + 标题标签
src/features/agent-monitor/index.tsx   ← 标题标签（如"Agents" → "Claude Code"）

docs/design-claude-memory-management-v0.2.md    ← "项目监控"口径更新
docs/requirements-claude-memory-management.md   ← "项目监控"口径更新
```

---

## 10. 附录：当前 vs 建议对比

### 10.1 顶栏

| 维度 | 当前 | 建议 |
|------|------|------|
| 品牌 | 纯文字 `text-sm font-semibold` | Mark + 文字 `text-base font-bold`，有分隔线 |
| 命名 | 项目监控 / Claude Code / 设置 | 模板项目 / Claude Code / 设置 |
| 高度 | h-12 (48px) | 保持 h-12 |
| 背景 | bg-background | bg-sidebar（统一壳层色）|

### 10.2 侧边栏

| 维度 | 当前 | 建议 |
|------|------|------|
| 顶部 | h-10 域标签区 | 移除，子导航直接顶格 |
| 分组标签 | "监控"/"记忆"（小字） | 保持 |
| 背景 | bg-sidebar (oklch 0.985) | 保持 |
| 宽度 | 13rem / 4rem | 保持 |

### 10.3 主内容区

| 维度 | 当前 | 建议 |
|------|------|------|
| 背景 | oklch(0.99)（近白） | oklch(0.97)（浅灰） |
| Padding | p-4 sm:p-6 lg:p-8 | p-4 sm:p-6 lg:p-8 xl:p-10 |
| 卡片背景 | oklch(1.0)（纯白） | 保持 |
| 卡片阴影 | 无 | ProjectCard 增加 `shadow-sm` |

### 10.4 卡片内部

| 维度 | 当前 | 建议 |
|------|------|------|
| StatusTile | `bg-muted/30` + `border-border/60` | `bg-tile` + `border-border` |
| SummaryTile | `Card` 默认 | 保持默认 |
| ProjectCard | `Card` 默认 + hover border | 增加 `shadow-sm`，hover `shadow-md` |

---

*文档结束。待用户审核后进入实施阶段。*
