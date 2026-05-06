import type { ClassifiedPuzzle, PuzzleData } from './types';
import { classifyPuzzle } from '../wasm/api';
import { puzzleKey } from './puzzle.svelte';

export type ClassificationResult = ClassifiedPuzzle | { error: string };

export type Difficulty = 'invalid' | 'normal' | 'hard' | 'very-hard' | 'extremely-hard';

export function difficulty(c: ClassifiedPuzzle, size: number): Difficulty {
  if (c.variant !== 'unique') return 'invalid';
  if (c.search_nodes <= 1) return 'normal';
  if (c.search_nodes <= size) return 'hard';
  if (c.search_nodes > 100) return 'extremely-hard';
  return 'very-hard';
}

export function classificationLabel(c: ClassificationResult | null, size: number): string {
  if (!c) return '';
  if ('error' in c) return c.error;
  if (c.variant === 'unsolvable') return 'No solution';
  if (c.variant === 'multiple') return 'Multiple solutions';
  switch (difficulty(c, size)) {
    case 'normal':
      return 'Normal';
    case 'hard':
      return 'Hard';
    case 'very-hard':
      return 'Very hard';
    case 'extremely-hard':
      return 'Extremely hard';
    default:
      return '';
  }
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
