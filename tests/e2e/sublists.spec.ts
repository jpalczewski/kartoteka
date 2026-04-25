import { test, expect, Page, BrowserContext } from "@playwright/test";

const BASE_URL = "http://localhost:3030";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+sub+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: Page, context: BrowserContext) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "Sub User", email, password: PASSWORD },
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

async function createListAndNavigate(page: Page, name: string): Promise<string> {
  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  await page.fill('input[placeholder="Nazwa listy..."]', name);
  await page.locator('button:has-text("Utwórz")').first().click();
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: name });
  await card.waitFor({ timeout: 5000 });
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);
  return page.url().split("/lists/")[1];
}

// ── Create sublist ────────────────────────────────────────────────────────────

test("create sublist appears in parent list", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Parent List");
  await page.goto(`/lists/${listId}`);

  await page.locator('button:has-text("Dodaj podlistę")').click();
  await page.fill('input[placeholder="Nazwa podlisty..."]', "My Sublist");
  await page.locator('button:has-text("Utwórz")').click();

  await expect(
    page.locator('[data-testid="sublist-section"]').filter({ hasText: "My Sublist" })
  ).toBeVisible();
});

test("multiple sublists all appear", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Multi Sublist Parent");
  await page.goto(`/lists/${listId}`);

  for (const name of ["Alpha", "Beta", "Gamma"]) {
    await page.locator('button:has-text("Dodaj podlistę")').click();
    await page.fill('input[placeholder="Nazwa podlisty..."]', name);
    await page.locator('button:has-text("Utwórz")').click();
    await expect(
      page.locator('[data-testid="sublist-section"]').filter({ hasText: name })
    ).toBeVisible();
  }

  await expect(page.locator('[data-testid="sublist-section"]')).toHaveCount(3);
});

// ── Add items to sublist ──────────────────────────────────────────────────────

test("add item to sublist shows it inside that section", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Item Parent");
  await page.goto(`/lists/${listId}`);

  await page.locator('button:has-text("Dodaj podlistę")').click();
  await page.fill('input[placeholder="Nazwa podlisty..."]', "Sub With Item");
  await page.locator('button:has-text("Utwórz")').click();

  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Sub With Item" });
  await expect(section).toBeVisible();

  await section.locator('input[placeholder="Nowy element..."]').fill("Subtask 1");
  await section.locator('button:has-text("Dodaj")').click();

  await expect(section.locator('text=Subtask 1')).toBeVisible();
});

test("sublist progress counter updates after adding item", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Progress Parent");
  await page.goto(`/lists/${listId}`);

  await page.locator('button:has-text("Dodaj podlistę")').click();
  await page.fill('input[placeholder="Nazwa podlisty..."]', "Progress Sub");
  await page.locator('button:has-text("Utwórz")').click();

  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Progress Sub" });
  await section.locator('input[placeholder="Nowy element..."]').fill("Task A");
  await section.locator('button:has-text("Dodaj")').click();
  await expect(section.locator('text=Task A')).toBeVisible();

  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("0/1");
});

// ── Toggle items in sublist ───────────────────────────────────────────────────

test("toggling item in sublist updates progress counter", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Toggle Parent");
  await page.goto(`/lists/${listId}`);

  await page.locator('button:has-text("Dodaj podlistę")').click();
  await page.fill('input[placeholder="Nazwa podlisty..."]', "Toggle Sub");
  await page.locator('button:has-text("Utwórz")').click();

  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Toggle Sub" });
  await section.locator('input[placeholder="Nowy element..."]').fill("Check Me");
  await section.locator('button:has-text("Dodaj")').click();
  await expect(section.locator('text=Check Me')).toBeVisible();
  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("0/1");

  await section.locator('[data-testid="item-toggle"]').first().click();
  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("1/1");
});

// ── Open sublist as full page ─────────────────────────────────────────────────

test("↗ link opens sublist as standalone list page", async ({ page, context }) => {
  await setup(page, context);
  const listId = await createListAndNavigate(page, "Nav Parent");
  await page.goto(`/lists/${listId}`);

  await page.locator('button:has-text("Dodaj podlistę")').click();
  await page.fill('input[placeholder="Nazwa podlisty..."]', "Nav Sub");
  await page.locator('button:has-text("Utwórz")').click();

  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Nav Sub" });
  await expect(section).toBeVisible();

  const href = await section.locator('[data-testid="sublist-open-link"]').getAttribute("href");
  await page.goto(href!);
  await page.waitForSelector("[data-hydrated]");

  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Nav Sub");
});
