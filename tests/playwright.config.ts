import { defineConfig } from "@playwright/test";
import path from "node:path";

const BASE_URL = "http://localhost:3030";

export default defineConfig({
  testDir: "./e2e",
  globalSetup: "./global-setup.ts",
  use: {
    baseURL: BASE_URL,
    headless: true,
  },
  workers: 1,
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
  webServer: {
    command: "/Users/erxyi/.cargo/target/debug/kartoteka",
    cwd: "../",
    url: BASE_URL,
    timeout: 60_000,
    reuseExistingServer: false,
    stdout: "pipe",
    env: {
      DATABASE_URL: `sqlite:${path.resolve(__dirname, "test.db")}`,
      OAUTH_SIGNING_SECRET: "test-secret-min-32-chars-abcdefgh",
      BIND_ADDR: "127.0.0.1:3030",
      RUST_LOG: "info",
      LEPTOS_OUTPUT_NAME: "kartoteka",
      LEPTOS_SITE_ROOT: "target/site",
    },
  },
});
