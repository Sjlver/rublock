<script lang="ts">
  import type { TabName } from '../state/types';
  import { tabState, setTab } from '../state/url.svelte';

  interface Tab {
    id: TabName;
    label: string;
  }

  const TABS: Tab[] = [
    { id: 'play', label: 'Play' },
    { id: 'solve', label: 'Solve' },
    { id: 'steps', label: 'Walkthrough' },
    { id: 'print', label: 'Print' },
    { id: 'howto', label: 'How to play' },
  ];
</script>

<nav class="bottom-nav" aria-label="Main sections">
  <div class="bottom-nav-inner">
    {#each TABS as tab (tab.id)}
      {@const isActive = tabState.active === tab.id}
      <button
        type="button"
        class="nav-btn"
        class:active={isActive}
        aria-current={isActive ? 'page' : undefined}
        onclick={() => setTab(tab.id)}
      >
        <div class="nav-btn-pill">
          {#if tab.id === 'play'}
            <!-- 2×2 grid with filled diagonal cells -->
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="3.5" y="3.5" width="17" height="17" rx="3" />
              <line x1="12" y1="3.5" x2="12" y2="20.5" />
              <line x1="3.5" y1="12" x2="20.5" y2="12" />
              <rect
                x="14.5"
                y="6"
                width="3.5"
                height="3.5"
                rx="0.6"
                fill={isActive ? 'currentColor' : 'none'}
              />
              <rect
                x="6"
                y="14.5"
                width="3.5"
                height="3.5"
                rx="0.6"
                fill={isActive ? 'currentColor' : 'none'}
              />
            </svg>
          {:else if tab.id === 'solve'}
            <!-- Wand / sparkle -->
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M5 19L15 9" />
              <path d="M14 8l2 2" />
              <path d="M18 4v3M16.5 5.5h3" />
              <path d="M6 5v2M5 6h2" />
              <path d="M19 14v2M18 15h2" />
            </svg>
          {:else if tab.id === 'steps'}
            <!-- Step bars -->
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M4 18h4v-4" />
              <path d="M10 14h4v-4" />
              <path d="M16 10h4V6" />
            </svg>
          {:else if tab.id === 'print'}
            <!-- Printer -->
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="6" y="3.5" width="12" height="6" rx="1" />
              <path
                d="M6 17H4.5A1.5 1.5 0 013 15.5v-5A1.5 1.5 0 014.5 9h15A1.5 1.5 0 0121 10.5v5A1.5 1.5 0 0119.5 17H18"
              />
              <rect x="6" y="14" width="12" height="6.5" rx="1" />
              <circle cx="17.5" cy="11.5" r="0.6" fill="currentColor" stroke="none" />
            </svg>
          {:else if tab.id === 'howto'}
            <!-- Help circle -->
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.7"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="8.5" />
              <path
                d="M9.5 9.2c0-1.3 1.1-2.3 2.5-2.3s2.5 1 2.5 2.3c0 .9-.5 1.5-1.4 2-.8.5-1.1 1-1.1 1.7"
              />
              <circle cx="12" cy="16.5" r="0.6" fill="currentColor" stroke="none" />
            </svg>
          {/if}
        </div>
        <span class="nav-btn-label">{tab.label}</span>
      </button>
    {/each}
  </div>
</nav>
