import { test, expect, Page } from "@playwright/test";

import { createListAndOpen, registerAndLogin, waitForHydration } from "./helpers";

/** Set timezone via the settings page select. */
async function setTimezone(page: Page, tz: string) {
  await page.goto("/settings");
  await waitForHydration(page);
  const tzSelect = page.locator('[data-testid="settings-timezone-select"]');
  await tzSelect.waitFor();
  await tzSelect.selectOption(tz);
  // Give the server function a moment to persist
  await page.waitForTimeout(600);
}

/** Return the raw text of [data-testid="list-created-at"] for the given list. */
async function readCreatedAt(page: Page, listId: string): Promise<string> {
  await page.goto(`/lists/${listId}`);
  await waitForHydration(page);
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
  await registerAndLogin(page, context, "TZ User");
  const listId = await createListAndOpen(page, "TZ Test List");

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
  await registerAndLogin(page, context, "TZ User");
  const listId = await createListAndOpen(page, "TZ Test List");

  await setTimezone(page, "UTC");
  const text = await readCreatedAt(page, listId);

  expect(text).toMatch(/^Utworzono: \d{2}\.\d{2}\.\d{4} \d{2}:\d{2}$/);
});
