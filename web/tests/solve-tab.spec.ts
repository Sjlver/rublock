import { test, expect } from '@playwright/test';

// Helpers for the solve panel.
const solveInput = (page: import('@playwright/test').Page) => page.locator('#solve-input');
const solveCard = (page: import('@playwright/test').Page) =>
  page.locator('.card:has(#solve-input)');
const solveButton = (page: import('@playwright/test').Page) =>
  solveCard(page).getByRole('button', { name: 'Solve' });
// Feedback is rendered inside the page header as [role="status"].
const solveFeedback = (page: import('@playwright/test').Page) => page.locator('[role="status"]');

async function openSolveTab(page: import('@playwright/test').Page) {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Solve' }).click();
}

test('entering valid targets and clicking Solve shows a solution grid', async ({ page }) => {
  await openSolveTab(page);

  // Known-solvable 6×6 puzzle: row=[7,0,4,7,4,3], col=[0,10,4,1,0,0]
  await solveInput(page).fill('7,0,4,7,4,3,0,10,4,1,0,0');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toHaveText('Solved.');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
});

test('solved grid contains a mix of black cells and digit cells', async ({ page }) => {
  await openSolveTab(page);

  await solveInput(page).fill('7,0,4,7,4,3,0,10,4,1,0,0');
  await solveButton(page).click();
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // A valid 6×6 solution has exactly 2 black cells per row (12 total).
  const blackCells = page.locator('.app-shell table.puzzle .cell.black');
  await expect(blackCells).toHaveCount(12);
});

test('entering invalid targets shows a parse error', async ({ page }) => {
  await openSolveTab(page);

  await solveInput(page).fill('not,valid,targets');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toBeVisible();
  await expect(solveFeedback(page)).toHaveClass(/error/);
  await expect(solveFeedback(page)).not.toBeEmpty();
});

test('entering unsolvable targets shows an error', async ({ page }) => {
  await openSolveTab(page);

  // All targets = max_sum (10 for N=6): contradicts itself via black-cell placement.
  await solveInput(page).fill('10,10,10,10,10,10,10,10,10,10,10,10');
  await solveButton(page).click();

  await expect(solveFeedback(page)).toHaveClass(/error/);
});

test('solve input is pre-populated with the current puzzle targets', async ({ page }) => {
  // Load a specific puzzle so we know the expected targets.
  // row=[7,0,4,7,4,3], col=[0,10,4,1,0,0]
  await page.goto('/?p=7047430a4100');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Solve' }).click();

  // The SolveTab $effect mirrors the current puzzle's targets into the input.
  await expect(solveInput(page)).toHaveValue('7,0,4,7,4,3,0,10,4,1,0,0');
});
