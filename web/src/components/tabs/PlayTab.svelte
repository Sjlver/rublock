<script lang="ts">
  import { onMount } from 'svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import InputBar from '../InputBar.svelte';
  import EmojiRain from '../EmojiRain.svelte';
  import {
    playState,
    applyUserInput,
    applyUserNote,
    checkCurrentPuzzle,
    clearSelection,
    loadRandomPuzzle,
    moveSelection,
    onSolved,
    redoInput,
    selectCell,
    toggleInputMode,
    undoInput,
  } from '../../state/puzzle.svelte';
  import { puzzleShareUrl } from '../../state/url.svelte';

  interface Props {
    selectedSize: number;
  }

  let { selectedSize = $bindable() }: Props = $props();

  let shareFeedback = $state('');
  let shareError = $state(false);
  let showEmojiRain = $state(false);

  onMount(() => {
    const offSolved = onSolved(() => {
      showEmojiRain = false;
      // Re-mount the component on next tick so a fresh animation starts even
      // if the user solves the puzzle a second time after editing.
      queueMicrotask(() => (showEmojiRain = true));
    });
    const onKey = (event: KeyboardEvent) => handlePlayKeydown(event);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('keydown', onKey);
      offSolved();
    };
  });

  function isKeyboardInputTarget(target: EventTarget | null): boolean {
    return (
      target instanceof Element &&
      target.closest('input, textarea, select, [contenteditable="true"]') !== null
    );
  }

  function handlePlayKeydown(event: KeyboardEvent): void {
    if (!playState.puzzleData || isKeyboardInputTarget(event.target)) return;

    const key = event.key.toLowerCase();
    const moves: Record<string, [number, number]> = {
      arrowup: [-1, 0],
      w: [-1, 0],
      arrowdown: [1, 0],
      s: [1, 0],
      arrowleft: [0, -1],
      a: [0, -1],
      arrowright: [0, 1],
      d: [0, 1],
    };

    if (key in moves) {
      event.preventDefault();
      const [dr, dc] = moves[key];
      moveSelection(dr, dc);
      return;
    }

    if (key === ' ') {
      event.preventDefault();
      toggleInputMode();
      return;
    }

    if (key === 'backspace' || key === 'delete') {
      event.preventDefault();
      applyUserInput(null);
      return;
    }

    if (key === '0' || key === 'b' || key === 'x') {
      event.preventDefault();
      applyUserInput('black');
      return;
    }

    if (key === '9' || key === 'o') {
      event.preventDefault();
      applyUserNote('digits-only');
      return;
    }

    if (/^[1-9]$/.test(key)) {
      const digit = Number.parseInt(key, 10);
      if (digit <= playState.puzzleData.size - 2) {
        event.preventDefault();
        applyUserInput(digit);
      }
    }
  }

  function onSizeSelectChange(event: Event): void {
    const target = event.currentTarget as HTMLSelectElement;
    const next = Number.parseInt(target.value, 10);
    selectedSize = next;
    loadRandomPuzzle(next);
  }

  function onPreviewClick(event: MouseEvent): void {
    if (!playState.selectedCell) return;
    const target = event.target as Element | null;
    if (!target?.closest('.puzzle')) clearSelection();
  }

  async function shareCurrentPuzzle(): Promise<void> {
    if (!playState.puzzleData) return;
    const url = puzzleShareUrl(playState.puzzleData);
    shareError = false;
    shareFeedback = '';
    try {
      if (navigator.share && /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent)) {
        await navigator.share({ title: 'Doplo puzzle', text: 'Try this Doplo puzzle:', url });
        shareFeedback = 'Shared.';
        return;
      }
      await navigator.clipboard.writeText(url);
      shareFeedback = 'Copied!';
    } catch (err) {
      console.error('Share failed:', err);
      shareError = true;
      shareFeedback = 'Could not share this puzzle.';
    }
  }

  let cellExtras = $derived(() => {
    const map = new Map<string, { wrong?: boolean }>();
    for (const k of playState.wrongCells) map.set(k, { wrong: true });
    return map;
  });

  let undoDisabled = $derived(playState.historyIndex === 0);
  let redoDisabled = $derived(playState.historyIndex === playState.history.length);
  let checkDisabled = $derived(playState.puzzleData === null);
  let inputDisabled = $derived(playState.selectedCell === null);
  let notesMode = $derived(playState.inputMode === 'notes');
</script>

<section class="tab-panel">
  <div class="panel-card">
    <div class="controls-row">
      <div class="field">
        <label for="play-size">Size</label>
        <select id="play-size" value={selectedSize} onchange={onSizeSelectChange}>
          <option value="5">5 × 5</option>
          <option value="6">6 × 6</option>
          <option value="7">7 × 7</option>
          <option value="8">8 × 8</option>
        </select>
      </div>
      <button class="btn-ghost" type="button" onclick={() => loadRandomPuzzle(selectedSize)}>
        Generate another
      </button>
      <button class="btn-ghost" type="button" disabled={undoDisabled} onclick={undoInput}>
        Undo
      </button>
      <button class="btn-ghost" type="button" disabled={redoDisabled} onclick={redoInput}>
        Redo
      </button>
      <button
        class="btn-primary"
        type="button"
        disabled={checkDisabled}
        onclick={checkCurrentPuzzle}
      >
        Check
      </button>
      <span class="feedback" class:error={playState.feedbackError} aria-live="polite">
        {playState.feedback}
      </span>
    </div>

    <div class="preview" onclick={onPreviewClick} role="presentation">
      {#if playState.puzzleData}
        <PuzzleGrid
          puzzle={playState.puzzleData}
          values={playState.cellValues}
          notes={playState.cellNotes}
          selected={playState.selectedCell}
          inputMode={playState.inputMode}
          cellExtras={cellExtras()}
          onCellClick={selectCell}
        />
        <div class="input-mode-hint">
          {#if playState.selectedCell}
            {playState.inputMode === 'notes' ? 'Mode: Notes' : 'Mode: Values'}
          {/if}
        </div>
        <InputBar
          size={playState.puzzleData.size}
          disabled={inputDisabled}
          {notesMode}
          onApply={applyUserInput}
        />
      {/if}
    </div>

    <div class="controls-row">
      <button class="btn-ghost" type="button" onclick={shareCurrentPuzzle}>
        Share this puzzle
      </button>
      <span class="feedback" class:error={shareError} aria-live="polite">{shareFeedback}</span>
    </div>
  </div>
</section>

{#if showEmojiRain}
  <EmojiRain onDone={() => (showEmojiRain = false)} />
{/if}
