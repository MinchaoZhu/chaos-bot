import { defineConfig, devices } from "@playwright/test";
import { fileURLToPath } from "node:url";
import path from "node:path";

const backendPort = Number(process.env.E2E_PORT ?? 3010);
const backendBaseURL = process.env.E2E_BASE_URL ?? `http://127.0.0.1:${backendPort}`;
const shellBaseURL = process.env.E2E_SHELL_BASE_URL ?? "http://127.0.0.1:1420";
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
  workers: 1,
  reporter: [["list"], ["html", { open: "never", outputFolder: reportDir }]],
  use: {
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "react-shell-desktop",
      testMatch: /react-shell\.spec\.ts/,
      use: {
        ...devices["Desktop Chrome"],
        baseURL: shellBaseURL,
        viewport: { width: 1440, height: 900 },
      },
    },
    {
      name: "react-shell-mobile",
      testMatch: /react-shell\.spec\.ts/,
      use: {
        ...devices["Pixel 7"],
        baseURL: shellBaseURL,
      },
    },
  ],
  webServer: [
    {
      command: "bash e2e/run-with-agent-config.sh",
      cwd: "..",
      url: `${backendBaseURL}/api/health`,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
    {
      command: `VITE_BACKEND_PROXY_TARGET=${backendBaseURL} npm --prefix frontend-react run dev`,
      cwd: "..",
      url: shellBaseURL,
      reuseExistingServer: !process.env.CI,
      timeout: 120_000,
    },
  ],
});
