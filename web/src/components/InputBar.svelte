<script lang="ts">
  import type { CellValue } from '../state/types';

  interface Props {
    size: number;
    disabled: boolean;
    /** Called on tap (short press) — places a note */
    onPlaceNote: (value: CellValue | 'digits-only') => void;
    /** Called on hold (long press) — places a value */
    onPlaceValue: (value: CellValue) => void;
    /** Called to erase the selected cell */
    onErase: () => void;
  }

  let { size, disabled, onPlaceNote, onPlaceValue, onErase }: Props = $props();

  const PRESS_MS = 320;

  type BtnState = {
    pressed: boolean;
    progress: number;
    timerId: ReturnType<typeof setTimeout> | null;
    rafId: number | null;
    longFired: boolean;
    startedAt: number;
  };

  // Reactive state map per button id
  let states = $state<Record<string, BtnState>>({});

  function ensureState(id: string): BtnState {
    if (!states[id]) {
      states[id] = {
        pressed: false,
        progress: 0,
        timerId: null,
        rafId: null,
        longFired: false,
        startedAt: 0,
      };
    }
    return states[id];
  }

  function pressed(id: string): boolean {
    return states[id]?.pressed ?? false;
  }
  function progress(id: string): number {
    return states[id]?.progress ?? 0;
  }

  function startPress(id: string, onLong: (() => void) | null, e: MouseEvent | TouchEvent): void {
    if (disabled) return;
    e.preventDefault();
    const s = ensureState(id);
    s.pressed = true;
    s.longFired = false;
    s.startedAt = performance.now();

    const tick = () => {
      const t = (performance.now() - s.startedAt) / PRESS_MS;
      s.progress = Math.min(1, t);
      if (t < 1) s.rafId = requestAnimationFrame(tick);
    };
    s.rafId = requestAnimationFrame(tick);

    if (onLong) {
      s.timerId = setTimeout(() => {
        s.longFired = true;
        onLong();
      }, PRESS_MS);
    }
  }

  function endPress(id: string, fire: boolean, onTap: () => void): void {
    const s = states[id];
    if (!s) return;
    s.pressed = false;
    s.progress = 0;
    if (s.timerId) {
      clearTimeout(s.timerId);
      s.timerId = null;
    }
    if (s.rafId) {
      cancelAnimationFrame(s.rafId);
      s.rafId = null;
    }
    if (fire && !s.longFired) onTap();
  }

  let max = $derived(size - 2);
  let digits = $derived(Array.from({ length: max }, (_, i) => i + 1));
</script>

<div class="input-bar" role="group" aria-label="Input buttons">
  <!-- BLACK: tap = 'black' note marker, hold = place black value -->
  <button
    type="button"
    class="pad-btn"
    class:pressed={pressed('black')}
    aria-label="Black cell (hold) / note (tap)"
    {disabled}
    onmousedown={(e) => startPress('black', () => onPlaceValue('black'), e)}
    onmouseup={() => endPress('black', true, () => onPlaceNote('black'))}
    onmouseleave={() => endPress('black', false, () => {})}
    ontouchstart={(e) => startPress('black', () => onPlaceValue('black'), e)}
    ontouchend={(e) => {
      e.preventDefault();
      endPress('black', true, () => onPlaceNote('black'));
    }}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="pad-progress"
      style:transform="scaleX({progress('black')})"
      style:opacity={progress('black') > 0 ? 1 : 0}
    ></div>
    <span class="pad-btn-icon">
      <svg width="22" height="22" viewBox="0 0 24 24">
        <rect x="3" y="3" width="18" height="18" rx="2.5" fill="currentColor" />
      </svg>
    </span>
  </button>

  <!-- Digit buttons: tap = note, hold = value -->
  {#each digits as d (d)}
    <button
      type="button"
      class="pad-btn"
      class:pressed={pressed(String(d))}
      aria-label="Digit {d} (hold to place, tap for note)"
      {disabled}
      onmousedown={(e) => startPress(String(d), () => onPlaceValue(d), e)}
      onmouseup={() => endPress(String(d), true, () => onPlaceNote(d))}
      onmouseleave={() => endPress(String(d), false, () => {})}
      ontouchstart={(e) => startPress(String(d), () => onPlaceValue(d), e)}
      ontouchend={(e) => {
        e.preventDefault();
        endPress(String(d), true, () => onPlaceNote(d));
      }}
      oncontextmenu={(e) => e.preventDefault()}
    >
      <div
        class="pad-progress"
        style:transform="scaleX({progress(String(d))})"
        style:opacity={progress(String(d)) > 0 ? 1 : 0}
      ></div>
      <span class="pad-btn-digit">{d}</span>
    </button>
  {/each}

  <!-- O button: tap = digits-only note marker, no hold action -->
  <button
    type="button"
    class="pad-btn"
    class:pressed={pressed('o')}
    aria-label="Must be a digit (note)"
    {disabled}
    onmousedown={(e) => startPress('o', null, e)}
    onmouseup={() => endPress('o', true, () => onPlaceNote('digits-only'))}
    onmouseleave={() => endPress('o', false, () => {})}
    ontouchstart={(e) => startPress('o', null, e)}
    ontouchend={(e) => {
      e.preventDefault();
      endPress('o', true, () => onPlaceNote('digits-only'));
    }}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="pad-progress"
      style:transform="scaleX({progress('o')})"
      style:opacity={progress('o') > 0 ? 1 : 0}
    ></div>
    <span class="pad-btn-icon">
      <svg
        width="20"
        height="20"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.7"
        stroke-linecap="round"
      >
        <circle cx="12" cy="12" r="6.5" />
      </svg>
    </span>
  </button>

  <!-- ERASE: tap = erase, hold = erase (same both ways) -->
  <button
    type="button"
    class="pad-btn"
    class:pressed={pressed('erase')}
    aria-label="Erase cell"
    {disabled}
    onmousedown={(e) => startPress('erase', onErase, e)}
    onmouseup={() => endPress('erase', true, onErase)}
    onmouseleave={() => endPress('erase', false, () => {})}
    ontouchstart={(e) => startPress('erase', onErase, e)}
    ontouchend={(e) => {
      e.preventDefault();
      endPress('erase', true, onErase);
    }}
    oncontextmenu={(e) => e.preventDefault()}
  >
    <div
      class="pad-progress"
      style:transform="scaleX({progress('erase')})"
      style:opacity={progress('erase') > 0 ? 1 : 0}
    ></div>
    <span class="pad-btn-icon">
      <!-- Eraser icon — not an X, per design notes -->
      <svg
        width="22"
        height="22"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.7"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path
          d="M16.8 3.2l4 4a1.6 1.6 0 010 2.3L11.5 19.7a1.6 1.6 0 01-2.3 0l-4.9-4.9a1.6 1.6 0 010-2.3L14.5 3.2a1.6 1.6 0 012.3 0z"
        />
        <path d="M9 9.5l5.5 5.5" />
        <path d="M3 21h11" stroke-opacity="0.55" />
      </svg>
    </span>
  </button>
</div>
