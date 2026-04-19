import { test, expect, APIRequestContext, Page } from "@playwright/test";

const BASE_URL = "http://localhost:3000";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+tz+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function registerAndLogin(
  request: APIRequestContext,
  page: Page,
  email: string
) {
  const res = await request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "TZ User", email, password: PASSWORD },
  });
  expect(res.ok()).toBeTruthy();

  await page.goto("/login");
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);
  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);
}

/** Create a list through the home page UI and return its ID. */
async function createListViaUi(page: Page): Promise<string> {
  await page.goto("/");
  const input = page.locator('input[placeholder="Nazwa listy..."]');
  await input.waitFor();
  await input.fill("TZ Test List");
  await page.locator('button:has-text("Utwórz")').first().click();

  // Wait for the list link to appear on the page
  const listLink = page.locator('a[href^="/lists/"]').first();
  await listLink.waitFor({ timeout: 5000 });
  const href = await listLink.getAttribute("href");
  return href!.replace("/lists/", "");
}

/** Set timezone via the settings page select. */
async function setTimezone(page: Page, tz: string) {
  await page.goto("/settings");
  // Timezone select is the second <select> (language is first)
  const tzSelect = page.locator("select").nth(1);
  await tzSelect.waitFor();
  await tzSelect.selectOption(tz);
  // Give the server function a moment to persist
  await page.waitForTimeout(600);
}

/** Return the raw text of [data-testid="list-created-at"] for the given list. */
async function readCreatedAt(page: Page, listId: string): Promise<string> {
  await page.goto(`/lists/${listId}`);
  const el = page.locator('[data-testid="list-created-at"]');
  await el.waitFor();
  return el.innerText();
}

/** Parse "Utworzono: DD.MM.YYYY HH:MM" → minutes since epoch. */
function parseMinutes(text: string): number {
  const m = text.match(/(\d{2})\.(\d{2})\.(\d{4}) (\d{2}):(\d{2})/);
  if (!m) throw new Error(`Cannot parse created-at text: "${text}"`);
  const [, dd, mm, yyyy, HH, MM] = m.map(Number);
  return Date.UTC(yyyy, mm - 1, dd, HH, MM) / 60_000;
}

test("changing timezone shifts the displayed creation time correctly", async ({
  page,
  context,
}) => {
  const email = uniqueEmail();
  await registerAndLogin(context.request, page, email);

  const listId = await createListViaUi(page);

  // --- UTC ---
  await setTimezone(page, "UTC");
  const textUtc = await readCreatedAt(page, listId);

  // --- Europe/Warsaw (UTC+1 winter / UTC+2 summer) ---
  await setTimezone(page, "Europe/Warsaw");
  const textWarsaw = await readCreatedAt(page, listId);

  expect(textUtc).not.toBe(textWarsaw);

  const diffMinutes = parseMinutes(textWarsaw) - parseMinutes(textUtc);
  // Warsaw is always 60 or 120 minutes ahead of UTC
  expect([60, 120]).toContain(diffMinutes);
});

test("creation time matches expected UTC format when timezone is UTC", async ({
  page,
  context,
}) => {
  const email = uniqueEmail();
  await registerAndLogin(context.request, page, email);

  const listId = await createListViaUi(page);

  await setTimezone(page, "UTC");
  const text = await readCreatedAt(page, listId);

  expect(text).toMatch(/^Utworzono: \d{2}\.\d{2}\.\d{4} \d{2}:\d{2}$/);
});
