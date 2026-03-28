import { test, expect } from "@playwright/test";

const PASSWORD = "testpassword123";
const NAME = "Test User";

function uniqueEmail() {
  return `test+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

test("signup redirects to /", async ({ page }) => {
  const email = uniqueEmail();

  await page.goto("/signup");
  await page.fill('input[type="text"]', NAME);
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);

  await Promise.all([
    page.waitForURL("http://localhost:8080/"),
    page.click('button[type="submit"]'),
  ]);

  expect(page.url()).toBe("http://localhost:8080/");
});

test("login redirects to /", async ({ page, context }) => {
  const email = uniqueEmail();

  // Create account via direct API call so we have a known-good user
  await context.request.post("http://localhost:8788/auth/api/sign-up/email", {
    headers: { "Content-Type": "application/json", "Origin": "http://localhost:8080" },
    data: { name: NAME, email, password: PASSWORD },
  });

  // Clear any session cookies, then login via UI
  await context.clearCookies();
  await page.goto("/login");

  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);

  await Promise.all([
    page.waitForURL("http://localhost:8080/"),
    page.click('button[type="submit"]'),
  ]);

  expect(page.url()).toBe("http://localhost:8080/");
});

test("wrong password shows error", async ({ page, context }) => {
  const email = uniqueEmail();

  await context.request.post("http://localhost:8788/auth/api/sign-up/email", {
    headers: { "Content-Type": "application/json", "Origin": "http://localhost:8080" },
    data: { name: NAME, email, password: PASSWORD },
  });

  await context.clearCookies();
  await page.goto("/login");

  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', "wrongpassword");
  await page.click('button[type="submit"]');

  await expect(page.locator(".alert-error")).toBeVisible();
  expect(page.url()).toContain("/login");
});
