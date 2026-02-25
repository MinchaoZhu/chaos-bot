import { expect, test } from "@playwright/test";
import { configureBackend, sendAndAssertConversation } from "./helpers";

test("react shell mobile layout supports pane switching flow", async ({ page }) => {
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
});

