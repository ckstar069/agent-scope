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

  test("项目卡片可点击并进入详情", async ({ page }) => {
    // 注入 mock 项目数据，使 Dashboard 渲染项目卡片
    await page.addInitScript(() => {
      const win = window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
      };

      win.__TAURI_INTERNALS__ = {
        invoke: (command) => {
          if (command === "list_projects") {
            return Promise.resolve([
              { path: "/tmp/mock-project", added_at: Date.now() / 1000 },
            ]);
          }
          if (command === "get_project_data") {
            return Promise.resolve({
              project_path: "/tmp/mock-project",
              stage: { name: "Stage 1", ordinal: 0 },
              stage_error: null,
              config: { project_name: "Mock Project" },
              config_error: null,
              git: {
                branch: "main",
                is_clean: true,
                modified_count: 0,
                staged_count: 0,
                untracked_count: 0,
                conflict_count: 0,
              },
              git_error: null,
              timestamp_ms: Date.now(),
            });
          }
          return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
        },
      };
    });

    await page.goto("/");

    // 等待项目卡片渲染
    await expect(page.getByText("Mock Project")).toBeVisible({ timeout: 8000 });

    // 点击卡片本身（全卡可点击）
    const card = page.locator('[role="button"][aria-label="查看 Mock Project 详情"]');
    await expect(card).toBeVisible();
    await card.click();

    // 应跳转到项目详情页
    await expect(page.getByRole("heading", { name: "项目详情" })).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("/tmp/mock-project")).toBeVisible();
  });
});
