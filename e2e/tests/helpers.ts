import { expect, type Page } from "@playwright/test";

const runtimeUrl = process.env.E2E_REACT_RUNTIME_URL ?? "http://127.0.0.1:1420";

async function clickFirstVisibleButton(page: Page, names: RegExp[]) {
  for (const name of names) {
    const button = page.getByRole("button", { name }).first();
    if (await button.isVisible().catch(() => false)) {
      await button.scrollIntoViewIfNeeded();
      await button.click({ force: true });
      return true;
    }
  }
  return false;
}

export async function openConversationPane(page: Page) {
  await clickFirstVisibleButton(page, [/^Conversation$/i, /^Chat$/i]);
}

export async function openSessionsPane(page: Page) {
  await clickFirstVisibleButton(page, [/^Sessions$/i]);
}

export async function openEventsPane(page: Page) {
  await clickFirstVisibleButton(page, [/^Events$/i]);
}

export async function openConfigPane(page: Page) {
  await clickFirstVisibleButton(page, [/^Config$/i]);
}

export async function openSkillsPane(page: Page) {
  await clickFirstVisibleButton(page, [/^Skills$/i]);
}

export async function configureBackend(page: Page) {
  let backendUrlInput = page.locator(".base-url input").first();
  if (!(await backendUrlInput.isVisible().catch(() => false))) {
    await openConfigPane(page);
    backendUrlInput = page.locator(".base-url input").first();
  }
  if (!(await backendUrlInput.isVisible().catch(() => false))) {
    await openSessionsPane(page);
    backendUrlInput = page.locator(".base-url input").first();
  }

  await expect(backendUrlInput).toBeVisible();
  await backendUrlInput.fill(runtimeUrl);

  const refreshButton = page.getByRole("button", { name: /^Refresh$/i }).first();
  if (await refreshButton.isVisible().catch(() => false)) {
    await refreshButton.click();
  }

  await expect
    .poll(async () => {
      await openSessionsPane(page);
      const sessionCount = await page.locator(".session-list li").count();
      if (sessionCount > 0) {
        return sessionCount;
      }

      const newButton = page.getByRole("button", { name: /^New$/i }).first();
      if (await newButton.isVisible().catch(() => false)) {
        await newButton.click({ force: true });
      }

      return page.locator(".session-list li").count();
    })
    .toBeGreaterThan(0);
}

export async function sendAndAssertConversation(page: Page, message: string) {
  await openConversationPane(page);
  await page.locator(".composer input").fill(message);
  await page.getByRole("button", { name: "Send" }).click();
  await expect(page.locator(".messages .msg.user").last()).toContainText(message);
  await expect(page.locator(".messages .msg.assistant").last()).toContainText(
    `Mock response to: ${message}`,
  );
}
