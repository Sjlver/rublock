<script lang="ts">
  import { onMount } from 'svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import InputBar from '../InputBar.svelte';
  import EmojiRain from '../EmojiRain.svelte';
  import PageHeader from '../PageHeader.svelte';
  import {
    playState,
    applyUserNote,
    applyUserValue,
    checkCurrentPuzzle,
    clearSelection,
    newPuzzle,
    moveSelection,
    onSolved,
    redoInput,
    selectCell,
    switchToSize,
    toggleInputMode,
    undoInput,
  } from '../../state/puzzle.svelte';
  import { puzzleShareUrl } from '../../state/url.svelte';
  import { trackEvent } from '../../analytics';

  let showEmojiRain = $state(false);
  let hintDismissed = $state<boolean>(
    (() => {
      try {
        return localStorage.getItem('rublock-hint-dismissed') === '1';
      } catch {
        return false;
      }
    })()
  );

  // Toast system: transient messages under the page title
  type ToastTone = 'error' | 'success' | 'info';
  let toastText = $state('');
  let toastTone = $state<ToastTone>('info');
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  let status = $state('Ready');

  function showToast(text: string, tone: ToastTone = 'info', durationMs = 2400): void {
    if (toastTimer) clearTimeout(toastTimer);
    toastText = text;
    toastTone = tone;
    toastTimer = setTimeout(() => {
      toastText = '';
    }, durationMs);
  }

  // Mirror puzzle state feedback into toasts
  let prevFeedback = '';
  $effect(() => {
    const text = playState.feedback;
    if (!text || text === prevFeedback) return;
    prevFeedback = text;
    const tone: ToastTone = playState.feedbackError
      ? 'error'
      : text.includes('solved')
        ? 'success'
        : 'info';
    showToast(text, tone);
  });

  // Generating status
  $effect(() => {
    const data = playState.puzzleData;
    if (data) status = 'Ready';
  });

  let displayStatus = $derived(toastText || status);
  let displayTone = $derived(toastText ? toastTone : 'info');

  onMount(() => {
    const offSolved = onSolved(() => {
      showEmojiRain = false;
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
      applyUserNote(null);
      return;
    }

    if (key === '0' || key === 'b' || key === 'x') {
      event.preventDefault();
      // Keyboard: follow inputMode like before
      if (playState.inputMode === 'notes') applyUserNote('black');
      else applyUserValue('black');
      return;
    }

    if (key === '9' || key === 'o') {
      event.preventDefault();
      applyUserNote('digits-only');
      return;
    }

    if (/^[1-9]$/.test(key)) {
      const digit = Number.parseInt(key, 10);
      if (digit <= playState.puzzleData.row_targets.length - 2) {
        event.preventDefault();
        if (playState.inputMode === 'notes') applyUserNote(digit);
        else applyUserValue(digit);
      }
    }
  }

  function onPreviewClick(event: MouseEvent): void {
    if (!playState.selectedCell) return;
    const target = event.target as Element | null;
    if (!target?.closest('.puzzle')) clearSelection();
  }

  async function shareCurrentPuzzle(): Promise<void> {
    if (!playState.puzzleData) return;
    const url = puzzleShareUrl(playState.puzzleData);
    trackEvent(`rublock/play/share/${playState.puzzleData.row_targets.length}`);
    try {
      if (navigator.share && /Mobi|Android|iPhone|iPad/i.test(navigator.userAgent)) {
        await navigator.share({ title: 'Doplo puzzle', text: 'Try this Doplo puzzle:', url });
        return;
      }
      await navigator.clipboard.writeText(url);
      showToast('Link copied to clipboard', 'success');
    } catch {
      showToast('Could not share this puzzle', 'error');
    }
  }

  function handleSizeClick(s: number): void {
    status = 'Switching…';
    switchToSize(s);
    status = 'Ready';
  }

  function handleNewPuzzle(): void {
    if (!playState.puzzleData) return;
    status = 'Generating…';
    queueMicrotask(() => {
      newPuzzle(playState.puzzleData!.row_targets.length);
      status = 'Ready';
    });
  }

  function dismissHint(): void {
    hintDismissed = true;
    try {
      localStorage.setItem('rublock-hint-dismissed', '1');
    } catch {}
  }

  let cellExtras = $derived.by(() => {
    const map = new Map<string, { wrong?: boolean }>();
    for (const k of playState.wrongCells) map.set(k, { wrong: true });
    return map;
  });

  let undoDisabled = $derived(playState.historyIndex === 0);
  let redoDisabled = $derived(playState.historyIndex === playState.history.length);
  let inputDisabled = $derived(playState.selectedCell === null);
  let notesMode = $derived(playState.inputMode === 'notes');

  const SIZES = [5, 6, 7, 8];
  let currentSize = $derived(playState.puzzleData?.row_targets.length ?? 6);
</script>

<PageHeader
  title="Play"
  status={displayStatus}
  statusTone={displayTone === 'error' ? 'error' : displayTone === 'success' ? 'success' : 'default'}
  onShare={shareCurrentPuzzle}
/>

<div class="tab-content">
  <!-- Size selector + New puzzle -->
  <div style="display:flex; align-items:center; gap:8px; margin-bottom:10px;">
    <div class="size-selector">
      {#each SIZES as s (s)}
        <button
          type="button"
          class="size-btn"
          class:active={s === currentSize}
          onclick={() => handleSizeClick(s)}
        >
          {s}×{s}
        </button>
      {/each}
    </div>
    <button
      type="button"
      style="flex:1; height:36px; border-radius:12px; border:1px solid var(--line-2);
             background:var(--card); color:var(--ink); font-size:13px; font-weight:600;
             display:inline-flex; align-items:center; justify-content:center; gap:6px;
             cursor:pointer; font-family:inherit;"
      onclick={handleNewPuzzle}
    >
      <!-- Refresh icon -->
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
        <path d="M20 8a8 8 0 10-1 9.5" />
        <path d="M20 4v4h-4" />
      </svg>
      New puzzle
    </button>
  </div>

  <!-- Board -->
  <div
    style="display:flex; justify-content:center; padding:6px 0 14px;"
    onclick={onPreviewClick}
    role="presentation"
  >
    {#if playState.puzzleData}
      <PuzzleGrid
        puzzle={playState.puzzleData}
        values={playState.cellValues}
        notes={playState.cellNotes}
        selected={playState.selectedCell}
        inputMode={playState.inputMode}
        {cellExtras}
        onCellClick={selectCell}
      />
    {/if}
  </div>

  <!-- Undo / Redo / Check toolbar -->
  <div class="toolbar" style="margin-bottom:12px;">
    <button
      type="button"
      class="toolbar-btn"
      disabled={undoDisabled}
      onclick={undoInput}
      aria-label="Undo"
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
        <path d="M9 7L4.5 11.5 9 16" />
        <path d="M4.5 11.5h10a5 5 0 010 10H12" />
      </svg>
      Undo
    </button>
    <button
      type="button"
      class="toolbar-btn"
      disabled={redoDisabled}
      onclick={redoInput}
      aria-label="Redo"
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
        <path d="M15 7l4.5 4.5L15 16" />
        <path d="M19.5 11.5h-10a5 5 0 100 10H12" />
      </svg>
      Redo
    </button>
    <!-- Check: secondary style (not primary blue) per design notes -->
    <button
      type="button"
      class="toolbar-btn"
      onclick={checkCurrentPuzzle}
      aria-label="Check answers"
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
        <circle cx="12" cy="12" r="8.5" />
        <path d="M8 12.2l2.7 2.7L16 9.6" />
      </svg>
      Check
    </button>
  </div>

  <!-- Keyboard mode badge (visible when in notes mode, helps keyboard users) -->
  {#if notesMode && !inputDisabled}
    <div style="margin-bottom:8px; display:flex; align-items:center; gap:6px;">
      <span class="mode-badge">
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        >
          <path d="M14.5 5.5l4 4" />
          <path d="M3.5 20.5l3.5-1 11-11-3.5-3.5-11 11-1 3.5z" />
        </svg>
        Notes mode
      </span>
      <span style="font-size:11px; color:var(--muted);">Space to switch</span>
    </div>
  {/if}

  <!-- Hint chip (dismissible, teaches the long-press gesture) -->
  {#if !hintDismissed}
    <div class="hint-chip" style="margin-bottom:10px;">
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M14.5 5.5l4 4" />
        <path d="M3.5 20.5l3.5-1 11-11-3.5-3.5-11 11-1 3.5z" />
      </svg>
      <span class="hint-chip-text">Tap for a note · hold for the answer</span>
      <button type="button" class="hint-dismiss" onclick={dismissHint} aria-label="Dismiss hint">
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        >
          <path d="M6 6l12 12M18 6L6 18" />
        </svg>
      </button>
    </div>
  {/if}

  <!-- Number pad -->
  {#if playState.puzzleData}
    <InputBar
      size={playState.puzzleData.row_targets.length}
      disabled={inputDisabled}
      onPlaceNote={(v) => applyUserNote(v)}
      onPlaceValue={(v) => applyUserValue(v)}
      onErase={() => applyUserNote(null)}
    />
  {/if}
</div>

{#if showEmojiRain}
  <EmojiRain onDone={() => (showEmojiRain = false)} />
{/if}
