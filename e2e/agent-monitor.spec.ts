import { test, expect, type Page } from "@playwright/test";

/**
 * AgentMonitor E2E 测试
 *
 * 测试 Agent 监控面板的空状态渲染。
 * Tauri event listener 在浏览器环境不可用，
 * totalSessions 始终为 0 → 显示"暂无活跃 Agent"空状态。
 */

test.describe("AgentMonitor", () => {
  test.beforeEach(async ({ page }) => {
    await openAgentMonitor(page);
  });

  test("显示页面标题和副标题", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Agent 监控" })).toBeVisible();
    await expect(page.getByText("通过 Tauri 事件流展示 Token 消耗速率（burn rate）、上下文窗口占用和会话在线状态。")).toBeVisible();
  });

  test("无 Agent 时显示空状态", async ({ page }) => {
    // Tauri listen 不可用 → 无数据 → 空状态
    await expect(page.getByText("暂无活跃 Agent")).toBeVisible({ timeout: 8000 });
    await expect(page.getByText("Collector 会继续监听 agent-update 事件")).toBeVisible();
  });

  test("显示汇总指标卡片", async ({ page }) => {
    // 即使无 Agent，汇总卡片仍渲染
    await expect(page.getByText("会话总数")).toBeVisible();
    await expect(page.getByText("已注册项目")).toBeVisible();
    await expect(page.getByText("刷新时间")).toBeVisible();
  });

  test("显示 token/s token/min 切换按钮", async ({ page }) => {
    await expect(page.getByRole("button", { name: "token/s" })).toBeVisible();
    await expect(page.getByRole("button", { name: "token/min" })).toBeVisible();

    // 默认 token/min 应该处于激活样式（secondary variant has bg-secondary class）
    const defaultActive = page.getByRole("button", { name: "token/min" });
    await expect(defaultActive).toHaveClass(/bg-secondary/);

    // 点击 token/s 切换
    await page.getByRole("button", { name: "token/s" }).click();

    // 点击后 token/s 应该获得激活样式，token/min 失去激活样式
    await expect(defaultActive).not.toHaveClass(/bg-secondary/);
    await expect(page.getByRole("button", { name: "token/s" })).toHaveClass(/bg-secondary/);
  });

  test("搜索输入框存在且可交互", async ({ page }) => {
    const searchInput = page.getByRole("textbox", { name: "搜索 Agent 会话" });
    await expect(searchInput).toBeVisible();

    // 输入文本
    await searchInput.fill("test");
    await expect(searchInput).toHaveValue("test");

    // 清空按钮出现
    await expect(page.getByRole("button", { name: "清空搜索" })).toBeVisible();

    // 点击清空
    await page.getByRole("button", { name: "清空搜索" }).click();
    await expect(searchInput).toHaveValue("");
  });

  test("搜索无匹配时显示空状态", async ({ page }) => {
    await mockAgentUpdateEvent(page);
    await openAgentMonitor(page);

    const searchInput = page.getByRole("textbox", { name: "搜索 Agent 会话" });
    await searchInput.fill("zzzznotexist");

    // 显示空状态
    await expect(page.getByText("没有匹配的会话")).toBeVisible();
    const clearSearchButton = page.getByRole("button", { name: "清空搜索" }).first();
    await expect(clearSearchButton).toBeVisible();

    // 点击清空恢复
    await clearSearchButton.click();
    await expect(page.getByText("暂无活跃 Agent")).not.toBeVisible();
  });

  test("主题切换按钮存在", async ({ page }) => {
    // ThemeToggle 在 Layout 中，所有页面都有
    const themeButton = page.locator(".theme-toggle, [aria-label*='模式'], [aria-label='跟随系统']").first();
    await expect(themeButton).toBeVisible();

    const beforeLabel = await themeButton.getAttribute("aria-label");
    await themeButton.click();
    await expect(themeButton).not.toHaveAttribute("aria-label", beforeLabel ?? "");
  });

  test("无 Agent 时控件正常显示", async ({ page }) => {
    // 搜索框可见
    await expect(page.getByRole("textbox", { name: "搜索 Agent 会话" })).toBeVisible();

    // token/s token/min 按钮可见
    await expect(page.getByRole("button", { name: "token/s" })).toBeVisible();
    await expect(page.getByRole("button", { name: "token/min" })).toBeVisible();

    // 汇总卡片可见
    await expect(page.getByText("会话总数")).toBeVisible();
    await expect(page.getByText("已注册项目")).toBeVisible();
    await expect(page.getByText("刷新时间")).toBeVisible();

    // 主题按钮可见
    const themeButton = page.locator(".theme-toggle, [aria-label*='模式'], [aria-label='跟随系统']").first();
    await expect(themeButton).toBeVisible();
  });

  test("mock 数据下显示其他工作目录标题和描述", async ({ page }) => {
    await mockAgentUpdateEvent(page);
    await openAgentMonitor(page);

    // 其他工作目录卡片标题
    await expect(page.getByText("其他工作目录")).toBeVisible({ timeout: 8000 });
    // 未匹配描述文本
    await expect(page.getByText("未匹配项目监控中已注册的项目路径")).toBeVisible();
  });
});

async function openAgentMonitor(page: Page) {
  await page.goto("/");
  await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
}

async function mockAgentUpdateEvent(page: Page) {
  await page.addInitScript(() => {
    const callbacks = new Map<number, (event: unknown) => void>();
    let nextCallbackId = 1;

    const sampleAgent = {
      agent_type: "build",
      session_id: "sample-session-001",
      cwd: "/tmp/sample-project",
      project_name: "sample-project",
      status: "Thinking",
      model: "gpt-test",
      context_percent: 12,
      context_window: 100000,
      total_input_tokens: 1200,
      total_output_tokens: 300,
      total_cache_read: 0,
      total_cache_create: 0,
      turn_count: 2,
      current_tasks: [],
      mem_mb: 128,
      git_branch: "main",
      git_added: 0,
      git_modified: 0,
      token_history: [],
      context_history: [],
      compaction_count: 0,
      token_rate: 1.5,
      pid: 12345,
      version: "test",
      effort: "medium",
      tool_calls: [],
      subagents: [],
      file_accesses: [],
      pending_since_ms: 0,
      thinking_since_ms: 0,
    };

    const unmappedAgent = {
      ...sampleAgent,
      session_id: "unmapped-session-001",
      cwd: "/tmp/other-workspace",
      project_name: "other-workspace",
    };

    const agentPayload = {
      projects: [{ project_path: "/tmp/sample-project", agents: [sampleAgent], count: 1 }],
      unmapped: [unmappedAgent],
      timestamp_ms: Date.now(),
      total_sessions: 2,
    };

    const win = window as unknown as {
      __TAURI_INTERNALS__: {
        transformCallback: (callback: (event: unknown) => void) => number;
        unregisterCallback: (id: number) => void;
        invoke: (command: string, args?: { handler?: number }) => Promise<unknown>;
      };
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => void;
      };
    };

    win.__TAURI_INTERNALS__ = {
      transformCallback: (callback) => {
        const id = nextCallbackId;
        nextCallbackId += 1;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback: (id) => {
        callbacks.delete(id);
      },
      invoke: (command, args) => {
        if (command === "plugin:event|listen") {
          const callbackId = args?.handler;
          window.setTimeout(() => {
            if (callbackId) {
              callbacks.get(callbackId)?.({ event: "agent-update", id: 1, payload: agentPayload });
            }
          }, 0);
          return Promise.resolve(1);
        }

        if (command === "plugin:event|unlisten") {
          return Promise.resolve();
        }

        return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
      },
    };
    win.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => undefined };
  });
}
