// Playwright (T8.3, T8.4, T8.5): Chromium against the local dev server.
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  testMatch: /.*\.spec\.ts/,
  timeout: 180_000,
  workers: 1,
  reporter: [["list"]],
  use: { baseURL: "http://127.0.0.1:5173", ...devices["Desktop Chrome"], viewport: { width: 1280, height: 1000 } },
  webServer: {
    command: "npm run dev",
    url: "http://127.0.0.1:5173",
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
