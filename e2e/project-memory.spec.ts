import { test, expect } from "@playwright/test";

test.describe("Project Memory Panel", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Projects" }).click();
  });

  test("项目记忆 Tab 存在", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目记忆" })).toBeVisible();
  });

  test("L1 静态记忆 Tab 默认选中", async ({ page }) => {
    const l1Tab = page.getByRole("button", { name: "静态记忆" });
    await expect(l1Tab).toBeVisible();
    await expect(l1Tab).toHaveClass(/bg-primary/);
  });

  test("L2 对话搜索 Tab 可切换", async ({ page }) => {
    await page.getByRole("button", { name: "对话搜索" }).click();
    await expect(page.getByPlaceholder("搜索对话内容...")).toBeVisible();
  });

  test("L3 候选记忆 Tab 显示空状态", async ({ page }) => {
    await page.getByRole("button", { name: "候选记忆" }).click();
    await expect(page.getByText("暂无候选记忆")).toBeVisible();
  });

  test("Tab 切换保留状态", async ({ page }) => {
    await page.getByRole("button", { name: "对话搜索" }).click();
    await expect(page.getByPlaceholder("搜索对话内容...")).toBeVisible();

    await page.getByRole("button", { name: "静态记忆" }).click();
    await expect(page.getByText("此项目未找到记忆文件")).toBeVisible();

    await page.getByRole("button", { name: "对话搜索" }).click();
    await expect(page.getByPlaceholder("搜索对话内容...")).toBeVisible();
  });

  test("项目记忆 Panel 包含正确的子标题", async ({ page }) => {
    const panel = page.locator("article").filter({ hasText: "项目记忆" });
    await expect(panel.getByText("CLAUDE.md、规则、笔记、设计文档")).toBeVisible();
  });
});
