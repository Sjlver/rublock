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
  import type { PuzzleData } from '../../state/types';
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
  import { t, tf } from '../../i18n/index.svelte';

  // Materialize the draft for the current size on mount.
  ensureDraft(createState.size);

  // The active draft is whatever is stored at `drafts[size]` — read via a
  // $derived so Svelte tracks both the active size and its draft contents.
  let draft = $derived(activeDraft());

  function handleSizeClick(s: SupportedSize): void {
    if (s === createState.size) return;
    ensureDraft(s);
    refreshValidValues();
  }

  function handleTargetClick(axis: 'row' | 'col', index: number): void {
    selectTarget(axis, index);
    refreshValidValues();
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
      showToast(t('create_share_invalid'), 'error', 2000);
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

  // ── Per-value validity ─────────────────────────────────────────────────────
  // For the currently selected target, classify the puzzle that would result
  // from each candidate value 0..=max. Buttons backed by `unique`-solution
  // values get a "valid" highlight; unclassified buttons stay neutral so the
  // user can still tap them (they're not disabled).
  //
  // We classify one value at a time and yield to the event loop between calls
  // so the UI stays responsive on size 8. A monotonically increasing token
  // cancels the in-flight loop whenever the relevant inputs change.
  let validValues = $state<(boolean | undefined)[]>([]);
  let classifyToken = 0;

  function refreshValidValues(): void {
    const d = activeDraft();
    classifyToken++;
    if (!d || !d.selected) {
      validValues = [];
      return;
    }
    const sel = { axis: d.selected.axis, index: d.selected.index };
    const m = maxTargetForSize(d.size);
    const rowSnap = [...d.rowTargets];
    const colSnap = [...d.colTargets];
    const myToken = classifyToken;
    validValues = new Array<boolean | undefined>(m + 1).fill(undefined);

    void (async () => {
      for (let v = 0; v <= m; v++) {
        if (myToken !== classifyToken) return;
        const puzzle: PuzzleData = {
          row_targets: [...rowSnap],
          col_targets: [...colSnap],
        };
        if (sel.axis === 'row') puzzle.row_targets[sel.index] = v;
        else puzzle.col_targets[sel.index] = v;
        let isValid = false;
        try {
          isValid = classifyPuzzle(puzzle).variant === 'unique';
        } catch {
          isValid = false;
        }
        if (myToken !== classifyToken) return;
        validValues[v] = isValid;
        // Yield so the browser can paint the freshly classified button and
        // process input before we tackle the next value.
        await new Promise<void>((r) => setTimeout(r, 0));
      }
    })();
  }
</script>

<PageHeader
  title={t('create_title')}
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
  <div class="create-values" role="group" aria-label={t('create_values_aria')}>
    {#each values as v (v)}
      <button
        type="button"
        class="create-value-btn"
        class:valid={validValues[v] === true}
        disabled={valueStripDisabled}
        onclick={() => handleValueClick(v)}
        aria-label={tf('create_set_target_aria', { n: v })}
        data-classified={validValues[v] !== undefined}
        data-valid={validValues[v] === true}
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
      aria-label={t('create_use_aria')}
      title={isUnique ? t('create_use_title_valid') : t('create_use_title_invalid')}
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
      {t('create_use')}
    </button>
  </div>
</div>
