// Post-solve "support the project" call-to-action.
//
// rublock is free and ad-free. Occasionally, after a solved puzzle, we show a
// full-screen card asking the player to support the project — either by
// donating (Liberapay / Ko-fi) or by sharing rublock with a friend. This module
// owns the policy:
//   * WHEN to show it — only when the lifetime solve count is prime, so the ask
//     is frequent at first (2nd, 3rd, 5th, 7th… solve) and then naturally rare.
//   * WHICH channel + copy to show — a persisted rotation so that, over time,
//     every (channel, copy) pairing is shown roughly equally. That's the data
//     we need to learn which combination converts best.
//   * Reporting impressions and clicks to GoatCounter (production only, via
//     `trackEvent`).
//
// The component (`SupportCard.svelte`) is intentionally dumb: it renders a
// prompt produced here and reports the events. All counters go through the
// centralised `storage.ts` helpers.

import type { MessageKey } from '../i18n/en';
import { trackEvent } from '../analytics';
import { readSolveCount, writeSolveCount, readSupportShown, writeSupportShown } from './storage';

export interface DonateChannel {
  kind: 'donate';
  /** Stable id, used in GoatCounter event paths and in tests. */
  id: 'liberapay' | 'kofi';
  /** Brand name shown to the player (a proper noun — not translated). */
  label: string;
  /** Destination URL. */
  url: string;
}

export interface ShareChannel {
  kind: 'share';
  id: 'share';
}

export type SupportChannel = DonateChannel | ShareChannel;

// NOTE(owner): the Liberapay handle `Sjlver` is confirmed live. The Ko-fi
// handle is a best guess — confirm or replace it once the Ko-fi account exists.
export const SUPPORT_CHANNELS: readonly SupportChannel[] = [
  { kind: 'donate', id: 'liberapay', label: 'Liberapay', url: 'https://liberapay.com/Sjlver/' },
  { kind: 'donate', id: 'kofi', label: 'Ko-fi', url: 'https://ko-fi.com/sjlver' },
  { kind: 'share', id: 'share' },
];

// Rotating calls-to-action per channel kind. The keys resolve through the i18n
// catalog, so the copy is translated. Keep these in sync with the
// `support_copy_*` / `support_share_copy_*` keys in `i18n/en.ts`.
const COPY_KEYS: Record<SupportChannel['kind'], readonly MessageKey[]> = {
  donate: [
    'support_copy_1',
    'support_copy_2',
    'support_copy_3',
    'support_copy_4',
    'support_copy_5',
  ],
  share: [
    'support_share_copy_1',
    'support_share_copy_2',
    'support_share_copy_3',
    'support_share_copy_4',
    'support_share_copy_5',
  ],
};

export interface SupportPrompt {
  channel: SupportChannel;
  copyKey: MessageKey;
  /** 0-based copy index; appears in the GoatCounter event path. */
  copyIndex: number;
}

/** Trial-division primality test. `n < 2` is not prime. */
export function isPrime(n: number): boolean {
  if (!Number.isInteger(n) || n < 2) return false;
  if (n % 2 === 0) return n === 2;
  if (n % 3 === 0) return n === 3;
  for (let i = 5; i * i <= n; i += 6) {
    if (n % i === 0 || n % (i + 2) === 0) return false;
  }
  return true;
}

/**
 * Record one solved puzzle and return the new lifetime solve count. The
 * support card is shown only when this count is prime.
 */
export function recordSolve(): number {
  const next = readSolveCount() + 1;
  writeSolveCount(next);
  return next;
}

/**
 * Pick the next CTA and advance the rotation. The channel cycles every show;
 * each channel then walks through its own copy list, so every (channel, copy)
 * pairing appears equally often. The "shown" counter is persisted so the
 * rotation continues across sessions rather than restarting at the same pairing.
 */
export function nextSupportPrompt(): SupportPrompt {
  const shown = readSupportShown();
  writeSupportShown(shown + 1);
  const channel = SUPPORT_CHANNELS[shown % SUPPORT_CHANNELS.length];
  const copies = COPY_KEYS[channel.kind];
  const copyIndex = Math.floor(shown / SUPPORT_CHANNELS.length) % copies.length;
  return { channel, copyKey: copies[copyIndex], copyIndex };
}

function eventPath(action: 'impression' | 'click', p: SupportPrompt): string {
  // e.g. "rublock/support/impression/liberapay/0" — slashes group the dimensions
  // so the GoatCounter dashboard can be filtered by action, channel, or copy.
  return `rublock/support/${action}/${p.channel.id}/${p.copyIndex}`;
}

export function trackSupportImpression(p: SupportPrompt): void {
  trackEvent(eventPath('impression', p));
}

export function trackSupportClick(p: SupportPrompt): void {
  trackEvent(eventPath('click', p));
}
