<script lang="ts">
  import type { CellValue } from '../state/types';

  interface Props {
    size: number;
    disabled: boolean;
    notesMode: boolean;
    onApply: (value: CellValue | 'digits-only') => void;
  }

  let { size, disabled, notesMode, onApply }: Props = $props();
</script>

<div class="input-bar" onclickcapture={(e) => e.stopPropagation()} role="group">
  <button type="button" class="btn-input" {disabled} onclick={() => onApply('black')}>
    BLACK
  </button>
  {#each Array(size - 2) as _, i (i)}
    {@const n = i + 1}
    <button type="button" class="btn-input" {disabled} onclick={() => onApply(n)}>{n}</button>
  {/each}
  <button
    type="button"
    class="btn-input"
    disabled={disabled || !notesMode}
    onclick={() => onApply('digits-only')}
  >
    O
  </button>
  <button type="button" class="btn-input" {disabled} onclick={() => onApply(null)}>CLEAR</button>
</div>
