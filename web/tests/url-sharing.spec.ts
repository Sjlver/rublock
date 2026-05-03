import { test, expect } from '@playwright/test';

// A 5×5 puzzle: row_targets=[5,10,15,20,25], col_targets=[3,6,9,12,15].
// Old comma-separated format — used by early shared links and must keep working.
const COMMA_URL = '/?p=5,10,15,20,25,3,6,9,12,15';

// The same puzzle in base62 (serializePuzzleTargets output).
// BASE62 = '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'
// 5→'5', 10→'a', 15→'f', 20→'k', 25→'p', 3→'3', 6→'6', 9→'9', 12→'c', 15→'f'
const BASE62_PARAM = '5afkp369cf';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

test('old comma-separated URL format loads the correct puzzle', async ({ page }) => {
  await page.goto(COMMA_URL);
  await waitForReady(page);

  // The row targets should appear as row header cells in the puzzle grid.
  const rowTargets = page.locator('.app-shell table.puzzle th[scope="row"].target');
  await expect(rowTargets).toHaveText(['5', '10', '15', '20', '25']);
});

test('base62 URL format loads the correct puzzle', async ({ page }) => {
  await page.goto(`/?p=${BASE62_PARAM}`);
  await waitForReady(page);

  const rowTargets = page.locator('.app-shell table.puzzle th[scope="row"].target');
  await expect(rowTargets).toHaveText(['5', '10', '15', '20', '25']);

  const colTargets = page.locator('.app-shell table.puzzle th[scope="col"].target');
  await expect(colTargets).toHaveText(['3', '6', '9', '12', '15']);
});

test('Share button copies a valid URL to the clipboard', async ({ page, context }) => {
  await page.goto('/');
  await waitForReady(page);

  // Grant clipboard permissions so clipboard.readText() works in the test.
  await context.grantPermissions(['clipboard-read', 'clipboard-write']);

  // The share button in the page header has aria-label="Share".
  await page.getByRole('button', { name: 'Share' }).click();

  // The status row shows the toast "Link copied to clipboard" on success.
  await expect(page.locator('[role="status"]')).toContainText('Link copied to clipboard');

  // The clipboard should contain a URL with a ?p= parameter.
  const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboardText).toMatch(/\?p=[0-9a-zA-Z]+/);
});
