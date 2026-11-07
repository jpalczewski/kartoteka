import { test, expect } from "@playwright/test";

import {
  BASE_URL,
  addItem,
  confirmModal,
  createListAndOpen,
  listAction,
  registerAndLogin,
  waitForHydration,
} from "./helpers";

// ── Happy flows ──────────────────────────────────────────────────────────────

test("rename list via inline click — Enter saves", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-heading-input"]');
  await nameInput.waitFor();
  await nameInput.fill("Renamed List");
  await page.keyboard.press("Enter");

  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Renamed List");
});

test("Escape cancels rename without saving", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-heading-input"]');
  await nameInput.waitFor();
  await nameInput.fill("Should Not Save");
  await page.keyboard.press("Escape");

  await expect(page.locator('[data-testid="list-name-heading"]')).toBeVisible();
  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Header Test List");
});

test("pin then unpin via dropdown", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  // Pin
  await listAction(page, "action-pin");

  // After pin: reload page and verify label changed to Odepnij
  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await page.locator('[data-testid="list-actions-btn"]').click();
  await expect(page.locator('[data-testid="action-pin"]')).toContainText("Odepnij");

  // Unpin
  await page.locator('[data-testid="action-pin"]').click();
  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await page.locator('[data-testid="list-actions-btn"]').click();
  await expect(page.locator('[data-testid="action-pin"]')).toContainText("Przypnij");
});

test("reset marks completed items as incomplete", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  // Add item and complete it
  await addItem(page, "Item to reset");
  await page.locator('[data-testid="item-toggle"]').first().click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Reset via dropdown (destructive actions ask for confirmation)
  await listAction(page, "action-reset");
  await confirmModal(page);
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("0/1");
});

test("archive navigates to home", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  await listAction(page, "action-archive");
  await confirmModal(page);
  await page.waitForURL(`${BASE_URL}/`, { timeout: 10_000 });

  expect(page.url()).toBe(`${BASE_URL}/`);
});

// ── Sad flows ────────────────────────────────────────────────────────────────

test("rename with empty/whitespace name shows error toast", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-heading-input"]');
  await nameInput.waitFor();
  await nameInput.fill("   ");
  await page.keyboard.press("Enter");

  // Server rejects the empty name → toast appears
  await expect(page.locator(".toast")).toBeVisible({ timeout: 5000 });
});

test("reset on list with no items is a no-op (no error)", async ({ page, context }) => {
  await registerAndLogin(page, context, "LHA User");
  const listId = await createListAndOpen(page, "Header Test List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  // No items — reset should not crash
  await listAction(page, "action-reset");
  await confirmModal(page);

  // Page should still be on the list (no error navigation)
  await expect(page.locator('[data-testid="list-name-heading"]')).toBeVisible({ timeout: 5000 });
  // No toast error
  await expect(page.locator(".toast.alert-error")).not.toBeVisible();
});
