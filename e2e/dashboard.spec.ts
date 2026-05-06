import { test, expect } from "@playwright/test";

/**
 * Dashboard E2E 测试
 * 
 * 测试 Dashboard 面板的空状态、错误状态、加载状态渲染。
 * Tauri API 在浏览器环境不可用，invoke 会 reject，
 * 组件应正确展示错误或加载中的 UI。
 */

test.describe("Dashboard", () => {
  test("显示页面标题和副标题", async ({ page }) => {
    await page.goto("/");

    await expect(page.getByRole("heading", { name: "项目仪表盘" })).toBeVisible();
    await expect(page.getByText("汇总已注册 FPGA 项目的 Stage、Git 工作区状态和实时 Agent 活跃度")).toBeVisible();
  });

  test("初始状态显示加载指示器或错误信息", async ({ page }) => {
    await page.goto("/");

    // 等待加载完成：要么显示错误卡片，要么空状态
    // Tauri invoke 在浏览器会立即 reject，Dashboard 显示错误提示
    await expect(
      page.locator("text=项目列表加载失败").or(page.getByText("正在加载项目列表"))
    ).toBeVisible({ timeout: 8000 });
  });

  test("Tauri API 不可用时显示错误横幅", async ({ page }) => {
    await page.goto("/");

    // Tauri invoke 在浏览器环境会 fail → error 状态
    await expect(page.getByText("项目列表加载失败")).toBeVisible({ timeout: 8000 });
  });

  test("无项目时导航到 Settings 的按钮存在", async ({ page }) => {
    await page.goto("/");

    // 即使加载失败，Dashboard header 区域始终渲染
    await expect(page.getByRole("button", { name: "添加项目" }).first()).toBeVisible({ timeout: 5000 });
  });
});
