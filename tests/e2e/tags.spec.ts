import { test, expect, Page } from "@playwright/test";

import { registerAndLogin, waitForHydration } from "./helpers";

/** Create a tag on the tags page and wait until it is listed. */
async function createTag(page: Page, name: string) {
  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="new-tag-input"]').fill(name);
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: name })).toBeVisible();
}

/** Row of the tag with the given name. */
function tagRow(page: Page, name: string) {
  return page.locator('[data-testid="tag-item"]').filter({ hasText: name });
}

/** Open the row's action menu (the trigger only becomes opaque on hover). */
async function openRowMenu(page: Page, name: string) {
  const row = tagRow(page, name);
  await row.hover();
  await row.locator('[data-testid="tag-actions-btn"]').click();
  await expect(page.locator('[data-testid="action-menu"]')).toBeVisible();
}

/** Delete a tag through its row menu and confirm the dialog. */
async function deleteTag(page: Page, name: string) {
  await openRowMenu(page, name);
  await page.locator('[data-testid="tag-delete-btn"]').click();
  await page.locator('[data-testid="confirm-modal-confirm"]').click();
}

test("navbar has Tags link that navigates to /tags", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  const link = page.locator('[data-testid="nav-tags"]');
  await expect(link).toBeVisible();
  await link.click();
  await page.waitForURL(/\/tags$/);
  await expect(page.locator('[data-testid="tags-title"]')).toBeVisible();
});

test("tags page shows empty state when no tags exist", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("create tag via button shows it in the list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Praca");
  await expect(tagRow(page, "Praca")).toBeVisible();
});

test("create tag via Enter key shows it in the list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="new-tag-input"]').fill("Dom");
  await page.locator('[data-testid="new-tag-input"]').press("Enter");
  await expect(tagRow(page, "Dom")).toBeVisible();
});

test("create tag clears input after submit", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Hobby");
  await expect(page.locator('[data-testid="new-tag-input"]')).toHaveValue("");
});

test("create tag with empty name does nothing", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("create tag uses the colour picked from the palette", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags");
  await waitForHydration(page);

  // Pick a non-default swatch, then create the tag with it.
  const swatch = page.locator('[data-testid="tag-color-swatch"]').nth(4);
  const picked = await swatch.evaluate((el) => getComputedStyle(el).backgroundColor);
  await swatch.click();
  await page.locator('[data-testid="new-tag-input"]').fill("Kolorowy");
  await page.locator('[data-testid="create-tag-btn"]').click();

  const badge = tagRow(page, "Kolorowy").locator(".tag-badge");
  await expect(badge).toBeVisible();
  await expect(badge).toHaveCSS("background-color", picked);
});

test("search filters the tag tree", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Praca");
  await createTag(page, "Dom");

  await page.locator('[data-testid="tag-search"]').fill("Pra");
  await expect(tagRow(page, "Praca")).toBeVisible();
  await expect(tagRow(page, "Dom")).toBeHidden();

  await page.locator('[data-testid="tag-search-clear"]').click();
  await expect(tagRow(page, "Dom")).toBeVisible();
});

test("search without a match shows the empty result state", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Praca");
  await page.locator('[data-testid="tag-search"]').fill("zzz");
  await expect(page.locator('[data-testid="tags-no-match"]')).toBeVisible();
});

test("rename tag from the row menu updates the row", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Stare");

  await openRowMenu(page, "Stare");
  await page.locator('[data-testid="tag-rename-btn"]').click();
  await page.locator('[data-testid="tag-rename-input"]').fill("Nowe");
  await page.locator('[data-testid="tag-rename-input"]').press("Enter");

  await expect(tagRow(page, "Nowe")).toBeVisible();
});

test("add subtag from the row menu nests it under the parent", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Hobby");

  await openRowMenu(page, "Hobby");
  await page.locator('[data-testid="tag-add-child-btn"]').click();
  await page.locator('[data-testid="tag-child-input"]').fill("Ogród");
  await page.locator('[data-testid="tag-child-input"]').press("Enter");

  await expect(tagRow(page, "Ogród")).toBeVisible();
  // The child is collapsible: collapsing the parent hides it again.
  await tagRow(page, "Hobby").locator('[data-testid="tag-toggle-btn"]').click();
  await expect(tagRow(page, "Ogród")).toBeHidden();
});

test("delete tag asks for confirmation and removes it on confirm", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "DoUsunięcia");

  await openRowMenu(page, "DoUsunięcia");
  await page.locator('[data-testid="tag-delete-btn"]').click();
  await expect(page.locator('[data-testid="confirm-modal"]')).toBeVisible();
  await page.locator('[data-testid="confirm-modal-confirm"]').click();

  await expect(tagRow(page, "DoUsunięcia")).toBeHidden();
});

test("cancelling the delete confirmation keeps the tag", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Zostaje");

  await openRowMenu(page, "Zostaje");
  await page.locator('[data-testid="tag-delete-btn"]').click();
  await page.locator('[data-testid="confirm-modal-cancel"]').click();

  await expect(tagRow(page, "Zostaje")).toBeVisible();
});

test("after deleting last tag empty state appears", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Jedyny");
  await deleteTag(page, "Jedyny");
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("click tag navigates to detail page showing tag name", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Podróże");
  await tagRow(page, "Podróże").locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);
  await expect(page.locator('[data-testid="tag-detail-name"]')).toContainText("Podróże");
});

test("tag detail page shows empty messages when nothing is linked", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Samotny");
  await tagRow(page, "Samotny").locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);
  await expect(page.locator('[data-testid="tag-no-items"]')).toBeVisible();
  await expect(page.locator('[data-testid="tag-no-lists"]')).toBeVisible();
});

test("tag detail page sets an icon that then shows on the tags page", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "ZIkona");
  await tagRow(page, "ZIkona").locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);

  await page.locator('[data-testid="tag-icon-input"]').fill("emoji:⭐");
  await page.locator('[data-testid="tag-icon-set-btn"]').click();

  // The tree badge picks the stored icon up on the next visit.
  await page.goto("/tags");
  await waitForHydration(page);
  await expect(tagRow(page, "ZIkona").locator(".tag-badge")).toContainText("⭐");
});

test("tag detail page with unknown id shows error", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags/00000000-0000-0000-0000-000000000000");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="tag-error"]')).toBeVisible();
});
