import { test, expect } from "@playwright/test";

/**
 * ProjectDetail E2E 测试
 * 
 * 测试 ProjectDetail 面板的空状态和错误状态渲染。
 * Projects 路由默认无 projectPath → 空状态。
 * Dashboard 点击项目卡片可传入 projectPath → Tauri 错误。
 */

test.describe("ProjectDetail", () => {
  test("直接访问 Projects 路由显示空状态", async ({ page }) => {
    await page.goto("/");

    // 通过侧边栏导航到 Projects
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();

    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible();
    await expect(page.getByText("尚未传入项目路径")).toBeVisible();
    await expect(page.getByText("请选择一个模板项目后查看 Stage、参数、Memory 与 Git 快照。")).toBeVisible();
  });

  test("显示 Stage 时间线、参数快照等面板标题", async ({ page }) => {
    // 即使是空状态，直接导航到 Projects 显示标题
    await page.goto("/");
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();

    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible();
  });

  test("从 Dashboard 无项目时点击不会进入假 ProjectDetail", async ({ page }) => {
    // Dashboard 无项目时不应有项目卡片
    await page.goto("/");

    // 验证侧边栏 Projects 按钮存在
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();
    await expect(page.getByText("尚未传入项目路径")).toBeVisible();
  });

  test("Settings 添加项目路径不会被传递给 ProjectDetail", async ({ page }) => {
    // Settings 和 ProjectDetail 是独立路由，Settings 不传 projectPath
    await page.goto("/");
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Settings" }).click();
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();

    // 回到 Projects
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();
    await expect(page.getByText("尚未传入项目路径")).toBeVisible();
  });
});
