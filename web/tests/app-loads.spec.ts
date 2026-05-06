import { test, expect } from '@playwright/test';

test('app loads and the puzzle grid is visible on the Play tab', async ({ page }) => {
  await page.goto('/');

  // The puzzle grid is rendered once WASM has initialized and a puzzle has
  // been generated. Use it as the readiness signal.
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
});

test('Play tab shows the difficulty chip after load', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Generated puzzles are always Normal — see classify_puzzle wiring in
  // PlayTab.svelte. The chip lives in the page header's [role="status"].
  await expect(page.locator('[role="status"]')).toHaveText('Normal');
});

test('Play tab is active by default', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Bottom nav: the active button has class "active" and contains the label "Play".
  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Play');
});
