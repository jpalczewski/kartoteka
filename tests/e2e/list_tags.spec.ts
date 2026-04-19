import { test, expect, Page, BrowserContext } from "@playwright/test";

const BASE_URL = "http://localhost:3030";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+listtags+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: Page, context: BrowserContext) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "ListTags User", email, password: PASSWORD },
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
}

async function createTag(page: Page, name: string) {
  await page.goto("/tags");
  await page.waitForSelector("[data-hydrated]");
  await page.locator('[data-testid="new-tag-input"]').fill(name);
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(
    page.locator('[data-testid="tag-item"]').filter({ hasText: name })
  ).toBeVisible();
}

async function createList(page: Page, name: string) {
  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  await page.fill('input[placeholder="Nazwa listy..."]', name);
  await page.click('button:has-text("Utwórz")');
  await expect(
    page.locator('[data-testid="list-card"]').filter({ hasText: name })
  ).toBeVisible();
}

test("assign tag to list from home page card", async ({ page, context }) => {
  await setup(page, context);
  await createTag(page, "Praca");
  await createList(page, "Moja Lista");

  await page.goto("/");
  const listCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Moja Lista" });
  const addBtn = listCard.locator('[data-testid="tag-add-btn"]');
  await expect(addBtn).toBeVisible();
  await addBtn.click();

  const option = listCard
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: "Praca" });
  await expect(option).toBeVisible();
  await option.click();

  await expect(
    listCard.locator(".tag-badge").filter({ hasText: "Praca" })
  ).toBeVisible();
});

test("remove tag from list on home page card", async ({ page, context }) => {
  await setup(page, context);
  await createTag(page, "Dom");
  await createList(page, "Lista Do Usunięcia");

  await page.goto("/");
  const listCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Lista Do Usunięcia" });

  // Assign first
  await listCard.locator('[data-testid="tag-add-btn"]').click();
  await listCard
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: "Dom" })
    .click();
  const badge = listCard.locator(".tag-badge").filter({ hasText: "Dom" });
  await expect(badge).toBeVisible();

  // Remove by clicking badge
  await badge.click();
  await expect(badge).not.toBeVisible();
});

test("assign tag to list from list detail page", async ({ page, context }) => {
  await setup(page, context);
  await createTag(page, "Projekt");
  await createList(page, "Lista Projektów");

  // Navigate to list detail via title click
  await page.goto("/");
  const listCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Lista Projektów" });
  await listCard.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');
  const addBtn = tagsSection.locator('[data-testid="tag-add-btn"]');
  await expect(addBtn).toBeVisible();
  await addBtn.click();

  const option = tagsSection
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: "Projekt" });
  await expect(option).toBeVisible();
  await option.click();

  await expect(
    tagsSection.locator(".tag-badge").filter({ hasText: "Projekt" })
  ).toBeVisible();
});

test("remove tag from list on detail page", async ({ page, context }) => {
  await setup(page, context);
  await createTag(page, "Zakupy");
  await createList(page, "Lista Zakupów");

  await page.goto("/");
  const listCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Lista Zakupów" });
  await listCard.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');

  // Assign first
  await tagsSection.locator('[data-testid="tag-add-btn"]').click();
  await tagsSection
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: "Zakupy" })
    .click();
  const badge = tagsSection.locator(".tag-badge").filter({ hasText: "Zakupy" });
  await expect(badge).toBeVisible();

  // Remove by clicking badge
  await badge.click();
  await expect(badge).not.toBeVisible();
});

test("tag assigned on detail page appears on home page card", async ({
  page,
  context,
}) => {
  await setup(page, context);
  await createTag(page, "Ważne");
  await createList(page, "Lista Ważna");

  // Assign from detail page
  await page.goto("/");
  const listCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Lista Ważna" });
  await listCard.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');
  await tagsSection.locator('[data-testid="tag-add-btn"]').click();
  await tagsSection
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: "Ważne" })
    .click();
  await expect(
    tagsSection.locator(".tag-badge").filter({ hasText: "Ważne" })
  ).toBeVisible();

  // Navigate home and verify badge shows on card
  await page.goto("/");
  const homeCard = page
    .locator('[data-testid="list-card"]')
    .filter({ hasText: "Lista Ważna" });
  await expect(
    homeCard.locator(".tag-badge").filter({ hasText: "Ważne" })
  ).toBeVisible();
});
