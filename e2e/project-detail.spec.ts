import { test, expect } from "@playwright/test";

/**
 * ProjectDetail E2E 测试
 *
 * 项目详情现在通过侧边栏项目列表进入。
 * 在 E2E 浏览器环境中，侧边栏项目列表依赖后端 list_projects，
 * 因此仅测试页面结构渲染和导航交互。
 */

test.describe("ProjectDetail", () => {
  test("项目监控域显示项目概览", async ({ page }) => {
    await page.goto("/");

    // 默认显示项目仪表盘
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("localStorage 项目路径保留后项目监控域仍显示概览", async ({ page }) => {
    await page.context().addInitScript(() => {
      localStorage.setItem("agent-scope:current-project", "/tmp/test-project");
    });
    await page.goto("/");

    // 有项目路径但默认 page 是 overview，显示 Dashboard
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("设置与项目监控域可正常切换", async ({ page }) => {
    await page.goto("/");

    // 切换到设置域
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "设置" }).click();
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();

    // 回到项目监控域
    await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "项目监控" }).click();
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });
});
