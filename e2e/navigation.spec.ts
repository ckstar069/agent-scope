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

  test("默认路由为 Dashboard", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("侧边栏四个导航按钮都存在", async ({ page }) => {
    const nav = page.locator('nav[aria-label="主导航"]');
    await expect(nav.getByRole("button", { name: "Dashboard" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "Projects" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "Agents" })).toBeVisible();
    await expect(nav.getByRole("button", { name: "Settings" })).toBeVisible();
  });

  test("导航到 Dashboard 显示正确内容", async ({ page }) => {
    // 先切到其他路由再切回
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Settings" }).click();
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Dashboard" }).click();

    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("导航到 Projects 显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();

    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible();
  });

  test("导航到 Agents 显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Agents" }).click();

    await expect(page.getByRole("heading", { name: "Agent 监控" })).toBeVisible();
  });

  test("导航到 Settings 显示正确内容", async ({ page }) => {
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Settings" }).click();

    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  });

  test("活动路由按钮有 aria-current='page'", async ({ page }) => {
    const nav = page.locator('nav[aria-label="主导航"]');

    // Dashboard 默认激活
    await expect(nav.getByRole("button", { name: "Dashboard" })).toHaveAttribute("aria-current", "page");

    await nav.getByRole("button", { name: "Settings" }).click();
    await expect(nav.getByRole("button", { name: "Settings" })).toHaveAttribute("aria-current", "page");
    await expect(nav.getByRole("button", { name: "Dashboard" })).not.toHaveAttribute("aria-current", "page");
  });

  test("侧边栏折叠/展开按钮存在", async ({ page }) => {
    const collapseBtn = page.getByRole("button", { name: "收起侧边栏" });
    await expect(collapseBtn).toBeVisible();

    // 点击折叠
    await collapseBtn.click();
    const expandBtn = page.getByRole("button", { name: "展开侧边栏" });
    await expect(expandBtn).toBeVisible();
  });

  test("应用在 dark 模式下渲染", async ({ page }) => {
    const darkContainer = page.locator("div.dark");
    await expect(darkContainer).toBeVisible();
  });
});
