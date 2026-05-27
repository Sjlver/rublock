// Centralised localStorage access for the entire app.
//
// Anything in the codebase that touches `localStorage` should go through this
// module so that:
//   * key naming stays consistent (`rublock-<feature>`),
//   * IO failures (private mode, quota exceeded, disabled cookies) degrade
//     silently and consistently,
//   * a single grep finds every persisted key.
//
// The play-state slot is large/structured and lives in `storage.svelte.ts`
// alongside the persistence effect; this file holds the small primitive
// keys plus the shared safe-IO helpers.

export const KEY_PLAY_STATE = 'rublock-play-state';
export const KEY_LOCALE = 'rublock-locale';
export const KEY_HINT_DISMISSED = 'rublock-hint-dismissed';
export const KEY_SOLVE_COUNT = 'rublock-solve-count';
export const KEY_SUPPORT_SHOWN = 'rublock-support-shown';

let writesDisabled = false;
let warnedOnce = false;

function warnOnce(err: unknown): void {
  if (warnedOnce) return;
  warnedOnce = true;
  console.warn('rublock storage unavailable:', err);
}

export function safeRead(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch (err) {
    warnOnce(err);
    return null;
  }
}

export function safeWrite(key: string, value: string): void {
  if (writesDisabled) return;
  try {
    localStorage.setItem(key, value);
  } catch (err) {
    writesDisabled = true;
    warnOnce(err);
  }
}

export function safeRemove(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // Nothing useful we can do.
  }
}

// ---------- Locale ----------

export function readStoredLocale(): string | null {
  return safeRead(KEY_LOCALE);
}

export function writeStoredLocale(value: string): void {
  safeWrite(KEY_LOCALE, value);
}

// ---------- Hint banner ----------

export function readHintDismissed(): boolean {
  return safeRead(KEY_HINT_DISMISSED) === '1';
}

export function writeHintDismissed(value: boolean): void {
  if (value) safeWrite(KEY_HINT_DISMISSED, '1');
  else safeRemove(KEY_HINT_DISMISSED);
}

// ---------- Post-solve support CTA counters ----------
//
// Two small non-negative integers. `solve-count` is the lifetime number of
// puzzles solved and drives the prime-number gate that decides when to show the
// support card; `support-shown` is how many times the card has been shown and
// drives the platform/copy rotation. Both persist so the cadence and the
// rotation continue across sessions instead of resetting each visit.

function readCount(key: string): number {
  const raw = safeRead(key);
  if (raw === null) return 0;
  const n = Number.parseInt(raw, 10);
  return Number.isFinite(n) && n >= 0 ? n : 0;
}

export function readSolveCount(): number {
  return readCount(KEY_SOLVE_COUNT);
}

export function writeSolveCount(value: number): void {
  safeWrite(KEY_SOLVE_COUNT, String(value));
}

export function readSupportShown(): number {
  return readCount(KEY_SUPPORT_SHOWN);
}

export function writeSupportShown(value: number): void {
  safeWrite(KEY_SUPPORT_SHOWN, String(value));
}
