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

export const createState = $state({
  size: 6 as SupportedSize,
  // Lazily built — see `ensureDraft`.
  draft: null as CreateDraft | null,
});

const drafts = new Map<number, CreateDraft>();

function makeDraftFromPuzzle(data: PuzzleData): CreateDraft {
  return {
    size: data.row_targets.length,
    rowTargets: [...data.row_targets],
    colTargets: [...data.col_targets],
    selected: null,
  };
}

/**
 * Materialize the draft for the current size.
 *
 * If we have a saved draft, use it. Otherwise seed the draft from the Play
 * tab's puzzle for that size — generating one if Play has not visited that
 * size yet. The latter call also caches the puzzle in Play's `sizeStates`
 * so the two tabs stay consistent.
 */
export function ensureDraft(size: SupportedSize): CreateDraft {
  const cached = drafts.get(size);
  if (cached) {
    createState.draft = cached;
    createState.size = size;
    return cached;
  }
  const fromPlay = ensurePlayPuzzleForSize(size);
  const fresh = makeDraftFromPuzzle(fromPlay);
  drafts.set(size, fresh);
  createState.draft = fresh;
  createState.size = size;
  return fresh;
}

export function switchCreateSize(size: SupportedSize): void {
  if (createState.size === size && createState.draft) return;
  ensureDraft(size);
}

export function selectTarget(axis: TargetAxis, index: number): void {
  if (!createState.draft) return;
  createState.draft.selected = { axis, index };
}

export function clearSelection(): void {
  if (!createState.draft) return;
  createState.draft.selected = null;
}

/** Write `value` into the selected target cell. */
export function setSelectedTargetValue(value: number): void {
  const d = createState.draft;
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
