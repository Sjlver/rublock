<script lang="ts">
  import { availableLocales, currentLocale, setLocale, t } from '../i18n/index.svelte';

  interface Props {
    title: string;
    status?: string;
    statusTone?: 'default' | 'error' | 'success';
    onShare?: () => void;
  }

  let { title, status = '', statusTone = 'default', onShare }: Props = $props();

  // Map locale code → label key. Catalogs only need entries for locales they
  // know about; unknown locales fall back to upper-cased code.
  const labelKeys: Record<string, 'loc_en' | 'loc_de' | 'loc_pt'> = {
    en: 'loc_en',
    de: 'loc_de',
    pt: 'loc_pt',
  };

  function localeLabel(code: string): string {
    const key = labelKeys[code];
    return key ? t(key) : code.toUpperCase();
  }
</script>

<div class="page-header">
  <div class="page-header-row">
    <h1 class="page-title">{title}</h1>
    <div class="header-actions">
      {#if onShare}
        <button
          type="button"
          class="btn-share"
          onclick={onShare}
          aria-label={t('header_share_aria')}
        >
          <!-- Share icon -->
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <circle cx="6" cy="12" r="2.4" />
            <circle cx="18" cy="6.5" r="2.4" />
            <circle cx="18" cy="17.5" r="2.4" />
            <path d="M8 11l8-3.5M8 13l8 3.5" />
          </svg>
          {t('header_share')}
        </button>
      {/if}
      <div class="locale-switcher">
        <select
          class="locale-select"
          aria-label={t('header_locale_aria')}
          value={currentLocale()}
          onchange={(e) => setLocale((e.currentTarget as HTMLSelectElement).value)}
        >
          {#each availableLocales() as code (code)}
            <option value={code}>{localeLabel(code)}</option>
          {/each}
        </select>
        <svg
          class="locale-chevron"
          width="10"
          height="10"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </div>
    </div>
  </div>
  {#if status}
    <div
      class="page-status"
      class:error={statusTone === 'error'}
      class:success={statusTone === 'success'}
      role="status"
      aria-live="polite"
    >
      {status}
    </div>
  {/if}
</div>

<style>
  .header-actions {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  /* Locale switcher: native <select> styled to match the share button.
     Native selects give us keyboard nav, screen-reader support, mobile
     pickers, and "open on click" for free, and scale to any number of
     options. */
  .locale-switcher {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .locale-select {
    appearance: none;
    -webkit-appearance: none;
    height: 34px;
    padding: 0 26px 0 12px;
    border: 1px solid var(--line-2);
    background: var(--card);
    border-radius: 999px;
    color: var(--ink);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    letter-spacing: -0.005em;
    cursor: pointer;
    box-shadow: 0 1px 0 rgba(20, 20, 40, 0.03);
  }
  .locale-select:hover {
    background: var(--tint);
  }
  .locale-select:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .locale-chevron {
    position: absolute;
    right: 10px;
    color: var(--muted);
    pointer-events: none;
  }
</style>
