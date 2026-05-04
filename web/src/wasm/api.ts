import init, { generate_puzzle, solve_puzzle, explain_puzzle } from './pkg/rublock.js';
// `?url` returns the bundled URL of the .wasm asset. We pass it explicitly to
// `init()` instead of relying on `import.meta.url`, so Vite hashes and ships
// the file like any other asset.
import wasmUrl from './pkg/rublock_bg.wasm?url';

import type { ExplainedPuzzle, PuzzleData, SolvedPuzzle } from '../state/types';

let initPromise: Promise<unknown> | null = null;

export function initWasm(): Promise<unknown> {
  if (!initPromise) initPromise = init(wasmUrl);
  return initPromise;
}

// The wasm exports throw on the failure path (size out of range, unsolvable
// puzzle, multiple solutions, …). Callers should wrap these in `try`/`catch`.
// `serde-wasm-bindgen` builds plain JS objects on the Rust side, so no
// `JSON.parse` is needed.

export function generatePuzzle(size: number): PuzzleData {
  return generate_puzzle(size) as PuzzleData;
}

export function solvePuzzle(data: PuzzleData): SolvedPuzzle {
  return solve_puzzle(
    Uint8Array.from(data.row_targets),
    Uint8Array.from(data.col_targets)
  ) as SolvedPuzzle;
}

export function explainPuzzle(data: PuzzleData): ExplainedPuzzle {
  return explain_puzzle(
    Uint8Array.from(data.row_targets),
    Uint8Array.from(data.col_targets)
  ) as ExplainedPuzzle;
}
