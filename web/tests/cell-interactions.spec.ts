import { test, expect } from '@playwright/test';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('[role="status"]')).toHaveText('Ready', { timeout: 15_000 });
}

test('clicking a cell selects it', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();

  await expect(firstCell).toHaveClass(/selected/);
});

test('input bar buttons are enabled after a cell is selected', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Before selection, input bar buttons are disabled.
  await expect(page.locator('.input-bar button').first()).toBeDisabled();

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();

  // After selection, buttons become enabled.
  await expect(page.locator('.input-bar button').first()).toBeEnabled();
});

test('clicking the BLACK input button marks a cell as black', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();

  await page.locator('.input-bar button', { hasText: 'BLACK' }).click();

  await expect(firstCell).toHaveClass(/black/);
  await expect(firstCell).toContainText('X');
});

test('clicking a digit button places a digit in the selected cell', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();

  // Click the "1" input button (always present for sizes 5–8).
  await page.locator('.input-bar button', { hasText: '1' }).click();

  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('clicking CLEAR removes a placed value', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();
  await page.locator('.input-bar button', { hasText: '1' }).click();

  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  await page.locator('.input-bar button', { hasText: 'CLEAR' }).click();

  await expect(firstCell.locator('.cell-value')).not.toBeVisible();
});

test('keyboard digit entry places a value in the selected cell', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();
  await page.keyboard.press('2');

  await expect(firstCell.locator('.cell-value')).toHaveText('2');
});

test('keyboard B key marks selected cell as black', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click();
  await page.keyboard.press('b');

  await expect(firstCell).toHaveClass(/black/);
});

test('clicking the same cell twice toggles into notes mode', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = page.locator('table.puzzle tbody tr:first-child td:first-child');
  await firstCell.click(); // select
  await firstCell.click(); // toggle mode

  // In notes mode the cell has the notes-mode class.
  await expect(firstCell).toHaveClass(/notes-mode/);
  await expect(page.locator('.input-mode-hint')).toContainText('Notes');
});

test('entering a correct complete solution shows "Puzzle solved!" feedback', async ({ page }) => {
  // Load a specific known-solvable 6×6 puzzle.
  // row_targets=[7,0,4,7,4,3], col_targets=[0,10,4,1,0,0]
  // base62: '7047430a4100'
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);

  // Navigate to Solve tab to reveal the solution.
  await page.locator('nav.tabs').getByRole('button', { name: 'Solve' }).click();

  // The solve input is auto-populated with the current puzzle's targets.
  await page.locator('.panel-card').getByRole('button', { name: 'Solve' }).click();
  await expect(page.locator('.solve-result table.puzzle')).toBeVisible({ timeout: 10_000 });

  // Read every cell from the solved grid.
  const solveCells = page.locator('.solve-result table.puzzle tbody td.cell');
  const count = await solveCells.count();
  const solution: string[] = [];
  for (let i = 0; i < count; i++) {
    const cell = solveCells.nth(i);
    const cls = (await cell.getAttribute('class')) ?? '';
    if (cls.includes('black')) {
      solution.push('black');
    } else {
      const valEl = cell.locator('.cell-value');
      solution.push((await valEl.count()) > 0 ? ((await valEl.textContent()) ?? '') : '');
    }
  }

  // Go back to Play tab (same puzzle is set, progress should be empty).
  await page.locator('nav.tabs').getByRole('button', { name: 'Play' }).click();

  // Enter each value cell by cell.
  const size = 6;
  const rows = page.locator('table.puzzle tbody tr');
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      const cell = rows.nth(r).locator('td').nth(c);
      await cell.click();
      const val = solution[r * size + c];
      if (val === 'black') {
        await page.keyboard.press('b');
      } else if (val) {
        await page.keyboard.press(val);
      }
    }
  }

  // A valid complete solution should trigger "Puzzle solved! 🎉" feedback.
  const feedback = page.locator('.panel-card .feedback').first();
  await expect(feedback).toContainText('Puzzle solved!', { timeout: 5_000 });
});
