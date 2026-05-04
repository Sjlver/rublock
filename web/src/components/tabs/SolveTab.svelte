<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { setPuzzle } from '../../state/puzzle.svelte';
  import { parseTargetsText, formatTargetsText } from '../../state/url.svelte';
  import { solvePuzzle } from '../../wasm/api';
  import { trackEvent } from '../../analytics';
  import { playState } from '../../state/puzzle.svelte';
  import type { SolvedPuzzle } from '../../state/types';

  let inputText = $state(playState.puzzleData ? formatTargetsText(playState.puzzleData) : '');
  let feedbackText = $state('Enter targets to solve any puzzle.');
  let feedbackError = $state(false);
  let solved = $state<SolvedPuzzle | null>(null);

  // Mirror current puzzle targets into the input unless the user has typed something.
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

  function solveFromInput(): void {
    feedbackError = false;
    feedbackText = '';
    solved = null;

    const parsed = parseTargetsText(inputText);
    if ('error' in parsed) {
      feedbackError = true;
      feedbackText = parsed.error;
      return;
    }

    let response;
    try {
      response = solvePuzzle(parsed);
    } catch (err) {
      feedbackError = true;
      feedbackText = err instanceof Error ? err.message : String(err);
      return;
    }

    feedbackText = 'Solved.';
    trackEvent(`rublock/solve/solve/${parsed.row_targets.length}`);
    solved = response;
    setPuzzle(parsed, { preserveProgressIfSame: true });
  }

  // Toast-style share (not wired to a URL for Solve tab)
  let shareStatus = $state('');
  async function handleShare(): Promise<void> {
    if (!solved) return;
    try {
      await navigator.clipboard.writeText(window.location.href);
      shareStatus = 'Link copied';
    } catch {
      shareStatus = 'Could not copy';
    }
    setTimeout(() => (shareStatus = ''), 2000);
  }
</script>

<PageHeader
  title="Solve"
  status={shareStatus || feedbackText}
  statusTone={feedbackError ? 'error' : feedbackText === 'Solved.' ? 'success' : 'default'}
  onShare={handleShare}
/>

<div class="tab-content">
  <div class="card">
    <label
      for="solve-input"
      style="display:block; font-size:12px; font-weight:600; color:var(--muted);
             text-transform:uppercase; letter-spacing:0.04em; margin-bottom:6px;"
    >
      Targets
      <span style="text-transform:none; font-weight:500;"> · rows then columns</span>
    </label>
    <input
      id="solve-input"
      type="text"
      class="solve-input"
      placeholder="r1,r2,…,rN,c1,c2,…,cN"
      bind:value={inputText}
      onkeydown={(e) => {
        if (e.key === 'Enter') solveFromInput();
      }}
    />
    <button type="button" class="solve-btn" onclick={solveFromInput}>
      <!-- Wand icon -->
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M5 19L15 9" />
        <path d="M14 8l2 2" />
        <path d="M18 4v3M16.5 5.5h3" />
        <path d="M6 5v2M5 6h2" />
        <path d="M19 14v2M18 15h2" />
      </svg>
      Solve
    </button>
  </div>

  {#if solved}
    <div class="card" style="margin-top:14px;">
      <div
        style="display:flex; align-items:center; justify-content:space-between; margin-bottom:10px;"
      >
        <span style="font-size:13px; font-weight:600; color:var(--ink);">Solution</span>
        <span class="solve-badge">Unique</span>
      </div>
      <div style="display:flex; justify-content:center;">
        <PuzzleGrid puzzle={solved} values={solved.cells} />
      </div>
    </div>
  {/if}
</div>
