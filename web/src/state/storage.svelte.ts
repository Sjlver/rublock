import { SvelteSet } from 'svelte/reactivity';
import { playState, sizeStates, type PerSizeState } from './puzzle.svelte';
import { KEY_PLAY_STATE, safeRead, safeRemove, safeWrite } from './storage';
import type {
  CellNotes,
  CellOperation,
  CellValue,
  InputMode,
  PuzzleData,
} from './types';

const STORAGE_VERSION = 1;
const DEBOUNCE_MS = 250;

// ---------- Play state ----------

export type SupportedSize = 5 | 6 | 7 | 8;

interface SavedSizeState {
  puzzle: PuzzleData;
  cellValues: CellValue[][];
  cellNotes: CellNotes[][];
  inputMode: InputMode;
  undoStack: CellOperation[];
  redoStack: CellOperation[];
  wrongCells: string[];
}

export interface SavedPlayState {
  version: 1;
  activeSize: SupportedSize;
  sizes: Partial<Record<SupportedSize, SavedSizeState>>;
}

const SUPPORTED_SIZES = new Set<SupportedSize>([5, 6, 7, 8]);

function isSupportedSize(n: unknown): n is SupportedSize {
  return typeof n === 'number' && SUPPORTED_SIZES.has(n as SupportedSize);
}

function isCellValue(v: unknown, size: number): v is CellValue {
  if (v === null) return true;
  if (v === 'black') return true;
  return typeof v === 'number' && Number.isInteger(v) && v >= 1 && v <= size;
}

function isCellNotes(v: unknown): v is CellNotes {
  return typeof v === 'number' && Number.isInteger(v) && v >= 0 && v <= 0xff;
}

function isInputMode(v: unknown): v is InputMode {
  return v === 'value' || v === 'notes';
}

function validateGrid<T>(
  raw: unknown,
  size: number,
  check: (v: unknown) => v is T
): T[][] | null {
  if (!Array.isArray(raw) || raw.length !== size) return null;
  const rows: T[][] = [];
  for (const row of raw) {
    if (!Array.isArray(row) || row.length !== size) return null;
    for (const cell of row) {
      if (!check(cell)) return null;
    }
    rows.push([...row]);
  }
  return rows;
}

function validateOperation(raw: unknown, size: number): CellOperation | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  if (
    typeof o.row !== 'number' ||
    typeof o.col !== 'number' ||
    !Number.isInteger(o.row) ||
    !Number.isInteger(o.col) ||
    o.row < 0 ||
    o.row >= size ||
    o.col < 0 ||
    o.col >= size
  ) {
    return null;
  }
  if (
    !isCellValue(o.oldValue, size) ||
    !isCellValue(o.newValue, size) ||
    !isCellNotes(o.oldNotes) ||
    !isCellNotes(o.newNotes)
  ) {
    return null;
  }
  return {
    row: o.row,
    col: o.col,
    oldValue: o.oldValue,
    newValue: o.newValue,
    oldNotes: o.oldNotes,
    newNotes: o.newNotes,
  };
}

function validatePuzzle(raw: unknown): { puzzle: PuzzleData; size: number } | null {
  if (!raw || typeof raw !== 'object') return null;
  const p = raw as Record<string, unknown>;
  const rt = p.row_targets;
  const ct = p.col_targets;
  if (!Array.isArray(rt) || !Array.isArray(ct)) return null;
  if (rt.length !== ct.length) return null;
  const size = rt.length;
  if (!isSupportedSize(size)) return null;
  for (const v of rt) {
    if (typeof v !== 'number' || !Number.isInteger(v) || v < 0 || v > 255) return null;
  }
  for (const v of ct) {
    if (typeof v !== 'number' || !Number.isInteger(v) || v < 0 || v > 255) return null;
  }
  return {
    puzzle: { row_targets: [...rt], col_targets: [...ct] },
    size,
  };
}

function validateSize(raw: unknown): SavedSizeState | null {
  if (!raw || typeof raw !== 'object') return null;
  const s = raw as Record<string, unknown>;

  const puzzleResult = validatePuzzle(s.puzzle);
  if (!puzzleResult) return null;
  const { puzzle, size } = puzzleResult;

  const cellValues = validateGrid<CellValue>(s.cellValues, size, (v): v is CellValue =>
    isCellValue(v, size)
  );
  if (!cellValues) return null;

  const cellNotes = validateGrid<CellNotes>(s.cellNotes, size, isCellNotes);
  if (!cellNotes) return null;

  if (!isInputMode(s.inputMode)) return null;

  if (!Array.isArray(s.undoStack) || !Array.isArray(s.redoStack)) return null;
  const undoStack: CellOperation[] = [];
  for (const op of s.undoStack) {
    const v = validateOperation(op, size);
    if (!v) return null;
    undoStack.push(v);
  }
  const redoStack: CellOperation[] = [];
  for (const op of s.redoStack) {
    const v = validateOperation(op, size);
    if (!v) return null;
    redoStack.push(v);
  }

  if (!Array.isArray(s.wrongCells)) return null;
  const wrongCells: string[] = [];
  for (const k of s.wrongCells) {
    if (typeof k !== 'string') return null;
    const m = /^(\d+),(\d+)$/.exec(k);
    if (!m) return null;
    const r = Number(m[1]);
    const c = Number(m[2]);
    if (r < 0 || r >= size || c < 0 || c >= size) return null;
    wrongCells.push(k);
  }

  return { puzzle, cellValues, cellNotes, inputMode: s.inputMode, undoStack, redoStack, wrongCells };
}

function validate(raw: unknown): SavedPlayState | null {
  if (!raw || typeof raw !== 'object') return null;
  const o = raw as Record<string, unknown>;
  if (o.version !== STORAGE_VERSION) return null;
  if (!isSupportedSize(o.activeSize)) return null;
  if (!o.sizes || typeof o.sizes !== 'object') return null;
  const sizes: Partial<Record<SupportedSize, SavedSizeState>> = {};
  for (const [k, v] of Object.entries(o.sizes as Record<string, unknown>)) {
    const sz = Number(k);
    if (!isSupportedSize(sz)) return null;
    const valid = validateSize(v);
    if (!valid) return null;
    if (valid.puzzle.row_targets.length !== sz) return null;
    sizes[sz] = valid;
  }
  if (!sizes[o.activeSize]) return null;
  return { version: 1, activeSize: o.activeSize, sizes };
}

export function loadSavedState(): SavedPlayState | null {
  const raw = safeRead(KEY_PLAY_STATE);
  if (!raw) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return null;
  }
  return validate(parsed);
}

function toPerSizeState(saved: SavedSizeState): PerSizeState {
  const wrongCells = new SvelteSet<string>();
  for (const k of saved.wrongCells) wrongCells.add(k);
  return {
    puzzleData: saved.puzzle,
    cellValues: saved.cellValues.map((row) => [...row]),
    cellNotes: saved.cellNotes.map((row) => [...row]),
    inputMode: saved.inputMode,
    selectedCell: null,
    undoStack: [...saved.undoStack],
    redoStack: [...saved.redoStack],
    wrongCells,
  };
}

/**
 * Hydrate the in-memory `playState` and `sizeStates` from a saved snapshot.
 * Run this *before* the URL `?p=` and "generate fresh" fallbacks.
 */
export function applySavedStateToMemory(saved: SavedPlayState): void {
  for (const key of Object.keys(sizeStates)) {
    delete sizeStates[Number(key) as SupportedSize];
  }
  for (const [k, v] of Object.entries(saved.sizes)) {
    if (!v) continue;
    sizeStates[Number(k) as SupportedSize] = toPerSizeState(v);
  }
  const active = sizeStates[saved.activeSize]!;
  playState.puzzleData = active.puzzleData;
  playState.cellValues = active.cellValues.map((row) => [...row]);
  playState.cellNotes = active.cellNotes.map((row) => [...row]);
  playState.inputMode = active.inputMode;
  playState.selectedCell = null;
  playState.undoStack = [...active.undoStack];
  playState.redoStack = [...active.redoStack];
  playState.wrongCells.clear();
  for (const k of active.wrongCells) playState.wrongCells.add(k);
}

function snapshotActiveSize(): SavedSizeState | null {
  if (!playState.puzzleData) return null;
  return {
    puzzle: {
      row_targets: [...playState.puzzleData.row_targets],
      col_targets: [...playState.puzzleData.col_targets],
    },
    cellValues: playState.cellValues.map((row) => [...row]),
    cellNotes: playState.cellNotes.map((row) => [...row]),
    inputMode: playState.inputMode,
    undoStack: playState.undoStack.map((op) => ({ ...op })),
    redoStack: playState.redoStack.map((op) => ({ ...op })),
    wrongCells: [...playState.wrongCells],
  };
}

function snapshotSizeState(s: PerSizeState): SavedSizeState {
  return {
    puzzle: {
      row_targets: [...s.puzzleData.row_targets],
      col_targets: [...s.puzzleData.col_targets],
    },
    cellValues: s.cellValues.map((row) => [...row]),
    cellNotes: s.cellNotes.map((row) => [...row]),
    inputMode: s.inputMode,
    undoStack: s.undoStack.map((op) => ({ ...op })),
    redoStack: s.redoStack.map((op) => ({ ...op })),
    wrongCells: [...s.wrongCells],
  };
}

function serializeAll(): string | null {
  const active = snapshotActiveSize();
  if (!active) return null;
  const activeSize = active.puzzle.row_targets.length as SupportedSize;
  if (!isSupportedSize(activeSize)) return null;

  const sizes: Partial<Record<SupportedSize, SavedSizeState>> = {};
  for (const [k, v] of Object.entries(sizeStates)) {
    if (!v) continue;
    const sz = Number(k);
    if (!isSupportedSize(sz)) continue;
    sizes[sz] = snapshotSizeState(v);
  }
  // The active size may not be in `sizeStates` yet (it's only persisted
  // there on a size switch). Always write the live `playState` snapshot.
  sizes[activeSize] = active;

  const payload: SavedPlayState = { version: 1, activeSize, sizes };
  return JSON.stringify(payload);
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;

function flushSaveNow(): void {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  const json = serializeAll();
  if (json === null) return;
  safeWrite(KEY_PLAY_STATE, json);
}

function scheduleSave(): void {
  if (saveTimer !== null) clearTimeout(saveTimer);
  saveTimer = setTimeout(flushSaveNow, DEBOUNCE_MS);
}

/**
 * Wire up a Svelte 5 `$effect` that watches `playState` + `sizeStates` and
 * writes the merged snapshot to localStorage on every change (debounced).
 *
 * Must be called synchronously from a component's `<script>` body — Svelte 5
 * forbids `$effect` outside component initialization. The `isReady` callback
 * gates writes until the host component has finished its boot logic; before
 * that, the effect just subscribes to deps without touching storage.
 */
export function installPersistEffect(isReady: () => boolean): void {
  $effect(() => {
    // Touch every reactive dep so deep mutations re-run the effect. Svelte 5
    // $state proxies arrays at every depth, so reading individual cells is
    // enough to subscribe to per-cell mutations.
    const pd = playState.puzzleData;
    if (pd) {
      void pd.row_targets.length;
      void pd.col_targets.length;
    }
    void playState.inputMode;
    void playState.undoStack.length;
    void playState.redoStack.length;
    void playState.wrongCells.size;
    for (let r = 0; r < playState.cellValues.length; r++) {
      const row = playState.cellValues[r];
      for (let c = 0; c < row.length; c++) void row[c];
    }
    for (let r = 0; r < playState.cellNotes.length; r++) {
      const row = playState.cellNotes[r];
      for (let c = 0; c < row.length; c++) void row[c];
    }

    for (const key of Object.keys(sizeStates)) {
      const s = sizeStates[Number(key) as SupportedSize];
      if (!s) continue;
      void s.inputMode;
      void s.undoStack.length;
      void s.redoStack.length;
      void s.wrongCells.size;
      for (let r = 0; r < s.cellValues.length; r++) {
        const row = s.cellValues[r];
        for (let c = 0; c < row.length; c++) void row[c];
      }
      for (let r = 0; r < s.cellNotes.length; r++) {
        const row = s.cellNotes[r];
        for (let c = 0; c < row.length; c++) void row[c];
      }
    }

    if (!isReady()) return;
    scheduleSave();
  });
}

/** Forget any saved play state. Exposed for tests / explicit reset. */
export function clearSavedState(): void {
  if (saveTimer !== null) {
    clearTimeout(saveTimer);
    saveTimer = null;
  }
  safeRemove(KEY_PLAY_STATE);
}
