import { test, expect } from "@playwright/test";

/**
 * ProjectDetail E2E 测试
 *
 * ProjectDetail 通过 localStorage 持久化的项目路径加载。
 * 在 E2E 浏览器环境中 Tauri API 不可用，页面会显示错误状态。
 */

test.describe("ProjectDetail", () => {
  test("通过 localStorage 进入项目详情显示页面结构", async ({ page }) => {
    await page.context().addInitScript(() => {
      localStorage.setItem("agent-scope:current-project", "/tmp/test-project");
    });
    await page.goto("/");

    // 页面应渲染项目详情框架（即使数据加载失败）
    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible();
    await expect(page.getByText("/tmp/test-project")).toBeVisible();
  });

  test("返回仪表盘按钮清除项目路径", async ({ page }) => {
    await page.context().addInitScript(() => {
      localStorage.setItem("agent-scope:current-project", "/tmp/test-project");
    });
    await page.goto("/");

    // 点击返回仪表盘
    await page.getByRole("button", { name: "返回仪表盘" }).click();

    // 应返回 Dashboard
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("Settings 和项目详情是独立路由", async ({ page }) => {
    await page.goto("/");
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "设置" }).click();
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();

    // 回到仪表盘（无项目路径）
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "仪表盘" }).click();
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });
});
