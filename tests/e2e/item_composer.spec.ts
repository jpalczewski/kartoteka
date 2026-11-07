import { test, expect, BrowserContext, Page } from "@playwright/test";

import {
  BASE_URL,
  createListAndOpen,
  registerAndLogin,
  submitAddItem,
  waitForHydration,
} from "./helpers";

// The add-item composer (single-line quick add + "more options") and the single-field
// editors of the expanded item row (status, recurrence, estimate).

const ALL_FEATURES = ["checklist", "deadlines", "quantity", "time_tracking"];

/** Create a list with the given features over the REST API and open its page. */
async function openFeatureList(
  page: Page,
  context: BrowserContext,
  name: string,
  features = ALL_FEATURES,
): Promise<string> {
  const res = await context.request.post(`${BASE_URL}/api/lists`, {
    headers: { "Content-Type": "application/json" },
    data: { name, features },
  });
  expect(res.ok(), await res.text()).toBeTruthy();
  const { id } = await res.json();
  await page.goto(`/lists/${id}`);
  await waitForHydration(page);
  return id;
}

/** Tomorrow as YYYY-MM-DD, so a recurring item never counts as overdue. */
function tomorrowIso(): string {
  return new Date(Date.now() + 86_400_000).toISOString().slice(0, 10);
}

/** Pick an emoji in the icon picker that the given button opens. */
async function pickEmoji(scope: Page | ReturnType<Page["locator"]>, button: string, emoji: string) {
  await scope.locator(`[data-testid="${button}"]`).click();
  await scope.locator('[data-testid="icon-picker-tab-emoji"]').click();
  await scope.locator(`[data-testid="icon-picker-emoji"][aria-label="${emoji}"]`).click();
}

const row = (page: Page, title: string) =>
  page.locator('[data-testid="item-row"]').filter({ hasText: title });

test("Enter adds the item and clears the input", async ({ page, context }) => {
  await registerAndLogin(page, context, "Composer Enter");
  await createListAndOpen(page, "Enter List");

  const title = page.locator('[data-testid="add-item-title"]');
  await title.fill("Milk");
  await title.press("Enter");

  await expect(page.getByText("Milk", { exact: true })).toBeVisible();
  await expect(title).toHaveValue("");
});

test("a server error keeps the text and shows an error", async ({ page, context }) => {
  await registerAndLogin(page, context, "Composer Error");
  await createListAndOpen(page, "Error List");

  const title = page.locator('[data-testid="add-item-title"]');
  await page.route(/\/leptos\/create_item/, (route) =>
    route.fulfill({ status: 500, contentType: "text/plain", body: "boom" }),
  );
  await title.fill("Bread");
  await title.press("Enter");

  await expect(page.locator(".alert-error")).toBeVisible();
  await expect(title).toHaveValue("Bread");
  await expect(row(page, "Bread")).toHaveCount(0);

  // Once the server answers again, the same text goes through and the input clears.
  await page.unroute(/\/leptos\/create_item/);
  await title.press("Enter");
  await expect(page.getByText("Bread", { exact: true })).toBeVisible();
  await expect(title).toHaveValue("");
});

test("the sublist quick add keeps its text on a server error", async ({ page, context }) => {
  await registerAndLogin(page, context, "Composer Sublist");
  await openFeatureList(page, context, "Sublist Parent", ["checklist"]);

  await page.locator('[data-testid="add-sublist-button"]').click();
  const form = page.locator('[data-testid="add-sublist-input"]');
  await form.locator('input[type="text"]').fill("Child");
  await form.locator("button").click();
  const section = page.locator('[data-testid="sublist-section"]').filter({ hasText: "Child" });
  await expect(section).toBeVisible();

  const input = section.locator('[data-testid="sublist-new-item"] input[type="text"]');
  await page.route(/\/leptos\/create_item/, (route) =>
    route.fulfill({ status: 500, contentType: "text/plain", body: "boom" }),
  );
  await input.fill("Sub Eggs");
  await input.press("Enter");
  await expect(page.locator(".alert-error")).toBeVisible();
  await expect(input).toHaveValue("Sub Eggs");

  await page.unroute(/\/leptos\/create_item/);
  await input.press("Enter");
  await expect(section.getByText("Sub Eggs", { exact: true })).toBeVisible();
  await expect(input).toHaveValue("");
});

test("the expanded composer adds description, quantity, deadline, recurrence and estimate", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Composer More");
  const listId = await openFeatureList(page, context, "Full List");

  const deadline = tomorrowIso();
  await submitAddItem(page, "Detailed", async () => {
    if (!(await page.locator('[data-testid="composer-description"]').isVisible())) {
      await page.locator('[data-testid="composer-more-toggle"]').click();
    }
    await page.locator('[data-testid="composer-description"]').fill("with notes");
    await page.locator('[data-testid="composer-quantity"]').fill("3");
    await page.locator('[data-testid="composer-unit"]').fill("l");
    await page.locator('[data-testid="add-item-deadline"]').fill(deadline);
    await page.locator('[data-testid="composer-recurrence-input"]').fill("1w");
    await page.locator('[data-testid="composer-estimate-input"]').fill("1h30m");
    await pickEmoji(page, "composer-icon-button", "🚀");
  });

  const created = row(page, "Detailed");
  await expect(created).toBeVisible();
  // The form is empty and collapsed again.
  await expect(page.locator('[data-testid="add-item-title"]')).toHaveValue("");
  await expect(page.locator('[data-testid="composer-more"]')).toHaveCount(0);

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  await expect(created.locator('[data-testid="item-estimate-chip"]')).toHaveText("1h 30m");
  await expect(created.locator('[data-testid="item-recurrence-chip"]')).toHaveText("1w");
  await expect(created).toContainText("3 l");
  await expect(created).toContainText("with notes");
  await expect(created).toContainText("🚀");
});

test("an invalid estimate in the composer is reported and keeps the draft", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Composer Invalid");
  await openFeatureList(page, context, "Invalid List");

  await page.locator('[data-testid="composer-more-toggle"]').click();
  await page.locator('[data-testid="composer-estimate-input"]').fill("soon");
  const title = page.locator('[data-testid="add-item-title"]');
  await title.fill("Never added");
  await title.press("Enter");

  await expect(page.locator(".alert-error")).toBeVisible();
  await expect(title).toHaveValue("Never added");
  await expect(row(page, "Never added")).toHaveCount(0);
});

test("the row editor sets status, recurrence and estimate, and they persist", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Editor Fields");
  const listId = await openFeatureList(page, context, "Editor List");

  await submitAddItem(page, "Edit me", async () => {
    if (!(await page.locator('[data-testid="add-item-deadline"]').isVisible())) {
      await page.locator('[data-testid="composer-more-toggle"]').click();
    }
    await page.locator('[data-testid="add-item-deadline"]').fill(tomorrowIso());
  });
  const item = row(page, "Edit me");
  await expect(item).toBeVisible();

  await item.locator('[data-testid="item-expand-btn"]').click();
  await item.locator('[data-testid="item-status-select"]').selectOption("in_progress");
  await expect(item).toContainText("In progress");

  await item.locator('[data-testid="item-recurrence-input"]').fill("2w");
  await item.locator('[data-testid="item-recurrence-input-mode"]').selectOption("floating");
  await item.locator('[data-testid="item-recurrence-save"]').click();
  await expect(item.locator('[data-testid="item-recurrence-chip"]')).toHaveText("2w");

  await item.locator('[data-testid="item-estimate-input"]').fill("45m");
  await item.locator('[data-testid="item-estimate-save"]').click();
  await expect(item.locator('[data-testid="item-estimate-chip"]')).toHaveText("45m");

  await pickEmoji(item, "item-icon-button", "🚀");
  await expect(item).toContainText("🚀");

  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
  const reloaded = row(page, "Edit me");
  await expect(reloaded).toContainText("In progress");
  await expect(reloaded.locator('[data-testid="item-recurrence-chip"]')).toHaveText("2w");
  await expect(reloaded.locator('[data-testid="item-estimate-chip"]')).toHaveText("45m");
  await expect(reloaded).toContainText("🚀");

  // The editor starts from what is stored.
  await reloaded.locator('[data-testid="item-expand-btn"]').click();
  await expect(reloaded.locator('[data-testid="item-status-select"]')).toHaveValue("in_progress");
  await expect(reloaded.locator('[data-testid="item-recurrence-input"]')).toHaveValue("2w");
  await expect(reloaded.locator('[data-testid="item-recurrence-input-mode"]')).toHaveValue(
    "floating",
  );
  await expect(reloaded.locator('[data-testid="item-estimate-input"]')).toHaveValue("45m");

  // Clearing both removes the chips again.
  await reloaded.locator('[data-testid="item-recurrence-input"]').fill("");
  await reloaded.locator('[data-testid="item-recurrence-save"]').click();
  await reloaded.locator('[data-testid="item-estimate-input"]').fill("");
  await reloaded.locator('[data-testid="item-estimate-save"]').click();
  await expect(reloaded.locator('[data-testid="item-recurrence-chip"]')).toHaveCount(0);
  await expect(reloaded.locator('[data-testid="item-estimate-chip"]')).toHaveCount(0);

  // The default icon removes the item's icon.
  await reloaded.locator('[data-testid="item-icon-button"]').click();
  await reloaded.locator('[data-testid="icon-picker-default"]').click();
  await expect(reloaded).not.toContainText("🚀");
});

test("setting the status to done completes the item", async ({ page, context }) => {
  await registerAndLogin(page, context, "Editor Done");
  await openFeatureList(page, context, "Done List");

  const title = page.locator('[data-testid="add-item-title"]');
  await title.fill("Finish me");
  await title.press("Enter");
  const item = row(page, "Finish me");
  await expect(item).toBeVisible();

  await item.locator('[data-testid="item-expand-btn"]').click();
  await item.locator('[data-testid="item-status-select"]').selectOption("done");
  await expect(item.locator('[data-testid="item-title-link"]')).toHaveClass(/line-through/);
  await expect(item.locator('[data-testid="item-toggle"]')).toBeChecked();
});

test("a recurrence needs a date: the editor says so and does not save", async ({
  page,
  context,
}) => {
  await registerAndLogin(page, context, "Editor Needs Date");
  await openFeatureList(page, context, "No Date List");

  const title = page.locator('[data-testid="add-item-title"]');
  await title.fill("Undated");
  await title.press("Enter");
  const item = row(page, "Undated");
  await expect(item).toBeVisible();

  await item.locator('[data-testid="item-expand-btn"]').click();
  await item.locator('[data-testid="item-recurrence-input"]').fill("1w");
  await expect(item.locator('[data-testid="item-recurrence-issue"]')).toBeVisible();
  await expect(item.locator('[data-testid="item-recurrence-save"]')).toBeDisabled();
});

test("lists without the time tracking feature offer no estimate", async ({ page, context }) => {
  await registerAndLogin(page, context, "Editor No Time");
  await openFeatureList(page, context, "Plain List", ["checklist"]);

  const title = page.locator('[data-testid="add-item-title"]');
  await title.fill("Plain");
  await title.press("Enter");
  const item = row(page, "Plain");
  await expect(item).toBeVisible();

  await page.locator('[data-testid="composer-more-toggle"]').click();
  await expect(page.locator('[data-testid="composer-estimate-input"]')).toHaveCount(0);
  await item.locator('[data-testid="item-expand-btn"]').click();
  await expect(item.locator('[data-testid="item-status-select"]')).toBeVisible();
  await expect(item.locator('[data-testid="item-estimate-input"]')).toHaveCount(0);
  await expect(item.locator('[data-testid="item-recurrence-input"]')).toHaveCount(0);
});
