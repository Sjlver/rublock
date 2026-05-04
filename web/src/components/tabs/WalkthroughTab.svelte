<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { playState, emptyNotes } from '../../state/puzzle.svelte';
  import { explainPuzzle } from '../../wasm/api';
  import { trackEvent } from '../../analytics';
  import type {
    CellNotes,
    CellValue,
    ExplainEvent,
    ExplainRule,
    ExplainStep,
    ExplainedPuzzle,
    PuzzleData,
  } from '../../state/types';

  // Bit layout used by the wasm `explain_puzzle` (BlackSolverState):
  //   bit 0       = "could be black"
  //   bits 1..N-2 = "could be that digit"
  // FULL_DOMAIN therefore has bits 0..N-2 set.
  function fullDomain(size: number): number {
    return (1 << (size - 1)) - 1;
  }

  function domainToCell(domain: number): { value: CellValue; notes: CellNotes } {
    const black = (domain & 1) !== 0;
    const digits: number[] = [];
    for (let d = 1; d <= 7; d++) {
      if (domain & (1 << d)) digits.push(d);
    }
    const total = digits.length + (black ? 1 : 0);
    if (total === 1) {
      return { value: black ? 'black' : digits[0], notes: emptyNotes() };
    }
    return {
      value: null,
      notes: { digits, marker: black ? 'black' : null },
    };
  }

  type WaveView = {
    index: number;
    values: CellValue[][];
    notes: CellNotes[][];
    extras: Map<string, { exNew: true }>;
    counts: { rule: ExplainRule; count: number }[];
    total: number;
  };

  type WalkthroughView = { initial: WaveView; waves: WaveView[] };

  function snapshotDomain(
    size: number,
    domain: number[][]
  ): { values: CellValue[][]; notes: CellNotes[][] } {
    const values: CellValue[][] = [];
    const notes: CellNotes[][] = [];
    for (let r = 0; r < size; r++) {
      const vRow: CellValue[] = [];
      const nRow: CellNotes[] = [];
      for (let c = 0; c < size; c++) {
        const cell = domainToCell(domain[r][c]);
        vRow.push(cell.value);
        nRow.push(cell.notes);
      }
      values.push(vRow);
      notes.push(nRow);
    }
    return { values, notes };
  }

  function summarizeRules(events: ExplainEvent[]): { rule: ExplainRule; count: number }[] {
    const counts = new Map<ExplainRule, number>();
    for (const e of events) counts.set(e.rule, (counts.get(e.rule) ?? 0) + 1);
    return [...counts.entries()]
      .sort((a, b) => b[1] - a[1])
      .map(([rule, count]) => ({ rule, count }));
  }

  function buildWalkthrough(puzzle: PuzzleData, steps: ExplainStep[]): WalkthroughView {
    const size = puzzle.row_targets.length;
    const domain: number[][] = Array.from({ length: size }, () =>
      Array.from({ length: size }, () => fullDomain(size))
    );

    const initialSnap = snapshotDomain(size, domain);
    const initial: WaveView = {
      index: 0,
      values: initialSnap.values,
      notes: initialSnap.notes,
      extras: new Map(),
      counts: [],
      total: 0,
    };

    const waves: WaveView[] = [];
    steps.forEach((step, idx) => {
      const touched = new Set<string>();
      for (const ev of step.events) {
        domain[ev.row][ev.col] = ev.after;
        touched.add(`${ev.row},${ev.col}`);
      }
      const snap = snapshotDomain(size, domain);
      const extras = new Map<string, { exNew: true }>();
      for (const k of touched) extras.set(k, { exNew: true });
      waves.push({
        index: idx + 1,
        values: snap.values,
        notes: snap.notes,
        extras,
        counts: summarizeRules(step.events),
        total: step.events.length,
      });
    });

    return { initial, waves };
  }

  // Friendly labels for the propagation rules. Wording avoids solver-internal
  // jargon — the user does not need to know what "arc consistency" is.
  const RULE_LABELS: Record<ExplainRule, string> = {
    TargetTuples: 'Target sums',
    ArcConsistency: 'Possibility check',
    Singleton: 'Forced cells',
    HiddenSingle: 'Only place',
    BlackConsistency: 'Two-blacks rule',
    Backtracking: 'Hypothesis',
  };

  const RULE_NOTES: Record<ExplainRule, string> = {
    TargetTuples:
      'Some digit or black placements simply cannot be part of any arrangement that adds up to the row or column target — those are removed.',
    ArcConsistency:
      'No remaining arrangement of this row or column still supports these options, so they are eliminated.',
    Singleton:
      'A nearby cell is now fully determined, and its value cannot repeat in the rest of its row or column.',
    HiddenSingle:
      'Only one cell in this row or column can still hold this digit or black, so the others lose it as a candidate.',
    BlackConsistency:
      'Each row and each column has exactly two blacks. These options would create a third one — so they go.',
    Backtracking: 'The solver tried a guess to break a deadlock. Rare for hand-solvable puzzles.',
  };

  function rulesHeading(counts: { rule: ExplainRule; count: number }[]): string {
    if (counts.length === 0) return '';
    return counts.map(({ rule, count }) => `${count} · ${RULE_LABELS[rule]}`).join('   ');
  }

  function rulesExplanation(counts: { rule: ExplainRule; count: number }[]): string {
    if (counts.length === 0) return '';
    if (counts.length === 1) return RULE_NOTES[counts[0].rule];
    // Several rules contributed in this wave — give the dominant rule's
    // explanation, mentioning that others helped.
    const [first, ...rest] = counts;
    const others = rest.map(({ rule }) => RULE_LABELS[rule].toLowerCase()).join(', ');
    return `${RULE_NOTES[first.rule]} The wave also includes deductions from ${others}.`;
  }

  type Result = { ok: true; data: ExplainedPuzzle } | { ok: false; error: string } | null;

  let result = $state<Result>(null);
  let lastKey = $state<string | null>(null);

  $effect(() => {
    const puzzle = playState.puzzleData;
    if (!puzzle) {
      result = null;
      lastKey = null;
      return;
    }
    const size = puzzle.row_targets.length;
    const key = `${size}|${puzzle.row_targets.join(',')}|${puzzle.col_targets.join(',')}`;
    if (key === lastKey) return;
    lastKey = key;
    trackEvent(`rublock/walkthrough/show/${size}`);
    try {
      result = { ok: true, data: explainPuzzle(puzzle) };
    } catch (err) {
      result = { ok: false, error: err instanceof Error ? err.message : String(err) };
    }
  });

  let view = $derived.by<WalkthroughView | null>(() => {
    if (!result || !result.ok) return null;
    return buildWalkthrough(
      {
        row_targets: result.data.row_targets,
        col_targets: result.data.col_targets,
      },
      result.data.steps
    );
  });

  let totalRemoved = $derived(view ? view.waves.reduce((n, w) => n + w.total, 0) : 0);

  let statusText = $derived.by(() => {
    if (!playState.puzzleData) return 'No puzzle loaded.';
    if (result?.ok === false) return result.error;
    if (!view) return '';
    const wavesLabel = `${view.waves.length} wave${view.waves.length === 1 ? '' : 's'}`;
    const removalsLabel = `${totalRemoved} note${totalRemoved === 1 ? '' : 's'} removed`;
    return `${wavesLabel} · ${removalsLabel}`;
  });
</script>

<PageHeader
  title="Walkthrough"
  status={statusText}
  statusTone={result?.ok === false ? 'error' : 'default'}
/>

<div class="tab-content">
  {#if !playState.puzzleData}
    <div class="walkthrough-placeholder">
      Pick a puzzle on the <strong>Play</strong> or <strong>Solve</strong> tab. The step-by-step solution
      will appear here.
    </div>
  {:else if result?.ok === false}
    <div class="walkthrough-placeholder" data-testid="walkthrough-error">
      Could not generate a walkthrough: {result.error}
    </div>
  {:else if view}
    <div class="card walkthrough-intro">
      <p class="howto-prose" style="margin: 0;">
        Watch the solver chip away at the current puzzle. Each grid below is one <strong
          >wave</strong
        > — every change in a wave can be made from what was known before it.
      </p>
      <p class="howto-prose" style="margin: 8px 0 0;">
        Cells start with every digit (small numbers) plus an
        <strong>x</strong> for "could be black". As options get ruled out, the notes shrink. When only
        one option is left, the cell is filled in. Cells that changed in a wave are highlighted in yellow.
      </p>
    </div>

    <section class="walkthrough-wave" data-testid="walkthrough-wave-initial">
      <h2 class="walkthrough-wave-title">Start</h2>
      <p class="walkthrough-wave-sub">Every cell could still hold any digit or be black.</p>
      <div class="walkthrough-grid">
        <PuzzleGrid
          puzzle={playState.puzzleData}
          values={view.initial.values}
          notes={view.initial.notes}
        />
      </div>
    </section>

    {#each view.waves as wave (wave.index)}
      <section class="walkthrough-wave" data-testid="walkthrough-wave">
        <h2 class="walkthrough-wave-title">
          Wave {wave.index}
          <span class="walkthrough-wave-count"
            >· {wave.total} note{wave.total === 1 ? '' : 's'} removed</span
          >
        </h2>
        <p class="walkthrough-wave-rules">{rulesHeading(wave.counts)}</p>
        <p class="walkthrough-wave-sub">{rulesExplanation(wave.counts)}</p>
        <div class="walkthrough-grid">
          <PuzzleGrid
            puzzle={playState.puzzleData}
            values={wave.values}
            notes={wave.notes}
            cellExtras={wave.extras}
          />
        </div>
      </section>
    {/each}
  {/if}
</div>
