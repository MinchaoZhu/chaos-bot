import { expect, test } from "@playwright/test";
import {
  configureBackend,
  openEventsPane,
  openSkillsPane,
  openSessionsPane,
  sendAndAssertConversation,
} from "./helpers";

test("react shell mobile layout supports pane switching flow", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".workspace.mobile")).toBeVisible();
  await expect(page.locator(".mobile-tabs, .primary-tabs")).toBeVisible();

  await openSessionsPane(page);
  await configureBackend(page);

  const message = `mobile-e2e-${Date.now()}`;
  await sendAndAssertConversation(page, message);

  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText(`[request] ${message}`);

  await openSkillsPane(page);
  await expect(page.locator(".skills-panel")).toBeVisible();
});
