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

  // Generated puzzles are solvable by propagation alone (see classify_puzzle
  // wiring in PlayTab.svelte and the `generate_puzzle_n` loop in src/wasm.rs),
  // so the chip is one of the propagation-only difficulty labels. The chip
  // lives in the page header's [role="status"].
  await expect(page.locator('[role="status"]')).toHaveText(/^(Easy|Medium|Challenging)$/);
});

test('Play tab is active by default', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Bottom nav: the active button has class "active" and contains the label "Play".
  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Play');
});
