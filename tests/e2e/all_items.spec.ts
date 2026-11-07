import { test, expect, Page } from "@playwright/test";

import {
  addItem,
  createList,
  createListOnCurrentPage,
  gotoPage,
  openList,
  registerAndLogin,
  saveItemDetail,
  waitForHydration,
} from "./helpers";

/** Create a list, open it and add the given items. */
async function createListWithItems(page: Page, listName: string, items: string[]) {
  await createList(page, listName);
  await openList(page, listName);
  for (const item of items) {
    await addItem(page, item);
  }
}

/** Create a notes list with a single item that has a description. */
async function createNotesListWithDescribedItem(
  page: Page,
  listName: string,
  itemTitle: string,
  description: string,
) {
  await gotoPage(page, "/");
  await waitForHydration(page);
  await page.getByLabel("Notes").check();
  await createListOnCurrentPage(page, listName);
  await openList(page, listName);

  await addItem(page, itemTitle);
  await page.locator('[data-testid="item-title-link"]').filter({ hasText: itemTitle }).click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);
  await page.locator('[data-testid="item-detail-description"]').fill(description);
  await saveItemDetail(page);
}

// ── Empty state ───────────────────────────────────────────────────────────────

test("all items page shows empty state with no items", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await gotoPage(page, "/all");
  await waitForHydration(page);
  await expect(page.getByText("No items.", { exact: true })).toBeVisible();
});

// ── Displays items ────────────────────────────────────────────────────────────

test("items from all lists appear on all items page", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "Lista A", ["Item Alpha"]);
  await createListWithItems(page, "Lista B", ["Item Beta"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);
  await expect(page.getByText("Item Alpha", { exact: true })).toBeVisible();
  await expect(page.getByText("Item Beta", { exact: true })).toBeVisible();
});

test("completion count reflects all items", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "Count List", ["One", "Two", "Three"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("0/3");
});

// ── Toggle from all items page ────────────────────────────────────────────────

test("toggling item on all page updates completion count", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "Toggle All List", ["Task X"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("0/1");

  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/1");
});

// ── Hide completed filter ─────────────────────────────────────────────────────

test("hiding completed removes completed items from view", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "Filter All List", ["Done Task", "Pending Task"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);

  // Complete the first item
  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/2");

  // Hide completed
  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.getByText("Done Task", { exact: true })).not.toBeVisible();
  await expect(page.getByText("Pending Task", { exact: true })).toBeVisible();
});

test("all completed + hide filter shows helpful message", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "All Done List", ["Solo Task"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);

  await page.locator('[data-testid="all-item-toggle"]').first().click();
  await expect(page.locator('[data-testid="all-completion-count"]')).toContainText("1/1");

  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.getByText("All items completed", { exact: false })).toBeVisible();
});

// ── Item link navigates to detail ─────────────────────────────────────────────

test("clicking item title on all page navigates to item detail", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createListWithItems(page, "Link List", ["Detail Link Item"]);

  await gotoPage(page, "/all");
  await waitForHydration(page);
  await page.getByText("Detail Link Item", { exact: true }).click();

  await expect(page).toHaveURL(/\/lists\/.+\/items\/.+/);
});

// ── Notes list items ──────────────────────────────────────────────────────────

test("notes items have no checkbox and show their description", async ({ page, context }) => {
  await registerAndLogin(page, context, "All User");
  await createNotesListWithDescribedItem(
    page,
    "Notes List",
    "Notes Item",
    "A note body that must be visible",
  );

  await gotoPage(page, "/all");
  await waitForHydration(page);

  await expect(page.getByText("Notes Item", { exact: true })).toBeVisible();
  await expect(page.locator('[data-testid="all-item-toggle"]')).toHaveCount(0);
  await expect(page.locator('[data-testid="all-item-description"]')).toBeVisible();
  await expect(page.locator('[data-testid="all-item-description"]')).toContainText(
    "A note body that must be visible",
  );
});
