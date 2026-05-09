import { test, expect } from "@playwright/test";

test.describe("Template Origin", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
  });

  test.describe("Settings 模板路径配置", () => {
    test.beforeEach(async ({ page }) => {
      await page.locator('nav[aria-label="主导航"]').getByRole("button", { name: "设置" }).click();
      await expect(page.getByRole("heading", { name: "设置" })).toBeVisible();
    });

    test("显示模板路径配置卡片", async ({ page }) => {
      await expect(page.getByRole("heading", { name: "模板项目路径" })).toBeVisible();
      await expect(page.getByText("用于区分文件来源（模板 vs 项目特有）")).toBeVisible();
    });

    test("模板路径输入框和保存按钮存在", async ({ page }) => {
      await expect(page.getByLabel("模板路径")).toBeVisible();
      await expect(page.getByRole("button", { name: "保存" }).first()).toBeVisible();
    });

    test("空路径保存显示错误", async ({ page }) => {
      const saveBtn = page.getByRole("button", { name: "保存" }).first();
      await saveBtn.click();
      // 使用 ^保存失败 精确匹配模板路径保存错误，避免匹配项目列表加载错误
      await expect(page.getByText(/^保存失败/)).toBeVisible({ timeout: 5000 });
    });

    test("浏览按钮存在且可点击", async ({ page }) => {
      const browseBtn = page.locator("button[title='浏览目录']").first();
      await expect(browseBtn).toBeVisible();
      await expect(browseBtn).toBeEnabled();
    });
  });

  test.describe("静态记忆面板来源筛选", () => {
    test("来源筛选下拉框在 MemoryFileTree 中渲染", async ({ page }) => {
      await page.goto("/");
      
      const projectCards = page.locator("button").filter({ hasText: "/" });
      const count = await projectCards.count();
      
      if (count > 0) {
        await projectCards.first().click();
        await expect(page.getByRole("tab", { name: "静态记忆" })).toBeVisible();
        await page.getByRole("tab", { name: "静态记忆" }).click();
        
        await expect(page.locator("select#memory-origin-filter")).toBeVisible();
        
        const select = page.locator("select#memory-origin-filter");
        await expect(select.locator("option[value='all']")).toBeVisible();
        await expect(select.locator("option[value='template']")).toBeVisible();
        await expect(select.locator("option[value='project']")).toBeVisible();
      }
    });
  });
});
