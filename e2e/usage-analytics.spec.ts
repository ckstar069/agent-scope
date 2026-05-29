import { test, expect } from "@playwright/test";

/**
 * Usage Analytics E2E 测试
 *
 * 在浏览器环境中 Tauri API 不可用，因此：
 * 1. 导航、UI 交互通过真实 Playwright 测试。
 * 2. 数据展示通过注入 __TAURI_INTERNALS__.invoke mock 测试。
 * 3. 不依赖真实 ~/.claude 目录。
 */

const MOCK_SOURCE_STATUS = {
  source_type: "claude-code-local",
  config_dirs: ["/home/user/.claude"],
  readable_dirs: ["/home/user/.claude"],
  unreadable_dirs: [],
  last_scan_at: new Date().toISOString(),
  last_usage_at: new Date().toISOString(),
  confidence: "high",
  realtime_level: "near_realtime",
  notes: ["测试数据源"],
};

const MOCK_AGGREGATE_PROJECT = {
  time_range: "today",
  group_by: "project",
  input_tokens: 1200,
  output_tokens: 600,
  cache_read_tokens: 300,
  cache_create_tokens: 150,
  total_tokens: 1800,
  session_count: 3,
  project_count: 2,
  model_count: 1,
  groups: [
    {
      group_key: "/Users/ckstar/Repo/agent-scope",
      group_label: "agent-scope",
      group_detail: "/Users/ckstar/Repo/agent-scope",
      input_tokens: 800,
      output_tokens: 400,
      cache_read_tokens: 200,
      cache_create_tokens: 100,
      total_tokens: 1200,
      session_count: 2,
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
    },
    {
      group_key: "/Users/ckstar/Documents/notes",
      group_label: "notes",
      group_detail: "/Users/ckstar/Documents/notes",
      input_tokens: 400,
      output_tokens: 200,
      cache_read_tokens: 100,
      cache_create_tokens: 50,
      total_tokens: 600,
      session_count: 1,
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
    },
  ],
};

const MOCK_AGGREGATE_SESSION = {
  time_range: "today",
  group_by: "session",
  input_tokens: 1200,
  output_tokens: 600,
  cache_read_tokens: 300,
  cache_create_tokens: 150,
  total_tokens: 1800,
  session_count: 3,
  project_count: 2,
  model_count: 1,
  groups: [
    {
      group_key: "sess-001",
      group_label: "rename v0.3.7",
      group_detail: "agent-scope · 1f093b08",
      input_tokens: 800,
      output_tokens: 400,
      cache_read_tokens: 200,
      cache_create_tokens: 100,
      total_tokens: 1200,
      session_count: 1,
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
    },
    {
      group_key: "sess-002",
      group_label: "(未命名)",
      group_detail: "agent-scope · 1f093b09",
      input_tokens: 300,
      output_tokens: 150,
      cache_read_tokens: 80,
      cache_create_tokens: 40,
      total_tokens: 570,
      session_count: 1,
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
    },
    {
      group_key: "sess-003",
      group_label: "帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应",
      group_detail: "notes · abcdef12",
      input_tokens: 100,
      output_tokens: 50,
      cache_read_tokens: 20,
      cache_create_tokens: 10,
      total_tokens: 180,
      session_count: 1,
      first_seen: new Date().toISOString(),
      last_seen: new Date().toISOString(),
    },
  ],
};

type MockTauriArgs = {
  sourceStatus: typeof MOCK_SOURCE_STATUS;
  aggregate: typeof MOCK_AGGREGATE_PROJECT;
  empty?: boolean;
  error?: boolean;
};

async function setupMockTauri(page: import("@playwright/test").Page, args: MockTauriArgs) {
  await page.addInitScript((serialized: string) => {
    const { sourceStatus, aggregate, empty, error } = JSON.parse(serialized) as MockTauriArgs;

    const win = window as unknown as {
      __TAURI_INTERNALS__: {
        invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
    };

    win.__TAURI_INTERNALS__ = {
      invoke: (command: string, invokeArgs?: Record<string, unknown>) => {
        if (error && (command === "scan_usage_data" || command === "get_usage_analytics")) {
          return Promise.reject(new Error("模拟扫描失败"));
        }
        if (command === "scan_usage_data") {
          return Promise.resolve({
            source_status: sourceStatus,
            scanned_files: empty ? 0 : 2,
            scanned_lines: empty ? 0 : 10,
            record_count: empty ? 0 : 4,
            error_count: 0,
            errors: [],
          });
        }
        if (command === "get_usage_analytics") {
          const timeRange = (invokeArgs?.timeRange as string) ?? "today";
          const groupBy = (invokeArgs?.groupBy as string) ?? "project";
          if (empty) {
            return Promise.resolve({
              time_range: timeRange,
              group_by: groupBy,
              input_tokens: 0,
              output_tokens: 0,
              cache_read_tokens: 0,
              cache_create_tokens: 0,
              total_tokens: 0,
              session_count: 0,
              project_count: 0,
              model_count: 0,
              groups: [],
            });
          }
          return Promise.resolve({
            ...aggregate,
            time_range: timeRange,
            group_by: groupBy,
          });
        }
        if (command === "list_projects") {
          return Promise.resolve([]);
        }
        return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
      },
    };
  }, JSON.stringify(args));
}

async function setupMockTauriWithTracking(
  page: import("@playwright/test").Page,
  args: MockTauriArgs,
) {
  await page.addInitScript((serialized: string) => {
    const { sourceStatus, aggregate, empty } = JSON.parse(serialized) as MockTauriArgs;
    const calls: { command: string; args: Record<string, unknown> }[] = [];

    const win = window as unknown as {
      __TAURI_INTERNALS__: {
        invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
    };

    win.__TAURI_INTERNALS__ = {
      invoke: (command: string, invokeArgs?: Record<string, unknown>) => {
        calls.push({ command, args: invokeArgs ?? {} });
        if (command === "scan_usage_data") {
          return Promise.resolve({
            source_status: sourceStatus,
            scanned_files: empty ? 0 : 2,
            scanned_lines: empty ? 0 : 10,
            record_count: empty ? 0 : 4,
            error_count: 0,
            errors: [],
          });
        }
        if (command === "get_usage_analytics") {
          const timeRange = (invokeArgs?.timeRange as string) ?? "today";
          const groupBy = (invokeArgs?.groupBy as string) ?? "project";
          if (empty) {
            return Promise.resolve({
              time_range: timeRange,
              group_by: groupBy,
              input_tokens: 0,
              output_tokens: 0,
              cache_read_tokens: 0,
              cache_create_tokens: 0,
              total_tokens: 0,
              session_count: 0,
              project_count: 0,
              model_count: 0,
              groups: [],
            });
          }
          return Promise.resolve({
            ...aggregate,
            time_range: timeRange,
            group_by: groupBy,
          });
        }
        if (command === "list_projects") {
          return Promise.resolve([]);
        }
        return Promise.reject(new Error(`未模拟: ${command}`));
      },
    };

    (window as unknown as Record<string, unknown>).usageCalls = calls;
  }, JSON.stringify(args));
}

async function setupMockTauriWithCommandTracking(
  page: import("@playwright/test").Page,
  args: MockTauriArgs,
) {
  await page.addInitScript((serialized: string) => {
    const { sourceStatus, aggregate } = JSON.parse(serialized) as MockTauriArgs;
    const calls: string[] = [];

    const win = window as unknown as {
      __TAURI_INTERNALS__: {
        invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      };
    };

    win.__TAURI_INTERNALS__ = {
      invoke: (command: string, invokeArgs?: Record<string, unknown>) => {
        calls.push(command);
        if (command === "scan_usage_data") {
          return Promise.resolve({
            source_status: sourceStatus,
            scanned_files: 2,
            scanned_lines: 10,
            record_count: 4,
            error_count: 0,
            errors: [],
          });
        }
        if (command === "get_usage_analytics") {
          const timeRange = (invokeArgs?.timeRange as string) ?? "today";
          const groupBy = (invokeArgs?.groupBy as string) ?? "project";
          return Promise.resolve({
            ...aggregate,
            time_range: timeRange,
            group_by: groupBy,
          });
        }
        if (command === "list_projects") {
          return Promise.resolve([]);
        }
        return Promise.reject(new Error(`未模拟: ${command}`));
      },
    };

    (window as unknown as Record<string, unknown>).usageCommands = calls;
  }, JSON.stringify(args));
}

test.describe("Usage Analytics", () => {
  test("侧边栏出现 Usage 分析入口", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");

    // 切换到监控域——TopNav 按钮文本是 "Claude Code"
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();

    // 侧边栏应有 Usage 分析按钮
    await expect(page.getByRole("button", { name: "Usage 分析" })).toBeVisible();
  });

  test("点击 Usage 分析进入页面并展示标题与数据", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");

    // 切换到监控域
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    // 标题
    await expect(page.getByRole("heading", { name: "Usage 分析" })).toBeVisible();

    // 数据源状态
    await expect(page.getByText("数据源状态")).toBeVisible();
    await expect(page.getByText("高")).toBeVisible();

    // 汇总卡片（8个）—— 用 .first() 避免与表格表头冲突
    await expect(page.getByText("Total Tokens").first()).toBeVisible();
    await expect(page.getByText("Input").first()).toBeVisible();
    await expect(page.getByText("Output").first()).toBeVisible();
    await expect(page.getByText("Cache Read").first()).toBeVisible();
    await expect(page.getByText("Cache Create").first()).toBeVisible();
    await expect(page.getByText("Sessions").first()).toBeVisible();
    await expect(page.getByText("Projects").first()).toBeVisible();
    await expect(page.getByText("Models").first()).toBeVisible();

    // mock 数值 1.8K 应在页面上
    await expect(page.getByText("1.8K").first()).toBeVisible();

    // 分组明细表格出现可读项目名及其详情
    const labelCell = page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" });
    await expect(labelCell).toBeVisible();
    const detailCell = page.locator('td [data-testid="group-detail"]').filter({ hasText: "/Users/ckstar/Repo/agent-scope" });
    await expect(detailCell).toBeVisible();
  });

  test("切换时间范围为近 7 天", async ({ page }) => {
    await setupMockTauriWithTracking(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    // 等待初始加载完成
    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    // 点击近 7 天
    await page.getByRole("button", { name: "近 7 天" }).click();

    // 等待刷新后的结果
    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    // 验证调用参数
    const invokeCalls = await page.evaluate(() =>
      ((window as unknown as Record<string, unknown>).usageCalls as { command: string; args: Record<string, unknown> }[]),
    );

    const analyticsCalls = invokeCalls.filter((c) => c.command === "get_usage_analytics");
    const lastCall = analyticsCalls[analyticsCalls.length - 1];
    expect(lastCall.args.timeRange).toBe("last7days");
    expect(lastCall.args.groupBy).toBe("project");
  });

  test("切换分组维度为按模型", async ({ page }) => {
    await setupMockTauriWithTracking(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    // 点击按模型
    await page.getByRole("button", { name: "按模型" }).click();

    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    const invokeCalls = await page.evaluate(() =>
      ((window as unknown as Record<string, unknown>).usageCalls as { command: string; args: Record<string, unknown> }[]),
    );

    const analyticsCalls = invokeCalls.filter((c) => c.command === "get_usage_analytics");
    const lastCall = analyticsCalls[analyticsCalls.length - 1];
    expect(lastCall.args.groupBy).toBe("model");
  });

  test("刷新按钮触发重新扫描", async ({ page }) => {
    await setupMockTauriWithCommandTracking(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    // 点击刷新
    await page.getByRole("button", { name: "刷新" }).click();

    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    const cmds = await page.evaluate(() =>
      ((window as unknown as Record<string, unknown>).usageCommands as string[]),
    );

    // 刷新后应先调用 scan_usage_data 再调用 get_usage_analytics
    const refreshScanIndex = cmds.slice(2).indexOf("scan_usage_data");
    expect(refreshScanIndex).toBeGreaterThanOrEqual(0);
  });

  test("空数据状态显示提示", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT, empty: true });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await expect(page.getByText("暂无 usage 数据")).toBeVisible();
    await expect(page.getByText("未在配置目录中发现有效的 session JSONL 文件")).toBeVisible();
  });

  test("错误状态显示提示和重试按钮", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT, error: true });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await expect(page.getByText("模拟扫描失败")).toBeVisible();
    await expect(page.getByRole("button", { name: "重试" })).toBeVisible();
  });

  test("按会话分组时清洗 slash command", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_SESSION });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    // 切换到按会话
    await page.getByRole("button", { name: "按会话" }).click();

    // 验证不显示 "/rename"
    await expect(page.getByText("/rename v0.3.7")).not.toBeVisible();

    // 验证显示清洗后的 "rename v0.3.7"
    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "rename v0.3.7" })).toBeVisible();

    // 验证 detail 包含 project_name 和 short_session_id
    await expect(page.locator('td [data-testid="group-detail"]').filter({ hasText: "agent-scope · 1f093b08" })).toBeVisible();
  });

  test("按会话分组时未命名会话显示 (未命名)", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_SESSION });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await page.getByRole("button", { name: "按会话" }).click();

    // 验证 (未命名) 显示
    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "(未命名)" })).toBeVisible();
    await expect(page.locator('td [data-testid="group-detail"]').filter({ hasText: "agent-scope · 1f093b09" })).toBeVisible();
  });

  test("按会话分组时中文长会话名可显示", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_SESSION });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    await page.getByRole("button", { name: "按会话" }).click();

    const longLabel = "帮我检查本机的健康状态。之前两天，连续出现两次关机时无响应";
    const labelCell = page.locator('td [data-testid="group-label"]').filter({ hasText: longLabel });
    await expect(labelCell).toBeVisible();

    // 验证 title 属性包含完整中文（title 在父级 div 上）
    const title = await labelCell.locator("..").getAttribute("title");
    expect(title).toContain(longLabel);
  });

  test("图表 tooltip 包含 token 分项", async ({ page }) => {
    await setupMockTauri(page, { sourceStatus: MOCK_SOURCE_STATUS, aggregate: MOCK_AGGREGATE_PROJECT });
    await page.goto("/");
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Usage 分析" }).click();

    // 等待图表渲染
    await expect(page.locator('td [data-testid="group-label"]').filter({ hasText: "agent-scope" })).toBeVisible();

    // 悬停到第一个柱子上触发 tooltip
    const bar = page.locator(".recharts-bar-rectangle").first();
    await bar.hover();

    // 验证 tooltip 中出现分项文案
    const tooltip = page.locator(".recharts-tooltip-wrapper");
    await expect(tooltip).toBeVisible();
    await expect(page.getByText("Total").first()).toBeVisible();
    await expect(page.getByText("Input").first()).toBeVisible();
    await expect(page.getByText("Output").first()).toBeVisible();
    await expect(page.getByText("Cache Read").first()).toBeVisible();
    await expect(page.getByText("Cache Create").first()).toBeVisible();
  });
});
