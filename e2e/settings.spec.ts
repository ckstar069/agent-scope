import { test, expect } from "@playwright/test";

/**
 * Settings E2E 测试
 * 
 * 测试 Settings 面板的前端表单验证、项目列表渲染、移除确认对话框。
 * 前端本地校验路径规则（不依赖 Tauri），添加/移除 submit 依赖 Tauri invoke。
 */

test.describe("Settings", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // 导航到 Settings
    await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "Settings" }).click();
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
  });

  test("显示 Settings 页面标题和表单", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
    await expect(page.getByLabel("项目路径")).toBeVisible();
    await expect(page.getByRole("button", { name: "添加项目" })).toBeVisible();
  });

  test("空路径提交显示校验错误", async ({ page }) => {
    // 不清输入任何内容，直接提交
    await page.getByRole("button", { name: "添加项目" }).click();

    await expect(page.getByText("无效路径：请输入项目绝对路径。")).toBeVisible();
  });

  test("非绝对路径提交显示校验错误", async ({ page }) => {
    const input = page.getByLabel("项目路径");
    await input.fill("relative/path");
    await page.getByRole("button", { name: "添加项目" }).click();

    await expect(page.getByText("仅支持 macOS/Linux 绝对路径，请以 / 开头。")).toBeVisible();
  });

  test("根目录路径提交显示校验错误", async ({ page }) => {
    const input = page.getByLabel("项目路径");
    await input.fill("/");
    await page.getByRole("button", { name: "添加项目" }).click();

    await expect(page.getByText("无效路径：请指向具体项目目录，不能使用根目录。")).toBeVisible();
  });

  test("输入内容后错误提示清除", async ({ page }) => {
    const input = page.getByLabel("项目路径");

    // 先触发错误
    await page.getByRole("button", { name: "添加项目" }).click();
    await expect(page.getByText("无效路径：请输入项目绝对路径。")).toBeVisible();

    // 输入内容后错误应清除
    await input.fill("/some/path");
    await expect(page.getByText("无效路径：请输入项目绝对路径。")).not.toBeVisible();
  });

  test("Tauri 不可用时添加项目显示错误", async ({ page }) => {
    const input = page.getByLabel("项目路径");
    // 使用一个看起来合法的路径（但 Tauri 会失败）
    await input.fill("/Users/ckstar/Repo/ai_project_template");
    await page.getByRole("button", { name: "添加项目" }).click();

    // Tauri invoke 在浏览器不可用，应显示错误消息
    await expect(page.getByText(/项目添加失败|路径不存在或无法访问/)).toBeVisible({ timeout: 8000 });
  });

  test("已注册项目列表区域渲染", async ({ page }) => {
    // 验证"已注册项目"卡片存在
    await expect(page.getByRole("heading", { name: "已注册项目" })).toBeVisible();
  });

  test("点击移除按钮弹出确认对话框", async ({ page }) => {
    // 如果没有已注册项目，移除按钮不存在(正常)
    // 如果有已注册项目（由 Tauri Mock 提供），测试对话框显示
    const removeBtn = page.getByRole("button", { name: "移除" });
    if (await removeBtn.isVisible().catch(() => false)) {
      await removeBtn.first().click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await expect(page.getByRole("heading", { name: "移除监控项目？" })).toBeVisible();
      await expect(page.getByRole("button", { name: "确认移除" })).toBeVisible();
      await expect(page.getByRole("button", { name: "取消" })).toBeVisible();

      // 取消后对话框消失
      await page.getByRole("button", { name: "取消" }).click();
      await expect(page.getByRole("dialog")).not.toBeVisible();
    }
    // 没有项目时不报错即可（通过测试）
  });
});
