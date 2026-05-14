import { defineConfig, devices } from "@playwright/test";

/**
 * AgentScope E2E 测试配置
 * 
 * 使用 Vite dev server 启动前端（非 Tauri），测试纯前端 UI 逻辑。
 * Tauri invoke/listen 在浏览器环境会失败，测试覆盖各面板的错误/空状态渲染。
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : 1,
  reporter: [["html", { outputFolder: ".sisyphus/evidence/e2e/report" }], ["list"]],
  timeout: 30000,

  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],

  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 60000,
  },

  snapshotDir: ".sisyphus/evidence/e2e/snapshots",
});
