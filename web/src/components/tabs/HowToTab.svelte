<script lang="ts">
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import type { CellValue, PuzzleData } from '../../state/types';

  const exampleSize = 5;
  const exampleTargets: PuzzleData = {
    size: exampleSize,
    row_targets: [6, 2, 5, 1, 0],
    col_targets: [3, 0, 3, 0, 0],
  };

  function emptyValues(): CellValue[][] {
    return Array.from({ length: exampleSize }, () => Array<CellValue>(exampleSize).fill(null));
  }

  const step0Values = emptyValues();

  const step1Values = emptyValues();
  step1Values[0][0] = 'black';
  step1Values[0][4] = 'black';
  const step1Extras = new Map([
    ['0,0', { exNew: true }],
    ['0,4', { exNew: true }],
  ]);

  const step2Values = emptyValues();
  step2Values[0][0] = 'black';
  step2Values[0][4] = 'black';
  step2Values[1][4] = 'black';
  const step2Extras = new Map([['1,4', { exNew: true }]]);

  const step3Values = emptyValues();
  step3Values[0][0] = 'black';
  step3Values[0][4] = 'black';
  step3Values[1][4] = 'black';
  step3Values[1][2] = 'black';
  step3Values[1][3] = 2;
  const step3Extras = new Map([
    ['1,2', { exNew: true }],
    ['1,3', { exNew: true }],
  ]);
</script>

<section class="tab-panel">
  <div class="howto panel-card">
    <h2>The goal</h2>
    <p>
      Fill the grid so that every row and column contains <strong>exactly two black squares</strong>
      and each digit from <strong>1 to N − 2</strong> exactly once (where N is the puzzle size). The
      numbers along the top and left are <em>targets</em> — each target is the sum of the digits
      that sit <em>between</em> the two black squares in that row or column.
    </p>

    <h2>Step-by-step example</h2>
    <p>
      Here is a fresh 5 × 5 puzzle. The digits used are 1, 2, and 3. Everything is empty — where do
      you start?
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step0Values} />
    </div>

    <h3>Step 1 — Row target 6 is the maximum possible sum</h3>
    <p>
      In a 5 × 5 puzzle the digits are 1, 2, and 3. Their total is 1 + 2 + 3 = <strong>6</strong>. A
      row target of 6 means <em>every digit</em> must lie between the two blacks — the only way to fit
      all three digits is to put the blacks at the very ends: column 1 and column 5.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step1Values} cellExtras={step1Extras} />
    </div>

    <h3>Step 2 — Column 5 target 0 means the blacks are neighbours</h3>
    <p>
      A target of 0 means <em>nothing</em> between the two blacks — they must be in adjacent rows. Column
      5 already has a black in row 1 (from step 1), so the second black in column 5 must sit immediately
      below it, in row 2.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step2Values} cellExtras={step2Extras} />
    </div>

    <h3>Step 3 — Row 2 target 2 pins the second black</h3>
    <p>
      Row 2 already has a black at column 5. Target 2 means exactly the digit <strong>2</strong>
      sits between the two blacks (alone, because 2 is the only single digit that sums to 2). Place digit
      2 at column 4, and put the second black at column 3 to close it in.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step3Values} cellExtras={step3Extras} />
    </div>

    <p>
      Each deduction unlocks the next. Keep going row by row and column by column until the puzzle
      is complete — there is always exactly one solution.
    </p>

    <h2>Rules at a glance</h2>
    <ul>
      <li>Each row and column contains exactly <strong>two black squares</strong>.</li>
      <li>
        The remaining N − 2 cells in each row and column hold the digits 1 to N − 2, each exactly
        once.
      </li>
      <li>
        The sum of the digits <strong>between</strong> the two black squares must equal the target shown.
      </li>
      <li>
        A target of <strong>0</strong> means the two blacks are adjacent (nothing between them).
      </li>
    </ul>

    <h2>Controls</h2>
    <table>
      <thead>
        <tr><th>Action</th><th>Mouse or touch</th><th>Desktop keyboard</th></tr>
      </thead>
      <tbody>
        <tr>
          <td>Select a cell</td>
          <td>Tap or click a cell</td>
          <td>Use the arrow keys or WASD to move the selection</td>
        </tr>
        <tr>
          <td>Switch between values and notes</td>
          <td>Select the same cell again</td>
          <td>Press Space</td>
        </tr>
        <tr>
          <td>Enter a digit</td>
          <td>Use the number buttons below the grid</td>
          <td>Press a digit key</td>
        </tr>
        <tr>
          <td>Make a cell black</td>
          <td>Use BLACK below the grid</td>
          <td>Press 0, B, or X</td>
        </tr>
        <tr>
          <td>Mark a notes-only digit cell</td>
          <td>Use O below the grid in notes mode</td>
          <td>Press 9 or O</td>
        </tr>
        <tr>
          <td>Clear a cell</td>
          <td>Use CLEAR below the grid</td>
          <td>Press Backspace or Delete</td>
        </tr>
      </tbody>
    </table>
    <p>
      Selecting another cell keeps the current mode, so you can enter several values or notes in a
      row.
    </p>
  </div>
</section>
