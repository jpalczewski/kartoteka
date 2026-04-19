import { defineConfig } from "@playwright/test";

const BASE_URL = "http://localhost:3000";

export default defineConfig({
  testDir: "./e2e",
  globalSetup: "./global-setup.ts",
  use: {
    baseURL: BASE_URL,
    headless: true,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
  webServer: {
    command: "cargo run -p kartoteka-server",
    cwd: "../",
    url: BASE_URL,
    timeout: 120_000,
    reuseExistingServer: !process.env.CI,
    env: {
      DATABASE_URL: "sqlite://tests/test.db",
      OAUTH_SIGNING_SECRET: "test-secret-min-32-chars-abcdefgh",
      BIND_ADDR: "127.0.0.1:3000",
      RUST_LOG: "warn",
    },
  },
});
