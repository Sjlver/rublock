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
    newPuzzleWithDifficulty,
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
  import { toastState } from '../../state/toast.svelte';
  import { shareUrl } from '../../share';
  import {
    classifyCached,
    classificationLabel,
    classificationTone,
    difficultyLabel,
    SELECTABLE_DIFFICULTIES,
    type Difficulty,
  } from '../../state/classification';
  import { t } from '../../i18n/index.svelte';
  import { readHintDismissed, writeHintDismissed } from '../../state/storage';
  import { isSupportedSize } from '../../state/storage.svelte';

  let showEmojiRain = $state(false);
  let hintDismissed = $state<boolean>(readHintDismissed());

  let status = $state('');

  // Classification of the active puzzle. Driven by `classify_puzzle` and cached
  // by puzzle targets, so size switches and "Use this puzzle" never re-classify
  // a puzzle we've already seen. Generated puzzles are always Normal but we
  // call `classify_puzzle` regardless — the cost is negligible at sizes 5–8.
  let classification = $derived(playState.puzzleData ? classifyCached(playState.puzzleData) : null);
  let currentSize = $derived(playState.puzzleData?.row_targets.length ?? 6);
  let classificationStatus = $derived(classificationLabel(classification, currentSize));
  let classificationStatusTone = $derived(classificationTone(classification));

  // Toast wins (it's transient feedback). Otherwise show transient
  // "Generating…/Switching…" if set, otherwise the classification chip.
  let displayStatus = $derived(toastState.text || status || classificationStatus);
  let displayTone = $derived(
    toastState.text ? toastState.tone : status ? 'default' : classificationStatusTone
  );

  onMount(() => {
    const offSolved = onSolved(() => {
      showEmojiRain = false;
      queueMicrotask(() => (showEmojiRain = true));
    });
    const onKey = (event: KeyboardEvent) => {
      closeMenuOnEscape(event);
      handlePlayKeydown(event);
    };
    document.addEventListener('keydown', onKey);
    document.addEventListener('mousedown', closeMenuOnOutsideClick);
    return () => {
      document.removeEventListener('keydown', onKey);
      document.removeEventListener('mousedown', closeMenuOnOutsideClick);
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

  function onBoardClick(event: MouseEvent): void {
    if (!playState.selectedCell) return;
    const target = event.target as Element | null;
    if (!target?.closest('.puzzle')) clearSelection();
  }

  async function shareCurrentPuzzle(): Promise<void> {
    if (!playState.puzzleData) return;
    const url = puzzleShareUrl(playState.puzzleData);
    trackEvent(`rublock/play/share/${playState.puzzleData.row_targets.length}`);
    await shareUrl(url);
  }

  function handleSizeClick(s: number): void {
    status = t('play_status_switching');
    switchToSize(s);
    status = '';
  }

  function handleNewPuzzle(): void {
    if (!playState.puzzleData) return;
    status = t('play_status_generating');
    queueMicrotask(() => {
      newPuzzle(playState.puzzleData!.row_targets.length);
      status = '';
    });
  }

  let difficultyMenuOpen = $state(false);

  function toggleDifficultyMenu(): void {
    difficultyMenuOpen = !difficultyMenuOpen;
  }

  function handleNewPuzzleWithDifficulty(d: Exclude<Difficulty, 'invalid'>): void {
    difficultyMenuOpen = false;
    if (!playState.puzzleData) return;
    status = t('play_status_generating');
    const targetsLength = playState.puzzleData!.row_targets.length;
    const size = isSupportedSize(targetsLength) ? targetsLength : 6;
    queueMicrotask(() => {
      try {
        newPuzzleWithDifficulty(size, d);
      } finally {
        status = '';
      }
    });
  }

  function closeMenuOnOutsideClick(event: MouseEvent): void {
    if (!difficultyMenuOpen) return;
    const target = event.target as Element | null;
    if (!target?.closest('.split-btn-wrap')) difficultyMenuOpen = false;
  }

  function closeMenuOnEscape(event: KeyboardEvent): void {
    if (difficultyMenuOpen && event.key === 'Escape') {
      event.preventDefault();
      difficultyMenuOpen = false;
    }
  }

  function dismissHint(): void {
    hintDismissed = true;
    writeHintDismissed(true);
  }

  let cellExtras = $derived.by(() => {
    const map = new Map<string, { wrong?: boolean }>();
    for (const k of playState.wrongCells) map.set(k, { wrong: true });
    return map;
  });

  let undoDisabled = $derived(playState.undoStack.length === 0);
  let redoDisabled = $derived(playState.redoStack.length === 0);
  let inputDisabled = $derived(playState.selectedCell === null);
  let notesMode = $derived(playState.inputMode === 'notes');

  const SIZES = [5, 6, 7, 8];
</script>

<PageHeader
  title={t('play_title')}
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
    <div class="split-btn-wrap" style="flex:1;">
      <div class="split-btn">
        <button
          type="button"
          class="split-btn-main"
          onclick={handleNewPuzzle}
          aria-label={t('play_new_puzzle')}
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
          {t('play_new_puzzle')}
        </button>
        <button
          type="button"
          class="split-btn-toggle"
          onclick={toggleDifficultyMenu}
          aria-haspopup="menu"
          aria-expanded={difficultyMenuOpen}
          aria-label={t('play_new_puzzle_difficulty_aria')}
        >
          <!-- Chevron-down icon -->
          <svg
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M6 9l6 6 6-6" />
          </svg>
        </button>
      </div>
      {#if difficultyMenuOpen}
        <div class="split-btn-menu" role="menu">
          {#each SELECTABLE_DIFFICULTIES as d (d)}
            <button
              type="button"
              role="menuitem"
              class="split-btn-menu-item"
              onclick={() => handleNewPuzzleWithDifficulty(d)}
            >
              {difficultyLabel(d)}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- Board -->
  <div
    style="display:flex; justify-content:center; padding:6px 0 14px;"
    onclick={onBoardClick}
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
      aria-label={t('play_undo_aria')}
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
      {t('play_undo')}
    </button>
    <button
      type="button"
      class="toolbar-btn"
      disabled={redoDisabled}
      onclick={redoInput}
      aria-label={t('play_redo_aria')}
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
      {t('play_redo')}
    </button>
    <button
      type="button"
      class="toolbar-btn"
      onclick={checkCurrentPuzzle}
      aria-label={t('play_check_aria')}
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
      {t('play_check')}
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
        {t('play_notes_mode')}
      </span>
      <span style="font-size:11px; color:var(--muted);">{t('play_space_to_switch')}</span>
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
      <span class="hint-chip-text">{t('play_hint_chip')}</span>
      <button
        type="button"
        class="hint-dismiss"
        onclick={dismissHint}
        aria-label={t('play_hint_dismiss_aria')}
      >
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
