<script lang="ts">
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { setPuzzle } from '../../state/puzzle.svelte';
  import { parseTargetsText, serializePuzzleTargets } from '../../state/url.svelte';
  import { solvePuzzle } from '../../wasm/api';
  import { trackEvent } from '../../analytics';
  import { playState } from '../../state/puzzle.svelte';
  import type { SolvedPuzzle } from '../../state/types';

  let inputText = $state(playState.puzzleData ? serializePuzzleTargets(playState.puzzleData) : '');
  let feedback = $state('');
  let error = $state(false);
  let solved = $state<SolvedPuzzle | null>(null);

  // Mirror the current puzzle's targets into the textbox unless the user has
  // typed something else.
  let lastSeenKey = $state<string | null>(null);
  $effect(() => {
    const data = playState.puzzleData;
    if (!data) return;
    const serialized = serializePuzzleTargets(data);
    if (lastSeenKey === null || lastSeenKey !== serialized) {
      inputText = serialized;
      lastSeenKey = serialized;
    }
  });

  function solveFromInput(): void {
    error = false;
    feedback = '';
    solved = null;

    const parsed = parseTargetsText(inputText);
    if ('error' in parsed) {
      error = true;
      feedback = parsed.error;
      return;
    }

    const response = solvePuzzle(parsed);
    if ('error' in response) {
      error = true;
      feedback = response.error;
      return;
    }

    feedback = 'Solved.';
    trackEvent('rublock/solve/solve');
    solved = response;
    setPuzzle(parsed, { preserveProgressIfSame: true });
  }
</script>

<section class="tab-panel">
  <div class="panel-card">
    <div class="controls-row">
      <div class="field" style="flex: 1;">
        <label for="solve-input">Targets</label>
        <input
          id="solve-input"
          type="text"
          placeholder="r1,r2,...,rN,c1,c2,...,cN"
          bind:value={inputText}
        />
      </div>
      <button class="btn-primary" type="button" onclick={solveFromInput}>Solve</button>
    </div>
    <div class="feedback" class:error aria-live="polite">{feedback}</div>
    <div class="solve-result">
      {#if solved}
        <PuzzleGrid puzzle={solved} values={solved.cells} />
      {/if}
    </div>
  </div>
</section>
