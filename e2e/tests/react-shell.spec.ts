import { expect, test, type Page } from "@playwright/test";

const runtimeUrl = process.env.E2E_REACT_RUNTIME_URL ?? "http://127.0.0.1:1420";

async function configureBackend(page: Page) {
  await page.locator(".base-url input").fill(runtimeUrl);
  await page.getByRole("button", { name: "Refresh" }).click();
  if ((await page.locator(".session-list li").count()) === 0) {
    await page.getByRole("button", { name: "New" }).click();
  }
  await expect.poll(async () => page.locator(".session-list li").count()).toBeGreaterThan(0);
}

async function sendAndAssertConversation(page: Page, message: string) {
  await page.locator(".composer input").fill(message);
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".messages .msg.user").last()).toContainText(message);
  await expect(page.locator(".messages .msg.assistant").last()).toContainText(
    `Mock response to: ${message}`,
  );
}

async function assertSkillsTabFlow(page: Page, mode: "desktop" | "mobile") {
  if (mode === "desktop") {
    await page.getByRole("button", { name: "Skills" }).click();
  } else {
    const skillsTab = page.getByRole("button", { name: "skills" });
    await skillsTab.scrollIntoViewIfNeeded();
    await skillsTab.click({ force: true });
  }

  await expect(page.locator(".skills-panel")).toBeVisible();
  // The built-in skill-creator skill should always be present after startup.
  const skillCards = page.locator(".skill-card");
  await expect(skillCards).toHaveCount(1);
  await expect(skillCards.first()).toContainText("skill-creator");
}

async function assertConfigTabFlow(page: Page, mode: "desktop" | "mobile") {
  if (mode === "desktop") {
    await page.getByRole("button", { name: "Config" }).click();
  } else {
    const configTab = page.getByRole("button", { name: "config" });
    await configTab.scrollIntoViewIfNeeded();
    await configTab.click({ force: true });
  }

  await expect(page.getByRole("heading", { name: "Runtime Config" })).toBeVisible();
  await expect(page.getByTestId("config-raw-editor")).toContainText('"llm"');

  await page.getByRole("button", { name: "Apply Config" }).click();
  await expect(page.locator(".config-status")).toContainText("apply ok");

  await page.getByRole("button", { name: "Reset Config" }).click();
  await expect(page.locator(".config-status")).toContainText("reset ok");

  await page.getByRole("button", { name: "Restart Runtime" }).click();
  await expect(page.locator(".config-status")).toContainText("restart ok");
}

test("react shell desktop layout supports full flow", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("desktop"), "desktop project only");

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "chaos-bot multi-platform UI foundation" })).toBeVisible();
  await expect(page.locator(".workspace.desktop")).toBeVisible();

  await configureBackend(page);

  const baseline = await page.locator(".session-list li").count();
  await page.getByRole("button", { name: "New" }).click();
  await expect(page.locator(".session-list li")).toHaveCount(baseline + 1);

  const message = `desktop-e2e-${Date.now()}`;
  await sendAndAssertConversation(page, message);

  await expect(page.locator(".event-panel")).toContainText(`[request] ${message}`);
  await assertConfigTabFlow(page, "desktop");
  await assertSkillsTabFlow(page, "desktop");
});

test("react shell mobile layout supports pane switching flow", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("mobile"), "mobile project only");

  await page.goto("/");
  await expect(page.locator(".workspace.mobile")).toBeVisible();
  await expect(page.locator(".mobile-tabs")).toBeVisible();

  await page.getByRole("button", { name: "sessions" }).click();
  await configureBackend(page);

  await page.getByRole("button", { name: "chat" }).click();
  const message = `mobile-e2e-${Date.now()}`;
  await sendAndAssertConversation(page, message);

  await page.getByRole("button", { name: "events" }).click();
  await expect(page.locator(".event-panel")).toContainText(`[request] ${message}`);

  await assertConfigTabFlow(page, "mobile");
  await assertSkillsTabFlow(page, "mobile");
});
