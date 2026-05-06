import { test, expect } from '@playwright/test';

// "Newspaper puzzle 1" — the same 6×6 puzzle the Rust solver tests pin against
// (see src/basic_solver.rs and src/black_solver.rs):
//   row_targets = [8, 2, 3, 8, 9, 0]
//   col_targets = [0, 0, 5, 9, 0, 4]
// Encoded with the BASE62 URL scheme used by url.svelte.ts.
const NEWSPAPER_URL = '/?p=823890005904';

// A 6×6 puzzle that needs backtracking to solve. Generated with
//   cargo run --release --bin gen_puzzle -- --size=6 --min-nodes=10 --max-nodes=200
//   row_targets = [4, 0, 0, 0, 1, 4]
//   col_targets = [3, 0, 2, 6, 3, 4]
// search nodes: 10 — well above zero, so the trace is guaranteed to contain
// at least one Backtracking step and the walkthrough collapses the search
// phase into a single summary card.
const BACKTRACKING_URL = '/?p=400014302634';

const walkthroughTabButton = (page: import('@playwright/test').Page) =>
  page.locator('nav.bottom-nav').getByRole('button', { name: 'Walkthrough' });

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

test('walkthrough tab renders the start grid plus one grid per wave', async ({ page }) => {
  await page.goto(NEWSPAPER_URL);
  await waitForReady(page);

  await walkthroughTabButton(page).click();
  await expect(page.locator('.page-title')).toHaveText('Walkthrough');

  // The "Start" grid is always present, then at least one wave grid.
  await expect(page.locator('[data-testid="walkthrough-wave-initial"]')).toBeVisible();
  const waves = page.locator('[data-testid="walkthrough-wave"]');
  await expect(waves.first()).toBeVisible();
  expect(await waves.count()).toBeGreaterThan(0);

  // Status line summarizes the wave count.
  await expect(page.locator('[role="status"]')).toContainText(/wave/);
});

test('the start grid has every cell showing the full notes (every digit + black marker)', async ({
  page,
}) => {
  await page.goto(NEWSPAPER_URL);
  await waitForReady(page);
  await walkthroughTabButton(page).click();

  const startGrid = page.locator('[data-testid="walkthrough-wave-initial"] table.puzzle');
  await expect(startGrid).toBeVisible();

  // 6×6 puzzle ⇒ digits 1..4 must appear in every cell, plus an "x" marker.
  const cells = startGrid.locator('tbody td.cell');
  await expect(cells).toHaveCount(36);

  // Inspect the first cell as a representative — every cell starts identically.
  const firstCell = cells.first();
  for (const digit of ['1', '2', '3', '4']) {
    await expect(firstCell.locator(`.note.note-${digit}`)).toHaveText(digit);
  }
  await expect(firstCell.locator('.note-marker')).toHaveText('x');

  // No cell is decided in the start grid — no `.cell-value` and no `.cell.black`.
  await expect(startGrid.locator('.cell .cell-value')).toHaveCount(0);
  await expect(startGrid.locator('.cell.black')).toHaveCount(0);
});

test('each wave highlights the cells it modified', async ({ page }) => {
  await page.goto(NEWSPAPER_URL);
  await waitForReady(page);
  await walkthroughTabButton(page).click();

  const firstWave = page.locator('[data-testid="walkthrough-wave"]').first();
  await expect(firstWave).toBeVisible();

  // The yellow `ex-new` class is applied to every cell touched by the wave.
  const highlighted = firstWave.locator('.cell.ex-new');
  expect(await highlighted.count()).toBeGreaterThan(0);

  // The Start grid never highlights anything.
  await expect(page.locator('[data-testid="walkthrough-wave-initial"] .cell.ex-new')).toHaveCount(
    0
  );
});

test('the final wave shows a fully solved grid', async ({ page }) => {
  await page.goto(NEWSPAPER_URL);
  await waitForReady(page);
  await walkthroughTabButton(page).click();

  const lastWave = page.locator('[data-testid="walkthrough-wave"]').last();
  await expect(lastWave).toBeVisible();

  // A solved 6×6 puzzle has exactly 12 black cells (2 per row, 6 rows).
  await expect(lastWave.locator('.cell.black')).toHaveCount(12);
  // Every other cell is a determined digit (24 cells with `.cell-value`).
  await expect(lastWave.locator('.cell:not(.black) .cell-value')).toHaveCount(24);
  // No more notes anywhere on the final wave.
  await expect(lastWave.locator('.cell .cell-notes')).toHaveCount(0);
});

test('a puzzle without backtracking does not get a search summary', async ({ page }) => {
  // The newspaper puzzle solves by propagation alone — no Backtracking events
  // in the trace, so the walkthrough should never emit the "Search" card or
  // the dedicated "Solved" final grid.
  await page.goto(NEWSPAPER_URL);
  await waitForReady(page);
  await walkthroughTabButton(page).click();

  await expect(page.locator('[data-testid="walkthrough-summary"]')).toHaveCount(0);
  await expect(page.locator('[data-testid="walkthrough-wave-final"]')).toHaveCount(0);
});

test('backtracking-heavy puzzle collapses the search phase into a summary plus final grid', async ({
  page,
}) => {
  // The interesting bit: don't render a wave for every search node. Instead
  // emit one "Search · X guesses" card and one final solved grid.
  await page.goto(BACKTRACKING_URL);
  await waitForReady(page);
  await walkthroughTabButton(page).click();
  await expect(page.locator('.page-title')).toHaveText('Walkthrough');

  // Exactly one summary card; mentions a positive guess count.
  const summary = page.locator('[data-testid="walkthrough-summary"]');
  await expect(summary).toHaveCount(1);
  await expect(summary).toContainText(/\d+ guess(es)?/);
  await expect(summary).toContainText(/Search/);

  // Exactly one final-grid card, and it ships a fully-solved 6×6 grid.
  const finalGrid = page.locator('[data-testid="walkthrough-wave-final"]');
  await expect(finalGrid).toHaveCount(1);
  await expect(finalGrid.locator('.cell.black')).toHaveCount(12);
  await expect(finalGrid.locator('.cell:not(.black) .cell-value')).toHaveCount(24);
  await expect(finalGrid.locator('.cell .cell-notes')).toHaveCount(0);

  // The summary appears after every regular wave and immediately before the
  // final solved grid — no further wave-by-wave rendering of the search.
  const order = await page.evaluate(() => {
    const nodes = Array.from(
      document.querySelectorAll(
        '[data-testid="walkthrough-wave"], [data-testid="walkthrough-summary"], [data-testid="walkthrough-wave-final"]'
      )
    );
    return nodes.map((n) => (n as HTMLElement).dataset.testid);
  });
  const summaryIdx = order.indexOf('walkthrough-summary');
  const finalIdx = order.indexOf('walkthrough-wave-final');
  expect(summaryIdx).toBeGreaterThanOrEqual(0);
  expect(finalIdx).toBe(summaryIdx + 1);
  expect(order.lastIndexOf('walkthrough-wave')).toBeLessThan(summaryIdx);
});
