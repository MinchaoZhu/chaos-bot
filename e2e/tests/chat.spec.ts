import fs from "node:fs/promises";
import path from "node:path";
import { expect, test, type Page } from "@playwright/test";

async function openApp(page: Page) {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "chaos-bot" })).toBeVisible();
  await expect
    .poll(async () => page.locator("#sessionList li").count())
    .toBeGreaterThan(0);
}

async function sendMessage(page: Page, content: string) {
  await page.locator("#messageInput").fill(content);
  await page.getByRole("button", { name: "发送" }).click();
}

test("page load + auto session creation", async ({ page }) => {
  await openApp(page);
});

test("create session and receive assistant streaming response", async ({ page }) => {
  await openApp(page);
  const sessionItems = page.locator("#sessionList li");
  const baseline = await sessionItems.count();
  expect(baseline).toBeGreaterThan(0);

  await page.getByRole("button", { name: "新建会话" }).click();
  await expect(sessionItems).toHaveCount(baseline + 1);
  await expect(sessionItems.first()).toHaveClass(/active/);

  const text = "hello from playwright";
  await sendMessage(page, text);

  await expect(page.locator(".message.user").last()).toHaveText(text);
  await expect(page.locator(".message.assistant").last()).toContainText(
    `Mock response to: ${text}`,
  );
});

test("tool call message appears in chat", async ({ page }) => {
  await openApp(page);

  await sendMessage(page, "use_tool: ls");
  await expect(page.locator(".message.tool").last()).toContainText("[tool] ls:");
});

test("switching sessions restores each conversation", async ({ page }) => {
  await openApp(page);

  const sessions = page.locator("#sessionList li");
  const firstSession = sessions.first();
  const firstSessionId = await firstSession.getAttribute("data-id");
  expect(firstSessionId).toBeTruthy();

  await sendMessage(page, "message in session A");
  await expect(page.locator(".message.assistant").last()).toContainText(
    "Mock response to: message in session A",
  );

  const baseline = await sessions.count();
  await page.getByRole("button", { name: "新建会话" }).click();
  await expect(sessions).toHaveCount(baseline + 1);
  await sendMessage(page, "message in session B");
  await expect(page.locator(".message.assistant").last()).toContainText(
    "Mock response to: message in session B",
  );

  await page.locator(`#sessionList li[data-id="${firstSessionId}"]`).click();
  await expect(page.locator(`#sessionList li[data-id="${firstSessionId}"]`)).toHaveClass(
    /active/,
  );
  await expect(page.locator("#messages")).toContainText("message in session A");
  await expect(page.locator("#messages")).not.toContainText("message in session B");
});

test("runtime files are created under workspace", async ({ page }) => {
  await openApp(page);

  const runtimeRoot = process.env.E2E_TMP_DIR;
  expect(runtimeRoot).toBeTruthy();

  const workspace = path.join(runtimeRoot as string, "workspace");
  for (const relativePath of [
    "agent.json",
    ".env.example",
    "MEMORY.md",
    "personality/SOUL.md",
    "personality/IDENTITY.md",
    "personality/USER.md",
    "personality/AGENTS.md",
    "data/sessions",
    "memory",
  ]) {
    await expect
      .poll(async () => {
        try {
          await fs.access(path.join(workspace, relativePath));
          return true;
        } catch {
          return false;
        }
      })
      .toBeTruthy();
  }
});
