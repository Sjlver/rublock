import { test, expect } from './fixtures';

// Helpers for the solve panel.
const solveInput = (page: import('@playwright/test').Page) => page.locator('#solve-input');
const solveButton = (page: import('@playwright/test').Page) =>
  page.locator('.panel-card').getByRole('button', { name: 'Solve' });
const solveFeedback = (page: import('@playwright/test').Page) =>
  page.locator('.panel-card .feedback');

async function openSolveTab(page: import('@playwright/test').Page) {
  await page.goto('/');
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });
  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();
}

test('entering valid targets and clicking Solve shows a solution grid', async ({ page }) => {
  await openSolveTab(page);

  // Known-solvable 6×6 puzzle: row=[7,0,4,7,4,3], col=[0,10,4,1,0,0]
  await solveInput(page).fill('7,0,4,7,4,3,0,10,4,1,0,0');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toHaveText('Solved.');
  await expect(page.locator('.solve-result table.puzzle')).toBeVisible({ timeout: 10_000 });
});

test('solved grid contains a mix of black cells and digit cells', async ({ page }) => {
  await openSolveTab(page);

  await solveInput(page).fill('7,0,4,7,4,3,0,10,4,1,0,0');
  await solveButton(page).click();
  await expect(page.locator('.solve-result table.puzzle')).toBeVisible({ timeout: 10_000 });

  // A valid 6×6 solution has exactly 2 black cells per row (12 total).
  const blackCells = page.locator('.solve-result .cell.black');
  await expect(blackCells).toHaveCount(12);
});

test('entering invalid targets shows a parse error', async ({ page }) => {
  await openSolveTab(page);

  await solveInput(page).fill('not,valid,targets');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toBeVisible();
  // The feedback element must contain an error (non-empty text, error class).
  await expect(solveFeedback(page)).toHaveClass(/error/);
  await expect(solveFeedback(page)).not.toBeEmpty();
});

test('entering unsolvable targets shows an error', async ({ page }) => {
  await openSolveTab(page);

  // All targets set to 99 — impossible to satisfy.
  await solveInput(page).fill('99,99,99,99,99,99,99,99,99,99,99,99');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toHaveClass(/error/);
});

test('solve input is pre-populated with the current puzzle targets', async ({ page }) => {
  // Load a specific puzzle so we know the expected targets.
  // row=[7,0,4,7,4,3], col=[0,10,4,1,0,0]
  await page.goto('/?p=7047430a4100');
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });

  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();

  // The SolveTab $effect mirrors the current puzzle's targets into the input.
  await expect(solveInput(page)).toHaveValue('7,0,4,7,4,3,0,10,4,1,0,0');
});

test('solving a puzzle on Solve tab loads it on the Play tab', async ({ page }) => {
  await openSolveTab(page);

  await solveInput(page).fill('7,0,4,7,4,3,0,10,4,1,0,0');
  await solveButton(page).click();
  await expect(page.locator('.solve-result table.puzzle')).toBeVisible({ timeout: 10_000 });

  // setPuzzle is called with preserveProgressIfSame:true; the puzzle is now
  // loaded in the Play tab.
  await page.locator('nav.tabs').getByRole('button', { name: 'Play' }).click();

  // The Play tab should show the same puzzle (row target 7 in first row).
  await expect(page.locator('table.puzzle th[scope="row"].target').first()).toHaveText('7');
});
