import { en, type MessageKey, type Messages } from './en';
import { de } from './de';
import { pt } from './pt';
import { fr } from './fr';

const STORAGE_KEY = 'rublock-locale';

const catalogs: Record<string, Messages> = { en, de, pt, fr };
const AVAILABLE = Object.keys(catalogs);

let locale = $state<string>('en');

export function currentLocale(): string {
  return locale;
}

export function availableLocales(): readonly string[] {
  return AVAILABLE;
}

/** Translate a message key. Falls back to English when the active catalog
 *  is missing a key (shouldn't happen given the type constraint, but the
 *  fallback keeps the UI rendering if a catalog is broken at runtime). */
export function t(key: MessageKey): string {
  const cat = catalogs[locale] ?? en;
  return cat[key] ?? en[key];
}

/** Substitute `{name}` placeholders with values. No escaping — output is
 *  rendered as text (or HTML-escaped first by `md()`). */
export function format(template: string, vars: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (_, name: string) =>
    Object.prototype.hasOwnProperty.call(vars, name) ? String(vars[name]) : `{${name}}`
  );
}

export function tf(key: MessageKey, vars: Record<string, string | number>): string {
  return format(t(key), vars);
}

export function plural(n: number, one: MessageKey, other: MessageKey): string {
  return format(t(n === 1 ? one : other), { n });
}

/** HTML-escape `s`, then turn `{*…*}` into `<strong>…</strong>`. Use with
 *  `{@html md(t('key'))}` for the small set of strings that need an inline
 *  bold span. Source is escaped first, so translation files cannot inject
 *  arbitrary markup. */
export function md(s: string): string {
  const esc = s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return esc.replace(/\{\*([^*]+)\*\}/g, '<strong>$1</strong>');
}

function readStored(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function writeStored(value: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, value);
  } catch {
    // Ignore — preference just won't persist.
  }
}

/** Resolve locale at boot: explicit choice > browser language > 'en'. */
export function detectLocale(): string {
  const stored = readStored();
  if (stored && stored in catalogs) return stored;

  // navigator.languages can be undefined in old environments / tests.
  const langs = (typeof navigator !== 'undefined' && navigator.languages) || [];
  for (const lang of langs) {
    const lower = lang.toLowerCase();
    const base = lower.split('-')[0];
    if (lower in catalogs) return lower;
    if (base in catalogs) return base;
  }
  return 'en';
}

export function setLocale(l: string): void {
  if (!(l in catalogs)) return;
  locale = l;
  writeStored(l);
  if (typeof document !== 'undefined') {
    document.documentElement.lang = l;
  }
}

/** Call once at boot, before mount. Picks up the stored or browser locale
 *  and sets `document.documentElement.lang` so the first paint matches. */
export function initLocale(): void {
  const l = detectLocale();
  locale = l;
  if (typeof document !== 'undefined') {
    document.documentElement.lang = l;
  }
}
