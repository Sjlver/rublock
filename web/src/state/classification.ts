import type { ClassifiedPuzzle, PuzzleData } from './types';
import { classifyPuzzle } from '../wasm/api';
import type { PuzzleConstraints } from '../wasm/api';
import { puzzleKey } from './puzzle.svelte';
import { t } from '../i18n/index.svelte';
import { isSupportedSize, type SupportedSize } from './storage.svelte';

export type ClassificationResult = ClassifiedPuzzle | { error: string };

export type Difficulty =
  | 'invalid'
  | 'easy'
  | 'medium'
  | 'challenging'
  | 'hard'
  | 'very-hard'
  | 'extremely-hard';

const U32_MAX = 0xffffffff;

const DIFFICULTY_THRESHOLDS = {
  5: [
    { difficulty: 'easy', minNodes: 1, maxNodes: 1, minWaves: 1, maxWaves: 3 },
    { difficulty: 'medium', minNodes: 1, maxNodes: 1, minWaves: 4, maxWaves: 6 },
    { difficulty: 'challenging', minNodes: 1, maxNodes: 1, minWaves: 7, maxWaves: U32_MAX },
    { difficulty: 'hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 1, maxWaves: 10 },
    { difficulty: 'very-hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 11, maxWaves: 17 },
    {
      difficulty: 'extremely-hard',
      minNodes: 2,
      maxNodes: U32_MAX,
      minWaves: 18,
      maxWaves: U32_MAX,
    },
  ],
  6: [
    { difficulty: 'easy', minNodes: 1, maxNodes: 1, minWaves: 1, maxWaves: 6 },
    { difficulty: 'medium', minNodes: 1, maxNodes: 1, minWaves: 7, maxWaves: 9 },
    { difficulty: 'challenging', minNodes: 1, maxNodes: 1, minWaves: 10, maxWaves: U32_MAX },
    { difficulty: 'hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 1, maxWaves: 14 },
    { difficulty: 'very-hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 15, maxWaves: 45 },
    {
      difficulty: 'extremely-hard',
      minNodes: 2,
      maxNodes: U32_MAX,
      minWaves: 46,
      maxWaves: U32_MAX,
    },
  ],
  7: [
    { difficulty: 'easy', minNodes: 1, maxNodes: 1, minWaves: 1, maxWaves: 8 },
    { difficulty: 'medium', minNodes: 1, maxNodes: 1, minWaves: 9, maxWaves: 15 },
    { difficulty: 'challenging', minNodes: 1, maxNodes: 1, minWaves: 16, maxWaves: U32_MAX },
    { difficulty: 'hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 1, maxWaves: 18 },
    { difficulty: 'very-hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 19, maxWaves: 350 },
    {
      difficulty: 'extremely-hard',
      minNodes: 2,
      maxNodes: U32_MAX,
      minWaves: 351,
      maxWaves: U32_MAX,
    },
  ],
  8: [
    { difficulty: 'easy', minNodes: 1, maxNodes: 1, minWaves: 1, maxWaves: 13 },
    { difficulty: 'medium', minNodes: 1, maxNodes: 1, minWaves: 14, maxWaves: 22 },
    { difficulty: 'challenging', minNodes: 1, maxNodes: 1, minWaves: 23, maxWaves: U32_MAX },
    { difficulty: 'hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 1, maxWaves: 100 },
    { difficulty: 'very-hard', minNodes: 2, maxNodes: U32_MAX, minWaves: 101, maxWaves: 10000 },
    {
      difficulty: 'extremely-hard',
      minNodes: 2,
      maxNodes: U32_MAX,
      minWaves: 10001,
      maxWaves: U32_MAX,
    },
  ],
} as const;

export function difficulty(c: ClassifiedPuzzle, size: SupportedSize): Difficulty {
  if (c.variant !== 'unique') return 'invalid';

  for (const { difficulty, maxNodes, maxWaves } of DIFFICULTY_THRESHOLDS[size]) {
    if (c.search_nodes <= maxNodes && c.propagation_waves <= maxWaves) {
      return difficulty;
    }
  }

  return 'invalid';
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

/** Inverse of `difficulty()`: the (search_nodes, propagation_waves) windows
 *  that classify to each difficulty for a given puzzle size. Used by the
 *  difficulty dropdown to ask the WASM generator for a matching puzzle. */
export function constraintsForDifficulty(
  difficulty: Exclude<Difficulty, 'invalid'>,
  size: SupportedSize
): PuzzleConstraints {
  for (const { difficulty: d, ...rest } of DIFFICULTY_THRESHOLDS[size]) {
    if (d == difficulty) {
      return rest;
    }
  }

  // unreachable
  return { minNodes: 1, maxNodes: 1, minWaves: 0, maxWaves: U32_MAX };
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
  if (isSupportedSize(size)) {
    return difficultyLabel(difficulty(c, size));
  }
  return '';
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
