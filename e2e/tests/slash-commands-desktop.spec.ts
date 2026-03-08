import { expect, test } from "@playwright/test";
import {
  configureBackend,
  openConversationPane,
  openEventsPane,
  openSessionsPane,
  sendAndAssertConversation,
} from "./helpers";

test("desktop slash commands cover model, new session, and compact flow", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".workspace.desktop")).toBeVisible();
  await configureBackend(page);

  await openConversationPane(page);
  await page.locator(".composer input").fill("/model");
  await page.getByRole("button", { name: "Send" }).click();
  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText("[command.model]");

  await openConversationPane(page);
  await page.locator(".composer input").fill("/models set mock-large");
  await page.getByRole("button", { name: "Send" }).click();
  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText("[command.models] switched model");

  await openSessionsPane(page);
  const baseline = await page.locator(".session-list li").count();
  await openConversationPane(page);
  await page.locator(".composer input").fill("/new");
  await page.getByRole("button", { name: "Send" }).click();
  await openSessionsPane(page);
  await expect(page.locator(".session-list li")).toHaveCount(baseline + 1);
  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText("[command.new] created");

  const message = `compact-seed-${Date.now()}`;
  await sendAndAssertConversation(page, message);

  await openConversationPane(page);
  await page.locator(".composer input").fill("/compact");
  await page.getByRole("button", { name: "Send" }).click();
  await openSessionsPane(page);
  await expect(page.locator(".session-list li")).toHaveCount(baseline + 2);
  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText("[command.compact] migrated");
});
