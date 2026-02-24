import { defineConfig } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "node:path";

const port = Number(process.env.E2E_PORT ?? 3010);
const baseURL = process.env.E2E_BASE_URL ?? `http://127.0.0.1:${port}`;
const currentDir = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(currentDir, "..");
const artifactsDir =
  process.env.E2E_ARTIFACTS_DIR ?? path.join(rootDir, ".tmp", "e2e", "artifacts");
const outputDir = path.join(artifactsDir, "test-results");
const reportDir = path.join(artifactsDir, "playwright-report");

export default defineConfig({
  testDir: "./tests",
  outputDir,
  timeout: 30_000,
  expect: {
    timeout: 10_000,
  },
  fullyParallel: false,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never", outputFolder: reportDir }]],
  use: {
    baseURL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  webServer: {
    command: "bash e2e/run-with-agent-config.sh",
    cwd: "..",
    url: `${baseURL}/api/health`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
