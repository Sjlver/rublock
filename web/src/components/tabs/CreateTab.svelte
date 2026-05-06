<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { setTab, puzzleShareUrl } from '../../state/url.svelte';
  import { replacePlayPuzzle } from '../../state/puzzle.svelte';
  import { showToast, toastState } from '../../state/toast.svelte';
  import { shareUrl } from '../../share';
  import { trackEvent } from '../../analytics';
  import { classifyPuzzle } from '../../wasm/api';
  import {
    classificationLabel,
    classificationTone,
    type ClassificationResult,
  } from '../../state/classification';
  import {
    SUPPORTED_SIZES,
    activeDraft,
    createState,
    draftAsPuzzle,
    ensureDraft,
    maxTargetForSize,
    selectTarget,
    setSelectedTargetValue,
    type SupportedSize,
  } from '../../state/create.svelte';

  // Materialize the draft for the current size on mount.
  ensureDraft(createState.size);

  // The active draft is whatever is stored at `drafts[size]` — read via a
  // $derived so Svelte tracks both the active size and its draft contents.
  let draft = $derived(activeDraft());

  function handleSizeClick(s: SupportedSize): void {
    if (s === createState.size) return;
    ensureDraft(s);
  }

  function handleTargetClick(axis: 'row' | 'col', index: number): void {
    selectTarget(axis, index);
  }

  function handleValueClick(value: number): void {
    setSelectedTargetValue(value);
  }

  // ── Live classification ────────────────────────────────────────────────────
  // We re-run `classify_puzzle` whenever the draft's targets change. The wasm
  // call is cheap for small sizes (5–7) and tolerable on 8. Drafts are
  // user-edited targets so we don't reuse the shared cache here — every edit
  // produces a new key anyway.
  let classification = $state<ClassificationResult | null>(null);

  $effect(() => {
    if (!draft) {
      classification = null;
      return;
    }
    const puzzle = draftAsPuzzle(draft);
    try {
      classification = classifyPuzzle(puzzle);
    } catch (err) {
      classification = { error: err instanceof Error ? err.message : String(err) };
    }
  });

  let isUnique = $derived(
    classification !== null && !('error' in classification) && classification.variant === 'unique'
  );

  // ── Use this puzzle ────────────────────────────────────────────────────────

  function handleUsePuzzle(): void {
    if (!draft || !isUnique) return;
    const data = draftAsPuzzle(draft);
    replacePlayPuzzle(data);
    trackEvent(`rublock/create/use/${data.row_targets.length}`);
    setTab('play');
  }

  // ── Share ──────────────────────────────────────────────────────────────────

  async function handleShare(): Promise<void> {
    if (!draft) return;
    const c = classification;
    if (!c || 'error' in c || c.variant !== 'unique') {
      showToast("Can't share an invalid puzzle", 'error', 2000);
      return;
    }
    const data = draftAsPuzzle(draft);
    const url = puzzleShareUrl(data);
    trackEvent(`rublock/create/share/${data.row_targets.length}`);
    await shareUrl(url);
  }

  let displayStatus = $derived(
    toastState.text || classificationLabel(classification, createState.size)
  );
  let displayTone = $derived(
    toastState.text ? toastState.tone : classificationTone(classification)
  );

  // ── Value strip ────────────────────────────────────────────────────────────
  let max = $derived(maxTargetForSize(createState.size));
  let values = $derived(Array.from({ length: max + 1 }, (_, i) => i));
  let valueStripDisabled = $derived(draft?.selected == null);
</script>

<PageHeader
  title="Create"
  status={displayStatus}
  statusTone={displayTone === 'error' ? 'error' : displayTone === 'success' ? 'success' : 'default'}
  onShare={handleShare}
/>

<div class="tab-content">
  <!-- Size selector -->
  <div style="display:flex; align-items:center; gap:8px; margin-bottom:10px;">
    <div class="size-selector">
      {#each SUPPORTED_SIZES as s (s)}
        <button
          type="button"
          class="size-btn"
          class:active={s === createState.size}
          onclick={() => handleSizeClick(s)}
        >
          {s}×{s}
        </button>
      {/each}
    </div>
  </div>

  <!-- Authoring grid: targets are tappable; cells are not filled. -->
  {#if draft}
    <div style="display:flex; justify-content:center; padding:6px 0 14px;">
      <PuzzleGrid
        puzzle={{
          row_targets: draft.rowTargets,
          col_targets: draft.colTargets,
        }}
        onTargetClick={handleTargetClick}
        selectedTarget={draft.selected}
      />
    </div>
  {/if}

  <!-- Value strip: tap to write into the selected target -->
  <div class="create-values" role="group" aria-label="Target value buttons">
    {#each values as v (v)}
      <button
        type="button"
        class="create-value-btn"
        disabled={valueStripDisabled}
        onclick={() => handleValueClick(v)}
        aria-label={`Set target to ${v}`}
      >
        {v}
      </button>
    {/each}
  </div>

  <!-- Actions -->
  <div class="toolbar" style="margin-top:12px;">
    <button
      type="button"
      class="toolbar-btn"
      disabled={!isUnique}
      onclick={handleUsePuzzle}
      aria-label="Use this puzzle"
      title={isUnique
        ? 'Send this puzzle to the Play tab'
        : 'Only puzzles with a unique solution can be used'}
    >
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
        <path d="M5 12l5 5L19 7" />
      </svg>
      Use this puzzle
    </button>
  </div>
</div>
