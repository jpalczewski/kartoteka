import { test, expect } from "@playwright/test";

const PASSWORD = "testpassword123";
const NAME = "Test User";
const BASE_URL = "http://localhost:3030";

function uniqueEmail() {
  return `test+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

test("signup redirects to /login", async ({ page }) => {
  const email = uniqueEmail();

  await page.goto("/signup");
  await page.waitForSelector("[data-hydrated]");
  await page.fill('input[type="text"]', NAME);
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);

  await Promise.all([
    page.waitForURL(`${BASE_URL}/login`),
    page.click('button[type="submit"]'),
  ]);

  expect(page.url()).toBe(`${BASE_URL}/login`);
});

test("login redirects to /", async ({ page, context }) => {
  const email = uniqueEmail();

  // Create account via REST endpoint so we have a known-good user.
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: NAME, email, password: PASSWORD },
  });
  expect(res.ok()).toBeTruthy();

  await context.clearCookies();
  await page.goto("/login");
  await page.waitForSelector("[data-hydrated]");

  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);

  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);

  expect(page.url()).toBe(`${BASE_URL}/`);
});

test("wrong password shows error", async ({ page, context }) => {
  const email = uniqueEmail();

  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: NAME, email, password: PASSWORD },
  });
  expect(res.ok()).toBeTruthy();

  await context.clearCookies();
  await page.goto("/login");
  await page.waitForSelector("[data-hydrated]");

  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', "wrongpassword");
  await page.click('button[type="submit"]');

  await expect(page.locator(".alert-error")).toBeVisible();
  expect(page.url()).toContain("/login");
});
