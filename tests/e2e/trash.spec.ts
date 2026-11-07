import { test, expect, BrowserContext, Page } from "@playwright/test";

import {
  BASE_URL,
  addItem,
  createList,
  listCard,
  openList,
  registerAndLogin,
  waitForHydration,
} from "./helpers";

// `/trash` is the review page for the pending-deletions queue (#226, #389): agents can
// only mark, the user decides here what actually goes. Deleting from the queue cascades
// through items, so the confirmation has to mention them.
//
// Assertions stay locale-agnostic (test ids, data attributes and `sublist|podlist`
// style regexes) because the app follows the browser language.

/** Mark (or unmark) an entity straight through the server fn the UI calls. */
async function toggleMark(
  context: BrowserContext,
  entityType: "item" | "list" | "container" | "comment",
  entityId: string,
): Promise<void> {
  const res = await context.request.post(`${BASE_URL}/leptos/toggle_pending_deletion`, {
    headers: { "Content-Type": "application/json" },
    data: { entity_type: entityType, entity_id: entityId },
  });
  expect(res.ok()).toBeTruthy();
}

/** Create a list on the home page and return its id. */
async function createListWithId(page: Page, name: string): Promise<string> {
  await createList(page, name);
  return openList(page, name);
}

/** Id of an item on the list page that is currently open. */
async function itemId(page: Page, title: string): Promise<string> {
  const href = await page
    .locator('[data-testid="item-title-link"]')
    .filter({ hasText: title })
    .first()
    .getAttribute("href");
  expect(href).toBeTruthy();
  return href!.split("/items/")[1];
}

/** Row of the trash page that belongs to the row with the given label. */
function trashRow(page: Page, label: string) {
  return page.locator('[data-testid="trash-row"]').filter({ hasText: label }).first();
}

/** The badge in the user dropdown; opened first so it can be seen the way a user would. */
async function navTrashCount(page: Page) {
  await page.locator('[data-testid="nav-user-menu"]').click();
  return page.locator('[data-testid="nav-trash-count"]');
}

// ── Row, context and nav badge ────────────────────────────────────────────────

test("a marked item shows up with its list as context and the badge shows 1", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Trash User");
  const listId = await createListWithId(page, "Trash Context List");
  await addItem(page, "Trash Context Item");
  const markedId = await itemId(page, "Trash Context Item");
  await toggleMark(context, "item", markedId);

  await page.goto("/trash");
  await waitForHydration(page);

  const row = trashRow(page, "Trash Context Item");
  await expect(row).toBeVisible();
  await expect(row.locator('[data-testid="trash-row-context"]')).toHaveText("Trash Context List");
  await expect(row.locator('[data-testid="trash-row-marked-at"]')).not.toBeEmpty();
  await expect(row.locator('[data-testid="trash-row-label"]')).toHaveAttribute(
    "href",
    `/lists/${listId}/items/${markedId}`,
  );

  // Items live in their own section.
  const section = page.locator('[data-testid="trash-section"][data-entity-type="item"]');
  await expect(section).toBeVisible();
  await expect(section.locator('[data-testid="trash-section-count"]')).toHaveText("1");

  const badge = await navTrashCount(page);
  await expect(badge).toHaveText("1");
});

// ── Unmark keeps the entity ──────────────────────────────────────────────────

test("unmarking a row removes it from the trash but keeps the list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Trash User");
  const listId = await createListWithId(page, "Trash Sparred List");
  await toggleMark(context, "list", listId);

  await page.goto("/trash");
  await waitForHydration(page);
  const row = trashRow(page, "Trash Sparred List");
  await expect(row).toBeVisible();

  await row.locator('[data-testid="trash-row-unmark"]').click();
  await expect(page.locator('[data-testid="trash-row"]')).toHaveCount(0, { timeout: 10_000 });
  await expect(page.locator('[data-testid="trash-empty"]')).toBeVisible();

  // The list itself is untouched.
  await page.goto("/");
  await waitForHydration(page);
  await expect(listCard(page, "Trash Sparred List")).toBeVisible();
});

// ── Deleting cascades through items ──────────────────────────────────────────

test("deleting a marked list warns about its items and removes the list", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Trash User");
  const listId = await createListWithId(page, "Trash Cascade List");
  await addItem(page, "Trash Cascade Item");
  await toggleMark(context, "list", listId);

  await page.goto("/trash");
  await waitForHydration(page);
  const row = trashRow(page, "Trash Cascade List");
  await expect(row).toBeVisible();
  // What the cascade takes along is announced on the row itself.
  await expect(row.locator('[data-testid="trash-row-cascade"]')).toContainText(/item|element/i);

  await row.locator('[data-testid="trash-row-delete"]').click();
  const dialog = page.locator('[data-testid="confirm-modal"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });
  const message = dialog.locator('[data-testid="confirm-modal-message"]');
  await expect(message).toContainText("1");
  await expect(message).toContainText(/item|element/i);

  await dialog.locator('[data-testid="confirm-modal-confirm"]').click();
  await expect(dialog).toHaveCount(0, { timeout: 10_000 });
  await expect(page.locator('[data-testid="trash-row"]')).toHaveCount(0, { timeout: 10_000 });
  await expect(page.locator('[data-testid="trash-empty"]')).toBeVisible();

  // The list is gone from home, together with its item.
  await page.goto("/");
  await waitForHydration(page);
  await expect(listCard(page, "Trash Cascade List")).toHaveCount(0);
});

// ── Delete all empties the queue ─────────────────────────────────────────────

test("delete all empties a two row trash", async ({ page, context }) => {
  await registerAndLogin(page, context, "Trash User");
  const firstId = await createListWithId(page, "Trash Bulk One");
  const secondId = await createListWithId(page, "Trash Bulk Two");
  await toggleMark(context, "list", firstId);
  await toggleMark(context, "list", secondId);

  await page.goto("/trash");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="trash-row"]')).toHaveCount(2);
  await expect(page.locator('[data-testid="trash-section-count"]').first()).toHaveText("2");

  await page.locator('[data-testid="trash-bulk-delete-all"]').click();
  const dialog = page.locator('[data-testid="confirm-modal"]');
  await expect(dialog).toBeVisible({ timeout: 10_000 });
  await expect(dialog.locator('[data-testid="confirm-modal-message"]')).toContainText("2");

  await dialog.locator('[data-testid="confirm-modal-confirm"]').click();
  await expect(page.locator('[data-testid="trash-empty"]')).toBeVisible({ timeout: 10_000 });
  await expect(page.locator('[data-testid="trash-row"]')).toHaveCount(0);

  await page.goto("/");
  await waitForHydration(page);
  await expect(listCard(page, "Trash Bulk One")).toHaveCount(0);
  await expect(listCard(page, "Trash Bulk Two")).toHaveCount(0);
});

// ── Selecting rows ──────────────────────────────────────────────────────────

test("selecting all rows enables the bulk actions", async ({ page, context }) => {
  await registerAndLogin(page, context, "Trash User");
  const listId = await createListWithId(page, "Trash Select List");
  await toggleMark(context, "list", listId);

  await page.goto("/trash");
  await waitForHydration(page);

  const unmarkSelected = page.locator('[data-testid="trash-bulk-unmark"]');
  await expect(unmarkSelected).toBeDisabled();

  await page.locator('[data-testid="trash-select-all"]').check();
  await expect(unmarkSelected).toBeEnabled();
  await expect(page.locator('[data-testid="trash-row-checkbox"]').first()).toBeChecked();

  await unmarkSelected.click();
  await expect(page.locator('[data-testid="trash-empty"]')).toBeVisible({ timeout: 10_000 });

  await page.goto("/");
  await waitForHydration(page);
  await expect(listCard(page, "Trash Select List")).toBeVisible();
});
