import init, { generate_puzzle, solve_puzzle, explain_puzzle } from './pkg/rublock.js';
// `?url` returns the bundled URL of the .wasm asset. We pass it explicitly to
// `init()` instead of relying on `import.meta.url`, so Vite hashes and ships
// the file like any other asset.
import wasmUrl from './pkg/rublock_bg.wasm?url';

import type { ExplainResponse, PuzzleData, SolveResponse } from '../state/types';

let initPromise: Promise<unknown> | null = null;

export function initWasm(): Promise<unknown> {
  if (!initPromise) initPromise = init(wasmUrl);
  return initPromise;
}

export function generatePuzzle(size: number): PuzzleData {
  const parsed = JSON.parse(generate_puzzle(size)) as PuzzleData & { error?: string };
  if (parsed.error) throw new Error(parsed.error);
  return parsed;
}

export function solvePuzzle(data: PuzzleData): SolveResponse {
  return JSON.parse(
    solve_puzzle(Uint8Array.from(data.row_targets), Uint8Array.from(data.col_targets))
  ) as SolveResponse;
}

export function explainPuzzle(data: PuzzleData): ExplainResponse {
  return JSON.parse(
    explain_puzzle(Uint8Array.from(data.row_targets), Uint8Array.from(data.col_targets))
  ) as ExplainResponse;
}
