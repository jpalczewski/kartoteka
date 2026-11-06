import { test, expect, Page, BrowserContext } from "@playwright/test";

const BASE_URL = "http://localhost:3030";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+all+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: Page, context: BrowserContext) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "All User", email, password: PASSWORD },
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

async function createListWithItems(page: Page, listName: string, items: string[]) {
  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  await page.fill('input[placeholder="Nazwa listy..."]', listName);
  await page.locator('button:has-text("Utwórz")').first().click();
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: listName });
  await card.waitFor({ timeout: 5000 });
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  for (const item of items) {
    await page.fill('input[placeholder="Nowy element..."]', item);
    await page.click('button:has-text("Dodaj")');
    await page.getByText(item, { exact: true }).waitFor();
  }
}

async function createNotesListWithDescribedItem(
  page: Page,
  listName: string,
  itemTitle: string,
  description: string,
) {
  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  await page.getByLabel("Notes").check();
  await page.fill('input[placeholder="Nazwa listy..."]', listName);
  await page.locator('button:has-text("Utwórz")').first().click();
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: listName });
  await card.waitFor({ timeout: 5000 });
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  await page.fill('input[placeholder="Nowy element..."]', itemTitle);
  await page.click('button:has-text("Dodaj")');
  await page.getByText(itemTitle, { exact: true }).waitFor();

  await page.getByText(itemTitle, { exact: true }).click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);
  await page.locator('[data-testid="item-detail-description"]').fill(description);
  await page.locator('[data-testid="item-detail-save"]').click();
}

// ── Empty state ───────────────────────────────────────────────────────────────

test("all items page shows empty state with no items", async ({ page, context }) => {
  await setup(page, context);
  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");
  await expect(page.locator('text=No items.')).toBeVisible();
});

// ── Displays items ────────────────────────────────────────────────────────────

test("items from all lists appear on all items page", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "Lista A", ["Item Alpha"]);
  await createListWithItems(page, "Lista B", ["Item Beta"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");
  await expect(page.locator('text=Item Alpha')).toBeVisible();
  await expect(page.locator('text=Item Beta')).toBeVisible();
});

test("completion count reflects all items", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "Count List", ["One", "Two", "Three"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("0/3");
});

// ── Toggle from all items page ────────────────────────────────────────────────

test("toggling item on all page updates completion count", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "Toggle All List", ["Task X"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("0/1");

  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/1");
});

// ── Hide completed filter ─────────────────────────────────────────────────────

test("hiding completed removes completed items from view", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "Filter All List", ["Done Task", "Pending Task"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");

  // Complete the first item
  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/2");

  // Hide completed
  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.locator('text=Done Task')).not.toBeVisible();
  await expect(page.locator('text=Pending Task')).toBeVisible();
});

test("all completed + hide filter shows helpful message", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "All Done List", ["Solo Task"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");

  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/1");

  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.locator('text=All items completed')).toBeVisible();
});

// ── Item link navigates to detail ─────────────────────────────────────────────

test("clicking item title on all page navigates to item detail", async ({ page, context }) => {
  await setup(page, context);
  await createListWithItems(page, "Link List", ["Detail Link Item"]);

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");
  await page.locator('text=Detail Link Item').click();

  await expect(page).toHaveURL(/\/lists\/.+\/items\/.+/);
});

// ── Notes list items ──────────────────────────────────────────────────────────

test("notes items have no checkbox and show their description", async ({ page, context }) => {
  await setup(page, context);
  await createNotesListWithDescribedItem(
    page,
    "Notes List",
    "Notes Item",
    "A note body that must be visible",
  );

  await page.goto("/all");
  await page.waitForSelector("[data-hydrated]");

  await expect(page.locator('text=Notes Item')).toBeVisible();
  await expect(page.locator('[data-testid="all-item-toggle"]')).toHaveCount(0);
  await expect(page.locator('[data-testid="all-item-description"]')).toBeVisible();
  await expect(page.locator('[data-testid="all-item-description"]')).toContainText(
    "A note body that must be visible",
  );
});
