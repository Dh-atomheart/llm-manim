import { expect, test } from "@playwright/test";

import { installTauriMock } from "./support/tauriMock";

async function gotoApp(
  page: Parameters<typeof installTauriMock>[0],
  options?: Parameters<typeof installTauriMock>[1],
) {
  await installTauriMock(page, options);
  await page.goto("/");
}

async function completeFirstLaunch(
  page: Parameters<typeof installTauriMock>[0],
) {
  await expect(
    page.getByRole("heading", { name: "初始化工作区" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "浏览" }).click();
  await expect(page.getByLabel("工作区目录")).toHaveValue("F:/mock-workspace");
  await page.getByRole("button", { name: "检查环境" }).click();
  await expect(
    page.getByText("运行环境可用，可在当前工作区启动 Manim 渲染").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "继续" }).click();
  await expect(page.getByRole("button", { name: "新建项目" })).toBeVisible();
}

async function addProvider(
  page: Parameters<typeof installTauriMock>[0],
  apiKey: string,
) {
  await page.getByRole("button", { name: "Provider 设置" }).click();
  await page.getByRole("button", { name: "添加" }).click();
  await page
    .locator('input[placeholder="DeepSeek / Anthropic / OpenAI"]')
    .fill("Mock Provider");
  await page.locator('input[type="url"]').fill("https://api.example.com");
  await page
    .locator('input[placeholder="deepseek-v3 / gpt-4o"]')
    .fill("mock-model-v1");
  await page.locator('input[aria-label="API Key"]').fill(apiKey);
  await page.getByRole("checkbox").check();
}

async function createProject(
  page: Parameters<typeof installTauriMock>[0],
  name: string,
) {
  await page.getByRole("button", { name: "新建项目" }).click();
  await page.getByPlaceholder("输入项目名称").fill(name);
  await page.getByRole("button", { name: "保存" }).click();
  await expect(page.getByText(`已创建项目「${name}」`)).toBeVisible();
}

test("首次启动到视频预览的主流程可用", async ({ page }) => {
  await gotoApp(page);
  await completeFirstLaunch(page);

  await addProvider(page, "sk-live-e2e-success");
  await page.getByRole("button", { name: "测试连接" }).click();
  await expect(page.getByText(/连接测试成功/)).toBeVisible();
  await page.getByRole("button", { name: "添加 Provider" }).click();
  await expect(page.getByText("Provider 已添加。")).toBeVisible();
  await expect(page.locator("body")).not.toContainText("sk-live-e2e-success");

  await createProject(page, "黄金样例项目");
  await page.locator("textarea").fill("公式推导：二次方程求根公式");
  await page.getByRole("button", { name: "提交任务" }).click();

  await expect(page.getByText("任务已提交，当前状态：排队中")).toBeVisible();
  await expect(page.getByText("运行中").first()).toBeVisible();
  await expect(page.getByText("已完成").first()).toBeVisible();
  await expect(page.locator("video")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "在文件管理器中打开" }),
  ).toBeVisible();
});

test("Provider 鉴权失败会显示可诊断错误且不泄露 API Key", async ({ page }) => {
  await gotoApp(page, { workspaceConfigured: true });

  await addProvider(page, "sk-bad-secret-key");
  await page.getByRole("button", { name: "测试连接" }).click();

  await expect(
    page.getByText("Provider 鉴权失败，请检查 API Key"),
  ).toBeVisible();
  await expect(page.locator("body")).not.toContainText("sk-bad-secret-key");
});

test("静态校验失败会显示错误码与建议动作", async ({ page }) => {
  await gotoApp(page, {
    workspaceConfigured: true,
    seedProviders: [{ name: "Mock Provider", apiKey: "sk-seeded-provider" }],
    seedProjects: [{ name: "失败场景项目" }],
  });

  await page
    .locator("textarea")
    .fill("[static-fail] 危险 API 静态校验失败用例");
  await page.getByRole("button", { name: "提交任务" }).click();

  await expect(page.getByText("E_STATIC_CHECK_FAILED").first()).toBeVisible();
  await expect(page.getByText("生成代码调用了受限能力")).toBeVisible();
  await expect(
    page.getByText(
      "请改写提示词，要求只使用 Manim Community Edition 并避免文件、网络或命令调用。",
    ),
  ).toBeVisible();
});

test("running 任务可以取消并手动重试到成功", async ({ page }) => {
  await gotoApp(page, {
    workspaceConfigured: true,
    seedProviders: [{ name: "Mock Provider", apiKey: "sk-seeded-provider" }],
    seedProjects: [{ name: "取消重试项目" }],
  });

  await page.locator("textarea").fill("[cancel] 长任务取消与重试");
  await page.getByRole("button", { name: "提交任务" }).click();

  await expect(page.getByText("运行中").first()).toBeVisible();
  await page.getByRole("button", { name: "取消", exact: true }).click();
  await expect(page.getByText("已取消").first()).toBeVisible();
  await expect(page.getByText("任务已取消")).toBeVisible();

  await page.getByRole("button", { name: "重试", exact: true }).click();
  await expect(page.getByText("已创建重试任务并重新加入队列。")).toBeVisible();
  await expect(page.getByText("已完成").first()).toBeVisible();
  await expect(page.locator("video")).toBeVisible();
});
