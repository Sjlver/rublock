import { test, expect } from '@playwright/test';

async function waitForReady(page: import('@playwright/test').Page) {
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();
}

// Locator for the first data cell in the grid (row 0, col 0).
function firstCellLocator(page: import('@playwright/test').Page) {
  return page.locator('.app-shell table.puzzle tbody tr:first-child td:nth-child(2)');
}

// Long-press a pad button to place a value (tap places a note in the new design).
async function longPress(button: import('@playwright/test').Locator, ms = 450) {
  const box = await button.boundingBox();
  if (!box) throw new Error('button has no bounding box');
  const page = button.page();
  const x = box.x + box.width / 2;
  const y = box.y + box.height / 2;
  await page.mouse.move(x, y);
  await page.mouse.down();
  await page.waitForTimeout(ms);
  await page.mouse.up();
}

test('clicking a cell selects it', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();

  await expect(firstCell).toHaveClass(/selected/);
});

test('input bar buttons are enabled after a cell is selected', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  // Before selection, input bar buttons are disabled.
  await expect(page.locator('.input-bar button').first()).toBeDisabled();

  await firstCellLocator(page).click();

  // After selection, buttons become enabled.
  await expect(page.locator('.input-bar button').first()).toBeEnabled();
});

test('holding the BLACK pad button marks a cell as black', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();

  // Long-press places the value (tap = note marker).
  await longPress(page.locator('.input-bar button[aria-label^="Black cell"]'));

  await expect(firstCell).toHaveClass(/black/);
});

test('tapping the BLACK pad button places a black note marker', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();

  // Short click = note marker (an "x" in the corner of the cell).
  await page.locator('.input-bar button[aria-label^="Black cell"]').click();

  await expect(firstCell).not.toHaveClass(/black/);
  await expect(firstCell.locator('.note-marker')).toHaveText('x');
});

test('holding a digit pad button places that digit as the cell value', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();

  await longPress(page.locator('.input-bar button[aria-label^="Digit 1 "]'));

  await expect(firstCell.locator('.cell-value')).toHaveText('1');
});

test('clicking the eraser button removes a placed value', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();
  // Use the keyboard to place a value — keyboard digits place values directly.
  await page.keyboard.press('1');
  await expect(firstCell.locator('.cell-value')).toHaveText('1');

  await page.locator('.input-bar button[aria-label="Erase cell"]').click();

  await expect(firstCell.locator('.cell-value')).not.toBeVisible();
});

test('keyboard digit entry places a value in the selected cell', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();
  await page.keyboard.press('2');

  await expect(firstCell.locator('.cell-value')).toHaveText('2');
});

test('keyboard B key marks selected cell as black', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click();
  await page.keyboard.press('b');

  await expect(firstCell).toHaveClass(/black/);
});

test('clicking the same cell twice toggles into notes mode', async ({ page }) => {
  await page.goto('/');
  await waitForReady(page);

  const firstCell = firstCellLocator(page);
  await firstCell.click(); // select
  await firstCell.click(); // toggle mode

  // The cell gains the notes-mode class and the keyboard mode badge appears.
  await expect(firstCell).toHaveClass(/notes-mode/);
  await expect(page.locator('.mode-badge')).toContainText('Notes');
});

test('entering a correct complete solution shows "Puzzle solved!" feedback', async ({ page }) => {
  // Load a specific known-solvable 6×6 puzzle.
  // row_targets=[7,0,4,7,4,3], col_targets=[0,10,4,1,0,0]
  // base62: '7047430a4100'
  await page.goto('/?p=7047430a4100');
  await waitForReady(page);

  // Navigate to Solve tab to reveal the solution.
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Solve' }).click();

  // The solve input is auto-populated with the current puzzle's targets.
  await page.locator('.card').getByRole('button', { name: 'Solve' }).click();
  await expect(page.locator('.app-shell table.puzzle')).toBeVisible();

  // Read every cell from the solved grid (only one table.puzzle is visible on Solve).
  const solveCells = page.locator('.app-shell table.puzzle tbody td.cell');
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
  await page.locator('nav.bottom-nav').getByRole('button', { name: 'Play', exact: true }).click();

  // Enter each value cell by cell using the keyboard (places values directly).
  const size = 6;
  const rows = page.locator('.app-shell table.puzzle tbody tr');
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

  // A valid complete solution surfaces "Puzzle solved!" via the page status toast.
  await expect(page.locator('[role="status"]')).toContainText('Puzzle solved!');
});
