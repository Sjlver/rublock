import { SvelteSet } from 'svelte/reactivity';
import { generatePuzzle, solvePuzzle } from '../wasm/api';
import { trackEvent } from '../analytics';
import { showToast } from './toast.svelte';
import { t, tf } from '../i18n/index.svelte';
import type {
  CellNotes,
  CellOperation,
  CellValue,
  InputMode,
  PuzzleData,
  SelectedCell,
  SolvedPuzzle,
} from './types';
import { noteDigitBit, NOTE_BLACK_BIT, NOTE_DIGITS_ONLY_BIT } from './types';

export function emptyCellNotes(size: number): CellNotes[][] {
  return Array.from({ length: size }, () => new Array<CellNotes>(size).fill(0));
}

export function emptyCellValues(size: number): CellValue[][] {
  return Array.from({ length: size }, () => Array<CellValue>(size).fill(null));
}

function sameOperation(a: CellOperation | undefined, b: CellOperation | undefined): boolean {
  if (!a || !b) return false;
  return (
    a.row === b.row &&
    a.col === b.col &&
    a.oldValue === b.oldValue &&
    a.newValue === b.newValue &&
    a.oldNotes === b.oldNotes &&
    a.newNotes === b.newNotes
  );
}

export function notesHaveContent(notes: CellNotes): boolean {
  return notes !== 0;
}

export function cellKey(row: number, col: number): string {
  return `${row},${col}`;
}

export function puzzleKey(data: PuzzleData): string {
  return `${data.row_targets.join(',')}|${data.col_targets.join(',')}`;
}

export const playState = $state({
  puzzleData: null as PuzzleData | null,
  cellValues: [] as CellValue[][],
  cellNotes: [] as CellNotes[][],
  inputMode: 'value' as InputMode,
  selectedCell: null as SelectedCell | null,
  undoStack: [] as CellOperation[],
  redoStack: [] as CellOperation[],
  wrongCells: new SvelteSet<string>(),
});

export function setPuzzle(data: PuzzleData): void {
  const samePuzzle =
    playState.puzzleData !== null && puzzleKey(playState.puzzleData) === puzzleKey(data);

  playState.puzzleData = data;
  if (!samePuzzle) {
    playState.cellValues = emptyCellValues(data.row_targets.length);
    playState.cellNotes = emptyCellNotes(data.row_targets.length);
    playState.selectedCell = null;
    playState.inputMode = 'value';
    playState.undoStack = [];
    playState.redoStack = [];
    playState.wrongCells.clear();
  }
}

export function loadRandomPuzzle(size: number): void {
  setPuzzle(generatePuzzle(size));
  trackEvent(`rublock/play/generate/${size}`);
}

interface PerSizeState {
  puzzleData: PuzzleData;
  cellValues: CellValue[][];
  cellNotes: CellNotes[][];
  inputMode: InputMode;
  selectedCell: SelectedCell | null;
  undoStack: CellOperation[];
  redoStack: CellOperation[];
  wrongCells: SvelteSet<string>;
}

const sizeStates = new Map<number, PerSizeState>();

function saveCurrentState(): void {
  if (!playState.puzzleData) return;
  sizeStates.set(playState.puzzleData.row_targets.length, {
    puzzleData: playState.puzzleData,
    cellValues: playState.cellValues.map((row) => [...row]),
    cellNotes: playState.cellNotes.map((row) => [...row]),
    inputMode: playState.inputMode,
    selectedCell: playState.selectedCell,
    undoStack: [...playState.undoStack],
    redoStack: [...playState.redoStack],
    wrongCells: new SvelteSet(playState.wrongCells),
  });
}

function loadSavedState(saved: PerSizeState): void {
  playState.puzzleData = saved.puzzleData;
  playState.cellValues = saved.cellValues;
  playState.cellNotes = saved.cellNotes;
  playState.inputMode = saved.inputMode;
  playState.selectedCell = saved.selectedCell;
  playState.undoStack = saved.undoStack;
  playState.redoStack = saved.redoStack;
  playState.wrongCells.clear();
  for (const k of saved.wrongCells) playState.wrongCells.add(k);
}

/** Switch to a new size, preserving in-progress puzzles per size. */
export function switchToSize(size: number): void {
  if (playState.puzzleData?.row_targets.length === size) return;
  saveCurrentState();

  const saved = sizeStates.get(size);
  if (saved) {
    loadSavedState(saved);
  } else {
    setPuzzle(generatePuzzle(size));
    trackEvent(`rublock/play/generate/${size}`);
  }
}

/**
 * Return the puzzle the Play tab uses for this size. If none exists yet
 * (because the user has not visited that size), generate one and cache it
 * in `sizeStates` so a later visit shows the same puzzle. The active Play
 * size is unchanged.
 */
export function ensurePlayPuzzleForSize(size: number): PuzzleData {
  if (playState.puzzleData?.row_targets.length === size) {
    return playState.puzzleData;
  }
  const saved = sizeStates.get(size);
  if (saved) return saved.puzzleData;
  const data = generatePuzzle(size);
  trackEvent(`rublock/play/generate/${size}`);
  const blank: PerSizeState = {
    puzzleData: data,
    cellValues: emptyCellValues(size),
    cellNotes: emptyCellNotes(size),
    inputMode: 'value',
    selectedCell: null,
    undoStack: [],
    redoStack: [],
    wrongCells: new SvelteSet(),
  };
  sizeStates.set(size, blank);
  return data;
}

/**
 * Replace the Play tab's puzzle for `data`'s size with `data`, then make
 * that size the active Play size. Per-size state for *other* sizes in
 * `sizeStates` is left intact so the user's partial progress on those
 * sizes survives the switch.
 */
export function replacePlayPuzzle(data: PuzzleData): void {
  const size = data.row_targets.length;
  saveCurrentState();
  sizeStates.delete(size);
  setPuzzle(data);
}

/** Generate a fresh puzzle for the current size, discarding any saved state. */
export function newPuzzle(size: number): void {
  sizeStates.delete(size);
  setPuzzle(generatePuzzle(size));
  trackEvent(`rublock/play/generate/${size}`);
}

function clearWrongCell(row: number, col: number): void {
  playState.wrongCells.delete(cellKey(row, col));
}

function commitCellEdit(row: number, col: number, newValue: CellValue, newNotes: CellNotes): void {
  if (!playState.puzzleData) return;
  const oldValue = playState.cellValues[row][col];
  const oldNotes = playState.cellNotes[row][col];

  if (oldValue === newValue && oldNotes === newNotes) return;

  const operation: CellOperation = { row, col, oldValue, newValue, oldNotes, newNotes };
  if (sameOperation(operation, playState.redoStack.at(-1))) {
    playState.undoStack.push(playState.redoStack.pop()!);
  } else {
    playState.redoStack.length = 0;
    playState.undoStack.push(operation);
  }

  playState.cellValues[row][col] = newValue;
  playState.cellNotes[row][col] = newNotes;
  clearWrongCell(row, col);
  autoCheckCompletion();
}

export function applyUserValue(value: CellValue): void {
  if (!playState.selectedCell) return;
  const { row, col } = playState.selectedCell;
  if (value === null) {
    commitCellEdit(row, col, null, playState.cellNotes[row][col]);
    return;
  }
  commitCellEdit(row, col, value, 0);
}

export function applyUserNote(value: CellValue | 'digits-only'): void {
  if (!playState.selectedCell) return;
  const { row, col } = playState.selectedCell;
  let nextValue = playState.cellValues[row][col];
  let notes = playState.cellNotes[row][col];

  if (value === null) {
    commitCellEdit(row, col, null, 0);
    return;
  }

  // Entering a note clears any placed value in this cell.
  if (nextValue !== null) nextValue = null;

  if (typeof value === 'number') {
    notes ^= noteDigitBit(value);
    commitCellEdit(row, col, nextValue, notes);
    return;
  }

  if (value === 'black') {
    // Exclusive with digits-only: toggling black clears digits-only
    notes =
      notes & NOTE_BLACK_BIT
        ? notes & ~NOTE_BLACK_BIT
        : (notes & ~NOTE_DIGITS_ONLY_BIT) | NOTE_BLACK_BIT;
    commitCellEdit(row, col, nextValue, notes);
    return;
  }

  if (value === 'digits-only') {
    // Exclusive with black: toggling digits-only clears black
    notes =
      notes & NOTE_DIGITS_ONLY_BIT
        ? notes & ~NOTE_DIGITS_ONLY_BIT
        : (notes & ~NOTE_BLACK_BIT) | NOTE_DIGITS_ONLY_BIT;
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
  const op = playState.undoStack.pop();
  if (!op) return;
  playState.redoStack.push(op);
  playState.cellValues[op.row][op.col] = op.oldValue;
  playState.cellNotes[op.row][op.col] = op.oldNotes;
  clearWrongCell(op.row, op.col);
}

export function redoInput(): void {
  const op = playState.redoStack.pop();
  if (!op) return;
  playState.undoStack.push(op);
  playState.cellValues[op.row][op.col] = op.newValue;
  playState.cellNotes[op.row][op.col] = op.newNotes;
  clearWrongCell(op.row, op.col);
}

export function moveSelection(deltaRow: number, deltaCol: number): void {
  if (!playState.puzzleData) return;
  const max = playState.puzzleData.row_targets.length - 1;
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
  const size = playState.puzzleData.row_targets.length;
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if (playState.cellValues[r][c] === null) return;
    }
  }
  let response: SolvedPuzzle;
  try {
    response = solvePuzzle(playState.puzzleData);
  } catch {
    return;
  }
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      if (playState.cellValues[r][c] !== response.cells[r][c]) return;
    }
  }
  showToast(t('toast_solved'), 'success');
  trackEvent(`rublock/play/complete/${size}`);
  for (const cb of solveCallbacks) cb();
}

export function checkCurrentPuzzle(): void {
  if (!playState.puzzleData) return;
  const size = playState.puzzleData.row_targets.length;
  trackEvent(`rublock/play/check/${size}`);

  let response: SolvedPuzzle;
  try {
    response = solvePuzzle(playState.puzzleData);
  } catch (err) {
    playState.wrongCells.clear();
    showToast(err instanceof Error ? err.message : String(err), 'error');
    return;
  }
  playState.wrongCells.clear();

  let entered = 0;
  for (let r = 0; r < size; r++) {
    for (let c = 0; c < size; c++) {
      const value = playState.cellValues[r][c];
      if (value === null) continue;
      entered += 1;
      if (value !== response.cells[r][c]) {
        playState.wrongCells.add(cellKey(r, c));
      }
    }
  }

  const wrongCount = playState.wrongCells.size;
  const totalCells = size * size;
  if (entered === 0) {
    showToast(t('toast_check_empty'));
  } else if (wrongCount === 0 && entered === totalCells) {
    showToast(t('toast_solved'), 'success');
  } else if (wrongCount === 0) {
    showToast(t('toast_check_all_correct'), 'success');
  } else if (wrongCount === 1) {
    showToast(t('toast_one_wrong'), 'error');
  } else {
    showToast(tf('toast_n_wrong', { n: wrongCount }), 'error');
  }
}
