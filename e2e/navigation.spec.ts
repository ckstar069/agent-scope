import { test, expect } from "@playwright/test";

/**
 * Navigation E2E 测试
 *
 * 测试顶部大域导航 + 侧边栏子导航的切换功能。
 * 验证三个大域及子页面都能正确渲染。
 */

test.describe("Navigation", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test("默认路由为项目监控域的项目概览", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("顶部四个大域导航按钮都存在", async ({ page }) => {
    const topNav = page.locator('nav[aria-label="大域导航"]');
    await expect(topNav.getByRole("button", { name: "项目监控" })).toBeVisible();
    await expect(topNav.getByRole("button", { name: "通用监控" })).toBeVisible();
    await expect(topNav.getByRole("button", { name: "Claude 记忆" })).toBeVisible();
    await expect(topNav.getByRole("button", { name: "设置" })).toBeVisible();
  });

  test("切换到 Claude 记忆域显示记忆资产", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude 记忆" }).click();

    // 顶部高亮
    const claudeMemoryBtn = page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude 记忆" });
    await expect(claudeMemoryBtn).toHaveAttribute("aria-current", "page");

    // 侧边栏应显示记忆资产和加载链模拟器
    const subNav = page.locator('nav[aria-label="子导航"]');
    await expect(subNav.getByRole("button", { name: "记忆资产" })).toBeVisible();
    await expect(subNav.getByRole("button", { name: "加载链模拟器" })).toBeVisible();

    // 默认显示 Claude 记忆内容
    await expect(page.getByRole("heading", { name: "Claude 记忆" })).toBeVisible();
  });

  test("Claude 记忆域可切换到加载链模拟器", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude 记忆" }).click();
    await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "加载链模拟器" }).click();

    await expect(page.getByRole("heading", { name: "加载链模拟器" })).toBeVisible();
  });

  test("设置域侧边栏不再显示 Claude 记忆", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "设置" }).click();

    const subNav = page.locator('nav[aria-label="子导航"]');
    await expect(subNav.getByRole("button", { name: "项目设置" })).toBeVisible();
    await expect(subNav.getByRole("button", { name: "通用设置" })).toBeVisible();
    await expect(subNav.getByRole("button", { name: "Claude 记忆" })).not.toBeVisible();
  });

  test("项目监控域侧边栏显示项目概览和项目列表", async ({ page }) => {
    const subNav = page.locator('nav[aria-label="子导航"]');
    await expect(subNav.getByRole("button", { name: "项目概览" })).toBeVisible();
  });

  test("切换到通用监控域显示 Agent 监控", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "通用监控" }).click();

    // 侧边栏应显示子导航项
    const subNav = page.locator('nav[aria-label="子导航"]');
    await expect(subNav.getByRole("button", { name: "Agent 监控" })).toBeVisible();
    await expect(subNav.getByRole("button", { name: "会话管理" })).toBeVisible();

    // 默认显示 Agent 监控内容
    await expect(page.getByRole("heading", { name: "Agent 监控" })).toBeVisible();
  });

  test("通用监控域可切换到会话管理", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "通用监控" }).click();
    await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "会话管理" }).click();

    await expect(page.getByRole("heading", { name: "会话管理" })).toBeVisible();
  });

  test("切换到设置域显示项目设置", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "设置" }).click();

    // 侧边栏应显示子导航项
    const subNav = page.locator('nav[aria-label="子导航"]');
    await expect(subNav.getByRole("button", { name: "项目设置" })).toBeVisible();
    await expect(subNav.getByRole("button", { name: "通用设置" })).toBeVisible();

    // 默认显示项目设置内容
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  });

  test("设置域可切换到通用设置", async ({ page }) => {
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "设置" }).click();
    await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "通用设置" }).click();

    await expect(page.getByText("界面字号")).toBeVisible();
    await expect(page.getByText("界面主题")).toBeVisible();
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
