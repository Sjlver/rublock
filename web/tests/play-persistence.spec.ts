import { test, expect, type Page } from '@playwright/test';

const STORAGE_KEY = 'rublock-play-state';

async function waitForReady(page: Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

function firstDataCell(page: Page) {
  return page.locator('.app-shell table.puzzle tbody tr:first-child td:nth-child(2)');
}

function rowTargetCells(page: Page) {
  return page.locator('.app-shell table.puzzle th[scope="row"].target');
}

async function readStoredState(page: Page): Promise<unknown | null> {
  return page.evaluate((key) => {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  }, STORAGE_KEY);
}

// Wait long enough for the 250 ms persistence debounce to flush.
async function waitForDebounce(page: Page) {
  await page.waitForTimeout(350);
}

// Each test gets a fresh BrowserContext from Playwright, so localStorage
// starts empty. The seed-localStorage helpers below run BEFORE every page
// load in the test, which is fine for tests that only navigate once.

test('a placed value survives a full page reload', async ({ page }) => {
  // Use a deterministic puzzle so the targets render predictably after reload.
  await page.goto('/?p=5,10,15,20,25,3,6,9,12,15');
  await waitForReady(page);

  const cell = firstDataCell(page);
  await cell.click();
  await page.keyboard.press('1');
  await expect(cell.locator('.cell-value')).toHaveText('1');

  await waitForDebounce(page);

  // Plain reload (no `?p=`) — localStorage must take over.
  await page.goto('/');
  await waitForReady(page);

  // Same puzzle (5×5 from the URL above) is restored along with the digit.
  await expect(rowTargetCells(page)).toHaveText(['5', '10', '15', '20', '25']);
  await expect(firstDataCell(page).locator('.cell-value')).toHaveText('1');
});

test('?p= takes precedence over saved state for the same size', async ({ page }) => {
  // First visit: save progress for one 5×5 puzzle.
  await page.goto('/?p=5,10,15,20,25,3,6,9,12,15');
  await waitForReady(page);
  await firstDataCell(page).click();
  await page.keyboard.press('1');
  await expect(firstDataCell(page).locator('.cell-value')).toHaveText('1');
  await waitForDebounce(page);

  // Second visit with a *different* 5×5 puzzle — URL wins; previous progress
  // for that size is discarded.
  await page.goto('/?p=2,4,6,8,10,1,3,5,7,9');
  await waitForReady(page);

  await expect(rowTargetCells(page)).toHaveText(['2', '4', '6', '8', '10']);
  // The previously placed value is gone for the new puzzle.
  await expect(firstDataCell(page).locator('.cell-value')).not.toBeVisible();
});

test('?p= for one size leaves saved progress on other sizes intact', async ({ page }) => {
  // Save progress on a 5×5 puzzle.
  await page.goto('/?p=5,10,15,20,25,3,6,9,12,15');
  await waitForReady(page);
  await firstDataCell(page).click();
  await page.keyboard.press('1');
  await expect(firstDataCell(page).locator('.cell-value')).toHaveText('1');
  await waitForDebounce(page);

  // Open with a *different size* — overrides only the 7×7 slot.
  // 14 base62 chars → size 7. Targets here are arbitrary; we only need a
  // valid 7×7 grid to render so we can switch back to 5×5.
  await page.goto('/?p=11111114444444');
  await waitForReady(page);
  // The 7×7 puzzle is now active.
  await expect(rowTargetCells(page)).toHaveCount(7);
  await waitForDebounce(page);

  // Switch back to 5×5 — saved progress (the '1' in the first cell) survives.
  await page.locator('.size-selector .size-btn', { hasText: '5×5' }).click();
  await expect(rowTargetCells(page)).toHaveText(['5', '10', '15', '20', '25']);
  await expect(firstDataCell(page).locator('.cell-value')).toHaveText('1');
});

test('no URL and empty storage falls back to a random 6×6 puzzle', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('6×6');
  await expect(rowTargetCells(page)).toHaveCount(6);
});

test('malformed saved state is ignored silently', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('rublock-play-state', 'not valid json');
  });

  await page.goto('/');
  await waitForReady(page);

  // App must boot normally with a default 6×6 puzzle rather than crashing.
  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('6×6');
  await expect(rowTargetCells(page)).toHaveCount(6);
});

test('a saved state with an unknown future version is ignored', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem(
      'rublock-play-state',
      JSON.stringify({ version: 99, activeSize: 6, sizes: {} })
    );
  });

  await page.goto('/');
  await waitForReady(page);
  await expect(page.locator('.size-selector .size-btn.active')).toHaveText('6×6');
  await expect(rowTargetCells(page)).toHaveCount(6);
});

test('persisted blob is versioned and includes the active size', async ({ page }) => {
  await page.goto('/?p=5,10,15,20,25,3,6,9,12,15');
  await waitForReady(page);

  await firstDataCell(page).click();
  await page.keyboard.press('1');
  await waitForDebounce(page);

  const stored = (await readStoredState(page)) as {
    version: number;
    activeSize: number;
    sizes: Record<string, { puzzle: { row_targets: number[] }; undoStack: unknown[] }>;
  } | null;
  expect(stored).not.toBeNull();
  expect(stored!.version).toBe(1);
  expect(stored!.activeSize).toBe(5);
  expect(stored!.sizes['5'].puzzle.row_targets).toEqual([5, 10, 15, 20, 25]);
  // The digit press generates one undo entry.
  expect(stored!.sizes['5'].undoStack.length).toBeGreaterThanOrEqual(1);
});

test('locale preference still round-trips after the storage refactor', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('rublock-locale', 'de');
  });

  await page.goto('/');
  await waitForReady(page);

  // German UI: bottom-nav "Play" reads "Spielen".
  await expect(page.locator('nav.bottom-nav button.active')).toContainText('Spielen');
});

test('hint-dismissed flag still round-trips after the storage refactor', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('rublock-hint-dismissed', '1');
  });

  await page.goto('/');
  await waitForReady(page);

  // The dismissible hint chip is hidden.
  await expect(page.locator('.hint-chip')).toHaveCount(0);
});
