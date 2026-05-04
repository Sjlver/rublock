import { SvelteSet } from 'svelte/reactivity';
import { generatePuzzle, solvePuzzle } from '../wasm/api';
import { trackEvent } from '../analytics';
import type {
  CellNotes,
  CellOperation,
  CellValue,
  InputMode,
  PuzzleData,
  SelectedCell,
  SolvedPuzzle,
} from './types';

export function emptyNotes(): CellNotes {
  return { digits: [], marker: null };
}

export function emptyCellNotes(size: number): CellNotes[][] {
  return Array.from({ length: size }, () => Array.from({ length: size }, () => emptyNotes()));
}

export function emptyCellValues(size: number): CellValue[][] {
  return Array.from({ length: size }, () => Array<CellValue>(size).fill(null));
}

function cloneNotes(notes: CellNotes): CellNotes {
  return { digits: [...notes.digits], marker: notes.marker };
}

function sortNoteDigitsInPlace(notes: CellNotes): void {
  notes.digits.sort((a, b) => a - b);
}

function sameNotes(a: CellNotes, b: CellNotes): boolean {
  if (a.marker !== b.marker) return false;
  if (a.digits.length !== b.digits.length) return false;
  const ad = [...a.digits].sort((x, y) => x - y);
  const bd = [...b.digits].sort((x, y) => x - y);
  return ad.every((v, i) => v === bd[i]);
}

export function notesHaveContent(notes: CellNotes): boolean {
  return notes.digits.length > 0 || notes.marker !== null;
}

export function cellKey(row: number, col: number): string {
  return `${row},${col}`;
}

export function puzzleKey(data: PuzzleData): string {
  return `${data.size}|${data.row_targets.join(',')}|${data.col_targets.join(',')}`;
}

function sameOperation(a: CellOperation | undefined, b: CellOperation | undefined): boolean {
  if (!a || !b) return false;
  return (
    a.row === b.row &&
    a.col === b.col &&
    a.oldValue === b.oldValue &&
    a.newValue === b.newValue &&
    sameNotes(a.oldNotes, b.oldNotes) &&
    sameNotes(a.newNotes, b.newNotes)
  );
}

export const playState = $state({
  puzzleData: null as PuzzleData | null,
  cellValues: [] as CellValue[][],
  cellNotes: [] as CellNotes[][],
  inputMode: 'value' as InputMode,
  selectedCell: null as SelectedCell | null,
  history: [] as CellOperation[],
  historyIndex: 0,
  wrongCells: new SvelteSet<string>(),
  feedback: '',
  feedbackError: false,
});

function setPuzzleData(
  data: PuzzleData,
  { preserveProgressIfSame = true }: { preserveProgressIfSame?: boolean } = {}
): void {
  const samePuzzle =
    playState.puzzleData !== null && puzzleKey(playState.puzzleData) === puzzleKey(data);
  const shouldReset = !(preserveProgressIfSame && samePuzzle);

  playState.puzzleData = data;
  if (shouldReset) {
    playState.cellValues = emptyCellValues(data.size);
    playState.cellNotes = emptyCellNotes(data.size);
    playState.selectedCell = null;
    playState.inputMode = 'value';
    playState.history = [];
    playState.historyIndex = 0;
    playState.wrongCells.clear();
    playState.feedback = '';
    playState.feedbackError = false;
  }
}

export function setPuzzle(data: PuzzleData, options?: { preserveProgressIfSame?: boolean }): void {
  setPuzzleData(data, options);
}

export function loadRandomPuzzle(size: number): void {
  setPuzzleData(generatePuzzle(size), { preserveProgressIfSame: false });
  trackEvent(`rublock/play/generate/${size}`);
}

interface PerSizeState {
  puzzleData: PuzzleData;
  cellValues: CellValue[][];
  cellNotes: CellNotes[][];
  selectedCell: SelectedCell | null;
  history: CellOperation[];
  historyIndex: number;
}

const sizeStates = new Map<number, PerSizeState>();

function saveCurrentState(): void {
  if (!playState.puzzleData) return;
  sizeStates.set(playState.puzzleData.size, {
    puzzleData: playState.puzzleData,
    cellValues: playState.cellValues.map((row) => [...row]),
    cellNotes: playState.cellNotes.map((row) => row.map((n) => cloneNotes(n))),
    selectedCell: playState.selectedCell,
    history: [...playState.history],
    historyIndex: playState.historyIndex,
  });
}

/** Switch to a new size, preserving in-progress puzzles per size. */
export function switchToSize(size: number): void {
  if (playState.puzzleData?.size === size) return;
  saveCurrentState();

  const saved = sizeStates.get(size);
  if (saved) {
    playState.puzzleData = saved.puzzleData;
    playState.cellValues = saved.cellValues;
    playState.cellNotes = saved.cellNotes;
    playState.selectedCell = saved.selectedCell;
    playState.history = saved.history;
    playState.historyIndex = saved.historyIndex;
    playState.wrongCells.clear();
    playState.feedback = '';
    playState.feedbackError = false;
  } else {
    setPuzzleData(generatePuzzle(size), { preserveProgressIfSame: false });
    trackEvent(`rublock/play/generate/${size}`);
  }
}

/** Generate a fresh puzzle for the current size, discarding any saved state. */
export function newPuzzle(size: number): void {
  sizeStates.delete(size);
  setPuzzleData(generatePuzzle(size), { preserveProgressIfSame: false });
  trackEvent(`rublock/play/generate/${size}`);
}

function clearWrongCell(row: number, col: number): void {
  playState.wrongCells.delete(cellKey(row, col));
  playState.feedback = '';
  playState.feedbackError = false;
}

function commitCellEdit(row: number, col: number, newValue: CellValue, newNotes: CellNotes): void {
  if (!playState.puzzleData) return;
  const oldValue = playState.cellValues[row][col];
  const oldNotes = cloneNotes(playState.cellNotes[row][col]);
  const nextNotes = cloneNotes(newNotes);
  sortNoteDigitsInPlace(nextNotes);

  if (oldValue === newValue && sameNotes(oldNotes, nextNotes)) return;

  const operation: CellOperation = {
    row,
    col,
    oldValue,
    newValue,
    oldNotes,
    newNotes: cloneNotes(nextNotes),
  };
  const nextOperation = playState.history[playState.historyIndex];
  if (sameOperation(operation, nextOperation)) {
    playState.historyIndex += 1;
  } else {
    playState.history = playState.history.slice(0, playState.historyIndex);
    playState.history.push(operation);
    playState.historyIndex = playState.history.length;
  }

  playState.cellValues[row][col] = newValue;
  playState.cellNotes[row][col] = cloneNotes(nextNotes);
  clearWrongCell(row, col);
  autoCheckCompletion();
}

export function applyUserValue(value: CellValue): void {
  if (!playState.selectedCell) return;
  const { row, col } = playState.selectedCell;
  if (value === null) {
    commitCellEdit(row, col, null, cloneNotes(playState.cellNotes[row][col]));
    return;
  }
  commitCellEdit(row, col, value, emptyNotes());
}

export function applyUserNote(value: CellValue | 'digits-only'): void {
  if (!playState.selectedCell) return;
  const { row, col } = playState.selectedCell;
  let nextValue = playState.cellValues[row][col];
  const notes = cloneNotes(playState.cellNotes[row][col]);

  if (value === null) {
    commitCellEdit(row, col, null, emptyNotes());
    return;
  }

  // Entering a note clears any placed value in this cell.
  if (nextValue !== null) nextValue = null;

  if (typeof value === 'number') {
    const i = notes.digits.indexOf(value);
    if (i >= 0) notes.digits.splice(i, 1);
    else notes.digits.push(value);
    commitCellEdit(row, col, nextValue, notes);
    return;
  }

  if (value === 'black') {
    notes.marker = notes.marker === 'black' ? null : 'black';
    commitCellEdit(row, col, nextValue, notes);
    return;
  }

  if (value === 'digits-only') {
    notes.marker = notes.marker === 'digits-only' ? null : 'digits-only';
    commitCellEdit(row, col, nextValue, notes);
  }
}

export function applyUserInput(value: CellValue | 'digits-only'): void {
  if (!playState.selectedCell) return;
  if (playState.inputMode === 'notes') {
    applyUserNote(value);
  } else if (value !== 'digits-only') {
    // The O button is disabled in value mode, so this branch never fires for
    // 'digits-only' in practice — guard explicitly so the types line up.
    applyUserValue(value);
  }
}

export function undoInput(): void {
  if (playState.historyIndex === 0) return;
  const op = playState.history[playState.historyIndex - 1];
  playState.historyIndex -= 1;
  playState.cellValues[op.row][op.col] = op.oldValue;
  playState.cellNotes[op.row][op.col] = cloneNotes(op.oldNotes);
  clearWrongCell(op.row, op.col);
}

export function redoInput(): void {
  if (playState.historyIndex === playState.history.length) return;
  const op = playState.history[playState.historyIndex];
  playState.historyIndex += 1;
  playState.cellValues[op.row][op.col] = op.newValue;
  playState.cellNotes[op.row][op.col] = cloneNotes(op.newNotes);
  clearWrongCell(op.row, op.col);
}

export function moveSelection(deltaRow: number, deltaCol: number): void {
  if (!playState.puzzleData) return;
  const max = playState.puzzleData.size - 1;
  const row = playState.selectedCell ? playState.selectedCell.row + deltaRow : 0;
  const col = playState.selectedCell ? playState.selectedCell.col + deltaCol : 0;
  playState.selectedCell = {
    row: Math.max(0, Math.min(max, row)),
    col: Math.max(0, Math.min(max, col)),
  };
}

export function toggleInputMode(): void {
  if (!playState.puzzleData) return;
  if (!playState.selectedCell) playState.selectedCell = { row: 0, col: 0 };
  playState.inputMode = playState.inputMode === 'value' ? 'notes' : 'value';
}

export function selectCell(row: number, col: number): void {
  const prev = playState.selectedCell;
  if (!prev || prev.row !== row || prev.col !== col) {
    playState.selectedCell = { row, col };
  } else {
    playState.inputMode = playState.inputMode === 'value' ? 'notes' : 'value';
  }
}

export function clearSelection(): void {
  playState.selectedCell = null;
}

const solveCallbacks = new Set<() => void>();

export function onSolved(callback: () => void): () => void {
  solveCallbacks.add(callback);
  return () => solveCallbacks.delete(callback);
}

function autoCheckCompletion(): void {
  if (!playState.puzzleData) return;
  for (let r = 0; r < playState.puzzleData.size; r++) {
    for (let c = 0; c < playState.puzzleData.size; c++) {
      if (playState.cellValues[r][c] === null) return;
    }
  }
  const response = solvePuzzle(playState.puzzleData);
  if ('error' in response) return;
  for (let r = 0; r < playState.puzzleData.size; r++) {
    for (let c = 0; c < playState.puzzleData.size; c++) {
      if (playState.cellValues[r][c] !== response.cells[r][c]) return;
    }
  }
  playState.feedback = 'Puzzle solved! 🎉';
  playState.feedbackError = false;
  trackEvent(`rublock/play/complete/${playState.puzzleData.size}`);
  for (const cb of solveCallbacks) cb();
}

export function checkCurrentPuzzle(): void {
  if (!playState.puzzleData) return;
  trackEvent(`rublock/play/check/${playState.puzzleData.size}`);

  let response: SolvedPuzzle;
  try {
    response = solvePuzzle(playState.puzzleData);
  } catch (err) {
    playState.wrongCells.clear();
    playState.feedbackError = true;
    playState.feedback = err instanceof Error ? err.message : String(err);
    return;
  }
  playState.wrongCells.clear();
  playState.feedbackError = false;

  let entered = 0;
  for (let r = 0; r < playState.puzzleData.size; r++) {
    for (let c = 0; c < playState.puzzleData.size; c++) {
      const value = playState.cellValues[r][c];
      if (value === null) continue;
      entered += 1;
      if (value !== response.cells[r][c]) {
        playState.wrongCells.add(cellKey(r, c));
      }
    }
  }

  const wrongCount = playState.wrongCells.size;
  const totalCells = playState.puzzleData.size * playState.puzzleData.size;
  if (entered === 0) {
    playState.feedback = 'Enter some cells, then check them.';
  } else if (wrongCount === 0 && entered === totalCells) {
    playState.feedback = 'Puzzle solved! 🎉';
  } else if (wrongCount === 0) {
    playState.feedback = 'All entered cells are correct.';
  } else {
    playState.feedback = `${wrongCount} wrong cell${wrongCount === 1 ? '' : 's'}.`;
  }
}
