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

test("navbar has Tags link that navigates to /tags", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  const link = page.locator('[data-testid="nav-tags"]');
  await expect(link).toBeVisible();
  await link.click();
  await page.waitForURL(/\/tags$/);
  await expect(page.locator('h2:has-text("Tagi")')).toBeVisible();
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

test("delete tag removes it from the list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "DoUsunięcia");
  const row = tagRow(page, "DoUsunięcia");
  await row.locator('[data-testid="tag-delete-btn"]').click();
  await expect(row).not.toBeVisible();
});

test("after deleting last tag empty state appears", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Jedyny");
  const row = tagRow(page, "Jedyny");
  await row.locator('[data-testid="tag-delete-btn"]').click();
  await expect(page.locator('[data-testid="tags-empty-state"]')).toBeVisible();
});

test("click tag navigates to detail page showing tag name", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Podróże");
  await tagRow(page, "Podróże").locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);
  await expect(page.locator('[data-testid="tag-detail-name"]')).toContainText("Podróże");
});

test("tag detail page shows empty message when no items are tagged", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await createTag(page, "Samotny");
  await tagRow(page, "Samotny").locator('[data-testid="tag-link"]').click();
  await expect(page).toHaveURL(/\/tags\/.+/);
  await expect(page.locator('[data-testid="tag-no-items"]')).toBeVisible();
});

test("tag detail page with unknown id shows error", async ({ page, context }) => {
  await registerAndLogin(page, context, "Tags User");
  await page.goto("/tags/00000000-0000-0000-0000-000000000000");
  await waitForHydration(page);
  await expect(page.locator('[data-testid="tag-error"]')).toBeVisible();
});
