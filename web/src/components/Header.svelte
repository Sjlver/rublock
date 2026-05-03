<script lang="ts">
  import type { TabName } from '../state/types';
  import { tabState, setTab } from '../state/url.svelte';

  interface Props {
    status: string;
  }
  let { status }: Props = $props();

  const TABS: { id: TabName; label: string }[] = [
    { id: 'play', label: 'Play' },
    { id: 'solve', label: 'Solve' },
    { id: 'explain', label: 'Explain' },
    { id: 'print', label: 'Print' },
    { id: 'howto', label: 'How to Play' },
  ];
</script>

<header class="app-header">
  <h1>Doplo Puzzle Generator</h1>
  <span class="app-status" role="status" aria-live="polite">{status}</span>
  <nav class="tabs" aria-label="Main sections">
    {#each TABS as tab (tab.id)}
      <button
        type="button"
        class="tab-btn"
        class:active={tabState.active === tab.id}
        onclick={() => setTab(tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>
</header>
