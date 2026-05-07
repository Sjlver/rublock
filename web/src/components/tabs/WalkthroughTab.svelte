<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { playState } from '../../state/puzzle.svelte';
  import { explainPuzzle } from '../../wasm/api';
  import { trackEvent } from '../../analytics';
  import { t, tf, plural, format, md } from '../../i18n/index.svelte';
  import type { MessageKey } from '../../i18n/en';
  import type {
    CellNotes,
    CellValue,
    ExplainEvent,
    ExplainRule,
    ExplainStep,
    ExplainedPuzzle,
    SolvedPuzzle,
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
    let count = black ? 1 : 0;
    let singleDigit = 0;
    for (let d = 1; d <= 7; d++) {
      if (domain & (1 << d)) {
        count++;
        singleDigit = d;
      }
    }
    if (count === 1) {
      return { value: black ? 'black' : singleDigit, notes: 0 };
    }
    // Domain bits 0..6 map directly to CellNotes bits (bit 0 = black, bits 1..6 = digits)
    return { value: null, notes: domain };
  }

  type WaveView = {
    index: number;
    values: CellValue[][];
    notes: CellNotes[][];
    extras: Map<string, { exNew: true }>;
    counts: { rule: ExplainRule; count: number }[];
    total: number;
  };

  // Big puzzles can produce tens of thousands of search nodes, and rendering
  // a grid for every one freezes the browser. Once the trace enters the
  // backtracking phase, we collapse all subsequent steps into one summary
  // entry and finish with a single fully-solved grid. See issue #33.
  type SummaryItem = { kind: 'summary'; searchNodes: number };
  type WaveItem = { kind: 'wave' } & WaveView;
  type FinalItem = { kind: 'final'; values: CellValue[][]; notes: CellNotes[][] };
  type WalkthroughItem = WaveItem | SummaryItem | FinalItem;

  type WalkthroughView = {
    initial: WaveView;
    items: WalkthroughItem[];
    deductiveWaves: number;
    totalRemoved: number;
  };

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

  function buildWalkthrough(solved: SolvedPuzzle, steps: ExplainStep[]): WalkthroughView {
    const size = solved.row_targets.length;
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

    const items: WalkthroughItem[] = [];
    let deductiveWaves = 0;
    let totalRemoved = 0;
    let backtrackingStarted = false;
    let searchNodes = 0;

    for (let idx = 0; idx < steps.length; idx++) {
      const step = steps[idx];
      const containsBacktracking = step.events.some((e) => e.rule === 'Backtracking');
      if (!backtrackingStarted && containsBacktracking) backtrackingStarted = true;

      if (!backtrackingStarted) {
        const touched = new Set<string>();
        for (const ev of step.events) {
          domain[ev.row][ev.col] = ev.after;
          touched.add(`${ev.row},${ev.col}`);
        }
        const snap = snapshotDomain(size, domain);
        const extras = new Map<string, { exNew: true }>();
        for (const k of touched) extras.set(k, { exNew: true });
        deductiveWaves++;
        totalRemoved += step.events.length;
        items.push({
          kind: 'wave',
          index: idx + 1,
          values: snap.values,
          notes: snap.notes,
          extras,
          counts: summarizeRules(step.events),
          total: step.events.length,
        });
      } else if (containsBacktracking) {
        // One search-tree node per Backtracking-bearing step (take or reject).
        searchNodes++;
      }
    }

    if (backtrackingStarted) {
      items.push({ kind: 'summary', searchNodes });
      // The recorder collects events from every branch (including rejected
      // ones), so we can't trust the rolling `domain` to reach the solved
      // state. Use the solver's own solved grid for the final view.
      const finalValues: CellValue[][] = solved.cells.map((row) => row.slice());
      const finalNotes: CellNotes[][] = Array.from({ length: size }, () =>
        Array.from({ length: size }, () => 0)
      );
      items.push({ kind: 'final', values: finalValues, notes: finalNotes });
    }

    return { initial, items, deductiveWaves, totalRemoved };
  }

  // Friendly labels for the propagation rules. Wording avoids solver-internal
  // jargon — the user does not need to know what "arc consistency" is.
  const RULE_LABEL_KEYS: Record<ExplainRule, MessageKey> = {
    TargetTuples: 'wt_rule_target_tuples',
    ArcConsistency: 'wt_rule_arc',
    Singleton: 'wt_rule_singleton',
    HiddenSingle: 'wt_rule_hidden',
    BlackConsistency: 'wt_rule_black',
    Backtracking: 'wt_rule_backtrack',
  };

  const RULE_NOTE_KEYS: Record<ExplainRule, MessageKey> = {
    TargetTuples: 'wt_rule_target_tuples_note',
    ArcConsistency: 'wt_rule_arc_note',
    Singleton: 'wt_rule_singleton_note',
    HiddenSingle: 'wt_rule_hidden_note',
    BlackConsistency: 'wt_rule_black_note',
    Backtracking: 'wt_rule_backtrack_note',
  };

  function rulesHeading(counts: { rule: ExplainRule; count: number }[]): string {
    if (counts.length === 0) return '';
    return counts.map(({ rule, count }) => `${count} · ${t(RULE_LABEL_KEYS[rule])}`).join('   ');
  }

  function rulesExplanation(counts: { rule: ExplainRule; count: number }[]): string {
    if (counts.length === 0) return '';
    if (counts.length === 1) return t(RULE_NOTE_KEYS[counts[0].rule]);
    // Several rules contributed in this wave — give the dominant rule's
    // explanation, mentioning that others helped.
    const [first, ...rest] = counts;
    const others = rest.map(({ rule }) => t(RULE_LABEL_KEYS[rule]).toLowerCase()).join(', ');
    return `${t(RULE_NOTE_KEYS[first.rule])} ${format(t('wt_extra_rules'), { others })}`;
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
    const key = `${puzzle.row_targets.join(',')}|${puzzle.col_targets.join(',')}`;
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
    return buildWalkthrough(result.data, result.data.steps);
  });

  let statusText = $derived.by(() => {
    if (!playState.puzzleData) return t('wt_status_no_puzzle');
    if (result?.ok === false) return result.error;
    if (!view) return '';
    const wavesLabel = plural(view.deductiveWaves, 'wt_status_waves_one', 'wt_status_waves_other');
    const removalsLabel = plural(
      view.totalRemoved,
      'wt_status_removed_one',
      'wt_status_removed_other'
    );
    return `${wavesLabel}${t('wt_status_join')}${removalsLabel}`;
  });
</script>

<PageHeader
  title={t('wt_title')}
  status={statusText}
  statusTone={result?.ok === false ? 'error' : 'default'}
/>

<div class="tab-content">
  {#if !playState.puzzleData}
    <div class="walkthrough-placeholder">
      {@html md(t('wt_placeholder'))}
    </div>
  {:else if result?.ok === false}
    <div class="walkthrough-placeholder" data-testid="walkthrough-error">
      {tf('wt_error', { err: result.error })}
    </div>
  {:else if view}
    <div class="card walkthrough-intro">
      <p class="howto-prose" style="margin: 0;">
        {@html md(t('wt_intro1'))}
      </p>
      <p class="howto-prose" style="margin: 8px 0 0;">
        {@html md(t('wt_intro2'))}
      </p>
    </div>

    <section class="walkthrough-wave" data-testid="walkthrough-wave-initial">
      <h2 class="walkthrough-wave-title">{t('wt_start')}</h2>
      <p class="walkthrough-wave-sub">{t('wt_start_sub')}</p>
      <div class="walkthrough-grid">
        <PuzzleGrid
          puzzle={playState.puzzleData}
          values={view.initial.values}
          notes={view.initial.notes}
        />
      </div>
    </section>

    {#each view.items as item, i (i)}
      {#if item.kind === 'wave'}
        <section class="walkthrough-wave" data-testid="walkthrough-wave">
          <h2 class="walkthrough-wave-title">
            {tf('wt_wave', { n: item.index })}
            <span class="walkthrough-wave-count"
              >{plural(item.total, 'wt_wave_one', 'wt_wave_other')}</span
            >
          </h2>
          <p class="walkthrough-wave-rules">{rulesHeading(item.counts)}</p>
          <p class="walkthrough-wave-sub">{rulesExplanation(item.counts)}</p>
          <div class="walkthrough-grid">
            <PuzzleGrid
              puzzle={playState.puzzleData}
              values={item.values}
              notes={item.notes}
              cellExtras={item.extras}
            />
          </div>
        </section>
      {:else if item.kind === 'summary'}
        <section class="walkthrough-wave" data-testid="walkthrough-summary">
          <h2 class="walkthrough-wave-title">
            {t('wt_search')}
            <span class="walkthrough-wave-count"
              >{plural(item.searchNodes, 'wt_guess_one', 'wt_guess_other')}</span
            >
          </h2>
          <p class="walkthrough-wave-sub">
            {t('wt_search_sub')}
          </p>
        </section>
      {:else}
        <section class="walkthrough-wave" data-testid="walkthrough-wave-final">
          <h2 class="walkthrough-wave-title">{t('wt_solved')}</h2>
          <p class="walkthrough-wave-sub">{t('wt_solved_sub')}</p>
          <div class="walkthrough-grid">
            <PuzzleGrid puzzle={playState.puzzleData} values={item.values} notes={item.notes} />
          </div>
        </section>
      {/if}
    {/each}
  {/if}
</div>
