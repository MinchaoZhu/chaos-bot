import { expect, type Page } from "@playwright/test";

const runtimeUrl = process.env.E2E_REACT_RUNTIME_URL ?? "http://127.0.0.1:1420";

export async function configureBackend(page: Page) {
  await page.locator(".base-url input").fill(runtimeUrl);
  await page.getByRole("button", { name: "Refresh" }).click();
  if ((await page.locator(".session-list li").count()) === 0) {
    await page.getByRole("button", { name: "New" }).click();
  }
  await expect.poll(async () => page.locator(".session-list li").count()).toBeGreaterThan(0);
}

export async function sendAndAssertConversation(page: Page, message: string) {
  await page.locator(".composer input").fill(message);
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".messages .msg.user").last()).toContainText(message);
  await expect(page.locator(".messages .msg.assistant").last()).toContainText(
    `Mock response to: ${message}`,
  );
}

