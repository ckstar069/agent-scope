import { test, expect } from "@playwright/test";

/**
 * Navigation E2E 测试
 *
 * 测试侧边栏导航切换路由功能。
 * 验证四个面板都能正确渲染。
 */

test.describe("Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("默认路由为仪表盘", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("侧边栏四个导航按钮都存在", async ({ page }) => {
    const nav = page.locator('nav[aria-label="主导航"]');
    await expect(nav.getByRole("button", { name: "仪表盘" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "代理监控" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "会话管理" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "设置" })).toBeVisible();
  });

  test("导航到仪表盘显示正确内容", async ({ page }) => {
    // 先切到其他路由再切回
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "设置" }).click();
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "仪表盘" }).click();

    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("导航到代理监控显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "代理监控" }).click();

    await expect(page.getByRole("heading", { name: "Agent 监控" })).toBeVisible();
  });

  test("导航到会话管理显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "会话管理" }).click();

    await expect(page.getByRole("heading", { name: "会话管理" })).toBeVisible();
  });

  test("导航到设置显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "设置" }).click();

    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  });

  test("活动路由按钮有 aria-current='page'", async ({ page }) => {
    const nav = page.locator('nav[aria-label="主导航"]');

    // 仪表盘默认激活
    await expect(nav.getByRole("button", { name: "仪表盘" })).toHaveAttribute("aria-current", "page");

    await nav.getByRole("button", { name: "设置" }).click();
    await expect(nav.getByRole("button", { name: "设置" })).toHaveAttribute("aria-current", "page");
    await expect(nav.getByRole("button", { name: "仪表盘" })).not.toHaveAttribute("aria-current", "page");
  });

  test("侧边栏折叠/展开按钮存在", async ({ page }) => {
    const collapseBtn = page.getByRole("button", { name: "收起侧边栏" });
    await expect(collapseBtn).toBeVisible();

    // 点击折叠
    await collapseBtn.click();
    const expandBtn = page.getByRole("button", { name: "展开侧边栏" });
    await expect(expandBtn).toBeVisible();
  });

  test("主题切换按钮存在且可切换主题", async ({ page }) => {
    const themeButton = page.locator("[aria-label='浅色模式'], [aria-label='深色模式'], [aria-label='跟随系统']").first();
    await expect(themeButton).toBeVisible();

    await themeButton.click();
    await themeButton.click();
    const darkContainer = page.locator("html.dark");
    await expect(darkContainer).toBeVisible();
  });
});
