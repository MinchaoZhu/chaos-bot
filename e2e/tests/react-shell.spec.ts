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
});

test("telegram webhook route supports channel session reuse", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("desktop"), "desktop project only");

  const requestPayload = (text: string, updateId: number) => ({
    update_id: updateId,
    message: {
      message_id: 100 + updateId,
      text,
      chat: { id: 555001 },
      from: { id: 889002 },
    },
  });

  const res1 = await page.request.post(`${runtimeUrl}/api/channels/telegram/webhook`, {
    headers: {
      "content-type": "application/json",
      "x-telegram-bot-api-secret-token": "e2e-telegram-secret",
    },
    data: requestPayload(`e2e-telegram-${Date.now()}`, 1),
  });
  expect(res1.ok()).toBeTruthy();
  const data1 = await res1.json();
  expect(data1.ok).toBeTruthy();
  expect(data1.ignored).toBeFalsy();
  expect(data1.session_id).toBeTruthy();

  const res2 = await page.request.post(`${runtimeUrl}/api/channels/telegram/webhook`, {
    headers: {
      "content-type": "application/json",
      "x-telegram-bot-api-secret-token": "e2e-telegram-secret",
    },
    data: requestPayload(`e2e-telegram-followup-${Date.now()}`, 2),
  });
  expect(res2.ok()).toBeTruthy();
  const data2 = await res2.json();
  expect(data2.ok).toBeTruthy();
  expect(data2.ignored).toBeFalsy();
  expect(data2.session_id).toBe(data1.session_id);
});

test("telegram webhook retries transient connector failures", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("desktop"), "desktop project only");

  const response = await page.request.post(`${runtimeUrl}/api/channels/telegram/webhook`, {
    headers: {
      "content-type": "application/json",
      "x-telegram-bot-api-secret-token": "e2e-telegram-secret",
    },
    data: {
      update_id: 30,
      message: {
        message_id: 130,
        text: "trigger [telegram-retry:2]",
        chat: { id: 555001 },
        from: { id: 889002 },
      },
    },
  });
  expect(response.ok()).toBeTruthy();
  const data = await response.json();
  expect(data.ok).toBeTruthy();
  expect(data.ignored).toBeFalsy();
  expect(data.session_id).toBeTruthy();
});

test("telegram webhook returns 500 on connector outage", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("desktop"), "desktop project only");

  const response = await page.request.post(`${runtimeUrl}/api/channels/telegram/webhook`, {
    headers: {
      "content-type": "application/json",
      "x-telegram-bot-api-secret-token": "e2e-telegram-secret",
    },
    data: {
      update_id: 31,
      message: {
        message_id: 131,
        text: "trigger [telegram-outage]",
        chat: { id: 555001 },
        from: { id: 889002 },
      },
    },
  });
  expect(response.status()).toBe(500);
  const body = await response.json();
  expect(body.code).toBe("internal_error");
});

test("telegram connector config panel applies runtime config", async ({ page }, testInfo) => {
  test.skip(!testInfo.project.name.includes("desktop"), "desktop project only");

  await page.goto("/");
  await configureBackend(page);

  const stateResponse = await page.request.get(`${runtimeUrl}/api/config`);
  expect(stateResponse.ok()).toBeTruthy();
  const state = await stateResponse.json();
  const baselineConfig = JSON.parse(JSON.stringify(state.running));

  try {
    const panel = page.locator(".connector-config");
    await expect(panel.getByRole("heading", { name: "Telegram connector" })).toBeVisible();

    await panel.getByRole("checkbox", { name: "enabled" }).check();
    await panel.getByRole("checkbox", { name: "polling mode" }).check();
    await panel.getByPlaceholder("https://api.telegram.org").fill("mock://telegram");
    await panel.getByPlaceholder("x-telegram-bot-api-secret-token").fill("e2e-telegram-secret");
    await panel.getByPlaceholder("BotFather token").fill("e2e-telegram-bot-token");
    await panel.getByRole("button", { name: "Apply" }).click();

    await expect(panel.locator(".config-notice")).toContainText("config.apply ok");

    const appliedResponse = await page.request.get(`${runtimeUrl}/api/config`);
    expect(appliedResponse.ok()).toBeTruthy();
    const applied = await appliedResponse.json();
    expect(applied.running.channels.telegram.enabled).toBe(true);
    expect(applied.running.channels.telegram.polling).toBe(true);
    expect(applied.running.channels.telegram.api_base_url).toBe("mock://telegram");
    expect(applied.running.channels.telegram.webhook_secret).toBe("e2e-telegram-secret");
    expect(applied.running.secrets.telegram_bot_token).toBe("e2e-telegram-bot-token");
  } finally {
    const restoreResponse = await page.request.post(`${runtimeUrl}/api/config/apply`, {
      headers: { "content-type": "application/json" },
      data: { config: baselineConfig },
    });
    expect(restoreResponse.ok()).toBeTruthy();
  }
});
