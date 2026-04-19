import { test, expect } from "@playwright/test";

const BASE_URL = "http://localhost:3000";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+cf+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: any, context: any) {
  const email = uniqueEmail();
  await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "CF User", email, password: PASSWORD },
  });
  await page.goto("/login");
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);
  await Promise.all([page.waitForURL(`${BASE_URL}/`), page.click('button[type="submit"]')]);
}

async function createListAndGetId(page: any): Promise<string> {
  await page.goto("/");
  const input = page.locator('input[placeholder="Nazwa listy..."]');
  await input.waitFor();
  await input.fill("Filter Test List");
  await page.locator('button:has-text("Utwórz")').first().click();
  const link = page.locator('a[href^="/lists/"]').first();
  await link.waitFor({ timeout: 5000 });
  return (await link.getAttribute("href"))!.replace("/lists/", "");
}

test("completed items are visible by default", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndGetId(page);
  await page.goto(`/lists/${listId}`);

  await page.fill('input[placeholder="Nowy element..."]', "Task A");
  await page.click('button:has-text("Dodaj")');

  // Complete the item — wait for it to appear first
  const itemCheckbox = page.locator('input[type="checkbox"]').first();
  await itemCheckbox.waitFor();
  await itemCheckbox.click();

  // Item still visible and count reflects completion
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1 ukończone");
  await expect(page.locator('text=Task A')).toBeVisible();
});

test("hiding completed removes them from view but count stays", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndGetId(page);
  await page.goto(`/lists/${listId}`);

  // Add two items
  await page.fill('input[placeholder="Nowy element..."]', "Done Item");
  await page.click('button:has-text("Dodaj")');
  await page.fill('input[placeholder="Nowy element..."]', "Pending Item");
  await page.click('button:has-text("Dodaj")');

  // Complete the first one
  await page.locator('input[type="checkbox"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/2");

  // Toggle hide completed
  await page.locator('[data-testid="hide-completed-toggle"]').click();

  // Completed item hidden, pending still visible, count unchanged
  await expect(page.locator('text=Done Item')).not.toBeVisible();
  await expect(page.locator('text=Pending Item')).toBeVisible();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/2");
});

test("all-completed + filter shows helpful message", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndGetId(page);
  await page.goto(`/lists/${listId}`);

  await page.fill('input[placeholder="Nowy element..."]', "Only Item");
  await page.click('button:has-text("Dodaj")');

  // Complete it
  await page.locator('input[type="checkbox"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Hide completed → all gone → helpful message appears
  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.locator('text=Wszystkie elementy ukończone')).toBeVisible();
});
