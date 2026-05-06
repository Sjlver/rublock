import { ensurePlayPuzzleForSize } from './puzzle.svelte';
import type { PuzzleData } from './types';

export type TargetAxis = 'row' | 'col';

/** Selection: which target cell the user is editing. */
export interface SelectedTarget {
  axis: TargetAxis;
  index: number;
}

/** Per-size draft for the Create tab. */
export interface CreateDraft {
  size: number;
  rowTargets: number[];
  colTargets: number[];
  selected: SelectedTarget | null;
}

export const SUPPORTED_SIZES = [5, 6, 7, 8] as const;
export type SupportedSize = (typeof SUPPORTED_SIZES)[number];

// Drafts live entirely inside `createState` so Svelte 5's $state proxy
// tracks them correctly. Mutating `createState.drafts[size].rowTargets[i]`
// goes through the proxy's setters and stays consistent across size
// switches; storing draft objects in a separate plain Map would risk
// drifting between the proxy's view and the cached reference.
export const createState = $state({
  size: 6 as SupportedSize,
  drafts: {} as Partial<Record<SupportedSize, CreateDraft>>,
});

function makeDraftFromPuzzle(data: PuzzleData): CreateDraft {
  return {
    size: data.row_targets.length,
    rowTargets: [...data.row_targets],
    colTargets: [...data.col_targets],
    selected: null,
  };
}

/**
 * Materialize the draft for `size`.
 *
 * If we have a saved draft, use it. Otherwise seed the draft from the Play
 * tab's puzzle for that size — generating one if Play has not visited that
 * size yet. The latter call also caches the puzzle in Play's `sizeStates`
 * so the two tabs stay consistent.
 */
export function ensureDraft(size: SupportedSize): CreateDraft {
  if (!createState.drafts[size]) {
    const fromPlay = ensurePlayPuzzleForSize(size);
    createState.drafts[size] = makeDraftFromPuzzle(fromPlay);
  }
  createState.size = size;
  return createState.drafts[size]!;
}

/** Currently active draft, or `null` if not yet materialized. */
export function activeDraft(): CreateDraft | null {
  return createState.drafts[createState.size] ?? null;
}

export function selectTarget(axis: TargetAxis, index: number): void {
  const d = activeDraft();
  if (!d) return;
  d.selected = { axis, index };
}

/** Write `value` into the selected target cell. */
export function setSelectedTargetValue(value: number): void {
  const d = activeDraft();
  if (!d || !d.selected) return;
  const { axis, index } = d.selected;
  if (axis === 'row') {
    d.rowTargets[index] = value;
  } else {
    d.colTargets[index] = value;
  }
}

export function draftAsPuzzle(d: CreateDraft): PuzzleData {
  return { row_targets: [...d.rowTargets], col_targets: [...d.colTargets] };
}

/** Largest legal target for a grid of size N: (N-2)(N-1)/2. */
export function maxTargetForSize(size: number): number {
  if (size < 2) return 0;
  return ((size - 2) * (size - 1)) / 2;
}
