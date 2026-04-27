import { test, expect } from '@playwright/test';

test('app loads and WASM initializes to Ready status', async ({ page }) => {
  await page.goto('/');

  // The status span transitions from "Loading…" to "Ready" once WASM is up.
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });
});

test('puzzle grid is visible on the Play tab after load', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });

  // The grid is a <table class="puzzle"> rendered by PuzzleGrid.
  await expect(page.locator('.preview table.puzzle')).toBeVisible();
});

test('Play tab is active by default', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });

  await expect(page.locator('nav.tabs button.active')).toHaveText('Play');
});
