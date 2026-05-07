import init, {
  generate_puzzle,
  solve_puzzle,
  explain_puzzle,
  classify_puzzle,
} from './pkg/rublock.js';
// `?url` returns the bundled URL of the .wasm asset. We pass it explicitly to
// `init()` instead of relying on `import.meta.url`, so Vite hashes and ships
// the file like any other asset.
import wasmUrl from './pkg/rublock_bg.wasm?url';

import type { ClassifiedPuzzle, ExplainedPuzzle, PuzzleData, SolvedPuzzle } from '../state/types';
import { t, tf } from '../i18n/index.svelte';

let initPromise: Promise<unknown> | null = null;

export function initWasm(): Promise<unknown> {
  if (!initPromise) initPromise = init(wasmUrl);
  return initPromise;
}

// The wasm exports throw on the failure path (size out of range, unsolvable
// puzzle, multiple solutions, …). The thrown values are plain English strings
// from src/wasm.rs. `translateWasmError` matches them against the known set
// and re-throws an Error with a localized message; unknown errors pass
// through unchanged so unexpected failures still surface verbatim.
// `serde-wasm-bindgen` builds plain JS objects on the Rust side, so no
// `JSON.parse` is needed.

function translateWasmError(err: unknown): Error {
  const raw = err instanceof Error ? err.message : String(err);

  // Exact-match cases
  switch (raw) {
    case 'row_targets length does not match puzzle size':
      return new Error(t('err_row_targets_length'));
    case 'col_targets length does not match puzzle size':
      return new Error(t('err_col_targets_length'));
    case 'row_targets and col_targets must have the same length':
      return new Error(t('err_targets_length_mismatch'));
    case 'size must be 5–8':
      return new Error(t('err_size_range'));
    case 'puzzle is unsolvable':
      return new Error(t('err_unsolvable'));
    case 'puzzle has multiple solutions':
      return new Error(t('err_multiple_solutions'));
    case 'solver returned an incomplete state':
      return new Error(t('err_incomplete_state'));
  }

  // Out-of-range target carries dynamic numbers; parse them out.
  // Rust format: "target {t} is out of range (max is {max} for size {N})"
  const m = raw.match(/^target (\d+) is out of range \(max is (\d+) for size (\d+)\)$/);
  if (m) {
    return new Error(tf('err_target_out_of_range', { t: m[1], max: m[2], size: m[3] }));
  }

  return err instanceof Error ? err : new Error(raw);
}

function call<T>(fn: () => T): T {
  try {
    return fn();
  } catch (err) {
    throw translateWasmError(err);
  }
}

export function generatePuzzle(size: number): PuzzleData {
  return call(() => generate_puzzle(size) as PuzzleData);
}

export function solvePuzzle(data: PuzzleData): SolvedPuzzle {
  return call(
    () =>
      solve_puzzle(
        Uint8Array.from(data.row_targets),
        Uint8Array.from(data.col_targets)
      ) as SolvedPuzzle
  );
}

export function explainPuzzle(data: PuzzleData): ExplainedPuzzle {
  return call(
    () =>
      explain_puzzle(
        Uint8Array.from(data.row_targets),
        Uint8Array.from(data.col_targets)
      ) as ExplainedPuzzle
  );
}

export function classifyPuzzle(data: PuzzleData): ClassifiedPuzzle {
  return call(
    () =>
      classify_puzzle(
        Uint8Array.from(data.row_targets),
        Uint8Array.from(data.col_targets)
      ) as ClassifiedPuzzle
  );
}
