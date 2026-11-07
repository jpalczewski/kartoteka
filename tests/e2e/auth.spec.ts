import { test, expect } from "@playwright/test";

import {
  BASE_URL,
  PASSWORD,
  login,
  registerAccount,
  uniqueEmail,
  waitForHydration,
} from "./helpers";

const NAME = "Test User";

// Registration auto-logs the new user in and redirects to the home page.
test("signup logs the new user in and redirects to /", async ({ page }) => {
  const email = uniqueEmail("signup");

  await page.goto("/signup");
  await waitForHydration(page);
  await page.fill('input[type="text"]', NAME);
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);

  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);

  expect(page.url()).toBe(`${BASE_URL}/`);
  await waitForHydration(page);
  await expect(page.locator('[data-testid="nav-tags"]')).toBeVisible();
});

test("login redirects to /", async ({ page, context }) => {
  // Create the account via REST so we have a known-good user.
  const email = await registerAccount(context, NAME);

  await context.clearCookies();
  await login(page, email);

  expect(page.url()).toBe(`${BASE_URL}/`);
});

test("wrong password shows error", async ({ page, context }) => {
  const email = await registerAccount(context, NAME);

  await context.clearCookies();
  await page.goto("/login");
  await waitForHydration(page);

  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', "wrongpassword");
  await page.click('button[type="submit"]');

  await expect(page.locator(".alert-error")).toBeVisible();
  expect(page.url()).toContain("/login");
});
