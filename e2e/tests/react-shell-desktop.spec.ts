import { expect, test } from "@playwright/test";
import { configureBackend, sendAndAssertConversation } from "./helpers";

test("react shell desktop layout supports full flow", async ({ page }) => {
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
});

