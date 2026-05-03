export type CellValue = number | 'black' | null;

export type NoteMarker = 'black' | 'digits-only' | null;

export interface CellNotes {
  digits: number[];
  marker: NoteMarker;
}

export interface PuzzleData {
  size: number;
  row_targets: number[];
  col_targets: number[];
}

export interface SolvedPuzzle extends PuzzleData {
  cells: CellValue[][];
}

export type SolveResponse = SolvedPuzzle | { error: string };

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

export type TabName = 'play' | 'solve' | 'explain' | 'print' | 'howto';

export type RuleName =
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
  rule: RuleName;
}

export interface ExplainStep {
  events: ExplainEvent[];
}

export interface ExplainResponse extends SolvedPuzzle {
  steps: ExplainStep[];
}
