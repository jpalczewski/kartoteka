import { test, expect } from "@playwright/test";

const BASE_URL = "http://localhost:3000";
const PASSWORD = "testpassword123";

function uniqueEmail() {
  return `test+lha+${Date.now()}+${Math.random().toString(36).slice(2)}@example.com`;
}

async function setup(page: any, context: any): Promise<string> {
  const email = uniqueEmail();
  await context.request.post(`${BASE_URL}/auth/register`, {
    headers: { "Content-Type": "application/json" },
    data: { name: "LHA User", email, password: PASSWORD },
  });
  await page.goto("/login");
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', PASSWORD);
  await Promise.all([
    page.waitForURL(`${BASE_URL}/`),
    page.click('button[type="submit"]'),
  ]);

  // Create a list and return its ID
  const input = page.locator('input[placeholder="Nazwa listy..."]');
  await input.waitFor();
  await input.fill("Header Test List");
  await page.locator('button:has-text("Utwórz")').first().click();
  const link = page.locator('a[href^="/lists/"]').first();
  await link.waitFor({ timeout: 5000 });
  return (await link.getAttribute("href"))!.replace("/lists/", "");
}

// ── Happy flows ──────────────────────────────────────────────────────────────

test("rename list via inline click — Enter saves", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-input"]');
  await nameInput.waitFor();
  await nameInput.fill("Renamed List");
  await page.keyboard.press("Enter");

  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Renamed List");
});

test("Escape cancels rename without saving", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-input"]');
  await nameInput.waitFor();
  await nameInput.fill("Should Not Save");
  await page.keyboard.press("Escape");

  await expect(page.locator('[data-testid="list-name-heading"]')).toBeVisible();
  await expect(page.locator('[data-testid="list-name-heading"]')).toHaveText("Header Test List");
});

test("pin then unpin via dropdown", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  // Pin
  await page.locator('[data-testid="list-actions-btn"]').click();
  await page.locator('[data-testid="action-pin"]').click();

  // After pin: reload page and verify label changed to Odepnij
  await page.goto(`/lists/${listId}`);
  await page.locator('[data-testid="list-actions-btn"]').click();
  await expect(page.locator('[data-testid="action-pin"]')).toContainText("Odepnij");

  // Unpin
  await page.locator('[data-testid="action-pin"]').click();
  await page.goto(`/lists/${listId}`);
  await page.locator('[data-testid="list-actions-btn"]').click();
  await expect(page.locator('[data-testid="action-pin"]')).toContainText("Przypnij");
});

test("reset marks completed items as incomplete", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  // Add item and complete it
  await page.fill('input[placeholder="Nowy element..."]', "Item to reset");
  await page.click('button:has-text("Dodaj")');
  const itemCheckbox = page.locator('input[type="checkbox"]').first();
  await itemCheckbox.waitFor();
  await itemCheckbox.click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("1/1");

  // Reset via dropdown
  await page.locator('[data-testid="list-actions-btn"]').click();
  await page.locator('[data-testid="action-reset"]').click();
  await expect(page.locator('[data-testid="completion-count"]')).toContainText("0/1");
});

test("archive navigates to home", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  await page.locator('[data-testid="list-actions-btn"]').click();
  await page.locator('[data-testid="action-archive"]').click();
  await page.waitForURL(`${BASE_URL}/`, { timeout: 5000 });

  expect(page.url()).toBe(`${BASE_URL}/`);
});

// ── Sad flows ────────────────────────────────────────────────────────────────

test("rename with empty/whitespace name shows error toast", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  await page.locator('[data-testid="list-name-heading"]').click();
  const nameInput = page.locator('[data-testid="list-name-input"]');
  await nameInput.waitFor();
  await nameInput.fill("   ");
  await page.keyboard.press("Enter");

  // Server returns error → toast appears; heading stays hidden until refresh
  // Check the input is still visible (no navigation away) and error appears
  await expect(page.locator('.toast')).toBeVisible({ timeout: 3000 });
});

test("reset on list with no items is a no-op (no error)", async ({ page, context }) => {
  const listId = await setup(page, context);
  await page.goto(`/lists/${listId}`);

  // No items — just click reset, should not crash
  await page.locator('[data-testid="list-actions-btn"]').click();
  await page.locator('[data-testid="action-reset"]').click();

  // Page should still be on the list (no error navigation)
  await expect(page.locator('[data-testid="list-name-heading"]')).toBeVisible({ timeout: 3000 });
  // No toast error
  await expect(page.locator('.toast.alert-error')).not.toBeVisible();
});
