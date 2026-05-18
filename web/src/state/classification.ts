import type { ClassifiedPuzzle, PuzzleData } from './types';
import { classifyPuzzle } from '../wasm/api';
import type { PuzzleConstraints } from '../wasm/api';
import { puzzleKey } from './puzzle.svelte';
import { t } from '../i18n/index.svelte';

export type ClassificationResult = ClassifiedPuzzle | { error: string };

export type Difficulty =
  | 'invalid'
  | 'easy'
  | 'medium'
  | 'challenging'
  | 'hard'
  | 'very-hard'
  | 'extremely-hard';

// Per-size wave thresholds for the propagation-only regime (search_nodes <= 1).
//
// Picked from a 1000-sample calibration run per size for N=5..7 (productive
// waves at the 33rd and 66th percentiles, so the three buckets are roughly
// equal-sized within propagation-only puzzles). N=8 is extrapolated from the
// N=5..7 trend — fully-propagation-solvable 8×8 puzzles are rare enough that
// a fresh calibration sample would take hours. See issue #46. The table
// covers the sizes the UI offers (5..=8); other sizes fall back to a
// heuristic.
const WAVE_THRESHOLDS: Record<number, { easy: number; medium: number }> = {
  5: { easy: 4, medium: 5 },
  6: { easy: 7, medium: 9 },
  7: { easy: 11, medium: 13 },
  8: { easy: 16, medium: 18 },
};

function propagationBucket(waves: number, size: number): 'easy' | 'medium' | 'challenging' {
  // Fallback for sizes outside the calibrated table: rough linear scaling.
  const t = WAVE_THRESHOLDS[size] ?? { easy: 2 * size - 6, medium: 2 * size - 3 };
  if (waves <= t.easy) return 'easy';
  if (waves <= t.medium) return 'medium';
  return 'challenging';
}

export function difficulty(c: ClassifiedPuzzle, size: number): Difficulty {
  if (c.variant !== 'unique') return 'invalid';
  if (c.search_nodes <= 1) return propagationBucket(c.propagation_waves, size);
  if (c.search_nodes <= size) return 'hard';
  if (c.search_nodes > 100) return 'extremely-hard';
  return 'very-hard';
}

/** Difficulties the user can pick from the New-puzzle dropdown, in the order
 *  they appear (easiest first). `invalid` is omitted — it's a classification
 *  outcome, not something to generate. */
export const SELECTABLE_DIFFICULTIES: Exclude<Difficulty, 'invalid'>[] = [
  'easy',
  'medium',
  'challenging',
  'hard',
  'very-hard',
  'extremely-hard',
];

const U32_MAX = 0xffffffff;

/** Inverse of `difficulty()`: the (search_nodes, propagation_waves) windows
 *  that classify to each difficulty for a given puzzle size. Used by the
 *  difficulty dropdown to ask the WASM generator for a matching puzzle. */
export function constraintsForDifficulty(
  d: Exclude<Difficulty, 'invalid'>,
  size: number
): PuzzleConstraints {
  const wt = WAVE_THRESHOLDS[size] ?? { easy: 2 * size - 6, medium: 2 * size - 3 };
  switch (d) {
    case 'easy':
      return { minNodes: 0, maxNodes: 1, minWaves: 0, maxWaves: wt.easy };
    case 'medium':
      return { minNodes: 0, maxNodes: 1, minWaves: wt.easy + 1, maxWaves: wt.medium };
    case 'challenging':
      return { minNodes: 0, maxNodes: 1, minWaves: wt.medium + 1, maxWaves: U32_MAX };
    case 'hard':
      return { minNodes: 2, maxNodes: size, minWaves: 0, maxWaves: U32_MAX };
    case 'very-hard':
      return { minNodes: size + 1, maxNodes: 100, minWaves: 0, maxWaves: U32_MAX };
    case 'extremely-hard':
      return { minNodes: 101, maxNodes: U32_MAX, minWaves: 0, maxWaves: U32_MAX };
  }
}

/** Label for a difficulty bucket, regardless of any particular puzzle. */
export function difficultyLabel(d: Difficulty): string {
  switch (d) {
    case 'easy':
      return t('cls_easy');
    case 'medium':
      return t('cls_medium');
    case 'challenging':
      return t('cls_challenging');
    case 'hard':
      return t('cls_hard');
    case 'very-hard':
      return t('cls_very_hard');
    case 'extremely-hard':
      return t('cls_extremely_hard');
    default:
      return '';
  }
}

export function classificationLabel(c: ClassificationResult | null, size: number): string {
  if (!c) return '';
  if ('error' in c) return c.error;
  if (c.variant === 'unsolvable') return t('cls_no_solution');
  if (c.variant === 'multiple') return t('cls_multiple');
  return difficultyLabel(difficulty(c, size));
}

export function classificationTone(
  c: ClassificationResult | null
): 'default' | 'error' | 'success' {
  if (!c) return 'default';
  if ('error' in c) return 'error';
  if (c.variant === 'unique') return 'success';
  return 'error';
}

const cache = new Map<string, ClassificationResult>();

/** Classify `data`, caching by puzzle targets so repeat calls are free. */
export function classifyCached(data: PuzzleData): ClassificationResult {
  const key = puzzleKey(data);
  let entry = cache.get(key);
  if (!entry) {
    try {
      entry = classifyPuzzle(data);
    } catch (err) {
      entry = { error: err instanceof Error ? err.message : String(err) };
    }
    cache.set(key, entry);
  }
  return entry;
}
