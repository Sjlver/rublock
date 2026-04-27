import { test, expect } from '@playwright/test';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });
}

test('switching to Solve tab adds ?t=solve to the URL', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();

  await expect(page).toHaveURL(/[?&]t=solve/);
});

test('switching back to Play tab removes ?t= from the URL', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();
  await expect(page).toHaveURL(/[?&]t=solve/);

  await page.locator('nav.tabs').getByRole('button', { name: 'Play' }).click();

  // Play is the default tab — syncUrl omits the ?t= param for it.
  await expect(page).not.toHaveURL(/[?&]t=/);
});

test('puzzle URL parameter is present in address bar after app loads', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // A randomly-generated puzzle should have been encoded as ?p=<base62>
  await expect(page).toHaveURL(/[?&]p=[0-9a-zA-Z]+/);
});

test('changing size on Play tab syncs to the Print tab dropdown', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Default size is 6; change it to 5 on the Play tab.
  await page.locator('#play-size').selectOption('5');

  // Switch to Print tab.
  await page.locator('nav.tabs').getByRole('button', { name: 'Print' }).click();

  // The Print tab size selector should now show 5 × 5 because selectedSize is
  // shared via bind:selectedSize in App.svelte.
  await expect(page.locator('#print-size')).toHaveValue('5');
});

test('partial progress is preserved after switching to Solve tab and back', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Click the first data cell in the puzzle grid (first <td> in the first body row).
  const firstCell = page.locator('.preview table.puzzle tbody tr:first-child td:nth-child(2)');
  await firstCell.click();

  // Enter digit 1 via keyboard (valid for 6×6 where digits are 1–4).
  await page.keyboard.press('1');

  // Verify the digit is displayed in the cell.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  // Switch to Solve tab and back.
  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();
  await page.locator('nav.tabs').getByRole('button', { name: 'Play' }).click();

  // The digit entered before must still be present.
  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('partial progress is preserved after peeking at solution on Solve tab', async ({ page }) => {
  // Load a specific puzzle so the solve input is predictable.
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);

  // Enter a value on the Play tab.
  const firstCell = page.locator('.preview table.puzzle tbody tr:first-child td:nth-child(2)');
  await firstCell.click();
  await page.keyboard.press('1');
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  // Switch to Solve tab and click Solve to peek at the full solution.
  // This calls setPuzzle(parsed, { preserveProgressIfSame: true }) internally.
  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();
  await page.locator('.panel-card').getByRole('button', { name: 'Solve' }).click();
  await expect(page.locator('.solve-result table.puzzle')).toBeVisible({ timeout: 10_000 });

  // Return to Play tab — progress must be intact.
  await page.locator('nav.tabs').getByRole('button', { name: 'Play' }).click();
  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('?t= URL param on load opens the correct tab directly', async ({ page }) => {
  await page.goto('/?t=solve');
  await waitForReady(page);

  await expect(page.locator('nav.tabs button.active')).toHaveText('Solve');
});
