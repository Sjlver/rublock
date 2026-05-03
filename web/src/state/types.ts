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

export type TabName = 'play' | 'solve' | 'print' | 'howto' | 'steps';
