import { test, expect } from '@playwright/test';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

test('Print tab has an independent size selector from Play', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Default size on the Play tab is 6.
  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('6×6');

  // Change Play tab size to 5.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('5×5');

  // Switch to Print tab — its size is independent and defaults to 6×6.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Print' }).click();
  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('6×6');
});

test('partial progress is preserved after switching to Solve tab and back', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Click the first data cell in the puzzle grid (first <td> in the first body row).
  const firstCell = page.locator('.app-shell table.puzzle tbody tr:first-child td:nth-child(2)');
  await firstCell.click();

  // Enter digit 1 via keyboard (valid for 6×6 where digits are 1–4).
  await page.keyboard.press('1');

  // Verify the digit is displayed in the cell.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  // Switch to Solve tab and back.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Solve' }).click();
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Play', exact: true }).click();

  // The digit entered before must still be present.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('partial progress is preserved after peeking at solution on Solve tab', async ({ page }) => {
  // Load a specific puzzle so the solve input is predictable.
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);

  // Enter a value on the Play tab.
  const firstCell = page.locator('.app-shell table.puzzle tbody tr:first-child td:nth-child(2)');
  await firstCell.click();
  await page.keyboard.press('1');
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  // Switch to Solve tab and click Solve to peek at the full solution.
  // This calls setPuzzle(parsed, { preserveProgressIfSame: true }) internally.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Solve' }).click();
  await page.locator('.card').getByRole('button', { name: 'Solve' }).click();
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Return to Play tab — progress must be intact.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Play', exact: true }).click();
  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('?t= URL param on load opens the correct tab directly', async ({ page }) => {
  await page.goto('/?t=solve');
  // Solve tab is active — wait for the page header on that tab.
  await expect(page.locator('.page-title')).toHaveText('Solve');

  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Solve');
});
