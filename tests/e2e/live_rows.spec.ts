import { test, expect, Page, Locator, BrowserContext } from "@playwright/test";

import {
  createListOnCurrentPage,
  openList,
  registerAndLogin,
  submitAddItem,
  waitForHydration,
} from "./helpers";

// Item views render keyed rows from a store (state/item_store.rs): a live update must
// update the changed row in place and leave every other row's DOM element alone, so row
// UI state (an open editor, focus) survives other people's changes.
//
// Each test opens the view in tab 1, marks row B's element, completes item A from tab 2,
// and checks in tab 1 that A shows as completed (live update arrived) and B's element is
// still the same object (row not re-created).

const MARK = "__liveRowMark";

/** Today as YYYY-MM-DD in the browser's timezone (the test user keeps the default, UTC). */
function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

/** Create a Schedule list (checklist + deadlines) with items A, B, C due today; returns its id. */
async function scheduleListWithItems(page: Page, name: string): Promise<string> {
  await page.goto("/");
  await waitForHydration(page);
  await page.locator('[data-testid="list-type-schedule"]').check();
  await createListOnCurrentPage(page, name);
  const listId = await openList(page, name);
  for (const title of ["Live A", "Live B", "Live C"]) {
    await submitAddItem(page, title, async () => {
      const deadline = page.locator('[data-testid="add-item-deadline"]');
      if (!(await deadline.isVisible())) {
        await page.locator('[data-testid="add-item-dates-toggle"]').click();
      }
      await deadline.fill(todayIso());
    });
    await expect(page.getByText(title, { exact: true })).toBeVisible();
  }
  return listId;
}

async function markRow(row: Locator) {
  await row.evaluate((el, mark) => {
    (el as unknown as Record<string, boolean>)[mark] = true;
  }, MARK);
}

async function isMarked(row: Locator): Promise<boolean> {
  return row.evaluate(
    (el, mark) => (el as unknown as Record<string, boolean>)[mark] === true,
    MARK,
  );
}

/** Complete an item from a second tab, on its list page. */
async function completeFromOtherTab(context: BrowserContext, listId: string, title: string) {
  const other = await context.newPage();
  await other.goto(`/lists/${listId}`);
  await waitForHydration(other);
  // Every list row has one toggle and one title link, in the same order.
  const titles = (await other.locator('[data-testid="item-title-link"]').allInnerTexts()).map((t) =>
    t.trim(),
  );
  const index = titles.indexOf(title);
  expect(index, `item "${title}" on the list page`).toBeGreaterThanOrEqual(0);
  await other.locator('[data-testid="item-toggle"]').nth(index).check();
  await other.close();
}

test("Today: a live update keeps untouched rows", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live Today");
  const listId = await scheduleListWithItems(page, "Live Today List");

  await page.goto("/today");
  await waitForHydration(page);
  const row = (title: string) =>
    page.locator('[data-testid="today-row"]').filter({ hasText: title });
  await expect(row("Live B")).toBeVisible();
  await markRow(row("Live B"));

  await completeFromOtherTab(context, listId, "Live A");

  await expect(row("Live A").locator('input[type="checkbox"]')).toBeChecked();
  expect(await isMarked(row("Live B"))).toBe(true);
});

test("All: a live update keeps untouched rows", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live All");
  const listId = await scheduleListWithItems(page, "Live All List");

  await page.goto("/all");
  await waitForHydration(page);
  const row = (title: string) => page.locator('[data-testid="all-row"]').filter({ hasText: title });
  await expect(row("Live B")).toBeVisible();
  await markRow(row("Live B"));

  await completeFromOtherTab(context, listId, "Live A");

  await expect(row("Live A").locator('[data-testid="all-item-toggle"]')).toBeChecked();
  expect(await isMarked(row("Live B"))).toBe(true);
});

test("Calendar week: a live update keeps untouched rows", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live Week");
  const listId = await scheduleListWithItems(page, "Live Week List");

  await page.goto("/calendar");
  await waitForHydration(page);
  await page.locator('[data-testid="calendar-view-week"]').click();
  const row = (title: string) =>
    page.locator('[data-testid="calendar-item-row"]').filter({ hasText: title });
  await expect(row("Live B")).toBeVisible();
  await markRow(row("Live B"));

  await completeFromOtherTab(context, listId, "Live A");

  await expect(row("Live A").locator('input[type="checkbox"]')).toBeChecked();
  expect(await isMarked(row("Live B"))).toBe(true);
});

test("Tag detail: a live update keeps untouched rows", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live Tag");
  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="new-tag-input"]').fill("Live");
  await page.locator('[data-testid="create-tag-btn"]').click();
  await expect(page.locator('[data-testid="tag-item"]').filter({ hasText: "Live" })).toBeVisible();

  const listId = await scheduleListWithItems(page, "Live Tag List");
  for (const title of ["Live A", "Live B", "Live C"]) {
    await page.goto(`/lists/${listId}`);
    await waitForHydration(page);
    await page.locator('[data-testid="item-title-link"]').filter({ hasText: title }).click();
    await page.waitForURL(/\/items\//);
    await waitForHydration(page);
    await page.locator('[data-testid="tag-add-btn"]').first().click();
    await page.locator('[data-testid="tag-dropdown-option"]').filter({ hasText: "Live" }).first().click();
    await expect(page.locator(".tag-badge").filter({ hasText: "Live" }).first()).toBeVisible();
  }

  await page.goto("/tags");
  await waitForHydration(page);
  await page.locator('[data-testid="tag-link"]').filter({ hasText: "Live" }).first().click();
  await page.waitForURL(/\/tags\/.+/);
  await waitForHydration(page);
  const row = (title: string) =>
    page.locator('[data-testid="tag-item-link"]').filter({ hasText: title });
  await expect(row("Live B")).toBeVisible();
  await markRow(row("Live B"));

  await completeFromOtherTab(context, listId, "Live A");

  await expect(row("Live A")).toHaveClass(/line-through/);
  expect(await isMarked(row("Live B"))).toBe(true);
});

test("List page: a live update keeps the add-item draft (#426)", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live Draft");
  const listId = await scheduleListWithItems(page, "Live Draft List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await page.locator('[data-testid="add-item-title"]').fill("Half typed");
  await page.locator('[data-testid="add-item-dates-toggle"]').click();
  await page.locator('[data-testid="add-item-deadline"]').fill(todayIso());

  // A change from another tab refetches and re-renders this list page.
  await completeFromOtherTab(context, listId, "Live A");
  await expect(
    page.locator('[data-testid="item-title-link"]').filter({ hasText: "Live A" }),
  ).toHaveClass(/line-through/);

  await expect(page.locator('[data-testid="add-item-title"]')).toHaveValue("Half typed");
  await expect(page.locator('[data-testid="add-item-deadline"]')).toHaveValue(todayIso());
});
