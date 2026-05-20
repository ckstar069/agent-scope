import { test, expect } from "@playwright/test";

/**
 * Project Memory Panel E2E 测试
 *
 * 项目详情现在通过侧边栏项目列表进入，在 E2E 浏览器环境中
 * list_projects 不可用，因此仅测试项目监控域的概览页面渲染。
 * 记忆面板的完整功能需在 Tauri 环境手动验证。
 */

test.describe("Project Memory Panel", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("项目监控域显示仪表盘标题", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
  });

  test("项目监控域显示副标题说明", async ({ page }) => {
    await expect(page.getByText("汇总已注册 FPGA 项目的 Stage、Git 工作区状态和实时 Agent 活跃度。")).toBeVisible();
  });
});
