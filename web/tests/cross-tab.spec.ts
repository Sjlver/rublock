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

test('partial progress is preserved after switching to Create tab and back', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Click the first data cell in the puzzle grid (first <td> in the first body row).
  const firstCell = page.locator('.app-shell table.puzzle tbody tr:first-child td:nth-child(2)');
  await firstCell.click();

  // Enter digit 1 via keyboard (valid for 6×6 where digits are 1–4).
  await page.keyboard.press('1');

  // Verify the digit is displayed in the cell.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  // Switch to Create tab and back.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Create' }).click();
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Play', exact: true }).click();

  // The digit entered before must still be present.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('?t= URL param on load opens the correct tab directly', async ({ page }) => {
  await page.goto('/?t=create');
  // Create tab is active — wait for the page header on that tab.
  await expect(page.locator('.page-title')).toHaveText('Create');

  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Create');
});
