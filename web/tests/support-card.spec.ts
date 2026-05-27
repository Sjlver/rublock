import { test, expect, type Page } from '@playwright/test';

// The post-solve "support the project" card shows only when the lifetime solve
// count is prime (see web/src/state/support.ts). These tests drive a real solve
// and seed the persisted count so the next solve lands on a chosen number.
//
// The card itself renders in every build mode (it's just links). The
// GoatCounter impression/click events fire only in `--mode production`, so they
// stay silent under Playwright's `--mode test` build and aren't asserted here.

// A known-solvable 6×6 puzzle and its hand-checked solution (same fixture the
// retired EthicalAds-slot test used). 'black' = a black square; the rest are
// placed digits.
const PUZZLE = '7047430a4100';
const SOLUTION: ReadonlyArray<ReadonlyArray<string>> = [
  ['3', 'black', '1', '2', '4', 'black'],
  ['4', '1', '2', '3', 'black', 'black'],
  ['1', '2', 'black', '4', 'black', '3'],
  ['black', '3', '4', 'black', '2', '1'],
  ['black', '4', 'black', '1', '3', '2'],
  ['2', 'black', '3', 'black', '1', '4'],
];

const KEY_SOLVE_COUNT = 'rublock-solve-count';

async function waitForReady(page: Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

// Seed the lifetime solve count so the *next* solve lands on a chosen value.
// Runs before the app boots on the upcoming navigation.
async function seedSolveCount(page: Page, value: number) {
  await page.addInitScript(({ key, value }) => localStorage.setItem(key, String(value)), {
    key: KEY_SOLVE_COUNT,
    value,
  });
}

async function solveCurrentPuzzle(page: Page) {
  const rows = page.locator('.app-shell table.puzzle tbody tr');
  for (let r = 0; r < 6; r++) {
    for (let c = 0; c < 6; c++) {
      await rows.nth(r).locator('td').nth(c).click();
      await page.keyboard.press(SOLUTION[r][c] === 'black' ? 'b' : SOLUTION[r][c]);
    }
  }
  // The solve toast confirms the solved-event fired and the latch updated.
  await expect(page.locator('[role="status"]')).toContainText('Puzzle solved!');
}

test('the support card appears on a prime-numbered solve', async ({ page }) => {
  await seedSolveCount(page, 1); // next solve → count 2 (prime)
  await page.goto(`/?p=${PUZZLE}`);
  await waitForReady(page);
  await solveCurrentPuzzle(page);

  const card = page.locator('[data-support-card]');
  await expect(card).toBeVisible();
  // The first card ever shown is deterministic: rotation index 0 → Liberapay.
  await expect(card).toHaveAttribute('data-support-platform', 'liberapay');
  // The CTA links to one of the two configured donation platforms.
  await expect(card.locator('a[href]')).toHaveAttribute(
    'href',
    /^https:\/\/(liberapay\.com|ko-fi\.com)\//
  );
});

test('no support card on a non-prime solve', async ({ page }) => {
  await seedSolveCount(page, 3); // next solve → count 4 (composite)
  await page.goto(`/?p=${PUZZLE}`);
  await waitForReady(page);
  await solveCurrentPuzzle(page);

  await expect(page.locator('[data-support-card]')).toHaveCount(0);
});

test('the support card can be dismissed', async ({ page }) => {
  await seedSolveCount(page, 1);
  await page.goto(`/?p=${PUZZLE}`);
  await waitForReady(page);
  await solveCurrentPuzzle(page);

  const card = page.locator('[data-support-card]');
  await expect(card).toBeVisible();
  await card.locator('.support-dismiss').click();
  await expect(card).toHaveCount(0);
});
