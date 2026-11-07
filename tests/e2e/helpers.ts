import { expect, BrowserContext, Page } from "@playwright/test";

/** Base URL of the app under test; `E2E_PORT` lets parallel checkouts run side by side. */
export const BASE_URL = `http://localhost:${process.env.E2E_PORT || "3030"}`;
export const PASSWORD = "testpassword123";

/** A throwaway e-mail so every test starts from a clean account. */
export function uniqueEmail(prefix = "test"): string {
  return `test+${prefix}+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

/** Wait until the WASM bundle has hydrated the page (`[data-hydrated]` is set on `body`). */
export async function waitForHydration(page: Page): Promise<void> {
  await page.waitForSelector("[data-hydrated]");
}

/**
 * Register a fresh account through the REST endpoint, then log in through the UI.
 * Returns the e-mail address that was registered.
 */
export async function registerAndLogin(
  page: Page,
  context: BrowserContext,
  name = "E2E User",
): Promise<string> {
  const email = await registerAccount(context, name);
  await context.clearCookies();
  await login(page, email);
  return email;
}

/** Create an account through the REST endpoint; returns its e-mail address. */
export async function registerAccount(
  context: BrowserContext,
  name = "E2E User",
): Promise<string> {
  const email = uniqueEmail();
  const res = await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name, email, password: PASSWORD },
  });
  expect(res.ok()).toBeTruthy();
  return email;
}

/** Log in through the login form; resolves once the home page is loaded and hydrated. */
export async function login(page: Page, email: string, password = PASSWORD): Promise<void> {
  await page.goto("/login");
  await waitForHydration(page);
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', password);
  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);
  await waitForHydration(page);
}

/** Create a list from the home page form and wait for its card to appear. */
export async function createList(page: Page, name: string): Promise<void> {
  await page.goto("/");
  await waitForHydration(page);
  await createListOnCurrentPage(page, name);
}

/**
 * Create a list with the form that is already open on the current page — lets a test pick a
 * list type (or fill other options) before submitting.
 */
export async function createListOnCurrentPage(page: Page, name: string): Promise<void> {
  const form = page.locator('[data-testid="create-entity-input"]');
  await form.locator('input[type="text"]').fill(name);
  await form.locator("button").click();
  await listCard(page, name).waitFor({ timeout: 10_000 });
}

/**
 * The home page card of the list with the given name. A recently opened list is shown twice
 * ("recent" section and the root list), so this returns the first match.
 */
export function listCard(page: Page, name: string) {
  return page.locator('[data-testid="list-card"]').filter({ hasText: name }).first();
}

/** Open a list from the home page card, returning its id. */
export async function openList(page: Page, name: string): Promise<string> {
  const card = listCard(page, name);
  await card.waitFor({ timeout: 10_000 });
  await card.locator('[data-testid="list-card-title"]').click();
  await page.waitForURL(/\/lists\/.+/);
  // Wait for the list page to hydrate: a form submitted before that is a native submit
  // that reloads the page and silently drops the new item (#422).
  await waitForHydration(page);
  return page.url().split("/lists/")[1];
}

/** Create a list from the home page and open it, returning its id. */
export async function createListAndOpen(page: Page, name: string): Promise<string> {
  await createList(page, name);
  return openList(page, name);
}

/** Add an item to the list page that is currently open and wait for it to render. */
export async function addItem(page: Page, title: string): Promise<void> {
  await submitAddItem(page, title);
  await expect(page.getByText(title, { exact: true })).toBeVisible();
}

/**
 * Fill and submit the add-item form, retrying when the submit never reached the server.
 *
 * The list page renders under `<Suspense>`: a live update for the list (e.g. the echo of the
 * list we just created) refetches it and re-creates the whole page, including this form, so
 * text typed just before is lost and nothing is submitted (#426). Until the list page moves
 * to keyed rows and `<Transition>`, wait for the `create_item` request and retry if it
 * didn't happen. Optional `before` re-applies extra form input (e.g. dates) on each try.
 */
export async function submitAddItem(
  page: Page,
  title: string,
  before?: () => Promise<void>,
): Promise<void> {
  for (let attempt = 0; attempt < 3; attempt++) {
    if (before) await before();
    await page.locator('[data-testid="add-item-title"]').fill(title);
    const created = page
      .waitForRequest((r) => r.url().includes("/leptos/create_item"), { timeout: 3_000 })
      .then(() => true)
      .catch(() => false);
    await page.locator('[data-testid="add-item-form"] button[type="submit"]').click();
    if (await created) return;
  }
  throw new Error(`add-item form never submitted "${title}"`);
}

/** Open the actions dropdown of the list page and pick one of its entries. */
export async function listAction(page: Page, testid: string): Promise<void> {
  await page.locator('[data-testid="list-actions-btn"]').click();
  await page.locator(`[data-testid="${testid}"]`).click();
}

/** Accept the confirmation modal that destructive list actions open. */
export async function confirmModal(page: Page): Promise<void> {
  await page.locator('[data-testid="confirm-modal-confirm"]').click();
}
