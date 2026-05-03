<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
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

  const controls = [
    { action: 'Place a note',         touch: 'Tap button',       kb: 'Space, then digit' },
    { action: 'Place an answer',       touch: 'Hold button',      kb: 'Digit (default)'   },
    { action: 'Toggle note/value mode',touch: '—',                kb: 'Space'             },
    { action: 'Mark cell black',       touch: 'Hold ■',           kb: '0, B, or X'        },
    { action: 'Mark digits-only',      touch: 'Tap ○',            kb: '9 or O'            },
    { action: 'Erase cell',            touch: 'Tap eraser',       kb: 'Backspace / Delete'},
    { action: 'Move selection',        touch: 'Tap cell',         kb: 'Arrow keys / WASD' },
  ];
</script>

<PageHeader title="How to play" />

<div class="tab-content">
  <div class="card">
    <p class="howto-prose">
      Each row and column has a <strong style="color:var(--ink)">target</strong> at its head.
      Place the digits and two black squares so the puzzle makes sense:
    </p>

    <div class="rule-row">
      <div class="rule-number">1</div>
      <div>
        <div class="rule-title">Two blacks</div>
        <div class="rule-body">Each row and each column contains exactly <strong>two black squares</strong>.</div>
      </div>
    </div>

    <div class="rule-row">
      <div class="rule-number">2</div>
      <div>
        <div class="rule-title">A permutation in between</div>
        <div class="rule-body">
          The other cells in each row and column hold the digits <strong>1 to N − 2</strong> — each appearing once.
        </div>
      </div>
    </div>

    <div class="rule-row">
      <div class="rule-number">3</div>
      <div>
        <div class="rule-title">Sum to the target</div>
        <div class="rule-body">
          The numbers <strong>between</strong> the two blacks must add up to the target shown.
          A target of <strong>0</strong> means the two blacks are adjacent.
        </div>
      </div>
    </div>

    <div class="divider"></div>

    <h2 style="font-size:13.5px; font-weight:700; margin-bottom:8px;">Step-by-step example</h2>
    <p class="howto-prose" style="font-size:13px;">
      Here is a fresh 5 × 5 puzzle. The digits used are 1, 2, and 3. Where do you start?
    </p>

    <!-- NOTE TO IMPLEMENTER: the worked examples below are from the original app.
         Keep them in the production redesign — see DESIGN_NOTES.md. -->

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step0Values} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      Step 1 — Row target 6 is the maximum possible sum
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      Digits 1 + 2 + 3 = <strong>6</strong>. Target 6 means every digit lies between the two blacks,
      so blacks go at the very ends: column 1 and column 5.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step1Values} cellExtras={step1Extras} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      Step 2 — Column 5 target 0 means the blacks are neighbours
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      Target 0 means nothing between the blacks — they must be adjacent.
      Column 5 already has a black in row 1, so the second black sits immediately below in row 2.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step2Values} cellExtras={step2Extras} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      Step 3 — Row 2 target 2 pins the second black
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      Row 2 already has a black at column 5. Target 2 means exactly digit <strong>2</strong>
      sits between the blacks. Place 2 at column 4, and the second black at column 3.
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step3Values} cellExtras={step3Extras} />
    </div>

    <p class="howto-prose" style="font-size:13px; margin-top:10px;">
      Each deduction unlocks the next. Keep going until the puzzle is complete —
      there is always exactly one solution.
    </p>

    <div class="divider"></div>

    <h2 style="font-size:13.5px; font-weight:700; margin-bottom:8px;">Controls</h2>
    <div class="controls-table">
      <div class="controls-row-item" style="padding-bottom:6px; font-size:11px; font-weight:700;
           color:var(--muted); text-transform:uppercase; letter-spacing:0.05em;">
        <div>Action</div>
        <div>Touch / mouse</div>
        <div>Keyboard</div>
      </div>
      {#each controls as row (row.action)}
        <div class="controls-row-item">
          <div class="controls-action">{row.action}</div>
          <div class="controls-touch">{row.touch}</div>
          <div class="controls-kb">{row.kb}</div>
        </div>
      {/each}
    </div>

    <p style="margin-top:12px; font-size:12px; color:var(--muted); line-height:1.5;">
      Source code:
      <a href="https://github.com/Sjlver/rublock" target="_blank" rel="noopener noreferrer"
         style="color:var(--accent); text-decoration:none;">
        github.com/Sjlver/rublock
      </a>
    </p>
  </div>
</div>
