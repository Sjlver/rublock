<script lang="ts">
  import type { CellValue, CellNotes, PuzzleData, SelectedCell } from '../state/types';
  import { noteDigitBit, NOTE_BLACK_BIT, NOTE_DIGITS_ONLY_BIT } from '../state/types';
  import { notesHaveContent } from '../state/puzzle.svelte';

  type CellExtras = { wrong?: boolean; exNew?: boolean };
  type TargetAxis = 'row' | 'col';

  interface Props {
    puzzle: PuzzleData;
    values?: CellValue[][] | null;
    notes?: CellNotes[][] | null;
    selected?: SelectedCell | null;
    inputMode?: 'value' | 'notes';
    cellExtras?: Map<string, CellExtras> | null;
    onCellClick?: (row: number, col: number) => void;
    /** When provided, target cells become tappable (Create tab). */
    onTargetClick?: (axis: TargetAxis, index: number) => void;
    selectedTarget?: { axis: TargetAxis; index: number } | null;
  }

  let {
    puzzle,
    values = null,
    notes = null,
    selected = null,
    inputMode = 'value',
    cellExtras = null,
    onCellClick,
    onTargetClick,
    selectedTarget = null,
  }: Props = $props();

  function cellKey(r: number, c: number): string {
    return `${r},${c}`;
  }

  function valueAt(r: number, c: number): CellValue {
    return values ? values[r][c] : null;
  }

  function notesAt(r: number, c: number): CellNotes | null {
    return notes ? notes[r][c] : null;
  }

  function targetKeyHandler(axis: TargetAxis, index: number) {
    return (e: KeyboardEvent) => {
      if (!onTargetClick) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        onTargetClick(axis, index);
      }
    };
  }
</script>

<div class="puzzle-wrap">
  <table class="puzzle">
    <thead>
      <tr>
        <th></th>
        {#each puzzle.col_targets as t, c (c)}
          <th
            scope="col"
            class="target"
            class:target-clickable={!!onTargetClick}
            class:target-selected={selectedTarget?.axis === 'col' && selectedTarget?.index === c}
            onclick={onTargetClick ? () => onTargetClick('col', c) : undefined}
            onkeydown={onTargetClick ? targetKeyHandler('col', c) : undefined}
            tabindex={onTargetClick ? 0 : undefined}
            role={onTargetClick ? 'button' : undefined}
            aria-label={onTargetClick ? `Column ${c + 1} target` : undefined}
            aria-pressed={onTargetClick
              ? selectedTarget?.axis === 'col' && selectedTarget?.index === c
              : undefined}
          >
            {t}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each puzzle.row_targets as rowTarget, r (r)}
        <tr>
          <th
            scope="row"
            class="target"
            class:target-clickable={!!onTargetClick}
            class:target-selected={selectedTarget?.axis === 'row' && selectedTarget?.index === r}
            onclick={onTargetClick ? () => onTargetClick('row', r) : undefined}
            onkeydown={onTargetClick ? targetKeyHandler('row', r) : undefined}
            tabindex={onTargetClick ? 0 : undefined}
            role={onTargetClick ? 'button' : undefined}
            aria-label={onTargetClick ? `Row ${r + 1} target` : undefined}
            aria-pressed={onTargetClick
              ? selectedTarget?.axis === 'row' && selectedTarget?.index === r
              : undefined}
          >
            {rowTarget}
          </th>
          {#each Array(puzzle.col_targets.length) as _, c (c)}
            {@const v = valueAt(r, c)}
            {@const n = notesAt(r, c)}
            {@const extras = cellExtras?.get(cellKey(r, c))}
            {@const isSelected = selected?.row === r && selected?.col === c}
            <td
              class="cell"
              class:black={v === 'black'}
              class:selected={isSelected}
              class:notes-mode={isSelected && inputMode === 'notes'}
              class:wrong={extras?.wrong}
              class:ex-new={extras?.exNew}
              onclick={onCellClick ? () => onCellClick(r, c) : undefined}
            >
              {#if v === 'black'}
                <span class="cell-value">X</span>
              {:else if v !== null}
                <span class="cell-value">{v}</span>
              {:else if n !== null && notesHaveContent(n)}
                <div class="cell-notes">
                  {#each [1, 2, 3, 4, 5, 6] as d (d)}
                    {#if n & noteDigitBit(d)}
                      <span class="note note-{d}">{d}</span>
                    {/if}
                  {/each}
                  {#if n & NOTE_BLACK_BIT}
                    <span class="note note-marker">x</span>
                  {:else if n & NOTE_DIGITS_ONLY_BIT}
                    <span class="note note-marker">o</span>
                  {/if}
                </div>
              {/if}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
