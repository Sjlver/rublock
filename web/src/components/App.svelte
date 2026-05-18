<script lang="ts">
  import { onMount } from 'svelte';
  import BottomNav from './BottomNav.svelte';
  import PlayTab from './tabs/PlayTab.svelte';
  import CreateTab from './tabs/CreateTab.svelte';
  import PrintTab from './tabs/PrintTab.svelte';
  import HowToTab from './tabs/HowToTab.svelte';
  import WalkthroughTab from './tabs/WalkthroughTab.svelte';
  import PuzzleGrid from './PuzzleGrid.svelte';
  import { initWasm, generatePuzzle } from '../wasm/api';
  import { reportFatal } from '../error-overlay';
  import { loadRandomPuzzle, replacePlayPuzzle } from '../state/puzzle.svelte';
  import {
    applySavedStateToMemory,
    installPersistEffect,
    loadSavedState,
  } from '../state/storage.svelte';
  import {
    parsePuzzleFromUrl,
    clearUrlParams,
    tabFromUrl,
    tabState,
    setTab,
  } from '../state/url.svelte';
  import { trackEvent } from '../analytics';
  import { currentLocale, tf } from '../i18n/index.svelte';
  import type { PuzzleData } from '../state/types';
  import { classifyCached, classificationLabel } from '../state/classification';

  const PUZZLES_PER_BATCH = 6;
  const promoUrl = `${window.location.origin}${window.location.pathname}`;
  let promo = $derived(tf('print_promo', { url: promoUrl }));

  // Mirror the active locale onto <html lang>. `setLocale()` also does this,
  // but the $effect keeps the two in sync if the state ever changes another
  // way (e.g. HMR resets module state).
  $effect(() => {
    document.documentElement.lang = currentLocale();
  });

  // Scroll the tab content back to the top whenever the active tab changes
  // — long tabs (Walkthrough, How-to) otherwise keep their scroll position
  // when the user comes back via the bottom nav.
  $effect(() => {
    void tabState.active;
    document.querySelector('.screen-content')?.scrollTo({ top: 0 });
  });

  let ready = $state(false);
  let persistReady = $state(false);
  let printBusy = $state(false);

  type PrintPuzzle = { data: PuzzleData; difficulty: string };
  type PrintGroup = { puzzles: PrintPuzzle[] };
  const printGroups = $state<PrintGroup[]>([]);

  function makePrintPuzzle(data: PuzzleData): PrintPuzzle {
    const size = data.row_targets.length;
    return { data, difficulty: classificationLabel(classifyCached(data), size) };
  }

  // Install the persistence effect during component init. It subscribes to
  // playState/sizeStates immediately but only writes once `persistReady`
  // flips after the boot logic in onMount has populated state.
  installPersistEffect(() => persistReady);

  onMount(async () => {
    // Boot priority for the Play tab puzzle:
    //   1. URL `?p=` overrides only its own size; other saved sizes survive.
    //   2. Otherwise restore from localStorage (all sizes).
    //   3. Otherwise generate a fresh 6×6.
    const saved = loadSavedState();
    const fromUrl = parsePuzzleFromUrl();
    try {
      await initWasm();

      if (saved) applySavedStateToMemory(saved);

      if (fromUrl) {
        // `replacePlayPuzzle` saves the previous active size into sizeStates,
        // drops any saved progress for the URL puzzle's size, and makes the
        // URL puzzle active.
        replacePlayPuzzle(fromUrl);
      } else if (!saved) {
        loadRandomPuzzle(6);
      }

      const t = tabFromUrl();
      if (t !== 'play') setTab(t);

      // Per DESIGN_NOTES.md: clear the URL param after loading — keep URL clean.
      clearUrlParams();

      persistReady = true;

      // Pre-generate one page so Ctrl+P works immediately.
      await fillPrintOutput(6, 1);
      ready = true;
    } catch (err) {
      console.error('Initialization failed:', err);
      reportFatal(err);
    }
  });

  async function fillPrintOutput(size: number, pages: number): Promise<void> {
    const total = pages * PUZZLES_PER_BATCH;
    printGroups.length = 0;
    let generated = 0;
    while (generated < total) {
      const pageSize = Math.min(PUZZLES_PER_BATCH, total - generated);
      const puzzles = Array.from({ length: pageSize }, () => makePrintPuzzle(generatePuzzle(size)));
      printGroups.push({ puzzles });
      generated += pageSize;
      if (generated < total) await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  async function generateAndPrint(size: number, pages: number): Promise<void> {
    printBusy = true;
    trackEvent(`rublock/print/print/${size}`);
    try {
      await fillPrintOutput(size, pages);
      window.print();
    } finally {
      printBusy = false;
    }
  }
</script>

<div class="app-shell">
  <div class="screen-content">
    {#if tabState.active === 'play'}
      <PlayTab />
    {:else if tabState.active === 'create'}
      <CreateTab />
    {:else if tabState.active === 'print'}
      <PrintTab onPrint={generateAndPrint} busy={printBusy || !ready} />
    {:else if tabState.active === 'howto'}
      <HowToTab />
    {:else if tabState.active === 'steps'}
      <WalkthroughTab />
    {/if}
  </div>

  <BottomNav />
</div>

<div id="print-output" aria-hidden="true">
  {#each printGroups as group}
    <div class="page-group">
      {#each group.puzzles as p}
        <div class="print-puzzle">
          <PuzzleGrid puzzle={p.data} />
          <div class="print-difficulty">{p.difficulty}</div>
        </div>
      {/each}
      <div class="promo">{promo}</div>
    </div>
  {/each}
</div>
