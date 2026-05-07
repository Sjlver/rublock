import { test, expect } from '@playwright/test';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

async function openCreateTab(page: import('@playwright/test').Page) {
  await page.goto('/');
  await waitForReady(page);
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Create' }).click();
  await expect(page.locator('.page-title')).toHaveText('Create');
}

test('Create tab opens with the same puzzle the Play tab is showing', async ({ page }) => {
  // Load a specific puzzle so the seeded targets are predictable.
  // row=[7,0,4,7,4,3], col=[0,10,4,1,0,0]
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);

  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Create' }).click();
  await expect(page.locator('.page-title')).toHaveText('Create');

  const rowTargets = page.locator('.app-shell table.puzzle th[scope="row"].target');
  await expect(rowTargets).toHaveText(['7', '0', '4', '7', '4', '3']);
});

test('selecting a target and tapping a value writes that value', async ({ page }) => {
  await openCreateTab(page);

  // First row target.
  const firstRowTarget = page.locator('.app-shell table.puzzle th[scope="row"].target').first();
  await firstRowTarget.click();
  await expect(firstRowTarget).toHaveClass(/target-selected/);

  // Tap value "3" — the value strip lists 0..=max for the current size.
  await page.getByRole('button', { name: 'Set target to 3' }).click();
  await expect(firstRowTarget).toHaveText('3');
});

test('switching size in Create reuses the Play puzzle for that size', async ({ page }) => {
  await openCreateTab(page);

  // Default Play size is 6. Read its row targets first.
  const sixTargets = await page
    .locator('.app-shell table.puzzle th[scope="row"].target')
    .allTextContents();

  // Switch to size 5 — Play has not visited 5 yet, so Create lazily generates one.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target')).toHaveCount(5);
  const fiveTargets = await page
    .locator('.app-shell table.puzzle th[scope="row"].target')
    .allTextContents();

  // Switch back to size 6 — the original puzzle should still be there.
  await page.locator('.size-selector .size-btn', { hasText: '6×6' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target')).toHaveText(
    sixTargets
  );

  // Switch to Play tab on size 5: it should show the puzzle Create just
  // materialized (i.e. the same row targets as `fiveTargets`).
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Play', exact: true }).click();
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target')).toHaveText(
    fiveTargets
  );
});

test('Create tab preserves drafts when switching sizes', async ({ page }) => {
  await openCreateTab(page);

  // Edit row target 0 on size 6.
  await page.locator('.app-shell table.puzzle th[scope="row"].target').first().click();
  await page.getByRole('button', { name: 'Set target to 2' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target').first()).toHaveText(
    '2'
  );

  // Go to size 5, edit a target there.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await page.locator('.app-shell table.puzzle th[scope="row"].target').first().click();
  await page.getByRole('button', { name: 'Set target to 3' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target').first()).toHaveText(
    '3'
  );

  // Back to 6: the earlier edit is preserved.
  await page.locator('.size-selector .size-btn', { hasText: '6×6' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target').first()).toHaveText(
    '2'
  );

  // And size 5 keeps its edit too.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target').first()).toHaveText(
    '3'
  );
});

test('share copies a URL for valid puzzles', async ({ page, context }) => {
  await openCreateTab(page);

  await context.grantPermissions(['clipboard-read', 'clipboard-write']);

  // Default seeded draft mirrors the Play puzzle, which is unique by
  // construction (loadRandomPuzzle generates uniquely-solvable puzzles).
  await page.getByRole('button', { name: 'Share' }).click();

  await expect(page.locator('[role="status"]')).toContainText('Link copied');

  const clipboardText = await page.evaluate(() => navigator.clipboard.readText());
  expect(clipboardText).toMatch(/\?p=[0-9a-zA-Z]+/);
});

test('selecting a target highlights values that lead to valid puzzles', async ({ page }) => {
  // Hard-coded size-6 puzzle: row=[7,0,4,7,4,3], col=[0,10,4,1,0,0].
  // It has a unique solution, so keeping row 0 at 7 stays valid.
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Create' }).click();
  await expect(page.locator('.page-title')).toHaveText('Create');

  // Select the first row target.
  await page.locator('.app-shell table.puzzle th[scope="row"].target').first().click();

  const valueButtons = page.locator('.create-values .create-value-btn');
  // Size 6 → max target is (6-2)*(6-1)/2 = 10, so 11 buttons (0..10).
  await expect(valueButtons).toHaveCount(11);

  // Classification runs sequentially 0..max, so once the last button is
  // marked classified every other one is too.
  await expect(valueButtons.last()).toHaveAttribute('data-classified', 'true');

  // Buttons stay tappable regardless of validity.
  for (let i = 0; i < 11; i++) {
    await expect(valueButtons.nth(i)).toBeEnabled();
  }

  // The current target value (7) is part of a uniquely-solvable puzzle, so
  // it must be marked valid.
  await expect(page.getByRole('button', { name: 'Set target to 7' })).toHaveAttribute(
    'data-valid',
    'true'
  );

  // At least one other value must be invalid for this target — otherwise the
  // highlight would be useless. We don't hard-code which one to keep the
  // test robust against solver tweaks.
  const validCount = await valueButtons.evaluateAll(
    (els) => els.filter((el) => el.getAttribute('data-valid') === 'true').length
  );
  expect(validCount).toBeGreaterThan(0);
  expect(validCount).toBeLessThan(11);
});

test('share toasts an error when the draft has multiple solutions', async ({ page }) => {
  await openCreateTab(page);

  // Force every target to 0 — yields a puzzle with many solutions for any
  // size. Share must error.
  const rowTargets = page.locator('.app-shell table.puzzle th[scope="row"].target');
  const count = await rowTargets.count();
  for (let i = 0; i < count; i++) {
    await rowTargets.nth(i).click();
    await page.getByRole('button', { name: 'Set target to 0' }).click();
  }
  const colTargets = page.locator('.app-shell table.puzzle th[scope="col"].target');
  const colCount = await colTargets.count();
  for (let i = 0; i < colCount; i++) {
    await colTargets.nth(i).click();
    await page.getByRole('button', { name: 'Set target to 0' }).click();
  }

  await page.getByRole('button', { name: 'Share' }).click();

  await expect(page.locator('[role="status"]')).toContainText("Can't share an invalid puzzle");
});
