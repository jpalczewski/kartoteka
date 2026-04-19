import { test, expect } from "@playwright/test";

const BASE_URL = "http://localhost:3000";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+tags+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: any, context: any) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "Tags User", email, password: PASSWORD },
  });
  expect(res.ok()).toBeTruthy();
  await page.goto("/login");
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);
  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);
}

test("navbar has Tags link that navigates to /tags", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/");
  const link = page.locator('[data-testid="nav-tags"]');
  await expect(link).toBeVisible();
  await link.click();
  await expect(page).toHaveURL(`${BASE_URL}/tags`);
  await expect(page.locator('h2:has-text("Tagi")')).toBeVisible();
});

test("tags page shows empty state when no tags exist", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("create tag via button shows it in the list", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Praca");
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: "Praca" })).toBeVisible();
});

test("create tag via Enter key shows it in the list", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Dom");
  await page.locator('[data-testid="new-tag-input"]').press("Enter");
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: "Dom" })).toBeVisible();
});

test("create tag clears input after submit", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Hobby");
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: "Hobby" })).toBeVisible();
  await expect(page.locator('[data-testid="new-tag-input"]')).toHaveValue("");
});

test("create tag with empty name does nothing", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("delete tag removes it from the list", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("DoUsunięcia");
  await page.locator('[data-testid="create-tag-btn"]').click();
  const row = page.locator('[data-testid="tag-item"]').filter({ hasText: "DoUsunięcia" });
  await expect(row).toBeVisible();
  await row.locator('[data-testid="delete-tag-btn"]').click();
  await expect(row).not.toBeVisible();
});

test("after deleting last tag empty state appears", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Jedyny");
  await page.locator('[data-testid="create-tag-btn"]').click();
  const row = page.locator('[data-testid="tag-item"]').filter({ hasText: "Jedyny" });
  await expect(row).toBeVisible();
  await row.locator('[data-testid="delete-tag-btn"]').click();
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("click tag navigates to detail page showing tag name", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Podróże");
  await page.locator('[data-testid="create-tag-btn"]').click();
  const row = page.locator('[data-testid="tag-item"]').filter({ hasText: "Podróże" });
  await expect(row).toBeVisible();
  await row.locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);
  await expect(page.locator('[data-testid="tag-detail-name"]')).toContainText("Podróże");
});

test("tag detail page shows empty linked lists message when no lists linked", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags");
  await page.locator('[data-testid="new-tag-input"]').fill("Samotny");
  await page.locator('[data-testid="create-tag-btn"]').click();
  const row = page.locator('[data-testid="tag-item"]').filter({ hasText: "Samotny" });
  await expect(row).toBeVisible();
  await row.locator('[data-testid="tag-link"]').click();
  await expect(page.locator('[data-testid="tag-no-lists"]')).toBeVisible();
});

test("tag detail page with unknown id shows error", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/tags/00000000-0000-0000-0000-000000000000");
  await expect(page.locator('[data-testid="tag-error"]')).toBeVisible();
});
