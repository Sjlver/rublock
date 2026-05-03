<script lang="ts">
  import type { CellValue, CellNotes, PuzzleData, SelectedCell } from '../state/types';
  import { notesHaveContent } from '../state/puzzle.svelte';

  type CellExtras = { wrong?: boolean; exNew?: boolean };

  interface Props {
    puzzle: PuzzleData;
    values?: CellValue[][] | null;
    notes?: CellNotes[][] | null;
    selected?: SelectedCell | null;
    inputMode?: 'value' | 'notes';
    cellExtras?: Map<string, CellExtras> | null;
    onCellClick?: (row: number, col: number) => void;
  }

  let {
    puzzle,
    values = null,
    notes = null,
    selected = null,
    inputMode = 'value',
    cellExtras = null,
    onCellClick,
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
</script>

<div class="puzzle-wrap">
  <table class="puzzle">
    <thead>
      <tr>
        <th></th>
        {#each puzzle.col_targets as t}
          <th scope="col" class="target">{t}</th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each puzzle.row_targets as rowTarget, r (r)}
        <tr>
          <th scope="row" class="target">{rowTarget}</th>
          {#each Array(puzzle.size) as _, c (c)}
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
                <!-- intentionally empty — black cell has dark bg -->
              {:else if v !== null}
                <span class="cell-value">{v}</span>
              {:else if n && notesHaveContent(n)}
                <div class="cell-notes">
                  {#each n.digits as d (d)}
                    {#if d >= 1 && d <= 7}
                      <span class="note note-{d}">{d}</span>
                    {/if}
                  {/each}
                  {#if n.marker === 'black' || n.marker === 'digits-only'}
                    <span class="note note-marker">{n.marker === 'black' ? 'x' : 'o'}</span>
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
