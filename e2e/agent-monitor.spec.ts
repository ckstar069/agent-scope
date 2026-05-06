import { test, expect } from "@playwright/test";

/**
 * AgentMonitor E2E 测试
 * 
 * 测试 Agent 监控面板的空状态渲染。
 * Tauri event listener 在浏览器环境不可用，
 * totalSessions 始终为 0 → 显示"暂无活跃 Agent"空状态。
 */

test.describe("AgentMonitor", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Agents" }).click();
  });

  test("显示页面标题和副标题", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "Agent 监控" })).toBeVisible();
    await expect(page.getByText("通过 Tauri 事件流展示实时 Token 速率、上下文窗口占用和会话在线状态")).toBeVisible();
  });

  test("无 Agent 时显示空状态", async ({ page }) => {
    // Tauri listen 不可用 → 无数据 → 空状态
    await expect(page.getByText("暂无活跃 Agent")).toBeVisible({ timeout: 8000 });
    await expect(page.getByText("Collector 会继续监听 agent-update 事件")).toBeVisible();
  });

  test("显示汇总指标卡片", async ({ page }) => {
    // 即使无 Agent，汇总卡片仍渲染
    await expect(page.getByText("会话总数")).toBeVisible();
    await expect(page.getByText("关联项目")).toBeVisible();
    await expect(page.getByText("刷新时间")).toBeVisible();
  });

  test("显示 token/s token/min 切换按钮", async ({ page }) => {
    await expect(page.getByRole("button", { name: "token/s" })).toBeVisible();
    await expect(page.getByRole("button", { name: "token/min" })).toBeVisible();

    // 默认 token/s 应该处于激活样式（secondary variant has bg-secondary class）
    const defaultActive = page.getByRole("button", { name: "token/s" });
    await expect(defaultActive).toHaveClass(/bg-secondary/);

    // 点击 token/min 切换
    await page.getByRole("button", { name: "token/min" }).click();

    // 点击后 token/min 应该获得激活样式，token/s 失去激活样式
    await expect(defaultActive).not.toHaveClass(/bg-secondary/);
    await expect(page.getByRole("button", { name: "token/min" })).toHaveClass(/bg-secondary/);
  });
});
