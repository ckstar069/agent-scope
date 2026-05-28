import { test, expect, type Page } from "@playwright/test";

/**
 * Claude Memory E2E 测试
 *
 * 测试 Claude 记忆管理页面的导航、渲染、错误状态和 mock 数据展示。
 * Playwright 浏览器环境中 Tauri invoke 不可用，因此：
 * - 基础测试验证 UI 结构和错误状态
 * - mock 测试通过注入 __TAURI_INTERNALS__ 模拟后端返回
 */

async function openClaudeMemory(page: Page) {
  await page.goto("/");
  await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
  await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "记忆资产" }).click();
}

const mockHealthReport = {
  overall_score: 85,
  freshness: { name: "freshness", score: 90, reason: "All assets are fresh", contributing_assets: [] },
  quality: { name: "quality", score: 80, reason: "Good quality", contributing_assets: [] },
  coverage: { name: "coverage", score: 85, reason: "Good coverage", contributing_assets: [] },
  cleanliness: { name: "cleanliness", score: 88, reason: "Clean", contributing_assets: [] },
  safety: { name: "safety", score: 82, reason: "Safe", contributing_assets: [] },
  top_issues: [],
  stale_assets: [],
  duplicate_groups: [],
};

const mockContextPressure = {
  total_assets: 3,
  existing_assets: 3,
  total_lines: 27,
  total_bytes: 624,
  estimated_tokens: 156,
  pressure_ratio: 0.00078,
  level: "normal",
  heavy_assets: [],
  alerts: [],
};

const mockReviewQueue = {
  items: [],
  pending_count: 0,
  reviewed_count: 0,
  ignored_count: 0,
  snoozed_count: 0,
  last_sync_at: null,
};

const mockOverview = {
  scanned_at_ms: Date.now(),
  host_profile: {
    host_id: "test-host-001",
    hostname: "test-host",
    os: "macos",
    home_dir: "/Users/test",
    claude_config_dir: "/Users/test/.claude",
    user_name: "test",
  },
  assets: [
    {
      id: "user-claude-md",
      scope: "user",
      asset_type: "user_claude_md",
      logical_path: "/Users/test/.claude/CLAUDE.md",
      native_path: "/Users/test/.claude/CLAUDE.md",
      content_hash: null,
      content_preview: "# User CLAUDE.md\n",
      content_truncated: false,
      line_count: 10,
      byte_size: 256,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "project-claude-md",
      scope: "project",
      asset_type: "project_claude_md",
      logical_path: "/Users/test/project/CLAUDE.md",
      native_path: "/Users/test/project/CLAUDE.md",
      content_hash: null,
      content_preview: "# Project CLAUDE.md\n",
      content_truncated: false,
      line_count: 5,
      byte_size: 128,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "project-rule",
      scope: "project",
      asset_type: "project_rule",
      logical_path: "/Users/test/project/.claude/rules/01-project-rule.md",
      native_path: "/Users/test/project/.claude/rules/01-project-rule.md",
      content_hash: null,
      content_preview: "# Project Rule\n",
      content_truncated: false,
      line_count: 12,
      byte_size: 240,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "global-rule",
      scope: "user",
      asset_type: "global_rule",
      logical_path: "/Users/test/.claude/rules/style.md",
      native_path: "/Users/test/.claude/rules/style.md",
      content_hash: null,
      content_preview: "---\npaths: [\"src/**/*.rs\"]\n---\n",
      content_truncated: false,
      line_count: 8,
      byte_size: 180,
      mtime_ms: Date.now(),
      frontmatter: {
        name: "rust-style",
        description: "Rust code style",
        paths: ["src/**/*.rs"],
        memory_scope: null,
        trigger: null,
        raw: "name: rust-style\ndescription: Rust code style\npaths:\n  - src/**/*.rs",
      },
      secret_issues: [
        {
          issue_type: "api_key",
          line_number: 3,
          column_start: 10,
          column_end: 30,
          matched_text: "sk-****abcd",
        },
      ],
      exists: true,
    },
    {
      id: "global-rule-no-fm",
      scope: "user",
      asset_type: "global_rule",
      logical_path: "/Users/test/.claude/rules/no-frontmatter.md",
      native_path: "/Users/test/.claude/rules/no-frontmatter.md",
      content_hash: null,
      content_preview: "# No frontmatter rule\n",
      content_truncated: false,
      line_count: 3,
      byte_size: 60,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "auto-memory-index",
      scope: "auto",
      asset_type: "auto_memory_index",
      logical_path: "/Users/test/.claude/projects/p1/memory/MEMORY.md",
      native_path: "/Users/test/.claude/projects/p1/memory/MEMORY.md",
      content_hash: null,
      content_preview: "# MEMORY\n",
      content_truncated: false,
      line_count: 250,
      byte_size: 8192,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "global-skill",
      scope: "user",
      asset_type: "global_skill",
      logical_path: "/Users/test/.claude/skills/git/SKILL.md",
      native_path: "/Users/test/.claude/skills/git/SKILL.md",
      content_hash: null,
      content_preview: "---\nname: git-helper\n---\n",
      content_truncated: false,
      line_count: 15,
      byte_size: 320,
      mtime_ms: Date.now(),
      frontmatter: {
        name: "git-helper",
        description: "Help with git",
        trigger: "user asks about git",
        paths: null,
        memory_scope: "project",
        raw: "name: git-helper\ndescription: Help with git\ntrigger: user asks about git\nmemory_scope: project",
      },
      secret_issues: [],
      exists: true,
    },
    {
      id: "truncated-file",
      scope: "user",
      asset_type: "user_claude_md",
      logical_path: "/Users/test/.claude/BIG.md",
      native_path: "/Users/test/.claude/BIG.md",
      content_hash: null,
      content_preview: "# Big file preview\nOnly first 2KB shown...",
      content_truncated: true,
      line_count: null,
      byte_size: 2097152,
      mtime_ms: Date.now(),
      frontmatter: null,
      secret_issues: [],
      exists: true,
    },
    {
      id: "missing-rule",
      scope: "user",
      asset_type: "global_rule",
      logical_path: "/Users/test/.claude/rules/missing.md",
      native_path: "/Users/test/.claude/rules/missing.md",
      content_hash: null,
      content_preview: null,
      content_truncated: false,
      line_count: null,
      byte_size: null,
      mtime_ms: null,
      frontmatter: null,
      secret_issues: [],
      exists: false,
    },
  ],
  summary: {
    total_assets: 9,
    total_existing: 8,
    by_scope: { user: 6, project: 2, auto: 1 },
    by_type: {
      user_claude_md: 2,
      project_claude_md: 1,
      global_rule: 3,
      project_rule: 1,
      auto_memory_index: 1,
      global_skill: 1,
    },
    total_secret_issues: 1,
  },
  errors: [],
};

async function mockClaudeMemoryInvoke(page: Page, options?: { rejectFileContent?: boolean }) {
  const mockData = JSON.stringify(mockOverview);
  const mockHealth = JSON.stringify(mockHealthReport);
  const mockPressure = JSON.stringify(mockContextPressure);
  const mockQueue = JSON.stringify(mockReviewQueue);
  const shouldReject = options?.rejectFileContent ?? false;
  await page.addInitScript(
    ({ data, health, pressure, queue, reject }) => {
      const overview = JSON.parse(data);
      const healthReport = JSON.parse(health);
      const contextPressure = JSON.parse(pressure);
      const reviewQueue = JSON.parse(queue);
      const win = window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
      };

      win.__TAURI_INTERNALS__ = {
        invoke: (command, _args) => {
          if (command === "get_claude_memory_dashboard") {
            return Promise.resolve({
              overview,
              health_report: healthReport,
              context_pressure: contextPressure,
              review_queue: reviewQueue,
            });
          }
          if (command === "get_claude_memory_overview") {
            return Promise.resolve(overview);
          }
          if (command === "get_claude_memory_file_content") {
            if (reject) {
              return Promise.reject(new Error("文件过大，无法读取"));
            }
            return Promise.resolve("# Mock Content\n\nThis is mock file content for testing.\n");
          }
          if (command === "get_review_queue") {
            return Promise.resolve(reviewQueue);
          }
          return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
        },
      };
    },
    { data: mockData, health: mockHealth, pressure: mockPressure, queue: mockQueue, reject: shouldReject },
  );
}

test.describe("ClaudeMemory", () => {
  test("从 Claude Code 域可导航到记忆资产", async ({ page }) => {
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "记忆资产" }).click();

    await expect(page.getByRole("heading", { name: "Claude 记忆" })).toBeVisible();
  });

  test("页面标题和刷新按钮可见", async ({ page }) => {
    await openClaudeMemory(page);

    await expect(page.getByRole("heading", { name: "Claude 记忆" })).toBeVisible();
    await expect(page.getByText("扫描 Instruction、Rules、Skills、Agents 和 Auto Memory")).toBeVisible();
    await expect(page.getByRole("button", { name: "刷新" })).toBeVisible();
  });

  test("Tauri 不可用时显示友好错误", async ({ page }) => {
    await openClaudeMemory(page);

    // Playwright 浏览器环境中 Tauri invoke 不可用，应显示错误状态
    // 错误消息可能为英文，不依赖具体文本，检查错误状态容器
    await expect(
      page.locator(".border-destructive\\/30.bg-destructive\\/5").first(),
    ).toBeVisible({ timeout: 10000 });
  });

  test("mock 数据下显示统计卡片", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("总资产")).toBeVisible();
    await expect(page.getByText("已存在")).toBeVisible();
    await expect(page.getByText("风险项")).toBeVisible();
    await expect(page.getByText("Claude 配置目录")).toBeVisible();

    // 精确断言统计数值（通过 data-stat 属性定位，避免与资产树中的数字混淆）
    await expect(page.locator('[data-stat="总资产"]')).toHaveText("9");
    await expect(page.locator('[data-stat="已存在"]')).toHaveText("8");
  });

  test("mock 数据下显示资产分组", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    // 等待加载完成
    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 验证分组标题（使用 heading role 避免匹配到副标题中的文字）
    await expect(page.getByRole("heading", { name: "Instruction" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Rules" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Auto Memory" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Skills & Agents" })).toBeVisible();
  });

  test("mock 数据下资产列表显示正确信息", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 验证具体资产项
    await expect(page.getByText("CLAUDE.md").first()).toBeVisible();
    await expect(page.getByText("style.md")).toBeVisible();
    await expect(page.getByText("MEMORY.md")).toBeVisible();
    await expect(page.getByText("SKILL.md")).toBeVisible();
  });

  test("MEMORY.md 过长显示警告标记", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 过长标记
    await expect(page.getByText("过长")).toBeVisible();
  });

  test("mock 数据下点击资产显示详情", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击第一个资产
    await page.getByText("CLAUDE.md").first().click();

    // 详情面板应显示元数据
    await expect(page.getByText("user_claude_md")).toBeVisible();
    // scope 标签在详情头部，用精确选择器避免匹配资产树中的同名标签
    await expect(page.locator(".bg-muted:has-text('user')").first()).toBeVisible();
  });

  test("mock 数据下 secret issue 显示在资产树中", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // secret issue 徽章
    await expect(page.getByText("1 项风险")).toBeVisible();
  });

  test("mock 数据下点击含 secret issue 的资产显示明细", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击有 secret issue 的 rule 资产
    await page.getByText("style.md").click();

    // 详情面板应显示 secret issue 明细
    await expect(page.getByText("敏感信息检测")).toBeVisible();
    await expect(page.getByText("api_key")).toBeVisible();
    await expect(page.getByText("sk-****abcd")).toBeVisible();
  });

  test("mock 数据下 skills 显示 frontmatter 信息", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击 skill 资产
    await page.getByText("SKILL.md").click();

    // frontmatter 信息
    await expect(page.getByText("git-helper")).toBeVisible();
    await expect(page.getByText("Help with git")).toBeVisible();
    await expect(page.getByText("user asks about git")).toBeVisible();
  });

  test("mock 数据下 rules 显示路径触发信息", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击 rule 资产
    await page.getByText("style.md").click();

    // 路径触发信息
    await expect(page.getByText("路径触发")).toBeVisible();
    await expect(page.getByText("src/**/*.rs")).toBeVisible();
  });

  test("rule 无 frontmatter 时显示全局加载", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击无 frontmatter 的 rule 资产
    await page.getByText("no-frontmatter.md").click();

    // 应显示全局加载
    await expect(page.getByText("全局加载")).toBeVisible();
    // 同时显示 asset_type 标签
    await expect(page.getByText("global_rule")).toBeVisible();
  });

  test("project rules 副标题显示项目名", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击 project rule 资产，验证副标题显示 "project" 而非 "rules"
    await page.getByText("01-project-rule.md").click();

    // 详情中 asset_type 标签应显示 project_rule
    await expect(page.getByText("project_rule")).toBeVisible();
    // 按钮的 accessible name 包含 "project"（而非 "rules"）
    await expect(
      page.getByRole("button", { name: /01-project-rule\.md.*project/ }),
    ).toBeVisible();
  });

  test("大文件 content_truncated 时显示预览和元数据", async ({ page }) => {
    await mockClaudeMemoryInvoke(page, { rejectFileContent: true });
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 点击截断文件
    await page.getByText("BIG.md").click();

    // 元数据应始终显示（使用更精确的选择器避免资产树中的同名元素）
    await expect(page.getByText("user_claude_md")).toBeVisible();
    await expect(page.getByText("预览已截断")).toBeVisible();
    await expect(page.getByText("2.0 MB").nth(1)).toBeVisible();

    // 预览截断提示和 preview 内容
    await expect(page.getByText("文件过大，仅展示扫描预览")).toBeVisible();
    await expect(page.getByText("Only first 2KB shown...")).toBeVisible();

    // 错误信息也应显示
    await expect(page.getByText("读取失败")).toBeVisible();
    await expect(page.getByText("文件过大，无法读取")).toBeVisible();
  });

  test("默认隐藏不存在的资产", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // missing.md 默认不应显示
    await expect(page.getByText("missing.md")).not.toBeVisible();
    // 但其他存在的资产应显示
    await expect(page.getByText("CLAUDE.md").first()).toBeVisible();
  });

  test("关闭隐藏不存在开关后显示缺失资产", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 默认隐藏
    await expect(page.getByText("missing.md")).not.toBeVisible();

    // 关闭开关
    await page.locator('button[role="switch"]').click();

    // missing.md 现在应可见
    await expect(page.getByText("missing.md")).toBeVisible();
    // 应有"不存在"标签（限定在 missing.md 按钮内）
    await expect(
      page.getByRole("button", { name: /missing\.md/ }).locator("span:has-text('不存在')"),
    ).toBeVisible();
  });

  test("重新隐藏不存在时清理已隐藏的选中项", async ({ page }) => {
    await mockClaudeMemoryInvoke(page);
    await openClaudeMemory(page);

    await expect(page.getByText("记忆资产").first()).toBeVisible();

    // 关闭隐藏不存在，点击 missing.md
    await page.locator('button[role="switch"]').click();
    await page.getByText("missing.md").click();

    // 右侧详情应显示 missing.md
    await expect(page.getByText("global_rule")).toBeVisible();

    // 重新开启隐藏不存在
    await page.locator('button[role="switch"]').click();

    // missing.md 被隐藏，右侧应回到空状态
    await expect(page.getByText("选择左侧记忆资产查看内容")).toBeVisible();
  });
});
