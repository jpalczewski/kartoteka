import { test, expect } from "@playwright/test";

import {
  BASE_URL,
  addItem,
  createListAndOpen,
  registerAndLogin,
  waitForHydration,
} from "./helpers";

/** Click the title of the item row with the given title. */
function itemLink(page: import("@playwright/test").Page, title: string) {
  return page.locator('[data-testid="item-title-link"]').filter({ hasText: title });
}

test("clicking item title navigates to item detail page", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Nav List");
  await addItem(page, "Click Me");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await itemLink(page, "Click Me").click();

  await expect(page).toHaveURL(/\/lists\/.+\/items\/.+/);
  await expect(page.locator('[data-testid="item-detail-title"]')).toHaveValue("Click Me");
});

test("back link returns to parent list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Back List");
  await addItem(page, "Back Item");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await itemLink(page, "Back Item").click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('a:has-text("← Back to list")').click();
  await expect(page).toHaveURL(`${BASE_URL}/lists/${listId}`);
});

// ── Edit title ────────────────────────────────────────────────────────────────

test("edit title and save updates the item", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Edit List");
  await addItem(page, "Original Title");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await itemLink(page, "Original Title").click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('[data-testid="item-detail-title"]').fill("Updated Title");
  await page.locator('[data-testid="item-detail-save"]').click();

  // Reload to confirm server persisted
  await page.reload();
  await expect(page.locator('[data-testid="item-detail-title"]')).toHaveValue("Updated Title");
});

test("edit description and save persists", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Desc List");
  await addItem(page, "Desc Item");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await itemLink(page, "Desc Item").click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await page.locator('[data-testid="item-detail-description"]').fill("My description text");
  await page.locator('[data-testid="item-detail-save"]').click();

  await page.reload();
  await expect(page.locator('[data-testid="item-detail-description"]')).toHaveValue(
    "My description text",
  );
});

// ── Toggle completion ─────────────────────────────────────────────────────────

test("toggle marks item as completed", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Toggle List");
  await addItem(page, "Toggle Item");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await itemLink(page, "Toggle Item").click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);

  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Mark as done");
  await page.locator('[data-testid="item-detail-toggle"]').click();
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Completed");
});

test("toggle from completed back to incomplete", async ({ page, context }) => {
  await registerAndLogin(page, context, "Item User");
  const listId = await createListAndOpen(page, "Uncheck List");
  await addItem(page, "Uncheck Item");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  // Complete via list page first
  await page.locator('[data-testid="item-toggle"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Open detail and uncheck
  await itemLink(page, "Uncheck Item").click();
  await page.waitForURL(/\/lists\/.+\/items\/.+/);
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Completed");
  await page.locator('[data-testid="item-detail-toggle"]').click();
  await expect(page.locator('[data-testid="item-detail-status"]')).toHaveText("Mark as done");
});
