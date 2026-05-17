import { test, expect } from '@playwright/test';

// Freshly generated puzzles are all solvable by propagation alone (see the
// `generate_puzzle_n` loop in src/wasm.rs), so the chip lands in one of the
// three propagation-only buckets. Tests that care about the chip after a
// puzzle is generated assert against this set rather than a single value.
const PROPAGATION_ONLY_LABEL = /^(Easy|Medium|Challenging)$/;

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

test('chip shows a propagation-only label for a freshly generated puzzle', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);
  await expect(page.locator('[role="status"]')).toHaveText(PROPAGATION_ONLY_LABEL);
});

test('chip shows "No solution" for an unsolvable share URL', async ({ page }) => {
  // 6×6 with every target = 1: a target of 1 means the cage between the two
  // black squares sums to 1, but achieving that on every row and column
  // simultaneously is impossible — see classify::tests::unsolvable_puzzle_*.
  await page.goto('/?p=111111111111');
  await waitForReady(page);
  await expect(page.locator('[role="status"]')).toHaveText('No solution');
});

test('chip shows "Multiple solutions" for an under-constrained share URL', async ({ page }) => {
  // 6×6 with every target = 0. The two black squares in each row/column are
  // adjacent, but there are many ways to place them — yields multiple
  // solutions.
  await page.goto('/?p=000000000000');
  await waitForReady(page);
  await expect(page.locator('[role="status"]')).toHaveText('Multiple solutions');
});

test('chip updates when switching sizes', async ({ page }) => {
  // Start on a multi-solution 6×6 puzzle.
  await page.goto('/?p=000000000000');
  await waitForReady(page);
  await expect(page.locator('[role="status"]')).toHaveText('Multiple solutions');

  // Switch to 5×5 — Play generates a fresh propagation-only puzzle.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target')).toHaveCount(5);
  await expect(page.locator('[role="status"]')).toHaveText(PROPAGATION_ONLY_LABEL);

  // Switching back to 6×6 restores the cached multi-solution puzzle, and the
  // chip follows.
  await page.locator('.size-selector .size-btn', { hasText: '6×6' }).click();
  await expect(page.locator('.app-shell table.puzzle th[scope="row"].target')).toHaveCount(6);
  await expect(page.locator('[role="status"]')).toHaveText('Multiple solutions');
});

test('chip updates after "Use this puzzle" from Create', async ({ page }) => {
  // Default Play puzzle is unique (loadRandomPuzzle only emits propagation-
  // solvable puzzles), so the seeded Create draft mirrors it and is also
  // unique — letting us exercise the "Use this puzzle" path end-to-end.
  await page.goto('/');
  await waitForReady(page);
  // Capture the label of the freshly generated puzzle so we can assert the
  // chip stays in sync as we move between tabs (the puzzle is the same, so
  // the label must be too — even though the *value* depends on wave count).
  const initialLabel = await page.locator('[role="status"]').innerText();
  expect(initialLabel).toMatch(PROPAGATION_ONLY_LABEL);

  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Create' }).click();
  await expect(page.locator('.page-title')).toHaveText('Create');
  await expect(page.locator('[role="status"]')).toHaveText(initialLabel);

  await page.getByRole('button', { name: 'Use this puzzle' }).click();
  await expect(page.locator('.page-title')).toHaveText('Play');
  // The chip on Play reflects the same puzzle's classification.
  await expect(page.locator('[role="status"]')).toHaveText(initialLabel);
});
