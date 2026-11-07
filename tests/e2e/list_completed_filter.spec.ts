import { test, expect } from "@playwright/test";

import {
  addItem,
  createListAndOpen,
  registerAndLogin,
  waitForHydration,
} from "./helpers";

test("completed items are visible by default", async ({ page, context }) => {
  await registerAndLogin(page, context, "CF User");
  const listId = await createListAndOpen(page, "Filter Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await addItem(page, "Task A");

  // Complete the item
  const itemCheckbox = page.locator('[data-testid="item-toggle"]').first();
  await itemCheckbox.waitFor();
  await itemCheckbox.click();

  // Item still visible and count reflects completion
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");
  await expect(page.getByText("Task A", { exact: true })).toBeVisible();
});

test("hiding completed removes them from view but count stays", async ({ page, context }) => {
  await registerAndLogin(page, context, "CF User");
  const listId = await createListAndOpen(page, "Filter Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  // Add two items — wait for each to appear before adding the next,
  // otherwise the second add races the first's refresh.
  await addItem(page, "Done Item");
  await addItem(page, "Pending Item");

  // Complete the first one
  await page.locator('[data-testid="item-toggle"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/2");

  // Toggle hide completed
  await page.locator('[data-testid="hide-completed-toggle"]').click();

  // Completed item hidden, pending still visible, count unchanged
  await expect(page.getByText("Done Item", { exact: true })).not.toBeVisible();
  await expect(page.getByText("Pending Item", { exact: true })).toBeVisible();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/2");
});

test("all-completed + filter shows helpful message", async ({ page, context }) => {
  await registerAndLogin(page, context, "CF User");
  const listId = await createListAndOpen(page, "Filter Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await addItem(page, "Only Item");

  // Complete it
  await page.locator('[data-testid="item-toggle"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Hide completed → all gone → helpful message appears
  await page.locator('[data-testid="hide-completed-toggle"]').click();
  await expect(page.getByText("Wszystkie elementy ukończone", { exact: false })).toBeVisible();
});
