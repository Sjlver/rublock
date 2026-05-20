import { test, expect } from '@playwright/test';

// Playwright runs against `build:test`, which uses `--mode test`. AdSlot
// renders nothing outside of `--mode production`, and no EthicalAds
// publisher id is wired in test builds. The slot and its loader script
// must both be absent from the DOM in this mode.
//
// Production verification (slot renders, script loads, ad fills) happens
// manually post-deploy once the EthicalAds publisher application is
// approved and VITE_ETHICALADS_PUBLISHER_ID is set in the deploy workflow.

test('ad slot is not present on a fresh load in test-mode builds', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  await expect(page.locator('[data-ad-slot]')).toHaveCount(0);
  await expect(page.locator('script[data-ea-script]')).toHaveCount(0);
});

test('ad slot stays absent even after solving in test-mode builds', async ({ page }) => {
  // Reuses the known-solvable puzzle and hand-checked solution from
  // cell-interactions.spec.ts. The PlayTab `solvedThisGame` latch flips
  // here, but AdSlot's MODE gate must keep the slot out of the DOM.
  await page.goto('/?p=7047430a4100');
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  const solution: string[][] = [
    ['3', 'black', '1', '2', '4', 'black'],
    ['4', '1', '2', '3', 'black', 'black'],
    ['1', '2', 'black', '4', 'black', '3'],
    ['black', '3', '4', 'black', '2', '1'],
    ['black', '4', 'black', '1', '3', '2'],
    ['2', 'black', '3', 'black', '1', '4'],
  ];
  const rows = page.locator('.app-shell table.puzzle tbody tr');
  for (let r = 0; r < 6; r++) {
    for (let c = 0; c < 6; c++) {
      await rows.nth(r).locator('td').nth(c).click();
      await page.keyboard.press(solution[r][c] === 'black' ? 'b' : solution[r][c]);
    }
  }

  // Solve toast confirms the solved-event fired (and the latch flipped).
  await expect(page.locator('[role="status"]')).toContainText('Puzzle solved!');
  await expect(page.locator('[data-ad-slot]')).toHaveCount(0);
  await expect(page.locator('script[data-ea-script]')).toHaveCount(0);
});
