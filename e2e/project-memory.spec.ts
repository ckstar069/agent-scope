import { test, expect } from "@playwright/test";

/**
 * Project Memory Panel E2E 测试
 *
 * 通过 localStorage 设置项目路径进入 ProjectDetail。
 * 在 E2E 浏览器环境中 Tauri API 不可用，数据加载会失败，
 * 但项目记忆面板仍应尝试渲染。
 */

test.describe("Project Memory Panel", () => {
  test.beforeEach(async ({ page }) => {
    await page.context().addInitScript(() => {
      localStorage.setItem("agent-scope:current-project", "/tmp/test-project");
    });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible();
  });

  test("项目记忆 Panel 标题存在", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目记忆" })).toBeVisible();
  });

  test("项目记忆 Panel 包含子标题", async ({ page }) => {
    await expect(page.getByText("CLAUDE.md、规则、笔记、设计文档")).toBeVisible();
  });
});
