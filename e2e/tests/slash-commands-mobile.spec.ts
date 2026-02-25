import { expect, test } from "@playwright/test";
import { configureBackend } from "./helpers";

test("mobile slash commands support pane switching and local command feedback", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".workspace.mobile")).toBeVisible();

  await page.getByRole("button", { name: "sessions" }).click();
  await configureBackend(page);

  await page.getByRole("button", { name: "chat" }).click();
  await page.locator(".composer input").fill("/sessions");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.getByRole("button", { name: "sessions" })).toHaveClass(/active/);

  await page.getByRole("button", { name: "chat" }).click();
  await page.locator(".composer input").fill("/clear");
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".composer input")).toHaveValue("");

  await page.locator(".composer input").fill("/help");
  await page.getByRole("button", { name: "Send" }).click();

  await page.getByRole("button", { name: "events" }).click();
  await expect(page.locator(".event-panel")).toContainText("[command.help]");
  await expect(page.locator(".event-panel")).toContainText("/model - Show current provider/model");
});

