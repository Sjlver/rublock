export type CellValue = number | 'black' | null;

// Bit 0: black marker
// Bits 1..=6: digits 1..6  (max digit is 6 for N=8)
// Bit 7: digits-only marker
export type CellNotes = number;
export const NOTE_BLACK_BIT = 1 << 0;
export const NOTE_DIGITS_ONLY_BIT = 1 << 7;
export function noteDigitBit(d: number): number {
  return 1 << d;
}

export interface PuzzleData {
  row_targets: number[];
  col_targets: number[];
}

export interface SolvedPuzzle extends PuzzleData {
  cells: CellValue[][];
}

export type ExplainRule =
  | 'TargetTuples'
  | 'ArcConsistency'
  | 'Singleton'
  | 'HiddenSingle'
  | 'BlackConsistency'
  | 'Backtracking';

export interface ExplainEvent {
  row: number;
  col: number;
  before: number;
  after: number;
  rule: ExplainRule;
}

export interface ExplainStep {
  events: ExplainEvent[];
}

export interface ExplainedPuzzle extends SolvedPuzzle {
  steps: ExplainStep[];
}

export interface CellOperation {
  row: number;
  col: number;
  oldValue: CellValue;
  newValue: CellValue;
  oldNotes: CellNotes;
  newNotes: CellNotes;
}

export interface SelectedCell {
  row: number;
  col: number;
}

export type InputMode = 'value' | 'notes';

export type TabName = 'play' | 'create' | 'print' | 'howto' | 'steps';

export type ClassifyVariant = 'unsolvable' | 'unique' | 'multiple';

export interface ClassifiedPuzzle {
  variant: ClassifyVariant;
  search_nodes: number;
  // Present for `unique` and `multiple`, omitted for `unsolvable`.
  cells?: CellValue[][];
}
