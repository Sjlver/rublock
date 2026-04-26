import type { PuzzleData, TabName } from './types';

const VALID_TABS = new Set<TabName>(['play', 'solve', 'print', 'howto']);

export const tabState = $state({ active: 'play' as TabName });

export function serializePuzzleTargets(data: PuzzleData): string {
  return [...data.row_targets, ...data.col_targets].join(',');
}

export function parseTargetsText(text: string): { error: string } | PuzzleData {
  const cleaned = text.trim();
  if (!cleaned) return { error: 'Enter targets in r1,...,rN,c1,...,cN format.' };
  const values = cleaned
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .map((s) => Number.parseInt(s, 10));
  if (values.some((n) => Number.isNaN(n) || n < 0 || n > 255)) {
    return { error: 'Targets must be comma-separated integers between 0 and 255.' };
  }
  if (values.length % 2 !== 0) {
    return { error: 'Target list must contain an even number of values.' };
  }
  const size = values.length / 2;
  if (size < 5 || size > 8) return { error: 'Puzzle size must be between 5 and 8.' };
  return {
    size,
    row_targets: values.slice(0, size),
    col_targets: values.slice(size),
  };
}

export function parsePuzzleFromUrl(): PuzzleData | null {
  const param = new URLSearchParams(window.location.search).get('p');
  if (!param) return null;
  const parsed = parseTargetsText(param);
  if ('error' in parsed) return null;
  return parsed;
}

export function tabFromUrl(): TabName {
  const raw = new URLSearchParams(window.location.search).get('t') as TabName | null;
  if (!raw || !VALID_TABS.has(raw) || raw === 'play') return 'play';
  return raw;
}

export function setTab(name: TabName): void {
  tabState.active = name;
}

/**
 * Keep the address bar in sync with app state: current puzzle as `p=`
 * (comma-separated, not URLSearchParams-serialized) and `t=` when the tab is
 * not Play.
 */
export function syncUrl(puzzleData: PuzzleData | null, active: TabName): void {
  if (!puzzleData) return;
  const p = serializePuzzleTargets(puzzleData);
  let search = `?p=${p}`;
  if (active !== 'play') search += `&t=${active}`;
  const next = `${window.location.pathname}${search}${window.location.hash}`;
  const cur = `${window.location.pathname}${window.location.search}${window.location.hash}`;
  if (next !== cur) history.replaceState(null, '', next);
}

/** Share link: puzzle only, no tab param. (Plain `p=` so commas stay readable.) */
export function puzzleShareUrl(data: PuzzleData): string {
  const base = `${window.location.origin}${window.location.pathname}`;
  const hash = window.location.hash || '';
  return `${base}?p=${serializePuzzleTargets(data)}${hash}`;
}
