import { test, expect, Page, BrowserContext, Locator } from "@playwright/test";

// Deleting a list/container also deletes its subtree, but only when the whole tree is
// empty (#223). The UI shows a preview first, blocks when anything holds items and asks
// for confirmation when only empty descendants go away.
//
// Copy assertions accept both locales (`podlist|sublist` etc.) so the spec does not
// depend on the browser language; fluent inserts invisible bidi isolation characters,
// so never match the whole message with an exact string.

const BASE_URL = "http://localhost:3030";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+subtree+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: Page, context: BrowserContext) {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "Subtree User", email, password: PASSWORD },
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

// Create a list (or a folder with `folder: true`) from the home create form.
async function createEntity(page: Page, name: string, folder = false): Promise<Locator> {
  const form = page.locator('[data-testid="create-entity-input"]');
  if (folder) {
    await page.locator('[data-testid="create-entity-tab-folder"]').first().click();
  }
  await form.locator('input[type="text"]').first().fill(name);
  await form.locator("button").first().click();
  const card = page
    .locator(folder ? '[data-testid="container-card"]' : '[data-testid="list-card"]')
    .filter({ hasText: name });
  await card.first().waitFor({ timeout: 5000 });
  return card;
}

async function openList(page: Page, name: string) {
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: name }).first();
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);
  await page.waitForSelector("[data-hydrated]");
}

// Add an empty sublist to the currently open list and return its standalone URL.
async function addSublist(page: Page, name: string): Promise<string> {
  await page.locator('[data-testid="add-sublist-button"]').click();
  const input = page.locator('[data-testid="add-sublist-input"] input[type="text"]');
  await input.fill(name);
  await input.press("Enter");
  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: name });
  await expect(section).toBeVisible();
  const href = await section.locator('[data-testid="sublist-open-link"]').getAttribute("href");
  return href!;
}

async function addItem(page: Page, title: string) {
  await page.locator('[data-testid="add-item-title"]').fill(title);
  await page.locator('[data-testid="add-item-form"] button[type="submit"]').click();
  await expect(page.getByText(title, { exact: true }).first()).toBeVisible({ timeout: 5000 });
}

// Ask for a delete from the home card and return the modal message element.
async function openDeleteDialog(page: Page, card: Locator, kind: "list" | "container") {
  await card
    .locator(kind === "list" ? '[data-testid="list-card-delete"]' : '[data-testid="container-card-delete"]')
    .first()
    .click();
  const dialog = page.locator('[data-testid="confirm-modal"]');
  await expect(dialog).toBeVisible({ timeout: 5000 });
  return { dialog, message: dialog.locator('[data-testid="confirm-modal-message"]') };
}

// ── (a) list with an empty sublist ────────────────────────────────────────────

test("deleting a list with an empty sublist mentions the sublist and removes both", async ({
  page,
  context,
}) => {
  await setup(page, context);
  await createEntity(page, "Subtree Parent");
  await openList(page, "Subtree Parent");
  const sublistUrl = await addSublist(page, "Subtree Child");

  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: "Subtree Parent" });
  const { dialog, message } = await openDeleteDialog(page, card, "list");

  // The dialog announces the empty descendants and offers the delete button.
  await expect(message).toContainText("1");
  await expect(message).toContainText(/(podlist|sublist)/i);
  await expect(dialog.locator('[data-testid="confirm-modal-confirm"]')).toBeVisible();

  await dialog.locator('[data-testid="confirm-modal-confirm"]').click();
  await expect(dialog).toHaveCount(0, { timeout: 5000 });
  await expect(card).toHaveCount(0, { timeout: 5000 });

  // The sublist went away with its parent.
  await page.goto(sublistUrl);
  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveCount(0, {
    timeout: 5000,
  });
});

// ── (b) list containing an item is blocked ────────────────────────────────────

test("deleting a list that still holds items is blocked", async ({ page, context }) => {
  await setup(page, context);
  await createEntity(page, "Subtree Blocked");
  await openList(page, "Subtree Blocked");
  await addItem(page, "Blocking item");

  await page.goto("/");
  await page.waitForSelector("[data-hydrated]");
  const card = page.locator('[data-testid="list-card"]').filter({ hasText: "Subtree Blocked" });
  const { dialog, message } = await openDeleteDialog(page, card, "list");

  await expect(message).toContainText("1");
  await expect(message).toContainText(/(element|item)/i);
  // Blocking dialog: only a close button, no way to delete.
  await expect(dialog.locator('[data-testid="confirm-modal-confirm"]')).toHaveCount(0);
  await dialog.locator('[data-testid="confirm-modal-cancel"]').click();
  await expect(dialog).toHaveCount(0);

  // Nothing was deleted.
  await expect(card.first()).toBeVisible();
});

// ── (c) plain empty container ─────────────────────────────────────────────────

test("deleting an empty container only asks for a plain confirmation", async ({
  page,
  context,
}) => {
  await setup(page, context);
  const card = await createEntity(page, "Subtree Empty Folder", true);

  const { dialog, message } = await openDeleteDialog(page, card, "container");

  await expect(message).not.toContainText(/(podlist|sublist|podkontener|subcontainer)/i);
  await expect(dialog.locator('[data-testid="confirm-modal-confirm"]')).toBeVisible();

  await dialog.locator('[data-testid="confirm-modal-confirm"]').click();
  await expect(dialog).toHaveCount(0, { timeout: 5000 });
  await expect(card).toHaveCount(0, { timeout: 5000 });
});
