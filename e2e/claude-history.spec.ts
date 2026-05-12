import { test, expect } from "@playwright/test";

/**
 * Claude Code 会话历史 E2E 测试
 *
 * 由于 Playwright 运行在浏览器环境中，无法调用 Tauri 后端 API，
 * 因此测试仅覆盖 UI 渲染、空状态和导航交互。
 */

test.describe("ClaudeHistory", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("导航到会话历史页面显示正确标题", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "会话历史" }).click();

    await expect(page.getByRole("heading", { name: "会话历史" })).toBeVisible();
  });

  test("Tauri 不可用时显示错误状态", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "会话历史" }).click();

    // Playwright 浏览器环境中 Tauri invoke 不可用，会显示错误
    await expect(page.locator("text=/error/i").first()).toBeVisible({ timeout: 10000 });
  });

  test("搜索输入框存在且可交互", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "会话历史" }).click();

    const searchInput = page.locator('input[placeholder*="搜索"]');
    await expect(searchInput).toBeVisible();

    await searchInput.fill("test query");
    await expect(searchInput).toHaveValue("test query");
  });

  test("刷新按钮存在且可点击", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "会话历史" }).click();

    const refreshBtn = page.getByRole("button", { name: "刷新" });
    await expect(refreshBtn).toBeVisible();
    await expect(refreshBtn).toBeEnabled();
  });
});
