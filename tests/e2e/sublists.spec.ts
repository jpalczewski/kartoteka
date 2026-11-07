import { test, expect, Page } from "@playwright/test";

import { createListAndOpen, registerAndLogin, waitForHydration } from "./helpers";

/** Open the "add sublist" form on the list page and create a sublist. */
async function createSublist(page: Page, name: string) {
  await page.locator('button:has-text("Dodaj podlistę")').click();
  const form = page.locator('[data-testid="add-sublist-input"]');
  await form.locator('input[type="text"]').fill(name);
  await form.locator("button").click();
}

/** The collapse section of the sublist with the given name. */
function sublistSection(page: Page, name: string) {
  return page.locator('[data-testid="sublist-section"]').filter({ hasText: name });
}

/** Add an item inside a sublist section and wait for it to appear. */
async function addSublistItem(page: Page, section: ReturnType<Page["locator"]>, title: string) {
  const form = section.locator('[data-testid="sublist-new-item"]');
  await form.locator('input[type="text"]').fill(title);
  await form.locator("button").click();
  await expect(section.getByText(title, { exact: true })).toBeVisible();
}

// ── Create sublist ────────────────────────────────────────────────────────────

test("create sublist appears in parent list", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Parent List");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await createSublist(page, "My Sublist");

  await expect(sublistSection(page, "My Sublist")).toBeVisible();
});

test("multiple sublists all appear", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Multi Sublist Parent");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);

  for (const name of ["Alpha", "Beta", "Gamma"]) {
    await createSublist(page, name);
    await expect(sublistSection(page, name)).toBeVisible();
  }

  await expect(page.locator('[data-testid="sublist-section"]')).toHaveCount(3);
});

// ── Add items to sublist ──────────────────────────────────────────────────────

test("add item to sublist shows it inside that section", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Item Parent");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await createSublist(page, "Sub With Item");

  const section = sublistSection(page, "Sub With Item");
  await expect(section).toBeVisible();
  await addSublistItem(page, section, "Subtask 1");

  await expect(section.getByText("Subtask 1", { exact: true })).toBeVisible();
});

test("sublist progress counter updates after adding item", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Progress Parent");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await createSublist(page, "Progress Sub");

  const section = sublistSection(page, "Progress Sub");
  await addSublistItem(page, section, "Task A");

  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("0/1");
});

// ── Toggle items in sublist ───────────────────────────────────────────────────

// APP BUG (#341): `SublistSection` renders `ItemRow` without `list_features`, so sublist items never
// get a completion checkbox (`ItemRow` only renders one for the `checklist` feature) even though
// the section tracks and displays a done/total progress counter. Toggle it from the sublist's own
// page, or extend `SublistSection` to pass the sublist's features.
test.fixme("toggling item in sublist updates progress counter", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Toggle Parent");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await createSublist(page, "Toggle Sub");

  const section = sublistSection(page, "Toggle Sub");
  await addSublistItem(page, section, "Check Me");
  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("0/1");

  await section.locator('[data-testid="item-toggle"]').first().click();
  await expect(section.locator('[data-testid="sublist-progress"]')).toContainText("1/1");
});

// ── Open sublist as full page ─────────────────────────────────────────────────

test("↗ link opens sublist as standalone list page", async ({ page, context }) => {
  await registerAndLogin(page, context, "Sub User");
  const listId = await createListAndOpen(page, "Nav Parent");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await createSublist(page, "Nav Sub");

  const section = sublistSection(page, "Nav Sub");
  await expect(section).toBeVisible();

  const href = await section.locator('[data-testid="sublist-open-link"]').getAttribute("href");
  await page.goto(href!);
  await waitForHydration(page);

  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Nav Sub");
});
