import { test, expect } from '@playwright/test';

// BASE62 char 'h' = 17. For a 5x5 puzzle, the maximum achievable target is
// (5-2)*(5-1)/2 = 6, so all-17 row/col targets are out of range. The puzzle
// is reachable through a share URL, so the WASM boundary must reject it
// without panicking and the UI must surface a readable error rather than
// freezing or throwing an uncaught exception.
const OUT_OF_RANGE_URL = '/?p=hhhhhhhhhh';

const walkthroughTabButton = (page: import('@playwright/test').Page) =>
  page.locator('nav.bottom-nav').getByRole('button', { name: 'Walkthrough' });

test('out-of-range targets in a share URL do not crash the app', async ({ page }) => {
  const consoleErrors: string[] = [];
  page.on('pageerror', (err) => consoleErrors.push(err.message));

  await page.goto(OUT_OF_RANGE_URL);

  // The puzzle grid still renders — the Play tab is independent of any
  // solver call, so a bad puzzle just shows up as bad targets.
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // No fatal error overlay was triggered: the bad input was rejected at the
  // WASM boundary rather than panicking past it.
  await expect(page.locator('[data-testid="error-overlay"]')).toHaveCount(0);
  expect(consoleErrors).toEqual([]);
});

test('walkthrough tab shows a friendly error for out-of-range targets', async ({ page }) => {
  await page.goto(OUT_OF_RANGE_URL);
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  await walkthroughTabButton(page).click();

  // The walkthrough catches the WASM error and surfaces the validation
  // message verbatim — "target N is out of range (max is M for size K)".
  const error = page.locator('[data-testid="walkthrough-error"]');
  await expect(error).toBeVisible();
  await expect(error).toContainText(/out of range/);
});
