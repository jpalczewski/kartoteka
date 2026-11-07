import { test, expect, Locator, Page } from "@playwright/test";

import { listCard, registerAndLogin, waitForHydration } from "./helpers";

// Create tabs (List / Folder / Project) and the icon + colour picker on home cards (#40).
// Copy assertions accept both locales; the UI language depends on the browser.

const RED = "#ef4444";
const BLUE = "#3b82f6";

const form = (page: Page) => page.locator('[data-testid="create-entity-input"]');
const tab = (page: Page, kind: "list" | "folder" | "project") =>
  page.locator(`[data-testid="create-entity-tab-${kind}"]`);

function containerCard(page: Page, name: string) {
  return page.locator('[data-testid="container-card"]').filter({ hasText: name }).first();
}

async function create(page: Page, name: string): Promise<void> {
  await form(page).locator('input[type="text"]').fill(name);
  await form(page).locator("button").click();
}

/** Inner SVG markup without the `<!---->` markers Leptos leaves between nodes. */
async function svgMarkup(svg: Locator): Promise<string> {
  return (await svg.innerHTML()).replace(/<!--.*?-->/g, "");
}

/** Open the card menu, pick "Appearance…" and return the dialog. */
async function openAppearance(page: Page, card: ReturnType<typeof listCard>, kind: "list" | "container") {
  await card.locator(`[data-testid="${kind}-card-menu"]`).click();
  await card.locator(`[data-testid="${kind}-action-appearance"]`).click();
  const dialog = page.locator('[data-testid="appearance-dialog"]');
  await expect(dialog).toBeVisible();
  return dialog;
}

test("create tabs show the active one and create each kind", async ({ page, context }) => {
  await registerAndLogin(page, context, "Appearance Tabs");

  // The active tab is exposed to assistive tech and styled by DaisyUI's `tab-active`.
  await expect(page.getByRole("tablist")).toBeVisible();
  await expect(tab(page, "list")).toHaveAttribute("aria-selected", "true");
  await expect(tab(page, "list")).toHaveClass(/tab-active/);
  await expect(tab(page, "folder")).toHaveAttribute("aria-selected", "false");

  // Keyboard: arrows move the selection and focus.
  await tab(page, "list").focus();
  await page.keyboard.press("ArrowRight");
  await expect(tab(page, "folder")).toHaveAttribute("aria-selected", "true");
  await expect(tab(page, "folder")).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(tab(page, "project")).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("ArrowRight");
  await expect(tab(page, "list")).toHaveAttribute("aria-selected", "true");

  await create(page, "Tabs List");
  await listCard(page, "Tabs List").waitFor();

  await tab(page, "folder").click();
  await expect(tab(page, "folder")).toHaveClass(/tab-active/);
  await create(page, "Tabs Folder");
  const folder = containerCard(page, "Tabs Folder");
  await folder.waitFor();
  await expect(folder.locator('[data-testid="container-status-chip"]')).toHaveCount(0);

  await tab(page, "project").click();
  await create(page, "Tabs Project");
  const project = containerCard(page, "Tabs Project");
  await project.waitFor();
  await expect(project.locator('[data-testid="container-status-chip"]')).toHaveText(
    /active|aktywny/i,
  );
});

test("set icon and colour on a list and a folder, survives reload", async ({ page, context }) => {
  await registerAndLogin(page, context, "Appearance Picker");

  await create(page, "Styled List");
  const list = listCard(page, "Styled List");
  await list.waitFor();
  await tab(page, "folder").click();
  await create(page, "Styled Folder");
  const folder = containerCard(page, "Styled Folder");
  await folder.waitFor();

  // ── List: lucide icon + palette colour ──────────────────────────────────────
  let dialog = await openAppearance(page, list, "list");
  await dialog.locator('[data-testid="icon-picker-search"]').fill("rock");
  const rocket = dialog.locator('[data-testid="icon-picker-icon-rocket"]');
  await expect(rocket).toBeVisible();
  await expect(dialog.locator('[data-testid="icon-picker-icon-star"]')).toHaveCount(0);
  const rocketSvg = await svgMarkup(rocket.locator("svg"));
  await rocket.click();
  await expect(rocket).toHaveAttribute("aria-pressed", "true");
  await dialog.locator(`[data-testid="appearance-color-picker"] button[aria-label="${RED}"]`).click();
  await dialog.locator('[data-testid="appearance-save"]').click();
  await expect(dialog).toHaveCount(0);

  const listIcon = () => listCard(page, "Styled List").locator("svg.lucide").first();
  await expect(listIcon()).toHaveAttribute("stroke", RED);
  expect(await svgMarkup(listIcon())).toBe(rocketSvg);

  // ── Folder: emoji + another colour ─────────────────────────────────────────
  dialog = await openAppearance(page, folder, "container");
  await dialog.locator('[data-testid="icon-picker-tab-emoji"]').click();
  await expect(dialog.locator('[data-testid="icon-picker-tab-emoji"]')).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await dialog.locator('[data-testid="icon-picker-emoji-input"]').fill("🦊");
  await dialog.locator('[data-testid="icon-picker-emoji-use"]').click();
  await dialog.locator(`[data-testid="appearance-color-picker"] button[aria-label="${BLUE}"]`).click();
  await dialog.locator('[data-testid="appearance-save"]').click();
  await expect(dialog).toHaveCount(0);

  const folderEmoji = () =>
    containerCard(page, "Styled Folder").locator("span", { hasText: "🦊" }).first();
  await expect(folderEmoji()).toBeVisible();
  await expect(folderEmoji()).toHaveCSS("color", "rgb(59, 130, 246)");

  // ── Reload: both are still there ───────────────────────────────────────────
  await page.reload();
  await waitForHydration(page);
  await expect(listIcon()).toHaveAttribute("stroke", RED);
  expect(await svgMarkup(listIcon())).toBe(rocketSvg);
  await expect(folderEmoji()).toBeVisible();
  await expect(folderEmoji()).toHaveCSS("color", "rgb(59, 130, 246)");

  // ── Clear: back to the default icon and neutral colour ─────────────────────
  dialog = await openAppearance(page, listCard(page, "Styled List"), "list");
  await dialog.locator('[data-testid="icon-picker-default"]').click();
  await dialog.locator('[data-testid="appearance-color-none"]').click();
  await dialog.locator('[data-testid="appearance-save"]').click();
  await expect(dialog).toHaveCount(0);
  await expect(listIcon()).toHaveAttribute("stroke", "currentColor");
  expect(await svgMarkup(listIcon())).not.toBe(rocketSvg);

  // Cancel discards the draft.
  dialog = await openAppearance(page, listCard(page, "Styled List"), "list");
  await dialog.locator(`[data-testid="appearance-color-picker"] button[aria-label="${RED}"]`).click();
  await dialog.locator('[data-testid="appearance-cancel"]').click();
  await expect(dialog).toHaveCount(0);
  await expect(listIcon()).toHaveAttribute("stroke", "currentColor");
  // Still on the home page: the dialog never triggered the card's open-list click.
  await expect(page).toHaveURL(/\/$/);
});
