import { expect, test } from "@playwright/test";
import {
  configureBackend,
  openConversationPane,
  openEventsPane,
  openSessionsPane,
} from "./helpers";

test("mobile slash commands support pane switching and local command feedback", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".workspace.mobile")).toBeVisible();

  await openSessionsPane(page);
  await configureBackend(page);

  await openConversationPane(page);
  await page.locator(".composer input").fill("/sessions");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByRole("button", { name: /^sessions$/i })).toHaveClass(/active/);

  await openConversationPane(page);
  await page.locator(".composer input").fill("/clear");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".composer input")).toHaveValue("");

  await page.locator(".composer input").fill("/help");
  await page.getByRole("button", { name: "Send" }).click();

  await openEventsPane(page);
  await expect(page.locator(".event-panel")).toContainText("[command.help]");
  await expect(page.locator(".event-panel")).toContainText("/model - Show current provider/model");
});
