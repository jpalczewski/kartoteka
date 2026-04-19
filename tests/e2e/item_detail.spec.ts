import { test, expect, Page, BrowserContext } from "@playwright/test";

const BASE_URL = "http://localhost:3030";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+item+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: Page, context: BrowserContext) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "Item User", email, password: PASSWORD },
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
  await page.waitForSelector("[data-hydrated]");
}

async function createListWithItem(
  page: Page,
  listName: string,
  itemTitle: string
): Promise<{ listId: string }> {
  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  await page.fill('input[placeholder="Nazwa listy..."]', listName);
  await page.locator('button:has-text("Utwórz")').first().click();
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: listName });
  await card.waitFor({ timeout: 5000 });
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);
  const listId = page.url().split("/lists/")[1];

  await page.fill('input[placeholder="Nowy element..."]', itemTitle);
  await page.click('button:has-text("Dodaj")');
  await page.getByText(itemTitle, { exact: true }).waitFor();

  return { listId };
}

// ── Navigation ────────────────────────────────────────────────────────────────

test("clicking item title navigates to item detail page", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Nav List", "Click Me");

  await page.goto(`/lists/${listId}`);
  await page.locator('text=Click Me').click();

  await expect(page).toHaveURL(/\/lists\/.+\/items\/.+/);
  await expect(page.locator('[data-testid="item-detail-title"]')).toHaveValue("Click Me");
});

test("back link returns to parent list", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Back List", "Back Item");

  await page.goto(`/lists/${listId}`);
  await page.locator('text=Back Item').click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('a:has-text("← Back to list")').click();
  await expect(page).toHaveURL(`${BASE_URL}/lists/${listId}`);
});

// ── Edit title ────────────────────────────────────────────────────────────────

test("edit title and save updates the item", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Edit List", "Original Title");

  await page.goto(`/lists/${listId}`);
  await page.locator('text=Original Title').click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('[data-testid="item-detail-title"]').fill("Updated Title");
  await page.locator('[data-testid="item-detail-save"]').click();

  // Reload to confirm server persisted
  await page.reload();
  await expect(page.locator('[data-testid="item-detail-title"]')).toHaveValue("Updated Title");
});

test("edit description and save persists", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Desc List", "Desc Item");

  await page.goto(`/lists/${listId}`);
  await page.locator('text=Desc Item').click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('[data-testid="item-detail-description"]').fill("My description text");
  await page.locator('[data-testid="item-detail-save"]').click();

  await page.reload();
  await expect(page.locator('[data-testid="item-detail-description"]')).toHaveValue("My description text");
});

// ── Toggle completion ─────────────────────────────────────────────────────────

test("toggle marks item as completed", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Toggle List", "Toggle Item");

  await page.goto(`/lists/${listId}`);
  await page.locator('text=Toggle Item').click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Mark as done");
  await page.locator('[data-testid="item-detail-toggle"]').click();
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Completed");
});

test("toggle from completed back to incomplete", async ({ page, context }) => {
  await setup(page, context);
  const { listId } = await createListWithItem(page, "Uncheck List", "Uncheck Item");

  await page.goto(`/lists/${listId}`);
  await page.waitForSelector("[data-hydrated]");
  // Complete via list page first
  await page.locator('[data-testid="item-toggle"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Open detail and uncheck
  await page.locator('text=Uncheck Item').click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Completed");
  await page.locator('[data-testid="item-detail-toggle"]').click();
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Mark as done");
});
