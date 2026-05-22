import { test, expect, type Page } from "@playwright/test";

/**
 * Load Chain Simulator E2E 测试
 *
 * 测试加载链模拟器页面的导航、渲染、mock 数据展示和错误状态。
 * Playwright 浏览器环境中 Tauri invoke 不可用，mock 测试通过注入 __TAURI_INTERNALS__ 模拟后端返回。
 */

const mockLoadChainResult = {
  cwd: "/Users/test/project",
  host_profile: {
    host_id: "test-host-001",
    hostname: "test-host",
    os: "macos",
    home_dir: "/Users/test",
    claude_config_dir: "/Users/test/.claude",
    user_name: "test",
  },
  startup_chain: [
    {
      order: 1,
      scope: "user",
      asset_type: "user_claude_md",
      native_path: "/Users/test/.claude/CLAUDE.md",
      logical_path: "/Users/test/.claude/CLAUDE.md",
      load_reason: "user instruction",
      line_count: 10,
      byte_size: 256,
      content_preview: "# User CLAUDE.md\n",
      content_truncated: false,
      exists: true,
    },
    {
      order: 2,
      scope: "project",
      asset_type: "project_claude_md",
      native_path: "/Users/test/project/CLAUDE.md",
      logical_path: "/Users/test/project/CLAUDE.md",
      load_reason: "project instruction",
      line_count: 5,
      byte_size: 128,
      content_preview: "# Project CLAUDE.md\n",
      content_truncated: false,
      exists: true,
    },
    {
      order: 3,
      scope: "project",
      asset_type: "project_local_md",
      native_path: "/Users/test/project/CLAUDE.local.md",
      logical_path: "/Users/test/project/CLAUDE.local.md",
      load_reason: "project local instruction",
      line_count: 3,
      byte_size: 64,
      content_preview: null,
      content_truncated: false,
      exists: true,
    },
    {
      order: 4,
      scope: "user",
      asset_type: "global_rule",
      native_path: "/Users/test/.claude/rules/style.md",
      logical_path: "/Users/test/.claude/rules/style.md",
      load_reason: "unconditional rule",
      line_count: 8,
      byte_size: 180,
      content_preview: "---\npaths: [\"src/**/*.rs\"]\n---\n",
      content_truncated: false,
      exists: true,
    },
    {
      order: 5,
      scope: "auto",
      asset_type: "auto_memory_index",
      native_path: "/Users/test/.claude/projects/p1/memory/MEMORY.md",
      logical_path: "/Users/test/.claude/projects/p1/memory/MEMORY.md",
      load_reason: "auto memory",
      line_count: 200,
      byte_size: 8192,
      content_preview: "# Auto Memory\n",
      content_truncated: true,
      exists: true,
    },
  ],
  path_scoped_rules: [
    {
      scope: "user",
      native_path: "/Users/test/.claude/rules/rust.md",
      logical_path: "/Users/test/.claude/rules/rust.md",
      name: "rust-style",
      paths: ["src/**/*.rs", "tests/**/*.rs"],
      exists: true,
    },
    {
      scope: "project",
      native_path: "/Users/test/project/.claude/rules/api.md",
      logical_path: "/Users/test/project/.claude/rules/api.md",
      name: "api-guidelines",
      paths: ["src/api/**"],
      exists: true,
    },
  ],
  excluded_assets: [
    {
      native_path: "/Users/test/.claude/secret.md",
      logical_path: "/Users/test/.claude/secret.md",
      scope: "user",
      excluded_by: "user",
      pattern: "**/secret.md",
    },
  ],
  warnings: [
    {
      level: "warning",
      code: "managed_settings_unreadable",
      message: "managed settings 不可读",
    },
    {
      level: "info",
      code: "auto_memory_not_found",
      message: "未找到 Auto Memory",
    },
  ],
};

async function mockLoadChainInvoke(page: Page) {
  const mockData = JSON.stringify(mockLoadChainResult);
  await page.addInitScript(
    ({ data }) => {
      const result = JSON.parse(data);
      const win = window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
      };

      // 保留已有的 invoke 实现（如果存在）
      const existingInvoke = win.__TAURI_INTERNALS__?.invoke;

      win.__TAURI_INTERNALS__ = {
        invoke: (command, args) => {
          if (command === "simulate_claude_memory_load_chain") {
            return Promise.resolve(result);
          }
          if (existingInvoke) {
            return existingInvoke(command, args);
          }
          return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
        },
      };
    },
    { data: mockData },
  );
}

async function openLoadChainSimulator(page: Page) {
  await page.goto("/");
  await page.locator('nav[aria-label="大域导航"]').getByRole("button", { name: "Claude Code" }).click();
  await page.locator('nav[aria-label="子导航"]').getByRole("button", { name: "加载链模拟器" }).click();
}

test.describe("LoadChainSimulator", () => {
  test("页面标题和描述可见", async ({ page }) => {
    await openLoadChainSimulator(page);

    await expect(page.getByRole("heading", { name: "加载链模拟器" })).toBeVisible();
    await expect(page.getByText("模拟 Claude Code 从指定目录启动时的记忆加载顺序")).toBeVisible();
  });

  test("输入路径和模拟按钮存在", async ({ page }) => {
    await openLoadChainSimulator(page);

    await expect(page.getByPlaceholder("输入目录路径（留空使用当前目录）")).toBeVisible();
    await expect(page.getByRole("button", { name: "模拟加载" })).toBeVisible();
  });

  test("mock 数据下显示 A 区域启动链", async ({ page }) => {
    await mockLoadChainInvoke(page);
    await openLoadChainSimulator(page);

    // 输入路径并点击模拟
    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    // A 区域标题和步骤数
    await expect(page.getByText("A 区域：启动链")).toBeVisible();
    await expect(page.getByText("5 步")).toBeVisible();

    // 验证启动链中的步骤（使用精确匹配避免与 warning 信息冲突）
    await expect(page.getByText("user instruction", { exact: true })).toBeVisible();
    await expect(page.getByText("project instruction", { exact: true })).toBeVisible();
    await expect(page.getByText("auto memory", { exact: true })).toBeVisible();

    // 验证 scope 标签
    await expect(page.getByText("user", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("project", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("auto", { exact: true }).first()).toBeVisible();
  });

  test("mock 数据下显示 B 区域路径作用域规则", async ({ page }) => {
    await mockLoadChainInvoke(page);
    await openLoadChainSimulator(page);

    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    await expect(page.getByText("B 区域：路径作用域规则")).toBeVisible();
    await expect(page.getByText("2 条")).toBeVisible();

    // 规则名称和 scope
    await expect(page.getByText("rust-style")).toBeVisible();
    await expect(page.getByText("api-guidelines")).toBeVisible();
  });

  test("mock 数据下显示被排除资产", async ({ page }) => {
    await mockLoadChainInvoke(page);
    await openLoadChainSimulator(page);

    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    await expect(page.getByText("被排除资产")).toBeVisible();
    await expect(page.getByText("1 项")).toBeVisible();

    // 排除路径和 pattern（使用精确选择器避免路径和 pattern 文本冲突）
    await expect(page.getByText("/Users/test/.claude/secret.md")).toBeVisible();
    await expect(page.locator("code").getByText("**/secret.md")).toBeVisible();
  });

  test("mock 数据下显示 warnings", async ({ page }) => {
    await mockLoadChainInvoke(page);
    await openLoadChainSimulator(page);

    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    // warning 和 info 都显示
    await expect(page.getByText("[managed_settings_unreadable]")).toBeVisible();
    await expect(page.getByText("managed settings 不可读")).toBeVisible();
    await expect(page.getByText("[auto_memory_not_found]")).toBeVisible();
  });

  test("Tauri 不可用时显示错误", async ({ page }) => {
    await openLoadChainSimulator(page);

    // 不注入 mock，直接点击模拟
    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    // 应显示错误状态
    await expect(
      page.locator(".border-destructive\\/30.bg-destructive\\/5").first(),
    ).toBeVisible({ timeout: 10000 });
  });

  test("失败后清除旧结果", async ({ page }) => {
    // 第一步：注入成功 mock，模拟一次成功请求
    await mockLoadChainInvoke(page);
    await openLoadChainSimulator(page);

    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("/Users/test/project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    // 确认结果已显示
    await expect(page.getByText("A 区域：启动链")).toBeVisible();
    await expect(page.getByText("5 步")).toBeVisible();

    // 第二步：覆盖为失败 mock（对已加载页面用 evaluate）
    await page.evaluate(() => {
      const win = window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
      };
      win.__TAURI_INTERNALS__ = {
        invoke: (command: string) => {
          if (command === "simulate_claude_memory_load_chain") {
            return Promise.reject(new Error("模拟加载失败：目录不存在"));
          }
          return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
        },
      };
    });

    // 再次点击模拟
    await page.getByRole("button", { name: "模拟加载" }).click();

    // 旧结果应被清除，错误应显示
    await expect(page.getByText("A 区域：启动链")).not.toBeVisible();
    await expect(page.locator(".border-destructive\\/30.bg-destructive\\/5").first()).toBeVisible({
      timeout: 10000,
    });
    await expect(page.getByText("模拟加载失败：目录不存在")).toBeVisible();
  });

  test("~ 路径输入发送到后端", async ({ page }) => {
    // 使用动态 mock 捕获前端发送的 cwd 参数
    await page.addInitScript(() => {
      const win = window as unknown as {
        __TAURI_INTERNALS__: {
          invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
        };
      };
      win.__TAURI_INTERNALS__ = {
        invoke: (command, args) => {
          if (command === "simulate_claude_memory_load_chain") {
            // 验证前端发送的 cwd 参数包含 ~（由后端展开）
            const cwd = (args as { cwd?: string })?.cwd;
            if (cwd && cwd.startsWith("~")) {
              return Promise.resolve({
                cwd: "/expanded/home/path",
                host_profile: {
                  host_id: "test",
                  hostname: "test",
                  os: "macos",
                  home_dir: "/Users/test",
                  claude_config_dir: "/Users/test/.claude",
                  user_name: "test",
                },
                startup_chain: [],
                path_scoped_rules: [],
                excluded_assets: [],
                warnings: [],
              });
            }
            return Promise.reject(new Error("cwd 不以 ~ 开头"));
          }
          return Promise.reject(new Error(`未模拟的 Tauri 命令: ${command}`));
        },
      };
    });

    await openLoadChainSimulator(page);

    await page.getByPlaceholder("输入目录路径（留空使用当前目录）").fill("~/my-project");
    await page.getByRole("button", { name: "模拟加载" }).click();

    // 应显示成功结果（startup_chain 为空但页面正常渲染）
    await expect(page.getByText("CWD: /expanded/home/path")).toBeVisible({ timeout: 10000 });
    await expect(page.getByText("无启动链步骤")).toBeVisible();
  });
});
