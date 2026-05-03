import { test, expect } from '@playwright/test';

test('app loads and the puzzle grid is visible on the Play tab', async ({ page }) => {
  await page.goto('/');

  // The puzzle grid is rendered once WASM has initialized and a puzzle has
  // been generated. Use it as the readiness signal.
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
});

test('Play tab shows "Ready" status after load', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // PageHeader renders a [role="status"] element with the current tab status.
  await expect(page.locator('[role="status"]')).toHaveText('Ready');
});

test('Play tab is active by default', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Bottom nav: the active button has class "active" and contains the label "Play".
  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Play');
});
