import { defineConfig } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

// Port is overridable so several checkouts can run the suite side by side.
const PORT = process.env.E2E_PORT || "3030";
const BASE_URL = `http://localhost:${PORT}`;
const REPO_ROOT = path.resolve(__dirname, "..");

// The test database has to be removed *before* the web server boots: the server creates its
// schema with the migrations at startup, so deleting test.db afterwards (as a Playwright
// `globalSetup` would, which runs after `webServer`) leaves an empty file without tables.
// Playwright re-imports this config in every worker process (where TEST_WORKER_INDEX is set);
// deleting the database there would wipe the schema of the already-running server.
if (process.env.TEST_WORKER_INDEX === undefined) {
  for (const f of ["test.db", "test.db-shm", "test.db-wal"]) {
    const p = path.resolve(__dirname, f);
    if (fs.existsSync(p)) fs.rmSync(p);
  }
}

// `cargo leptos build` writes the server binary into the cargo target dir; honour
// CARGO_TARGET_DIR so the suite also works from a worktree with a custom target dir.
const targetDir = process.env.CARGO_TARGET_DIR
  ? path.resolve(REPO_ROOT, process.env.CARGO_TARGET_DIR)
  : path.resolve(REPO_ROOT, "target");
const serverBin = process.env.KARTOTEKA_BIN || path.resolve(targetDir, "debug", "kartoteka");

export default defineConfig({
  testDir: "./e2e",
  use: {
    baseURL: BASE_URL,
    headless: true,
  },
  workers: 1,
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
  webServer: {
    command: serverBin,
    cwd: REPO_ROOT,
    url: BASE_URL,
    timeout: 60_000,
    reuseExistingServer: false,
    stdout: "pipe",
    env: {
      DATABASE_URL: `sqlite:${path.resolve(__dirname, "test.db")}`,
      OAUTH_SIGNING_SECRET: "test-secret-min-32-chars-abcdefgh",
      BIND_ADDR: `127.0.0.1:${PORT}`,
      RUST_LOG: "error",
      LEPTOS_OUTPUT_NAME: "kartoteka",
      LEPTOS_SITE_ROOT: "target/site",
      // The root Cargo.toml sets `hash-files = true`: without this the SSR HTML references
      // unhashed assets, hydration never runs and `[data-hydrated]` never appears.
      LEPTOS_HASH_FILES: "true",
    },
  },
});
