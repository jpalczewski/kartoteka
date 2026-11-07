import { test, expect, Page } from "@playwright/test";

import { createList, listCard, registerAndLogin, waitForHydration } from "./helpers";

/** Create a tag on the tags page and wait until it is listed. */
async function createTag(page: Page, name: string) {
  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="new-tag-input"]').fill(name);
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: name })).toBeVisible();
}

/** Assign an existing tag through the "+" dropdown of the given tag list. */
async function assignTag(page: Page, container: ReturnType<Page["locator"]>, name: string) {
  await container.locator('[data-testid="tag-add-btn"]').click();
  await container
    .locator('[data-testid="tag-dropdown-option"]')
    .filter({ hasText: name })
    .click();
}

/** Remove a tag by hovering its badge and confirming the removal dialog. */
async function removeTag(page: Page, container: ReturnType<Page["locator"]>, name: string) {
  const badge = container.locator(".tag-badge").filter({ hasText: name });
  await badge.hover();
  await container.locator('[data-testid="tag-remove-btn"]').first().click();
  await page.locator('[data-testid="confirm-modal-confirm"]').click();
}

test("assign tag to list from home page card", async ({ page, context }) => {
  await registerAndLogin(page, context, "ListTags User");
  await createTag(page, "Praca");
  await createList(page, "Moja Lista");

  await page.goto("/");
  await waitForHydration(page);
  const card = listCard(page, "Moja Lista");

  await assignTag(page, card, "Praca");

  await expect(card.locator(".tag-badge").filter({ hasText: "Praca" })).toBeVisible();
});

test("remove tag from list on home page card", async ({ page, context }) => {
  await registerAndLogin(page, context, "ListTags User");
  await createTag(page, "Dom");
  await createList(page, "Lista Do Usunięcia");

  await page.goto("/");
  await waitForHydration(page);
  const card = listCard(page, "Lista Do Usunięcia");

  // Assign first
  await assignTag(page, card, "Dom");
  const badge = card.locator(".tag-badge").filter({ hasText: "Dom" });
  await expect(badge).toBeVisible();

  // Remove via the badge's "×" button (confirmation dialog)
  await removeTag(page, card, "Dom");
  await expect(badge).not.toBeVisible();
});

test("assign tag to list from list detail page", async ({ page, context }) => {
  await registerAndLogin(page, context, "ListTags User");
  await createTag(page, "Projekt");
  await createList(page, "Lista Projektów");

  // Navigate to list detail via title click
  await page.goto("/");
  await waitForHydration(page);
  await listCard(page, "Lista Projektów").locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');
  await assignTag(page, tagsSection, "Projekt");

  await expect(tagsSection.locator(".tag-badge").filter({ hasText: "Projekt" })).toBeVisible();
});

test("remove tag from list on detail page", async ({ page, context }) => {
  await registerAndLogin(page, context, "ListTags User");
  await createTag(page, "Zakupy");
  await createList(page, "Lista Zakupów");

  await page.goto("/");
  await waitForHydration(page);
  await listCard(page, "Lista Zakupów").locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');

  // Assign first
  await assignTag(page, tagsSection, "Zakupy");
  const badge = tagsSection.locator(".tag-badge").filter({ hasText: "Zakupy" });
  await expect(badge).toBeVisible();

  // Remove via the badge's "×" button (confirmation dialog)
  await removeTag(page, tagsSection, "Zakupy");
  await expect(badge).not.toBeVisible();
});

test("tag assigned on detail page appears on home page card", async ({ page, context }) => {
  await registerAndLogin(page, context, "ListTags User");
  await createTag(page, "Ważne");
  await createList(page, "Lista Ważna");

  // Assign from detail page
  await page.goto("/");
  await waitForHydration(page);
  await listCard(page, "Lista Ważna").locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);

  const tagsSection = page.locator('[data-testid="list-tags-section"]');
  await assignTag(page, tagsSection, "Ważne");
  await expect(tagsSection.locator(".tag-badge").filter({ hasText: "Ważne" })).toBeVisible();

  // Navigate home and verify the badge shows on the card
  await page.goto("/");
  await waitForHydration(page);
  const homeCard = listCard(page, "Lista Ważna");
  await expect(homeCard.locator(".tag-badge").filter({ hasText: "Ważne" })).toBeVisible();
});
