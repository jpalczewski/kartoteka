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

/** Create a sublist on the open list page and add one item to it. */
async function addSublistWithItem(page: Page, name: string, itemTitle: string) {
  await page.locator('[data-testid="add-sublist-button"]').click();
  const form = page.locator('[data-testid="add-sublist-input"]');
  await form.locator('input[type="text"]').fill(name);
  await form.locator("button").click();
  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: name });
  await expect(section).toBeVisible();
  const newItem = section.locator('[data-testid="sublist-new-item"]');
  await newItem.locator('input[type="text"]').fill(itemTitle);
  await newItem.locator("button").click();
  await expect(section.getByText(itemTitle, { exact: true })).toBeVisible();
}

test("List page: a live update keeps rows, sublists, editors and inputs", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Live List");
  const listId = await scheduleListWithItems(page, "Live List Page");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await addSublistWithItem(page, "Live Sub", "Sub Item");

  // A fresh load, so the marks below sit on elements of the final page.
  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  const row = (title: string) =>
    page.locator('[data-testid="item-row"]').filter({ hasText: title });
  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Live Sub" });
  await expect(row("Live B")).toBeVisible();
  await expect(section.locator('[data-testid="item-row"]')).toHaveCount(1);

  // Row B with its description editor open and a draft typed into it.
  await row("Live B").locator('[data-testid="item-expand-btn"]').click();
  const editor = row("Live B").locator("textarea");
  await editor.fill("typing in row B");
  // The name editor, open with half a name.
  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-heading-input"]');
  await nameInput.fill("Half a name");
  // The sublist name input and the sublist's own add-item input, with text typed in.
  await page.locator('[data-testid="add-sublist-button"]').click();
  const sublistNameInput = page.locator('[data-testid="add-sublist-input"] input[type="text"]');
  await sublistNameInput.fill("Half a sublist");
  const sublistItemInput = section.locator('[data-testid="sublist-new-item"] input[type="text"]');
  await sublistItemInput.fill("Half a sub item");
  // The comment box.
  const comment = page.locator('textarea[placeholder="Dodaj komentarz..."]');
  await comment.fill("Half a comment");

  const marked: Record<string, Locator> = {
    "row B": row("Live B"),
    "row editor": editor,
    "sublist row": section.locator('[data-testid="item-row"]'),
    "sublist section": section,
    "name input": nameInput,
    "sublist name input": sublistNameInput,
    "sublist item input": sublistItemInput,
    "comment box": comment,
  };
  for (const locator of Object.values(marked)) await markRow(locator);

  // A change in the list and one in the sublist, from another tab.
  await completeFromOtherTab(context, listId, "Live A");
  await expect(row("Live A").locator('[data-testid="item-title-link"]')).toHaveClass(
    /line-through/,
  );
  await completeFromOtherTab(context, listId, "Sub Item");
  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("1/1");

  for (const [what, locator] of Object.entries(marked)) {
    expect(await isMarked(locator), `${what} is still the same element`).toBe(true);
  }
  await expect(editor).toHaveValue("typing in row B");
  await expect(nameInput).toHaveValue("Half a name");
  await expect(sublistNameInput).toHaveValue("Half a sublist");
  await expect(sublistItemInput).toHaveValue("Half a sub item");
  await expect(comment).toHaveValue("Half a comment");
});

test("List page: a live update keeps rows of a plain list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Live Plain");
  await page.goto("/");
  await waitForHydration(page);
  await createListOnCurrentPage(page, "Live Plain List");
  const listId = await openList(page, "Live Plain List");
  for (const title of ["Live A", "Live B"]) {
    await submitAddItem(page, title);
    await expect(page.getByText(title, { exact: true })).toBeVisible();
  }

  const row = (title: string) =>
    page.locator('[data-testid="item-row"]').filter({ hasText: title });
  await markRow(row("Live B"));
  await completeFromOtherTab(context, listId, "Live A");

  await expect(row("Live A").locator('[data-testid="item-title-link"]')).toHaveClass(
    /line-through/,
  );
  expect(await isMarked(row("Live B"))).toBe(true);
});
