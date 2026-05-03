<script lang="ts">
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { playState } from '../../state/puzzle.svelte';
  import { parseTargetsText, formatTargetsText } from '../../state/url.svelte';
  import { explainPuzzle } from '../../wasm/api';
  import { trackEvent } from '../../analytics';
  import type { ExplainResponse, ExplainStep, RuleName } from '../../state/types';

  let inputText = $state(playState.puzzleData ? formatTargetsText(playState.puzzleData) : '');
  let feedback = $state('');
  let error = $state(false);
  let result = $state<ExplainResponse | null>(null);
  let stepIndex = $state(0);

  let lastSeenKey = $state<string | null>(null);
  $effect(() => {
    const data = playState.puzzleData;
    if (!data) return;
    const serialized = formatTargetsText(data);
    if (lastSeenKey === null || lastSeenKey !== serialized) {
      inputText = serialized;
      lastSeenKey = serialized;
    }
  });

  const RULE_LABELS: Record<RuleName, string> = {
    TargetTuples: 'target constraint',
    ArcConsistency: 'arc-consistency',
    Singleton: 'singleton',
    HiddenSingle: 'hidden single',
    BlackConsistency: 'black-consistency',
    Backtracking: 'backtracking',
  };

  function runExplain(): void {
    error = false;
    feedback = '';
    result = null;
    stepIndex = 0;

    const parsed = parseTargetsText(inputText);
    if ('error' in parsed) {
      error = true;
      feedback = parsed.error;
      return;
    }

    const response = explainPuzzle(parsed);
    if ('error' in response) {
      error = true;
      feedback = response.error;
      return;
    }

    trackEvent(`rublock/explain/explain/${parsed.size}`);
    result = response;
    feedback = `${response.steps.length} propagation step${response.steps.length === 1 ? '' : 's'}.`;
  }

  function currentStep(): ExplainStep | null {
    if (!result || result.steps.length === 0) return null;
    return result.steps[stepIndex] ?? null;
  }

  function highlightedCells(): Map<string, { exNew: boolean }> {
    const step = currentStep();
    const map = new Map<string, { exNew: boolean }>();
    if (!step) return map;
    for (const e of step.events) {
      map.set(`${e.row},${e.col}`, { exNew: true });
    }
    return map;
  }

  function stepSummary(step: ExplainStep): string {
    const byRule = new Map<RuleName, number>();
    for (const e of step.events) {
      const removed = countBits(e.before & ~e.after);
      byRule.set(e.rule, (byRule.get(e.rule) ?? 0) + removed);
    }
    return [...byRule.entries()]
      .map(([rule, count]) => `${count} bit${count === 1 ? '' : 's'} via ${RULE_LABELS[rule]}`)
      .join(', ');
  }

  function countBits(n: number): number {
    let count = 0;
    while (n) {
      count += n & 1;
      n >>>= 1;
    }
    return count;
  }

  function prev(): void {
    if (stepIndex > 0) stepIndex--;
  }

  function next(): void {
    if (result && stepIndex < result.steps.length - 1) stepIndex++;
  }
</script>

<section class="tab-panel">
  <div class="panel-card">
    <div class="controls-row">
      <div class="field" style="flex: 1;">
        <label for="explain-input">Targets</label>
        <input
          id="explain-input"
          type="text"
          placeholder="r1,r2,...,rN,c1,c2,...,cN"
          bind:value={inputText}
        />
      </div>
      <button class="btn-primary" type="button" onclick={runExplain}>Explain</button>
    </div>
    <div class="feedback" class:error aria-live="polite">{feedback}</div>

    {#if result && result.steps.length > 0}
      {@const step = currentStep()}
      <div class="explain-nav">
        <button type="button" onclick={prev} disabled={stepIndex === 0}>&lsaquo; Prev</button>
        <span class="step-counter">Step {stepIndex + 1} of {result.steps.length}</span>
        <button type="button" onclick={next} disabled={stepIndex === result.steps.length - 1}>
          Next &rsaquo;
        </button>
      </div>

      {#if step}
        <p class="step-summary">{stepSummary(step)}</p>
        <div class="step-events">
          {#each step.events as e (e.row + ',' + e.col + ',' + e.before + ',' + e.rule)}
            <div class="step-event">
              <span class="event-cell">({e.row + 1},{e.col + 1})</span>
              <span class="event-rule">{RULE_LABELS[e.rule]}</span>
              <span class="event-bits"
                >{countBits(e.before & ~e.after)} bit{countBits(e.before & ~e.after) === 1
                  ? ''
                  : 's'} removed</span
              >
            </div>
          {/each}
        </div>
      {/if}

      <div class="explain-grid">
        <PuzzleGrid puzzle={result} values={result.cells} cellExtras={highlightedCells()} />
      </div>
    {/if}
  </div>
</section>

<style>
  .explain-nav {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin: 0.75rem 0 0.5rem;
  }

  .step-counter {
    font-weight: 600;
    min-width: 10ch;
    text-align: center;
  }

  .step-summary {
    color: #555;
    font-size: 0.9rem;
    margin: 0.25rem 0 0.5rem;
  }

  .step-events {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    max-height: 10rem;
    overflow-y: auto;
    margin-bottom: 0.75rem;
    font-size: 0.85rem;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
  }

  .step-event {
    display: flex;
    gap: 0.75rem;
    color: #333;
  }

  .event-cell {
    font-family: monospace;
    min-width: 5ch;
  }

  .event-rule {
    color: #666;
    flex: 1;
  }

  .event-bits {
    color: #999;
    white-space: nowrap;
  }

  .explain-grid {
    margin-top: 0.5rem;
  }
</style>
