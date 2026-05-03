import type { PuzzleData, TabName } from './types';
import { trackEvent } from '../analytics';

const VALID_TABS = new Set<TabName>(['play', 'solve', 'explain', 'print', 'howto']);

const BASE62 = '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ';

export const tabState = $state({ active: 'play' as TabName });

export function serializePuzzleTargets(data: PuzzleData): string {
  return [...data.row_targets, ...data.col_targets].map((n) => BASE62[n]).join('');
}

export function formatTargetsText(data: PuzzleData): string {
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

function decodeBase62Targets(p: string): PuzzleData | null {
  const values = [...p].map((c) => BASE62.indexOf(c));
  if (values.some((n) => n < 0)) return null;
  if (values.length % 2 !== 0) return null;
  const size = values.length / 2;
  if (size < 5 || size > 8) return null;
  return { size, row_targets: values.slice(0, size), col_targets: values.slice(size) };
}

export function parsePuzzleFromUrl(): PuzzleData | null {
  const param = new URLSearchParams(window.location.search).get('p');
  if (!param) return null;
  if (param.includes(',')) {
    const parsed = parseTargetsText(param);
    return 'error' in parsed ? null : parsed;
  }
  return decodeBase62Targets(param);
}

export function tabFromUrl(): TabName {
  const raw = new URLSearchParams(window.location.search).get('t') as TabName | null;
  if (!raw || !VALID_TABS.has(raw) || raw === 'play') return 'play';
  return raw;
}

export function setTab(name: TabName): void {
  tabState.active = name;
  trackEvent(`rublock/${name}/tab-view`);
}

/**
 * Keep the address bar in sync with app state: current puzzle as `p=`
 * (base62-encoded, 0-9a-zA-Z) and `t=` when the tab is not Play.
 */
export function syncUrl(puzzleData: PuzzleData | null, active: TabName): void {
  if (!puzzleData) return;
  const url = new URL(window.location.href);
  url.search = '';
  url.searchParams.set('p', serializePuzzleTargets(puzzleData));
  if (active !== 'play') url.searchParams.set('t', active);
  if (url.toString() !== window.location.href) history.replaceState(null, '', url);
}

/** Share link: puzzle only, no tab param. */
export function puzzleShareUrl(data: PuzzleData): string {
  const url = new URL(window.location.href);
  url.search = '';
  url.searchParams.set('p', serializePuzzleTargets(data));
  return url.toString();
}
