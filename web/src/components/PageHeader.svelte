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
  const labelKeys: Record<string, 'loc_en' | 'loc_de'> = {
    en: 'loc_en',
    de: 'loc_de',
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
      <div class="size-selector locale-switcher" role="group" aria-label={t('header_locale_aria')}>
        {#each availableLocales() as code (code)}
          <button
            type="button"
            class="size-btn"
            class:active={code === currentLocale()}
            aria-pressed={code === currentLocale()}
            onclick={() => setLocale(code)}
          >
            {localeLabel(code)}
          </button>
        {/each}
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

  .locale-switcher .size-btn {
    padding: 4px 8px;
    font-size: 11.5px;
    letter-spacing: 0.02em;
  }
</style>
